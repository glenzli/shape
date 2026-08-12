# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. An empty launch presents
explicit start-new and continue-project actions. Once a project is open, the product direction is a
node-first Scene canvas: adding an editing node does not depend on node selection, typed output
ports expose a `+` insertion affordance, and focused work happens only after the user opens a node.
The current artifact-as-Scene implementation remains a compatibility projection while a real
Scene-owned mutable graph is added; it must not be mistaken for the final multi-source persistence
model.
A blank or non-text compatibility canvas still exposes the AI Text Editor in the node library.
Choosing it creates an unaccepted text target plus a durable zero-input `text.edit` Working Graph,
so the node and its authored intent exist before any material does. The focused workspace clearly
shows “waiting for materials” and keeps generation disabled. This is a migration bridge, not a
claim that the current artifact-as-Scene model can already persist arbitrary cross-media edges.
A text work starts as one atomic accepted source revision and immediately enters a recoverable
writing draft; an AI image work atomically creates an unaccepted raster identity and its zero-input
`image.generate` Working Graph, then opens the image workspace directly. The shell can
also open a bundle at startup, display its validated project metadata and artifact list, and render
bounded accepted text or an on-demand verified raster preview supplied by Rust. Text documents can
accumulate multiple transient candidates, compare and accept them in place, or branch one into a
new artifact. An 8-bit PNG/JPEG can be imported into a canonical `image.raster` revision, cropped,
resized, losslessly reoriented, blurred, or given a flattened drop shadow; every edit is compared
as a transient Candidate before its exact versioned operation is accepted and reopened.
Accepted text can also enter the preset-only Speech Synthesis workspace, produce a transient
`audio.clip` Candidate through Infer Runtime, play the selected exact WAV in memory, and accept it
as a new cross-artifact output without advancing the source text.
Navigation, draft, Candidate Shelf, compare, and semantic-history
regions retain complete English and Simplified Chinese message identities. Language can switch at
runtime between system, English, and Simplified Chinese. Appearance defaults to the system color
scheme and can be pinned to light or dark; both preferences persist across launches.

The shell is organized around a workflow overview, media-specific focused workspaces, and
a context inspector with Explore, Details, and Lineage modes. The graph is the default Scene view:
single selection updates shared inspection state, while an explicit open or double-click enters
the selected node's workspace. Source, Operator, and Output are different semantic roles rather
than one generic artifact card. Node-focused operation panels are absent from the graph home and
returning to the graph preserves project-level context. Lineage is not placeholder UI: Rust
loads the accepted revision's persisted parent identities and creative Transformation, while C++
projects localized transformation labels and resolves transformation inputs to source artifact
names. Candidates remain in Explore and outside durable lineage until acceptance. Branching creates
the new artifact, its first revision, target-specific execution receipt, and cross-artifact input
edge in one atomic persistence transaction without advancing the source artifact.

The central surface switches between the selected Scene's typed Operator Graph and a node-focused
media workspace. Rust projects accepted Revision/Transformation history as
`Source -> Operator -> Output`, including exact typed ports and cross-artifact Source boundaries;
QML lays out and draws that graph without becoming semantic authority. Scene selection is shared
across the navigator, graph, workspace, and inspector. The graph provides real zoom/fit controls,
a compact selection summary, and a searchable media-compatible Operator palette. Palette selection
uses a Rust-owned Operator catalog for source-bound actions; the always-authorable AI Text Editor
either reuses a compatible text draft or creates the detached compatibility target described above.
The selection immediately enters the dedicated Operator workspace. Unexecuted drafts are stored in the
project's mutable Working Graph, restore with the same identity on reopen, and remain outside
accepted history. Text transformation is presented as an AI Text Editor node and a dedicated
`AiTextEditingWorkspace.qml`. The workspace keeps connected material context outside the editor,
then owns quick actions, optional prompt, readable tone samples, style, audience, one-or-three Candidate pacing, selectable
output, targeted follow-up preparation, and explicit output locking. Compatibility work uses
canonical `text.edit`; `shape.operator-draft.text-transform@20260813.1` checkpoints
rewrite/expand/polish/shorten/summarize, exact authored instruction, one-or-two tone facets,
subtle/balanced/strong intensity, preset or authored audience, style, and bounded variant count.
Built-in tones pair Shape-owned marks with a plain-language description and one expression sample;
selection order explicitly means primary and supporting tone. A bounded personal tone library lives
in the application preferences, but the selected name, instruction, example, and visual mark are
copied into the project draft so later library edits never change old node intent. The previous
`20260812.2` expression schema and older `text.transform` drafts remain readable. The Infer Text
controller receives only the draft identity; Rust reopens the Working Graph, compiles the persisted
intent, and revalidates it before execution.
Raster authoring is likewise presented as one Image Editing action. Its workspace exposes Frame and
Size as internal tools backed by the existing exact `image.crop` and `image.resize` contracts;
choosing one does not invent a generic executor or erase the accepted Transformation identity.
When either tool produces a Candidate, sibling crop/resize drafts are retired together so the one
product stage cannot leave a stale hidden draft. Material-aware AI assistance stays visibly
unavailable until Infer exposes a real typed image-edit execution surface.
Image and audio compatibility executions may still retire their tool draft after producing a
Candidate. AI text execution deliberately does not: the node Draft is reusable authored intent,
while each exact generated result belongs to the Candidate Shelf. Locking a text Candidate advances
immutable history and rebases the same draft to the new accepted head.
The zero-input AI Image draft remains recoverable while generated Candidates are transient and is
cleared only when one Candidate is explicitly accepted.
Pending candidates remain outside the durable accepted graph until explicit acceptance or branching.

