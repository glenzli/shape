# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. It can open a real `.shape`
bundle at startup, display its validated project metadata and artifact list, and render bounded
accepted text or an on-demand verified raster preview supplied by Rust. Text documents can
accumulate multiple transient candidates, compare and accept them in place, or branch one into a
new artifact. An 8-bit PNG/JPEG can be imported into a canonical `image.raster` revision, shaped
with a direct crop frame, compared against its transient crop candidate, accepted, and reopened.
Accepted text can also enter the preset-only Speech Synthesis workspace, produce a transient
`audio.clip` Candidate through Infer Runtime, play the selected exact WAV in memory, and accept it
as a new cross-artifact output without advancing the source text.
Navigation, draft, Candidate Shelf, compare, and semantic-history
regions retain complete English and Simplified Chinese message identities. Language can switch at
runtime between system, English, and Simplified Chinese. Appearance defaults to the system color
scheme and can be pinned to light or dark; both preferences persist across launches.

The shell is organized around a Scene Operator Graph home, media-specific focused workspaces, and
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
across the navigator, graph, workspace, and inspector. Pending candidates appear as dashed ghost
operators, but remain outside the durable accepted graph until explicit acceptance or branching.

This foundation currently adapts each existing Artifact into a single-output Scene. That
compatibility boundary lets media-specific Operator work proceed without pretending that the final
Scene, named multi-output, or reusable GraphComponent persistence model already exists.

[`shape-desktop-bridge`](../../crates/shape-desktop-bridge/src/lib.rs) owns the generated CXX ABI.
Rust's bounded `DesktopSession` validates SQLite and content objects and owns the open project;
`CandidateShelf` owns its in-memory, newest-first candidate collection and exact-ID mutations.
In-place acceptance and new-artifact branching still delegate to atomic project use cases. C++ maps
explicit snapshots and commands into Qt presentation values; QML never reads or writes project
files. Ordinary snapshots carry raster/audio metadata rather than encoded payloads; selected
accepted and candidate PNG/WAV bytes are fetched separately. Images enter a byte-bounded,
display-scaled native preview cache, while only one selected WAV enters the playback owner. Pinned
or durable explorations, image composites and layers, model-backed image editing, waveform editing,
recording, Shadow/Echo interop, and third-party editor process control remain behind future
capability adapters.

`InferRuntimeController` runs the bounded public-contract probe away from the UI thread and projects
only checking, reachable, compatible, contract-version, endpoint-source, instance/generation, and
stable-error state. Rust selects endpoints in this order: canonical numeric-loopback
`SHAPE_INFER_RUNTIME_URL` override, live owner-only
`infer-runtime.consumer@0.1.0-candidate.2` Infra Discovery offer, then the temporary
`http://127.0.0.1:8787` migration fallback. It validates the exact Discovery schema, filesystem
ownership and modes, lease, generation, offer binding, and raw endpoint before HTTP.

`InferTextController` separately owns one asynchronous authenticated generation lifecycle. Settings
can copy the one-time managed token for Infer App `shape` into
`AppConfigLocation/secrets/infer-runtime.token`; Rust atomically maintains its owner-only directory
and file and never puts the token in QSettings, a project, exported settings, arguments, environment,
or diagnostics. The request is fixed to `assistant.general`, local-first, local-only, offline,
no-fallback, and zero cloud cost. Proxies, redirects, hostnames, and remote origins are rejected.
The worker opens a read-only project view and returns an opaque generated candidate; the UI-thread
`DesktopSession` rechecks expected-head identity before adding it to the transient shelf. Only the
existing accept operation advances durable history. Infer App/ACL creation and daemon restart remain
explicit Infer Console operations rather than Shape configuration mutations.

`infer_runtime_access` owns the shared credential status/install boundary now consumed by both Text
and Speech. `InferSpeechController` owns its own asynchronous preset-synthesis lifecycle and hands
the move-only result to `DesktopSession`, which rechecks the accepted source head before adding the
new audio target to the shelf. Candidate projection distinguishes target Artifact identity from
the source Artifact that owns review context. `AudioPreviewController` separately owns one selected
WAV allocation, `QBuffer`, `QMediaPlayer`, position, seek, and terminal playback errors; ordinary
snapshots and QML never carry encoded audio. `AudioSpeechOperatorWorkspace.qml` owns voice-preset,
pace, naming, disclosure, generation, and audition presentation. This is a preview surface, not the
future Echo-backed composition/render engine.

`UiPreferences` is the process-level owner for appearance, effective system color scheme,
translation lifecycle, and persistence. `MainTitleBar.qml` owns the fused toolbar/title region.
The shared shell uses Qt's expanded client area and safe-area margins on every platform; the small
Objective-C++ adapter only aligns native macOS traffic-light buttons with that shared toolbar.
`DesktopBackend` is a presentation facade over the Rust session; project and candidate authority
never enters QML or UI settings. `TextOperatorWorkspace.qml` owns accepted text and Candidate
presentation for `text.edit` and `text.transform`; `TextCompareWorkspace.qml` remains its compare
owner. `RasterCropOperatorWorkspace.qml` separately owns crop-frame interaction, and
`ImageCompareWorkspace.qml` owns raster comparison without becoming a pixel or persistence
authority. `ProjectNavigator.qml` owns Scene selection,
`ContextInspector.qml` owns inspector navigation, `VariantsPanel.qml` owns artifact-scoped shelf
selection and review controls, and the details and lineage panels own their respective read-only
projections. `BranchArtifactDialog.qml` owns branch naming and submission;
`SceneOperatorGraphWorkspace.qml` owns typed graph layout, single-selection, and open/review intent,
while `OperatorWorkspaceHost.qml` owns exact Operator routes, Source/Output read-only routes,
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

The shell requires Qt 6.9 or newer for the cross-platform expanded client area. Platform-specific
code is isolated to native window-control alignment; QML layout, themes, and settings are shared.
