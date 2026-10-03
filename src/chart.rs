// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use crate::astro::houses::{angle_longitudes, compute_angles, house_set};
use crate::astro::AstroTime;
use crate::{Body, CalculationError, ChartBody, Ephemeris, Epoch, HouseSystem, Location};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct ChartInput {
    pub epoch: Epoch,
    pub location: Location,
    pub house_system: HouseSystem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartValues {
    pub bodies: Vec<ChartBody>,
    pub cusps: Option<Vec<f64>>,
}

pub fn calculate_chart(
    ephemeris: &Ephemeris,
    input: ChartInput,
) -> Result<ChartValues, CalculationError> {
    let lat = input.location.latitude;
    let lon = input.location.longitude;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(CalculationError::InvalidInput(
            "latitude must be in [-90, 90] and longitude in [-180, 180]",
        ));
    }
    crate::check_coverage(ephemeris, input.epoch.to_et_seconds(), 0.0)?;
    let mut bodies = crate::cheb::chart::chart_bodies(ephemeris, input.epoch)
        .map_err(CalculationError::Evaluation)?;
    let time = AstroTime::from_tdb(input.epoch);
    let angles = compute_angles(time, lat, lon);
    let angle_names = [
        Body::AscendantSymbol,
        Body::Midheaven,
        Body::Descendant,
        Body::ImumCoeli,
    ];
    let angle_lons = angle_longitudes(angles);
    bodies.extend(
        angle_names
            .iter()
            .zip(angle_lons)
            .map(|(body, longitude)| ChartBody {
                name: body.wire_name().to_string(),
                longitude,
                speed: 0.0,
                declination: 0.0,
            }),
    );
    let cusps = house_set(input.house_system, angles, time, lat, lon).cusps;
    Ok(ChartValues { bodies, cusps })
}
