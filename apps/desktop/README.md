# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. It can open a real `.shape`
bundle at startup, display its validated project metadata and artifact list, and render the bounded
accepted text projection supplied by Rust. A text document can now accumulate multiple transient
candidates, switch and compare them beside its accepted revision, discard an exact option,
explicitly accept one in place, or branch one into a newly named artifact. Navigation, draft,
Candidate Shelf, compare, and semantic-history
regions retain complete English and Simplified Chinese message identities. Language can switch at
runtime between system, English, and Simplified Chinese. Appearance defaults to the system color
scheme and can be pinned to light or dark; both preferences persist across launches.

The shell is organized around a project navigator, a media-specific central workspace, and a
context inspector with Explore, Details, and Lineage modes. Lineage is not placeholder UI: Rust
loads the accepted revision's persisted parent identities and creative Transformation, while C++
projects localized transformation labels and resolves transformation inputs to source artifact
names. Candidates remain in Explore and outside durable lineage until acceptance. Branching creates
the new artifact, its first revision, target-specific execution receipt, and cross-artifact input
edge in one atomic persistence transaction without advancing the source artifact.

The central workspace switches between the selected Artifact and a real current-head Project Graph.
Rust projects accepted cross-artifact derivation edges from persisted Transformation inputs; QML
lays out and draws those edges without becoming graph authority. Artifact selection is shared across
the navigator, graph, workspace, and inspector. Pending candidates appear as dashed ghost nodes
connected to their sources, but remain absent from the durable graph snapshot until explicit
acceptance or branching.

[`shape-desktop-bridge`](../../crates/shape-desktop-bridge/src/lib.rs) owns the generated CXX ABI.
Rust's bounded `DesktopSession` validates SQLite and content objects and owns the open project;
`CandidateShelf` owns its in-memory, newest-first candidate collection and exact-ID mutations.
In-place acceptance and new-artifact branching still delegate to atomic project use cases. C++ maps
explicit snapshots and commands into Qt presentation values; QML never reads or writes project
files. Pinned or durable explorations, image buffers, authenticated Infer Runtime execution,
Shadow/Echo interop, and third-party editor process control remain behind future capability
adapters.

The first Infer Runtime boundary is deliberately smaller than an executor. `InferRuntimeController`
runs a bounded public-contract probe away from the UI thread and projects only checking, reachable,
compatible, contract-version, endpoint-source, instance/generation, and stable-error state. Rust
selects endpoints in this order: canonical numeric-loopback `SHAPE_INFER_RUNTIME_URL` override,
live owner-only `infer-runtime.consumer@0.1.0-candidate.2` Infra Discovery offer, then the temporary
`http://127.0.0.1:8787` migration fallback. It validates the exact Discovery schema, filesystem
ownership and modes, lease, generation, offer binding, and raw endpoint before HTTP. Proxies,
redirects, hostnames, remote hosts, credentials, authenticated submission, and automatic Infer
configuration changes are excluded. An unavailable runtime is surfaced as a refreshable status in
the intent panel while direct editing and accepted project content remain usable.

`UiPreferences` is the process-level owner for appearance, effective system color scheme,
translation lifecycle, and persistence. `MainTitleBar.qml` owns the fused toolbar/title region.
The shared shell uses Qt's expanded client area and safe-area margins on every platform; the small
Objective-C++ adapter only aligns native macOS traffic-light buttons with that shared toolbar.
`DesktopBackend` is a presentation facade over the Rust session; project and candidate authority
never enters QML or UI settings. `TextCompareWorkspace.qml` owns the side-by-side text comparison
without taking persistence responsibility. `ProjectNavigator.qml` owns project-level selection,
`ContextInspector.qml` owns inspector navigation, `VariantsPanel.qml` owns artifact-scoped shelf
selection and review controls, and the details and lineage panels own their respective read-only
projections. `BranchArtifactDialog.qml` owns branch naming and submission;
`ProjectGraphWorkspace.qml` owns graph layout and node interaction, while `WorkspaceSurface.qml`
owns artifact/graph navigation. `Main.qml` remains an assembly root.

Build and smoke-start:

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev
```

After creating a demo through `shape-cli`, launch the platform executable with
`--project /path/to/project.shape`. The `shape-desktop-project-smoke` test performs the entire
create → link → open → propose → accept → propose → branch path automatically, including source
head preservation, the projected cross-artifact input edge, exact shelf selection/discard,
packaged graph activation, graph/candidate selection synchronization, and presence of the packaged
Infer Runtime status control. The runtime itself may remain offline during this smoke path.

The shell requires Qt 6.9 or newer for the cross-platform expanded client area. Platform-specific
code is isolated to native window-control alignment; QML layout, themes, and settings are shared.
