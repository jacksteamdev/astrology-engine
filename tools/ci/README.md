# Regular CI

The Verify workflow runs once for each PR update, on pushes to `main`, and on
manual dispatch. Feature branches without a PR can use manual dispatch. New PR
updates cancel superseded runs. Main and manual runs always select every job.

PR selection uses the merge base of the PR base/head commits. Renames include
both paths, and unavailable comparisons select everything. The `changes` job
runs the CI helper tests and records its selections in the run summary:

| Changes | Checks |
| --- | --- |
| Prose documentation | Usage policy, helper tests, final gate |
| Worker example source | Worker, notices |
| Browser example source | Browser, notices |
| Native builder or Python preparation source | Native, preparation tooling, notices |
| Runtime, shared verification, CI, licensing, notice inputs, unknown files | Everything |

Notice input fingerprints from both base and head take precedence over prose
and adapter-specific rules. Executable documentation is code. Native tests,
preparation tooling, and Worker integration run independently. The final
`verify` job accepts intentionally skipped jobs and rejects selected jobs that
fail, are cancelled, or unexpectedly skip. No deployment or full-data generation
occurs in regular CI.

## Caches and tool downloads

Rust dependency caches are separate for each job and workspace, keyed by the
Rust toolchain, manifests/lockfiles and compiler environment. They exclude
workspace build products. Bun/pip caches hold package downloads, not installed
application output. Dependency installation remains locked and hash-checked.

The `ci-tools` composite action caches tools separately from source compilation.
Its key includes platform, tool versions, download checksums, setup code and the
Rust toolchain. Successful setup saves its cache immediately so later failures
do not discard it. Every restored executable must report the expected version.

`worker-build` and `cargo-about` are installed with exact versions and `--locked`
on a cold cache. Their versions come from the existing notice tool constants.
`downloads.json` records the official Linux x64 helper archives and SHA-256
hashes: wasm-bindgen 0.2.122, Binaryen 129, and esbuild 0.28.0. These match the
Worker build tool's resolver. Archives are verified before extraction; cached
binaries are version checked. Update URLs, checksums, expected version output,
and corresponding notice material together when intentionally upgrading tools.

Transport errors, HTTP 408/429 and 5xx responses get at most three attempts,
with 2/5-second backoff and a 60-second timeout per attempt. Other HTTP errors,
checksum mismatches, wrong versions, compilation errors and test failures are
not retried. Cargo uses its bounded network retry setting during installation.
Tool setup exports the supported Worker binary overrides and the browser's
`WASM_BINDGEN` override, so builds use the validated binaries. Wasm optimization
stays enabled. Local developer build setup is unchanged.

To invalidate a broken tool cache, delete its entry in GitHub Actions caches;
never bypass version or checksum checks. No final Wasm/site/native distribution,
notice bundle, source archive, or dataset is restored as a CI artifact cache.

## Local checks

```sh
bun test tools/ci
bun tools/ci/changes.ts # without a PR event, selects every job
bun tools/ci/setup.ts info worker # prints the platform/tool cache identity
bun examples/configurable-chart/node_modules/typescript/bin/tsc --project tools/ci/tsconfig.json
actionlint
```

TypeScript checking uses the browser example's locked development dependencies.
The notices job installs them and checks both notice and CI tooling, independent
of whether browser integration was selected. The full-data workflow retains its
explicit invocation and existing setup.
