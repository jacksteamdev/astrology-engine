# Dataset preparation and verification

The root guidance applies. Use the [tools guide](README.md) for environment setup,
commands, output files and failure recovery; keep this file focused on contracts
that span the Python orchestrator, native Rust builder and Wasm consumer.

## Generation boundaries

- There are two distinct fits: [jpl/](jpl/) fits Horizons position vectors into
  intermediate asteroid SPKs; [dataset-builder/](dataset-builder/) fits derived
  chart coordinates into the runtime coefficient format. Changing the first
  stage's residual tolerance is not equivalent to changing the final angular
  tolerance. Check each stage and the runtime consumer separately.
- Kernel states, the [provider adapters](dataset-builder/src/astro/provider.rs)
  and [apparent-coordinate calculations](dataset-builder/src/astro/apparent.rs)
  must agree on origin, frame and units. The adapters convert km and km/s to
  AU and AU/day; asteroid Sun-centered vectors are combined with the Sun's
  barycentric state before Earth-relative calculations. Do not insert another
  conversion or subtract Earth at the kernel boundary without tracing both
  planet and asteroid paths.
- Shared frame/time code and binary format come from the runtime's `generation`
  exports through the builder modules. Keep ANISE and kernel I/O here; changes
  to shared math or format also require reading [runtime guidance](../src/AGENTS.md).
  Preserve fitted support beyond the public domain for runtime speed samples.

## Artifact identity and evidence

For artifact identity, completion and repeatability, use
[Build a dataset](README.md#build-a-dataset) and
[Check repeatability](README.md#check-repeatability). Hand off generated metadata
with the bytes when moving to [Worker integration](../examples/cloudflare-worker/AGENTS.md).

The [parity runner](verification/parity.py) uses the frozen fixture manifest's
dataset identity, whereas generation validates the newly built artifact through
the local Worker harness. These answer different questions. Preserve capture
identifiers, floating-point bits and large timestamp integers across
[comparison](verification/compare.py) and [capture transport](../examples/cloudflare-worker/verify.mjs).
Do not regenerate expected captures from the changed implementation merely to
make parity pass. Use the focused checks and optional full-data procedures in
the guide, and report which artifact and validation layer actually ran.
