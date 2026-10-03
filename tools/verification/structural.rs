// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

mod common;
use astrology_engine::{
    calculate_chart, coverage, find_sun_crossing, BlobError, Body, CalculationError, ChartInput,
    Direction, Ephemeris, Epoch, EvalError, HouseSystem, Location, SeriesBody, SignDivisions,
    SunSearchInput, ZodiacConfiguration, ZodiacReference,
};

fn manufactured_polynomials_keep_chart_relationships_and_house_options() {
    let eph = Ephemeris::parse(common::polynomial_blob()).unwrap();
    for system in [
        HouseSystem::Equal,
        HouseSystem::Placidus,
        HouseSystem::WholeSign,
    ] {
        let chart = calculate_chart(
            &eph,
            ChartInput {
                epoch: Epoch::from_et_seconds(0.0),
                location: Location {
                    latitude: 0.0,
                    longitude: 0.0,
                },
                house_system: system,
                zodiac: ZodiacConfiguration {
                    reference: ZodiacReference::Tropical,
                    divisions: SignDivisions::Equal,
                    ophiuchus: None,
                },
            },
        )
        .unwrap();
        assert_eq!(
            chart
                .bodies
                .iter()
                .map(|b| b.values.name.as_str())
                .collect::<Vec<_>>(),
            Body::ALL.map(Body::wire_name)
        );
        assert_eq!(
            chart.bodies[1].values.longitude,
            (chart.bodies[0].values.longitude + 180.0).rem_euclid(360.0)
        );
        assert_eq!(chart.bodies[0].values.speed, chart.bodies[1].values.speed);
        assert_eq!(
            chart.bodies[0].values.declination,
            chart.bodies[1].values.declination
        );
        assert_eq!(
            chart.bodies[3].values.declination,
            chart.bodies[4].values.declination
        );
        assert_eq!(chart.bodies[3].values.speed, chart.bodies[4].values.speed);
        assert!(chart.bodies[15..]
            .iter()
            .all(|b| b.values.speed == 0.0 && b.values.declination == 0.0));
        assert_eq!(chart.cusps.len(), 12);
        if system == HouseSystem::WholeSign {
            assert!(chart.cusps.iter().all(|c| c % 30.0 == 0.0));
        }
    }
}

fn linear_series_evaluates_endpoints_and_a_known_daily_rate() {
    let eph = Ephemeris::parse(common::polynomial_blob()).unwrap();
    assert_eq!(
        eph.longitude(SeriesBody::Sun, -100_000_000.0).unwrap(),
        45.0
    );
    assert_eq!(eph.longitude(SeriesBody::Sun, 100_000_000.0).unwrap(), 49.0);
    let place = eph
        .place(SeriesBody::Sun, Epoch::from_et_seconds(0.0))
        .unwrap();
    assert!((place.speed - 4.0 / 200_000_000.0 * 86400.0).abs() < 1e-12);
    assert!(matches!(
        eph.longitude(SeriesBody::Sun, 100_000_001.0),
        Err(EvalError::OutOfDomain { .. })
    ));
}

fn requests_use_the_published_domain_and_keep_the_search_margin_in_both_directions() {
    let eph = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let window = coverage(&eph);
    assert!(window.contains(window.lo_et));
    assert!(window.contains(window.hi_et));
    for direction in [Direction::Backward, Direction::Forward] {
        assert!(matches!(
            find_sun_crossing(
                &eph,
                SunSearchInput {
                    epoch: Epoch::from_et_seconds(window.lo_et),
                    target_longitude: 47.0,
                    direction
                }
            ),
            Err(CalculationError::OutOfCoverage { .. })
        ));
    }
    assert!(matches!(
        calculate_chart(
            &eph,
            ChartInput {
                epoch: Epoch::from_et_seconds(window.lo_et - 1.0),
                location: Location {
                    latitude: 0.0,
                    longitude: 0.0
                },
                house_system: HouseSystem::Equal,
                zodiac: ZodiacConfiguration {
                    reference: ZodiacReference::Tropical,
                    divisions: SignDivisions::Equal,
                    ophiuchus: None
                },
            }
        ),
        Err(CalculationError::OutOfCoverage { .. })
    ));
}

