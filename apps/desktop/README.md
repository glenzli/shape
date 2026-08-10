# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. It can open a real `.shape`
bundle at startup, display its validated project metadata and artifact list, and render the bounded
accepted text projection supplied by Rust. A text document can now be edited into one transient
candidate, compared beside its accepted revision, discarded, or explicitly accepted. Navigation,
draft, variants, compare, and semantic-history regions retain complete English and Simplified
Chinese message identities. Language can switch at runtime between system, English, and Simplified
Chinese. Appearance defaults to the system color scheme and can be pinned to light or dark; both
preferences persist across launches.

[`shape-desktop-bridge`](../../crates/shape-desktop-bridge/src/lib.rs) owns the generated CXX ABI.
Rust's bounded `DesktopSession` validates SQLite and content objects, owns the open project plus at
most one transient text candidate, and delegates acceptance to the existing atomic project use
case. C++ maps explicit snapshots and commands into Qt presentation values; QML never reads or
writes project files. Richer multi-candidate drafts, image buffers, Infer Runtime HTTP, Shadow/Echo
interop, and third-party editor process control remain behind future capability adapters.

`UiPreferences` is the process-level owner for appearance, effective system color scheme,
translation lifecycle, and persistence. `MainTitleBar.qml` owns the fused toolbar/title region.
The shared shell uses Qt's expanded client area and safe-area margins on every platform; the small
Objective-C++ adapter only aligns native macOS traffic-light buttons with that shared toolbar.
`DesktopBackend` is a presentation facade over the Rust session; project and candidate authority
never enters QML or UI settings. `TextCompareWorkspace.qml` owns the side-by-side text comparison
without taking persistence responsibility.

Build and smoke-start:

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev
```

After creating a demo through `shape-cli`, launch the platform executable with
`--project /path/to/project.shape`. The `shape-desktop-project-smoke` test performs this entire
create → link → open → propose candidate → verify accepted head is unchanged → accept → reopen path
automatically.

The shell requires Qt 6.9 or newer for the cross-platform expanded client area. Platform-specific
code is isolated to native window-control alignment; QML layout, themes, and settings are shared.
