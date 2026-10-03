use astrology_engine::{
    calculate_chart, find_sun_crossing, Body, CalculationError, ChartBody, ChartInput, Direction,
    Ephemeris, Epoch, HouseSystem, Location, SunSearchInput,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use worker::*;

thread_local! {
    static EPHEMERIS: RefCell<Option<(String, String, Ephemeris)>> = const { RefCell::new(None) };
}

#[derive(Deserialize)]
struct ChartRequest {
    date: String,
    location: LocationInput,
    #[serde(rename = "houseSystem")]
    house_system: Option<String>,
    #[serde(rename = "designLookback")]
    #[allow(dead_code)]
    design_lookback: Option<bool>,
}

#[derive(Deserialize)]
struct LocationInput {
    latitude: f64,
    longitude: f64,
}

#[derive(Serialize)]
struct Chart {
    #[serde(rename = "utcDate")]
    utc_date: String,
    location: Location,
    #[serde(rename = "planets")]
    bodies: Vec<ChartBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cusps: Option<Vec<f64>>,
}

#[derive(Deserialize)]
struct FindMomentRequest {
    #[serde(rename = "birthMoment")]
    birth_moment: String,
    planet: String,
    #[serde(rename = "targetLongitude")]
    target_longitude: f64,
    direction: String,
}

enum ChartError {
    OutOfCoverage(String),
    Internal(Error),
}
impl From<Error> for ChartError {
    fn from(error: Error) -> Self {
        Self::Internal(error)
    }
}
#[derive(Default)]
struct PhaseTimings;

fn error_json(message: &str, status: u16) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({"error":message}))?.with_status(status))
}

fn response_error(error: ChartError) -> Result<Response> {
    match error {
        ChartError::OutOfCoverage(message) => error_json(&message, 422),
        ChartError::Internal(_) => error_json("internal", 500),
    }
}

fn et_to_date(et: f64) -> String {
    let (y, m, d, ..) = Epoch::from_et_seconds(et).to_gregorian_utc();
    format!("{y:04}-{m:02}-{d:02}")
}

fn calculation_error(error: CalculationError) -> ChartError {
    match error {
        CalculationError::OutOfCoverage { window, lookback_s } => {
            ChartError::OutOfCoverage(format!(
                "date outside ephemeris coverage ({}..{})",
                et_to_date(window.lo_et + lookback_s),
                et_to_date(window.hi_et)
            ))
        }
        error => ChartError::Internal(Error::RustError(error.to_string())),
    }
}

fn pre_gate(env: &Env, epoch: Epoch) -> std::result::Result<(), ChartError> {
    let parse = |name: &str| -> std::result::Result<i32, ChartError> {
        env.var(name)?.to_string().trim().parse().map_err(|_| {
            ChartError::Internal(Error::RustError("invalid coverage configuration".into()))
        })
    };
    let min = parse("KERNEL_COVERAGE_MIN_YEAR")?;
    let max = parse("KERNEL_COVERAGE_MAX_YEAR")?;
    if min > max {
        return Err(ChartError::Internal(Error::RustError(
            "inverted coverage configuration".into(),
        )));
    }
    let (year, ..) = epoch.to_gregorian_utc();
    if year < min || year > max {
        return Err(ChartError::OutOfCoverage(format!(
            "date outside ephemeris coverage ({min}..{max})"
        )));
    }
    Ok(())
}

async fn ensure_ephemeris(env: &Env) -> std::result::Result<bool, ChartError> {
    let key = env.var("EPHEMERIS_KEY")?.to_string();
    let hash = env.var("EPHEMERIS_SHA256")?.to_string();
    let warm = EPHEMERIS.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|(k, h, _)| *k == key && *h == hash)
    });
    if warm {
        return Ok(true);
    }
    let object = env
        .bucket("EPHEMERIS")?
        .get(&key)
        .execute()
        .await?
        .ok_or_else(|| Error::RustError("dataset missing".into()))?;
    let bytes = object
        .body()
        .ok_or_else(|| Error::RustError("dataset body missing".into()))?
        .bytes()
        .await?;
    if format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Err(ChartError::Internal(Error::RustError(
            "dataset SHA-256 mismatch".into(),
        )));
    }
    let ephemeris = Ephemeris::parse(bytes)
        .map_err(|e| ChartError::Internal(Error::RustError(e.to_string())))?;
    EPHEMERIS.with(|slot| *slot.borrow_mut() = Some((key, hash, ephemeris)));
    Ok(false)
}

fn with_ephemeris<T>(
    calculate: impl FnOnce(&Ephemeris) -> std::result::Result<T, CalculationError>,
) -> std::result::Result<T, ChartError> {
    EPHEMERIS.with(|slot| {
        let borrowed = slot.borrow();
        let (_, _, ephemeris) = borrowed
            .as_ref()
            .ok_or_else(|| Error::RustError("dataset unavailable".into()))?;
        calculate(ephemeris).map_err(calculation_error)
    })
}

fn parse_house_system(value: Option<&str>) -> HouseSystem {
    match value {
        Some("equal") => HouseSystem::Equal,
        Some("whole-sign") => HouseSystem::WholeSign,
        _ => HouseSystem::Placidus,
    }
}

