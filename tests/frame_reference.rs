// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

#![cfg(feature = "generation")]

use astrology_engine::tooling::{
    frames::{e_tilt, gast_degrees, nutation_rot, precession_rot, true_obliquity, PrecessDir},
    time::AstroTime,
    vec::Vec3,
};
use astrology_engine::Epoch;

#[derive(serde::Deserialize)]
struct Fixtures {
    cases: Vec<Case>,
}
#[derive(serde::Deserialize)]
struct Case {
    utc: String,
    ae_frame: FrameReference,
}
#[derive(serde::Deserialize)]
struct FrameReference {
    mobl: f64,
    tobl: f64,
    dpsi: f64,
    deps: f64,
    gast_deg: f64,
}
fn load_fixtures() -> Fixtures {
    serde_json::from_str(include_str!("fixtures/frame_reference.json")).unwrap()
}
fn tdb_epoch(utc: &str) -> Epoch {
    Epoch::from_gregorian_str(utc).unwrap()
}

// Tolerances. The transforms should reproduce astronomy-engine to sub-µas; we
// allow a margin that still asserts a faithful port while absorbing the tiny
// TDB-vs-TT (≤1.7 ms) and DeltaT-inversion differences in how `AstroTime` is built.
const OBLIQUITY_TOL_DEG: f64 = 1.0e-7; // ~3.6e-4 arcsec
const NUTATION_TOL_ASEC: f64 = 1.0e-3;
const GAST_TOL_DEG: f64 = 1.0e-6; // ~3.6e-3 arcsec of rotation

#[test]
fn true_obliquity_matches_reference() {
    for c in load_fixtures().cases {
        let t = AstroTime::from_tdb(tdb_epoch(&c.utc));
        let got = true_obliquity(t);
        assert!(
            (got - c.ae_frame.tobl).abs() < OBLIQUITY_TOL_DEG,
            "{}: true obliquity {got} vs ref {} (Δ {:.3e}°)",
            c.utc,
            c.ae_frame.tobl,
            got - c.ae_frame.tobl
        );
    }
}

#[test]
fn nutation_series_matches_reference() {
    for c in load_fixtures().cases {
        let t = AstroTime::from_tdb(tdb_epoch(&c.utc));
        let et = e_tilt(t);
        assert!((et.mobl - c.ae_frame.mobl).abs() < OBLIQUITY_TOL_DEG);
        assert!(
            (et.dpsi - c.ae_frame.dpsi).abs() < NUTATION_TOL_ASEC,
            "{}: dpsi {} vs ref {} ({}″)",
            c.utc,
            et.dpsi,
            c.ae_frame.dpsi,
            et.dpsi - c.ae_frame.dpsi
        );
        assert!(
            (et.deps - c.ae_frame.deps).abs() < NUTATION_TOL_ASEC,
            "{}: deps {} vs ref {} ({}″)",
            c.utc,
            et.deps,
            c.ae_frame.deps,
            et.deps - c.ae_frame.deps
        );
        // Mean obliquity sanity: true = mean + deps/3600.
        assert!((et.mobl + et.deps / 3600.0 - et.tobl).abs() < 1.0e-12);
    }
}

#[test]
fn gast_matches_reference() {
    for c in load_fixtures().cases {
        let t = AstroTime::from_tdb(tdb_epoch(&c.utc));
        let got = gast_degrees(t);
        let mut d = (got - c.ae_frame.gast_deg).rem_euclid(360.0);
        if d > 180.0 {
            d -= 360.0;
        }
        assert!(
            d.abs() < GAST_TOL_DEG,
            "{}: GAST {got}° vs ref {}° (Δ {:.3e}°)",
            c.utc,
            c.ae_frame.gast_deg,
            d
        );
    }
}

#[test]
fn equation_of_equinoxes_is_consistent() {
    // ee = dpsi·cos(mean_obliq)/15 (sidereal seconds). Independent recomputation.
    for c in load_fixtures().cases {
        let t = AstroTime::from_tdb(tdb_epoch(&c.utc));
        let et = e_tilt(t);
        let expected =
            et.dpsi * (et.mobl * astrology_engine::tooling::frames::DEG2RAD).cos() / 15.0;
        assert!((et.ee - expected).abs() < 1.0e-12);
    }
}

#[test]
fn precession_round_trips_to_identity() {
    // From2000 then Into2000 must return the original vector — the matrices are
    // mutual inverses by construction.
    let t = AstroTime::from_tdb(tdb_epoch("2026-05-28T12:00:00Z"));
    let v = Vec3::new(0.3, -0.7, 0.5);
    let fwd = precession_rot(t, PrecessDir::From2000).rotate(v);
    let back = precession_rot(t, PrecessDir::Into2000).rotate(fwd);
    for (a, b) in [(back.x, v.x), (back.y, v.y), (back.z, v.z)] {
        assert!(
            (a - b).abs() < 1.0e-12,
            "precession round-trip drifted: {a} vs {b}"
        );
    }
    // And it actually rotates (not the identity): precession over 26 yr is ~0.36°.
    let moved = ((fwd.x - v.x).powi(2) + (fwd.y - v.y).powi(2) + (fwd.z - v.z).powi(2)).sqrt();
    assert!(moved > 1.0e-4, "precession produced no rotation");
}

#[test]
fn nutation_round_trips_to_identity() {
    let t = AstroTime::from_tdb(tdb_epoch("1981-08-06T05:30:00Z"));
    let v = Vec3::new(1.0, 0.0, 0.0);
    let fwd = nutation_rot(t, PrecessDir::From2000).rotate(v);
    let back = nutation_rot(t, PrecessDir::Into2000).rotate(fwd);
    for (a, b) in [(back.x, v.x), (back.y, v.y), (back.z, v.z)] {
        assert!(
            (a - b).abs() < 1.0e-12,
            "nutation round-trip drifted: {a} vs {b}"
        );
    }
}

#[test]
fn precession_at_j2000_is_near_identity() {
    // At t≈0 (2000-01-01), precession from J2000 is essentially the identity.
    let t = AstroTime::from_tdb(tdb_epoch("2000-01-01T00:00:00Z"));
    let v = Vec3::new(1.0, 0.0, 0.0);
    let fwd = precession_rot(t, PrecessDir::From2000).rotate(v);
    let moved = ((fwd.x - v.x).powi(2) + (fwd.y - v.y).powi(2) + (fwd.z - v.z).powi(2)).sqrt();
    assert!(
        moved < 1.0e-3,
        "precession at J2000 should be tiny, got {moved}"
    );
}
