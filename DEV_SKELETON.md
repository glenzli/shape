# Shape Development Skeleton

## Purpose

- Orient implementation work before it enters Shape's source tree.
- Preserve product, persistence, executor, and cross-platform boundaries shared by all subsystems.

## Source of truth

- [`README.md`](README.md) routes work to the implemented subsystem.
- [`SHAPE_PLAN.md`](SHAPE_PLAN.md) defines product philosophy, target architecture, and milestone
  gates. It is not evidence that a planned capability exists.
- The nearest crate/application README, module declaration, source, and adjacent contract tests
  define current behavior.
- [`AGENTS.md`](AGENTS.md) defines engineering and test topology.

## Stable constraints

- Shape is intent-anchored: accepted creative intent may transform source material, but imported
  Shadow/Echo originals remain read-only references.
- `Artifact` is a stable creative identity; `ArtifactRevision` is an immutable accepted state;
  media-internal components do not automatically become Creative Graph nodes.
- `Transformation` owns creative meaning. Execution plans and receipts own physical implementation
  facts. Neither may impersonate the other.
- A successful executor result is only a candidate. User acceptance is the sole authority that
  advances an artifact head.
- The desktop shell, media engines, Infer Runtime, and third-party tools consume narrow contracts;
  none owns the Creative Document Model.
- Existing accepted bytes remain viewable when an executor, model, or external application is
  unavailable.

## Entry hints

- Creative identities and invariants: `crates/shape-domain/src/lib.rs`.
- Executor lifecycle and provenance: `crates/shape-execution/src/lib.rs`.
- Project bundle, SQLite, and content-addressed objects: `crates/shape-store/src/lib.rs`.
- Product use-case orchestration: `crates/shape-core/src/lib.rs`.
- Bounded Rust/CXX desktop projection: `crates/shape-desktop-bridge/src/lib.rs`.
- Runnable foundation slice: `apps/shape-cli/src/main.rs`.
- Qt/QML desktop assembly: `apps/desktop/README.md`.

## Growth review baseline

The first foundation intentionally keeps four Rust semantic owners:

- `shape-domain`: pure creative contracts and validation;
- `shape-execution`: independently changing job/executor lifecycle;
- `shape-store`: one atomic persistence owner spanning SQLite metadata and durable CAS publication;
- `shape-core`: application use cases and built-in transformations.

The first real desktop edit path now owns one additional `shape-desktop-bridge`. Its
`DesktopSession` keeps one `ShapeProject` and coordinates durable mutations, while
`session::candidate_shelf::CandidateShelf` owns the transient, identity-addressed text candidate
collection. Candidates project newest-first through CXX without UI policy. Successful in-place
acceptance clears siblings prepared against the old head; branching and discard consume only the
chosen candidate. Acceptance still crosses the existing `shape-core` use case and `shape-store`
compare-and-swap commit boundary. `shape-execution::infer_runtime` now owns the first real Infer
Runtime consumer. Its `discovery` owner validates the owner-only
`infra.discovery.registration@20260810.1` manifest, exact Consumer offer, lease, generation, and
canonical numeric-loopback endpoint. The HTTP owner then performs a bounded, proxy-free,
redirect-free public-contract probe. Explicit diagnostics override remains first; the fixed 8787
origin is only a temporary final migration fallback. Its separate `credential` owner atomically
loads and rotates only a managed 256-bit token from Shape's owner-only secret store. The `responses`
owner implements the first real `text.generate` executor with a fixed `assistant.general`,
local-first/local-only/offline/no-fallback/zero-cost request. Runtime response identity enters the
payload-free execution receipt for later Job/explain lookup, but Runtime success remains only a
Shape Candidate. `shape-core::propose_generated_text` owns the generative transformation and
accepted-text context; `shape-desktop-bridge::infer_text` prepares a candidate outside the live
session and `DesktopSession` revalidates its expected head before adopting it. Detailed provenance
inspection, additional Intents, media bridges, capability registry, preview renderer, and project
dependency resolver remain deferred until real product paths consume them. Do not create empty
crates for roadmap boxes.

The desktop visual foundation adds two deliberately separate owners rather than growing the
project projection: `UiPreferences` owns persistent appearance/language lifecycle, while
`MainTitleBar.qml` owns integrated window chrome and native safe areas. Individual workspace QML
components keep ownership of their own visual regions. This split should be revisited when a
second settings domain or a second top-level workspace creates a concrete growth trigger.
`TextCompareWorkspace.qml` separately owns accepted-versus-candidate presentation; the C++
`DesktopBackend` remains a presentation facade and never becomes the draft or persistence owner.
The independent `InferRuntimeController` owns the asynchronous desktop probe lifecycle and exposes
only availability, compatibility, contract version, endpoint source, instance/generation identity,
and stable error identity to QML. Endpoint selection remains authoritative in Rust. The controller
never enters `DesktopSession`, blocks the UI thread, edits Infer configuration, or makes accepted
content dependent on runtime availability.
The independent `InferTextController` owns credential readiness/import plus one background
generation lifecycle. Its move-only Rust result stays private until `DesktopBackend` hands it to the
session on the UI thread. It rejects concurrent generation, waits during destruction, carries a
complete request generation and artifact identity, and exposes only stable localized failure codes.
The graph-aware desktop information architecture adds `ProjectNavigator.qml` for project-level
creative-object selection and `ContextInspector.qml` for Explore, Details, and Lineage modes.
`ArtifactWorkspace.qml` remains the media-workspace owner. The bridge projects only lineage already
proven by the current accepted revision and its persisted Transformation. Once the first real
cross-artifact branch path became available, `ProjectGraphWorkspace.qml` became the presentation
owner for accepted current-head topology and transient candidate ghosts; `WorkspaceSurface.qml`
owns artifact/graph workspace navigation. `VariantsPanel.qml` owns artifact-scoped Candidate Shelf
selection and review controls, while candidate identity and mutation remain in Rust. Rust remains
the semantic graph projection owner, and QML layout never becomes durable graph authority.
