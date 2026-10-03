// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use crate::kernel::store::KernelStore;
use crate::types::{Body, FrameError, StateProvider, StateVector};
use hifitime::Epoch;
pub const KM_PER_AU: f64 = 149_597_870.7;
pub struct KmStateProvider<'a> {
    store: &'a KernelStore,
}
impl<'a> KmStateProvider<'a> {
    pub fn new(store: &'a KernelStore) -> Self {
        KmStateProvider { store }
    }
}
impl StateProvider for KmStateProvider<'_> {
    fn state_at(&self, body: Body, tdb: Epoch) -> Result<StateVector, FrameError> {
        let km = self
            .store
            .state_at(body, tdb)
            .map_err(|_| FrameError::StateUnavailable(body))?;
        let inv = 1.0 / KM_PER_AU;
        Ok(StateVector {
            pos: [km.pos[0] * inv, km.pos[1] * inv, km.pos[2] * inv],
            vel: [
                km.vel[0] * inv * 86_400.0,
                km.vel[1] * inv * 86_400.0,
                km.vel[2] * inv * 86_400.0,
            ],
            frame: km.frame,
        })
    }
}
const ASTEROID_CENTER_ID: i32 = 10;
pub struct DualStateProvider<'a> {
    pub target: Body,
    pub asteroid_store: &'a KernelStore,
    pub planet_store: &'a KernelStore,
    pub earth_provider: KmStateProvider<'a>,
}
impl StateProvider for DualStateProvider<'_> {
    fn state_at(&self, body: Body, tdb: Epoch) -> Result<StateVector, FrameError> {
        if body == Body::Earth {
            return self.earth_provider.state_at(body, tdb);
        }
        if body != self.target {
            return Err(FrameError::StateUnavailable(body));
        }
        let ast_rel_sun = self
            .asteroid_store
            .state_at_centered(body, ASTEROID_CENTER_ID, tdb)
            .map_err(|_| FrameError::StateUnavailable(body))?;
        let sun_ssb = self
            .planet_store
            .state_at(Body::Sun, tdb)
            .map_err(|_| FrameError::StateUnavailable(Body::Sun))?;
        let pos_km = [
            ast_rel_sun.pos[0] + sun_ssb.pos[0],
            ast_rel_sun.pos[1] + sun_ssb.pos[1],
            ast_rel_sun.pos[2] + sun_ssb.pos[2],
        ];
        let vel_km = [
            ast_rel_sun.vel[0] + sun_ssb.vel[0],
            ast_rel_sun.vel[1] + sun_ssb.vel[1],
            ast_rel_sun.vel[2] + sun_ssb.vel[2],
        ];
        let inv = 1.0 / KM_PER_AU;
        Ok(StateVector {
            pos: [pos_km[0] * inv, pos_km[1] * inv, pos_km[2] * inv],
            vel: [
                vel_km[0] * inv * 86_400.0,
                vel_km[1] * inv * 86_400.0,
                vel_km[2] * inv * 86_400.0,
            ],
            frame: ast_rel_sun.frame,
        })
    }
}
