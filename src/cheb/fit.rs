// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

pub fn clenshaw(coeffs: &[f64], x: f64) -> f64 {
    let mut b1 = 0.0_f64;
    let mut b2 = 0.0_f64;
    for &c in coeffs.iter().skip(1).rev() {
        let b0 = 2.0 * x * b1 - b2 + c;
        b2 = b1;
        b1 = b0;
    }
    coeffs[0] + x * b1 - b2
}
