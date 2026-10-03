// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

const SEARCH_WINDOW_DAYS: f64 = 730.0;
const SCAN_STEP_DAYS: f64 = 1.0;
const BISECT_EPSILON_DAYS: f64 = 1.0 / 86_400.0;
const SUN_MEAN_MOTION_DEG_PER_DAY: f64 = 360.0 / 365.2422;
pub const SEED_BRACKET_HALF_DAYS: f64 = 4.0;
pub fn normalize360(value: f64) -> f64 {
    value.rem_euclid(360.0)
}
pub fn signed_delta(a: f64, b: f64) -> f64 {
    ((a - b).rem_euclid(360.0) + 540.0).rem_euclid(360.0) - 180.0
}
fn bisect(
    sample: &mut impl FnMut(f64) -> f64,
    target: f64,
    lo_days: f64,
    hi_days: f64,
    lo_diff: f64,
) -> f64 {
    let mut lo = lo_days;
    let mut hi = hi_days;
    let mut lo_sign = lo_diff.signum();
    while hi - lo > BISECT_EPSILON_DAYS {
        let mid = (lo + hi) / 2.0;
        let mid_diff = signed_delta(sample(mid), target);
        if mid_diff == 0.0 {
            return mid;
        }
        if mid_diff.signum() == lo_sign {
            lo = mid;
            lo_sign = mid_diff.signum();
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}
fn is_real_crossing(prev_diff: f64, cur_diff: f64) -> bool {
    prev_diff.signum() != cur_diff.signum() && (prev_diff.abs() + cur_diff.abs()) < 180.0
}
fn coarse_scan(
    sample: &mut impl FnMut(f64) -> f64,
    target: f64,
    direction: Direction,
) -> Option<f64> {
    let step_sign = match direction {
        Direction::Backward => -1.0,
        Direction::Forward => 1.0,
    };
    let mut prev_days = 0.0_f64;
    let mut prev_diff = signed_delta(sample(prev_days), target);
    if prev_diff == 0.0 {
        return Some(prev_days);
    }
    let mut i = 1.0_f64;
    while i * SCAN_STEP_DAYS <= SEARCH_WINDOW_DAYS {
        let cur_days = step_sign * i * SCAN_STEP_DAYS;
        let cur_diff = signed_delta(sample(cur_days), target);
        if is_real_crossing(prev_diff, cur_diff) {
            let (lo, hi, lo_diff) = if cur_days < prev_days {
                (cur_days, prev_days, cur_diff)
            } else {
                (prev_days, cur_days, prev_diff)
            };
            return Some(bisect(sample, target, lo, hi, lo_diff));
        }
        prev_days = cur_days;
        prev_diff = cur_diff;
        i += 1.0;
    }
    None
}
fn seed_backward_sun(sample: &mut impl FnMut(f64) -> f64, target: f64, arc: f64) -> Option<f64> {
    let guess_days = -(arc / SUN_MEAN_MOTION_DEG_PER_DAY);
    let lo = guess_days - SEED_BRACKET_HALF_DAYS;
    let hi = guess_days + SEED_BRACKET_HALF_DAYS;
    let lo_diff = signed_delta(sample(lo), target);
    let hi_diff = signed_delta(sample(hi), target);
    if !is_real_crossing(lo_diff, hi_diff) {
        return None;
    }
    Some(bisect(sample, target, lo, hi, lo_diff))
}
pub fn find_crossing(
    mut sample: impl FnMut(f64) -> f64,
    target: f64,
    direction: Direction,
    is_sun: bool,
) -> Option<f64> {
    let target = normalize360(target);
    if is_sun && direction == Direction::Backward {
        let birth_sun = sample(0.0);
        let arc = normalize360(birth_sun - target);
        if let Some(days) = seed_backward_sun(&mut sample, target, arc) {
            return Some(days);
        }
    }
    coarse_scan(&mut sample, target, direction)
}
