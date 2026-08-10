# Shape

Shape is a cross-platform, AI-native creative environment for preserving what matters, changing
what is intended, exploring alternatives, and accepting durable artifact revisions.

The repository is in its foundation phase. Text, image, and preset-only speech slices prove the
interactive creative lifecycle, including creation of a typed audio Artifact with exact Runtime
provenance and selected-only local audition before acceptance.

## Repository map

| Area | Stable entry | Responsibility |
| --- | --- | --- |
| Product and staged design | [`SHAPE_PLAN.md`](SHAPE_PLAN.md) | Product philosophy, architecture target, external executors, roadmap, and completion gates |
| Engineering orientation | [`DEV_SKELETON.md`](DEV_SKELETON.md) | Stable implementation boundaries and navigation |
| Creative contracts | [`shape-domain`](crates/shape-domain/src/lib.rs) | Scene/Artifact identities, immutable revisions, typed Operator Graph, raster/audio contracts, transformations, constraints, and validation |
| Execution contracts | [`shape-execution`](crates/shape-execution/src/lib.rs) | Infer discovery/contract clients, executor lifecycle, bounded raster and preset speech adapters, outputs, and provenance receipts |
| Project persistence | [`shape-store`](crates/shape-store/src/lib.rs) | `.shape` bundle, SQLite ownership, durable BLAKE3 object store, and atomic Artifact/Scene accepted-head commits |
| Use cases | [`shape-core`](crates/shape-core/src/lib.rs) | Project and Scene creation, graph draft/accept, text/raster/speech Candidates, cross-artifact branching, and inspection |
| Desktop bridge | [`shape-desktop-bridge`](crates/shape-desktop-bridge/src/lib.rs) | Bounded CXX session for validated project snapshots, graph edges, on-demand raster/audio previews, and a cross-media Candidate Shelf |
| Foundation CLI | [`shape-cli`](apps/shape-cli/src/main.rs) | Real public consumer used for end-to-end smoke and inspection |
| Desktop shell | [`apps/desktop`](apps/desktop/README.md) | Cross-platform Qt/QML assembly that creates, opens, and edits real `.shape` projects |
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

./scripts/build_and_promote_debug.sh
./scripts/run_debug.sh [/path/to/project.shape]
```

CMake output is written to the sibling `.shape-local-build` directory, not the source worktree.

`scripts/run_debug.sh` is the stable local launcher shared with the Shadow/Echo development
convention. `scripts/build_and_promote_debug.sh` validates Rust and the packaged desktop smoke paths,
copies the app into an immutable revision-stamped release, and atomically advances
`.shape-local-build/current-debug`. The launcher starts that canonical app in the background and
logs to `.shape-local-build/logs/shape-debug.log`; use `--foreground` for attached output or
`--check` to inspect resolved paths without launching. Pass a `.shape` bundle, or set
`SHAPE_DEBUG_PROJECT_PATH`, to open a project directly.

## Current boundary

- Implemented: pure Creative Document contracts, durable project store, executor lifecycle,
  separately revisioned Scenes with explicit named outputs and stale-draft-safe graph acceptance,
  deterministic text acceptance and new-artifact branching, plus an `image.raster` slice that
  safely imports bounded 8-bit PNG/JPEG sources into a canonical RGBA8 PNG contract. Raster crop
  follows the same Draft/Preview → Candidate → Compare → Accept → Reopen lifecycle without putting
  previews in durable history. Selected accepted/candidate image bytes are verified and loaded on
  demand into a bounded native preview cache; ordinary project snapshots carry only raster
  metadata. The cross-platform desktop can create or open a project, atomically create its first
  accepted Text Scene, and begin compatible project-backed Operator drafts from the graph. An
  unexecuted draft restores on reopen without entering immutable accepted history; execution turns
  it into a transient Candidate, and only explicit acceptance publishes durable history. The
  desktop keeps text and image candidates on one typed shelf while each medium owns
  its central workspace. It resolves Infer Runtime through strict owner-only
  Infra Discovery, imports a Shape-specific managed credential into an owner-only secret store,
  and can execute one local-only `assistant.general` text generation as a transient candidate.
  The `audio.speech_synthesize` desktop workspace consumes immutable accepted text and creates a
  transient audio Candidate through a single versioned preset. The selected Candidate can be
  auditioned from an in-memory WAV device and becomes a new `audio.clip` only after explicit
  acceptance. Shape re-parses exact PCM S16 LE WAV bytes,
  requires preset voice/disclosure and local-only no-fallback Runtime provenance, and atomically
  rejects a stale text source. Schema `20260811.3` additively upgrades initial, Scene-era, and
  audio-era projects with mutable Working Graph storage without changing existing accepted heads.
  Offline or unconfigured AI never affects direct editing.
- Deferred: desktop editing of the new persistent Scene graph, GraphComponent instances, Infer-side
  App provisioning, Job/explain provenance inspection, pinned or durable explorations,
  composite/layer image structure, waveform editing, recording, Voice Reference execution,
  `audio.generate`/`audio.transform`, Shadow/Echo suite adapters, external executors,
  project imports/exports, and AI image planning. The required Infer speech alias/ACL is locally
  proven but remains an externally unpublished dependency until Infer commits it.
- License: Shape source is licensed under the [`MIT License`](LICENSE). Shadow's GPL components
  remain behind independent-process or documented protocol boundaries so Shape's own distribution
  does not silently inherit a different license obligation.
