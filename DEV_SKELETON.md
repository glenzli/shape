# Shape Development Skeleton

## Purpose and source of truth

Shape owns general material creation, transformation and accepted source history. Deck owns cards
and printing, Storybook Co-Editor owns books, and Vox owns script-led voice production. Shadow and
Echo remain independent specialist editors.

[README.md](README.md) routes to implemented owners. [SHAPE_PLAN.md](SHAPE_PLAN.md) retains staged
design and historical milestones; it is not evidence that a historical desktop feature is still
available. [AGENTS.md](AGENTS.md) defines dependency direction, build output, localization and tests.
The nearest source, module documentation and adjacent tests own actual behavior.

## Product boundaries

- Shape provides general text, image and sound operations, candidate review, provenance and versions.
  A graph records processing relationships; users need not draw a node diagram for ordinary creation.
- Professional card, book and voice-production workspaces belong to their dedicated applications.
  Do not restore them as hidden tabs, embedded editors or alternate creation templates.
- Existing CardKit, PictureBook and speech data remains readable. Compatibility code owns bounded
  inspection, exact snapshot export and migration; it does not provide a second production workflow.
- External applications exchange material copies, request identities and receipts. Each application
  owns its projects and writable database. A launched process is not proof of completed editing.
- No generic executor, bridge, media kernel or plugin SDK should precede a real consumer.
- Existing accepted bytes remain readable when a model or external application is unavailable.

See [application boundaries](docs/NATURAL_WORKFLOW.md) for the migration and interaction policy.

## Semantic owners

| Owner | Responsibility |
| --- | --- |
| `shape-domain` | Platform-independent creative identities, typed graphs, media contracts and validation |
| `shape-execution` | Executor lifecycle, physical media operations, Runtime adaptation and receipt validation |
| `shape-store` | SQLite metadata, verified BLAKE3 objects and atomic durable publication |
| `shape-core` | General creative use cases, transient candidates and bounded legacy migration |
| `shape-desktop-bridge` | Terminal Rust/CXX session projection and candidate adoption |
| `apps/shape-cli` | Strict public JSON adapters over the same use cases |
| `apps/desktop` | Qt/QML interaction, asynchronous controllers and concrete external application adapters |

Keep domain free of Qt, C++, provider, filesystem, SQLite and network types. Application assembly
may depend on the desktop bridge; domain, execution, store and core may not depend back on it.
Keep state, mutation authority, lifecycle, failure policy and focused tests with their semantic owner.
Entrypoints and CMake/QML composition roots assemble these owners.

## Creative history and working state

`Artifact` identifies a creative value; `ArtifactRevision` is an immutable accepted state.
`Scene` and its typed Source/Operator/Output graph describe user-authored processing relationships.
Media-internal tools and physical executor steps do not automatically become Scene nodes.

`Transformation` records creative meaning. Execution plans and receipts record physical facts.
Execution success only creates a transient candidate. Explicit acceptance alone advances the durable
head and rechecks the source head, output head and exact authored operation in the store transaction.

Mutable WorkingGraph state remains separate from accepted history. Restoring or exporting a draft
must not adopt it. Legacy scripts retain their original format and exact bytes when shown as text.
Historical material pins remain historical pins; a reader must not silently select the latest head.

## Media and Runtime

- `shape-domain::image_raster` owns raster interpretation. `shape-execution::raster` owns bounded
  decode and deterministic pixel operations. Core image owners propose and explicitly accept results.
  See the [raster index](crates/shape-execution/src/raster/README.md) for per-operation limits.
- `shape-domain::ai_image` owns source-less and material-conditioned image semantics. Generated
  variants are separate candidates for one logical output. A text-only route cannot substitute for
  an unavailable typed raster-output capability.
- Audio contracts, exact PCM validation and accepted provenance remain available for source material,
  sound generation, external editing and legacy recording reads. Voice-production direction and
  synthesis UI belong to Vox.
- `audio.generate` owns sound kind, authored/effective prompt, duration and seed. Its Runtime receipt
  and output remain transient until explicit acceptance. Prompt preparation cannot silently fall back.
- The pinned official Infer Runtime client owns Discovery, transport, capability negotiation and
  owner-only credential loading. Shape owns its policy and candidate boundary. There is no product
  fixed-port fallback, provider impersonation or executor success shortcut around acceptance.
- Imported Shadow/Echo originals are read-only references. Material edit returns require ticket,
  source, project and output identity checks. PCM24 conversion and known-sRGB normalization remain
  explicit and retain original bytes and lineage.

## Desktop lifecycle

`DesktopSession` coordinates project mutations and the typed candidate shelf. Ordinary snapshots
carry bounded metadata; selected raster/audio bytes cross CXX only on demand into bounded native
preview/playback owners. QML does not become persistence authority.

The text, image, sound, material-edit and Agent file-task controllers retain complete asynchronous
lifecycles. Dispatch, completion, cancellation and late-response admission bind the full project,
draft, source and output identity. A project switch or changed target cannot retarget a running task.
The source viewer, candidate comparison and accepted history remain separate.

Compatibility presentation reads legacy snapshots without checkpointing them. It can open bound
materials and export the selected saved or accepted snapshot to the proper specialist application.
Missing applications leave a usable export and a clear status. Card/book/voice authoring controls,
print renderers and their obsolete native registrations do not belong in this surface.

Concrete Deck material tasks continue using Shape's general image operation and explicit image
acceptance. Specialist outputs do not enter Shape as fabricated raster/audio artifacts merely to
display a PDF or ZIP link.

## Validation and developer launch

Rust tests stay adjacent to production owners; crate-facade and black-box tests retain the topology
in AGENTS.md. Build native tests before running them. Cross-language changes need an actual linked
consumer and packaged QML/resource loading, not compile-only evidence.

Legacy migration checks must cover saved drafts, accepted history, exact bytes, stale selections,
repeated imports, invalid material identities and source-project preservation. Exercise the actual
independent importer. A producer-only serialization test is insufficient.

Developer build/promotion lives under `scripts/`. The full gate adds formatting, Clippy and workspace
tests to a packaged desktop build and smoke checks. Reuse validation only for unchanged inputs and
an identified artifact. Promote that exact validated bundle into an immutable release, then atomically
advance `current-debug`; the stable launcher never consumes an agent-specific candidate directory.
The `build:shape-canonical-debug` resource has one promotion steward.
