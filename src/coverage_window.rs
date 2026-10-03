// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

const SECONDS_PER_DAY: f64 = 86_400.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoverageWindow {
    pub lo_et: f64,
    pub hi_et: f64,
}
impl CoverageWindow {
    pub fn contains(&self, et: f64) -> bool {
        et >= self.lo_et && et <= self.hi_et
    }
    pub fn contains_with_lookback(&self, et: f64, lookback_s: f64) -> bool {
        et - lookback_s >= self.lo_et && et <= self.hi_et
    }
}
#[cfg(feature = "generation")]
pub fn intersect_windows(windows: &[CoverageWindow]) -> Option<CoverageWindow> {
    let mut it = windows.iter();
    let first = *it.next()?;
    let merged = it.fold(first, |acc, w| CoverageWindow {
        lo_et: acc.lo_et.max(w.lo_et),
        hi_et: acc.hi_et.min(w.hi_et),
    });
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(merged.lo_et <= merged.hi_et) {
        None
    } else {
        Some(merged)
    }
}
pub const DESIGN_LOOKBACK_S: f64 = 89.0 * SECONDS_PER_DAY;
