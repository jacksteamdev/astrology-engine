pub use crate::coverage_window::{intersect_windows, CoverageWindow};
use anise::naif::daf::DAF;
use anise::naif::spk::summary::SPKSummaryRecord;
use anise::naif::SPK;
pub const WORD_BYTES: u64 = 8;
pub const HEADER_BYTES: u64 = 64 * 1024;
pub type IndexResult<T> = core::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentPlan {
    pub target_id: i32,
    pub byte_start: u64,
    pub byte_len: u64,
    pub data_type: i32,
}
impl SegmentPlan {
    pub fn byte_end(&self) -> u64 {
        self.byte_start + self.byte_len
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChebyshevMeta {
    pub init: f64,
    pub interval_len: f64,
    pub record_size: u64,
    pub record_count: u64,
}
impl ChebyshevMeta {
    pub fn coverage_window(&self) -> CoverageWindow {
        CoverageWindow {
            lo_et: self.init,
            hi_et: self.init + self.interval_len * self.record_count as f64,
        }
    }
}
pub fn coverage_window_for(metas: &[ChebyshevMeta]) -> Option<CoverageWindow> {
    let windows: Vec<CoverageWindow> = metas.iter().map(|m| m.coverage_window()).collect();
    intersect_windows(&windows)
}
pub fn plan_segments(header_bytes: &[u8], target_ids: &[i32]) -> IndexResult<Vec<SegmentPlan>> {
    let spk: SPK = DAF::<SPKSummaryRecord>::parse(header_bytes).map_err(|e| e.to_string())?;
    let summaries = spk.data_summaries(None).map_err(|e| e.to_string())?;
    target_ids
        .iter()
        .map(|&target_id| {
            let summary = summaries
                .iter()
                .find(|s| s.target_id == target_id)
                .ok_or_else(|| format!("no DAF summary for target {target_id}"))?;
            let byte_start = (summary.start_idx as u64 - 1) * WORD_BYTES;
            let byte_end = summary.end_idx as u64 * WORD_BYTES;
            Ok(SegmentPlan {
                target_id,
                byte_start,
                byte_len: byte_end - byte_start,
                data_type: summary.data_type_i as i32,
            })
        })
        .collect()
}
pub fn read_cheby_meta(segment_tail: &[u8]) -> IndexResult<ChebyshevMeta> {
    if segment_tail.len() < 32 {
        return Err(format!(
            "segment tail too short for Chebyshev trailer: {} bytes (need 32)",
            segment_tail.len()
        ));
    }
    let n = segment_tail.len();
    let read = |i: usize| -> f64 {
        let mut b = [0u8; 8];
        b.copy_from_slice(&segment_tail[n - 32 + i * 8..n - 32 + i * 8 + 8]);
        f64::from_le_bytes(b)
    };
    let init = read(0);
    let interval_len = read(1);
    let record_size = read(2);
    let record_count = read(3);
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(interval_len > 0.0) || !(record_size >= 2.0) || !(record_count >= 1.0) {
        return Err (format ! ("implausible Chebyshev trailer: INIT={init} INTLEN={interval_len} RSIZE={record_size} N={record_count}")) ;
    }
    Ok(ChebyshevMeta {
        init,
        interval_len,
        record_size: record_size as u64,
        record_count: record_count as u64,
    })
}
