# Shape Desktop

This directory owns Shape's Qt 6/QML host for writing, image generation, sound generation and
material editing. The creation chooser starts ordinary writing, generates an image or sound,
or imports an existing material. Shape keeps source material, pending Candidates and immutable
accepted versions distinct. Running an operation produces a Candidate; using a result explicitly
advances the accepted history.

## Production owners

- `DesktopBackend` is the terminal CXX adapter and projects stable artifact, draft and Candidate
  identities. Rust remains authoritative for persistence, compare-and-swap and exact material bytes.
- `WorkspaceSurface.qml` routes the graph, media-specific workspace and read-only specialist
  compatibility page. `OperatorWorkspaceHost.qml` assembles the supported general media tools.
- `TextAuthoringWorkspace.qml` owns recoverable ordinary writing and source-bound text editing.
  New writing is plain text. Opening an older document preserves its persisted format, unknown
  settings and exact text; it does not convert a saved speech script to plain text.
- `InferTextController`, `InferAgentController`, `InferImageController` and `InferSoundController`
  execute the actual supported Infer operations. Requests retain their originating bundle and
  exact node/configuration identity; late results cannot enter another copied project.
- `MaterialEditController` owns separate image/audio working copies, strict Shadow/Echo
  request/result verification and explicit review. Returning a file does not automatically adopt it.
- `LegacyWorkController` owns read-only card/book/speech snapshot selection, exact material links,
  specialist export and concrete Storybook/Vox launch adapters. It has no authoring or renderer.
- `DeckHandoffController` owns a selected card snapshot's verified Deck request/result lifecycle.
  `DeckMaterialTaskController` retains the independent Deck-to-Shape image-generation workflow.

The embedded CardKit, PictureBook, card PDF layout and speech-production workspaces/controllers
have been removed from the executable, QML module and native test registrations. Deck owns card
production and print PDFs; Storybook Co Editor owns book production; Vox owns voice production.
No generic plugin framework or shared live database connects these applications.

## Existing projects and specialist handoffs

Opening a project does not initialize or rewrite old speech defaults. Existing card/book artifacts
and saved drafts open a lightweight compatibility page in the main workspace. It shows a chosen
accepted version or saved unfinished draft, readable contents and exact source-material links.
Unfinished content remains explicitly unfinished.

**Cards:** Continue in Deck sends the selected saved snapshot. An accepted snapshot binds the exact
accepted revision and does not create a durable draft or include later unaccepted edits. Continuing
an already linked Deck project is a separate, explicitly labelled action. Deck receives its own
project and material identities. Returned PDF/ZIP files are verified local output links, never
mislabelled image/audio artifacts. Missing receipts and unsupported app versions end in a visible,
retryable error. The original Shape project is unchanged.

**Books:** Export for Storybook writes a new editable `.scproj` outside the Shape bundle. Accepted
versions and saved unfinished drafts use different exact selectors. Draft exports preserve the raw
script and complete saved state in `shape-draft.json`, with an unaccepted status and review warnings.
Storybook can be opened separately; no unsupported command-line import is claimed.

**Voice:** Continue in Vox is available from ordinary writing, current accepted text material and
historical speech work. The primary action selects one new `.vox` destination. Shape prepares the
transfer in its own settings directory, checks the configured Vox companion CLI supports migration,
and launches Vox with the exact manifest hash. Launching is reported as launching; Vox verifies the
transfer before creating the receiving project. A secondary action exports transfer files only.
The transfer keeps accepted text, saved authoring draft, voice/cue settings and historical audio
separate. Existing audio retains its original script and provenance. Source and destination bundles
are never shared or overwritten.

Every delayed export callback binds the canonical bundle, project, artifact and selected source or
draft digest. Changing that selection rejects a stale callback before publishing output. App choices
preserve the user's absolute alias path; each launch resolves the current executable. Storybook and
Vox can be selected in the compatibility page, or configured for testing with
`SHAPE_STORYBOOK_BINARY` / `SHAPE_VOX_BINARY`.

## General material paths

Import accepts bounded PNG/JPEG images, UTF-8 text/code and PCM S16 LE WAV. Paste accepts clipboard
text or images. Imported material becomes an immutable Source; later operations still create
reviewable Candidates. Text/code preserves exact UTF-8 bytes and its format. The offline HTML
preview uses an ephemeral Qt WebEngine profile with external network, file access, permissions and
storage disabled. Other code is never executed. Raster operations retain crop, resize, orientation,
blur and flattened-shadow contracts. Audio remains playable and exportable as exact WAV.

The graph is an optional inspection/editing route for accepted lineage and saved processing intent.
Selection uses stable IDs. A focused media workspace preserves its own input, parameters, preview
and result shelf; accepted cross-artifact output retains its source relationship. General audio
creation remains sound effects/music generation, not a hidden speech-production surface.

## Build and validation

Use the repository presets and an external build directory. The native executable links the one
Rust desktop bridge; generated CXX headers are selected from the active Cargo build, not an old
output directory. English message identities have finished Simplified Chinese translations.
Language and appearance can change at runtime.

The packaged native gate has three registered checks:

- `shape-desktop-smoke`: empty launch, creation/navigation and common workspace contracts.
- `shape-desktop-project-smoke`: real project, text/raster Candidate-to-explicit-Accept, material
  editing/audio preview and reopen.
- `shape-desktop-convergence-smoke`: read-only legacy snapshots, selected book export, rejected late
  callbacks, ordinary writing-to-Vox and existing Deck receipts/standalone-image-task lifecycle.

The convergence wrapper uses the adjacent Rust **test-only** legacy factory. In a clean checkout
it generates an independent temporary mixed project using
`full_legacy_transfer_preserves_unaccepted_text_voices_cues_and_historical_audio_source`.
An already generated fixture can be supplied with `SHAPE_COMPAT_FIXTURE_BUNDLE` to reuse matching
proof without recompiling the same test. No production authoring backdoor or media fixture library
is included. All project bundles, generated media, evidence and Cargo/CMake outputs stay outside
the worktree. Native tests must run against the newly linked executable.

`--smoke-convergence OUTPUT` consumes a project supplied with `--project`. Default checks use
isolated settings and fake app lifecycle executables; setting `SHAPE_VOX_BINARY` opts into the real
installed Vox import consumer. System file-dialog admission is tested through real pointer actions;
completion and stale-selection fences are tested through the production controller methods. The
suite does not claim to automate the operating system's file-picker interaction.

Live image/sound and `--smoke-deck-material-live` are separate explicit opt-ins. Ordinary native
checks do not read real Infer credentials or make model calls. Release validation reuses only
matching source/build evidence; the root release steward owns canonical promotion.
