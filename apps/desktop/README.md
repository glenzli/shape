# Shape Desktop

This directory owns the cross-platform Qt 6/QML application assembly. The current foundation is a
real, buildable workspace shell that establishes navigation, intent, canvas, variants, and semantic
history regions with complete English and Simplified Chinese message identities.

It deliberately has no fabricated backend bridge. The runnable Rust consumer is `shape-cli`; a
narrow Qt/Rust bridge will be introduced when the desktop opens and accepts a real project through
`shape-core`. Image buffers, Infer Runtime HTTP, Shadow/Echo interop, and third-party editor process
control remain behind future capability adapters.

Build and smoke-start:

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev
```

The shell uses only public Qt Quick APIs and contains no macOS-only UI or filesystem assumptions.
