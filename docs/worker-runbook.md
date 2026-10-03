# Deploy the example Worker as a demo

Build a dataset from official JPL inputs, try the HTTP API locally, then deploy
it in your own Cloudflare account. The result is a public `workers.dev` endpoint
backed by a private R2 bucket. A **Worker** runs the HTTP adapter; **R2** stores
the ephemeris file; a **binding** lets the Worker read that bucket.

This example demonstrates the Rust library over HTTP. It has no authentication,
application rate limiting, or browser CORS handling and is not a full production
implementation. Use the sample requests below and remove the demo when finished.

The walkthrough was tested on macOS 15.5 arm64 with an existing Cloudflare
account. Fresh OS installation, Linux, minimum Node 22, and first-time account
activation have not been tested.

## How the Worker fits together

![Worker architecture: a client sends HTTPS requests to the Rust Wasm adapter, which validates input and obtains a parsed ephemeris from its isolate-local cache. A cold load reads cheb.bin from private R2 through the EPHEMERIS binding, checks SHA-256, and parses the bytes. The Rust library calculates a chart or solar crossing; the adapter returns JSON. Separate local tooling generates the dataset from official JPL inputs and uploads it before serving requests.](assets/worker-architecture.png)

The adapter owns HTTP validation, R2 access, hashing, caching, and response
formatting. The library receives parsed data and calculation inputs; it makes
no storage or network calls. Each Worker isolate maintains its own cache, keyed
by object key and expected hash. Warm requests reuse that data without reading
R2 again. Invalid requests can return early, and `/health` never loads data.

Dataset generation happens before deployment, outside the request path. For
that pipeline, see [From JPL data to a natal chart](architecture.md).
The diagram's [editable SVG](assets/worker-architecture.svg) is included.

## 1. Prepare a checkout and tools

Start with a fresh checkout of this repository. Open a Bash or Zsh terminal at
its root: the directory containing `Cargo.toml`, `rust-toolchain.toml`, and
`tools/`. Commands use macOS/Linux shell syntax. Stop when a command fails;
resolve it before continuing. Windows users need a suitable Linux environment.

Install these tools if needed:

