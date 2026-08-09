# Shape

Shape is a cross-platform, AI-native creative environment for preserving what matters, changing
what is intended, exploring alternatives, and accepting durable artifact revisions.

The repository is in its foundation phase. The first implemented vertical slice is deliberately
small: create a project bundle, create one artifact, execute a deterministic text transformation,
atomically accept immutable revisions, close the process, and inspect the accepted head again.

## Repository map

| Area | Stable entry | Responsibility |
| --- | --- | --- |
| Product and staged design | [`SHAPE_PLAN.md`](SHAPE_PLAN.md) | Product philosophy, architecture target, external executors, roadmap, and completion gates |
| Engineering orientation | [`DEV_SKELETON.md`](DEV_SKELETON.md) | Stable implementation boundaries and navigation |
| Creative contracts | [`shape-domain`](crates/shape-domain/src/lib.rs) | IDs, artifacts, immutable revisions, transformations, constraints, and validation |
| Execution contracts | [`shape-execution`](crates/shape-execution/src/lib.rs) | Executor probes, job state, attempts, outputs, and provenance receipts |
| Project persistence | [`shape-store`](crates/shape-store/src/lib.rs) | `.shape` bundle, SQLite ownership, durable BLAKE3 object store, and atomic accepted-head commits |
| Use cases | [`shape-core`](crates/shape-core/src/lib.rs) | Project creation, artifact creation, built-in text transformation, acceptance, and inspection |
| Foundation CLI | [`shape-cli`](apps/shape-cli/src/main.rs) | Real public consumer used for end-to-end smoke and inspection |
| Desktop shell | [`apps/desktop`](apps/desktop/README.md) | Cross-platform Qt/QML application assembly; backend bridge intentionally deferred |
| Repository checks | [`xtask`](xtask/src/main.rs) | Formatting, lint, tests, CMake configuration, and prerequisite checks |

The nearest code-owned module documentation is the navigation index. Planned modules in
`SHAPE_PLAN.md` are not implemented facts.

## Quick start

```sh
cargo xtask check

cargo run --package shape-cli -- demo /tmp/shape-foundation.shape
cargo run --package shape-cli -- inspect /tmp/shape-foundation.shape

cmake --preset native-dev
cmake --build --preset native-dev

cmake --preset desktop-dev
cmake --build --preset desktop-dev
```

CMake output is written to the sibling `.shape-local-build` directory, not the source worktree.

## Current boundary

- Implemented: pure Creative Document contracts, durable project store, executor lifecycle,
  deterministic text acceptance, CLI, and a static cross-platform desktop shell.
- Deferred: Rust/Qt bridge, image buffers, preview renderer, Infer Runtime client, Shadow/Echo suite
  adapters, external executors, project imports/exports, and AI planning.
- License: Shape source is licensed under the [`MIT License`](LICENSE). Shadow's GPL components
  remain behind independent-process or documented protocol boundaries so Shape's own distribution
  does not silently inherit a different license obligation.
