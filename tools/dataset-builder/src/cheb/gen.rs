// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use super::fit::{boundary_jump_arcsec, fit_segment, max_residual_arcsec, FitKind};
use super::format::{Quantity, SeriesBody, WriteSeries};
const N_EVAL_PER_SEGMENT: usize = 60;
pub struct FitOutcome {
    pub series: WriteSeries,
    pub max_residual_arcsec: f64,
    pub max_boundary_jump_arcsec: f64,
    pub n_segments: usize,
}
pub fn fit_series(
    body: SeriesBody,
    quantity: Quantity,
    lo_et: f64,
    hi_et: f64,
    seg_len_s: f64,
    degree: usize,
    oracle: &dyn Fn(f64) -> f64,
) -> FitOutcome {
    let kind = match quantity {
        Quantity::Longitude => FitKind::Angle,
        Quantity::Declination => FitKind::Scalar,
    };
    let n_seg = (((hi_et - lo_et) / seg_len_s) + 1e-6).floor().max(1.0) as usize;
    let mut coeffs = Vec::with_capacity(n_seg * (degree + 1));
    let mut per_segment: Vec<Vec<f64>> = Vec::with_capacity(n_seg);
    let mut worst = 0.0_f64;
    for s in 0..n_seg {
        let s_lo = lo_et + s as f64 * seg_len_s;
        let s_hi = s_lo + seg_len_s;
        let seg = fit_segment(s_lo, s_hi, degree, kind, oracle);
        worst = worst.max(max_residual_arcsec(
            &seg,
            s_lo,
            s_hi,
            kind,
            N_EVAL_PER_SEGMENT,
            oracle,
        ));
        coeffs.extend_from_slice(&seg);
        per_segment.push(seg);
    }
    let max_jump = boundary_jump_arcsec(&per_segment, kind);
    FitOutcome {
        series: WriteSeries {
            body,
            quantity,
            n_coeffs: (degree + 1) as u32,
            t0_et: lo_et,
            seg_len_s,
            coeffs,
        },
        max_residual_arcsec: worst,
        max_boundary_jump_arcsec: max_jump,
        n_segments: n_seg,
    }
}
