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
compare-and-swap commit boundary. `shape-execution::infer_runtime` owns the real Infer Runtime
consumer while the frozen official `infer-runtime-client` SDK owns
`infra.discovery.registration@20260812.1`, generation-aware endpoint selection, canonical
numeric-loopback validation, proxy/redirect policy, owner-only credential loading, exact
`infer-runtime.consumer-core@20260813.1` and
`infer-runtime.capability-catalog@20260813.1` negotiation, capability headers, and public error
decoding. `infer_runtime::sdk` is only the synchronous `Executor` adaptation consumed by the probe,
Responses, speech, image generation, and Job clients; it does not recreate wire contracts. An
explicit development endpoint may override Discovery, but there is no product fixed-port fallback.
The separate Shape `credential` owner continues only the existing managed-token install/status
lifecycle; token bytes are handed to the SDK by owner-only file path.

The `responses` owner requests the stable `text.edit` Intent with
`infer.deployment_ids=ollama_qwen3_5_4b`, `capability_floor=foundational`, and an exact
local-first/local-only/offline/no-fallback/zero-cost policy. It reads the typed SDK Job back and
copies exact Core/Capability, named routing, and successful Attempt facts into the payload-free
receipt, but Runtime success remains only a Shape Candidate. `shape-core::propose_generated_text`
owns the generative transformation and
accepted-text context; `shape-desktop-bridge::infer_text` prepares a candidate outside the live
session and `DesktopSession` revalidates its expected head before adopting it. Detailed provenance
UI, additional Intents, and the project dependency resolver remain deferred until real product
paths consume them. Do not create empty crates for roadmap boxes.

The first raster path extends those existing owners instead of introducing a media-kernel crate.
`shape-domain::image_raster` owns the platform-independent RGBA8, alpha, orientation, color, and
ICC contracts; dedicated image owners separately freeze Crop, Resize, orientation Transform,
Gaussian Blur, flattened Drop Shadow, and Unsharp Mask semantics.
`shape-execution::raster` owns bounded PNG/JPEG decode, EXIF normalization and canonical PNG
materialization, while its `crop` owner executes exact deterministic pixels. `shape-core::project::image` owns
the atomic import-origin commit and image Candidate/Accept use cases. The desktop shelf is now a
typed text-or-image collection with the same identity, expected-head, discard, and sibling
invalidation rules. Image-candidate clones share one immutable byte allocation, so exact-ID
acceptance does not duplicate the full encoded payload. Ordinary bridge snapshots project only
image dimensions and immutable content identity; selected accepted or candidate PNG bytes cross
the Rust/CXX boundary only on demand and are decoded into a display-scaled, byte-bounded native
cache. `ImageEditorWorkspace.qml` is the product-level image editing owner; it composes
`RasterCropOperatorWorkspace.qml`, `RasterResizeOperatorWorkspace.qml`, and
`RasterEffectsWorkspace.qml` as internal tools while
`ImageCompareWorkspace.qml` owns accepted-versus-candidate presentation. The bridge retires any
crop/resize Working Graph drafts together when either tool produces a Candidate, so the unified
stage cannot leave a sibling tool draft stale. Image branching, composites, masks, and model-backed
image operations remain deferred until a concrete consumer freezes each contract. Creative color
adjustment is intentionally not assigned to a second built-in photo pipeline: the planned
`image.color_grade` semantic Operator should use an explicit Shadow external-edit Adapter first. The
Adapter pins the Shape input revision and color contract, receives materialized bytes plus a receipt,
and can only create a transient Candidate; neither application may mutate the other's database.
`shape-domain::image_resize` now separately owns the typed `image.resize` dimensions, aspect, and
resampling contract. One opaque prepared plan carries its validated source and output interpretation
into `shape-execution::raster::resize`; request JSON cannot reinterpret it. The executor accepts only
materialized canonical PNG, preserves alpha/color/ICC interpretation, and applies explicit 32K-axis
and 64-Mi-pixel bounds. `shape-core::project::image::raster_resize` owns the corresponding transient
Candidate and explicit Accept use case. `shape-desktop-bridge::operator_catalog::image_resize` owns
the versioned draft codec; `RasterResizeOperatorWorkspace.qml` is its real desktop consumer and
checkpoints authored dimensions/policies before execution reloads the exact draft identity.

