# Picture-book capability / 绘本创作

Shape can create and continue an accepted, revisioned picture book from its text
and image materials. The desktop offers a shared page sequence, language scripts,
reading-layout preview, PDF output and an editable Storybook `.scproj` handoff.
The JSON CLI exposes the same composition and acceptance contracts for AI callers.
This integrates the real **Storybook Co-editor** authoring workflow, not the
unrelated UI component explorer. The original application and existing books are
not migrated or modified.

## Desktop workflow

1. Open or create a Shape project. Choose **Picture books / 绘本** in the project
   rail, or **Create or continue a picture book** from the creation menu.
2. Start a new book or select a saved one. Set its title, canvas and text size.
   Add image or blank pages; select accepted artwork for each page. Page zero is
   the cover. Mark optional non-cover pages **Print only**.
3. Select an accepted text material for each language, or type a script and choose
   **Adopt script as new material**. This creates a derived material; the source
   remains unchanged. Adopt the current script, or choose **Discard draft**, before
   switching languages, choosing another text material or refreshing materials.
   These replacement actions are disabled while script text is unadopted.
4. Choose **Review book**, inspect pages and languages, then **Adopt book
   version**. Reviewing and closing/discarding a candidate does not save a book
   revision. Accepted versions survive closing and reopening the project.
5. Export **PDF** for the selected language, or **.scproj** for further editing in
   Storybook. Export uses the adopted version. Use a new destination outside the
   Shape project. Reading PDFs omit print-only pages; the optional inclusive PDF
   is a page proof, not printer imposition.

The basic reading layout fits artwork above text, preserves its aspect ratio,
and handles Unicode text with installed fonts. Review/export checks all pages for
overflow and missing glyphs; shorten the text, enlarge the canvas or lower the
font size if preparation fails. Missing language blocks remain textless and are
reported for review; another language is never silently substituted.

Drafts are temporary window state, not autosaved project revisions. Closing the
workspace keeps its draft during the current app session; **Discard draft** or
switching to a saved/new book replaces it. PDF export is a synchronous bounded
operation. Canceling its file dialog makes no change; there is no mid-export
cancel control.

## Material editing is an upstream stage

Use **Edit artwork material** to enter the existing material workflow. Review and
adopt a Shadow return, reopen the book workspace, and explicitly select that
derived accepted image in the artwork picker. A newer head on an already bound
material can be selected with **Use current material versions**. Review and adopt
the changed book afterward. Neither action changes old book revisions or the
Original material. Refreshing unchanged pins preserves the draft, reviewed
candidate and preview. An artwork-only refresh preserves the active script,
language and page selection; a changed active script pin loads its newly adopted
text. Changed pins require reviewing and adopting the book again.

Echo remains available for audio material stages, including explicit PCM24
conversion. This book slice has no audio/narration track. It does not add Shadow
support for transparent/16-bit/ICC image input or producer export cancellation.
See [material editing](MATERIAL_EDIT.md) for those exact contracts.

## Script and native compatibility

Each language stores a full script: `[Cover]` / `[封面]`, `[Author]` / `[作者]`,
optional `[Title]` / `[扉页]`, and body tags `[1]`, `[2]`, … . A title block adds
page one and shifts numbered body pages. Every language must agree on whether
this title page exists. All languages share ordered image/blank pages. Artwork
reordering retains positional script tags; the print-only marker follows its page.

Shape rejects duplicate/unknown tags, numeric aliases, discarded preamble,
page zero and out-of-range body blocks. Textless pages are allowed. Author text
becomes a cover credit and supplied publication metadata; no credit is invented.

The exported ZIP contains Storybook schema `20260907.01`, `project.json`, PNG
assets and `shape-origin.json`. Native page settings use **numeric page-index
keys**, including `print_only`; asset-path keys are not interpreted by Storybook.
Repeated artwork has independent page paths. The origin pins the Shape project,
input revisions and exact content digests without machine paths or credentials.
Exports are standalone snapshots; later Shape changes cannot modify them.

Storybook has its own typography and full-page artwork layout, so its preview is
not pixel-identical to Shape's reading PDF. Export supplies readable text defaults
and separates the cover title and author baselines. Review native typography for
long credits or specialized publication. Advanced text/image positioning, print
imposition, `.scpub` publishing and native archive intake back into Shape remain
in the original application's workflow. No automatic publication is performed.