Presentation uses progressive disclosure. The project rail calls compatibility Scenes “works,”
Source/Operator/Output appear as starting point/creative step/current result, and Candidate actions
appear as new versions and “Use this version.” These labels do not alter Rust identities or durable
contracts. Accepted graph cards are role- and media-specific: the current Result card contains a
real text excerpt, selected image preview, or audio summary instead of only a type label. The focused
workspace header never displays raw route keys; “See workflow” remains the
explicit route back to the graph for users who want structural control.

This foundation currently adapts each existing Artifact into a single-output Scene. That
compatibility boundary lets media-specific Operator work proceed without pretending that the final
Scene, named multi-output, or reusable GraphComponent persistence model already exists.

[`shape-desktop-bridge`](../../crates/shape-desktop-bridge/src/lib.rs) owns the generated CXX ABI.
Rust's bounded `DesktopSession` validates SQLite and content objects and owns the open project;
`CandidateShelf` owns its in-memory, newest-first candidate collection and exact-ID mutations.
`OperatorDrafts` mirrors the project-backed, typed Working Graph entries that precede execution;
the Rust store validates their exact accepted-head anchor before saving.
In-place acceptance and new-artifact branching still delegate to atomic project use cases. C++ maps
explicit snapshots and commands into Qt presentation values; QML never reads or writes project
files. Ordinary snapshots carry raster/audio metadata rather than encoded payloads; selected
accepted and candidate PNG/WAV bytes are fetched separately. Images enter a byte-bounded,
display-scaled native preview cache, while only one selected WAV enters the playback owner. Pinned
or durable explorations, image composites and layers, model-backed image editing, waveform editing,
recording, Shadow/Echo interop, and third-party editor process control remain behind future
capability adapters. The first planned external-edit route is Shadow-backed creative color grading:
the desktop will open a pinned Shape revision under an explicit representation/color contract and
will treat returned bytes plus receipt as a Candidate, never as an in-place database mutation.

`InferRuntimeController` runs the bounded public-contract probe away from the UI thread and projects
only checking, reachable, compatible, contract-version, endpoint-source, instance/generation, and
stable-error state. Rust selects endpoints in this order: canonical numeric-loopback
`SHAPE_INFER_RUNTIME_URL` override, live owner-only
`infer-runtime.consumer@0.1.0-candidate.4` Infra Discovery offer, then the temporary
`http://127.0.0.1:8787` migration fallback. It validates the exact Discovery schema, filesystem
ownership and modes, generation, offer binding, and raw endpoint before HTTP. The
`infra.discovery.registration@20260812.1` manifest has no lease or liveness timestamp; a failed
connection causes a stable-manifest re-read without repairing or deleting provider state.
Every protected contract, inference, and Job request carries
`Infer-Consumer-Contract: 0.1.0-candidate.4`; Shape rejects a manifest whose supported-version set
contains anything else.

