# Shape

Shape is a cross-platform, AI-native creative environment for preserving what matters, changing
what is intended, exploring alternatives, and accepting durable artifact revisions.

The repository is in its foundation phase. Its first two deliberately small vertical slices prove
the shared creative lifecycle with text and images: create or import an artifact, execute a
deterministic transformation, review a transient candidate, atomically accept an immutable
revision, close the process, and inspect the accepted head again.

## Repository map

| Area | Stable entry | Responsibility |
| --- | --- | --- |
| Product and staged design | [`SHAPE_PLAN.md`](SHAPE_PLAN.md) | Product philosophy, architecture target, external executors, roadmap, and completion gates |
| Engineering orientation | [`DEV_SKELETON.md`](DEV_SKELETON.md) | Stable implementation boundaries and navigation |
| Creative contracts | [`shape-domain`](crates/shape-domain/src/lib.rs) | IDs, artifacts, immutable revisions, typed raster/color contracts, transformations, constraints, and validation |
| Execution contracts | [`shape-execution`](crates/shape-execution/src/lib.rs) | Public Infer contract probe, executor lifecycle, bounded raster import/crop, attempts, outputs, and provenance receipts |
| Project persistence | [`shape-store`](crates/shape-store/src/lib.rs) | `.shape` bundle, SQLite ownership, durable BLAKE3 object store, and atomic accepted-head commits |
| Use cases | [`shape-core`](crates/shape-core/src/lib.rs) | Project creation, text and raster transformations, in-place acceptance, cross-artifact branching, and inspection |
| Desktop bridge | [`shape-desktop-bridge`](crates/shape-desktop-bridge/src/lib.rs) | Bounded CXX session for validated project snapshots, graph edges, on-demand raster previews, and a cross-media Candidate Shelf |
| Foundation CLI | [`shape-cli`](apps/shape-cli/src/main.rs) | Real public consumer used for end-to-end smoke and inspection |
| Desktop shell | [`apps/desktop`](apps/desktop/README.md) | Cross-platform Qt/QML assembly that opens and displays real `.shape` projects |
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
ctest --preset desktop-dev
```

CMake output is written to the sibling `.shape-local-build` directory, not the source worktree.

## Current boundary

- Implemented: pure Creative Document contracts, durable project store, executor lifecycle,
  deterministic text acceptance and new-artifact branching, plus an `image.raster` slice that
  safely imports bounded 8-bit PNG/JPEG sources into a canonical RGBA8 PNG contract. Raster crop
  follows the same Draft/Preview → Candidate → Compare → Accept → Reopen lifecycle without putting
  previews in durable history. Selected accepted/candidate image bytes are verified and loaded on
  demand into a bounded native preview cache; ordinary project snapshots carry only raster
  metadata. The cross-platform desktop keeps text and image candidates on one typed shelf while
  each medium owns its central workspace. The desktop resolves Infer Runtime through strict owner-only
  Infra Discovery, imports a Shape-specific managed credential into an owner-only secret store,
  and can execute one local-only `assistant.general` text generation as a transient candidate.
  Runtime response identity is retained as physical provenance; accepted history changes only
  after the ordinary explicit accept operation. Offline or unconfigured AI never affects direct
  editing.
- Deferred: Infer-side App provisioning, Job/explain provenance inspection, pinned or durable
  explorations, composite/layer image structure, additional deterministic image operations,
  Shadow/Echo suite adapters, external executors, project imports/exports, and AI image planning.
- License: Shape source is licensed under the [`MIT License`](LICENSE). Shadow's GPL components
  remain behind independent-process or documented protocol boundaries so Shape's own distribution
  does not silently inherit a different license obligation.
