// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

//! Fagan–Bradley zodiac, using the existing IAU 2006 precession rotation.
//!
//! This is the traditional longitude-offset convention on the ecliptic of
//! date, not a three-dimensional rotation onto a fixed B1950 ecliptic.
use crate::astro::frames::{e_tilt, precession_matrix};
use crate::astro::AstroTime;
use crate::{CalculationError, Epoch};

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

/// Match the engine's centered one-day speed interval and time model.
pub(crate) fn ayanamsa_daily_rate(epoch: Epoch) -> f64 {
    let half_day = hifitime::Duration::from_days(0.5);
    true_ayanamsa(AstroTime::from_tdb(epoch + half_day))
        - true_ayanamsa(AstroTime::from_tdb(epoch - half_day))
}
