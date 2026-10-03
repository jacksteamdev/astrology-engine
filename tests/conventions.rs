// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use astrology_engine::{
    assign_sign, calculate_chart, effective_configuration, true_sky_offset, whole_sign_cusps,
    zodiac_sectors, ActualHouseSystem, ChartInput, Ephemeris, Epoch, HouseSystem, Location,
    Ophiuchus, SignDivisions, SpeedReference, ZodiacConfiguration, ZodiacReference,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[path = "../tools/verification/common/mod.rs"]
mod common;

fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/conventions.json")).unwrap()
}
fn config(value: &Value) -> ZodiacConfiguration {
    serde_json::from_value(value.clone()).unwrap()
}
#[track_caller]
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-12,
        "{actual:.17} != {expected:.17}"
    );
}

#[test]
fn source_generated_sectors_boundaries_and_projections() {
    let source = reference();
    for case in source["sectors"].as_array().unwrap() {
        let sectors = zodiac_sectors(config(&case["config"])).unwrap();
        assert_eq!(
            sectors,
            serde_json::from_value::<Vec<astrology_engine::SignSector>>(case["sectors"].clone())
                .unwrap()
        );
    }
    for case in source["boundaries"].as_array().unwrap() {
        let config = config(&case["config"]);
        let longitude = case["longitude"].as_f64().unwrap();
        assert_eq!(
            assign_sign(longitude, config).unwrap(),
            serde_json::from_value::<astrology_engine::SignPosition>(case["assignment"].clone())
                .unwrap()
        );
        assert_eq!(
            whole_sign_cusps(longitude, config).unwrap(),
            serde_json::from_value::<[f64; 12]>(case["cusps"].clone()).unwrap()
        );
    }
    for case in source["projections"].as_array().unwrap() {
        let offset = true_sky_offset(case["utc"].as_str().unwrap().parse().unwrap()).unwrap();
        close(offset, case["offset"].as_f64().unwrap());
        close(
            (case["longitude"].as_f64().unwrap() - offset).rem_euclid(360.0),
            case["projected"].as_f64().unwrap(),
        );
    }
}

#[test]
fn source_date_limits_are_inclusive() {
    for case in reference()["date_limits"].as_array().unwrap() {
        let milliseconds = case["unix_ms"].as_f64().unwrap();
        let epoch = Epoch::from_unix_milliseconds(milliseconds);
        let offset = true_sky_offset(epoch).unwrap();
        assert_eq!(offset, case["offset"].as_f64().unwrap());
        assert_eq!(
            (-offset).rem_euclid(360.0),
            case["projected"].as_f64().unwrap()
        );
        let outside = Epoch::from_unix_milliseconds(milliseconds + milliseconds.signum());
        assert!(true_sky_offset(outside).is_err());
    }
}

#[test]
fn invalid_inputs_and_explicit_effective_configuration() {
    let c = ZodiacConfiguration {
        reference: ZodiacReference::TrueSky,
        divisions: SignDivisions::Constellation,
        ophiuchus: Some(Ophiuchus::Enabled),
    };
    for longitude in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(assign_sign(longitude, c).is_err());
        assert!(whole_sign_cusps(longitude, c).is_err());
    }
    for reference in [ZodiacReference::Tropical, ZodiacReference::FaganBradley] {
        assert!(effective_configuration(ZodiacConfiguration { reference, ..c }).is_err());
    }
    assert!(effective_configuration(ZodiacConfiguration {
        ophiuchus: None,
        ..c
    })
    .is_err());
    for ophiuchus in [None, Some(Ophiuchus::Enabled), Some(Ophiuchus::Disabled)] {
        let equal = ZodiacConfiguration {
            divisions: SignDivisions::Equal,
            ophiuchus,
            ..c
        };
        assert_eq!(effective_configuration(equal).unwrap().ophiuchus, None);
        let json = serde_json::to_string(&equal).unwrap();
        assert_eq!(
            serde_json::from_str::<ZodiacConfiguration>(&json).unwrap(),
            equal
        );
    }
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    for latitude in [f64::NAN, f64::INFINITY, -91.0, 91.0] {
        assert!(calculate_chart(
            &ephemeris,
            ChartInput {
                epoch: Epoch::from_et_seconds(0.0),
                location: Location {
                    latitude,
                    longitude: 0.0
                },
                house_system: HouseSystem::Equal,
                zodiac: c,
            },
        )
        .is_err());
    }
    for et in [-50_000_001.0, 50_000_001.0] {
        assert!(calculate_chart(
            &ephemeris,
            ChartInput {
                epoch: Epoch::from_et_seconds(et),
                location: Location {
                    latitude: 0.0,
                    longitude: 0.0
                },
                house_system: HouseSystem::Equal,
                zodiac: c,
            },
        )
        .is_err());
    }
}

