# Ephemeris extraction verification — 2026-10-03

The native extractor preserves coefficient bytes and source fitted guard margins.
The engine runtime, HDCHEB01 format and R2 Worker adapter are unchanged.

## Author checks

- `cargo test --locked --workspace`: passes; existing source-capture conventions
  test remains explicitly ignored without its separate captured dataset.
- Workspace formatting/Clippy and browser-example Wasm formatting/Clippy pass.
- Browser example Wasm build, TypeScript check and production build pass.
- Runtime dependency/ambient-I/O boundaries, comparator negative controls,
  acquisition/generation checks with hash-locked Python dependencies, and usage
  policy integration tests pass.
- Four extractor tests cover byte copying, guards, exact support boundaries,
  narrow source endpoints, corrupt CRCs, duplicate/nonfinite series, bad offsets,
  insufficient support and explicit UTC parsing.
- `verify-segments.ts` executed actual Wasm against source SHA-256
  `06d4c26e589207bf479dbd217fafb3e4d6c559626425b156912e39a4d859487b` and 300
  extracted annual files. It compared 4,713 natal charts and 1,570 design charts
  at seasonal/New Year/leap-day samples. Maximum numerical difference was zero;
  design crossing ETs matched exactly. One source-boundary admission rejection
  was preserved. Unexpected search errors fail the harness.

Reproduce using the commands in `examples/configurable-chart/README.md` and
`tools/README.md`, supplying the same immutable input for identity comparisons.
The website's safe birth range uses 300 years; the generic example recipe uses
source public bounds, which may additionally include a partial boundary year.
No full-data regeneration or new astronomical accuracy claim is made.

## Independent review

A separate reviewer found no blocking issues. Exact-support boundary and
malformed-input test gaps were filled. Manifest UTC strings use ISO Z, and the
browser rejects non-finite parsed coverage. Expected parity rejections are
restricted to the existing source-boundary admission rule.

The reviewer independently ran the four compiled extractor tests successfully;
a fresh Cargo invocation was sandbox-blocked on its build lock. Fresh workspace
build/test results above are author checks. Cross-dataset parity is also
same-engine author evidence, not independent astronomical validation.
