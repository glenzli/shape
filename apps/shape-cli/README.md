# Shape CLI

`main.rs` routes the public commands. `demo` creates a synthetic project; `inspect` opens and
summarizes an existing project through Core (including supported project schema upgrades).

`card_kit.rs` routes recoverable card-list drafts, exact review/accept, historical reads, offline
material-kit export and accepted-version home-print planning through the same Core action used by
the desktop. The concrete JSON actions,
material bindings, readiness checks and bounds are in [Card material kits](../../docs/CARD_KIT.md).
Run `shape-cli card-kit /path/to/project.shape /path/to/request.json`; separate invocations recover
the same project-backed draft and require its last observed digest before saving or accepting.
`{"action":"print_plan","source":{"artifact_id":"<UUID>","revision_id":"<UUID>"}}`
returns unique readable card SVGs and the exact A4 page/crop geometry consumed by the desktop PDF
writer. Optional settings select Letter, card size, margins and crop marks; they do not alter history.
The CLI returns the bounded plan as JSON; the desktop paints the PDF using that plan.

`script_check.rs` owns read-only file intake and the JSON diagnostics command used by external
AI tools and authoring scripts. It calls the same `shape-domain::speech_script` parser as desktop
review and synthesis planning. It does not open a project, run inference, resolve voices/assets,
accept a candidate, or change its input.

```sh
cargo run --quiet --package shape-cli -- script-check /path/to/script.txt
```

The input must be a regular UTF-8 file of at most 65,536 bytes. Standard output contains one JSON
object with schema `shape.cli.script-check@20261002.1`:

| Field | Meaning |
| --- | --- |
| `grammar_revision` | Version of the domain grammar used for this check |
| `source_digest`, `source_bytes` | BLAKE3 hexadecimal identity of the exact input bytes and their length |
| `valid` | The parser reported no syntax issues and found spoken text |
| `issues` | Bounded list of one-based source `line` and stable domain `code`; raw source is omitted |
| `roles`, `cues` | Declared names, in deterministic order |
| `note_lines` | One-based lines of explicit non-spoken production notes, excluding speaker/language controls |
| `explicit_pause_ms` | Expanded pause and repeat-gap duration; excludes speech and cue audio |
| `synthesis_checked` | Always `false`: voice bindings, assets, Runtime access and audio have not been checked |

Exit status is **0** for valid syntax, **2** for script issues (still JSON on stdout), or **1** for
usage, file, encoding or size errors (message on stderr; no report). If syntax is invalid, roles,
cues and timing describe only the partial parse. The digest lets a caller detect whether the exact
file changed after review; this command does not reserve or adopt that revision.

An AI caller can check a draft, repair the reported lines, and check the changed file again before
offering it for human review. Syntax validation does not verify names, facts, pronunciation, a
requested total duration or listening quality. Production notes remain visible review reminders.
See the [script rules](../../docs/SPEECH_SCRIPT.md) and
[writing guidance](../../docs/SPEECH_SCRIPT_PROMPT.md) for those authoring boundaries.

Black-box command tests live in `tests/script_check.rs` and exercise the built CLI with bounded
temporary files; they do not substitute a test harness for this public command.

## Project status for an external tool

```sh
cargo run --quiet --package shape-cli -- project-status /path/to/project.shape
```

`project_status.rs` is a public consumer of `ShapeProject::inspect`. It emits one JSON object with
schema `shape.cli.project-status@20261003.1`, current accepted Artifact/Scene heads, and saved draft
input/output versions. It opens the current project schema read-only and reads all projected state
in one SQLite transaction. It never runs the ordinary opener's schema migration, loads content
objects, creates candidates, changes a draft or accepts an output. SQLite still uses its normal WAL
locking sidecars; do not copy an active database without its WAL or use an immutable read shortcut.

| Field | Contract |
| --- | --- |
| `project`, `artifacts`, `scenes` | Saved project identity, names, kinds and accepted revision IDs |
| `drafts[].input` | Exact accepted source Artifact/revision pinned by the saved draft, or null |
| `drafts[].current_input_revision` | Source head observed in this read snapshot, or null |
| `drafts[].input_status` | `current`, `changed`, `missing`, `unbound`, or `not_required` |
| `drafts[].expected_output_revision`, `current_output_revision`, `output_status` | Saved target anchor versus current target head (`current` or `changed`) |
| `drafts[].next_action` | `refresh_input_and_review`, `bind_accepted_input`, `reopen_output_and_review`, or `review_in_desktop` |
| `drafts[].graph_digest` | BLAKE3 of the serialized typed saved graph, including its exact configuration strings |
| `snapshot_digest` | BLAKE3 of the emitted report before adding this digest; equal inspected state is repeatable |
| `drafts[].execution_checked` | Always false: matching versions do not establish grammar, voice, provider or execution readiness |
| `candidates` | `observed: false`, `visibility: desktop_session_only`; a separate CLI process cannot see the desktop's transient shelf |
| `acceptance` | Always `explicit_session_action_required`; this report never authorizes or performs acceptance |

