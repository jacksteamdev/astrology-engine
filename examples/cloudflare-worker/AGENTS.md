# Worker integration contracts

The root guidance applies. This crate is excluded from the root workspace;
root Cargo checks do not cover it. Use the separate manifest and commands in
the [verification workflow](../../.github/workflows/verify.yml).
The [runbook](../../docs/worker-runbook.md) owns setup, configuration, local
requests, deployment and cleanup procedures.

## Keep adapter and runtime responsibilities aligned

- Treat R2 bytes, `EPHEMERIS_KEY`, `EPHEMERIS_SHA256` and coverage-year variables
  as one artifact configuration. Obtain identity and actual coverage from the
  generated artifact using the runbook; illustrative years and the checked-in
  reference hash are not defaults for every generated dataset. The year gate
  is coarse; the runtime still enforces its exact ET domain and search admission.
- Cache identity is the object key plus expected hash. Publish a parsed value
  only after loading, hashing and parsing succeed so failed initialization can
  recover. In the [local harness](verify.mjs), cold/warm requests and retry after
  invalid storage exercise this lifecycle across the R2 and Rust boundaries.
- HTTP representation is distinct from library values: Whole Sign
  cusps are omitted in JSON even though the runtime returns twelve cusps. Public
  search results discard sub-minute precision. The [capture adapter](src/verification.rs) retains floating-point
  bits and precise instants for parity. Keep public formatting separate from
  capture transport, including lossless large-integer handling in `verify.mjs`.

## Build modes and validation

`build:verification` enables internal routes used by the shared
[structural cases](../../tools/verification/structural.rs), capture/parity and
dataset generation. `build` produces the ordinary adapter without those routes.
Both write the same build output: rebuild the intended mode before running its
checks or deploying. An ordinary build cannot run the verification harness;
a verification build is not the public demo artifact.

For a changed adapter or runtime, build verification mode here and run
`node examples/cloudflare-worker/verify.mjs` from the repository root, following
the tools guide's prerequisites. Check ordinary-build route exclusion separately
when changing feature gates. Miniflare exercises local Wasm and emulated R2;
public HTTP validation, hosted storage and deployment remain separate checks.
