# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. An empty launch presents
explicit start-new and continue-project actions. Once a project is open, the product direction is a
node-first Scene canvas: adding an editing node does not depend on node selection, typed output
ports expose a `+` insertion affordance, and focused work happens only after the user opens a node.
The current artifact-as-Scene implementation remains a compatibility projection while a real
Scene-owned mutable graph is added; it must not be mistaken for the final multi-source persistence
model.
The creation chooser also offers task templates for picture books, cards and image
refinement. Each template opens its concrete workspace; the graph and single-purpose tools remain
available. `CardKitController` owns recoverable card drafting, exact review identity and accepted
export through the Rust card-kit action contract. `CardKitWorkspace.qml` owns the shared brief and
style, stable card selection, text fields, accepted-artwork choice and material return routing.
New card sets use automatic layout: an image, text, or both are enough. Each card can override the
set with a complete-image, text, image-and-text or game-card preset. Titles and game fields are
optional. `CardLayoutOptions.qml` owns these inherited controls, color selection and its exact
project/bundle/kit/card callback fence. Switching layouts keeps all text and material pins; the
workspace names content hidden by the selected layout. Removing an image only detaches its binding.
Set proportions support portrait, landscape, square and custom width-to-height input without a free
canvas. Old sets without a layout keep their original appearance and readiness rules until the
author explicitly chooses a flexible layout.
Draft edits save in the project after a short pause and before closing or opening a material tool;
a failed save keeps the current text in the workspace. Artwork generation and Shadow refinement
produce reviewable versions, and explicit adoption updates the requested card by stable id even
after reordering. Export writes an accepted card-material ZIP with SVG cards and exact source
materials; it is an inspectable kit rather than a print-ready or gameplay-certified deck. See
[`CARD_KIT.md`](../../docs/CARD_KIT.md) for persistence, review and export bounds.
The local card controller records the project, kit, stable card id, original artwork pin and
external-edit stage in the same isolated desktop settings as Shadow's handoff observer. Reopening
validates the card source, current original head and persisted stage before resuming that observer;
it reuses the working copy and still requires explicit adoption. The card preview can also open at
its full canvas size for scrollable inspection.
The card list intake previews spreadsheet rows before appending them. It accepts optional titles,
body text and image-description columns, recognized two/three-column headings and quoted multiline
cells. A single column always keeps every row as a card unless the author explicitly marks it as a
heading. A body-only row is valid; an image prompt alone is not printable content. It preserves
existing cards, keeps repeated titles as separate stable identities, rejects malformed/oversized
lists and enforces the 64-card and composition-size bounds before mutation. Only the sole completely
untouched starter card is replaced by incoming rows; authored or additional cards remain intact.
Cancelling the intake
does not change the card draft.
`CardImageIntakeDialog.qml` owns native multiple-file selection, the pending confirmation list and
per-image progress/cancellation/retry. Each successfully imported image is saved as a stable complete
card without printing its file name. The loop yields between files, rechecks project/bundle/kit
identity before each import, keeps a successful source for draft-save retries, and inserts a retried
card at its original batch position. Cancellation keeps completed cards and leaves the rest pending;
an untouched starter can be replaced. Images of different proportions are contained without cropping.
The first added image can set an empty set's proportions; unsupported extreme ratios keep the current
canvas with an explicit explanation. A legacy set must be explicitly converted or replaced before
this intake starts.
`CardPrintController` owns the separate accepted-version print session and rejects delayed export
callbacks after its project, source or layout changes. The Rust `print_plan` action owns all paper,
card, quantity and crop-mark geometry in millimetres. `card_print_export` consumes that plan through
one painter for both sheet previews and vector PDF output; each stable card's SVG is stored once,
and only one decoded SVG renderer stays resident while consecutive copies reuse it. A4 and US
Letter layouts support custom card size, margins, gaps and crop marks. PDF publication uses a
temporary sibling file and an atomic no-clobber hard link; destinations inside any `.shape` bundle,
including through directory symlinks, are rejected. Print PDFs at **100% / Actual size**, with Fit
to page disabled. This single-sided layout does not submit a job to a physical printer.
Print starts with the core's normalized defaults for the set's proportions, and changing one
measurement can preserve that ratio. The explicit **Accept and arrange for print** action reviews
and accepts the current card version before opening print; already accepted current content opens
directly. A later print-layout failure never reports that a successful acceptance was undone.
AI/Shadow returns still require their own explicit adoption before they become card content.
Card image-import dialogs also capture the kit, stable card, project id and bundle path before
opening; a stale callback creates no material. ZIP dialogs retain the exact accepted kit pin.
The text and speech workflow uses the [node contract](../../docs/NODE_MODEL.md).
Production notes remain visible as non-spoken callouts during script review and voice audition,
so pronunciation and timing reminders travel with the accepted script without entering speech.
A failed audio selection clears the previous preview immediately, so playback cannot silently use
another version. Speech retry guidance distinguishes same-session recovery from a fresh synthesis
after text or settings change.
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
`OperatorDrafts` mirrors the project-backed, typed Working Graph entries that precede execution.
The Rust store compares both the accepted head and the exact previously observed graph in one
write transaction before saving or discarding. A stale parallel session cannot overwrite another
editor's draft, even before the first acceptance. Reopen the project after a draft conflict to
load the current authored state. Acceptance rebases the stored graph atomically; cached desktop
state never writes over that graph as a follow-up cleanup.
In-place acceptance and new-artifact branching still delegate to atomic project use cases. C++ maps
explicit snapshots and commands into Qt presentation values; QML never reads or writes project
files. Ordinary snapshots carry raster/audio metadata rather than encoded payloads; selected
accepted and candidate PNG/WAV bytes are fetched separately. Images enter a byte-bounded,
display-scaled native preview cache, while only one selected WAV enters the playback owner. Pinned
or durable explorations, image composites and layers, model-backed image editing, waveform editing,
recording and arbitrary third-party editor integrations remain behind future capability adapters.
The bounded Shadow/Echo file handoff below pins the input revision and validates returned bytes
as a Candidate. It does not attach to a provider GUI or mutate the original material.

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
never enters QML or UI settings. `InferAgentController` runs a bounded Agent file task from an
accepted text/code Source on a worker thread. `AgentFileTaskWorkspace.qml` lets the user write a
task and review the complete returned file beside its source. The result joins the transient
Candidate Shelf; Accept creates a separate text Artifact tied to the exact source Revision. The
project does not hand Infer a host file path. `ProjectWelcome.qml`, `CreateProjectDialog.qml`,
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
builds the packaged desktop app, and runs the five packaged desktop smoke paths before advancing
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
`ShapeMessageDialog` presents application notices and export results with those same components,
a localized, content-sized close button, bounded plain-text content, and Enter/Escape dismissal.
Native file and folder pickers retain their platform behavior.

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

