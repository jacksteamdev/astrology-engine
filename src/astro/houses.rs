// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use super::frames::{gast_degrees, true_obliquity};
use super::time::AstroTime;
const DEG2RAD: f64 = core::f64::consts::PI / 180.0;
const RAD2DEG: f64 = 180.0 / core::f64::consts::PI;
fn normalize360(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angles {
    pub ascendant: f64,
    pub midheaven: f64,
}
fn right_ascension_of_midheaven(time: AstroTime, longitude_deg: f64) -> f64 {
    normalize360(gast_degrees(time) + longitude_deg)
}
pub fn compute_angles(time: AstroTime, latitude_deg: f64, longitude_deg: f64) -> Angles {
    let ramc = right_ascension_of_midheaven(time, longitude_deg);
    let eps = true_obliquity(time);
    let ramc_rad = ramc * DEG2RAD;
    let eps_rad = eps * DEG2RAD;
    let lat_rad = latitude_deg * DEG2RAD;
    let midheaven = normalize360(ramc_rad.sin().atan2(ramc_rad.cos() * eps_rad.cos()) * RAD2DEG);
    let ascendant = normalize360(
        ramc_rad
            .cos()
            .atan2(-(eps_rad.sin() * lat_rad.tan() + eps_rad.cos() * ramc_rad.sin()))
            * RAD2DEG,
    );
    Angles {
        ascendant,
        midheaven,
    }
}
pub fn angle_longitudes(angles: Angles) -> [f64; 4] {
    [
        angles.ascendant,
        angles.midheaven,
        normalize360(angles.ascendant + 180.0),
        normalize360(angles.midheaven + 180.0),
    ]
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HouseSystem {
    Equal,
    WholeSign,
    Placidus,
}
#[derive(Debug, Clone, PartialEq)]
pub struct HouseSet {
    pub system: HouseSystem,
    pub ascendant: f64,
    pub midheaven: f64,
    pub cusps: Option<Vec<f64>>,
}
fn equal_cusps(ascendant: f64) -> Vec<f64> {
    (0..12)
        .map(|n| normalize360(ascendant + f64::from(n) * 30.0))
        .collect()
}
pub fn house_set(
    system: HouseSystem,
    angles: Angles,
    time: AstroTime,
    latitude_deg: f64,
    longitude_deg: f64,
) -> HouseSet {
    match system {
        HouseSystem::Equal => HouseSet {
            system,
            ascendant: angles.ascendant,
            midheaven: angles.midheaven,
            cusps: Some(equal_cusps(angles.ascendant)),
        },
        HouseSystem::WholeSign => HouseSet {
            system,
            ascendant: angles.ascendant,
            midheaven: angles.midheaven,
            cusps: None,
        },
        HouseSystem::Placidus => {
            let cusps = placidus_cusps(angles, time, latitude_deg, longitude_deg);
            HouseSet {
                system,
                ascendant: angles.ascendant,
                midheaven: angles.midheaven,
                cusps: Some(cusps),
            }
        }
    }
}
fn right_ascension_to_longitude(ra_deg: f64, eps_rad: f64) -> f64 {
    let ra = ra_deg * DEG2RAD;
    normalize360((ra.sin() / eps_rad.cos()).atan2(ra.cos()) * RAD2DEG)
}
fn trisect_quadrant(start: f64, end: f64) -> [f64; 3] {
    let span = normalize360(end - start);
    [
        start,
        normalize360(start + span / 3.0),
        normalize360(start + (2.0 * span) / 3.0),
    ]
}
fn porphyry_cusps(ascendant: f64, midheaven: f64) -> Vec<f64> {
    let descendant = normalize360(ascendant + 180.0);
    let imum_coeli = normalize360(midheaven + 180.0);
    let [c1, c2, c3] = trisect_quadrant(ascendant, imum_coeli);
    let [c4, c5, c6] = trisect_quadrant(imum_coeli, descendant);
    let [c7, c8, c9] = trisect_quadrant(descendant, midheaven);
    let [c10, c11, c12] = trisect_quadrant(midheaven, ascendant);
    vec![c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12]
}
struct CuspIteration {
    base: f64,
    fraction: f64,
    term: f64,
    arg_sign: f64,
    seed: f64,
}
fn placidus_cusp_ra(it: &CuspIteration, tan_lat: f64, tan_eps: f64) -> Option<f64> {
    let mut ra = it.seed;
    for _ in 0..200 {
        let arg = it.arg_sign * (ra * DEG2RAD).sin() * tan_lat * tan_eps;
        if arg.abs() > 1.0 {
            return None;
        }
        let next = it.base + it.term * it.fraction * (arg.acos() * RAD2DEG);
        if (next - ra).abs() < 1e-12 {
            return Some(next);
        }
        ra = next;
    }
    None
}
pub fn placidus_cusps(
    angles: Angles,
    time: AstroTime,
    latitude_deg: f64,
    longitude_deg: f64,
) -> Vec<f64> {
    placidus_cusps_with_fallback(angles, time, latitude_deg, longitude_deg).0
}

pub(crate) fn placidus_cusps_with_fallback(
    angles: Angles,
    time: AstroTime,
    latitude_deg: f64,
    longitude_deg: f64,
) -> (Vec<f64>, bool) {
    let Angles {
        ascendant,
        midheaven,
    } = angles;
    let ramc = right_ascension_of_midheaven(time, longitude_deg);
    let eps_rad = true_obliquity(time) * DEG2RAD;
    let tan_lat = (latitude_deg * DEG2RAD).tan();
    let tan_eps = eps_rad.tan();
    let it_c11 = CuspIteration {
        base: ramc,
        fraction: 1.0 / 3.0,
        term: 1.0,
        arg_sign: -1.0,
        seed: ramc + 30.0,
    };
    let it_c12 = CuspIteration {
        base: ramc,
        fraction: 2.0 / 3.0,
        term: 1.0,
        arg_sign: -1.0,
        seed: ramc + 60.0,
    };
    let it_c2 = CuspIteration {
        base: ramc + 180.0,
        fraction: 2.0 / 3.0,
        term: -1.0,
        arg_sign: 1.0,
        seed: ramc + 120.0,
    };
    let it_c3 = CuspIteration {
        base: ramc + 180.0,
        fraction: 1.0 / 3.0,
        term: -1.0,
        arg_sign: 1.0,
        seed: ramc + 150.0,
    };
    let ra11 = placidus_cusp_ra(&it_c11, tan_lat, tan_eps);
    let ra12 = placidus_cusp_ra(&it_c12, tan_lat, tan_eps);
    let ra2 = placidus_cusp_ra(&it_c2, tan_lat, tan_eps);
    let ra3 = placidus_cusp_ra(&it_c3, tan_lat, tan_eps);
    let (ra11, ra12, ra2, ra3) = match (ra11, ra12, ra2, ra3) {
        (Some(a), Some(b), Some(c), Some(d)) => (a, b, c, d),
        _ => return (porphyry_cusps(ascendant, midheaven), true),
    };
    let cusp11 = right_ascension_to_longitude(ra11, eps_rad);
    let cusp12 = right_ascension_to_longitude(ra12, eps_rad);
    let cusp2 = right_ascension_to_longitude(ra2, eps_rad);
    let cusp3 = right_ascension_to_longitude(ra3, eps_rad);
    (
        vec![
            ascendant,
            cusp2,
            cusp3,
            normalize360(midheaven + 180.0),
            normalize360(cusp11 + 180.0),
            normalize360(cusp12 + 180.0),
            normalize360(ascendant + 180.0),
            normalize360(cusp2 + 180.0),
            normalize360(cusp3 + 180.0),
            midheaven,
            cusp11,
            cusp12,
        ],
        false,
    )
}
