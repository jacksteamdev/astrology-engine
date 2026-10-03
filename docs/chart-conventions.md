# Configurable natal charts

`calculate_chart` accepts an explicit `ChartInput`, including its `zodiac` field
of type `ZodiacConfiguration`. References are Tropical, Fagan–Bradley and Chimenti True
Sky. Sign divisions can be equal or, for True Sky, constellation midpoints.
There is one chart entry point and one `ChartValues` result contract.

```rust
use astrology_engine::{
    calculate_chart, ChartInput, Ephemeris, Ophiuchus,
    SignDivisions, ZodiacConfiguration, ZodiacReference,
};

fn chart(ephemeris: &Ephemeris, input: ChartInput) -> Result<(), Box<dyn std::error::Error>> {
    let values = calculate_chart(
        ephemeris,
        ChartInput {
            zodiac: ZodiacConfiguration {
                reference: ZodiacReference::TrueSky,
                divisions: SignDivisions::Constellation,
                ophiuchus: Some(Ophiuchus::Enabled),
            },
            ..input
        },
    )?;
    println!("{}: {:?}", values.convention_revision, values.cusps);
    Ok(())
}
```

The result contains projected bodies and sign assignments, twelve cusps,
requested and actual house systems, effective configuration, reference offset,
speed reference, and effective zodiac sectors. Each sector has a stable sign
identifier, a start in degrees and a width in degrees. `house_sectors` is present
for Whole Sign and names the separate table used to construct those houses.

The runtime supplies no configuration defaults. Constellation divisions require
True Sky and an explicit enabled/disabled Ophiuchus choice. Equal divisions
ignore either toggle value; effective metadata returns
`ophiuchus: null`. Unknown configuration fields and unknown enum values are
rejected during deserialization. Serialized names use kebab case, such as
`true-sky`, `fagan-bradley`, and `whole-sign`; sign identifiers use lowercase.

## Recorded conventions

The convention revision is `astrology-engine-conventions-v1`. Regression fixtures
preserve the behavior of a previous version of the astrology engine. These
checks do not establish independent astronomical accuracy.

True Sky subtracts `31.21558087 + (JD - 2451545.0) * 0.000038247508` degrees.
JD follows the source's UTC JavaScript Date calculation, including millisecond
precision. There is no additional nutation correction. Its thirteen boundary
starts stay fixed across epochs. The offset helper admits the source Date range;
chart calculation additionally enforces the supplied dataset's coverage and
existing fitted-series support for speed samples. Fractional milliseconds in a
UTC instant do not affect the projection offset, but the ephemeris calculation
retains the original instant's precision.

True Sky preserves the source reading layer's tropical longitudinal speeds and
equatorial declinations. Consequently `speed_reference` is `tropical`, even
though the returned longitudes use True Sky. Fagan–Bradley applies the engine's
ayanamsa and its daily rate, returning `selected-reference` speeds. The four
angle speeds remain zero, meaning not calculated. Speeds are degrees per day;
longitudes, declinations, offsets, sector starts and widths are degrees.

`zodiac_sectors`, `assign_sign` and `whole_sign_cusps` are pure reusable functions.
Lookup uses half-open intervals; exact starts belong to the new sector, and
finite longitudes wrap into [0°, 360°). Non-finite values fail explicitly.
Disabling Ophiuchus removes its start boundary so Scorpio contains both sectors.
Whole Sign always uses twelve sectors and merges Ophiuchus independently of the
zodiac toggle. Equal and Placidus cusps do not depend on sign labels. Placidus
reports Porphyry fallback; any resulting house set that overlaps is rejected.

For a projected Ascendant at 228° with constellation divisions and Ophiuchus
enabled, the zodiac label is Ophiuchus, 4.5755° into its 12.3573° sector. House 1
starts in the merged Scorpio sector at 210.1972°. The twelve house starts are:

```text
210.1972, 235.7818, 269.2677, 294.8435, 318.0103, 0,
19.7286, 56.5875, 86.0412, 103.19, 141.6065, 191.32
```

With Ophiuchus disabled, the same longitude is Scorpio, 17.8028° into its
25.5846° sector. The house starts are unchanged.

Tropical preserves the underlying numerical positions. At extreme latitudes,
floating-point angle calculations can round to 360° (equivalent to 0°); sign
lookup wraps those longitudes into [0°, 360°).

## Migrating the chart API

Replace `calculate_sidereal_chart` and `calculate_configured_chart` calls with
`calculate_chart`, and put the chosen `ZodiacConfiguration` in `ChartInput.zodiac`.
Former tropical callers must explicitly choose Tropical with Equal divisions and
`ophiuchus: None`. The runtime supplies no defaults or legacy wrappers.

`ChartValues` replaces the former configured and sidereal result types. Each
body is a `ChartEntry` with `values: ChartBody` and `sign: SignPosition`; serialized
body fields remain flattened. Use `reference_offset_degrees` for the reference
offset (the ayanamsa for Fagan–Bradley), and the typed `house_system` for the actual
method. Cusps are always `[f64; 12]`, including tropical Whole Sign houses.
Overlapping polar house sets now fail for every reference.

The Worker example continues to use its existing tropical HTTP request and
response fields, including omitted Whole Sign cusps. Overlapping polar charts
follow its existing calculation-error mapping: public HTTP 500 with `internal`,
and the explicit overlap message in verification captures.

## Browser and host integration

The [standalone browser/Wasm example](../examples/configurable-chart/README.md)
accepts caller-supplied dataset bytes, an explicit instant, coordinates and
configuration. It requires no private resolver or website packages. Native hosts
can parse `Ephemeris` once and reuse it for many calls. Dataset identity is
separate from the engine revision. Resolving place names and local birth times
remains the host's responsibility.

## Verification

[Regression replay instructions](../tools/verification/conventions.md) describe
how to compare current outputs with the committed fixtures. Structural checks
also cover invalid settings, non-finite input, coverage admission and unified
chart behavior.

Boundary tables and assignments match the recorded outputs exactly.
Chart comparisons use a 1e-12 absolute tolerance for native/Wasm floating-point
roundoff in degrees or degrees/day. The recorded Wasm run's largest residual was
1.14e-13 degrees; speeds matched exactly. This release also retains the existing
frozen source-Wasm parity checks for tropical charts and Sun searches.
