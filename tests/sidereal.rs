// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

#[path = "../tools/verification/common/mod.rs"]
mod common;
use astrology_engine::{
    calculate_chart, calculate_sidereal_chart, coverage, fagan_bradley_ayanamsa, Body, ChartInput,
    Ephemeris, Epoch, HouseSystem, Location,
};

#[test]
fn sidereal_conversion_keeps_tropical_results_and_physical_declinations() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let window = coverage(&ephemeris);
    for et in [window.lo_et, 0.0, window.hi_et] {
        for latitude in [0.0, 51.5, 80.0, -80.0] {
            for system in [
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
                    house_system: system,
                };
                let tropical = calculate_chart(&ephemeris, input).unwrap();
                let result = calculate_sidereal_chart(&ephemeris, input);
                let tropical_sweep: f64 = tropical.cusps.as_ref().map_or(360.0, |cusps| {
                    (0..12)
                        .map(|i| (cusps[(i + 1) % 12] - cusps[i]).rem_euclid(360.0))
                        .sum()
                });
                if (tropical_sweep - 360.0).abs() > 1e-8 {
                    assert!(result.is_err());
                    assert_eq!(calculate_chart(&ephemeris, input).unwrap(), tropical);
                    continue;
                }
                let sidereal = result.unwrap();
                assert_eq!(calculate_chart(&ephemeris, input).unwrap(), tropical);
                assert_eq!(sidereal.bodies.len(), 19);
                for (index, (t, s)) in tropical.bodies.iter().zip(&sidereal.bodies).enumerate() {
                    assert_eq!(t.name, s.name);
                    assert_eq!(t.declination, s.declination);
                    assert_eq!(
                        s.longitude,
                        (t.longitude - sidereal.ayanamsa_degrees).rem_euclid(360.0)
                    );
                    if index >= 15 {
                        assert_eq!(s.speed, 0.0);
                    }
                }
                assert_eq!(sidereal.cusps.len(), 12);
                if system == HouseSystem::WholeSign {
                    let asc = sidereal
                        .bodies
                        .iter()
                        .find(|b| b.name == Body::AscendantSymbol.wire_name())
                        .unwrap()
                        .longitude;
                    assert_eq!(sidereal.cusps[0], (asc / 30.0).floor() * 30.0);
                    assert!(sidereal.cusps.iter().all(|cusp| cusp % 30.0 == 0.0));
                    assert_eq!(sidereal.house_system, "whole-sign");
                    assert!(tropical.cusps.is_none());
                } else {
                    for (t, s) in tropical.cusps.unwrap().iter().zip(&sidereal.cusps) {
                        assert_eq!(*s, (t - sidereal.ayanamsa_degrees).rem_euclid(360.0));
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
    };
    let chart = calculate_sidereal_chart(&ephemeris, input).unwrap();
    assert_eq!(chart.house_system, "placidus");
    let before = calculate_sidereal_chart(
        &ephemeris,
        ChartInput {
            epoch: Epoch::from_et_seconds(-43200.0),
            ..input
        },
    )
    .unwrap();
    let after = calculate_sidereal_chart(
        &ephemeris,
        ChartInput {
            epoch: Epoch::from_et_seconds(43200.0),
            ..input
        },
    )
    .unwrap();
    for i in 0..15 {
        let rate = (after.bodies[i].longitude - before.bodies[i].longitude + 180.0)
            .rem_euclid(360.0)
            - 180.0;
        assert!((chart.bodies[i].speed - rate).abs() < 1e-12);
    }
    for utc in ["1700-01-01T00:00:00Z", "2300-01-01T00:00:00Z"] {
        assert!(fagan_bradley_ayanamsa(utc.parse().unwrap()).is_err());
    }
}

#[test]
fn valid_polar_fallback_is_reported_but_overlapping_cusps_are_rejected() {
    let ephemeris = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let input = ChartInput {
        epoch: "2000-01-01T12:00:00Z".parse().unwrap(),
        location: Location {
            latitude: 80.0,
            longitude: 90.0,
        },
        house_system: HouseSystem::Placidus,
    };
    let valid = calculate_sidereal_chart(&ephemeris, input).unwrap();
    assert_eq!(valid.house_system, "porphyry");
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
    let tropical = calculate_chart(&ephemeris, invalid).unwrap();
    let cusps = tropical.cusps.unwrap();
    let sweep: f64 = (0..12)
        .map(|i| (cusps[(i + 1) % 12] - cusps[i]).rem_euclid(360.0))
        .sum();
    assert!((sweep - 1080.0).abs() < 1e-8);
    let error = calculate_sidereal_chart(&ephemeris, invalid)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Choose Equal or Whole Sign"));
    for house_system in [HouseSystem::Equal, HouseSystem::WholeSign] {
        assert!(calculate_sidereal_chart(
            &ephemeris,
            ChartInput {
                house_system,
                ..invalid
            }
        )
        .is_ok());
    }
}
