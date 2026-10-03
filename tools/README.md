# Dataset and verification tools

Use these tools to acquire JPL inputs, generate the ephemeris dataset consumed by
Astrology Engine, and check the generated data and runtime behavior. For the
data flow, read [From JPL data to a natal chart](../docs/architecture.md).

Run the commands below from the **repository root**.

## Tool map

| Path | Purpose |
| --- | --- |
| [`regenerate.py`](regenerate.py) | Coordinates recorded JPL acquisition, kernel preparation, coefficient generation, local Wasm validation, and provenance. |
| [`jpl/`](jpl/) | Python helpers for Horizons vectors, Chebyshev fitting into intermediate SPK kernels, SPK writing, and validation. |
| [`dataset-builder/`](dataset-builder/) | Native Rust CLI that reads the source kernels through ANISE and produces the final `HDCHEB01` coefficient dataset. |
| [`verification/`](verification/) | Acquisition and fitting checks, runtime boundary checks, and comparison with the frozen Wasm regression baseline. |

## Prepare the environment

The [full-data CI workflow](../.github/workflows/full-data.yml) uses Python 3.13,
Node.js 22, and Bun 1.3.14. Have those available, along with `rustup` and a native
Rust build environment. The repository pins Rust 1.96.0 in
[`rust-toolchain.toml`](../rust-toolchain.toml).

Install the Python dependencies into a local environment:

```sh
python3.13 -m venv .venv
.venv/bin/python -m pip install --require-hashes -r tools/jpl/requirements.lock
```

Prepare the native builder and the local Wasm verification adapter:

```sh
rustup toolchain install 1.96.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
cargo install worker-build --version 0.8.3 --locked
cargo build --locked --release -p dataset-builder
(
  cd examples/cloudflare-worker
  bun install --frozen-lockfile
  bun run build:verification
)
```

The verification build is required: generation invokes the example Worker's
local validation routes through Miniflare/workerd. This runs locally and does
not deploy a Worker or require a Cloudflare account. Node is invoked explicitly
by the generation script, even though Bun installs the adapter dependencies.

## Acquire JPL inputs

```sh
.venv/bin/python tools/regenerate.py acquire --cache data/cache/jpl
```

This downloads DE440s from JPL's NAIF archive and Ceres/Chiron vectors from JPL
Horizons, including separate samples for checking the intermediate fits. Each
cached response has request metadata and a SHA-256 hash; `acquisition.json` records the completed
input set. Reusing a cache checks its recorded requests and hashes.

Acquisition needs network access for inputs absent from the cache. Keep the
completed cache to rebuild from the same recorded responses. To acquire a new
input set, use a different cache directory.

## Build a dataset

```sh
.venv/bin/python tools/regenerate.py build --cache data/cache/jpl --out data/generated/first
```

The output directory **must not already exist**. The build reads the recorded
JPL responses without reacquiring them, prepares and checks the Ceres/Chiron
kernels, runs the Rust coefficient fitter, checks the output hash, and runs local
Wasm validation. `--builder /path/to/dataset-builder` can select a different
builder executable; the default is `target/release/dataset-builder`.

Successful output includes:

| File | Role |
| --- | --- |
| `cheb.bin` | The coefficient binary to supply to `Ephemeris::parse`. |
| `manifest.json` | Dataset SHA-256, byte size, coverage, and series summaries. |
| `residual-report.json` | Final coefficient fitting results. |
| `de440s.bsp`, `ceres.bsp`, `chiron.bsp` | Kernels used during preparation. |
| `kernels.json` | Intermediate Ceres/Chiron fit and validation results. |
| `wasm-validation.json` | Local Worker validation results for this dataset. |
| `provenance.json` | Completion record with input/output hashes, builder identity, environment, and command. Written after the checks succeed. |

A failed run can leave a partial output directory. Inspect the error and use a
new output path for the next attempt. The presence of `cheb.bin` alone does not
mean the later validation steps completed.

Run a chart with the resulting dataset:

```sh
cargo run --locked --example natal_chart -- data/generated/first/cheb.bin
```

Use the **generated** manifest for this artifact's hash and coverage. The
checked-in [`data/manifest.json`](../data/manifest.json) describes a historical
reference dataset. Freshly acquired inputs are not promised to reproduce that
reference's bytes, and generation does not update the example Worker's configured
dataset hash or storage object.

## Check repeatability

With the same cache and builder executable, generate into another fresh directory
and compare the coefficient binaries:

```sh
.venv/bin/python tools/regenerate.py build --cache data/cache/jpl --out data/generated/repeated
cmp data/generated/first/cheb.bin data/generated/repeated/cheb.bin
```

`cmp` exits successfully without output when the bytes match. The builder
executable's fingerprint is embedded in the dataset, so keep the executable fixed
for this check. Provenance records can differ between runs.

## Run focused checks

After the environment setup, these Python checks use local fixtures or mocked
responses:

```sh
.venv/bin/python tools/verification/test_compare.py
.venv/bin/python tools/verification/test_acquisition.py
.venv/bin/python tools/verification/test_generation.py
.venv/bin/python tools/verification/check_boundaries.py
```

They check result comparison, acquisition/cache failure handling, a small pinned
Ceres fit and SPK readback, and separation of the runtime from host dependencies
and ambient I/O. The boundary check runs `cargo tree --offline`, so Rust
dependencies must already be cached.

With the verification adapter built, run its local fixture checks:

```sh
node examples/cloudflare-worker/verify.mjs
```

To check a particular dataset separately and save the report:

```sh
node examples/cloudflare-worker/verify.mjs --dataset data/generated/first/cheb.bin --report data/generated/first/recheck.json
```

### Compare with the frozen Wasm regression baseline

This check requires the exact dataset identified by
[`tests/fixtures/manifest.json`](../tests/fixtures/manifest.json). The script
rejects other dataset hashes, including a newly generated dataset with a
different identity.

```sh
.venv/bin/python tools/verification/parity.py --dataset /path/to/reference-cheb.bin --report /tmp/astrology-parity.json
```

It replays the recorded inputs against the local Wasm runtime and compares
results by identifier. For existing JSONL captures, the comparison tool can also
run directly:

```sh
.venv/bin/python tools/verification/compare.py /path/to/expected.jsonl /path/to/actual.jsonl --report /tmp/astrology-comparison.json
```

Each capture row contains `id` and `value`; duplicate identifiers are rejected.
These comparisons check behavior preservation. They do not establish independent
astronomical accuracy.

See the [regular verification workflow](../.github/workflows/verify.yml) for the
repository's formatting, linting, packaging, and local Wasm checks, and the
[full-data workflow](../.github/workflows/full-data.yml) for explicit regeneration
and reference-dataset parity runs. The [CI guide](ci/README.md) describes PR
selection, independent jobs, tool caching and download failure handling.

## Commit checks

Install the repository hook after cloning:

```sh
git config --local core.hooksPath .githooks
```

The hook checks the proposed commit, including dependency files and compressed
fixtures, against the usage prohibition in [AGENTS.md](../AGENTS.md). Prose
documentation is exempt. Run the same check explicitly with
`python3 tools/verification/check_prohibited_usage.py --index`, or use
`--tree HEAD` to check a committed revision. CI also runs the checker and its
integration tests.

## Distributing the native builder

Run `bun tools/notices/notices.ts package-builder` from the repository root to
create `target/distribution/dataset-builder/` with the executable and its license
notices. Distribute that directory together. A plain `cargo build` compiles the
program but does not assemble a redistributable notice bundle. See the
[notice guide](notices/README.md) for dependency updates and MPL source access.
