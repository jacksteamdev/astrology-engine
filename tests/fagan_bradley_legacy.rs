// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use astrology_engine::{calculate_chart, ChartInput, Ephemeris, Location, ZodiacConfiguration};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn compare(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            let delta = (a.as_f64().unwrap() - b.as_f64().unwrap()).abs();
            assert!(
                delta <= 1e-12,
                "numeric regression: {a} != {b} (delta {delta})"
            );
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                compare(a, b);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
            for (key, b) in b {
                compare(&a[key], b);
            }
        }
        _ => assert_eq!(actual, expected),
    }
}

#[test]
fn comparison_bounds_rounding_without_hiding_regressions() {
    compare(
        &json!({"cusps": [29.288207831506444]}),
        &json!({"cusps": [29.288207831506437]}),
    );
    assert!(std::panic::catch_unwind(|| compare(&json!(1.0), &json!(1.00000001))).is_err());
    assert!(
        std::panic::catch_unwind(|| compare(&json!(["SUN", "MOON"]), &json!(["MOON", "SUN"])))
            .is_err()
    );
}

#[test]
#[ignore = "requires FAGAN_BRADLEY_DATASET with the historical website dataset"]
fn historical_fagan_bradley_charts() {
    let fixture = include_bytes!("fixtures/fagan-bradley-71350f0.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(fixture)),
        "d70c88d0452efce0f0725a8baf8109992915b4221beed2ec1d75840bf3eb3c8a"
    );
    let source: Value = serde_json::from_slice(fixture).unwrap();
    assert_eq!(
        source["engineRevision"],
        "71350f0889c3b5e1585935d2017c97efe0bb5848"
    );
    let bytes =
        std::fs::read(std::env::var("FAGAN_BRADLEY_DATASET").expect("set FAGAN_BRADLEY_DATASET"))
            .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        source["datasetSha256"].as_str().unwrap()
    );
    let ephemeris = Ephemeris::parse(bytes).unwrap();
    let cases = source["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        let input = &case["input"];
        let chart = calculate_chart(
            &ephemeris,
            ChartInput {
                epoch: input[0].as_str().unwrap().parse().unwrap(),
                location: Location {
                    latitude: input[1].as_f64().unwrap(),
                    longitude: input[2].as_f64().unwrap(),
                },
                house_system: serde_json::from_value(input[3].clone()).unwrap(),
                zodiac: serde_json::from_value::<ZodiacConfiguration>(
                    json!({"reference":"fagan-bradley", "divisions":"equal", "ophiuchus":null}),
                )
                .unwrap(),
            },
        )
        .unwrap();
        let bodies: Vec<_> = chart
            .bodies
            .iter()
            .map(|body| {
                json!({
                    "name": body.values.name, "longitude": body.values.longitude,
                    "speed": body.values.speed, "declination": body.values.declination,
                })
            })
            .collect();
        compare(
            &json!({"bodies": bodies, "cusps": chart.cusps, "ayanamsa_degrees": chart.reference_offset_degrees, "house_system": chart.house_system}),
            &case["chart"],
        );
    }
    println!("Compared 8 historical Fagan-Bradley charts at 1e-12 degree tolerance");
}
