#![allow(clippy::inconsistent_digit_grouping)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::should_implement_trait)]

mod astro;
mod cheb;
mod coverage_window;
mod types;

pub mod chart;
pub mod conventions;
pub use conventions::{
    assign_sign, calculate_configured_chart, effective_configuration, true_sky_offset,
    whole_sign_cusps, zodiac_sectors, ActualHouseSystem, ConfiguredBody, ConfiguredChartValues,
    Ophiuchus, Sign, SignDivisions, SignPosition, SignSector, SpeedReference, ZodiacConfiguration,
    ZodiacReference, CONVENTION_REVISION,
};
pub mod search;
pub mod sidereal;
pub use sidereal::{calculate_sidereal_chart, fagan_bradley_ayanamsa, SiderealChartValues};

pub use astro::find_moment::Direction;
pub use astro::houses::HouseSystem;
pub use chart::{calculate_chart, ChartInput, ChartValues};
pub use cheb::chart::earth_from_sun;
pub use cheb::{BlobError, ChebPlace, Ephemeris, EvalError, Quantity, SeriesBody};
pub use coverage_window::CoverageWindow;
pub use hifitime::Epoch;
pub use search::{find_sun_crossing, SunSearchInput};
pub use types::{Body, ChartBody, Location};

#[derive(Debug)]
pub enum CalculationError {
    InvalidInput(&'static str),
    OutOfCoverage {
        window: CoverageWindow,
        lookback_s: f64,
    },
    Evaluation(EvalError),
}

impl std::fmt::Display for CalculationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => f.write_str(message),
            Self::OutOfCoverage { window, lookback_s } => write!(
                f,
                "epoch outside coverage [{}, {}] ET seconds",
                window.lo_et + lookback_s,
                window.hi_et
            ),
            Self::Evaluation(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for CalculationError {}

pub fn coverage(ephemeris: &Ephemeris) -> CoverageWindow {
    let (lo_et, hi_et) = ephemeris.domain_et();
    CoverageWindow { lo_et, hi_et }
}

fn check_coverage(ephemeris: &Ephemeris, et: f64, lookback_s: f64) -> Result<(), CalculationError> {
    let window = coverage(ephemeris);
    if window.contains_with_lookback(et, lookback_s) {
        Ok(())
    } else {
        Err(CalculationError::OutOfCoverage { window, lookback_s })
    }
}

#[cfg(feature = "generation")]
pub mod tooling {
    pub use crate::astro::{frames, time, vec};
    pub use crate::cheb::fit::clenshaw;
    pub use crate::cheb::format;
    pub use crate::coverage_window::intersect_windows;
    pub use crate::types::{Frame, FrameError, StateProvider, StateVector};
}
