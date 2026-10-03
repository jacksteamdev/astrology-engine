use astrology_engine::{coverage, Ephemeris, Epoch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let dataset = match (args.next(), args.next()) {
        (Some(path), None) => std::path::PathBuf::from(path),
        _ => {
            return Err(
                "Usage: cargo run --locked --quiet --example dataset_coverage -- /path/to/cheb.bin"
                    .into(),
            );
        }
    };
    let bytes = std::fs::read(&dataset)
        .map_err(|error| format!("Cannot read {}: {error}", dataset.display()))?;
    let ephemeris =
        Ephemeris::parse(bytes).map_err(|error| format!("Cannot load ephemeris: {error}"))?;
    let bounds = coverage(&ephemeris);
    let (min_year, ..) = Epoch::from_et_seconds(bounds.lo_et).to_gregorian_utc();
    let (max_year, ..) = Epoch::from_et_seconds(bounds.hi_et).to_gregorian_utc();

    println!("KERNEL_COVERAGE_MIN_YEAR={min_year}");
    println!("KERNEL_COVERAGE_MAX_YEAR={max_year}");
    Ok(())
}
