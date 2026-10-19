# Regression ownership after the website migration

The library owns numerical calculations, native/Wasm consistency and extraction
parity. Website CI owns its adapter presets and state, segment selection/loading,
course/UI code and packaged asset delivery. Website releases do not run this
repository's verification workflows.

`tests/true_sky_source.rs` preserves the original True Sky source fixture from the
website. It runs with `cargo test --locked --workspace` without external data.

The eight historical Fagan-Bradley charts are preserved byte-for-byte in
`tests/fixtures/fagan-bradley-71350f0.json`. They were captured from engine revision
`71350f0889c3b5e1585935d2017c97efe0bb5848`, before the chart API migration.
Their dataset SHA-256 is
`06d4c26e589207bf479dbd217fafb3e4d6c559626425b156912e39a4d859487b`.
This differs from the dataset used by `tests/fixtures/conventions.json`.
Use authorized local bytes or the explicit full-data workflow's separate
`fagan_bradley_dataset_url` input; no private website credential belongs here.

```sh
FAGAN_BRADLEY_DATASET=/absolute/path/cheb.bin cargo test --locked --test fagan_bradley_legacy -- --include-ignored
cd examples/configurable-chart
bun install --frozen-lockfile
bun run build:wasm
bun verify-fagan-bradley.ts /absolute/path/cheb.bin
```

Both runtimes compare numbers within an absolute tolerance of 1e-12 and compare
all other captured values and ordering exactly. Tiny arithmetic rounding is
allowed; fixtures must never be regenerated to hide an implementation change.
Comparator negative controls reject changes above tolerance and reordered bodies.

The explicit full-data workflow also prepares segments and runs
`verify-segments.ts` against its production dataset. Full-data checks remain
manual, separate from normal source CI and from independent astronomical accuracy.
