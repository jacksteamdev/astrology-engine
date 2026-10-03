use crate::{CalculationError, Direction, Ephemeris, Epoch, SeriesBody};

#[derive(Clone, Copy, Debug)]
pub struct SunSearchInput {
    pub epoch: Epoch,
    pub target_longitude: f64,
    pub direction: Direction,
}

pub fn find_sun_crossing(
    ephemeris: &Ephemeris,
    input: SunSearchInput,
) -> Result<Option<Epoch>, CalculationError> {
    if !(0.0..360.0).contains(&input.target_longitude) {
        return Err(CalculationError::InvalidInput(
            "targetLongitude must be in [0, 360)",
        ));
    }
    crate::check_coverage(
        ephemeris,
        input.epoch.to_et_seconds(),
        crate::coverage_window::DESIGN_LOOKBACK_S,
    )?;
    let mut sample = |days: f64| {
        let at = input.epoch + hifitime::Duration::from_days(days);
        ephemeris
            .longitude(SeriesBody::Sun, at.to_et_seconds())
            .unwrap_or(f64::NAN)
    };
    let days = crate::astro::find_moment::find_crossing(
        &mut sample,
        input.target_longitude,
        input.direction,
        true,
    );
    Ok(days.map(|days| input.epoch + hifitime::Duration::from_days(days)))
}
