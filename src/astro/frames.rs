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

use super::time::{AstroTime, DAYS_PER_CENTURY};
#[cfg(feature = "generation")]
use super::vec::Vec3;
pub const DEG2RAD: f64 = 0.017453292519943296;
#[cfg(feature = "generation")]
pub const RAD2DEG: f64 = 57.295779513082321;
pub const ASEC2RAD: f64 = 4.848136811095359935899141e-6;
const ASEC360: f64 = 1_296_000.0;
#[cfg(feature = "generation")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecessDir {
    From2000,
    Into2000,
}
#[cfg(feature = "generation")]
#[derive(Debug, Clone, Copy)]
pub struct RotationMatrix {
    pub rot: [[f64; 3]; 3],
}
#[cfg(feature = "generation")]
impl RotationMatrix {
    pub fn rotate(&self, v: Vec3) -> Vec3 {
        let r = &self.rot;
        Vec3 {
            x: r[0][0] * v.x + r[1][0] * v.y + r[2][0] * v.z,
            y: r[0][1] * v.x + r[1][1] * v.y + r[2][1] * v.z,
            z: r[0][2] * v.x + r[1][2] * v.y + r[2][2] * v.z,
        }
    }
}
#[derive(Debug, Clone, Copy)]
struct NutationAngles {
    dpsi: f64,
    deps: f64,
}
fn iau2000b(time: AstroTime) -> NutationAngles {
    let mod_arg = |x: f64| (x % ASEC360) * ASEC2RAD;
    let t = time.tt / DAYS_PER_CENTURY;
    let elp = mod_arg(1_287_104.79305 + t * 129_596_581.0481);
    let f = mod_arg(335_779.526232 + t * 1_739_527_262.8478);
    let d = mod_arg(1_072_260.70369 + t * 1_602_961_601.2090);
    let om = mod_arg(450_160.398036 - t * 6_962_890.5431);
    let (mut sarg, mut carg) = (om.sin(), om.cos());
    let mut dp = (-172_064_161.0 - 174_666.0 * t) * sarg + 33_386.0 * carg;
    let mut de = (92_052_331.0 + 9_086.0 * t) * carg + 15_377.0 * sarg;
    let mut arg = 2.0 * (f - d + om);
    sarg = arg.sin();
    carg = arg.cos();
    dp += (-13_170_906.0 - 1_675.0 * t) * sarg - 13_696.0 * carg;
    de += (5_730_336.0 - 3_015.0 * t) * carg - 4_587.0 * sarg;
    arg = 2.0 * (f + om);
    sarg = arg.sin();
    carg = arg.cos();
    dp += (-2_276_413.0 - 234.0 * t) * sarg + 2_796.0 * carg;
    de += (978_459.0 - 485.0 * t) * carg + 1_374.0 * sarg;
    arg = 2.0 * om;
    sarg = arg.sin();
    carg = arg.cos();
    dp += (2_074_554.0 + 207.0 * t) * sarg - 698.0 * carg;
    de += (-897_492.0 + 470.0 * t) * carg - 291.0 * sarg;
    sarg = elp.sin();
    carg = elp.cos();
    dp += (1_475_877.0 - 3_633.0 * t) * sarg + 11_817.0 * carg;
    de += (73_871.0 - 184.0 * t) * carg - 1_924.0 * sarg;
    NutationAngles {
        dpsi: -0.000135 + (dp * 1.0e-7),
        deps: 0.000388 + (de * 1.0e-7),
    }
}
fn mean_obliq(time: AstroTime) -> f64 {
    let t = time.tt / DAYS_PER_CENTURY;
    let asec = ((((-0.0000000434 * t - 0.000000576) * t + 0.00200340) * t - 0.0001831) * t
        - 46.836769)
        * t
        + 84381.406;
    asec / 3600.0
}
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(feature = "generation"), allow(dead_code))]
pub struct EarthTilt {
    pub dpsi: f64,
    pub deps: f64,
    pub ee: f64,
    pub mobl: f64,
    pub tobl: f64,
}
pub fn e_tilt(time: AstroTime) -> EarthTilt {
    let nut = iau2000b(time);
    let mean_ob = mean_obliq(time);
    let true_ob = mean_ob + nut.deps / 3600.0;
    EarthTilt {
        dpsi: nut.dpsi,
        deps: nut.deps,
        ee: nut.dpsi * (mean_ob * DEG2RAD).cos() / 15.0,
        mobl: mean_ob,
        tobl: true_ob,
    }
}
pub fn true_obliquity(time: AstroTime) -> f64 {
    e_tilt(time).tobl
}
pub(crate) fn precession_matrix(time: AstroTime) -> [[f64; 3]; 3] {
    let t = time.tt / DAYS_PER_CENTURY;
    let mut eps0 = 84381.406;
    let mut psia = ((((-0.0000000951 * t + 0.000132851) * t - 0.00114045) * t - 1.0790069) * t
        + 5038.481507)
        * t;
    let mut omegaa =
        ((((0.0000003337 * t - 0.000000467) * t - 0.00772503) * t + 0.0512623) * t - 0.025754) * t
            + eps0;
    let mut chia = ((((-0.0000000560 * t + 0.000170663) * t - 0.00121197) * t - 2.3814292) * t
        + 10.556403)
        * t;
    eps0 *= ASEC2RAD;
    psia *= ASEC2RAD;
    omegaa *= ASEC2RAD;
    chia *= ASEC2RAD;
    let (sa, ca) = (eps0.sin(), eps0.cos());
    let (sb, cb) = ((-psia).sin(), (-psia).cos());
    let (sc, cc) = ((-omegaa).sin(), (-omegaa).cos());
    let (sd, cd) = (chia.sin(), chia.cos());
    let xx = cd * cb - sb * sd * cc;
    let yx = cd * sb * ca + sd * cc * cb * ca - sa * sd * sc;
    let zx = cd * sb * sa + sd * cc * cb * sa + ca * sd * sc;
    let xy = -sd * cb - sb * cd * cc;
    let yy = -sd * sb * ca + cd * cc * cb * ca - sa * cd * sc;
    let zy = -sd * sb * sa + cd * cc * cb * sa + ca * cd * sc;
    let xz = sb * sc;
    let yz = -sc * cb * ca - sa * cc;
    let zz = -sc * cb * sa + cc * ca;
    [[xx, xy, xz], [yx, yy, yz], [zx, zy, zz]]
}
#[cfg(feature = "generation")]
pub fn precession_rot(time: AstroTime, dir: PrecessDir) -> RotationMatrix {
    let matrix = precession_matrix(time);
    match dir {
        PrecessDir::From2000 => RotationMatrix { rot: matrix },
        PrecessDir::Into2000 => RotationMatrix {
            rot: core::array::from_fn(|i| core::array::from_fn(|j| matrix[j][i])),
        },
    }
}
#[cfg(feature = "generation")]
pub fn nutation_rot(time: AstroTime, dir: PrecessDir) -> RotationMatrix {
    let tilt = e_tilt(time);
    let oblm = tilt.mobl * DEG2RAD;
    let oblt = tilt.tobl * DEG2RAD;
    let psi = tilt.dpsi * ASEC2RAD;
    let (cobm, sobm) = (oblm.cos(), oblm.sin());
    let (cobt, sobt) = (oblt.cos(), oblt.sin());
    let (cpsi, spsi) = (psi.cos(), psi.sin());
    let xx = cpsi;
    let yx = -spsi * cobm;
    let zx = -spsi * sobm;
    let xy = spsi * cobt;
    let yy = cpsi * cobm * cobt + sobm * sobt;
    let zy = cpsi * sobm * cobt - cobm * sobt;
    let xz = spsi * sobt;
    let yz = cpsi * cobm * sobt - sobm * cobt;
    let zz = cpsi * sobm * sobt + cobm * cobt;
    match dir {
        PrecessDir::From2000 => RotationMatrix {
            rot: [[xx, xy, xz], [yx, yy, yz], [zx, zy, zz]],
        },
        PrecessDir::Into2000 => RotationMatrix {
            rot: [[xx, yx, zx], [xy, yy, zy], [xz, yz, zz]],
        },
    }
}
fn era(time: AstroTime) -> f64 {
    let thet1 = 0.7790572732640 + 0.00273781191135448 * time.ut;
    let thet3 = time.ut.rem_euclid(1.0);
    let mut theta = 360.0 * (thet1 + thet3).rem_euclid(1.0);
    if theta < 0.0 {
        theta += 360.0;
    }
    theta
}
pub fn sidereal_time(time: AstroTime) -> f64 {
    let t = time.tt / DAYS_PER_CENTURY;
    let eqeq = 15.0 * e_tilt(time).ee;
    let theta = era(time);
    let st = eqeq
        + 0.014506
        + (((((-0.0000000368 * t - 0.000029956) * t - 0.00000044) * t + 1.3915817) * t)
            + 4612.156534)
            * t;
    let mut gst = (st / 3600.0 + theta).rem_euclid(360.0) / 15.0;
    if gst < 0.0 {
        gst += 24.0;
    }
    gst
}
pub fn gast_degrees(time: AstroTime) -> f64 {
    sidereal_time(time) * 15.0
}
#[cfg(feature = "generation")]
pub fn ecliptic_of_date(eqj: Vec3, time: AstroTime) -> (f64, f64, f64) {
    let et = e_tilt(time);
    let mean_pos = precession_rot(time, PrecessDir::From2000).rotate(eqj);
    let eqd = nutation_rot(time, PrecessDir::From2000).rotate(mean_pos);
    let tobl = et.tobl * DEG2RAD;
    let (cos_ob, sin_ob) = (tobl.cos(), tobl.sin());
    let ex = eqd.x;
    let ey = eqd.y * cos_ob + eqd.z * sin_ob;
    let ez = -eqd.y * sin_ob + eqd.z * cos_ob;
    let xyproj = ex.hypot(ey);
    let mut elon = 0.0;
    if xyproj > 0.0 {
        elon = RAD2DEG * ey.atan2(ex);
        if elon < 0.0 {
            elon += 360.0;
        }
    }
    let elat = RAD2DEG * ez.atan2(xyproj);
    let dist = (ex * ex + ey * ey + ez * ez).sqrt();
    (elon, elat, dist)
}
#[cfg(feature = "generation")]
pub fn eqd_declination(eqj: Vec3, time: AstroTime) -> f64 {
    let mean_pos = precession_rot(time, PrecessDir::From2000).rotate(eqj);
    let eqd = nutation_rot(time, PrecessDir::From2000).rotate(mean_pos);
    let r = (eqd.x * eqd.x + eqd.y * eqd.y + eqd.z * eqd.z).sqrt();
    if r > 0.0 {
        RAD2DEG * (eqd.z / r).asin()
    } else {
        0.0
    }
}
#[cfg(feature = "generation")]
pub fn eqj_to_ect_state(pos: Vec3, vel: Vec3, time: AstroTime) -> (Vec3, Vec3) {
    let tobl = e_tilt(time).tobl * DEG2RAD;
    let (cos_ob, sin_ob) = (tobl.cos(), tobl.sin());
    let to_ecl = |v: Vec3| {
        let p = nutation_rot(time, PrecessDir::From2000)
            .rotate(precession_rot(time, PrecessDir::From2000).rotate(v));
        Vec3 {
            x: p.x,
            y: p.y * cos_ob + p.z * sin_ob,
            z: -p.y * sin_ob + p.z * cos_ob,
        }
    };
    (to_ecl(pos), to_ecl(vel))
}