fn admission_rejects_nonfinite_locations_and_invalid_search_targets() {
    let eph = Ephemeris::parse(common::polynomial_blob()).unwrap();
    for latitude in [f64::NAN, f64::INFINITY, -91.0, 91.0] {
        assert!(matches!(
            calculate_chart(
                &eph,
                ChartInput {
                    epoch: Epoch::from_et_seconds(0.0),
                    location: Location {
                        latitude,
                        longitude: 0.0
                    },
                    house_system: HouseSystem::Equal,
                    zodiac: ZodiacConfiguration {
                        reference: ZodiacReference::Tropical,
                        divisions: SignDivisions::Equal,
                        ophiuchus: None
                    },
                }
            ),
            Err(CalculationError::InvalidInput(_))
        ));
    }
    for target_longitude in [f64::NAN, f64::INFINITY, -1.0, 360.0] {
        assert!(matches!(
            find_sun_crossing(
                &eph,
                SunSearchInput {
                    epoch: Epoch::from_et_seconds(0.0),
                    target_longitude,
                    direction: Direction::Forward
                }
            ),
            Err(CalculationError::InvalidInput(_))
        ));
    }
}

fn bounded_search_returns_an_instant_or_none() {
    let eph = Ephemeris::parse(common::polynomial_blob()).unwrap();
    let epoch = Epoch::from_et_seconds(0.0);
    assert!(find_sun_crossing(
        &eph,
        SunSearchInput {
            epoch,
            target_longitude: 47.01,
            direction: Direction::Forward
        }
    )
    .unwrap()
    .is_some());
    assert!(find_sun_crossing(
        &eph,
        SunSearchInput {
            epoch,
            target_longitude: 180.0,
            direction: Direction::Forward
        }
    )
    .unwrap()
    .is_none());
}

fn truncated_and_malformed_headers_fail_without_evaluation() {
    let blob = common::polynomial_blob();
    for length in [0, 7, 63, 64, 100, blob.len() - 1] {
        assert!(Ephemeris::parse(blob[..length].to_vec()).is_err());
    }
    let mut bad = blob.clone();
    bad[0] = 0;
    assert!(matches!(Ephemeris::parse(bad), Err(BlobError::BadMagic)));
    let mut bad = blob.clone();
    bad[64] = 255;
    common::refresh_table(&mut bad);
    assert!(matches!(
        Ephemeris::parse(bad),
        Err(BlobError::BadSeries(_))
    ));
    let mut bad = blob.clone();
    bad[68..72].copy_from_slice(&65_u32.to_le_bytes());
    common::refresh_table(&mut bad);
    assert!(matches!(
        Ephemeris::parse(bad),
        Err(BlobError::BadSeries(_))
    ));
    let mut bad = blob.clone();
    bad[96..104].copy_from_slice(&(blob.len() as u64).to_le_bytes());
    common::refresh_table(&mut bad);
    assert!(matches!(
        Ephemeris::parse(bad),
        Err(BlobError::BadSeries(_))
    ));
    let mut bad = blob.clone();
    bad[72..80].copy_from_slice(&f64::NAN.to_le_bytes());
    common::refresh_table(&mut bad);
    assert!(matches!(
        Ephemeris::parse(bad),
        Err(BlobError::BadSeries(_))
    ));
}

pub fn run() -> usize {
    manufactured_polynomials_keep_chart_relationships_and_house_options();
    linear_series_evaluates_endpoints_and_a_known_daily_rate();
    requests_use_the_published_domain_and_keep_the_search_margin_in_both_directions();
    admission_rejects_nonfinite_locations_and_invalid_search_targets();
    bounded_search_returns_an_instant_or_none();
    truncated_and_malformed_headers_fail_without_evaluation();
    6
}

pub fn fixture() -> Vec<u8> {
    common::polynomial_blob()
}
