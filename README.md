# Shape

Shape is a cross-platform, AI-native creative environment for preserving what matters, changing
what is intended, exploring alternatives, and accepting durable artifact revisions.

The repository is in its foundation phase. Text, image, and preset-only speech slices prove the
interactive creative lifecycle, including creation of a typed audio Artifact with exact Runtime
provenance and selected-only local audition before acceptance.

## Repository map

| Area | Stable entry | Responsibility |
| --- | --- | --- |
| Product and staged design | [`SHAPE_PLAN.md`](SHAPE_PLAN.md) | Product philosophy, architecture target, external executors, roadmap, and completion gates |
| Engineering orientation | [`DEV_SKELETON.md`](DEV_SKELETON.md) | Stable implementation boundaries and navigation |
| Creative contracts | [`shape-domain`](crates/shape-domain/src/lib.rs) | Scene/Artifact identities, immutable revisions, typed Operator Graph, raster/audio contracts, transformations, constraints, and validation |
| Execution contracts | [`shape-execution`](crates/shape-execution/src/lib.rs) | Infer discovery/contract clients, executor lifecycle, bounded raster and preset speech adapters, outputs, and provenance receipts |
| Project persistence | [`shape-store`](crates/shape-store/src/lib.rs) | `.shape` bundle, SQLite ownership, durable BLAKE3 object store, and atomic Artifact/Scene accepted-head commits |
| Use cases | [`shape-core`](crates/shape-core/src/lib.rs) | Project and Scene creation, graph draft/accept, text/raster/speech Candidates, cross-artifact branching, and inspection |
| Desktop bridge | [`shape-desktop-bridge`](crates/shape-desktop-bridge/src/lib.rs) | Bounded CXX session for validated project snapshots, graph edges, on-demand raster/audio previews, and a cross-media Candidate Shelf |
| Foundation CLI | [`shape-cli`](apps/shape-cli/src/main.rs) | Real public consumer used for end-to-end smoke and inspection |
| Desktop shell | [`apps/desktop`](apps/desktop/README.md) | Cross-platform Qt/QML assembly that creates, opens, and edits real `.shape` projects |
| Repository checks | [`xtask`](xtask/src/main.rs) | Formatting, lint, tests, CMake configuration, and prerequisite checks |

The nearest code-owned module documentation is the navigation index. Planned modules in
`SHAPE_PLAN.md` are not implemented facts.

## Quick start

```sh
cargo xtask check

cargo run --package shape-cli -- demo /tmp/shape-foundation.shape
cargo run --package shape-cli -- inspect /tmp/shape-foundation.shape

cmake --preset native-dev
cmake --build --preset native-dev

cmake --preset desktop-dev
cmake --build --preset desktop-dev
ctest --preset desktop-dev

./scripts/build_and_promote_debug.sh
./scripts/run_debug.sh [/path/to/project.shape]
```

CMake output is written to the sibling `.shape-local-build` directory, not the source worktree.

`scripts/run_debug.sh` is the stable local launcher shared with the Shadow/Echo development
convention. `scripts/build_and_promote_debug.sh` defaults to a fast promotion gate: translation
completeness, desktop build, and the packaged desktop smoke paths. Use
`./scripts/build_and_promote_debug.sh --full` before commits, cross-module handoffs, or release-like
checkpoints to additionally run Rust formatting, workspace Clippy, and all Rust tests. Either mode
copies the app into an immutable revision-stamped release and atomically advances
`.shape-local-build/current-debug`. The launcher starts that canonical app in the background and
logs to `.shape-local-build/logs/shape-debug.log`; use `--foreground` for attached output or
`--check` to inspect resolved paths without launching. Pass a `.shape` bundle, or set
`SHAPE_DEBUG_PROJECT_PATH`, to open a project directly.

文本与配音的节点职责、输入输出和界面入口见 [节点与文本工作流](docs/NODE_MODEL.md)。
新建文本先选格式；编辑原稿会生成独立分支；配音自动读取已采用文稿的格式。

## Current boundary

