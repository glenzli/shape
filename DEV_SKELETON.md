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
- `Scene` is the user-authored creative orchestration boundary. Its typed Operator Graph connects
  pure Sources through creative Operators to one or more named Outputs. `Artifact` remains the
  stable identity of a creative value and `ArtifactRevision` remains an immutable accepted state;
  media-internal components do not automatically become Scene Operator nodes.
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
- Typed Source/Operator/Output contracts: `crates/shape-domain/src/operator_graph.rs`.
- Portable audio and voice-authorization contracts: `crates/shape-domain/src/audio.rs`.
- Executor lifecycle and provenance: `crates/shape-execution/src/lib.rs`.
- Project bundle, SQLite, and content-addressed objects: `crates/shape-store/src/lib.rs`.
- Product use-case orchestration: `crates/shape-core/src/lib.rs`.
- Bounded Rust/CXX desktop projection: `crates/shape-desktop-bridge/src/lib.rs`.
- Runnable foundation slice: `apps/shape-cli/src/main.rs`.
- Qt/QML desktop assembly: `apps/desktop/README.md`.
- Canonical local debug build and launch lifecycle: `scripts/build_and_promote_debug.sh` and
  `scripts/run_debug.sh`.

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
inspection, additional Intents, media bridges, capability registry, and project dependency resolver
remain deferred until real product paths consume them. Do not create empty crates for roadmap
boxes.

The first raster path extends those existing owners instead of introducing a media-kernel crate.
`shape-domain::image_raster` owns the platform-independent RGBA8, alpha, orientation, color, and
ICC contracts; `shape-domain::image_crop` separately owns the typed `image.crop` rectangle contract.
`shape-execution::raster` owns bounded PNG/JPEG decode, EXIF normalization and canonical PNG
materialization, while its `crop` owner executes exact deterministic pixels. `shape-core::project::image` owns
the atomic import-origin commit and image Candidate/Accept use cases. The desktop shelf is now a
typed text-or-image collection with the same identity, expected-head, discard, and sibling
invalidation rules. Image-candidate clones share one immutable byte allocation, so exact-ID
acceptance does not duplicate the full encoded payload. Ordinary bridge snapshots project only
image dimensions and immutable content identity; selected accepted or candidate PNG bytes cross
the Rust/CXX boundary only on demand and are decoded into a display-scaled, byte-bounded native
cache. `RasterCropOperatorWorkspace.qml` owns direct crop gestures, while `ImageCompareWorkspace.qml`
owns accepted-versus-candidate presentation. Image branching, composites, masks, color adjustment,
external editors, and model-backed image operations remain
deferred until a concrete consumer freezes each contract.

The first audio foundation continues those same dependency directions without introducing a media
kernel. `shape-domain::audio` owns `audio.generate`,
`audio.speech_synthesize`, and `audio.transform` port contracts, exact accepted WAV interpretation,
versioned preset aliases, and the local-only consent/disclosure boundary for Voice References.
`shape-execution::audio` validates exact PCM S16 LE WAV bytes; `infer_runtime::speech` is the first
real preset-only `speech.synthesize` consumer and returns bounded payload-free Job/routing/Attempt
facts. `shape-core::project::audio` keeps synthesized bytes transient and creates a new AudioClip
only through `shape-store` acceptance. Schema `20260811.2` re-parses bytes, persists provenance in
the immutable receipt, and transactionally verifies that the accepted text input is still current.
`shape-desktop-bridge::infer_speech` prepares the move-only result outside the live session;
`infer_runtime_access` owns shared credential access; the session rechecks the text head and keeps
target Audio Artifact identity separate from source review context. Ordinary snapshots project only
duration/sample/channel/origin metadata. `AudioPreviewController` fetches exact WAV bytes only for
the selected Candidate or accepted clip, and owns the in-memory `QBuffer`/Qt Multimedia playback
lifecycle. `AudioSpeechOperatorWorkspace.qml` owns preset, pace, disclosure, generation, and
audition presentation. Recording, waveform editing, authorized-voice execution, general sound
generation, audio transforms, and Echo asset resolution remain deferred. The required Infer alias
and Shape speech ACL are locally validated but not yet published by Infer.

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
`InferSpeechController` owns the analogous but separate speech lifecycle because its admission,
parameters, result type, and failure policy evolve independently from Text.
The graph-aware desktop information architecture adds `ProjectNavigator.qml` for Scene selection
and `ContextInspector.qml` for Explore, Details, and Lineage modes. `OperatorWorkspaceHost.qml`
owns focused-workspace routing and lifecycle; Text and Raster Crop remain separate semantic owners.
`shape-domain::operator_graph` now owns the platform-independent typed
DAG contract: explicit Source, Operator, and Output roles; typed ports; multiple outputs; unique
input binding; and cycle rejection. `shape-desktop-bridge::operator_graph` projects persisted
accepted Revision/Transformation history into that contract. It stops cross-Artifact history at a
pure Source boundary, so executor steps and media-internal structure never leak into the Scene.
`SceneOperatorGraphWorkspace.qml` owns graph layout and interaction; `WorkspaceSurface.qml` owns
Scene-graph-first navigation and the explicit node-focused workspace boundary. Selection only
updates shared context, while explicit open/review intent enters the media-specific workspace.
`TextOperatorWorkspace.qml` serves deterministic `text.edit` and Infer-backed `text.transform`;
`RasterCropOperatorWorkspace.qml` serves `image.crop`;
`AudioSpeechOperatorWorkspace.qml` serves preset-only `audio.speech_synthesize`; Source, Output,
future family fallbacks and unknown Operators use `ReadOnlyNodeWorkspace.qml` without acquiring edit
authority.
`VariantsPanel.qml` owns artifact-scoped Candidate Shelf selection and review controls, while
candidate identity and mutation remain in Rust. QML never becomes durable graph authority.

The desktop compatibility slice still treats every existing Artifact as one single-output Scene
and derives its graph from immutable accepted history. Behind that projection,
`shape-domain::scene` now owns stable Scene identity, immutable `SceneRevision`, and the requirement
that every accepted Output node has one unique portable name. `shape-store` persists Scene heads and
graph revisions with expected-head compare-and-swap, verifies that every durable node binding
resolves to a real Artifact Revision or Transformation, and additively migrates initial and
Scene-era schemas to the current audio-capable revision.
`shape-core::project::scene` keeps graph candidates transient until explicit acceptance. Two Scenes
can therefore evolve and reopen independently without sharing draft state. The desktop compatibility
slice now exposes type-compatible session-local Operator drafts so the authoring entry flow is real
without writing incomplete nodes into persistent history. Persistent desktop Scene graph mutation,
editable Operator parameters, a real multi-output Operator/UI flow, and reusable GraphComponent
instances remain deferred until their real UI and Operator consumers freeze those contracts.

Developer launch lifecycle is a separate repository-tooling owner under `scripts/`. A validated
candidate app is copied into an immutable revision-stamped release, then a product-side lock guards
the atomic `current-debug` symlink advance. The stable launcher never points at an agent-specific
candidate build and never overwrites a running application bundle.
