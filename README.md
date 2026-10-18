# Shape

Shape is a cross-platform creative workspace for making and transforming materials with AI.
Keep the source, compare candidates, and explicitly accept the version you want to keep.

Shape owns reusable text, image and sound operations, their exact inputs, and their revision history.
Specialist production belongs to independent applications:

| Application | Responsibility |
| --- | --- |
| [Deck](https://github.com/glenzli/deck) | Card authoring, layout, quantities and printing with crop marks |
| [Storybook Co-Editor](https://github.com/glenzli/storybook-co-editor) | Book scripts, pages, typography and publishing |
| [Vox](https://github.com/glenzli/vox) | Script-led voice production, speaker direction and takes |
| [Shadow](https://github.com/glenzli/shadow) | Specialist image editing |
| [Echo](https://github.com/glenzli/echo) | Specialist audio editing and mixing |

See [application boundaries](docs/NATURAL_WORKFLOW.md) for the product and compatibility policy.

## Creative work

- Write manually or use AI to create, rewrite, translate and derive text from an exact source.
- Continue a selected text or saved writing draft in Vox when it is ready for voice production.
- Generate images, compare alternatives, and apply bounded crop, resize, orientation and effects.
- Generate sound materials, audition saved or candidate audio, and export selected verified WAV bytes.
- Import PNG/JPEG, UTF-8 text/code and supported PCM WAV files as source materials.
- Send a material copy to Shadow or Echo; review the return before accepting a derived version.
- Use the graph to inspect or combine processing steps. Each operation retains its input identity;
  changing selection or opening another window does not change a running task's target.

Candidates are transient. Only explicit acceptance advances an immutable artifact revision.
Unavailable models or external applications do not prevent reading existing accepted materials.
Self-contained HTML can be previewed in a bounded offline surface; other imported code remains text.

## Existing specialist work

Older `.shape` projects may contain cards, picture books, speech drafts and accepted recordings.
They remain readable and exportable through a compatibility surface. Shape no longer offers their
specialist authoring, page-layout, card-printing or voice-production workspaces.

- Continue a saved card snapshot in Deck with its exact materials and stable card identities.
- Export a selected book snapshot for Storybook Co-Editor while retaining unfinished source work.
- Transfer a script and its saved voice settings, cues and optional accepted audio to an independent
  Vox project. Accepted history and unfinished drafts remain distinct.

The original Shape project remains intact. Independent applications own their new projects and
databases. A successful launch is separate from a verified output; returning a material still requires
review. See [Deck handoff](docs/DECK_HANDOFF.md), [legacy composition contracts](docs/CREATIVE_TEMPLATES.md)
and [material editing](docs/MATERIAL_EDIT.md).

## Repository map

| Area | Entry | Responsibility |
| --- | --- | --- |
| Product design | [SHAPE_PLAN.md](SHAPE_PLAN.md) | Product boundaries and staged design; historical milestones are not current capability claims |
| Engineering orientation | [DEV_SKELETON.md](DEV_SKELETON.md) | Ownership, invariants and navigation |
| Creative contracts | [shape-domain](crates/shape-domain/src/lib.rs) | Artifact/Scene identities, media contracts and immutable revision semantics |
| Execution | [shape-execution](crates/shape-execution/src/lib.rs) | Executor lifecycle, Infer adapters, raster/audio validation and provenance |
| Persistence | [shape-store](crates/shape-store/src/lib.rs) | `.shape` bundles, SQLite, BLAKE3 objects and atomic accepted-head updates |
| Use cases | [shape-core](crates/shape-core/src/lib.rs) | General creative operations and bounded legacy read/export compatibility |
| Desktop bridge | [shape-desktop-bridge](crates/shape-desktop-bridge/src/lib.rs) | Bounded CXX session projections and shared use cases |
| CLI | [shape-cli](apps/shape-cli/README.md) | Project inspection, exact material reads and machine-readable actions |
| Desktop | [apps/desktop](apps/desktop/README.md) | Qt/QML workspace, material review and specialist application handoffs |
| Checks | [xtask](xtask/src/main.rs) | Formatting, lint, tests and prerequisite checks |

## Run locally

```sh
cargo xtask check
cargo run --package shape-cli -- demo /tmp/shape-foundation.shape
cargo run --package shape-cli -- inspect /tmp/shape-foundation.shape
cargo run --package shape-cli -- project-status /tmp/shape-foundation.shape

cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev

./scripts/build_and_promote_debug.sh --full
./scripts/run_debug.sh [/path/to/project.shape]
```

Build products stay in sibling build directories. The debug promotion script validates the packaged
application, copies it into an immutable release and atomically advances
`.shape-local-build/current-debug`. `--full` also runs Rust formatting, workspace Clippy and tests;
use it before commits and cross-module checkpoints. `run_debug.sh --check` prints the current paths
without launching the application.

The CLI and desktop use the same business and acceptance boundaries. Public JSON actions and their
limits are documented beside the owning implementation. A historical schema or parser remaining in
the repository does not make the corresponding specialist production workflow a Shape feature.

## License

Shape is licensed under the [MIT License](LICENSE). External editors run through independent process
and material contracts; their databases, implementation and license obligations remain with them.
