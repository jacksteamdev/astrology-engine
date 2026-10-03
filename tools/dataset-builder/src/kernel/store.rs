use crate::coverage_window::CoverageWindow;
use crate::types::{Body, Frame as FrameTag, StateVector};
use anise::almanac::Almanac;
use anise::constants::frames::SSB_J2000;
use anise::frames::Frame;
use anise::naif::daf::DAF;
use anise::naif::spk::summary::SPKSummaryRecord;
use anise::naif::SPK;
use hifitime::Epoch;
pub type StoreResult<T> = core::result::Result<T, String>;
pub fn almanac_from_slice(spk_bytes: &[u8]) -> StoreResult<Almanac> {
    let spk: SPK = DAF::<SPKSummaryRecord>::parse(spk_bytes).map_err(|e| e.to_string())?;
    Ok(Almanac::from_spk(spk))
}
pub fn eval_state_from_spk(almanac: &Almanac, body: Body, tdb: Epoch) -> StoreResult<StateVector> {
    let id = body
        .naif_id()
        .ok_or_else(|| format!("{body:?} is derived; state_at has nothing to read"))?;
    let target = Frame::from_ephem_j2000(id);
    let state = almanac
        .translate_geometric(target, SSB_J2000, tdb)
        .map_err(|e| e.to_string())?;
    Ok(StateVector {
        pos: [state.radius_km.x, state.radius_km.y, state.radius_km.z],
        vel: [
            state.velocity_km_s.x,
            state.velocity_km_s.y,
            state.velocity_km_s.z,
        ],
        frame: FrameTag::Eqj,
    })
}
pub fn eval_state_centered_from_spk(
    almanac: &Almanac,
    body: Body,
    center_id: i32,
    tdb: Epoch,
) -> StoreResult<StateVector> {
    let id = body
        .naif_id()
        .ok_or_else(|| format!("{body:?} is derived; state_at has nothing to read"))?;
    let target = Frame::from_ephem_j2000(id);
    let center = Frame::from_ephem_j2000(center_id);
    let state = almanac
        .translate_geometric(target, center, tdb)
        .map_err(|e| e.to_string())?;
    Ok(StateVector {
        pos: [state.radius_km.x, state.radius_km.y, state.radius_km.z],
        vel: [
            state.velocity_km_s.x,
            state.velocity_km_s.y,
            state.velocity_km_s.z,
        ],
        frame: FrameTag::Eqj,
    })
}
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    pub kernel: String,
    pub body_ids: Vec<i32>,
    pub t_lo_et: f64,
    pub t_hi_et: f64,
}
pub struct KernelStore {
    almanac: Almanac,
}
impl KernelStore {
    pub fn new(almanac: Almanac, _coverage: Coverage, _window: CoverageWindow) -> Self {
        KernelStore { almanac }
    }
    pub fn from_spk_bytes(
        spk_bytes: Vec<u8>,
        kernel: impl Into<String>,
        body_ids: Vec<i32>,
        t_lo_et: f64,
        t_hi_et: f64,
    ) -> StoreResult<Self> {
        Self::from_slice(&spk_bytes, kernel, body_ids, t_lo_et, t_hi_et)
    }
    pub fn from_slice(
        spk_bytes: &[u8],
        kernel: impl Into<String>,
        body_ids: Vec<i32>,
        t_lo_et: f64,
        t_hi_et: f64,
    ) -> StoreResult<Self> {
        let almanac = almanac_from_slice(spk_bytes)?;
        Ok(KernelStore::new(
            almanac,
            Coverage {
                kernel: kernel.into(),
                body_ids,
                t_lo_et,
                t_hi_et,
            },
            CoverageWindow {
                lo_et: t_lo_et,
                hi_et: t_hi_et,
            },
        ))
    }
    pub fn state_at(&self, body: Body, tdb: Epoch) -> StoreResult<StateVector> {
        eval_state_from_spk(&self.almanac, body, tdb)
    }
    pub fn state_at_centered(
        &self,
        body: Body,
        center_id: i32,
        tdb: Epoch,
    ) -> StoreResult<StateVector> {
        eval_state_centered_from_spk(&self.almanac, body, center_id, tdb)
    }
}
