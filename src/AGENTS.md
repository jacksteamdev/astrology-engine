# Runtime contracts

The root guidance applies. Keep this layer free of ambient I/O and mutable
global state. Callers own dataset loading and reuse the parsed ephemeris.
Run the runtime boundary check listed in the [tools guide](../tools/README.md)
when changing dependencies or moving responsibilities across crates.

## Changes that cross module boundaries

- **Binary compatibility:** [cheb/format.rs](cheb/format.rs) is shared by the
  runtime parser and the native writer through `tooling` exports in
  [lib.rs](lib.rs). Coordinate format or series changes with
  [the builder](../tools/dataset-builder/src/main.rs), evaluation and the
  [structural harness](../tools/verification/structural.rs); do not fork a
  second format definition in the builder.
- **Coverage:** public admission uses the blob header domain, while
  [cheb/eval.rs](cheb/eval.rs) samples the fitted series span. Chart speeds
  need samples half a day on either side. The builder guards the public domain
  inside fitted support. Preserve this relationship when changing either
  coverage or finite differences; test public endpoints as well as interior
  dates, rather than assuming a successful central longitude proves a chart
  can be calculated.
- **Search:** [search.rs](search.rs) applies the 89-day lookback admission rule
  to both directions. The solver in [astro/find_moment.rs](astro/find_moment.rs)
  separately bounds its coarse scan to 730 days in the chosen direction;
  unavailable samples become NaN at the wrapper. Admission does not promise a
  crossing. Preserve the distinction between an error and `Ok(None)` through
  the Worker adapter when changing search behavior.
- **Chart compatibility:** fitted series, derived bodies, appended angles and
  wire names live across [cheb/chart.rs](cheb/chart.rs),
  [cheb/eval.rs](cheb/eval.rs), [chart.rs](chart.rs) and [types.rs](types.rs).
  Body order and derived values are observable in Worker output and frozen
  captures. Treat changes to those conventions or house fallback/cusp behavior
  as behavior changes, even when a different formula seems more natural.

## Verification path

The structural cases live outside this directory and execute through the
Worker's `verification` feature in workerd. Follow
[the Worker guidance](../examples/cloudflare-worker/AGENTS.md) to build and run
them after relevant runtime changes. Frozen parity additionally needs the exact
dataset in [the fixture manifest](../tests/fixtures/manifest.json); a fresh
dataset is not an interchangeable baseline. Use
[the tools guide](../tools/README.md) for that separate check.