Compatibility references are the original MIT-licensed Storybook Co-editor
(`Copyright (c) 2026 Glen Li`): `project_model.rs`, `project_archive.rs`,
`story/script.ts`, `project/pageSettings.ts` and `editor/pageCollection.ts`.
Shape implements a bounded compatible manifest and script subset; it does not
embed the Storybook UI or replace its authoring engine.

## AI / CLI workflow

Run `shape-cli picture-book /absolute/project.shape /absolute/request.json`.
Every response uses `shape.picture-book-result/1`; errors return `status:"error"`
and nonzero exit. Request files are strict JSON, regular files, at most 256 KiB.

The composition object has this shape (replace all UUID placeholders with
accepted Shape identities):

```json
{
  "name": "Moon / 月亮",
  "default_language": "en",
  "languages": {
    "en": {"artifact_id": "TEXT_UUID", "revision_id": "TEXT_REVISION_UUID"}
  },
  "pages": [
    {"kind": "image", "source": {"artifact_id": "IMAGE_UUID", "revision_id": "IMAGE_REVISION_UUID"}},
    {"kind": "blank"},
    {"kind": "blank", "print_only": true}
  ],
  "canvas_width": 1024,
  "canvas_height": 768,
  "font_size": 26
}
```

| Action | Request fields beyond `action` | Result |
| --- | --- | --- |
| `adopt_script` | `name`, `text`, optional `source` pin | New or branched text revision; explicit durable action |
| `preview` | `composition`, optional `target` book pin | Parsed text, missing/stale input details and `review_digest`; no durable book write |
| `accept` | Same `composition` and `target`, exact `review_digest` | Accepted book revision; rechecks material and book heads |
| `read` | `source` book pin | Exact historical composition and preview metadata |
| `export` | `source` book pin, new absolute `destination` ending `.scproj` | Native archive receipt with BLAKE3 and byte count |

For a new book omit `target`. For an edit use its accepted artifact/revision pin.
Preview is structural validation; typography and PDF rendering belong to the
desktop. The historical one-shot request `{ "composition": {...},
"destination": "/absolute/new-book.scproj" }` remains supported and exports
current materials without creating a book Artifact. Omitting `font_size` uses 26.

## Persistence, bounds and ownership

`ArtifactKind::PictureBook` stores `shape.picture-book/1` in the ordinary CAS.
`book.compose` produces a transient candidate. Acceptance atomically checks the
output head, exact source heads, source types and provenance. Previously accepted
versions retain their pinned materials after source heads advance. Re-adopting
an old pin requires explicitly choosing current sources; reading/exporting the
old accepted book remains supported.

The project schema is `20261004.1`; prior supported bundles, including
`20260811.5`, migrate on opening. Older Shape binaries do not understand the new
book kind. Do not open a migrated project with an older build.

Limits: 64 ordered pages, 8 languages, 64 distinct materials, one revision per
material, 256 KiB per script/manifest, 128 MiB total material reads and canvas
axes up to 8192. Desktop reading layout requires axes of at least 256 and text
size 12–96. Saved Artifact names follow the existing 120-byte name bound;
legacy snapshot export retains its 256-byte title bound. Font availability can
differ by machine. This release was checked with installed PingFang SC on macOS.

`shape-domain::picture_book` owns the portable contract; store validation owns
durable provenance; `shape-core::project::picture_book::{script,version}` owns
script interpretation and use cases. The bridge is a terminal adapter with the
ordinary candidate shelf. `PictureBookController` owns desktop drafts/commands,
and `picture_book_renderer` supplies shared preview/PDF layout.

## Validation boundary

Adjacent tests cover transient review/cancel, stale output and source heads,
repeated edits, reopen, unchanged originals, missing translations, exact returned
material bytes, no-clobber exports and indexed native page settings. The packaged
`shape-desktop-picture-book-smoke` also exercises QML page movement, bilingual
layout, three-page reading PDFs, a four-page inclusive proof and a real QQuickWindow
capture. Its synthetic fixture never edits a user's existing book.

The native interop harness runs the original Storybook model and archive
open/write/reopen code, plus original page-settings/move helpers. This is not a
claim of a completed Storybook GUI publishing round trip. The synthetic book workflow also passes in a visible Cocoa window using Metal,
with an inspected actual screenshot. Native Storybook publication UI, print
imposition, Windows and font portability remain unverified in this slice.
