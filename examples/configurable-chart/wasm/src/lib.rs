// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use astrology_engine::{
    calculate_configured_chart, ChartInput, Ephemeris, Epoch, HouseSystem, Location,
    ZodiacConfiguration,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(tag = "scale", content = "value", rename_all = "lowercase")]
enum Instant {
    Utc(String),
    Et(f64),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    instant: Instant,
    latitude: f64,
    longitude: f64,
    house_system: HouseSystem,
    configuration: ZodiacConfiguration,
}

/// Stateless adapter: callers supply the dataset bytes and explicit settings.
#[wasm_bindgen]
pub fn calculate(bytes: &[u8], request_json: &str) -> Result<String, String> {
    let request: Request = serde_json::from_str(request_json).map_err(|error| error.to_string())?;
    let epoch = match request.instant {
        Instant::Utc(utc) => {
            if !utc.ends_with('Z') && !utc.ends_with(" UTC") {
                return Err("Use an explicit UTC timestamp ending in Z or UTC".into());
            }
            utc.parse::<Epoch>().map_err(|error| error.to_string())?
        }
        Instant::Et(seconds) if seconds.is_finite() => Epoch::from_et_seconds(seconds),
        Instant::Et(_) => return Err("ET seconds must be finite".into()),
    };
    let ephemeris = Ephemeris::parse(bytes.to_vec()).map_err(|error| error.to_string())?;
    let chart = calculate_configured_chart(
        &ephemeris,
        ChartInput {
            epoch,
            location: Location {
                latitude: request.latitude,
                longitude: request.longitude,
            },
            house_system: request.house_system,
        },
        request.configuration,
    )
    .map_err(|error| error.to_string())?;
    serde_json::to_string(&chart).map_err(|error| error.to_string())
}
