# Shape and Deck handoff

Shape prepares accepted source materials; Deck owns card editing, layout and printing. Existing
Shape CardKit revisions remain readable and immutable. The adapters exchange files, never write the
other application's database, and never interpret foreign artifact ids as local artifact pins.

This document defines the implemented entry points as they are added in this change. A successful
launch is not a completed edit. Only the explicit return actions below publish terminal receipts.

## One editable card set

The shared Shape CLI/desktop card-kit JSON channel accepts:

```json
{"action":"deck_transfer","artifact_id":"01a14c2d-733f-7871-b118-d2944688cadb","draft_digest":"<64 lower-case hex>","request_id":"<unique token>","destination":"/absolute/new-transfer-directory"}
```

The destination must not exist or be inside the source project. `manifest.json` is the publication
marker, written last. Failed partial directories contain no valid transfer; retry into a new
directory. The manifest contains the entire saved draft, including hidden fields, unresolved text
and stable card ids. Transfer does not require a printable or accepted card set and does not accept
any material Candidate. It preserves an absent `layout` field and does not upgrade legacy content.

```json
{
  "schema":"deck.card-transfer/1",
  "request_id":"shape-to-deck-unique-token",
  "origin":{
    "producer":"shape",
    "project_id":"01a14c2d-7000-7000-8000-000000000001",
    "kit_id":"01a14c2d-733f-7871-b118-d2944688cadb",
    "expected_head":null,
    "draft_digest":"<64 lower-case hex>"
  },
  "composition":{
    "name":"Learning cards","brief":"","style":"",
    "canvas_width":750,"canvas_height":1050,
    "cards":[{"id":"spring","title":"SPRING","rules":"","art_prompt":"A flowering tree","quantity":1}]
  },
  "materials":[]
}
```

Each unique `composition.inputs()` pin has exactly one material record, including hidden artwork
and text. Records have `source` (foreign `WorkingInput`), `content` (the existing `ContentRef`),
`contract` (the existing `ArtifactContentContract`, or `null` for legacy text), and `relative_path`.
`ContentRef.digest` retains its existing serialization as an array of 32 byte values; the separate
`*_blake3` fields in this protocol are 64 lower-case hexadecimal characters. PNG records require an
ImageRaster contract. Paths are simple relative material paths inside this directory; symlinks,
missing, extra and duplicate source records are rejected. Bounds are 64 cards, 20 MiB per PNG,
8 KiB per text and 128 MiB of unique material bytes, matching Shape CardKit inputs.

Deck's public CLI receives
`deck-cli import-transfer NEW.deck MANIFEST.json --expected-blake3 HEX`. It checks the manifest and
every source byte before publishing a new Deck project, imports materials into new local pins,
rewrites those pins in the composition, and preserves stable card ids and all other content.
The import receipt records foreign-to-local mappings and any image canonicalization. Retrying the
same target and exact manifest reopens the same imported identity; unrelated targets are not
overwritten. A materials ZIP is a publication output, not an editable project transfer.

## Human editing and output return

Deck accepts `--project ABSOLUTE.deck --kit ARTIFACT_ID` to open its own project and exact card set.
For a tracked external edit it accepts `--external-edit REQUEST.json --result NEW-RESULT.json`.
Shape writes one request, then launches that real entry point:

```json
{
  "schema":"deck.external-edit/1",
  "request_id":"shape-to-deck-unique-token",
  "origin":{
    "producer":"shape",
    "project_id":"01a14c2d-7000-7000-8000-000000000001",
    "kit_id":"01a14c2d-733f-7871-b118-d2944688cadb",
    "expected_head":null,
    "draft_digest":"<64 lower-case hex>"
  },
  "input":{
    "kind":"transfer",
    "manifest_path":"/absolute/transfer/manifest.json",
    "manifest_blake3":"<64 lower-case hex>"
  }
}
```

On a later edit, `input` is
`{"kind":"project","project_path":"/absolute/cards.deck","project_id":"<Deck project UUID>","kit_id":"<Deck kit UUID>"}`.
Deck asks for a new local project destination for the first transfer; it never edits the Shape
bundle. The associated Deck project is subsequent work, independent of newer Shape drafts. Shape
offers an explicit fresh snapshot transfer when source content has changed.

The user chooses **Return outputs** after explicitly exporting PDF or ZIP:

```json
{
  "schema":"deck.external-edit-result/1",
  "request_id":"shape-to-deck-unique-token",
  "request_blake3":"<hash of exact request file bytes>",
  "outcome":"completed",
  "document":{
    "project_path":"/absolute/cards.deck",
    "project_id":"<Deck project UUID>","kit_id":"<Deck kit UUID>",
    "accepted_revision_id":"<Deck accepted kit revision UUID>"
  },
  "outputs":[{
    "kind":"print_pdf","path":"/absolute/cards.pdf",
    "byte_length":12345,"blake3":"<64 lower-case hex>","media_type":"application/pdf"
  }],
  "error":null
}
```

