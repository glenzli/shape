# Shape to Vox transfer

Shape retains general material creation and accepted history. Vox owns script-driven speech
production. The migration adapter reads an existing Shape project and publishes a standalone
directory; it never checkpoints, accepts, repairs or upgrades the source project.

`shape-cli legacy-snapshot PROJECT.shape` returns `shape.legacy-snapshot/1`: legacy card/book
identities and saved graph digests, text heads and writing-draft digests, and saved speech graph
identities. This is saved state only; another process's transient candidates are not observed.

`shape-cli vox-transfer PROJECT.shape REQUEST.json` accepts this bounded JSON request:

```json
{
  "request_id": "shape-to-vox-unique-token",
  "text_artifact_id": "<UUID>",
  "expected_text_head": null,
  "authoring_graph_digest": "<64 lower-case hexadecimal characters>",
  "speech_target_artifact_id": "<optional UUID>",
  "speech_graph_digest": "<optional exact saved graph digest>",
  "accepted_audio": {"artifact_id": "<UUID>", "revision_id": "<UUID>"},
  "destination": "/absolute/new-transfer-directory"
}
```

`expected_text_head` must be present. It is the selected accepted text revision, or JSON `null`
for an unaccepted writing draft. Optional fields may be omitted or null. When the selected text
has a saved authoring graph, its exact digest is required and the complete unaccepted text and
settings are always transferred. A speech target and its graph digest must be selected together.
The source text artifact and output heads and selected graph digests are checked again before
publication. Historical speech input pins are carried with their exact original text.

The request is at most 16 KiB. Request ids contain 1–128 ASCII letters, digits, dots, underscores
or hyphens. Destination is a new absolute directory outside the canonical source bundle; existing
paths, symlink aliases into the bundle and replacement destinations are rejected. Each material
file is written without overwriting a destination, and `manifest.json` is written last. An
incomplete directory without that manifest is not a published transfer; retry into a new directory.
The result contains `manifest_path`, exact hexadecimal `manifest_blake3`, `request_id` and origin.

The manifest's exact schema is `vox.shape-transfer/1`:

| Field | Meaning |
| --- | --- |
| `request_id` | Caller-supplied transfer identity |
| `origin` | `producer: "shape"`, project id, selected `text_artifact_id` and `observed_text_head` |
| `accepted_text` | Exact accepted text material, or null |
| `authoring_draft` | `status: "unaccepted"`, target id/head, graph digest, complete original graph and settings, explicit text format and exact text material, or null |
| `speech_draft` | `status: "unaccepted"`, target id/head, graph digest, draft id, original input pin, exact `input_text`, original graph/settings, normalized `SpeechSynthesisOperation`, and `issue`, or null |
| `cue_materials` | Deduplicated exact referenced cue WAVs and their decoded audio contracts |
| `accepted_audio` | Existing Shape accepted audio and its original source/provenance, or null |

An accepted material has `source` (`WorkingInput`), complete original `revision`, original
`content` (`ContentRef`), original `contract` (`ArtifactContentContract`, possibly null for legacy
text), and `relative_path`. A draft text or cue material has `content`, explicit `contract` and
`relative_path`. Original `ContentRef.digest` uses Shape's existing array of 32 bytes; the separate
graph and manifest digest strings are hexadecimal. Paths have the form `materials/DIGEST.txt` or
`materials/DIGEST.wav`. Exact length and BLAKE3 are verified before publication. Text is bounded
to 4 MiB, saved authoring text to its existing 48 KiB limit, each cue to 8 MiB, accepted audio to
128 MiB, and deduplicated transfer bytes to 168 MiB. WAV contracts are independently re-parsed.

`authoring_draft.format` preserves the stored plain/script interpretation. Its `settings` contains
the original versioned configuration, including legacy profile and unknown fields; draft bytes
remain separate from accepted text. Speech settings preserve language, speed in thousandths,
disclosure, versioned preset aliases, role bindings and cue choices. An older legal unconfigured
speech graph has null `settings` and `operation`, with `issue: "speech_settings_unconfigured"`;
the receiver must preserve it without inventing voice defaults or enabling execution.

`accepted_audio` has `status: "accepted_external_origin"`, `material`, complete original
`transformation`, original `receipt`, original speech `operation` when available, and `input_text`.
For speech audio, `input_text` is the exact historical transformation input; it can differ from
the currently selected accepted text. Audio without a provable speech input has null `input_text`.
It remains imported external history. The receiver must never attribute these bytes to a new Vox
synthesis or silently associate them with a newer script.

The receiver verifies the expected manifest digest, all material identities, contracts and paths
before publishing its own project. It retains accepted sources and unaccepted drafts separately,
records the original manifest and provenance, and requires explicit acceptance for future candidates.

Desktop uses the same owner through `session_legacy_snapshot_json()` and
`session_vox_transfer_json(request_json)`. These calls do not initialize old speech defaults or save
any draft. The producer owner is `shape-core::project::vox_transfer`; the store's current-schema
read-only opener enforces bounded metadata without migrations. Legacy PCM/script parsers remain
available for interpreting existing bytes; their presence does not advertise Shape speech production.