### Local sound and music

The zero-input `audio.generate` source opens `SoundGenerationWorkspace.qml`. The draft owns
original prompt, SFX/music choice, 1–30 second duration and explicit u32 seed. Generate snapshots
all displayed fields before dispatch; `InferSoundController` owns asynchronous execution and
cancellation, while the bridge reopens the checkpointed draft and rejects stale adoption.
Chinese/mixed descriptions use the explicit versioned SDK preparation helper once per unchanged
prompt in the controller session. Preparation is local Qwen 3.5 4B; failures stop the chain.
English stays byte-exact. Retries reuse the verified preparation; a changed original invalidates it.
Only `model_choice` selects Stable Audio 3 Small SFX/Music; deployment metadata is forbidden by the
sound contract. Job readback binds the selected physical model, exact seed/duration and local-only,
offline, no-fallback, zero-cost policy to verified 44.1 kHz stereo PCM16 WAV bytes.

A sound remains a transient Candidate until explicit acceptance. Playback, exact WAV export,
switching and deletion use the existing audio and Candidate owners. Acceptance validates the
original/effective prompt record and authored-request digest again, then stores immutable audio and
provenance. Generation details remain inspectable after reopening. Stop and controller destruction
drop waiting futures and prevent adoption; they do not claim to terminate an already running provider.
The opt-in `--smoke-sound-live ABS_OUTPUT_DIRECTORY` path checks real Chinese SFX, mixed Chinese
instruments, English bypass, numeric field snapshots, failure/stop, reuse, candidate switching/deletion,
playback, exact export, acceptance and reopen using the production controller and packaged workspace.


