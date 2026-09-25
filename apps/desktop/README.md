# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. An empty launch presents
explicit start-new and continue-project actions. Once a project is open, the product direction is a
node-first Scene canvas: adding an editing node does not depend on node selection, typed output
ports expose a `+` insertion affordance, and focused work happens only after the user opens a node.
The current artifact-as-Scene implementation remains a compatibility projection while a real
Scene-owned mutable graph is added; it must not be mistaken for the final multi-source persistence
model.
The text and speech workflow uses the [node contract](../../docs/NODE_MODEL.md).
Text creation starts empty and offers plain/script formats before the user writes. Text editing requires
an accepted original and allocates an independent output, with rewrite, translate, summarize, polish, expand, outline and script
preparation tasks. Both open `TextAuthoringWorkspace.qml`; optional tone, audience and style settings
reuse `TextExpressionPalette.qml`. `TextFormatPanel.qml` exposes the format and writing presets.
Rules are displayed as examples and a copyable offline guide. Speech consumes the adopted document's
persisted format and owns only delivery settings. Stable text/speech nodes survive repeated acceptance.
An 8-bit PNG/JPEG can be imported into a canonical `image.raster` revision, cropped,
resized, losslessly reoriented, blurred, or given a flattened drop shadow; every edit is compared
as a transient Candidate before its exact versioned operation is accepted and reopened.
The single Import material action also accepts bounded UTF-8 text/code files as editable text
sources and PCM S16 LE WAV files as playable audio sources. Explicit import accepts an immutable
source revision; later generated edits still use reviewable Candidates. Self-contained `.html`
files remain exact text Sources and can be opened in an ephemeral, offline Qt WebEngine preview.
The preview uses a memory-only profile, denies external URL requests, local files, permissions,
clipboard access, popups, and storage; closing it destroys the script context. Export preserves
the accepted HTML bytes. Other code files remain text and are not run. Audio imported from
outside Shape carries an unverified-origin disclosure rather
than claiming it was recorded or synthesized by Shape. The source path is not needed to reopen the
accepted material. Frame-accurate video export, runnable JS bundles, arbitrary audio codecs, and
an agent workspace handoff are not implemented by this intake path.
The material chooser can also paste clipboard text or an image directly into the open project.
Text keeps its exact UTF-8 content within 1 MiB. PNG/JPEG clipboard payloads enter the bounded
raster import path; a decoded clipboard image is encoded as PNG before that import. Both become
accepted Source revisions without a temporary file or recorded source path. Shape currently keeps
the canonical image revision rather than a separate original clipboard payload.
In the active Asset rail, compact Import and Paste actions expose those same paths after project
creation. Pasted text switches to Works, where text sources are currently listed; pasted images
stay in Assets.
Imported text/code retains the original file suffix in its work name; the source viewer presents
code as plain, monospaced text. Accepted text can be exported byte-for-byte as a supported UTF-8
text/code extension, including `.js`, without making it executable inside Shape.
The final-file strip exports accepted text as UTF-8 text/code, accepted raster images as canonical PNG,
and accepted audio as WAV through its existing export dialog. Files are outside the Shape bundle
and do not create extra graph nodes.
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
uses the Rust-owned Operator catalog for both source-free creation and source-bound editing.
Text and speech graphs use stable node identities and explicit source bindings; generation history
is inspected separately. Both text actions use the same workspace. Writing presets, tone, audience,
style and exact buffers are stored in the authoring configuration. Personal presets are snapshots,
not live dependencies. Infer receives a persisted node identity and revalidates its exact configuration.
Raster authoring is likewise presented as one Image Editing action. Its workspace exposes Frame and
Size as internal tools backed by the existing exact `image.crop` and `image.resize` contracts;
choosing one does not invent a generic executor or erase the accepted Transformation identity.
When either tool produces a Candidate, sibling crop/resize drafts are retired together so the one
product stage cannot leave a stale hidden draft. Material-aware AI assistance stays visibly
unavailable until Infer exposes a real typed image-edit execution surface.
Image compatibility executions may still retire their tool draft after producing a
Candidate. AI text execution deliberately does not: the node Draft is reusable authored intent,
while each exact generated result belongs to the Candidate Shelf. Locking a text Candidate advances
immutable history and rebases the same draft to the new accepted head.
The zero-input AI Image draft remains recoverable while generated Candidates are transient. It can
be discarded before acceptance and is also cleared when one Candidate is explicitly accepted.
One image run can request one to four independently receipted Candidates. They appear in the same
Candidate Shelf with small previews as each request completes. Progress shows the number ready; Stop
finishes the active request and skips later requests. Completed Candidates remain available after a
stop or later request failure. Choosing one does not modify the accepted image until Use this
version, which becomes available when the run ends. Selecting an earlier Candidate while later
requests run is preserved as new results arrive. A style reference
must bind an accepted raster revision to a separate material-conditioned image operator with an
explicit role; that execution path is reserved until the runtime exposes typed raster input.
An unfinished speech step remains removable even when its saved Operator graph is already shown
on the canvas.
Pending candidates remain outside the durable accepted graph until explicit acceptance or branching.

