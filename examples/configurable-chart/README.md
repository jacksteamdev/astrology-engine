# Configurable chart in a browser

This example runs the public `calculate_chart` API as Wasm. The browser fetches
`/ephemeris.bin` from the local server and reuses those bytes for subsequent
calculations. Enter a UTC birth time, coordinates and chart settings. There are
no private website or birth-resolver dependencies.

First, follow the [dataset generation guide](../../tools/README.md#build-a-dataset)
to create an `HDCHEB01` ephemeris, or use an existing compatible dataset. Set
`EPHEMERIS_FILE` below to the absolute path of the resulting `cheb.bin` file.

Use Rust 1.96, Bun 1.3.14 and wasm-bindgen-cli 0.2.122:

```sh
cargo install wasm-bindgen-cli --version 0.2.122 --locked
cd examples/configurable-chart
bun install --frozen-lockfile
bun run build:wasm
bun run check
EPHEMERIS_FILE=/absolute/path/to/cheb.bin bun run dev
```

`WASM_BINDGEN=/absolute/path/to/wasm-bindgen bun run build:wasm` selects an
existing compatible CLI. Open the local URL printed by Vite. The server exposes
only the configured dataset at `/ephemeris.bin`; it reads the existing file
without copying it into the repository.

`bun run build` creates the static site in `build/`. A static host must provide
the dataset at `/ephemeris.bin` alongside that site. The dataset is not bundled
into the build. Wasm assets, site output, installed dependencies and compiler
outputs are ignored by Git.

The exported Wasm function accepts bytes and a JSON request:

```typescript
import init, { calculate } from './engine/chart.js';

await init();
const response = await fetch('/ephemeris.bin');
if (!response.ok) throw new Error('Could not load the ephemeris');
const datasetBytes = new Uint8Array(await response.arrayBuffer());
const chart = JSON.parse(calculate(datasetBytes, JSON.stringify({
  instant: { scale: 'utc', value: '2000-01-01T12:00:00Z' },
  latitude: 51.5074,
  longitude: -0.1278,
  house_system: 'whole-sign',
  configuration: {
    reference: 'true-sky',
    divisions: 'constellation',
    ophiuchus: 'enabled',
  },
})));
```

The adapter also accepts `{ scale: 'et', value: seconds }` for precise replay of
source ET inputs. UTC strings must explicitly end in `Z` or ` UTC`. Calls are
stateless and parse the supplied bytes each time; a native application can reuse
a parsed `Ephemeris` with the library API.

The result includes effective settings, the convention revision, sign sectors,
twelve house cusps, and requested/actual house systems. True Sky preserves
tropical speeds, identified by `speed_reference: "tropical"`.
See [the convention contract](../../docs/chart-conventions.md) for the separate
zodiac and house tables, Ophiuchus handling, and Fagan–Bradley conventions.

To replay recorded regression charts through this Wasm adapter:

```sh
bun run verify /path/to/reference-cheb.bin ../../target/conventions-wasm-report.json
```

The verifier requires the exact dataset hash in
`tests/fixtures/conventions.json`; dataset bytes are independent of the engine's
Git revision. Frozen captures include the dataset endpoints, polar house cases,
and all supported Tropical/True Sky division and Ophiuchus settings. Fagan–Bradley
serialization and all house methods also receive Wasm smoke checks.

## Distribution notices

Wasm preparation and the static build include `LICENSE`,
`THIRD_PARTY_NOTICES.txt`, `THIRD_PARTY_NOTICES.html`, and
`RUST_LIBRARY_NOTICES.html`. Preserve those files
when publishing the output. The page's Third-party notices link provides the
license texts and exact-version MPL source links to recipients. Builds fail if
the checked-in notices are stale; refresh them using the
[notice guide](../../tools/notices/README.md).

## Optional two-year browser downloads

The full-file example remains available. To select a small dataset by UTC birth
year, build the native extraction tool and prepare an empty output directory:

```sh
# From the repository root:
cargo build --release --locked -p dataset-builder
cd examples/configurable-chart
bun prepare-segments.ts /absolute/path/to/cheb.bin /absolute/path/to/new-segments
VITE_EPHEMERIS_SEGMENTS=true EPHEMERIS_SEGMENTS_DIR=/absolute/path/to/new-segments bun run dev
```

For a birth in year Y the recipe extracts Y-1 through Y+1, with a day of public
padding and the source's fitted guards. The original coefficients are copied,
not fitted again. Date changes reuse the Wasm instance, passing different bytes
to the existing stateless calculation export. The browser verifies SHA-256 and
keeps three recent files in memory. A static deployment must separately publish
the generated directory at `/ephemeris/` and build with
`VITE_EPHEMERIS_SEGMENTS=true`; the local environment path is not bundled.

This overlap accommodates a natal chart and the Human Design Sun crossing 88°
before birth plus its design chart. It is not a promise for arbitrary searches
up to the solver's 730-day scan bound. Source coverage still limits earliest
birth/design dates. No Human Design UI is added by this example.

After building Wasm, `bun verify-segments.ts /source/cheb.bin /segments` compares
full and extracted data for every birth year, seasons, New Year edges and leap
days, including the `design_moment_et(bytes, utc)` export and design charts.
This is same-engine Wasm parity, not independent astronomical validation.
Coefficient bytes are unchanged; arbitrary fractional block origins may still
introduce evaluation rounding when rebased. Real-data parity is checked at
1e-12 degrees, with exact design crossing ET equality.