`InferTextController` separately owns one asynchronous authenticated generation lifecycle. Settings
can copy the one-time managed token for Infer App `shape` into
`AppConfigLocation/secrets/infer-runtime.token`; Rust atomically maintains its owner-only directory
and file and never puts the token in QSettings, a project, exported settings, arguments, environment,
or diagnostics. The request is fixed to candidate.4 `text.edit` with an interactive
`infer.capability_floor=foundational`, the authorized `ollama_qwen3_5_4b` Deployment, local-first,
local-only, offline, no-fallback, and zero cloud cost. Shape does not negotiate or deserialize older
Consumer vocabularies; a retired Runtime requires an upgrade. Proxies, redirects, hostnames, and
remote origins are rejected.
The worker opens a read-only project view and returns an opaque generated candidate; the UI-thread
`DesktopSession` rechecks expected-head identity before adding it to the transient shelf. Only the
existing accept operation advances durable history. Infer App/ACL creation and daemon restart remain
explicit Infer Console operations rather than Shape configuration mutations.

`infer_runtime_access` owns the shared credential status/install boundary now consumed by text and
media generation. `InferSpeechController` owns its own asynchronous preset-synthesis lifecycle and hands
the move-only result to `DesktopSession`, which rechecks the accepted source head before adding the
new audio target to the shelf. Candidate projection distinguishes target Artifact identity from
the source Artifact that owns review context. `AudioPreviewController` separately owns one selected
WAV allocation, `QBuffer`, `QMediaPlayer`, position, seek, and terminal playback errors; ordinary
snapshots and QML never carry encoded audio. `AudioSpeechOperatorWorkspace.qml` owns voice-preset,
pace, naming, disclosure, generation, and audition presentation. Preset, catalog, language, pace,
and mandatory synthetic disclosure are stored as one exact Operator-owned configuration. Legacy
speech drafts receive and persist the validated default on open, and generation re-reads the exact
draft identity instead of accepting UI parameters as execution authority. This is a preview surface,
not the future Echo-backed composition/render engine.

`InferImageController` owns the separate high-payload candidate.4 image generation lifecycle.
Rust reopens the exact zero-input draft and validates its prompt/canvas before credential access,
then requires cloud/subscription Job provenance before returning an opaque Candidate. The desktop
never sends provider, model, sampler, or checkpoint choices. `AiImageOperatorWorkspace.qml` owns the
central canvas and compact output-size choice, while `OperatorIntentSidebar.qml` is its real
comprehensive intent consumer. Candidate PNG bytes cross only for selected preview. Current Infer
App authority does not yet admit this cloud route, so the UI reports the permission failure without
fabricating a result or changing project history.