#[test]
fn all_references_return_complete_house_and_sign_metadata() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    for reference in [
        ZodiacReference::Tropical,
        ZodiacReference::FaganBradley,
        ZodiacReference::TrueSky,
    ] {
        for house_system in [
            HouseSystem::Equal,
            HouseSystem::WholeSign,
            HouseSystem::Placidus,
        ] {
            let zodiac = ZodiacConfiguration {
                reference,
                divisions: SignDivisions::Equal,
                ophiuchus: None,
            };
            let chart = calculate_chart(
                &ephemeris,
                ChartInput {
                    epoch: "2000-01-01T12:00:00Z".parse().unwrap(),
                    location: Location {
                        latitude: 80.0,
                        longitude: 90.0,
                    },
                    house_system,
                    zodiac,
                },
            )
            .unwrap();
            assert_eq!(chart.configuration, zodiac);
            assert_eq!(chart.cusps.len(), 12);
            assert_eq!(chart.bodies.len(), 19);
            assert_eq!(chart.requested_house_system, house_system);
            assert_eq!(
                chart.speed_reference,
                if reference == ZodiacReference::FaganBradley {
                    SpeedReference::SelectedReference
                } else {
                    SpeedReference::Tropical
                }
            );
            assert_eq!(
                chart.house_system,
                match house_system {
                    HouseSystem::Equal => ActualHouseSystem::Equal,
                    HouseSystem::WholeSign => ActualHouseSystem::WholeSign,
                    HouseSystem::Placidus => ActualHouseSystem::Porphyry,
                }
            );
            for entry in &chart.bodies {
                assert_eq!(
                    entry.sign,
                    assign_sign(entry.values.longitude, zodiac).unwrap()
                );
            }
            assert_eq!(
                chart.house_sectors.is_some(),
                house_system == HouseSystem::WholeSign
            );
            let json = serde_json::to_value(&chart).unwrap();
            assert!(json["bodies"][0].get("longitude").is_some());
            assert!(json["bodies"][0].get("values").is_none());
            assert!(json["bodies"][0].get("sign").is_some());
        }
    }
}

#[test]
#[ignore = "requires CONVENTIONS_DATASET with the captured source dataset"]
fn source_generated_full_charts() {
    let path = std::env::var("CONVENTIONS_DATASET").expect("set CONVENTIONS_DATASET");
    let bytes = std::fs::read(path).unwrap();
    let source = reference();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        source["metadata"]["dataset_sha256"].as_str().unwrap()
    );
    let ephemeris = Ephemeris::parse(bytes).unwrap();
    assert_eq!(
        ephemeris.domain_et(),
        serde_json::from_value::<(f64, f64)>(source["metadata"]["domain_et"].clone()).unwrap()
    );
    let mut rejected = 0;
    let mut accepted = 0;
    for case in source["charts"].as_array().unwrap() {
        let input = &case["input"];
        let house_system: HouseSystem = serde_json::from_value(input["house"].clone()).unwrap();
        let config = config(&input["config"]);
        let input = ChartInput {
            epoch: if input["use_et"] == true {
                Epoch::from_et_seconds(input["et"].as_f64().unwrap())
            } else {
                input["utc"].as_str().unwrap().parse().unwrap()
            },
            location: Location {
                latitude: input["latitude"].as_f64().unwrap(),
                longitude: input["longitude"].as_f64().unwrap(),
            },
            house_system,
            zodiac: config,
        };
        let result = calculate_chart(&ephemeris, input);
        let expected: Vec<f64> = serde_json::from_value(case["cusps"].clone()).unwrap();
        let sweep: f64 = (0..12)
            .map(|i| (expected[(i + 1) % 12] - expected[i]).rem_euclid(360.0))
            .sum();
        if (sweep - 360.0).abs() > 1e-8 {
            assert!(result.unwrap_err().to_string().contains("overlap"));
            rejected += 1;
            continue;
        }
        let chart = result.unwrap();
        accepted += 1;
        close(
            chart.reference_offset_degrees,
            case["offset"].as_f64().unwrap(),
        );
        assert_eq!(chart.speed_reference, SpeedReference::Tropical);
        assert_eq!(chart.configuration, config);
        for (actual, expected) in chart.cusps.into_iter().zip(expected) {
            close(actual, expected);
        }
        let expected = case["bodies"].as_array().unwrap();
        assert_eq!(chart.bodies.len(), expected.len());
        for (actual, expected) in chart.bodies.iter().zip(expected) {
            assert_eq!(actual.values.name, expected["name"]);
            close(
                actual.values.longitude,
                expected["longitude"].as_f64().unwrap(),
            );
            close(actual.values.speed, expected["speed"].as_f64().unwrap());
            close(
                actual.values.declination,
                expected["declination"].as_f64().unwrap(),
            );
            assert_eq!(
                serde_json::to_value(actual.sign.sign).unwrap(),
                expected["sign"]["sign"]
            );
            close(
                actual.sign.degrees_in_sign,
                expected["sign"]["degrees_in_sign"].as_f64().unwrap(),
            );
            close(
                actual.sign.width_degrees,
                expected["sign"]["width_degrees"].as_f64().unwrap(),
            );
        }
        if let Some(table) = chart.house_sectors {
            assert_eq!(table.len(), 12);
            assert!(table
                .iter()
                .all(|s| s.sign != astrology_engine::Sign::Ophiuchus));
        }
    }
    println!(
        "Compared {accepted} source charts; rejected {rejected} overlapping source house sets"
    );
    assert!(accepted > 0 && rejected > 0);
}