Presentation uses progressive disclosure. The project rail calls compatibility Scenes “works,”
Source/Operator appear as starting point/creative step, with each Output shown at its producing
step's endpoint. Candidate actions appear as new versions and “Use this version.” These labels do
not alter Rust identities or durable contracts. The endpoint keeps the result's own selection and
history identity; opening it focuses its producing step, which presents accepted content and
pending work together. The focused workspace header has a labeled return to the graph.
The rail groups text edits and speech-derived audio under their source text as one visible work.
The work canvas joins each Artifact's existing graph at an exact accepted input revision: an
original text can lead to script text, speech synthesis, and audio output in one view. A downstream
node with a single existing connection opens from its upstream port; the node library adds a new
branch only when requested. A manually written root text appears as one content card, while AI
creation keeps its execution step and output endpoint visible. Unaccepted draft nodes, including zero-input starting
points, can be discarded with their empty reserved outputs; accepted history remains immutable.
An existing speech step is reused when continuing from its source text.
An older pinned input is marked on its connection, so the canvas does not imply that the next step
uses the current text. Node opening selects the owning Artifact and retains its independent
accepted history. Pending versions stay attached to the selected output's step. This relationship
view is derived from persisted input bindings; it does not write a Scene graph or invent editable
cross-Artifact ports. An output without an accepted revision says so instead of displaying zero-valued
media metadata.

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

`InferRuntimeController` runs the bounded official-SDK contract probe away from the UI thread and
projects only checking, reachable, compatible, contract-version, endpoint-source,
instance/generation, and stable-error state. The SDK selects a canonical numeric-loopback
`SHAPE_INFER_RUNTIME_URL` development override or the live owner-only
`infer-runtime.consumer-core@20260813.1` Infra Discovery offer. There is no fixed-port product
fallback. The SDK validates the unchanged `infra.discovery.registration@20260812.1` document,
filesystem ownership/modes, generation, binding, Core OpenAPI digest, dated Capability Catalog, and
the exact Responses/speech capability records without repairing or deleting provider state. Each
data-plane call additionally verifies its capability schema before transport.

`InferTextController` separately owns one asynchronous authenticated generation lifecycle. Settings
can copy the one-time managed token for Infer App `shape` into
`AppConfigLocation/secrets/infer-runtime.token`; Rust atomically maintains its owner-only directory
and file and never puts the token in QSettings, a project, exported settings, arguments, environment,
or diagnostics. The request uses stable `text.edit`, the ACL-authorized
`infer.deployment_ids=ollama_qwen3_5_4b`, interactive `capability_floor=foundational`, local-first,
local-only, offline, no-fallback, and zero cost. The SDK sends the exact Core and Responses
capability headers and reads back typed Job/Attempt/named-routing provenance. Candidate contracts
are not retained. Proxies, redirects, hostnames, and remote origins are rejected by the SDK.
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

The speech workspace exposes nine Runtime voice aliases (catalogue `20260922.1`), with automatic
mixed-language detection by default and an independent explicit language override. Existing authored
`20260811.1` Chinese presets remain readable. Narration accepts up to 64 KiB of UTF-8 text, splits
around sentence/word boundaries at 240 characters, and assembles at most 128 MiB of matching PCM WAV.
Leading/trailing whitespace stays attached to spoken segments. Progress reports completed segments;
Stop finishes the active request first. Retrying the same source and settings in the same app session
reuses successful segments. Changes to text, voice, language, pace, or disclosure invalidate that
cache; completed narration releases it. Runtime failures never publish partial audio or accepted
history. The local Qwen worker applies pitch-preserving pace after synthesis.

`AudioExportController` independently owns export. The toolbar opens a native WAV save dialog for
selected candidate or accepted audio. It writes the exact verified bytes through `QSaveFile`, with
no direct-write fallback, and rejects destinations inside a project bundle. Export does not accept a
Candidate. `AudioExportDialog.qml` owns destination selection and localized completion/error feedback.

