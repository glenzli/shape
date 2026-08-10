# Shape development conventions

Read `DEV_SKELETON.md` before changing architecture, persistent contracts, or cross-platform
boundaries. `SHAPE_PLAN.md` owns the product and staged design; source and adjacent tests own
implemented behavior.

## Source ownership

- Apply the `maintain-source-cohesion` skill before adding a new responsibility to an existing
  owner. Prefer a few semantic owners over empty or forwarding crates.
- Keep `lib.rs`, `main.rs`, root CMake files, and QML composition roots readable as navigation and
  assembly boundaries.
- Update the nearest README or module documentation whenever a responsibility moves.
- Do not create an executor, bridge, media kernel, or SDK until a real consumer exercises it.

## Stable dependency direction

```text
shape-core -> shape-store -> shape-execution -> shape-domain
shape-core -> shape-execution
shape-core -> shape-domain
shape-desktop-bridge -> shape-core
desktop application -> shape-desktop-bridge
```

`shape-domain` is platform- and runtime-independent. Qt, C++, provider, filesystem, SQLite, and
network types may not enter it. `shape-core` orchestrates use cases but does not own persistence or
executor mechanics. `shape-desktop-bridge` is a terminal adapter: application code may depend on it,
but no domain, execution, store, or core owner may depend back on the bridge.

## Tests

- Rust production owners register `#[cfg(test)] mod tests;`; executable tests live in adjacent
  `<owner>/tests.rs`, never inline in production files.
- `src/tests/` is reserved for crate-facade contracts spanning sibling owners. Top-level `tests/`
  is reserved for black-box tests of the public crate API.
- Native tests live under the owning subsystem's `tests/` directory and must be built before being
  run; stale CTest binaries are not evidence.

## Cross-platform and build output

- macOS on Apple Silicon is the first implementation target. Windows must not require an
  architectural rewrite.
- Platform-specific UI or media behavior lives behind a platform shim or capability adapter.
- Cargo/CMake build products, model files, project bundles, previews, and media fixtures stay out
  of the worktree. CMake presets write to the sibling `.shape-local-build` directory.
- The canonical debug application is the shared `build:shape-canonical-debug` resource and uses
  the coordination pseudo-path `@external/shape-canonical-debug`. Only one release steward may
  promote it at a time through `scripts/build_and_promote_debug.sh`; `scripts/run_debug.sh` only
  consumes the atomically advanced `current-debug` application.
- Rust uses `rustfmt.toml`; C/C++ uses `.clang-format`.

## Localization

English `tr()`/`qsTr()` source text is the canonical desktop message identity. Every user-visible
message must have one finished Simplified Chinese translation before it ships. Technical schema and
capability identifiers remain language-neutral.

## Persistence and creative history

- Accepted artifact revisions are immutable.
- Mutable preview state never crosses the durable commit boundary.
- Commits use expected-head compare-and-swap so concurrent or stale drafts cannot overwrite an
  accepted head.
- Content identity is BLAKE3 over exact bytes. Stored objects are verified before consumption.
- Execution success creates a candidate; only an explicit accept operation advances an artifact's
  accepted revision.