Content bodies, prompts, raw draft settings, credentials and provider calls are excluded. Unsaved
desktop changes are outside this persisted view. A digest is evidence of the observed state, not a
lease or a replacement for the existing expected-head/working-graph checks when a later action runs.
For a changed source, preserve the authored draft, explicitly refresh its input in the desktop,
review the result and produce a new candidate before accepting. Old candidates are invalidated
when their source changes. Repeating a refresh with the same source preserves the current candidate.

The inspection admits at most 1,024 rows in each metadata collection and 4 MiB of combined projected
metadata, with a separate 64 KiB manifest limit and a 250 ms SQLite busy timeout. Oversized, corrupt,
missing or non-current-schema bundles fail without partial output or automatic repair. Exit **0**
means the report was read successfully, even if a draft is stale. Project failures emit a JSON
`status: "error"`, `code: "project_unavailable"` and a diagnostic message with exit **1**. Usage errors
remain on stderr with exit **1**. An old project must first be upgraded deliberately in the desktop.

`tests/project_status.rs` drives the actual command through a live WAL-backed project: independent
branch creation, source revision changes, rejected stale adoption, explicit refresh, and successful
adoption that preserves the newer original. Store-owned tests verify read-only SQL, unchanged
manifest/database/WAL bytes, coherent concurrent snapshots and metadata size/row limits.

### Check the choices for a production script

```sh
shape-cli script-check dialogue.txt --bindings bindings.json
```

The optional binding file is a regular UTF-8 JSON file, bounded to 64 KiB, containing the portable
`SpeechScriptOptions` shape: `revision`, `roles` (name to `alias`/`catalog_revision`) and `cues`
(name to a `SpeechCueAction`). Use the current grammar revision reported by `script-check`.
Bindings are supplied explicitly; this command does not read or alter a desktop session.

Every report now includes `prerequisites` from the same domain owner used by desktop audition:
`ready`, `syntax_valid`, `options_valid`, `spoken_passages`, `missing_roles` and `unresolved_cues`.
Each missing choice has a `name` and one-based `line` for its first playback use, sorted by name.
Unused declarations do not block; built-in cues and explicitly skipped cues are resolved.
`spoken_passages` counts authored speech events before repeat playback or Runtime segmentation.
`bindings_checked` records whether a file was supplied, and `bindings_digest` is the exact-byte
BLAKE3 identity of that file (null when omitted). Without a file, prerequisites describe the empty
binding set and the existing syntax-only exit behavior is unchanged. With `--bindings`, exit **2**
also means incomplete or invalid portable choices; `valid` continues to describe syntax only.
Malformed JSON, invalid UTF-8, non-files and oversized inputs return exit **1** without a report.

The AI workflow is: check syntax, provide explicit voices/cue choices, repair each reported missing
choice, check again, then hand the script and choices to human review. A changed source or binding
digest requires another check. Portable readiness never establishes that a voice exists in the
Runtime, an imported WAV can be loaded, access is granted, speech sounds right, or a candidate may
be accepted. `synthesis_checked` remains false. Project version checks remain in `project-status`.


## Read an exact accepted material after reopening

```sh
shape-cli material-edit /path/to/project.shape /path/to/read-material.json
```

`read-material.json` contains `{"action":"read","source":{"artifact_id":"<UUID>","revision_id":"<UUID>"}}`.
The existing `shape.material-edit-result/1` envelope returns the verified exact image/audio
revision, its content reference and media contract, the artifact's observed head, and any stored
material-edit source/normalization lineage. Historical versions remain readable after their head
advances. The requested pin is never silently replaced by the newest head.

This action opens the current schema read-only, verifies bounded durable bytes and returns only
metadata; it does not migrate, repair, produce a candidate or authorize acceptance. A book caller
can repin a page using `picture-book read → preview → accept`; an audio caller can carry the content
reference into an explicit cue in `script-check --bindings` for portable readiness. That check does
not persist the cue in a speech draft or verify playback/synthesis. See the complete
[material editing contract](../../docs/MATERIAL_EDIT.md) for request fields, limits and recovery.

`tests/material_edit.rs` drives these public consumers in new processes, including old-version
reads, repeated returns, bilingual book continuation and explicit PCM24 normalization provenance.