An opt-in real-service check is available on the built desktop binary:
`shape-desktop --smoke-speech-live /absolute/existing/output-directory`.
It requires configured Shape credentials and an available local Runtime, creates its own project,
and exercises packaged QML voice/language choice, multi-segment mixed-language synthesis, a second
voice, actual playback/seek, byte-exact candidate/accepted export, bundle-path rejection, explicit
acceptance and reopen. It is deliberately excluded from offline CTest smoke checks.

`InferImageController` owns the separate high-payload source-less image generation lifecycle over
the stable Core and Responses capability.
Rust reopens the exact zero-input draft and validates its prompt/canvas before credential access,
then requires cloud/subscription Job provenance before returning an opaque Candidate. Each
successful physical request enters a thread-safe queue for UI-thread Candidate adoption and thumbnail
caching; Stop is checked before the next request. The desktop
uses the settings default (GPT-5.6 Luna initially) or the model chosen for the current run (GPT-6 Luna/Sol are also available), without exposing provider-native model details. `AiImageOperatorWorkspace.qml` owns the
central canvas and compact output-size choice, while `OperatorIntentSidebar.qml` is its real
comprehensive intent consumer. Candidate PNG bytes cross only for selected preview. Infer must grant Shape the image.generate intent, Luna route, subscription access, cloud text input,
and balanced cloud request overrides. Missing route grants are reported without changing history.
Material-conditioned `image.edit` remains unavailable because Runtime has not published a stable
raster-output provider/Capability/SDK contract; the desktop does not substitute text output.

`UiPreferences` is the process-level owner for appearance, effective system color scheme,
translation lifecycle, and persistence. `MainTitleBar.qml` owns the fused toolbar/title region.
The shared shell uses Qt's expanded client area and safe-area margins on every platform; the small
Objective-C++ adapter only aligns native macOS traffic-light buttons with that shared toolbar.
`DesktopBackend` is a presentation facade over the Rust session; project and candidate authority
never enters QML or UI settings. `ProjectWelcome.qml`, `CreateProjectDialog.qml`,
`CreateSceneTypeDialog.qml`, `CreateTextSceneDialog.qml`, and `CreateAiImageSceneDialog.qml` own the
goal-first launch and first-content presentation flow without acquiring
persistence authority. `RecentProjects` stores only paths and display names after successful opens;
the welcome page disables entries whose local bundle is missing and reopens entries through
`DesktopBackend`. `TextOperatorWorkspace.qml` owns accepted text and Candidate presentation
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
uses a labeled return action in `OperatorWorkspaceHost.qml` for all focused workspaces.
`CandidateFilmstrip.qml` replaces the permanent dashboard shelf
with horizontal, identity-addressed Select/Compare/Accept/Discard/Branch actions wired to the same
session authority. `OperatorIntentSidebar.qml` is the extracted presentation owner for
comprehensive Intent/Change/Preserve/Reference nodes and is instantiated for the real persisted
`image.generate` draft. The older `ProjectNavigator.qml`, `ContextInspector.qml`, and
`VariantsPanel.qml` remain packaged compatibility components but are no longer the main shell.
`BranchArtifactDialog.qml` owns branch naming and submission;
`SceneOperatorGraphWorkspace.qml` owns typed graph layout, zoom, single-selection, and open/review
intent. Its separate final-file bar renders an accepted audio revision to a WAV through the existing
`AudioExportController`; it is not a Scene node or a durable Project Export. Other file formats can
use the same presentation boundary when their export controllers exist.
`CreativeGraphNodeCard.qml` owns accepted media- and role-specific node content,
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

The default promotion is intentionally fast: it checks finished Simplified Chinese translations,
builds the packaged desktop app, and runs the two packaged desktop smoke paths before advancing
`current-debug`. Use `./scripts/build_and_promote_debug.sh --full` before commits, cross-module
handoffs, or release-like checkpoints to also run Rust formatting, workspace Clippy, and all Rust
tests. Promotion copies a complete candidate bundle into an immutable external release and atomically
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

`SpeechScriptPanel.qml` presents the full accepted-source parse, binds role voices and resolves sequential cues. `session/speech_script.rs` owns project-backed authoring and full-source preview; the backend imports bounded cue files on a worker and commits their immutable content reference on the UI session. The portable grammar belongs to `shape-domain::speech_script`; PCM assembly and validation belong to `shape-execution::speech_script`. Both core proposal and durable store acceptance validate the same source-bound timeline. See [script rules](../../docs/SPEECH_SCRIPT.md).

`SpeechScriptHelpDialog.qml` exposes the packaged script rules and AI writing prompt from the speech workspace, including plain-text mode. `SpeechScriptDocumentation` reads the canonical `docs/SPEECH_SCRIPT*.md` resources and copies their exact text; no source checkout or network access is required at runtime.