Output kinds are `print_pdf` (`application/pdf`) and `materials_zip` (`application/zip`). Completed
requires a document with an accepted revision and at least one verified output, at most 16 outputs
and 512 MiB in total. Cancelled and failed receipts have no outputs; `document` may retain an imported
or saved project with `accepted_revision_id:null` so a cancelled export does not lose editable work.
`error` is null except for a bounded failed explanation. Receipts are new, no-clobber files. Shape
checks exact request identity, each file's regular-file status, length, BLAKE3 and type signature,
then presents local links. These PDF/ZIP files do not become ImageRaster artifacts or accepted
Shape revisions. Returning outputs does not modify the original Shape card set.

## A single generated image for Deck

Deck launches the actual Shape executable with
`--deck-material-task REQUEST.json --result NEW-RESULT.json`. This dedicated entry creates an
isolated task project and opens the existing image-generation workspace; it never opens Deck's
database. Shape uses its own Infer app identity and credentials. Reference-conditioned generation
is not provided by this entry point.

```json
{
  "schema":"deck.material-task/1",
  "request_id":"deck-image-unique-token",
  "target":{
    "project_id":"<Deck project UUID>","kit_id":"<Deck kit UUID>",
    "card_id":"spring","draft_digest":"<Deck saved draft digest>","artwork":null,
    "card_snapshot":{"id":"spring","title":"SPRING","art_prompt":"A flowering tree","artwork":null}
  },
  "prompt":"A flowering tree in spring, a simple learning-card illustration.",
  "canvas":{"width":750,"height":1050}
}
```

The target is opaque return context. Shape validates bounded strings and the exact request hash;
only Deck may interpret or bind its own ids. `artwork` is null or the original Deck WorkingInput.
New tasks include optional `card_snapshot`, the complete original CardKitCard (at most 24 KiB), with
matching `id` and `artwork`. Deck compares that stable original card when receiving a result; editing
other cards or reordering the set is allowed. Older requests without a snapshot retain strict
whole-draft digest matching. No hidden content is deleted when taking the snapshot.
Request JSON is at most 64 KiB, the nonblank prompt at most **16 KiB** UTF-8. Requested canvas axes
are 1..4096 with at most 16,777,216 pixels, a deliberately bounded adapter to the current image
consumer; this does not change CardKit or portable AI-image schema limits. The model's actual output
size can differ from the authored canvas.

Generation produces normal Shape Candidates. The user compares and explicitly accepts one into the
task's one image artifact, then explicitly chooses **Return to Deck**. Saving a draft or merely
receiving a Candidate never returns an image. The result is:

```json
{
  "schema":"deck.material-task-result/1",
  "request_id":"deck-image-unique-token",
  "request_blake3":"<hash of exact request file bytes>",
  "outcome":"completed",
  "output":{
    "path":"/absolute/task-output/image.png","byte_length":12345,
    "blake3":"<64 lower-case hex>","media_type":"image/png",
    "source":{"artifact_id":"<Shape image UUID>","revision_id":"<Shape accepted revision UUID>"},
    "contract":{"kind":"image_raster","contract":{
      "schema_revision":"20260811.1","width":1024,"height":1024,
      "pixel_format":"rgba8","bit_depth":8,"alpha_mode":"straight","orientation":"top_left",
      "color_primaries":"srgb","transfer_function":"srgb","color_profile":{"kind":"srgb"},
      "light_reference":"display_referred","premultiplied":false
    }}
  },
  "error":null
}
```

Returned PNGs are at most 20 MiB, 4096 per axis and 16,777,216 pixels, with their complete verified
ImageRaster contract. Deck checks request id/hash, bytes and contract, imports the image into a new
Deck-local accepted pin, and offers **Use this image** for the exact original card. This final card
binding is explicit. Switching cards does not retarget a running request; changing project, kit,
source artwork or the original card invalidates binding. Cancelled/failed use `output:null`.

## Lifecycle and recovery

Both request/result formats reject unknown fields and malformed or oversized identities. Results
must not alias a request, manifest, project metadata or source material. Read and publication paths
must be absolute regular files with no symlink leaf. Existing results are never overwritten.

Shape stores outbound request/result locations and the associated Deck identity in local settings,
keyed by canonical Shape bundle path plus project and kit. Restarting Shape can resume waiting or
read a returned receipt. A project copy with the same UUID has a different key. A known failed launch
or process exit without receipt is an incomplete, retryable session; it is never a successful edit.
No short timeout interrupts a human editing session. Stopping waiting does not kill the external
application or delete its project. Late callbacks are checked against the captured request and
target; they cannot mutate the currently selected card/project. The isolated Shape image-task window
writes a cancelled terminal receipt on explicit cancel/close, and failed startup writes a failed
receipt whenever the request and destination can be safely validated.

Both GUI executables answer `--handoff-capabilities` without starting the GUI. Shape emits
`{"schema":"shape.handoff-capabilities/1","deck_material_task":"deck.material-task/1","deck_material_result":"deck.material-task-result/1"}`;
Deck emits `{"schema":"deck.handoff-capabilities/1","external_edit":"deck.external-edit/1","material_result":"deck.material-task-result/1"}`.
The launcher probes this bounded public response before sending new flags. An executable path alone
does not establish protocol support. Installed macOS applications resolve their exact
`CFBundleExecutable`; companion CLI binaries are not selected as GUI editors.
