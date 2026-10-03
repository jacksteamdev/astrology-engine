
thread_local! {
    static CAPTURE_INSTANT: RefCell<Option<Epoch>> = const { RefCell::new(None) };
}

fn captured_chart(chart: &Chart) -> serde_json::Value {
    serde_json::json!({
        "chart": chart,
        "bits": chart.bodies.iter().map(|b| [b.longitude, b.speed, b.declination].map(|v| format!("{:016x}", v.to_bits()))).collect::<Vec<_>>(),
        "cusp_bits": chart.cusps.as_ref().map(|v| v.iter().map(|x| format!("{:016x}", x.to_bits())).collect::<Vec<_>>())
    })
}

fn captured_error(error: ChartError) -> serde_json::Value {
    match error {
        ChartError::OutOfCoverage(message) => serde_json::json!({"status": 422, "error": message}),
        ChartError::Internal(error) => serde_json::json!({"status": 500, "error": error.to_string()}),
    }
}

async fn capture_batch(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let inputs: Vec<serde_json::Value> = req.json().await?;
    let mut results = Vec::with_capacity(inputs.len());
    for input in inputs {
        let base = if let Some(et) = input.get("et").and_then(|v| v.as_f64()) {
            Epoch::from_et_seconds(et)
        } else {
            Epoch::from_gregorian_str(input["date"].as_str().unwrap()).unwrap()
        };
        let mut warm = false;
        let mut phases = PhaseTimings;
        let value = if input["kind"] == "chart" {
            let request = ChartRequest {
                date: input.get("date").and_then(|v| v.as_str()).unwrap_or("capture-et").into(),
                location: LocationInput { latitude: input["lat"].as_f64().unwrap(), longitude: input["lon"].as_f64().unwrap() },
                house_system: input.get("house").and_then(|v| v.as_str()).map(str::to_string),
                design_lookback: None,
            };
            match build_chart(&ctx.env, &request, base, &mut warm, &mut phases).await {
                Ok(chart) => captured_chart(&chart),
                Err(error) => captured_error(error),
            }
        } else {
            CAPTURE_INSTANT.with(|slot| *slot.borrow_mut() = None);
            let direction = if input["direction"] == "forward" { Direction::Forward } else { Direction::Backward };
            match find_moment_instant(&ctx.env, Body::Sun, base, input["target"].as_f64().unwrap(), direction, &mut warm, &mut phases).await {
                Ok(moment) => {
                    let instant = CAPTURE_INSTANT.with(|slot| *slot.borrow());
                    serde_json::json!({"moment": moment, "instant": instant.map(|v| serde_json::json!({
                        "et_bits": format!("{:016x}", v.to_et_seconds().to_bits()),
                        "tai_parts": v.to_tai_duration().to_parts(),
                    }))})
                },
                Err(error) => captured_error(error),
            }
        };
        results.push(serde_json::json!({"id": input["id"], "value": value}));
    }
    Response::from_json(&results)
}

async fn validate_dataset(_: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Err(error) = ensure_ephemeris(&ctx.env).await { return response_error(error); }
    let result = with_ephemeris(|ephemeris| {
        let domain = astrology_engine::coverage(ephemeris);
        for fraction in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let epoch = Epoch::from_et_seconds(domain.lo_et + (domain.hi_et - domain.lo_et) * fraction);
            let chart = calculate_chart(ephemeris, ChartInput {
                epoch, location: Location { latitude: 45.0, longitude: -90.0 }, house_system: HouseSystem::Placidus,
            })?;
            assert_eq!(chart.bodies.len(), 19);
            assert!(chart.bodies.iter().all(|b| b.longitude.is_finite() && b.speed.is_finite() && b.declination.is_finite()));
            if fraction == 0.5 {
                for direction in [Direction::Backward, Direction::Forward] {
                    assert!(find_sun_crossing(ephemeris, SunSearchInput { epoch, direction, target_longitude: 0.0 })?.is_some());
                }
            }
        }
        Ok(())
    });
    match result {
        Ok(()) => Response::from_json(&serde_json::json!({"charts":5, "searches":2, "runtime":"wasm32-unknown-unknown/workerd"})),
        Err(error) => response_error(error),
    }
}
