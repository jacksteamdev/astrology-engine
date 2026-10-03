// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use dataset_builder::astro::node::ascending_node_longitude;
use dataset_builder::astro::{apparent_lon_dec, KmStateProvider};
use dataset_builder::cheb::format::{write_blob, Quantity, SeriesBody, WriteSeries};
use dataset_builder::cheb::gen::{fit_series, FitOutcome};
use dataset_builder::coverage_window::CoverageWindow;
use dataset_builder::kernel::daf_index::{
    coverage_window_for, plan_segments, read_cheby_meta, ChebyshevMeta, HEADER_BYTES,
};
use dataset_builder::kernel::store::KernelStore;
use dataset_builder::types::Body;
use hifitime::{Duration, Epoch};
const DAY_S: f64 = 86_400.0;
const GATE_LON_DEC_ARCSEC: f64 = 0.1;
const GATE_NODE_ARCSEC: f64 = 1.0;
const GUARD_LO_S: f64 = 5.0 * DAY_S;
const GUARD_HI_S: f64 = 1.0 * DAY_S;
fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
const PLANET_IDS: [i32; 12] = [10, 3, 399, 301, 1, 2, 4, 5, 6, 7, 8, 9];
struct SeriesSpec {
    body: SeriesBody,
    quantity: Quantity,
    seg_days: f64,
    degree: usize,
    gate_arcsec: f64,
}
fn prod_specs() -> Vec<SeriesSpec> {
    use Quantity::{Declination as Dec, Longitude as Lon};
    use SeriesBody::*;
    let mut specs = Vec::new();
    let mut push = |body, q, s, d, g| {
        specs.push(SeriesSpec {
            body,
            quantity: q,
            seg_days: s,
            degree: d,
            gate_arcsec: g,
        })
    };
    let g = GATE_LON_DEC_ARCSEC;
    push(Sun, Lon, 32.0, 10, g);
    push(Sun, Dec, 32.0, 8, g);
    push(Moon, Lon, 8.0, 10, g);
    push(Moon, Dec, 8.0, 10, g);
    push(Mercury, Lon, 8.0, 10, g);
    push(Mercury, Dec, 8.0, 10, g);
    push(Venus, Lon, 16.0, 10, g);
    push(Venus, Dec, 16.0, 8, g);
    push(Mars, Lon, 16.0, 10, g);
    push(Mars, Dec, 16.0, 8, g);
    push(Jupiter, Lon, 32.0, 10, g);
    push(Jupiter, Dec, 32.0, 8, g);
    push(Saturn, Lon, 32.0, 10, g);
    push(Saturn, Dec, 32.0, 8, g);
    push(Uranus, Lon, 32.0, 8, g);
    push(Uranus, Dec, 32.0, 6, g);
    push(Neptune, Lon, 32.0, 8, g);
    push(Neptune, Dec, 32.0, 6, g);
    push(Pluto, Lon, 32.0, 8, g);
    push(Pluto, Dec, 32.0, 6, g);
    push(Chiron, Lon, 32.0, 12, g);
    push(Chiron, Dec, 32.0, 10, g);
    push(Ceres, Lon, 32.0, 12, g);
    push(Ceres, Dec, 32.0, 10, g);
    push(NodeOmega, Lon, 8.0, 10, GATE_NODE_ARCSEC);
    specs
}
struct SeriesRow {
    body: SeriesBody,
    quantity: Quantity,
    seg_days: f64,
    degree: usize,
    gate_arcsec: f64,
    n_segments: usize,
    max_residual_arcsec: f64,
    max_boundary_jump_arcsec: f64,
    ladder: String,
    series_hi_et: f64,
    series: WriteSeries,
    ok: bool,
}
fn fit_with_ladder(
    spec: &SeriesSpec,
    t0: f64,
    sampling_hi: f64,
    oracle: &dyn Fn(f64) -> f64,
) -> SeriesRow {
    let steps = [
        (spec.seg_days, spec.degree),
        (spec.seg_days, spec.degree + 2),
        (spec.seg_days / 2.0, spec.degree),
        (spec.seg_days / 2.0, spec.degree + 2),
    ];
    let mut tried: Vec<String> = Vec::new();
    let mut last: Option<(FitOutcome, f64, usize)> = None;
    for (s, d) in steps {
        let outcome = fit_series(
            spec.body,
            spec.quantity,
            t0,
            sampling_hi,
            s * DAY_S,
            d,
            oracle,
        );
        tried.push(format!("{s}d/deg{d}→{:.4}″", outcome.max_residual_arcsec));
        let ok = outcome.max_residual_arcsec <= spec.gate_arcsec;
        last = Some((outcome, s, d));
        if ok {
            break;
        }
    }
    let (outcome, s, d) = last.expect("ladder ran at least once");
    let ok = outcome.max_residual_arcsec <= spec.gate_arcsec;
    SeriesRow {
        body: spec.body,
        quantity: spec.quantity,
        seg_days: s,
        degree: d,
        gate_arcsec: spec.gate_arcsec,
        n_segments: outcome.n_segments,
        max_residual_arcsec: outcome.max_residual_arcsec,
        max_boundary_jump_arcsec: outcome.max_boundary_jump_arcsec,
        ladder: tried.join(" ; "),
        series_hi_et: t0 + outcome.n_segments as f64 * s * DAY_S,
        series: outcome.series,
        ok,
    }
}
fn is_asteroid(body: SeriesBody) -> bool {
    matches!(body, SeriesBody::Chiron | SeriesBody::Ceres)
}
fn body_of(series_body: SeriesBody) -> Body {
    match series_body {
        SeriesBody::Sun => Body::Sun,
        SeriesBody::Moon => Body::Moon,
        SeriesBody::Mercury => Body::Mercury,
        SeriesBody::Venus => Body::Venus,
        SeriesBody::Mars => Body::Mars,
        SeriesBody::Jupiter => Body::Jupiter,
        SeriesBody::Saturn => Body::Saturn,
        SeriesBody::Uranus => Body::Uranus,
        SeriesBody::Neptune => Body::Neptune,
        SeriesBody::Pluto => Body::Pluto,
        SeriesBody::Chiron => Body::Chiron,
        SeriesBody::Ceres => Body::Ceres,
        SeriesBody::NodeOmega => Body::Moon,
    }
}
fn fit(args: &[String]) {
    use dataset_builder::astro::DualStateProvider;
    use sha2::{Digest, Sha256};
    use std::cell::RefCell;
    use std::collections::HashMap;
    let de440s = flag_value(args, "--de440s").expect("required --de440s");
    let ceres = flag_value(args, "--ceres").expect("required --ceres");
    let chiron = flag_value(args, "--chiron").expect("required --chiron");
    let out = flag_value(args, "--out").expect("required --out");
    let report_path = flag_value(args, "--report").expect("required --report");
    let manifest_out = flag_value(args, "--manifest").expect("required --manifest");
    let fingerprint = flag_value(args, "--fingerprint")
        .map(|h| u32::from_str_radix(h.trim_start_matches("0x"), 16).expect("hex fingerprint"))
        .unwrap_or(0);
    let planet_bytes = std::fs::read(&de440s).unwrap_or_else(|e| panic!("read {de440s}: {e}"));
    let ceres_bytes = std::fs::read(&ceres).unwrap_or_else(|e| panic!("read {ceres}: {e}"));
    let chiron_bytes = std::fs::read(&chiron).unwrap_or_else(|e| panic!("read {chiron}: {e}"));
    let true_window = |bytes: &[u8], ids: &[i32]| -> CoverageWindow {
        let header_len = (HEADER_BYTES as usize).min(bytes.len());
        let plans = plan_segments(&bytes[..header_len], ids).expect("plan segments");
        let metas: Vec<ChebyshevMeta> = plans
            .iter()
            .map(|p| read_cheby_meta(&bytes[..p.byte_end() as usize]).expect("cheby trailer"))
            .collect();
        coverage_window_for(&metas).expect("kernel segments share a window")
    };
    let planet_win = true_window(&planet_bytes, &PLANET_IDS);
    let ceres_win = true_window(&ceres_bytes, &[2_000_001]);
    let chiron_win = true_window(&chiron_bytes, &[2_002_060]);
    let lo = planet_win.lo_et.max(ceres_win.lo_et).max(chiron_win.lo_et) + 1.0 * DAY_S;
    let hi = planet_win.hi_et.min(ceres_win.hi_et).min(chiron_win.hi_et) - 1.0 * DAY_S;
    let days = (hi - lo) / DAY_S;
    println!("fit span: {days:.0} days ({:.1} years)", days / 365.25);
    let specs = prod_specs();
    let started = std::time::Instant::now();
    let mut rows: Vec<SeriesRow> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for sb in SeriesBody::ALL {
            let body_specs: Vec<&SeriesSpec> = specs.iter().filter(|s| s.body == sb).collect();
            if body_specs.is_empty() {
                continue;
            }
            let planet_bytes = &planet_bytes;
            let ceres_bytes = &ceres_bytes;
            let chiron_bytes = &chiron_bytes;
            handles . push (scope . spawn (move | | { let planet_store = KernelStore :: from_spk_bytes (planet_bytes . clone () , "de440s (cheb-gen)" , PLANET_IDS . to_vec () , - f64 :: INFINITY , f64 :: INFINITY ,) . expect ("parse de440s") ; let ast_store = match sb { SeriesBody :: Ceres => Some (KernelStore :: from_spk_bytes (ceres_bytes . clone () , "ceres" , vec ! [2_000_001] , - f64 :: INFINITY , f64 :: INFINITY ,) . expect ("parse ceres") ,) , SeriesBody :: Chiron => Some (KernelStore :: from_spk_bytes (chiron_bytes . clone () , "chiron" , vec ! [2_002_060] , - f64 :: INFINITY , f64 :: INFINITY ,) . expect ("parse chiron") ,) , _ => None , } ; let cache : RefCell < HashMap < u64 , (f64 , f64) > > = RefCell :: new (HashMap :: new ()) ; let body = body_of (sb) ; let lon_dec = | et : f64 | -> (f64 , f64) { if let Some (hit) = cache . borrow () . get (& et . to_bits ()) { return * hit ; } let base = Epoch :: from_et_seconds (et) ; let tdb_at = | d : f64 | base + Duration :: from_days (d) ; let pair = if sb == SeriesBody :: NodeOmega { let prov = KmStateProvider :: new (& planet_store) ; (ascending_node_longitude (& prov , base) . expect ("node Ω") , 0.0 ,) } else if is_asteroid (sb) { let ast = ast_store . as_ref () . expect ("asteroid store") ; let provider = DualStateProvider { target : body , asteroid_store : ast , planet_store : & planet_store , earth_provider : KmStateProvider :: new (& planet_store) , } ; apparent_lon_dec (& provider , body , base , tdb_at) . expect ("asteroid lon/dec") } else { let prov = KmStateProvider :: new (& planet_store) ; apparent_lon_dec (& prov , body , base , tdb_at) . expect ("planet lon/dec") } ; cache . borrow_mut () . insert (et . to_bits () , pair) ; pair } ; let lon_oracle = | et : f64 | lon_dec (et) . 0 ; let dec_oracle = | et : f64 | lon_dec (et) . 1 ; let mut out : Vec < SeriesRow > = Vec :: new () ; for spec in body_specs { let oracle : & dyn Fn (f64) -> f64 = match spec . quantity { Quantity :: Longitude => & lon_oracle , Quantity :: Declination => & dec_oracle , } ; let row = fit_with_ladder (spec , lo , hi , oracle) ; println ! ("  {:?}/{:?}: {} segs of {} d deg {} → max resid {:.4}″ (gate {}″), seam {:.4}″  {}" , row . body , row . quantity , row . n_segments , row . seg_days , row . degree , row . max_residual_arcsec , row . gate_arcsec , row . max_boundary_jump_arcsec , if row . ok { "OK" } else { "FAIL" } ,) ; out . push (row) ; } out })) ;
        }
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("fit thread"))
            .collect()
    });
    println!("fit wall time: {:.1}s", started.elapsed().as_secs_f64());
    rows.sort_by_key(|r| (r.body as u16, r.quantity as u16));
    let failed: Vec<String> = rows
        .iter()
        .filter(|r| !r.ok)
        .map(|r| {
            format!(
                "{:?}/{:?} ({}″ > {}″)",
                r.body, r.quantity, r.max_residual_arcsec, r.gate_arcsec
            )
        })
        .collect();
    if !failed.is_empty() {
        eprintln!(
            "residual gate FAILED for: {} — not writing a blob",
            failed.join(", ")
        );
        std::process::exit(1);
    }
    let fitted_lo = lo;
    let fitted_hi = rows
        .iter()
        .map(|r| r.series_hi_et)
        .fold(f64::INFINITY, f64::min);
    let domain_lo = fitted_lo + GUARD_LO_S;
    let domain_hi = fitted_hi - GUARD_HI_S;
    let (lo_y, ..) = Epoch::from_et_seconds(domain_lo).to_gregorian_utc();
    let (hi_y, hi_m, hi_d, ..) = Epoch::from_et_seconds(domain_hi).to_gregorian_utc();
    println!(
        "domain: [{}, {}] ET s → years {lo_y}..{hi_y} (top edge {hi_y}-{hi_m:02}-{hi_d:02})",
        domain_lo, domain_hi
    );
    let series: Vec<WriteSeries> = rows
        .iter()
        .map(|r| WriteSeries {
            body: r.series.body,
            quantity: r.series.quantity,
            n_coeffs: r.series.n_coeffs,
            t0_et: r.series.t0_et,
            seg_len_s: r.series.seg_len_s,
            coeffs: r.series.coeffs.clone(),
        })
        .collect();
    let blob = write_blob(
        domain_lo,
        domain_hi,
        fitted_lo,
        fitted_hi,
        fingerprint,
        &series,
    );
    if let Some(dir) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(dir).expect("create output dir");
    }
    std::fs::write(&out, &blob).unwrap_or_else(|e| panic!("write {out}: {e}"));
    let sha256 = {
        let mut h = Sha256::new();
        h.update(&blob);
        format!("{:x}", h.finalize())
    };
    let series_json : Vec < serde_json :: Value > = rows . iter () . map (| r | { serde_json :: json ! ({ "body" : format ! ("{:?}" , r . body) , "quantity" : format ! ("{:?}" , r . quantity) , "segDays" : r . seg_days , "degree" : r . degree , "nSegments" : r . n_segments , "maxResidualArcsec" : r . max_residual_arcsec , "maxBoundaryJumpArcsec" : r . max_boundary_jump_arcsec , "gateArcsec" : r . gate_arcsec , "ladder" : r . ladder , }) }) . collect () ;
    let report = serde_json :: json ! ({ "generator" : "dataset-builder fit" , "engineFingerprint" : format ! ("{fingerprint:08x}") , "domainEt" : { "lo" : domain_lo , "hi" : domain_hi } , "fittedEt" : { "lo" : fitted_lo , "hi" : fitted_hi } , "blob" : { "bytes" : blob . len () , "sha256" : sha256 } , "series" : series_json , });
    if let Some(dir) = std::path::Path::new(&report_path).parent() {
        std::fs::create_dir_all(dir).expect("create report dir");
    }
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap())
        .unwrap_or_else(|e| panic!("write {report_path}: {e}"));
    let manifest = serde_json :: json ! ({ "key" : "cheb/v1/cheb.bin" , "format" : "HDCHEB01" , "bytes" : blob . len () , "sha256" : sha256 , "engineFingerprint" : format ! ("{fingerprint:08x}") , "domainEt" : { "lo" : domain_lo , "hi" : domain_hi } , "series" : series_json , });
    std::fs::write(
        &manifest_out,
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap_or_else(|e| panic!("write {manifest_out}: {e}"));
    println!("wrote {out}: {} bytes, sha256 {sha256}", blob.len());
    println!("wrote {report_path} + {manifest_out}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        Some("extract" | "inspect")
    ) {
        if let Err(error) = dataset_builder::extract::command(&args) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    if args.first().map(String::as_str) != Some("fit") {
        eprintln!("usage: dataset-builder fit --de440s PATH --ceres PATH --chiron PATH --out PATH --report PATH --manifest PATH [--fingerprint HEX]");
        std::process::exit(2);
    }
    fit(&args[1..]);
}
