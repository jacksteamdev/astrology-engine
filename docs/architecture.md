# From JPL data to a natal chart

Astrology Engine turns astronomical source data into reusable data for chart
calculations. Host-side tooling prepares the dataset; the Rust library evaluates
it for a birth instant and combines the results with birthplace-dependent chart
geometry.

![Pipeline from JPL DE440s and Horizons data through prepared kernels, chart-coordinate samples, and Chebyshev fitting into cheb.bin. At runtime, the Rust library evaluates the dataset for a birth instant and combines those positions with location and house system to produce natal chart values.](assets/jpl-to-natal-chart.png)

## 1. Acquire JPL source data

The preparation pipeline starts with two sources:

- **DE440s:** a JPL ephemeris kernel supplying planetary, Earth, and Moon states.
- **JPL Horizons:** position and velocity samples for Ceres and Chiron.

The acquisition step records the requests, downloaded responses, and their
SHA-256 hashes. Later generation can reuse those recorded inputs. The
[`acquire` stage](../tools/regenerate.py) is the part that contacts JPL.

## 2. Prepare chart-coordinate samples

The build step retains DE440s and fits the recorded Ceres and Chiron vectors into
SPK kernels, a format for evaluating positions over time. It checks those fitted
kernels against separately sampled Horizons vectors before continuing.

The native Rust dataset builder reads the kernels through ANISE. It converts
their position data into the quantities the chart runtime needs: geocentric,
tropical longitudes of date and declinations, using the builder's light-time and
frame corrections. It also derives a separate lunar-node longitude series from
the Moon's motion relative to Earth.

These coordinate transformations happen during preparation. Their results become
the reference samples for the final fit.

## 3. Fit and package Chebyshev coefficients

The builder divides each coordinate series into time segments and fits a
Chebyshev polynomial to each segment. Each polynomial represents how one
coordinate changes over its time interval. Different series can use different
segment lengths and polynomial degrees.

The builder checks the fitted values against reference evaluations and rejects a
dataset when its residual checks fail. Successful output includes:

- **`cheb.bin`:** the `HDCHEB01` binary containing the coefficient series, segment
  metadata, and coverage information.
- **`manifest.json`:** the generated artifact's identity, coverage, and fit
  summaries.
- **`residual-report.json`:** results of the final coefficient fitting checks.

The orchestration script then checks the binary's hash, runs local Wasm
validation, and records input/output provenance. The fitting checks describe how
well the generated data follows its reference calculations; they do not alone
establish independent astronomical accuracy.

The **SPK kernels are preparation inputs**. The **coefficient binary is the data
the chart runtime consumes**.

## 4. Evaluate the dataset at the birth instant

The caller supplies the coefficient binary to `Ephemeris::parse` and reuses the
parsed ephemeris across calculations. File access, storage, and checking the
artifact's SHA-256 belong to the caller; the library accepts the bytes.

For a natal chart, `calculate_chart` takes that ephemeris, an unambiguous birth
instant, latitude and longitude, a house system, and explicit zodiac settings. The caller resolves local
time and timezone ambiguity before passing the instant.

For each stored body coordinate, the runtime selects the segment containing the
birth instant and evaluates its polynomial. It obtains longitude and declination
from the fitted series, and calculates longitudinal speed from nearby longitude
evaluations. It also assembles the Earth and lunar-node entries used by the
chart.

This stage needs the prepared binary and a time. It makes no JPL requests and
does not load the preparation kernels.

## 5. Assemble the natal chart

The runtime combines those body values with angles and houses calculated from
the birth instant, birthplace, and selected house system. Birthplace affects this
chart geometry; it does not alter the stored geocentric body positions.

The result is `ChartValues`: body and angle entries with sign assignments, twelve
house cusps, and metadata describing the effective zodiac and actual house method. These are the numerical values an application can use to display a
chart. The current result contains 15 body/node entries and four chart angles.

The same pipeline supports the Human Design design chart's planetary positions.
The caller first calculates a tropical birth chart, subtracts 88° from the birth Sun's
longitude, and asks `find_sun_crossing` to search backward. It then calls
`calculate_chart` at the returned instant, reusing the same ephemeris. Human
Design gate calculations remain outside this library. The
[runnable example](../examples/birth_and_design.rs) demonstrates this composition.

## Component boundaries

| Component | Responsibility |
| --- | --- |
| [`tools/regenerate.py`](../tools/regenerate.py) and [`tools/jpl/`](../tools/jpl/) | Acquire and record JPL inputs, prepare the Ceres/Chiron kernels, and coordinate generation and checks. |
| [`tools/dataset-builder/`](../tools/dataset-builder/) | Evaluate source kernels, prepare chart coordinates, fit coefficients, and write the runtime dataset. ANISE belongs here. |
| [Rust library](../src/lib.rs) | Parse supplied coefficient bytes, evaluate them, calculate charts, and search solar longitude crossings. |
| [Example Cloudflare Worker](../examples/cloudflare-worker/) | Load and hash-check the dataset from R2, cache the parsed ephemeris, and expose the library over HTTP. It is a separate adapter. |

Fresh generation writes its own manifest and provenance under the chosen output
directory. The checked-in [`data/manifest.json`](../data/manifest.json) identifies
a historical reference dataset; newly acquired JPL inputs are not promised to
reproduce that artifact's bytes. Consumers must use the identity and coverage of
the dataset they actually load.

For setup commands, generation, and verification, see the
[tools guide](../tools/README.md). Return to the
[README and runnable workflow](../README.md).
