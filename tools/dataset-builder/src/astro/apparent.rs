// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

// This file incorporates work covered by the following copyright and
// permission notice:
// Astronomy Engine attribution applies to src/astro/time.rs, src/astro/frames.rs, and the corresponding host apparent-position transforms.
//
// Astronomy Engine — MIT License
// Copyright (c) 2019-2025 Don Cross <cosinekitty@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use super::frames::{ecliptic_of_date, eqd_declination};
use super::time::AstroTime;
use super::vec::Vec3;
use crate::types::{Body, Frame, FrameError, StateProvider, StateVector};
const C_AUDAY: f64 = 173.1446326846693;
fn normalize360(deg: f64) -> f64 {
    let d = deg.rem_euclid(360.0);
    if d < 0.0 {
        d + 360.0
    } else {
        d
    }
}
fn geo_vector_eqj(
    provider: &impl StateProvider,
    body: Body,
    base_offset: f64,
    tdb_at: &dyn Fn(f64) -> hifitime::Epoch,
) -> Result<Vec3, FrameError> {
    if body == Body::Earth {
        return Ok(Vec3::new(0.0, 0.0, 0.0));
    }
    let mut lt = 0.0_f64;
    for _ in 0..10 {
        let sample = tdb_at(base_offset - lt);
        let target = state_pos(provider, body, sample)?;
        let earth = state_pos(provider, Body::Earth, sample)?;
        let rel = target.sub(earth);
        let lt2 = rel.length() / C_AUDAY;
        if lt2 > 1.0 {
            return Err(FrameError::LightTimeDiverged);
        }
        if (lt2 - lt).abs() < 1.0e-9 {
            return Ok(rel);
        }
        lt = lt2;
    }
    Err(FrameError::LightTimeDiverged)
}
fn state_pos(
    provider: &impl StateProvider,
    body: Body,
    tdb: hifitime::Epoch,
) -> Result<Vec3, FrameError> {
    let s: StateVector = provider.state_at(body, tdb)?;
    debug_assert!(matches!(s.frame, Frame::Eqj), "state_at must return EQJ");
    Ok(Vec3::from_array(s.pos))
}
pub fn apparent_lon_dec(
    provider: &impl StateProvider,
    body: Body,
    tdb: hifitime::Epoch,
    tdb_at: impl Fn(f64) -> hifitime::Epoch,
) -> Result<(f64, f64), FrameError> {
    let time = AstroTime::from_tdb(tdb);
    let geo = geo_vector_eqj(provider, body, 0.0, &tdb_at)?;
    let (lon, _lat, _dist) = ecliptic_of_date(geo, time);
    Ok((normalize360(lon), eqd_declination(geo, time)))
}