| Tool | Version used by this repository | Installation reference |
| --- | --- | --- |
| Rust via rustup | 1.96.0, including `wasm32-unknown-unknown` | [Rust installation](https://www.rust-lang.org/tools/install) |
| Native compiler/linker | Required by Rust builds | macOS Command Line Tools (`xcode-select --install`), or your Linux distribution's build tools |
| Python | 3.13, with `venv` and pip | [Python downloads](https://www.python.org/downloads/) |
| Node.js | 22 or later | [Node.js downloads](https://nodejs.org/en/download) |
| Bun | 1.3.14 | [Bun installation](https://bun.sh/docs/installation) |
| curl | Available on the command line | [curl downloads](https://curl.se/download.html) |

Bun installs the pinned JavaScript dependencies; Node runs the verification
harness. The native Rust builder prepares the data, while the Worker is built
to WebAssembly. Python dependencies are pinned with hashes.

From the **repository root**:

```sh
rustup toolchain install 1.96.0 --profile minimal --component rustfmt --component clippy --target wasm32-unknown-unknown
cargo install worker-build --version 0.8.3 --locked
python3.13 -m venv .venv
.venv/bin/python -m pip install --require-hashes -r tools/jpl/requirements.lock
cargo build --locked --release -p dataset-builder
(
  cd examples/cloudflare-worker
  bun install --frozen-lockfile
  bun run build:verification
)
```

This installs Wrangler 4.112.0 and Miniflare 4.20260714.0 from `bun.lock`.
Keep the example's compatibility date, `2024-11-01`, unchanged for this guide.
Record the checkout and installed versions with your test evidence:

```sh
git rev-parse HEAD
rustc --version
cargo --version
worker-build --version
python3.13 --version
node --version
bun --version
```

Run the fixture checks from the **repository root**:

```sh
node examples/cloudflare-worker/verify.mjs
```

Expect exit code zero, `"passed": true`, and `"structural": { "passed": 6 }`.
Without `--dataset`, this uses a synthetic fixture. It checks HTTP validation,
missing/corrupt data, retry, caching, chart shape, and solar searches. It does
not establish that a real ephemeris works or that anything has been deployed.

## 2. Generate the dataset

These steps run locally; no Cloudflare account is needed yet. Acquisition needs
internet access to JPL. Generation fits roughly three centuries of data and can
use substantial CPU and memory; no completion time is promised.

From the **repository root**, download and record the official inputs:

```sh
.venv/bin/python tools/regenerate.py acquire --cache data/cache/jpl
```

This acquires DE440s from JPL's NAIF archive and Ceres/Chiron vectors from JPL
Horizons. `data/cache/jpl/` contains raw responses, request metadata, hashes,
and `acquisition.json`. Reusing the cache checks the recorded requests and
hashes; retain it to rebuild from the same inputs. See the
[generation tool guide](../tools/README.md) for the pipeline and repeatability.

Build into a **new, nonexistent output directory**. The parent directory may
already exist. Save the log as generation evidence:

```sh
mkdir -p data/generated
set -o pipefail
.venv/bin/python tools/regenerate.py build \
  --cache data/cache/jpl \
  --out data/generated/demo \
  2>&1 | tee data/generated/demo-build.log
```

The command prepares intermediate kernels, checks their fits, builds `cheb.bin`,
and invokes the local Wasm harness with that dataset. The **verification-feature
build from step 1 is required**. `provenance.json` is written only after these
checks pass. A `cheb.bin` left by a failed run is insufficient to continue.

If you already have a dataset generated by this tooling, you can skip acquisition
and generation. Use its complete output directory, including provenance and
manifest, in place of `data/generated/demo` throughout this guide. A binary of
unknown origin, or the checked-in historical manifest alone, does not satisfy
this shortcut. The original generation log is optional; the coverage command
below reads the UTC year bounds directly from the dataset.

For that shortcut, rerun the current verification build against the existing
artifact before continuing (from the **repository root**):

```sh
node examples/cloudflare-worker/verify.mjs \
  --dataset data/generated/demo/cheb.bin \
  --report data/generated/demo/recheck.json
```

Require exit code zero and `"passed": true`. This harness assumes the standard
generation range, including the year 2000; a custom narrow dataset needs a
different validation procedure and is outside this walkthrough.

### Check the artifact, provenance, and coverage

Run this from the **repository root**. It checks the generated file against its
own manifest, the provenance output hashes, the binary coverage header, and the
recorded real-data Wasm validation:

```sh
.venv/bin/python - <<'PY'
import hashlib
import json
import math
import struct
from pathlib import Path

root = Path('data/generated/demo')
manifest = json.loads((root / 'manifest.json').read_text())
provenance = json.loads((root / 'provenance.json').read_text())
validation = json.loads((root / 'wasm-validation.json').read_text())
blob = (root / 'cheb.bin').read_bytes()
digest = hashlib.sha256(blob).hexdigest()
assert provenance['usable'] is True
assert provenance['inputs']['complete'] is True
for name, expected in provenance['outputs'].items():
    assert hashlib.sha256((root / name).read_bytes()).hexdigest() == expected, name
assert manifest['format'] == 'HDCHEB01' and blob[:8] == b'HDCHEB01'
assert len(blob) == manifest['bytes'] and digest == manifest['sha256']
lo, hi = struct.unpack_from('<dd', blob, 16)
assert math.isfinite(lo) and math.isfinite(hi) and lo < hi
assert {'lo': lo, 'hi': hi} == manifest['domainEt']
assert validation['passed'] is True and validation['dataset_sha256'] == digest
assert validation['dataset_validation'] == {
    'charts': 5, 'searches': 2, 'runtime': 'wasm32-unknown-unknown/workerd'
}
print('EPHEMERIS_SHA256:', digest)
print('Bytes:', len(blob))
print('Coverage in ET seconds:', lo, '..', hi)
print('Artifact checks passed')
PY
```

The manifest's `domainEt` uses ephemeris-time seconds, not Unix timestamps.
After the artifact checks pass, run this from the **repository root** for both
newly generated and existing datasets:

```sh
cargo run --locked --quiet --example dataset_coverage -- data/generated/demo/cheb.bin
```

The command parses the dataset and uses the library's time conversion to print
its UTC coverage years, for example:

```text
KERNEL_COVERAGE_MIN_YEAR=1849
KERNEL_COVERAGE_MAX_YEAR=2149
```

Copy the **actual printed values** into the corresponding string values in
`wrangler.jsonc` in step 3. Do not copy these illustrative years without checking
your dataset. No generation log is needed. This command reports coverage; the
artifact checks above establish that the file matches its manifest and provenance.

The year variables are a coarse request filter. They do not promise that every
day in either boundary year is covered: the library also enforces the binary's
exact domain. Solar searches additionally require 89 days before the starting
instant, including for forward searches in the current implementation.

Keep the cache, manifest, `kernels.json`, `residual-report.json`, validation
report, and provenance together. Hash checks detect mismatched files; they do
not authenticate an unknown supplier. Fresh JPL acquisitions are not promised
to reproduce the checked-in reference dataset's bytes. These checks also do not
establish independent astronomical accuracy or parity with the historical
source dataset.

## 3. Configure the demo

Create a [Cloudflare account](https://dash.cloudflare.com/sign-up), then enable
R2 under **Storage & databases → R2 → Overview**. R2 requires completing its
subscription setup; included usage is not a promise that your demo costs
nothing. Check the current [R2 setup instructions](https://developers.cloudflare.com/r2/get-started/)
and [Workers plan information](https://developers.cloudflare.com/workers/platform/pricing/)
for your account.

Change to the **Worker directory**. Stay here through deployment and cleanup:

```sh
cd examples/cloudflare-worker
bunx --no-install wrangler --version
bunx --no-install wrangler login
bunx --no-install wrangler whoami
```

Expect Wrangler 4.112.0. `login` opens a browser; authorize your own account.
Use the account ID reported by `whoami`. This guide uses the project's installed
CLI rather than downloading a different version. See
[Wrangler authentication](https://developers.cloudflare.com/workers/wrangler/commands/general/#login).

Choose unused names in that account. This guide uses `astrology-engine-demo`
for the Worker and `astrology-engine-demo-ephemeris` for the bucket. If either
already exists, choose different names and substitute them in **both the
configuration and all commands**. Keep demo resources separate from existing apps.

Edit `wrangler.jsonc` to the following shape, replacing the two `REPLACE_...`
values and setting the coverage years from step 2:

```json
{
  "name": "astrology-engine-demo",
  "account_id": "REPLACE_WITH_YOUR_ACCOUNT_ID",
  "main": "build/worker/shim.mjs",
  "compatibility_date": "2024-11-01",
  "workers_dev": true,
  "preview_urls": false,
  "build": { "command": "bun run build" },
  "vars": {
    "EPHEMERIS_KEY": "cheb/demo/cheb.bin",
    "EPHEMERIS_SHA256": "REPLACE_WITH_GENERATED_SHA256",
    "KERNEL_COVERAGE_MIN_YEAR": "1849",
    "KERNEL_COVERAGE_MAX_YEAR": "2150"
  },
  "r2_buckets": [
    { "binding": "EPHEMERIS", "bucket_name": "astrology-engine-demo-ephemeris" }
  ]
}
```

`EPHEMERIS` must match the binding used by the Rust adapter. `EPHEMERIS_KEY`
is the path **inside** the bucket. The hash must be the lowercase, 64-character
SHA-256 printed in step 2, without a prefix. Do not retain the checked-in hash.

Setting `workers_dev` to `true` explicitly enables the public endpoint; the
checked-in example disables it. Keep preview URLs disabled. No custom domain
is required. If your account has no workers.dev subdomain yet, set one under
**Workers & Pages** following [Cloudflare's workers.dev instructions](https://developers.cloudflare.com/workers/configuration/routing/workers-dev/).
The endpoint will be `https://astrology-engine-demo.YOUR_SUBDOMAIN.workers.dev`.

The R2 bucket stays private: the Worker accesses it through its
[R2 binding](https://developers.cloudflare.com/r2/api/workers/workers-api-usage/).
Do not enable an R2 public URL. No R2 API key belongs in the Rust code or vars.

## 4. Check an ordinary build locally

The generation harness used a build with internal verification routes. Rebuild
without that feature before running or deploying the public demo:

```sh
bun run build
bunx --no-install wrangler r2 object put astrology-engine-demo-ephemeris/cheb/demo/cheb.bin \
  --file ../../data/generated/demo/cheb.bin --local --persist-to .wrangler/demo-state
bunx --no-install wrangler dev --local --ip 127.0.0.1 --port 8787 \
  --persist-to .wrangler/demo-state
```

Wait for Wrangler's ready message. This uses local storage and does not require
creating a remote bucket. The upload and server must use the same persistence
directory. `--local` disables remote bindings; see the
[R2 CLI reference](https://developers.cloudflare.com/workers/wrangler/commands/r2/).

In a **second terminal**, change to `examples/cloudflare-worker` and set:

```sh
DEMO_URL='http://127.0.0.1:8787'
```

Run every request in step 5 against this URL. When finished, stop the dev server
with Ctrl-C. A successful local run validates this build/configuration with local
R2 data; it does not prove account permissions or remote R2 access.

## 5. Exercise the HTTP API

Use these same requests locally and, later, against the deployed URL. `curl -i`
prints the HTTP status, headers, and body. These are **expected observations**,
not a captured hosted test report.

### Health and a chart

```sh
curl -i "$DEMO_URL/health"
curl -i "$DEMO_URL/charts" \
  -H 'Content-Type: application/json' \
  --data '{"date":"2000-01-01T12:00:00Z","location":{"latitude":45,"longitude":-90},"houseSystem":"placidus"}'
```

Health returns **200** with the text `ok`. It does not read R2.
The chart must return **200** with this shape:

| Field | Expected value or shape |
| --- | --- |
| `utcDate` | `2000-01-01T12:00:00Z` |
| `location` | `{"latitude":45,"longitude":-90}` |
| `planets` | 19 entries, each with `name`, `longitude`, `speed`, `declination`; this includes chart angles |
| `cusps` | 12 numbers for this Placidus request |

Require finite numbers, longitudes and cusps in `[0, 360)`, and no `error` field.
Negative speeds are valid. The `x-ephemeris-cache` header is `cold` when that
isolate loads the dataset and `warm` when it reuses it. Repeat the request:
the payload should match, but a hosted request can reach another isolate, so
do not require the second hosted response to say `warm`.

For a machine check, save the response and inspect it with Node (still in the
**Worker directory**):

```sh
mkdir -p .wrangler/demo-evidence
curl --fail-with-body --silent --show-error \
  -D .wrangler/demo-evidence/chart.headers \
  -o .wrangler/demo-evidence/chart.json "$DEMO_URL/charts" \
  -H 'Content-Type: application/json' \
  --data '{"date":"2000-01-01T12:00:00Z","location":{"latitude":45,"longitude":-90},"houseSystem":"placidus"}'
node --input-type=module - <<'JS'
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const chart = JSON.parse(readFileSync('.wrangler/demo-evidence/chart.json', 'utf8'));
const angle = value => Number.isFinite(value) && value >= 0 && value < 360;
assert.equal(chart.utcDate, '2000-01-01T12:00:00Z');
assert.deepEqual(chart.location, { latitude: 45, longitude: -90 });
assert.equal(Object.hasOwn(chart, 'error'), false);
assert.equal(chart.planets.length, 19);
assert.equal(new Set(chart.planets.map(body => body.name)).size, 19);
assert.ok(chart.planets.some(body => body.name === 'SUN'));
assert.ok(chart.planets.every(body => typeof body.name === 'string'
  && angle(body.longitude) && Number.isFinite(body.speed) && Number.isFinite(body.declination)));
assert.equal(chart.cusps.length, 12);
assert.ok(chart.cusps.every(angle));
console.log('Chart shape and finite values passed');
JS
```

Save local and hosted evidence separately before rerunning: these filenames are
overwritten. Numeric sanity checks are smoke tests, not accuracy measurements.

### Solar crossing and input failures

```sh
curl -i "$DEMO_URL/charts/find-moment" \
  -H 'Content-Type: application/json' \
  --data '{"birthMoment":"2000-01-01T12:00:00Z","planet":"SUN","targetLongitude":0,"direction":"forward"}'
curl -i "$DEMO_URL/charts" -H 'Content-Type: application/json' --data '{}'
curl -i "$DEMO_URL/charts" -H 'Content-Type: application/json' \
  --data '{"date":"2000-01-01T12:00:00Z","location":{"latitude":91,"longitude":0}}'
curl -i "$DEMO_URL/charts" -H 'Content-Type: application/json' \
  --data '{"date":"1700-01-01T00:00:00Z","location":{"latitude":45,"longitude":-90}}'
curl -i "$DEMO_URL/__structural"
curl -i "$DEMO_URL/__fixture"
curl -i "$DEMO_URL/__validate"
curl -i "$DEMO_URL/__capture" -H 'Content-Type: application/json' --data '[]'
```

| Request | Expected result |
| --- | --- |
| Solar crossing | **200**, `{"moment":"..."}`: a UTC instant after the starting instant, within two years, formatted `YYYY-MM-DDTHH:MM:00.000Z` |
| Empty chart object | **400**, `{"error":"request body must be JSON with date + location"}` |
| Latitude 91 | **400**, an `error` describing the latitude/longitude ranges |
| Year 1700 | **422**, an `error` beginning `date outside ephemeris coverage` |
| Each `/__...` route | **404** in the ordinary build; any successful verification response means the wrong build is running |

Repeat the crossing request with `"direction":"backward"`; require a **200**
moment before the starting instant and within two years. The HTTP adapter
formats results to the minute; it does not expose the library's full precision.
Only the uppercase wire name `SUN` is supported for crossing searches.

For an additional chart check, use `"houseSystem":"whole-sign"`: expect **200**
and 19 entries, with `cusps` **absent**, not an empty array. Supported house
systems are `placidus`, `equal`, and `whole-sign`; omitting the field selects
Placidus. Supplying `designLookback` does not calculate a design chart: that
field is currently ignored by this example.

## 6. Upload and deploy

Return to the **Worker directory** in your first terminal. Confirm `whoami`
and `account_id` still identify your intended account. Create the new remote
bucket without asking Wrangler to rewrite the config:

```sh
bunx --no-install wrangler r2 bucket create astrology-engine-demo-ephemeris --no-update-config
bunx --no-install wrangler r2 object put astrology-engine-demo-ephemeris/cheb/demo/cheb.bin \
  --file ../../data/generated/demo/cheb.bin --remote
```

The `--remote` upload is separate from the local upload. Bucket creation and
object operations are documented in the [R2 command reference](https://developers.cloudflare.com/workers/wrangler/commands/r2/).
Confirm the bucket and `cheb/demo/cheb.bin` object appear in your account's R2
dashboard. If you regenerate later, update the expected hash together with the
object; an old warm isolate can continue using its cached bytes.

Check packaging before uploading the Worker, then deploy:

```sh
bunx --no-install wrangler deploy --dry-run
bunx --no-install wrangler deploy
```

Both commands use `build.command` to make an ordinary release build.
The dry run checks packaging without uploading the Worker; it does not prove
that the remote bucket exists or is readable. See the
[deploy command reference](https://developers.cloudflare.com/workers/wrangler/commands/workers/#deploy).
Do not substitute a previously built verification bundle.

Copy the exact HTTPS URL printed by the successful deployment:

```sh
DEMO_URL='https://astrology-engine-demo.YOUR_SUBDOMAIN.workers.dev'
```

Replace `YOUR_SUBDOMAIN`, then repeat **all of step 5** against that URL.
Record the deployment/version ID, URL, UTC time, dataset SHA-256, tool versions,
statuses, and response bodies. Hosted success requires successful chart and
both crossing requests, the expected input rejections, and absent verification
routes. A successful deploy or `/health` response alone is insufficient.

## 7. Diagnose failures

| Symptom | Check and recovery |
| --- | --- |
| `worker-build` or `wasm32-unknown-unknown` missing | Complete step 1 and ensure Cargo's binary directory is on `PATH`. |
| Generation fails requesting JPL data | Inspect the acquisition error. Retry against the same complete cache; an incomplete or hash-mismatched cache entry is rejected. Keep failed evidence and use a new cache path if reacquiring. |
| Generation says the output directory exists | Use a fresh output path and substitute it everywhere. Do not treat a partial previous output as usable. |
| Fit or real-data validation fails | Stop before upload. Preserve the log and partial reports. A fit failure is not fixed by weakening a threshold in this walkthrough. |
| `verify.mjs` gets 404 for an internal route | Rebuild with `bun run build:verification`, rerun the harness, then rebuild normally before serving the demo. |
| Wrangler cannot write its log file | Use a writable path for that command, e.g. `WRANGLER_LOG_PATH=/tmp/astrology-demo-wrangler.log bunx --no-install wrangler deploy --dry-run`. |
| Authentication/permission or R2 subscription failure | Check `whoami`, `account_id`, account access to Workers/R2, and R2 activation. Local success cannot establish these. |
| Hosted URL is unavailable | Check the URL printed by deployment, the account's workers.dev subdomain, and `workers_dev: true` in the deployed configuration. |
| `/health` is 200 but a valid chart is 500 | Check binding `EPHEMERIS`, bucket name, object key, upload destination, lowercase SHA-256, and integer coverage vars. All are required. |
| Local chart is 500 after a remote upload | Upload again with `--local` and the same `--persist-to` path used by the dev server. Remote and local stores are separate. |
| `{"error":"internal"}` for missing/corrupt data | The adapter deliberately collapses internal errors to this body. It does not expose or log a specific hash/parser error on the ordinary route. Download/check the object as below. |
| 400 response | Read `error`; check JSON fields, coordinates, house system, UTC date, `SUN`, direction, and target longitude in `[0, 360)`. |
| 422 response | The instant may be outside configured years or exact binary coverage, lack the 89-day search lookback, or have no crossing found within the search window. Read `error` and choose an interior date. |
| Requests still work after deleting the R2 object | A warm isolate retains parsed data. Restart the local server when diagnosing initialization; deleting storage alone does not clear the cache. |
| Cloudflare reports a runtime resource limit | Record the platform error and review [Workers limits](https://developers.cloudflare.com/workers/platform/limits/) for your plan. The demo reads, hashes, and parses the entire dataset on cold initialization; passing local tests does not establish hosted resource requirements. |

To check the uploaded bytes, from the **Worker directory**:

```sh
mkdir -p .wrangler/demo-evidence
bunx --no-install wrangler r2 object get astrology-engine-demo-ephemeris/cheb/demo/cheb.bin \
  --file .wrangler/demo-evidence/downloaded.bin --remote
cmp ../../data/generated/demo/cheb.bin .wrangler/demo-evidence/downloaded.bin
```

`cmp` should exit zero without output. Re-run the step 2 artifact check if the
local file may have changed. If both files match, compare the configured hash
and key against that artifact. Both coverage variables must parse as integers
with minimum ≤ maximum. A missing binding can prevent deployment or fail when
the adapter first accesses it.

For platform errors, inspect a request while running:

```sh
bunx --no-install wrangler tail
```

Tail output can reveal runtime exceptions and resource failures; it cannot
recover the dataset error details this adapter discards. See
[Wrangler tail](https://developers.cloudflare.com/workers/wrangler/commands/workers/#tail).

To reproduce storage failures, stop the local dev server. In the **Worker
directory**, remove only the local object:

```sh
bunx --no-install wrangler r2 object delete astrology-engine-demo-ephemeris/cheb/demo/cheb.bin \
  --local --persist-to .wrangler/demo-state
```

Restart the step 4 server and send the valid chart request to
`http://127.0.0.1:8787`: expect **500**. Stop the server again and upload invalid
bytes to exercise hash rejection:

```sh
mkdir -p .wrangler/demo-evidence
echo 'invalid ephemeris' > .wrangler/demo-evidence/invalid-data.txt
bunx --no-install wrangler r2 object put astrology-engine-demo-ephemeris/cheb/demo/cheb.bin \
  --file .wrangler/demo-evidence/invalid-data.txt --local --persist-to .wrangler/demo-state
```

Restart and send the same local request: expect **500**. Stop, repeat the real
file upload from step 4, restart, and require **200**. The fixture harness also
exercises these cases in its own isolated local storage. These failure tests
do not change the hosted object.

## 8. Remove the demo

Stop any local dev/tail processes with Ctrl-C. In the **Worker directory**,
check the account and resource names, then remove only the demo you created:

```sh
bunx --no-install wrangler whoami
bunx --no-install wrangler delete --name astrology-engine-demo
bunx --no-install wrangler r2 object delete astrology-engine-demo-ephemeris/cheb/demo/cheb.bin --remote
bunx --no-install wrangler r2 bucket delete astrology-engine-demo-ephemeris
```

Confirm the named Worker is absent in Workers & Pages and the bucket is absent
in R2. Worker deletion does not remove the dataset bucket. A nonempty bucket
cannot be deleted; inspect any remaining objects before removing them. See the
[Worker delete command](https://developers.cloudflare.com/workers/wrangler/commands/workers/#delete)
and [R2 delete commands](https://developers.cloudflare.com/workers/wrangler/commands/r2/).

You may remove `.wrangler/demo-state` and `.wrangler/demo-evidence` locally when
finished. Keep the generated dataset, JPL cache, and provenance if you want to
reuse them. Removing this demo does not cancel an account's R2 subscription.

## Distribution notices

Use `bun run build` or `bun run build:verification` rather than invoking
`worker-build` directly when preparing distributable artifacts. Both wrappers
include the project license and applicable notices in `build/` and
`build/worker/`; the Wrangler build command uses the ordinary wrapper too. Keep
these files with any shared artifact directory. The notices include exact MPL
source links. See the [notice guide](../tools/notices/README.md) before updating
dependencies or distributing modified dependency source.
