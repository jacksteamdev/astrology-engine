# Tasks

## Validate engine outputs against independent JPL references

**Status:** TODO

Current checks compare intermediate Ceres/Chiron kernels with JPL samples and
coefficient fits with the builder's own reference calculations. Source-Wasm
parity checks preservation of existing behavior; Worker verification checks
runtime integration. We still need an end-to-end numerical comparison between
the public engine's planetary outputs and independent JPL reference values.

### Approach

Build a standalone native validation harness that loads a generated `cheb.bin`
through `Ephemeris::parse`, calls the public chart/evaluation API, and compares
the returned planetary longitudes and declinations with independently acquired
JPL reference samples. This exercises preparation, serialization, loading, and
runtime evaluation together.

Expected values must not come from engine output, original-application snapshots,
or the generator's own coordinate-conversion helpers. Match time scales,
observer/center, coordinate frames, units, and correction conventions explicitly.
Any required reference conversion must be independently implemented and checked.

### Comparison contract

The existing Horizons acquisition requests geometric heliocentric ICRF vectors
for Ceres and Chiron. Those samples cannot be compared directly with the engine's
geocentric longitudes of date and declinations. Define a reference request or
independent conversion that yields the same observable as each tested output.

Record time-scale handling, reference plane/equinox, observer/center, and the
specific light-time, aberration, and other corrections included on each side.
Do not assume that similarly named "apparent" coordinates use identical
conventions, or silently treat TDB, TT, and UTC as interchangeable. If a
convention cannot be matched, report the comparison as unsupported and explain
the gap.

Use the shortest wrapped angular difference for longitude and signed-coordinate
differences for declination. The engine's speed is a centered finite difference
using longitude samples half a day before and after the requested instant;
comparing it with an instantaneous reference velocity requires a separate error
budget or a matching reference calculation.

Independent validation may use the same underlying JPL ephemeris solution as
generation. The required independence is in acquiring the expected values and
computing the reference observables, so a shared implementation error cannot
simply reproduce itself. Include separate validation samples that were not used
to choose the fit configuration or acceptance tolerances.

### Acceptance criteria

- [ ] Record JPL source/solution, requests, retrieval time, raw responses, and
  hashes; retain references for repeatable offline validation after acquisition.
- [ ] List supported bodies and quantities. Include speeds only after reconciling
  reference and engine definitions. Treat derived Earth/node entries, chart
  angles, and house cusps separately unless suitable independent references exist.
- [ ] Select reference epochs independently of the fitting grid, including
  representative dates, supported coverage edges, longitude wraparound, and
  motion changes.
- [ ] Define and justify error metrics and tolerances before evaluating results.
  Specify units and absolute versus relative errors per body and quantity;
  account for reference conventions, fitting error, and numerical evaluation.
  Report discrepancies rather than adjusting expectations to match the engine.
- [ ] Test the public runtime using the final generated binary, rather than
  checking only in-memory coefficients or intermediate kernels.
- [ ] Produce a machine-readable report identifying source revision, toolchain,
  dataset/reference hashes, case counts, per-body/per-quantity maximum errors,
  worst cases, omissions, and pass/fail status.
- [ ] Exit unsuccessfully for missing or non-finite results and exceeded
  tolerances. Demonstrate failure with deliberately perturbed, missing, and
  non-finite results. Also reject duplicate case identifiers and reference
  metadata/hash mismatches.
- [ ] Document reference acquisition and validation commands. The numerical
  harness must run natively without Bun, Node, Cloudflare, or Worker infrastructure.
- [ ] Keep validation separate from generation: it reports results without
  modifying coefficients. CI and Worker checks may supplement it.
- [ ] Record actual findings and unresolved discrepancies; existing fit, parity,
  and Worker checks do not substitute for this comparison.

### Prior art: Astronomy Engine

The project is [cosinekitty/astronomy](https://github.com/cosinekitty/astronomy),
not `cosinekitty/astronomy-engine`. Source inspection on 2026-09-29 confirmed
these relevant patterns:

- **Public runtime outputs compared with JPL fixtures.** Its JavaScript harness
  imports the distributed library and compares `BaryState`, `HelioState`, and
  `GeoMoonState` outputs with locally stored Horizons vector files. Tests use
  body-specific position and velocity tolerances.
  [Representative state-vector tests](https://github.com/cosinekitty/astronomy/blob/master/generate/test.js#L2413-L2525).
- **Explicit comparison machinery.** The Horizons parser and state comparator
  show how epochs and reference vectors enter the test, and how relative versus
  absolute errors are applied. These vector-error tolerances are not suitable
  defaults for our longitude/declination comparisons.
  [Parser and comparator](https://github.com/cosinekitty/astronomy/blob/master/generate/test.js#L2203-L2253).
- **Reference accuracy and implementation parity are different checks.** The
  project describes validation against JPL Horizons and NOVAS C 3.1 using DE405,
  alongside agreement between its language implementations.
  [Project explanation](https://github.com/cosinekitty/astronomy#why-i-created-this-thing).
- **Contributor tooling is outside the distributed runtime.** Tests and
  generation tools live under `generate/`, while library implementations live
  under `source/`. Its orchestration still combines model generation and testing;
  it is not an example of fully independent generation and verification commands.
  [Contributor guide](https://github.com/cosinekitty/astronomy/blob/master/generate/README.md)
  and [orchestration](https://github.com/cosinekitty/astronomy/blob/master/generate/run).

The relevant precedent is testing actual runtime outputs against external
numerical references. Astronomy Engine uses a different model, including
truncated VSOP87 for planetary calculations; this research does not establish a
matching Chebyshev runtime architecture. Its outputs must not replace JPL
reference values in this task.

The upstream tests were inspected, not executed. The inspection did not establish
all fixture frame/correction settings, independently held-out sample selection,
raw-request provenance, negative controls, or a report schema equivalent to the
one required here. The links above follow `master`; pin the exact upstream
revision and inspect fixture headers before relying on implementation details.

### Starting points

- [Generation orchestration](tools/regenerate.py)
- [Intermediate JPL kernel validation](tools/jpl/validate.py)
- [Existing Ceres fit/readback check](tools/verification/test_generation.py)
- [Coefficient fit checks](tools/dataset-builder/src/cheb/gen.rs)
- [Runtime evaluation](src/cheb/eval.rs) and [chart calculation](src/chart.rs)
- [Current Worker dataset checks](examples/cloudflare-worker/src/verification.rs)
