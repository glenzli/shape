# Shape CLI

`main.rs` routes the public commands. `demo` creates a synthetic project; `inspect` opens and
summarizes an existing project through Core (including supported project schema upgrades).

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
