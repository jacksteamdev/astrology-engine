use astrology_engine::{
    calculate_chart, find_sun_crossing, Body, ChartInput, Direction, Ephemeris, Epoch, HouseSystem,
    Location, SunSearchInput,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = std::env::args_os()
        .nth(1)
        .ok_or("Usage: cargo run --example birth_and_design -- /path/to/cheb.bin")?;
    let ephemeris = Ephemeris::parse(std::fs::read(dataset)?)
        .map_err(|error| format!("Cannot load ephemeris: {error}"))?;

    let birth_moment = Epoch::from_gregorian_str("2000-01-01T12:00:00 UTC")?;
    let location = Location {
        latitude: 51.5074,
        longitude: -0.1278,
    };
    let chart_at = |epoch| {
        calculate_chart(
            &ephemeris,
            ChartInput {
                epoch,
                location,
                house_system: HouseSystem::Equal,
            },
        )
    };

    let birth_chart = chart_at(birth_moment)?;
    let birth_sun = birth_chart
        .bodies
        .iter()
        .find(|body| body.name == Body::Sun.wire_name())
        .ok_or("Birth chart has no Sun")?;
    let target_longitude = (birth_sun.longitude - 88.0).rem_euclid(360.0);
    let design_moment = find_sun_crossing(
        &ephemeris,
        SunSearchInput {
            epoch: birth_moment,
            target_longitude,
            direction: Direction::Backward,
        },
    )?
    .ok_or("No design moment found within the search window and dataset coverage")?;
    let design_chart = chart_at(design_moment)?;

    println!("Birth moment: {birth_moment}");
    println!("Design moment: {design_moment}");
    println!("Longitudes in degrees:");
    println!("{:<20} {:>12} {:>12}", "Body", "Birth", "Design");
    for (birth, design) in birth_chart.bodies.iter().zip(&design_chart.bodies) {
        println!(
            "{:<20} {:>12.6} {:>12.6}",
            birth.name, birth.longitude, design.longitude
        );
    }

    Ok(())
}