`shape-execution::raster::pixels` owns the shared fixed-point sRGB/linear conversion,
premultiplied-alpha math, source-over composition, and reusable two-buffer three-box Gaussian
approximation. Blur and `image.unsharp_mask` both admit at most 16,777,216 pixels: a 4096-square
source passes planning, while an 8192-square source fails before allocation. Unsharp Mask preserves
source alpha exactly and applies radius, fixed-point amount, and threshold to linear-premultiplied
RGB under revision `20260813.1`; its desktop controls are transient, and only Candidate acceptance
persists those exact parameters. This is a portable CPU foundation, not a generic Filter SDK or a
second photo-development pipeline.

`shape-domain::ai_image` owns the first comprehensive image-generation family without leaking a
provider workflow into the creative graph. `image.generate` is a zero-material Source Operator;
`image.generate_from_materials` requires one or more accepted raster references with explicit
roles. Both have one logical image output, so requested variants belong to the Candidate Shelf and
never change graph port arity. The exact `20260811.1` parameter codec rejects model/provider/sampler
fields, unknown revisions, duplicate materials, unanchored preserve constraints, and unsafe canvas
or candidate bounds. `shape-execution::infer_runtime::image_generation` is the first physical
consumer for the source-less form: it uses the SDK's exact `infer.responses@20260812.1` capability,
revalidates canonical Base64 PNG against the 20 MiB/4096-axis/16,777,216-pixel limits, and requires
a successful cloud/subscription Job with no fallback. `infer_runtime::job_provenance` validates
Shape policy over the SDK's typed Job vocabulary used by text, speech, and image adapters.
`shape-core::project::image::ai_generate` keeps the
normalized PNG, typed operation, and receipt transient; explicit Accept alone advances an existing
unaccepted ImageRaster source identity to its first immutable revision. The desktop atomically
creates that identity together with a zero-input Working Graph, persists the exact prompt/canvas,
re-reads the draft before credential access, and adopts results through an independent
`InferImageController`. `AiImageOperatorWorkspace.qml` and the real `OperatorIntentSidebar`
consumer present one comprehensive Source Operator. The draft remains recoverable while Candidates
are transient and is cleared only after acceptance. The live Shape App ACL does not currently
authorize the required `image.generate`/subscription/balanced/cloud-only execution policy, so real
generation fails closed without changing accepted history. Material-conditioned execution remains
unavailable until Infer publishes a stable typed raster-output provider, Capability Schema, and SDK
client; a mock, text-only, or image-description route must not be substituted.

The first audio foundation continues those same dependency directions without introducing a media
kernel. `shape-domain::audio` owns `audio.generate`,
`audio.speech_synthesize`, and `audio.transform` port contracts, exact accepted WAV interpretation,
versioned preset aliases, and the local-only consent/disclosure boundary for Voice References.
`shape-execution::audio` validates exact PCM S16 LE WAV bytes; `infer_runtime::speech` uses the
official `infer.audio.speech@20260811.1` unary WAV client, requests only
`mlx_qwen3_tts_custom_voice_1_7b`, and returns bounded payload-free Core/Capability/Job/routing/
Attempt facts. `shape-core::project::audio` keeps synthesized bytes transient and creates a new AudioClip
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

