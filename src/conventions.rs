//! Explicit chart conventions preserved by regression fixtures.
use crate::astro::houses::{compute_angles, placidus_cusps_with_fallback};
use crate::astro::AstroTime;
use crate::{
    calculate_chart, calculate_sidereal_chart, Body, CalculationError, ChartBody, ChartInput,
    Ephemeris, Epoch, HouseSystem,
};
use serde::{Deserialize, Serialize};

pub const CONVENTION_REVISION: &str = "astrology-engine-conventions-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZodiacReference {
    Tropical,
    FaganBradley,
    TrueSky,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignDivisions {
    Equal,
    Constellation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ophiuchus {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZodiacConfiguration {
    pub reference: ZodiacReference,
    pub divisions: SignDivisions,
    pub ophiuchus: Option<Ophiuchus>,
}

/// Equal divisions ignore the Ophiuchus toggle.
/// Constellation mode requires an explicit choice; callers own defaults.
pub fn effective_configuration(
    config: ZodiacConfiguration,
) -> Result<ZodiacConfiguration, CalculationError> {
    match (config.reference, config.divisions, config.ophiuchus) {
        (_, SignDivisions::Equal, _) => Ok(ZodiacConfiguration {
            ophiuchus: None,
            ..config
        }),
        (ZodiacReference::TrueSky, SignDivisions::Constellation, Some(_)) => Ok(config),
        (ZodiacReference::TrueSky, SignDivisions::Constellation, None) => {
            Err(CalculationError::InvalidInput(
                "Constellation divisions require an explicit Ophiuchus choice",
            ))
        }
        _ => Err(CalculationError::InvalidInput(
            "Constellation divisions require True Sky",
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Ophiuchus,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

const EQUAL: [(Sign, f64); 12] = [
    (Sign::Aries, 0.0),
    (Sign::Taurus, 30.0),
    (Sign::Gemini, 60.0),
    (Sign::Cancer, 90.0),
    (Sign::Leo, 120.0),
    (Sign::Virgo, 150.0),
    (Sign::Libra, 180.0),
    (Sign::Scorpio, 210.0),
    (Sign::Sagittarius, 240.0),
    (Sign::Capricorn, 270.0),
    (Sign::Aquarius, 300.0),
    (Sign::Pisces, 330.0),
];
const CONSTELLATION: [(Sign, f64); 13] = [
    (Sign::Aries, 0.0),
    (Sign::Taurus, 19.7286),
    (Sign::Gemini, 56.5875),
    (Sign::Cancer, 86.0412),
    (Sign::Leo, 103.19),
    (Sign::Virgo, 141.6065),
    (Sign::Libra, 191.32),
    (Sign::Scorpio, 210.1972),
    (Sign::Ophiuchus, 223.4245),
    (Sign::Sagittarius, 235.7818),
    (Sign::Capricorn, 269.2677),
    (Sign::Aquarius, 294.8435),
    (Sign::Pisces, 318.0103),
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignSector {
    pub sign: Sign,
    pub start_degrees: f64,
    pub width_degrees: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignPosition {
    pub sign: Sign,
    pub degrees_in_sign: f64,
    pub width_degrees: f64,
}

fn sectors(config: ZodiacConfiguration) -> Vec<SignSector> {
    let table: &[(Sign, f64)] = match config.divisions {
        SignDivisions::Equal => &EQUAL,
        SignDivisions::Constellation => &CONSTELLATION,
    };
    let boundaries: Vec<_> = table
        .iter()
        .copied()
        .filter(|(sign, _)| {
            *sign != Sign::Ophiuchus || config.ophiuchus != Some(Ophiuchus::Disabled)
        })
        .collect();
    boundaries
        .iter()
        .enumerate()
        .map(|(i, &(sign, start_degrees))| SignSector {
            sign,
            start_degrees,
            width_degrees: boundaries.get(i + 1).map_or(360.0, |b| b.1) - start_degrees,
        })
        .collect()
}

/// Ordered starts in the selected zodiac reference, with widths totaling 360°.
pub fn zodiac_sectors(config: ZodiacConfiguration) -> Result<Vec<SignSector>, CalculationError> {
    Ok(sectors(effective_configuration(config)?))
}

fn normalize(longitude: f64) -> Result<f64, CalculationError> {
    if !longitude.is_finite() {
        return Err(CalculationError::InvalidInput("Longitude must be finite"));
    }
    let value = longitude.rem_euclid(360.0);
    Ok(if value == 360.0 || value == 0.0 {
        0.0
    } else {
        value
    })
}

fn position(longitude: f64, table: &[SignSector]) -> SignPosition {
    let sector = table
        .iter()
        .rev()
        .find(|sector| longitude >= sector.start_degrees)
        .unwrap_or(&table[0]);
    SignPosition {
        sign: sector.sign,
        degrees_in_sign: longitude - sector.start_degrees,
        width_degrees: sector.width_degrees,
    }
}

/// Half-open sign intervals; finite longitudes wrap into [0°, 360°).
pub fn assign_sign(
    longitude: f64,
    config: ZodiacConfiguration,
) -> Result<SignPosition, CalculationError> {
    Ok(position(normalize(longitude)?, &zodiac_sectors(config)?))
}

fn house_sectors(config: ZodiacConfiguration) -> Vec<SignSector> {
    sectors(ZodiacConfiguration {
        ophiuchus: Some(Ophiuchus::Disabled),
        ..config
    })
}

fn whole_sign(ascendant: f64, table: &[SignSector]) -> [f64; 12] {
    let first = table
        .iter()
        .rposition(|sector| ascendant >= sector.start_degrees)
        .unwrap_or(0);
    core::array::from_fn(|i| table[(first + i) % 12].start_degrees)
}

/// Twelve houses; constellation houses always merge Ophiuchus into Scorpio.
pub fn whole_sign_cusps(
    ascendant: f64,
    config: ZodiacConfiguration,
) -> Result<[f64; 12], CalculationError> {
    Ok(whole_sign(
        normalize(ascendant)?,
        &house_sectors(effective_configuration(config)?),
    ))
}

/// Chimenti offset using UTC milliseconds (JavaScript Date precision).
/// There is no extra nutation term or model-date cutoff in the source. Charts
/// remain limited by the caller's ephemeris coverage and fitted speed support.
pub fn true_sky_offset(epoch: Epoch) -> Result<f64, CalculationError> {
    let milliseconds = epoch.to_unix_milliseconds().floor();
    if !milliseconds.is_finite() || milliseconds.abs() > 8.64e15 {
        return Err(CalculationError::InvalidInput(
            "True Sky epoch is outside the source Date range",
        ));
    }
    let jd = 2451545.0 + (milliseconds - 946728000000.0) / 86400000.0;
    Ok(31.21558087 + (jd - 2451545.0) * 0.000038247508)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActualHouseSystem {
    Equal,
    WholeSign,
    Placidus,
    Porphyry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeedReference {
    Tropical,
    SelectedReference,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfiguredBody {
    #[serde(flatten)]
    pub values: ChartBody,
    pub sign: SignPosition,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfiguredChartValues {
    pub configuration: ZodiacConfiguration,
    pub convention_revision: &'static str,
    pub reference_offset_degrees: f64,
    /// True Sky preserves source tropical speeds; Fagan–Bradley corrects them.
    /// The four angle speeds remain zero (not calculated) for every reference.
    pub speed_reference: SpeedReference,
    pub bodies: Vec<ConfiguredBody>,
    pub zodiac_sectors: Vec<SignSector>,
    /// Present only for Whole Sign; this table may differ from zodiac sectors.
    pub house_sectors: Option<Vec<SignSector>>,
    pub cusps: [f64; 12],
    pub requested_house_system: HouseSystem,
    pub house_system: ActualHouseSystem,
}

fn checked_cusps(cusps: Vec<f64>) -> Result<[f64; 12], CalculationError> {
    let cusps: [f64; 12] = cusps
        .try_into()
        .map_err(|_| CalculationError::InvalidInput("Expected twelve house cusps"))?;
    let widths: [f64; 12] =
        core::array::from_fn(|i| (cusps[(i + 1) % 12] - cusps[i]).rem_euclid(360.0));
    if widths
        .iter()
        .any(|width| !width.is_finite() || *width <= 0.0)
        || (widths.iter().sum::<f64>() - 360.0).abs() > 1e-8
    {
        return Err(CalculationError::InvalidInput("These house cusps overlap at this time and latitude. Choose Equal or Whole Sign houses."));
    }
    Ok(cusps)
}

/// Calculate with explicit conventions while retaining the original entry points.
pub fn calculate_configured_chart(
    ephemeris: &Ephemeris,
    input: ChartInput,
    config: ZodiacConfiguration,
) -> Result<ConfiguredChartValues, CalculationError> {
    let configuration = effective_configuration(config)?;
    let (bodies, cusps, offset, actual, speed_reference) =
        if configuration.reference == ZodiacReference::FaganBradley {
            let chart = calculate_sidereal_chart(ephemeris, input)?;
            let actual = match chart.house_system {
                "equal" => ActualHouseSystem::Equal,
                "whole-sign" => ActualHouseSystem::WholeSign,
                "porphyry" => ActualHouseSystem::Porphyry,
                _ => ActualHouseSystem::Placidus,
            };
            (
                chart.bodies,
                chart.cusps,
                chart.ayanamsa_degrees,
                actual,
                SpeedReference::SelectedReference,
            )
        } else {
            let chart = calculate_chart(ephemeris, input)?;
            let offset = if configuration.reference == ZodiacReference::TrueSky {
                true_sky_offset(input.epoch)?
            } else {
                0.0
            };
            let bodies = chart
                .bodies
                .into_iter()
                .map(|body| {
                    Ok(ChartBody {
                        longitude: normalize(body.longitude - offset)?,
                        ..body
                    })
                })
                .collect::<Result<Vec<_>, CalculationError>>()?;
            let (cusps, actual) = match input.house_system {
                HouseSystem::WholeSign => {
                    let ascendant = bodies
                        .iter()
                        .find(|body| body.name == Body::AscendantSymbol.wire_name())
                        .expect("chart appends Ascendant")
                        .longitude;
                    (
                        whole_sign_cusps(ascendant, configuration)?.to_vec(),
                        ActualHouseSystem::WholeSign,
                    )
                }
                HouseSystem::Equal => (
                    chart
                        .cusps
                        .expect("Equal houses supply cusps")
                        .into_iter()
                        .map(|c| normalize(c - offset))
                        .collect::<Result<Vec<_>, _>>()?,
                    ActualHouseSystem::Equal,
                ),
                HouseSystem::Placidus => {
                    let time = AstroTime::from_tdb(input.epoch);
                    let angles =
                        compute_angles(time, input.location.latitude, input.location.longitude);
                    let (cusps, fallback) = placidus_cusps_with_fallback(
                        angles,
                        time,
                        input.location.latitude,
                        input.location.longitude,
                    );
                    (
                        cusps
                            .into_iter()
                            .map(|c| normalize(c - offset))
                            .collect::<Result<Vec<_>, _>>()?,
                        if fallback {
                            ActualHouseSystem::Porphyry
                        } else {
                            ActualHouseSystem::Placidus
                        },
                    )
                }
            };
            (bodies, cusps, offset, actual, SpeedReference::Tropical)
        };
    if bodies.iter().any(|body| {
        !body.longitude.is_finite() || !body.speed.is_finite() || !body.declination.is_finite()
    }) {
        return Err(CalculationError::InvalidInput(
            "Chart contains non-finite body values",
        ));
    }
    let table = sectors(configuration);
    Ok(ConfiguredChartValues {
        configuration,
        convention_revision: CONVENTION_REVISION,
        reference_offset_degrees: offset,
        speed_reference,
        bodies: bodies
            .into_iter()
            .map(|values| ConfiguredBody {
                sign: position(values.longitude, &table),
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
