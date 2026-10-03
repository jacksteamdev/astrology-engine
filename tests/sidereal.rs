// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

#[path = "../tools/verification/common/mod.rs"]
mod common;
use astrology_engine::{
    calculate_chart, coverage, fagan_bradley_ayanamsa, ActualHouseSystem, ChartInput, Ephemeris,
    Epoch, HouseSystem, Location, SignDivisions, ZodiacConfiguration, ZodiacReference,
};

fn zodiac(reference: ZodiacReference) -> ZodiacConfiguration {
    ZodiacConfiguration {
        reference,
        divisions: SignDivisions::Equal,
        ophiuchus: None,
    }
}

#[test]
fn sidereal_conversion_preserves_physical_declinations_and_angle_speeds() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let window = coverage(&ephemeris);
    for et in [window.lo_et, 0.0, window.hi_et] {
        for latitude in [0.0, 51.5, 80.0, -80.0] {
            for house_system in [
                HouseSystem::Equal,
                HouseSystem::WholeSign,
                HouseSystem::Placidus,
            ] {
                let input = ChartInput {
                    epoch: Epoch::from_et_seconds(et),
                    location: Location {
                        latitude,
                        longitude: 0.0,
                    },
                    house_system,
                    zodiac: zodiac(ZodiacReference::Tropical),
                };
                let tropical = calculate_chart(&ephemeris, input);
                let sidereal = calculate_chart(
                    &ephemeris,
                    ChartInput {
                        zodiac: zodiac(ZodiacReference::FaganBradley),
                        ..input
                    },
                );
                let (tropical, sidereal) = match (tropical, sidereal) {
                    (Ok(t), Ok(s)) => (t, s),
                    (Err(t), Err(s)) => {
                        assert!(t.to_string().contains("overlap"));
                        assert_eq!(t.to_string(), s.to_string());
                        continue;
                    }
                    other => panic!("Reference validation mismatch: {other:?}"),
                };
                assert_eq!(sidereal.bodies.len(), 19);
                for (index, (t, s)) in tropical.bodies.iter().zip(&sidereal.bodies).enumerate() {
                    assert_eq!(t.values.name, s.values.name);
                    assert_eq!(t.values.declination, s.values.declination);
                    assert_eq!(
                        s.values.longitude,
                        (t.values.longitude - sidereal.reference_offset_degrees).rem_euclid(360.0)
                    );
                    if index >= 15 {
                        assert_eq!(s.values.speed, 0.0);
                    }
                }
                if house_system == HouseSystem::WholeSign {
                    assert_eq!(
                        sidereal.cusps[0],
                        (sidereal.bodies[15].values.longitude / 30.0).floor() * 30.0
                    );
                    assert!(sidereal.cusps.iter().all(|c| c % 30.0 == 0.0));
                    assert_eq!(sidereal.house_system, ActualHouseSystem::WholeSign);
                    assert_eq!(tropical.cusps.len(), 12);
                } else {
                    for (t, s) in tropical.cusps.iter().zip(&sidereal.cusps) {
                        assert_eq!(
                            *s,
                            (t - sidereal.reference_offset_degrees).rem_euclid(360.0)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn speeds_include_the_changing_origin() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let input = ChartInput {
        epoch: Epoch::from_et_seconds(0.0),
        location: Location {
            latitude: 0.0,
            longitude: 0.0,
        },
        house_system: HouseSystem::Placidus,
        zodiac: zodiac(ZodiacReference::FaganBradley),
    };
    let chart = calculate_chart(&ephemeris, input).unwrap();
    assert_eq!(chart.house_system, ActualHouseSystem::Placidus);
    let before = calculate_chart(
        &ephemeris,
        ChartInput {
            epoch: Epoch::from_et_seconds(-43200.0),
            ..input
        },
    )
    .unwrap();
    let after = calculate_chart(
        &ephemeris,
        ChartInput {
            epoch: Epoch::from_et_seconds(43200.0),
            ..input
        },
    )
    .unwrap();
    for i in 0..15 {
        let rate = (after.bodies[i].values.longitude - before.bodies[i].values.longitude + 180.0)
            .rem_euclid(360.0)
            - 180.0;
        assert!((chart.bodies[i].values.speed - rate).abs() < 1e-12);
    }
    for utc in ["1700-01-01T00:00:00Z", "2300-01-01T00:00:00Z"] {
        assert!(fagan_bradley_ayanamsa(utc.parse().unwrap()).is_err());
        assert!(calculate_chart(
            &ephemeris,
            ChartInput {
                epoch: utc.parse().unwrap(),
                ..input
            }
        )
        .is_err());
    }
    for utc in ["1800-01-02T00:00:00 TT", "2199-12-31T00:00:00 TT"] {
        assert!(fagan_bradley_ayanamsa(utc.parse().unwrap()).is_ok());
    }
}

#[test]
fn all_references_report_valid_polar_fallback_and_reject_overlapping_cusps() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    for reference in [
        ZodiacReference::Tropical,
        ZodiacReference::FaganBradley,
        ZodiacReference::TrueSky,
    ] {
        let input = ChartInput {
            epoch: "2000-01-01T12:00:00Z".parse().unwrap(),
            location: Location {
                latitude: 80.0,
                longitude: 90.0,
            },
            house_system: HouseSystem::Placidus,
            zodiac: zodiac(reference),
        };
        let valid = calculate_chart(&ephemeris, input).unwrap();
        assert_eq!(valid.house_system, ActualHouseSystem::Porphyry);
        let sweep: f64 = (0..12)
            .map(|i| (valid.cusps[(i + 1) % 12] - valid.cusps[i]).rem_euclid(360.0))
            .sum();
        assert!((sweep - 360.0).abs() < 1e-8);
        let invalid = ChartInput {
            location: Location {
                latitude: 80.0,
                longitude: 20.0,
            },
            ..input
        };
        assert!(calculate_chart(&ephemeris, invalid)
            .unwrap_err()
            .to_string()
            .contains("Choose Equal or Whole Sign"));
        for house_system in [HouseSystem::Equal, HouseSystem::WholeSign] {
            assert!(calculate_chart(
                &ephemeris,
                ChartInput {
                    house_system,
                    ..invalid
                }
            )
            .is_ok());
        }
    }
}
