// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Body {
    Sun,
    Earth,
    Moon,
    NorthNode,
    SouthNode,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
    Chiron,
    Ceres,
    AscendantSymbol,
    Midheaven,
    Descendant,
    ImumCoeli,
}
impl Body {
    pub const ALL: [Body; 19] = [
        Body::Sun,
        Body::Earth,
        Body::Moon,
        Body::NorthNode,
        Body::SouthNode,
        Body::Mercury,
        Body::Venus,
        Body::Mars,
        Body::Jupiter,
        Body::Saturn,
        Body::Uranus,
        Body::Neptune,
        Body::Pluto,
        Body::Chiron,
        Body::Ceres,
        Body::AscendantSymbol,
        Body::Midheaven,
        Body::Descendant,
        Body::ImumCoeli,
    ];
    pub fn wire_name(self) -> &'static str {
        match self {
            Body::Sun => "SUN",
            Body::Earth => "EARTH",
            Body::Moon => "MOON",
            Body::NorthNode => "NORTH_NODE",
            Body::SouthNode => "SOUTH_NODE",
            Body::Mercury => "MERCURY",
            Body::Venus => "VENUS",
            Body::Mars => "MARS",
            Body::Jupiter => "JUPITER",
            Body::Saturn => "SATURN",
            Body::Uranus => "URANUS",
            Body::Neptune => "NEPTUNE",
            Body::Pluto => "PLUTO",
            Body::Chiron => "CHIRON",
            Body::Ceres => "CERES",
            Body::AscendantSymbol => "ASCENDANT_SYMBOL",
            Body::Midheaven => "MIDHEAVEN",
            Body::Descendant => "DESCENDANT",
            Body::ImumCoeli => "IMUM_COELI",
        }
    }
    #[cfg(feature = "generation")]
    pub fn naif_id(self) -> Option<i32> {
        match self {
            Body::Sun => Some(10),
            Body::Earth => Some(399),
            Body::Moon => Some(301),
            Body::Mercury => Some(1),
            Body::Venus => Some(2),
            Body::Mars => Some(4),
            Body::Jupiter => Some(5),
            Body::Saturn => Some(6),
            Body::Uranus => Some(7),
            Body::Neptune => Some(8),
            Body::Pluto => Some(9),
            Body::Ceres => Some(2_000_001),
            Body::Chiron => Some(2_002_060),
            Body::NorthNode
            | Body::SouthNode
            | Body::AscendantSymbol
            | Body::Midheaven
            | Body::Descendant
            | Body::ImumCoeli => None,
        }
    }
    pub fn supports_find_moment(self) -> bool {
        matches!(self, Body::Sun)
    }
}
#[cfg(feature = "generation")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frame {
    Eqj,
    Ect,
}
#[cfg(feature = "generation")]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateVector {
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    pub frame: Frame,
}
#[cfg(feature = "generation")]
impl StateVector {
    pub fn eqj(pos: [f64; 3], vel: [f64; 3]) -> Self {
        StateVector {
            pos,
            vel,
            frame: Frame::Eqj,
        }
    }
}
#[cfg(feature = "generation")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    LightTimeDiverged,
    StateUnavailable(Body),
}
#[cfg(feature = "generation")]
impl core::fmt::Display for FrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FrameError::LightTimeDiverged => {
                write!(
                    f,
                    "light-travel solver did not converge (body too distant?)"
                )
            }
            FrameError::StateUnavailable(b) => {
                write!(f, "no ephemeris state available for {b:?}")
            }
        }
    }
}
#[cfg(feature = "generation")]
impl std::error::Error for FrameError {}
#[cfg(feature = "generation")]
pub trait StateProvider {
    fn state_at(&self, body: Body, tdb: hifitime::Epoch) -> Result<StateVector, FrameError>;
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChartBody {
    pub name: String,
    pub longitude: f64,
    pub speed: f64,
    pub declination: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}
