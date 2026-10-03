// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

//! One chart pipeline with explicit zodiac and house conventions.
use crate::astro::houses::{
    angle_longitudes, compute_angles, house_set, placidus_cusps_with_fallback,
};
use crate::astro::AstroTime;
use crate::conventions::{checked_cusps, house_sectors, normalize, position, sectors};
use crate::{
    effective_configuration, fagan_bradley_ayanamsa, true_sky_offset, whole_sign_cusps,
    ActualHouseSystem, Body, CalculationError, ChartBody, Ephemeris, Epoch, HouseSystem, Location,
    SignPosition, SignSector, SpeedReference, ZodiacConfiguration, ZodiacReference,
    CONVENTION_REVISION,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct ChartInput {
    pub epoch: Epoch,
    pub location: Location,
    pub house_system: HouseSystem,
    pub zodiac: ZodiacConfiguration,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChartEntry {
    #[serde(flatten)]
    pub values: ChartBody,
    pub sign: SignPosition,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChartValues {
    pub configuration: ZodiacConfiguration,
    pub convention_revision: &'static str,
    pub reference_offset_degrees: f64,
    /// True Sky retains tropical speeds; Fagan–Bradley corrects them.
    /// Angle speeds remain zero (not calculated) for every reference.
    pub speed_reference: SpeedReference,
    pub bodies: Vec<ChartEntry>,
    pub zodiac_sectors: Vec<SignSector>,
    /// Present only for Whole Sign; may differ from the zodiac sectors.
    pub house_sectors: Option<Vec<SignSector>>,
    pub cusps: [f64; 12],
    pub requested_house_system: HouseSystem,
    pub house_system: ActualHouseSystem,
}

/// Calculate bodies, angles, signs and twelve validated house cusps.
/// The caller supplies the dataset, instant, location and all chart settings.
pub fn calculate_chart(
    ephemeris: &Ephemeris,
    input: ChartInput,
) -> Result<ChartValues, CalculationError> {
    let configuration = effective_configuration(input.zodiac)?;
    let (offset, rate, speed_reference) = match configuration.reference {
        ZodiacReference::Tropical => (0.0, 0.0, SpeedReference::Tropical),
        ZodiacReference::TrueSky => (true_sky_offset(input.epoch)?, 0.0, SpeedReference::Tropical),
        ZodiacReference::FaganBradley => (
            fagan_bradley_ayanamsa(input.epoch)?,
            crate::sidereal::ayanamsa_daily_rate(input.epoch),
            SpeedReference::SelectedReference,
        ),
    };
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
    bodies.extend(
        angle_names
            .iter()
            .zip(angle_longitudes(angles))
            .map(|(body, longitude)| ChartBody {
                name: body.wire_name().to_string(),
                longitude,
                speed: 0.0,
                declination: 0.0,
            }),
    );
    let bodies: Vec<_> = bodies
        .into_iter()
        .enumerate()
        .map(|(index, body)| ChartBody {
            longitude: if configuration.reference == ZodiacReference::Tropical {
                body.longitude
            } else {
                (body.longitude - offset).rem_euclid(360.0)
            },
            speed: if configuration.reference == ZodiacReference::FaganBradley && index < 15 {
                body.speed - rate
            } else {
                body.speed
            },
            ..body
        })
        .collect();
    if bodies.iter().any(|body| {
        !body.longitude.is_finite() || !body.speed.is_finite() || !body.declination.is_finite()
    }) {
        return Err(CalculationError::InvalidInput(
            "Chart contains non-finite body values",
        ));
    }
    let (cusps, actual) = match input.house_system {
        HouseSystem::WholeSign => (
            whole_sign_cusps(bodies[15].longitude, configuration)?.to_vec(),
            ActualHouseSystem::WholeSign,
        ),
        HouseSystem::Equal => (
            house_set(HouseSystem::Equal, angles, time, lat, lon)
                .cusps
                .expect("Equal houses supply cusps")
                .into_iter()
                .map(|c| {
                    if configuration.reference == ZodiacReference::Tropical {
                        c
                    } else {
                        (c - offset).rem_euclid(360.0)
                    }
                })
                .collect(),
            ActualHouseSystem::Equal,
        ),
        HouseSystem::Placidus => {
            let (cusps, fallback) = placidus_cusps_with_fallback(angles, time, lat, lon);
            (
                cusps
                    .into_iter()
                    .map(|c| {
                        if configuration.reference == ZodiacReference::Tropical {
                            c
                        } else {
                            (c - offset).rem_euclid(360.0)
                        }
                    })
                    .collect(),
                if fallback {
                    ActualHouseSystem::Porphyry
                } else {
                    ActualHouseSystem::Placidus
                },
            )
        }
    };
    let table = sectors(configuration);
    Ok(ChartValues {
        configuration,
        convention_revision: CONVENTION_REVISION,
        reference_offset_degrees: offset,
        speed_reference,
        bodies: bodies
            .into_iter()
            .map(|values| ChartEntry {
                sign: position(
                    normalize(values.longitude).expect("body longitudes are finite"),
                    &table,
                ),
                values,
            })
            .collect(),
        zodiac_sectors: table,
        house_sectors: (input.house_system == HouseSystem::WholeSign)
            .then(|| house_sectors(configuration)),
        cusps: checked_cusps(cusps)?,
        requested_house_system: input.house_system,
        house_system: actual,
    })
}