- Implemented: pure Creative Document contracts, durable project store, executor lifecycle,
  separately revisioned Scenes with explicit named outputs and stale-draft-safe graph acceptance,
  deterministic text acceptance and new-artifact branching, plus an `image.raster` slice that
  safely imports bounded 8-bit PNG/JPEG sources into a canonical RGBA8 PNG contract. Raster crop
  follows the same Draft/Preview → Candidate → Compare → Accept → Reopen lifecycle without putting
  previews in durable history. Selected accepted/candidate image bytes are verified and loaded on
  demand into a bounded native preview cache; ordinary project snapshots carry only raster
  metadata. The cross-platform desktop can create or open a project, atomically create its first
  accepted Text Scene, and begin compatible project-backed Operator drafts from the graph. An
  unexecuted draft restores on reopen without entering immutable accepted history; execution turns
  it into a transient Candidate, and only explicit acceptance publishes durable history. The
  desktop keeps text and image candidates on one typed shelf while each medium owns
  its central workspace. It uses the official Infer Runtime SDK frozen at revision
  `8588a945047cedaea62035969e479e7fb7ff795c` for owner-only Infra Discovery,
  `infer-runtime.consumer-core@20260813.1`, the dated Capability Catalog, transport, credentials,
  and public errors. Shape imports its existing managed credential into an owner-only secret store
  and requests local-only `text.edit` through the ACL-authorized
  `ollama_qwen3_5_4b` Deployment as a transient candidate.
  The `audio.speech_synthesize` desktop workspace consumes immutable accepted text and creates a
  transient audio Candidate through nine versioned voice presets. Language defaults to automatic
  detection for mixed text and can be explicitly selected independently of the voice. Long text
  is split at sentence/word boundaries and assembled as one PCM WAV with per-segment receipts.
  The selected Candidate or accepted audio can be exported as WAV. A Candidate can be
  auditioned from an in-memory WAV device and becomes a new `audio.clip` only after explicit
  acceptance. Shape re-parses exact PCM S16 LE WAV bytes,
  requires preset voice/disclosure and local-only no-fallback Runtime provenance, and atomically
  rejects a stale text source. Schema `20260811.5` additively upgrades initial, Scene-era, audio-era,
  and Working-Graph-era projects without changing existing accepted heads. Mutable Operator state
  now uses bounded versioned configuration envelopes. `text.transform` restores an exact authored
  mode plus instruction, keeps rewrite/expand/polish/shorten inside one stable Operator identity,
  and reloads that project-backed draft before Infer execution. `audio.speech_synthesize` likewise
  persists its preset, language, pace, and disclosure contract; legacy unconfigured speech drafts
  receive the same validated preset default before the desktop session becomes available. Offline
  or unconfigured AI never affects direct editing.
- Deterministic Image Editing now includes Crop, Resize, orientation Transform, Blur, Drop Shadow,
  and Unsharp Mask. The concise [raster algorithm index](crates/shape-execution/src/raster/README.md)
  records their parameters, limits, and integration entry points. Every result remains a transient
  Candidate until explicit acceptance records its typed operation in immutable history.
- Shape will not duplicate mature photo-editing systems by default. Creative color grading and RAW
  work remain Shape-level semantic Operators, but their preferred future execution route is an
  explicit Shadow external-edit Adapter. Shape pins the input revision and color contract, accepts
  only materialized output plus a receipt as a Candidate, and advances history only after explicit
  acceptance. Shadow never writes the Shape project, and Shape never copies Shadow's internal Recipe.
- The desktop shell now follows the node-focused workbench model: a compact Scene/Component/Asset
  rail, a clickable graph-context strip above the active media workspace, and a horizontal Candidate
  filmstrip wired to exact Select/Compare/Accept/Discard/Branch actions. The generic Inspector is no
  longer a permanent third column. A separate comprehensive-Operator intent surface is packaged
  without placeholder state; its first real consumer is the zero-input AI Image Source Operator.
- New Content also exposes empty-first Free Writing and Narration Script workspaces. Listening,
  narration and dialogue templates support AI drafting, adapting existing material and manual writing.
  Script rules are included in AI requests and available before typing. Reviewed text is explicitly
  adopted before entering voice selection, audition and WAV export; rewriting preserves saved audio.
- AI image semantics are split deliberately. `image.generate` is a zero-material Source Operator;
  `image.generate_from_materials` requires accepted raster materials with explicit roles. Both have
  one logical output and place multiple generated options on the Candidate Shelf. The typed domain
  contract exists now. The source-less form uses the stable Core plus
  `infer.responses@20260812.1` SDK client and a Core Candidate/explicit-Accept path: PNG payloads
  and typed cloud Job provenance are revalidated,
  and no accepted revision is written before user acceptance. The desktop can atomically create a
  source-less AI Image Scene, restore its exact prompt/canvas draft, and open its dedicated canvas
  plus Intent sidebar. Live execution remains fail-closed under the current Shape App ACL until
  Infer explicitly grants the required subscription/balanced/cloud-only authority.
  Material-conditioned `image.edit` execution stays unavailable until Infer publishes a stable
  typed raster-output provider, Capability Schema, and SDK client; text output or a mock route is
  not substituted.
- Deferred: desktop editing of the new persistent Scene graph, GraphComponent instances, Infer-side
  App provisioning, operator-facing Job/explain inspection, pinned or durable explorations,
  composite/layer image structure, waveform editing, recording, Voice Reference execution,
  `audio.generate`/`audio.transform`, Shadow/Echo suite adapters and external-edit sessions,
  project imports/exports and material-conditioned image execution.
  The required Infer speech alias/ACL is locally
  proven but remains an externally unpublished dependency until Infer commits it.
- License: Shape source is licensed under the [`MIT License`](LICENSE). Shadow's GPL components
  remain behind independent-process or documented protocol boundaries so Shape's own distribution
  does not silently inherit a different license obligation.

### Narration scripts / 配音脚本

The text workspace authors production scripts with declared roles, delivery, scenes, exact-repeat blocks, pauses and deterministic cues. The speech node requires explicit role voices and replays each repeated recording without another model call; output is previewed and exported as WAV. See [the rules](docs/SPEECH_SCRIPT.md) and [the AI writing prompt](docs/SPEECH_SCRIPT_PROMPT.md). 配音脚本的格式规则和 AI 写稿提示词见上述文档。