async fn build_chart(
    env: &Env,
    request: &ChartRequest,
    base: Epoch,
    warm: &mut bool,
    _phases: &mut PhaseTimings,
) -> std::result::Result<Chart, ChartError> {
    pre_gate(env, base)?;
    *warm = ensure_ephemeris(env).await?;
    let location = Location {
        latitude: request.location.latitude,
        longitude: request.location.longitude,
    };
    let chart = with_ephemeris(|eph| {
        calculate_chart(
            eph,
            ChartInput {
                epoch: base,
                location,
                house_system: parse_house_system(request.house_system.as_deref()),
            },
        )
    })?;
    Ok(Chart {
        utc_date: request.date.clone(),
        location,
        bodies: chart.bodies,
        cusps: chart.cusps,
    })
}

async fn post_charts(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let request: ChartRequest = match serde_json::from_str(&req.text().await.unwrap_or_default()) {
        Ok(value) => value,
        Err(_) => return error_json("request body must be JSON with date + location", 400),
    };
    if !(-90.0..=90.0).contains(&request.location.latitude)
        || !(-180.0..=180.0).contains(&request.location.longitude)
    {
        return error_json(
            "latitude must be in [-90, 90] and longitude in [-180, 180]",
            400,
        );
    }
    let epoch = match Epoch::from_gregorian_str(&request.date) {
        Ok(value) => value,
        Err(_) => {
            return error_json(
                "date must be an ISO-8601 UTC instant (e.g. 2000-01-01T12:00:00Z)",
                400,
            )
        }
    };
    if request
        .house_system
        .as_deref()
        .is_some_and(|s| !["equal", "placidus", "whole-sign"].contains(&s))
    {
        return error_json(
            "houseSystem must be one of: equal, placidus, whole-sign",
            400,
        );
    }
    let mut warm = false;
    match build_chart(&ctx.env, &request, epoch, &mut warm, &mut PhaseTimings).await {
        Ok(chart) => {
            let mut response = Response::from_json(&chart)?;
            response
                .headers_mut()
                .set("x-ephemeris-cache", if warm { "warm" } else { "cold" })?;
            Ok(response)
        }
        Err(error) => response_error(error),
    }
}

fn format_moment_iso(epoch: Epoch) -> String {
    let (y, m, d, h, mi, ..) = epoch.to_gregorian_utc();
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:00.000Z")
}

async fn find_moment_instant(
    env: &Env,
    _body: Body,
    base: Epoch,
    target_longitude: f64,
    direction: Direction,
    warm: &mut bool,
    _phases: &mut PhaseTimings,
) -> std::result::Result<Option<String>, ChartError> {
    pre_gate(env, base)?;
    *warm = ensure_ephemeris(env).await?;
    let instant = with_ephemeris(|eph| {
        find_sun_crossing(
            eph,
            SunSearchInput {
                epoch: base,
                target_longitude,
                direction,
            },
        )
    })?;
    #[cfg(feature = "verification")]
    CAPTURE_INSTANT.with(|slot| *slot.borrow_mut() = instant);
    Ok(instant.map(format_moment_iso))
}

async fn post_find_moment(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let request: FindMomentRequest =
        match serde_json::from_str(&req.text().await.unwrap_or_default()) {
            Ok(value) => value,
            Err(_) => return error_json(
                "request body must be JSON with birthMoment + planet + targetLongitude + direction",
                400,
            ),
        };
    if !(0.0..360.0).contains(&request.target_longitude) {
        return error_json("targetLongitude must be in [0, 360)", 400);
    }
    let direction = match request.direction.as_str() {
        "backward" => Direction::Backward,
        "forward" => Direction::Forward,
        _ => return error_json("direction must be \"backward\" or \"forward\"", 400),
    };
    let body = match Body::ALL
        .into_iter()
        .find(|b| b.wire_name() == request.planet)
    {
        Some(Body::Sun) => Body::Sun,
        Some(_) => {
            return error_json(
                "find-moment is only available for the Sun (the design-moment use case)",
                400,
            )
        }
        None => return error_json("unknown planet", 400),
    };
    let epoch = match Epoch::from_gregorian_str(&request.birth_moment) {
        Ok(value) => value,
        Err(_) => {
            return error_json(
                "birthMoment must be an ISO-8601 UTC instant (e.g. 2000-01-01T12:00:00Z)",
                400,
            )
        }
    };
    match find_moment_instant(
        &ctx.env,
        body,
        epoch,
        request.target_longitude,
        direction,
        &mut false,
        &mut PhaseTimings,
    )
    .await
    {
        Ok(Some(moment)) => Response::from_json(&serde_json::json!({"moment":moment})),
        Ok(None) => error_json("no crossing within ±2 years of the birth moment", 422),
        Err(error) => response_error(error),
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new()
        .get("/health", |_, _| Response::ok("ok"))
        .post_async("/charts", post_charts)
        .post_async("/charts/find-moment", post_find_moment);
    #[cfg(feature = "verification")]
    let router = router
        .post_async("/__capture", capture_batch)
        .get("/__structural", |_, _| {
            Response::from_json(&serde_json::json!({"passed": structural::run()}))
        })
        .get("/__fixture", |_, _| {
            Response::from_bytes(structural::fixture())
        })
        .get_async("/__validate", validate_dataset);
    router.run(req, env).await
}

#[cfg(feature = "verification")]
include!("verification.rs");

#[cfg(feature = "verification")]
#[path = "../../../tools/verification/structural.rs"]
mod structural;
