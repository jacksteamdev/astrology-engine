# Preparing distribution notices

The root third-party notice document distinguishes copied code, dependency
software, developer tools and data. The checked-in bundles are ready for offline
artifact builds; normal builds do not install cargo-about or fetch licenses.

## Refresh and review

Use the repository Rust toolchain and Bun version. Install the pinned generator:

```sh
cargo install cargo-about --version 0.9.2 --features cli --locked
cargo fetch --locked
cargo fetch --locked --manifest-path examples/cloudflare-worker/Cargo.toml
cargo fetch --locked --manifest-path examples/configurable-chart/wasm/Cargo.toml
bun tools/notices/notices.ts update
bun tools/notices/notices.ts verify
bun tools/notices/notices.ts check-sources
bun test tools/notices/notices.test.ts
```

`CARGO_ABOUT=/absolute/path/to/cargo-about` selects an isolated installation.
`update` collects the locked Rust graphs using offline cargo-about, renders the
four bundles, and records input/output hashes and exact package/source identities
in `notices/inventory.json`. `verify` repeats collection without writing and
compares every output. `check` validates the checked-in snapshot without Cargo,
network access, or installed dependencies. Build scripts run `check` before
copying notices. Changes to manifests, lockfiles, notice tools or packaging
configuration require regeneration and review.

Review every changed component and its complete packaged license and NOTICE
files. MIT is preferred when offered as an alternative. Other permissive terms
are retained, including Unicode and BSD requirements. MPL is explicitly allowed
for hifitime and ANISE. Unknown expressions and unreviewed synthesized license
texts fail rather than being treated as MIT. Build-only Rust dependencies are
included conservatively; dev-only Rust dependencies are excluded.

Exact-version combined/truncated license documents are also checked against
`reviewed-files.json`; their complete texts are retained even when cargo-about
uses a synthesized SPDX license text. Some crates require supplemental handling. libm's combined document and source
comments contain additional math-library grants; sofars' LICENSE includes SOFA
terms that its Cargo metadata does not express. Preserve them verbatim. ANISE's
published crate omits its root LICENSE and AUTHORS files; their exact Git revision
and SHA-256 hashes are recorded in `supplements.json`. Generated JavaScript
attribution includes wasm-bindgen-cli-support, worker-build and Vite. Supplemental
files are copied license/notice texts only, not vendored dependency source.
When any corresponding generator version changes, review its emitted code and
refresh that supplement's version, exact source URL, file contents and hashes.

The pinned MPL archive links use published versions rather than moving branch
URLs. `check-sources` downloads those archives to check availability. This is a
release check, not a build-time network dependency. Continue keeping the source
accessible for recipients: supply an accessible copy if upstream disappears.
Distributors who modify covered source must provide their modified source under
MPL; links to the unchanged upstream package are not a substitute.

## Artifact commands

```sh
# Native builder executable and notices: target/distribution/dataset-builder/
bun tools/notices/notices.ts package-builder

# Additional runtime distribution assembled by a caller:
bun tools/notices/notices.ts copy runtime /absolute/path/to/distribution
```

Both Worker build modes copy notices to `build/` and `build/worker/`. Wrangler's
build command calls the same ordinary-build wrapper. Browser Wasm preparation
copies notices into `engine/` and `public/`; the static build refreshes `public/`
and Vite carries it into `build/`. The browser page links to the self-contained
HTML notice page, including clickable MPL source downloads. Preserve the LICENSE
and all notice files when redistributing an artifact directory. A Worker runs
server-side; its HTTP API does not need a new license endpoint.

The Cargo source package includes the project license, root notices, all four
text bundles/inventory and the accompanying Rust standard-library document. Rust package metadata alone does not
relicense dependencies or describe complete binary-distribution obligations.
These bundles do not cover packaging entire npm/Python development environments,
complete compiler/toolchain or SDK installations. The Rust 1.96.0 standard
library copyright document is included as `RUST_LIBRARY_NOTICES.html`, copied
verbatim from that toolchain distribution and checked against its recorded hash. A distributor adding
other code or custom target libraries must review those additions separately.
