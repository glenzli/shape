# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. It can open a real `.shape`
bundle at startup, display its validated project metadata and artifact list, and render the bounded
accepted text projection supplied by Rust. Navigation, intent, variants, and semantic-history
regions retain complete English and Simplified Chinese message identities.

[`shape-desktop-bridge`](../../crates/shape-desktop-bridge/src/lib.rs) owns the generated CXX ABI.
Rust validates SQLite and content objects; C++ maps one explicit snapshot into Qt presentation
values; QML never reads project files. The bridge is currently read-only. Image buffers, mutation,
Infer Runtime HTTP, Shadow/Echo interop, and third-party editor process control remain behind future
capability adapters.

Build and smoke-start:

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev
```

After creating a demo through `shape-cli`, launch the platform executable with
`--project /path/to/project.shape`. The `shape-desktop-project-smoke` test performs this entire
create → link → open → QML-load path automatically.

The shell uses only public Qt Quick APIs and contains no macOS-only UI or filesystem assumptions.