The workbench shell now composes four additional presentation owners instead of retaining a
permanent three-column dashboard. `WorkbenchProjectRail` owns compact project navigation;
`SceneGraphContextStrip` preserves clickable Source/Operator/Output context above a focused media
workspace; `CandidateFilmstrip` owns horizontal transient-result review actions; and
`OperatorIntentSidebar` owns the reusable Intent/Change/Preserve/Reference presentation and is now
consumed by the persisted zero-input `image.generate` draft. `Main.qml` only binds these regions to backend identities and
signals. None of these QML owners may write project state or infer acceptance.
The product direction is node-first. The Scene Working Graph is the primary creation surface even
before content exists: Source and Operator nodes may be created while detached, typed connections
bind outputs to inputs, and an output-port `+` is a shortcut for inserting a compatible downstream
node. The current artifact-as-Scene desktop projection is an explicit compatibility slice, not the
authority for future graph mutation. It must not be expanded to fake multi-source Scene membership;
the next persistence revision needs a real Scene-owned Working Graph with node geometry, ports,
edges, detached-node recovery, and output bindings.
The current compatibility bridge now permits one honest detached `text.edit` entry: adding the AI
Text Editor without a selected text source atomically creates an unaccepted `TextDocument` target
and a zero-input Working Graph whose reusable intent is durable. The workspace labels it as waiting
for materials and refuses execution. This proves node-before-material authoring without claiming
that the compatibility Artifact is already a multi-node Scene or that typed connection mutation
exists.
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
`InferImageController` likewise owns the high-payload cloud-image lifecycle; large PNG bytes remain
opaque until the UI-thread session adopts the Candidate and a selected preview is requested.
The graph-aware desktop information architecture adds `ProjectNavigator.qml` for Scene selection
and `ContextInspector.qml` for Explore, Details, and Lineage modes. `OperatorWorkspaceHost.qml`
owns focused-workspace routing and lifecycle; text, image editing, and audio workspaces remain
separate semantic owners.
`shape-domain::operator_graph` now owns the platform-independent typed
DAG contract: explicit Source, Operator, and Output roles; typed ports; multiple outputs; unique
input binding; and cycle rejection. `shape-desktop-bridge::operator_graph` projects persisted
accepted Revision/Transformation history into that contract. It stops cross-Artifact history at a
pure Source boundary, so executor steps and media-internal structure never leak into the Scene.
`SceneOperatorGraphWorkspace.qml` owns graph layout and interaction, while
`CreativeGraphNodeCard.qml` owns role- and media-specific accepted-node presentation. Source cards
show origin context, editing cards show their creative method family, and the current Result card
renders the selected Artifact's actual text, image, or audio summary. `WorkspaceSurface.qml` owns
Scene-graph-first navigation and the explicit node-focused workspace boundary. Selection only
updates shared context, while explicit open/review intent enters the media-specific workspace.
`AiTextEditingWorkspace.qml` owns the focused AI text-edit lifecycle: external material summary,
quick action, authored prompt, sequential one-or-three Candidate generation, selectable
output, targeted follow-up preparation, and explicit output lock. New compatibility drafts still use
canonical `text.edit`, while legacy persisted `text.transform` drafts remain readable. The
`TextExpressionPalette.qml` semantic owner presents readable built-in expression samples,
primary/supporting one-or-two tone composition, intensity, audience, and personal preset authoring.
`TextExpressionLibrary` owns the
bounded preference lifecycle for those reusable personal presets; project drafts copy the selected
authored meaning rather than retaining a live preference reference. The
`shape.operator-draft.text-transform@20260813.1` codec persists
rewrite/expand/polish/shorten/summarize, exact instruction, ordered tone facets, optional custom
example, intensity, audience, style, and bounded variant count. The previous `20260812.2` expression
schema remains readable.
Generation re-reads that project-backed configuration; producing a Candidate no longer deletes
reusable text-node intent, and locking a text Candidate rebases the Working Graph to the new accepted
head. `ImageEditorWorkspace.qml`
presents one Image Editing stage; `image.crop` and `image.resize` remain exact internal contracts and
accepted Transformation identities rather than separate palette entries;
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
Scene-era schemas through the audio-capable and Working Graph revisions to schema `20260811.5`.
`shape-core::project::scene` keeps graph candidates transient until explicit acceptance. Two Scenes
can therefore evolve and reopen independently without sharing draft state. The desktop compatibility
slice stores type-compatible Operator drafts in a mutable Artifact Working Graph keyed to the exact
accepted head. Unexecuted drafts restore on reopen without becoming accepted nodes; stale saves fail
closed. `shape-desktop-bridge::operator_catalog` owns the currently executable compatibility and
machine routing descriptors, while QML owns localized labels and search. Working Graph anchors and
input data types are optional only as one validated pair for a true zero-input Source Operator;
accepted-input drafts preserve their previous serialized form. Full persistent Scene graph
`shape-domain::working_graph` also owns a bounded versioned JSON envelope for mutable Operator-owned
configuration without interpreting media semantics. `operator_catalog::text_transform` validates
AI text-edit state: the exact rewrite/expand/polish/shorten/summarize mode, instruction, tone,
intensity, audience, style, and variant-count contract, including all older configuration revisions and legacy
`text.transform` draft identity. The visible catalog exposes one AI Text Editor action, while
`operator_catalog::audio_speech` validates the preset, language, pace, and
mandatory disclosure contract. Both Infer controllers execute from an exact project-backed draft
identity and re-read its Working Graph configuration instead of trusting a QML projection. Full
Scene-owned Working Graph mutation, durable typed connections, multi-material text pipeline
compilation, durable unaccepted Explorations, a real multi-output flow, and reusable GraphComponent
instances remain the next contract slice. The compatibility UI may expose node-first affordances,
but it must label the current text-only executor boundary and must not claim that image/audio inputs
participated before their real compiler stages exist.

Developer launch lifecycle is a separate repository-tooling owner under `scripts/`. A validated
candidate app is copied into an immutable revision-stamped release, then a product-side lock guards
the atomic `current-debug` symlink advance. The stable launcher never points at an agent-specific
candidate build and never overwrites a running application bundle.
