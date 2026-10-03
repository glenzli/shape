# Picture-book composition capability

Shape can freeze accepted text and image materials into a native editable
Storybook `.scproj`. This is a first workflow integration with the existing
picture-book authoring application at `storybook-co-editor`, not UI-component
reuse. Storybook owns the next stages: typography and readability, per-page image
layout, multilingual review, print imposition, PDF output and `.scpub` publication.
Shape's existing image/audio editing stages can supply derived accepted material
before composition. No source repository, user book or account is migrated.

## Compose a book

Run `shape-cli picture-book /absolute/project.shape /absolute/request.json`:

```json
{
  "composition": {
    "name": "Moon",
    "default_language": "en",
    "languages": {
      "en": {"artifact_id": "ACCEPTED_TEXT_ARTIFACT_UUID", "revision_id": "ACCEPTED_TEXT_REVISION_UUID"}
    },
    "pages": [
      {"kind": "image", "source": {"artifact_id": "ACCEPTED_IMAGE_ARTIFACT_UUID", "revision_id": "ACCEPTED_IMAGE_REVISION_UUID"}},
      {"kind": "blank"},
      {"kind": "blank", "print_only": true}
    ],
    "canvas_width": 1024,
    "canvas_height": 768
  },
  "destination": "/absolute/new-book.scproj"
}
```

Replace the illustrative UUIDs with accepted Shape identities. Each language's
text is its full Storybook script: `[Cover]` / `[封面]`, optional `[Title]` /
`[扉页]`, `[Author]` / `[作者]`, and numbered body blocks `[1]`, `[2]`, … .
Page zero is the cover; a title block adds page one and shifts all numbered body
pages by one. All languages must agree on this title-page role and share the same
ordered image/blank sequence. Textless pages are allowed. Author text is both a
cover credit and the supplied publication author; no missing credit is invented.

Shape rejects duplicate tags (including numeric aliases), page zero, out-of-range
pages, discarded preamble and unknown tags at a line start. This avoids upstream
last-write-wins or silently lost text. The original script bytes are preserved.
Page roles and parsed text are also recorded for compatibility checks.

The stored ZIP has Storybook schema `20260907.01`, `project.json`, canonical PNG
page assets and `shape-origin.json`. Each occurrence of an image has its own page
path, so repeated artwork can be laid out independently. The origin manifest pins
the Shape project, accepted inputs and exact BLAKE3 content references; it has no
machine paths or credentials. The native book is a standalone editable snapshot.
Changing a source in Shape does not silently change an exported book.

Open the resulting `.scproj` in Storybook, adjust layout/readability and review
each language, then use Storybook's ordinary explicit save/export/publish actions.
Shape never publishes the book merely because composition succeeded. Export
stdout has schema `shape.picture-book-result/1`, an archive digest and byte count.
`status:"error"` also produces nonzero exit. Existing destinations are never
overwritten; publish to a new path for a new composition.

## Bounds and ownership

At most 64 ordered pages, 8 language scripts, 256 KiB per script, 128 MiB of total
input bytes and an 8192-pixel canvas axis are accepted. All sources must match
current accepted heads and their exact stored bytes. Input heads are rechecked
before no-clobber publication. The immutable pinned snapshot remains authoritative
if a peer advances a source immediately afterward. Failed temporary archives are
removed; original sources and Shape heads are unchanged. No archive is extracted
or external provider process launched by this command.

`shape-core::project::picture_book` owns composition/export; its `script` child
owns the compatibility subset and ambiguity checks. The CLI is a strict JSON
adapter. No new persistent Artifact type, execution provider or database schema
is introduced for this bounded handoff.

## Deliberate next decisions

| Extension | Current slice | Decision needed |
| --- | --- | --- |
| Full page editor inside Shape | Native Storybook handles editing | Embed/reuse its engine or maintain a separate editor; large UI/architecture scope |
| Book Artifact / downstream PDF revision | New external `.scproj` snapshot with origin manifest | Freeze a book contract, candidate acceptance and durable revision model before adding persistence |
| Return an edited `.scproj` to Shape | No automatic archive intake | Define bounded extraction, provenance reconciliation and explicit accept semantics |
| Shape page-composition UI | Core and AI CLI capability | Product interaction for ordered pages/languages and page preview |
| Automated PDF / public publication | Existing Storybook user workflow | Pin export settings, font/licensing treatment and explicit publication authority |

Validation uses synthetic bilingual material, native ZIP readback, no-clobber and
stale-input failures, and the original Storybook parser/model where available.
The synthetic bilingual archive also passes the original native archive extractor,
`ProjectState` deserializer, writer and reopen path, retaining both image assets
and the origin manifest. These checks do not establish a visually inspected
finished book or a completed native PDF/publication round trip.