Desktop smoke modes use a temporary settings file for appearance, language, model defaults and
recent projects, including the opt-in live sound path. Ordinary offscreen smoke also uses an
empty temporary credential location; opt-in live tests continue to read the configured credential.
These checks never rewrite the user's preferences or add synthetic projects to their recent list.

Text authoring keeps the adoption action disabled when its pinned original becomes stale. The
action footer explains the required input refresh; choosing the latest original preserves the
authored branch text. Native smoke covers a real intervening accepted revision, explicit refresh,
and separate-output adoption without overwriting that newer original.

Input refresh checkpoints local authored changes first and returns to the writing request for
review. The stale-source card explains that old candidates are cleared while draft text and
accepted outputs stay. A refresh during generation is disabled. Repeating the same-source refresh
at the bridge boundary preserves the current candidate instead of treating a duplicate action as
discard; a real source version change still invalidates its obsolete candidates.


Speech audition projects the canonical `SpeechScriptPlan::readiness` diagnostics used by the CLI
binding check. The first missing role voice or unresolved cue is named with its first used line;
syntax repair remains in writing. Used repeat-between cues are included, unused declarations do
not block, and explicit skip/built-in choices resolve without pretending audio has been generated.
Portable prerequisites are separate from Runtime access, actual voice availability, listening and
explicit acceptance. The guide remains a general production-script format, not segment rerecording.


A narration whose source advanced offers explicit input refresh even before its first recording.
Source review reads the exact pinned revision, and a changed input revision reloads the script plan
even when voice/cue settings are identical. When the source remains a script, refresh preserves
those authored choices, invalidates old candidates, and shows any newly required role or cue.
Changing to plain text clears script-only options. Pending pace edits settle before refresh;
generation and cue-import activity disable it. Accepted recordings remain immutable and available.

Narration operation feedback is returned by `DesktopBackend::speechDraftOperationFinished` with
its project path and draft identity. The audition workspace displays only its own operation result;
unrelated successes cannot erase a failed cue import or draft save. Recovery uses the existing
Working Graph conflict checks, and cue constraints are visible before file selection.
`InferSpeechController` retains the admitted project/draft identity through terminal status. QML
scopes progress and errors to that identity, explains a busy executor in other works, and uses
`cancelFor` so a stale control from another narration cannot stop the owner. This preserves the
single active executor and the existing transient retry state; no durable task model is added.

`tests/speech_feedback_smoke.cpp` runs within `--smoke-text-cycle`. It uses two synthetic works,
a real invalid/valid WAV import, a second desktop session for conflicting save/refresh operations,
and the real speech admission signal before its worker starts. The offline credential remains
absent, so the worker terminates before any Infer request; these checks do not claim live synthesis
or nonzero generated-segment progress. They verify cross-work status presentation and stop authority
through the packaged QML controls without controlling the visible desktop.
The output-name editor stays bound to the selected output when the same QML workspace is reused;
the two-work fixture checks both the displayed name and the editing-finished save target.


### External material stages

Accepted images/audio can create a separate Shadow/Echo editing stage from the
operator palette. `MaterialEditWorkspace` presents the pinned original, working-copy
handoff, return review and explicit cancellation/refresh. `MaterialEditController`
performs file export/normalization off the GUI thread, scopes completion to the
originating project/output/session, and leaves adoption to the existing candidate
shelf. The PCM24 precision-conversion checkbox is off by default and resets when
the stage/input changes. See [material-edit contracts](../../docs/MATERIAL_EDIT.md).

