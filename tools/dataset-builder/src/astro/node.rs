use super::frames::{eqj_to_ect_state, RAD2DEG};
use super::time::AstroTime;
use super::vec::Vec3;
use crate::types::{Body, FrameError, StateProvider};
fn normalize360(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}
fn geocentric_moon_ect(
    provider: &impl StateProvider,
    tdb: hifitime::Epoch,
) -> Result<(Vec3, Vec3), FrameError> {
    let moon = provider.state_at(Body::Moon, tdb)?;
    let earth = provider.state_at(Body::Earth, tdb)?;
    let pos = Vec3::new(
        moon.pos[0] - earth.pos[0],
        moon.pos[1] - earth.pos[1],
        moon.pos[2] - earth.pos[2],
    );
    let vel = Vec3::new(
        moon.vel[0] - earth.vel[0],
        moon.vel[1] - earth.vel[1],
        moon.vel[2] - earth.vel[2],
    );
    let time = AstroTime::from_tdb(tdb);
    Ok(eqj_to_ect_state(pos, vel, time))
}
pub fn ascending_node_longitude(
    provider: &impl StateProvider,
    tdb: hifitime::Epoch,
) -> Result<f64, FrameError> {
    let (pos, vel) = geocentric_moon_ect(provider, tdb)?;
    let h = pos.cross(vel);
    Ok(normalize360(h.x.atan2(-h.y) * RAD2DEG))
}