`UiPreferences` is the process-level owner for appearance, effective system color scheme,
translation lifecycle, and persistence. `MainTitleBar.qml` owns the fused toolbar/title region.
The shared shell uses Qt's expanded client area and safe-area margins on every platform; the small
Objective-C++ adapter only aligns native macOS traffic-light buttons with that shared toolbar.
`DesktopBackend` is a presentation facade over the Rust session; project and candidate authority
never enters QML or UI settings. `ProjectWelcome.qml`, `CreateProjectDialog.qml`,
`CreateSceneTypeDialog.qml`, `CreateTextSceneDialog.qml`, and `CreateAiImageSceneDialog.qml` own the
goal-first launch and first-content presentation flow without acquiring
persistence authority. `TextOperatorWorkspace.qml` owns accepted text and Candidate presentation
for the single Writing step; `IntentPanel.qml` owns its direct-writing/AI-assistance method switch,
and `TextCompareWorkspace.qml` remains its compare
owner. `ImageEditorWorkspace.qml` owns the one product-level image editing surface;
`RasterCropOperatorWorkspace.qml` owns crop-frame interaction,
`RasterResizeOperatorWorkspace.qml` owns project-backed dimensions, aspect policy and resampling,
`RasterEffectsWorkspace.qml` owns transient orientation, blur, unsharp-mask sharpening, and
flattened drop-shadow controls,
and `ImageCompareWorkspace.qml` owns raster comparison. Effects create real transient Candidates;
their accepted Transformation records the exact versioned parameters, while QML never owns pixels,
history, or executor policy. Rust remains the exact pixel, draft, and persistence authority.
`WorkbenchProjectRail.qml` now owns the compact Scene/Component/Asset navigation surface while
accepted Artifact projection remains the temporary Scene compatibility model. Entering an Operator
keeps `SceneGraphContextStrip.qml` visible above the media workspace; it emits identity-based open
intent without owning the graph. `CandidateFilmstrip.qml` replaces the permanent dashboard shelf
with horizontal, identity-addressed Select/Compare/Accept/Discard/Branch actions wired to the same
session authority. `OperatorIntentSidebar.qml` is the extracted presentation owner for
comprehensive Intent/Change/Preserve/Reference nodes and is instantiated for the real persisted
`image.generate` draft. The older `ProjectNavigator.qml`, `ContextInspector.qml`, and
`VariantsPanel.qml` remain packaged compatibility components but are no longer the main shell.
`BranchArtifactDialog.qml` owns branch naming and submission;
`SceneOperatorGraphWorkspace.qml` owns typed graph layout, zoom, single-selection, and open/review
intent; `CreativeGraphNodeCard.qml` owns accepted media- and role-specific node content,
`CreativeDraftNodeCard.qml` owns mutable step affordances, and
`CreativeCandidateNodeCard.qml` owns transient generated-version presentation. Keeping those cards
separate prevents the graph layout owner from also becoming the visual-state owner. Text Source and
Result cards receive the exact immutable revision preview projected by Rust, so neither card reuses
the selected Artifact head as a substitute for its own bound content. `SceneGraphToolbar.qml` owns
the scene breadcrumb, graph controls, and palette entry.
`shape-desktop-bridge::operator_catalog` owns executable compatibility and routing descriptors;
`OperatorPalette.qml` owns their localized labels and presentation-only search, while
`GraphSelectionInspector.qml` owns the compact selected-node action summary without acquiring node
or lifecycle authority. `ShapeButton.qml`, `ShapeIconButton.qml`, and the shared SVG resources keep
toolbar action treatment and content centering consistent. `Theme.qml` is the desktop visual
contract shared in density and neutral-surface hierarchy with Shadow and Echo; Shape-specific
canvas-grid and creative-state tokens live there instead of being redefined by workspaces.
Connected rails, canvases, inspectors, and focused editors use separators for depth, reserving
floating cards and shadows for palettes and dialogs. `SourceMaterialWorkspace.qml` owns the
content-first, non-technical viewer for the exact immutable Source revision; internal identities
remain outside its primary UI. `OperatorWorkspaceHost.qml` owns exact Operator routes and the
separate Source/Output read-only routes,
family fallbacks, Candidate identity, and the return lifecycle. `WorkspaceSurface.qml` remains the
graph-first composition boundary and `Main.qml` remains an assembly root. A new media workspace can
therefore evolve in its own QML owner and register through the Host without taking graph authority
or changing another media editor.

Build and smoke-start:

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev
```

For the stable Shadow/Echo-style local launch path, build, validate, promote, and launch with:

```sh
./scripts/build_and_promote_debug.sh
./scripts/run_debug.sh [/path/to/project.shape]
```

Promotion copies a complete candidate bundle into an immutable external release and atomically
advances `current-debug`; it never modifies a running application in place. `run_debug.sh` detaches
by default, supports `--foreground` and `--check`, and writes its log under the sibling
`.shape-local-build` directory.

After creating a demo through `shape-cli`, launch the platform executable with
`--project /path/to/project.shape`. The `shape-desktop-project-smoke` test performs the entire
create → link → open → propose → accept → propose → branch path automatically, including source
head preservation, the projected cross-artifact input edge, exact shelf selection/discard,
packaged Scene graph activation, Source -> Text Edit -> Output projection, node-focused workspace
entry/return, candidate selection synchronization, and presence of the packaged
Infer Runtime status control. The same packaged smoke also imports a raster, verifies its preview,
creates and compares a crop candidate without advancing the head, accepts it, and reopens the
cropped dimensions. It additionally opens the packaged Speech workspace from an accepted Text
Operator and retranslates that live workspace between English and Simplified Chinese. Rust bridge
tests cover audio generation adoption, selected-only WAV retrieval, acceptance, and reopen. The
runtime itself may remain offline during deterministic desktop smoke paths.

The no-project packaged smoke additionally follows the user-visible entry path: it creates a new
bundle, creates the first accepted Text Scene, begins and opens a compatible Text Edit draft,
reopens the project to verify that the exact draft restores, then discards it and verifies the
mutable Working Graph is empty. It then creates a zero-input AI Image Scene, verifies the projected
draft and dedicated workspace, and reopens the same draft identity without contacting Infer.

The shell requires Qt 6.9 or newer for the cross-platform expanded client area. Platform-specific
code is isolated to native window-control alignment; QML layout, themes, and settings are shared.
