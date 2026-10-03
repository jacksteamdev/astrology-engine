// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

use astrology_engine::{
    calculate_chart, ChartInput, Ephemeris, Epoch, HouseSystem, Location, SignDivisions,
    ZodiacConfiguration, ZodiacReference,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = std::env::args_os()
        .nth(1)
        .ok_or("Usage: cargo run --example natal_chart -- /path/to/cheb.bin")?;
    let ephemeris = Ephemeris::parse(std::fs::read(dataset)?)
        .map_err(|error| format!("Cannot load ephemeris: {error}"))?;

    let birth_moment = Epoch::from_gregorian_str("2000-01-01T12:00:00 UTC")?;
    let chart = calculate_chart(
        &ephemeris,
        ChartInput {
            epoch: birth_moment,
            location: Location {
                latitude: 51.5074,
                longitude: -0.1278,
            },
            house_system: HouseSystem::Equal,
            zodiac: ZodiacConfiguration {
                reference: ZodiacReference::Tropical,
                divisions: SignDivisions::Equal,
                ophiuchus: None,
            },
        },
    )?;

    println!("Birth moment: {birth_moment}");
    println!("Longitudes in degrees:");
    for body in &chart.bodies {
        println!("{:<20} {:>12.6}", body.values.name, body.values.longitude);
    }
    println!("House cusps in degrees: {:?}", chart.cusps);

    Ok(())
}
