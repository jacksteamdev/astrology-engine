# Astrology Engine

A Rust library for calculating astrology charts: planetary positions, chart
angles, and house cusps.

For a Fagan–Bradley sidereal chart, use `calculate_sidereal_chart` with the same
`ChartInput`. It returns the date's ayanamsa, sidereal positions, twelve house
cusps, and the actual house method (including Porphyry fallback). The tropical
`calculate_chart` API remains available. The sidereal model covers 1800–2200 TT;
chart dates must also lie within the supplied dataset.

For explicit zodiac and sign-division choices, use `calculate_configured_chart`.
It adds Tropical, Fagan–Bradley and Chimenti True Sky configuration, optional
Ophiuchus merging, sign assignments and zodiac-aware houses. See the
[convention contract](docs/chart-conventions.md) and the standalone
[browser/Wasm example](examples/configurable-chart/README.md).

## Calculate a natal chart

Supply a birth instant, latitude and longitude, and a house system.
`calculate_chart` returns the chart's body positions, angles, and optional house
cusps using a reusable ephemeris dataset.

To run the example, use the Rust toolchain specified in
[`rust-toolchain.toml`](rust-toolchain.toml). You also need an ephemeris dataset:
a binary file containing the data the library uses to calculate celestial
positions. The file must use the library's `HDCHEB01` format; the examples call
it `cheb.bin`.

Dataset files are not included in the repository. Follow the
[dataset generation guide](tools/README.md) to create one, then run this command
from the repository root, replacing `/path/to/cheb.bin` with your file's path:

```sh
cargo run --locked --example natal_chart -- /path/to/cheb.bin
```

The example uses a birth instant of **1 January 2000 at 12:00 UTC**, a London
location, and equal houses. Convert the person's local birth date and time to
UTC before calling the library. The dataset must cover the birth time and the
times 12 hours before and after it, which the library uses to calculate speeds.

The complete source is [`examples/natal_chart.rs`](examples/natal_chart.rs):

```rust
use astrology_engine::{calculate_chart, ChartInput, Ephemeris, Epoch, HouseSystem, Location};

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
        },
    )?;

    println!("Birth moment: {birth_moment}");
    println!("Longitudes in degrees:");
    for body in &chart.bodies {
        println!("{:<20} {:>12.6}", body.name, body.longitude);
    }
    if let Some(cusps) = &chart.cusps {
        println!("House cusps in degrees: {cusps:?}");
    }

    Ok(())
}
```

The example prints the body and angle longitudes and the twelve equal-house
cusps. Each body entry also contains longitudinal speed and declination. The
returned `ChartValues` contains the numerical results your application can use
to draw a chart.

Load and parse the ephemeris once, then reuse it for subsequent charts.

## Human Design design-date search

The library can also find the design instant used in Human Design: the time
before birth when the Sun's longitude was 88° less than its longitude at birth.
Starting with a natal chart, subtract 88° from the birth Sun's longitude, adjust
the result to the range from 0° inclusive to 360° exclusive, and search backward
with `find_sun_crossing` and `Direction::Backward`.

The offset is an angle; the search determines the corresponding date and time.
You can then call `calculate_chart` at that instant to obtain its planetary
positions, using the same ephemeris.

The [birth and design example](examples/birth_and_design.rs) demonstrates the
complete workflow:

```sh
cargo run --locked --example birth_and_design -- /path/to/cheb.bin
```

This search requires at least 89 days of dataset coverage before birth. The
dataset must also cover the returned design instant and the times 12 hours
before and after it, which the library uses to calculate speeds.

## Architecture and entry points

Read [From JPL data to a natal chart](docs/architecture.md) to learn how NASA JPL
data becomes an ephemeris dataset and how the library uses that dataset to
calculate charts.

- [Public Rust API](src/lib.rs): chart calculation, ephemeris evaluation, and
  solar longitude search.
- [Example Cloudflare Worker](examples/cloudflare-worker/): an HTTP adapter that
  loads the dataset from R2 and calls the library.
- [Deploy the Worker demo](docs/worker-runbook.md): generate a dataset, test
  locally, deploy a public demo in your Cloudflare account, and clean up.

## License and distribution

Astrology Engine is licensed under MPL-2.0; see [LICENSE](LICENSE) and
[NOTICE](NOTICE). Distributed changes to covered source files remain under MPL.
Separate applications can use their own licenses. Built distributions include
the first-party source archive and a notices page linking to it. Dependencies
retain their own terms; see
[third-party notices](THIRD_PARTY_NOTICES.md) and the
[notice preparation guide](tools/notices/README.md). Native and Wasm distributions
must carry the applicable notices and source-access information.
