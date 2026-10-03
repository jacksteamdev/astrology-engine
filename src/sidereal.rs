// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

//! Fagan–Bradley zodiac, using the existing IAU 2006 precession rotation.
//!
//! This is the traditional longitude-offset convention on the ecliptic of
//! date, not a three-dimensional rotation onto a fixed B1950 ecliptic.
use crate::astro::frames::{e_tilt, precession_matrix};
use crate::astro::houses::{compute_angles, placidus_cusps_with_fallback};
use crate::astro::AstroTime;
use crate::{
    calculate_chart, Body, CalculationError, ChartBody, ChartInput, Ephemeris, Epoch, HouseSystem,
};
use serde::Serialize;

const J2000: f64 = 2_451_545.0;
const B1950: f64 = 2_433_282.423_459_05;
const B1950_OFFSET: f64 = 24.0 + 2.0 / 60.0 + 31.36 / 3600.0;
const MODERN_PRECESSION_CORRECTION: f64 = 0.41256 / 3600.0;
const MIN_JD: f64 = 2_378_496.5; // 1800-01-01 TT
const MAX_JD: f64 = 2_524_593.5; // 2200-01-01 TT

fn mean_ayanamsa(tt: f64) -> f64 {
    let time = AstroTime { ut: tt, tt };
    let reference = AstroTime {
        ut: B1950 - J2000,
        tt: B1950 - J2000,
    };
    // The date's equinox [1, 0, 0], carried backwards to J2000 and
    // forwards to B1950. Matrices store their column vectors as rows.
    let date_matrix = precession_matrix(time);
    let reference_matrix = precession_matrix(reference);
    let vector: [f64; 3] = core::array::from_fn(|i| {
        (0..3)
            .map(|j| reference_matrix[j][i] * date_matrix[j][0])
            .sum()
    });
    let epsilon = e_tilt(reference).mobl.to_radians();
    let longitude = (vector[1] * epsilon.cos() + vector[2] * epsilon.sin())
        .atan2(vector[0])
        .to_degrees();
    B1950_OFFSET - MODERN_PRECESSION_CORRECTION - longitude
}

fn sidereal_longitude(tropical: f64, offset: f64) -> f64 {
    (tropical - offset).rem_euclid(360.0)
}

fn whole_sign_cusps(ascendant: f64) -> Vec<f64> {
    (0..12)
        .map(|i| ((ascendant / 30.0).floor() * 30.0 + f64::from(i) * 30.0).rem_euclid(360.0))
        .collect()
}

fn true_ayanamsa(time: AstroTime) -> f64 {
    mean_ayanamsa(time.tt) + e_tilt(time).dpsi / 3600.0
}

/// Offset to subtract from the engine's true-equinoctial tropical longitudes.
/// Supported from 1800-01-01 through 2200-01-01 TT (inclusive); chart coverage
/// is additionally limited by the caller's ephemeris. The underlying mean
/// reference is B1950, 24°02′31.36″, with the published 0.41256″
/// modern-precession correction and IAU 2006 precession.
pub fn fagan_bradley_ayanamsa(epoch: Epoch) -> Result<f64, CalculationError> {
    let time = AstroTime::from_tdb(epoch);
    if !(MIN_JD..=MAX_JD).contains(&(time.tt + J2000)) {
        return Err(CalculationError::InvalidInput(
            "Fagan–Bradley is supported from 1800-01-01 through 2200-01-01 TT",
        ));
    }
    Ok(true_ayanamsa(time))
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SiderealChartValues {
    pub bodies: Vec<ChartBody>,
    pub cusps: Vec<f64>,
    pub ayanamsa_degrees: f64,
    /// Actual cusp construction, including `porphyry` when Placidus falls back.
    pub house_system: &'static str,
}

/// Calculate a Fagan–Bradley chart without changing the tropical API.
/// Equal/Placidus cusps are shifted into the sidereal zodiac; Whole Sign
/// boundaries are rebuilt from the sidereal Ascendant. Declinations remain
/// equatorial. Angle speeds retain the tropical API's zero (not calculated).
pub fn calculate_sidereal_chart(
    ephemeris: &Ephemeris,
    input: ChartInput,
) -> Result<SiderealChartValues, CalculationError> {
    let offset = fagan_bradley_ayanamsa(input.epoch)?;
    let tropical = calculate_chart(ephemeris, input)?;
    let time = AstroTime::from_tdb(input.epoch);
    // Match the engine's centered one-day speed interval and its time model.
    let half_day = hifitime::Duration::from_days(0.5);
    let rate = true_ayanamsa(AstroTime::from_tdb(input.epoch + half_day))
        - true_ayanamsa(AstroTime::from_tdb(input.epoch - half_day));
    let bodies: Vec<_> = tropical
        .bodies
        .into_iter()
        .enumerate()
        .map(|(index, body)| ChartBody {
            longitude: sidereal_longitude(body.longitude, offset),
            speed: if index < 15 {
                body.speed - rate
            } else {
                body.speed
            },
            ..body
        })
        .collect();
    let ascendant = bodies
        .iter()
        .find(|body| body.name == Body::AscendantSymbol.wire_name())
        .expect("calculate_chart appends the Ascendant")
        .longitude;
    let (cusps, house_system) = match input.house_system {
        HouseSystem::WholeSign => (whole_sign_cusps(ascendant), "whole-sign"),
        HouseSystem::Equal => (
            tropical
                .cusps
                .unwrap()
                .into_iter()
                .map(|c| sidereal_longitude(c, offset))
                .collect(),
            "equal",
        ),
        HouseSystem::Placidus => {
            let angles = compute_angles(time, input.location.latitude, input.location.longitude);
            let (cusps, fallback) = placidus_cusps_with_fallback(
                angles,
                time,
                input.location.latitude,
                input.location.longitude,
            );
            (
                cusps
                    .into_iter()
                    .map(|c| sidereal_longitude(c, offset))
                    .collect(),
                if fallback { "porphyry" } else { "placidus" },
            )
        }
    };
    // Some inherited polar Placidus/Porphyry configurations wind around the
    // zodiac three times. Do not present overlapping sectors as twelve houses.
    let widths: Vec<_> = (0..cusps.len())
        .map(|i| (cusps[(i + 1) % cusps.len()] - cusps[i]).rem_euclid(360.0))
        .collect();
    if cusps.len() != 12
        || widths
            .iter()
            .any(|width| !width.is_finite() || *width <= 0.0)
        || (widths.iter().sum::<f64>() - 360.0).abs() > 1e-8
    {
        return Err(CalculationError::InvalidInput(
            "These house cusps overlap at this time and latitude. Choose Equal or Whole Sign houses.",
        ));
    }
    Ok(SiderealChartValues {
        bodies,
        cusps,
        ayanamsa_degrees: offset,
        house_system,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraparound_and_whole_sign_boundaries() {
        let offset = 24.75;
        assert_eq!(sidereal_longitude(offset, offset), 0.0);
        assert_eq!(sidereal_longitude(offset - 0.25, offset), 359.75);
        assert_eq!(whole_sign_cusps(29.999)[0], 0.0);
        assert_eq!(whole_sign_cusps(30.0)[0], 30.0);
        assert_eq!(whole_sign_cusps(359.999)[1], 0.0);
        assert_eq!(whole_sign_cusps(0.0)[11], 330.0);
    }
}