The New Content dialog exposes Free Writing and Production Script before requesting text. A listening
exercise or dialogue is described in the writer's request and uses the same script format; both entries open
`TextAuthoringWorkspace.qml` with an empty persisted draft.
AI drafting, adapting reference text and manual writing share explicit candidate acceptance. The
writing workspace offers the rules before content entry, full-text reading/source review and a direct
handoff to voices with script parsing enabled. Returning to writing retains the format, draft requirements and voice
settings; a new accepted text revision invalidates transient speech previews without changing saved
recordings. `SpeechScriptReadingView.qml` renders Rust's parser projection rather than parsing QML text.

`operator_catalog/text_authoring.rs` owns the versioned authoring configuration and compiles the
bundled rules into existing Infer text requests. `session/text_authoring.rs` owns empty-source creation,
buffer persistence, complete-text retrieval, manual proposals and speech handoff. Its source transition
uses an unaccepted artifact plus zero-input graph; the first explicit acceptance creates an editable
accepted-input graph. No placeholder text revision is stored. Native text-authoring smoke covers the
packaged entry, pre-writing help, invalid grammar, manual acceptance and return to writing.
`--smoke-text-authoring DIRECTORY` separately exercises real AI drafting, valid script adoption,
Qwen speech and byte-verified WAV export in an isolated project; it requires configured Infer access.

`ShapeButton`, `ShapeTextField`, `ShapeTextArea`, `ShapeComboBox` and `ShapeDialog` own shared theme,
focus, padding and popup treatment. Loading indicators stay inside the existing button geometry.

### Production scripts

Script format `20260922.2` declares production purpose, role identities, delivery and sound cues before
playback. The source script owns roles, pauses, cues, delivery and repeats for both AI drafts and
handwritten text. Legacy drafts that stored listening or dialogue as a profile or example are read as
the script format and their content intent is moved into the written request. Narration also maps to
the script format. The bridge checks the same grammar for preview and adoption. `ScriptPresentation.js` owns
shared localized diagnostics and control labels.

`shape-domain::speech_script::production` owns portable delivery and declaration values. The parser
resolves scoped language/delivery, validates declared names and bounded repeat blocks, and preserves
control events in the document plan. `SpeechScriptReadingView` shows the controls without duplicating
the text. `SpeechScriptPanel` requires every named role to have an explicit voice, shows shared-voice
warnings, and uses declared built-in cues or user-bound WAV resources. The help dialog includes visual
examples, the copyable AI prompt and the full offline specification.

The execution compiler emits one speech action per unique utterance plus replay references. Assembly
copies prior PCM ranges in the final bounded WAV buffer; receipt validation rejects changed replay
bytes even when their digest is recomputed. Global/role/local delivery is sent through the existing
Infer speech `instructions` field and Qwen worker `instruct` parameter. Repetition guarantees exact
bytes within the recording; an optional declared cue plays only between repetitions, before the
configured silent gap. New synthesis of different sentences still requires listening review.

## Bounded forms and common node templates

`ShapeTextEditor.qml` owns a fixed text viewport, keyboard editing and visible overflow scrolling;
`ShapeTextArea.qml` remains the primitive for content-sized documents inside existing scroll views.
Writing scrolls between bounded fields, and speech source/settings columns scroll independently.
`ShapeButton.qml` keeps action width tied to its label, including while its internal busy indicator runs.

The Rust node catalog exposes six text-edit task templates and the zero-input image source.
Templates reuse `text.edit` input/output/CAS semantics and store their task in authoring configuration.
The image consumer pins the selected, allowlisted deployment (`codex_gpt_5_6_luna` by default), compiles the preferred canvas into its model instructions,
and checks the bounded PNG and named route. Native pixels and actual dimensions are retained;
exact sizing belongs to an image editing node. No model fallback is requested. Text editing defaults to local Qwen; GPT-6 Luna/Sol are explicit cloud subscription choices. Settings hold model and effort defaults for text and image; `AiModelPicker.qml` presents both controls on one row and allows per-run overrides without changing settings or the project draft. Each request snapshots both choices. The local text model uses its Runtime effort; cloud text defaults to low, while image generation defaults to Runtime effort. Settings and the picker limit `ultra` to Sol; the bridge and executor reject unsupported effort choices. Shape verifies the selected named deployment in the returned Job before creating a Candidate.
`--smoke-image-generation DIRECTORY` exercises the real controller, candidate preview, explicit
acceptance and project reopening; it is an opt-in live test, not part of ordinary desktop smoke.

The node library supports Chinese search, arrow-key selection and Enter. It shows human-readable
input/output types; text comparison and the offline script guide expose scrollbars when overflowing.
