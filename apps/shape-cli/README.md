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