`material_edit_smoke` runs inside the packaged `--smoke-text-cycle` path using
synthetic pixels/WAV and isolated settings. Build fresh before running it. It checks
first-source preview, return review, invalid replacement, A/B request ownership,
late completion, cancel/retry, adoption and reopen. The audio workspace test uses its real review
button to reject PCM24 by default, enable conversion, decode both auditions and adopt the return.
The work rail preserves image/audio material names and reserves “Text and audio” for actual mixed
works. It does not operate a visible
Shadow/Echo GUI or make a performance claim.

The CXX header synchronizer uses this Cargo invocation's `build-script-executed` output directory.
Historical dependency fingerprints are valid cache entries and are never used to select headers.
Missing or ambiguous current output fails before replacing the stable include files. Run
`python3 apps/desktop/cmake/tests/test_sync_cxxbridge_headers.py` for its stale-cache regression.

`ShapeIcon` keeps the accelerated effect path and uses a small `Canvas.Image` tint only
when `GraphicsInfo` reports the software backend. This avoids invisible action icons
in offscreen/fallback rendering while preserving theme colors; no provider or media
rendering contract changes. See [Qt Canvas backends](https://doc.qt.io/qt-6/qml-qtquick-canvas.html).

## Picture books

The project rail's **Picture books / 绘本** opens a versioned composition workspace.
`PictureBookController` connects the shared candidate shelf to page/script editing;
`picture_book_renderer` draws both previews and reading PDFs. Pending script text
blocks language/text-source replacement and material refresh until adoption or
explicit draft discard. A refresh with unchanged material pins preserves the
reviewed candidate and preview; an artwork-only update keeps the active script.
See
[the workflow and boundaries](../../docs/PICTURE_BOOK.md). The packaged
`shape-desktop-picture-book-smoke` drives this real controller/QML surface with
synthetic bilingual material and records rendered pages, PDFs and a screenshot.
Use `SHAPE_BUILD_JOBS=2 CARGO_BUILD_JOBS=2 scripts/build_and_promote_debug.sh --full`
when sharing the machine under a two-job build budget.

## Dedicated editors and Deck material tasks

New card sets open Deck; new picture books open the installed Storybook Co Editor without inventing
a document-import argument. Existing Shape CardKit/PictureBook history remains readable in its old
workspace. A legacy card set's **Continue in Deck** sends a saved, complete material snapshot to an
independent Deck project. `DeckHandoffController` owns protocol probing, exact bundle/request identity,
process/restart recovery and independently verified PDF/ZIP links. It never imports those files as
images or accepts another application's artifact ids.

`shape-desktop --handoff-capabilities` reports the supported file protocols without opening the GUI.
`shape-desktop --deck-material-task REQUEST.json --result NEW-RESULT.json` creates/reopens one isolated
image task, opens the existing AI image workspace with the prompt/canvas, and uses Shape's own Infer
identity. Candidates still require explicit acceptance; **Return to Deck** exports the exact accepted
PNG, while cancelling/closing writes a cancelled receipt. `DeckMaterialTaskController` owns this
separate task lifecycle. See [Deck handoff](../../docs/DECK_HANDOFF.md) for bounds and strict schemas.

The packaged `--smoke-deck-handoff DIRECTORY` path runs offline with isolated settings and produces
retained evidence for a real legacy/incomplete transfer from a mixed audio/image/text project,
process exit without a receipt, the real task workspace/target, explicit cancellation and no-clobber
results. Its process fixture tests lifecycle only; real Deck importer/GUI acceptance is separate
producer-to-consumer evidence. `--smoke-deck-material-cancel` is a test-only modifier for the actual
material-task entry and never generates with real credentials.
The separate opt-in `--smoke-deck-material-live` modifier runs the actual image consumer, explicitly
accepts its exact target Candidate, and clicks the real **Return to Deck** button; it intentionally
uses Shape's existing Infer credential and is excluded from offline CTest.

## Application icon

[Icon assets and provenance](assets/README.md) own the selected imagegen artwork, exact source
hashes and mechanical PNG/ICNS conversion commands. CMake embeds the PNG for Qt window branding
and copies the matching ICNS into the macOS bundle; no developer-local image path is used at runtime.
