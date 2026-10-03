// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use astrology_engine::tooling::clenshaw;
pub fn to_unit(t: f64, lo: f64, hi: f64) -> f64 {
    2.0 * (t - lo) / (hi - lo) - 1.0
}
fn cheb_basis(x: f64, n: usize) -> Vec<f64> {
    let mut t = vec![0.0; n + 1];
    t[0] = 1.0;
    if n >= 1 {
        t[1] = x;
    }
    for k in 2..=n {
        t[k] = 2.0 * x * t[k - 1] - t[k - 2];
    }
    t
}
pub fn cheb_fit(xs: &[f64], ys: &[f64], deg: usize) -> Vec<f64> {
    let m = deg + 1;
    let mut ata = vec![vec![0.0_f64; m]; m];
    let mut atb = vec![0.0_f64; m];
    for (&x, &y) in xs.iter().zip(ys) {
        let basis = cheb_basis(x, deg);
        for i in 0..m {
            atb[i] += basis[i] * y;
            for j in 0..m {
                ata[i][j] += basis[i] * basis[j];
            }
        }
    }
    solve(ata, atb)
}
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for col in 0..n {
        let mut piv = col;
        for r in (col + 1)..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        a.swap(col, piv);
        b.swap(col, piv);
        let d = a[col][col];
        let (pivot_rows, below) = a.split_at_mut(col + 1);
        let pivot = &pivot_rows[col];
        for (off, row) in below.iter_mut().enumerate() {
            let f = row[col] / d;
            for (rc, pc) in row[col..].iter_mut().zip(&pivot[col..]) {
                *rc -= f * pc;
            }
            b[col + 1 + off] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for j in (i + 1)..n {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    x
}
pub fn unwrap_deg(series: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(series.len());
    for (i, &v) in series.iter().enumerate() {
        if i == 0 {
            out.push(v);
            continue;
        }
        let mut d = v - series[i - 1];
        while d > 180.0 {
            d -= 360.0;
        }
        while d < -180.0 {
            d += 360.0;
        }
        out.push(out[i - 1] + d);
    }
    out
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FitKind {
    Angle,
    Scalar,
}
pub fn fit_segment(
    lo: f64,
    hi: f64,
    deg: usize,
    kind: FitKind,
    f: &dyn Fn(f64) -> f64,
) -> Vec<f64> {
    let n_fit = (deg + 1) * 3;
    let mut ts = Vec::with_capacity(n_fit);
    for i in 0..n_fit {
        let theta = std::f64::consts::PI * (i as f64 + 0.5) / (n_fit as f64);
        let x = -theta.cos();
        ts.push(lo + (hi - lo) * (x + 1.0) / 2.0);
    }
    let raw: Vec<f64> = ts.iter().map(|&t| f(t)).collect();
    let vals = match kind {
        FitKind::Angle => unwrap_deg(&raw),
        FitKind::Scalar => raw,
    };
    let xs: Vec<f64> = ts.iter().map(|&t| to_unit(t, lo, hi)).collect();
    cheb_fit(&xs, &vals, deg)
}
fn wrapped_abs_deg(a: f64, b: f64) -> f64 {
    let mut d = (a - b).rem_euclid(360.0);
    if d > 180.0 {
        d = 360.0 - d;
    }
    d
}
pub fn max_residual_arcsec(
    coeffs: &[f64],
    lo: f64,
    hi: f64,
    kind: FitKind,
    n_eval: usize,
    f: &dyn Fn(f64) -> f64,
) -> f64 {
    let mut worst = 0.0_f64;
    for i in 0..n_eval {
        let t = lo + (hi - lo) * (i as f64) / ((n_eval - 1) as f64);
        let truth = f(t);
        let approx = clenshaw(coeffs, to_unit(t, lo, hi));
        let resid = match kind {
            FitKind::Angle => wrapped_abs_deg(approx, truth),
            FitKind::Scalar => (approx - truth).abs(),
        };
        worst = worst.max(resid * 3600.0);
    }
    worst
}
pub fn boundary_jump_arcsec(segments: &[Vec<f64>], kind: FitKind) -> f64 {
    let mut worst = 0.0_f64;
    for pair in segments.windows(2) {
        let left = clenshaw(&pair[0], 1.0);
        let right = clenshaw(&pair[1], -1.0);
        let jump = match kind {
            FitKind::Angle => wrapped_abs_deg(left, right),
            FitKind::Scalar => (left - right).abs(),
        };
        worst = worst.max(jump * 3600.0);
    }
    worst
}
