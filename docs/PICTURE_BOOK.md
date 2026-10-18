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
2. Start a new book or continue a saved draft/version. Set its title, canvas and
   text size. Add image or blank pages; select accepted artwork, import an image,
   or choose **Generate artwork for this page**. The generation dialog starts
   with the current page's script and editable requirements. Review its candidate
   and explicitly adopt it for that page. Page zero is the cover. Mark optional
   non-cover pages **Print only**.
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

Save dialogs capture the project bundle and exact adopted book version when
opened. A delayed file-selection callback cannot export a different book/version
or an identically identified copy of the project. PDF language and print-only
choice are captured at the same time.

The basic reading layout fits artwork above text, preserves its aspect ratio,
and handles Unicode text with installed fonts. Review/export checks all pages for
overflow and missing glyphs; shorten the text, enlarge the canvas or lower the
font size if preparation fails. Missing language blocks remain textless and are
reported for review; another language is never silently substituted.

Draft changes are checkpointed in the project, including the current page,
language, unadopted script and page-edit binding. Check the workspace's saving
status before closing; a failed save leaves the local draft available for retry.
Reopen the project and select the saved draft to continue. A checkpoint preserves
unfinished script syntax; it does not adopt a text material or a book revision.
**Discard draft** explicitly replaces it with the accepted book, or a new draft
when none has been accepted. PDF export is a synchronous bounded operation.
Canceling its file dialog makes no change; there is no mid-export cancel control.

## Material editing is an upstream stage

Use **Adjust this page in Shadow** to open a derived image stage inside the book
workspace. The book keeps its current page, language and unadopted script while
the original image remains pinned and unchanged. **Open in Shadow** prepares a
private working copy and launches the selected editor using its native request
contract. **Return edited image** in Shadow automatically produces a reviewable
Shape candidate. **Adopt image for this page** accepts the derived material and
changes only that page in the book draft, including when another page uses the
same original. The book itself still needs **Review book** and **Adopt book
version**. Returning to the page without adoption leaves the candidate available;
late returns cannot replace a changed page sequence or another project's draft.
Generation also binds the exact book, page sequence, source and image stage; it
cannot overwrite a different page after navigation/reordering, and never replaces
the unadopted script used as prompt context. A newer head on an already bound
material can be selected with **Use current material versions**. Review and adopt
the changed book afterward. Neither action changes old book revisions or the
Original material. Refreshing unchanged pins preserves the draft, reviewed
candidate and preview. An artwork-only refresh preserves the active script,
language and page selection; a changed active script pin loads its newly adopted
text. Changed pins require reviewing and adopting the book again.

Echo remains available for audio material stages, including explicit PCM24
conversion. This book slice has no audio/narration track. For repeated Shadow
edits, Shape recognizes the complete known Qt sRGB profile emitted by the verified
Shadow exporter and prepares a separate, pixel-identical opaque 8-bit working
copy. The original accepted PNG remains intact, and the preparation is recorded
and reverified. Arbitrary ICC profiles, transparent/16-bit input and producer
export cancellation remain unsupported. See [material editing](MATERIAL_EDIT.md)
for the exact profile identity and verification boundary.

## Script and native compatibility

Each language stores a full script: `[Cover]` / `[封面]`, `[Author]` / `[作者]`,
optional `[Title]` / `[扉页]`, and body tags `[1]`, `[2]`, … . A title block adds
page one and shifts numbered body pages. Every language must agree on whether
this title page exists. All languages share ordered image/blank pages. Artwork
reordering retains positional script tags; the print-only marker follows its page.

English role tags are case-insensitive. Shape rejects duplicate/unknown tags,
colliding numeric aliases, discarded preamble,
page zero and out-of-range body blocks. Textless pages are allowed. Author text
becomes a cover credit and supplied publication metadata; no credit is invented.

The exported ZIP contains Storybook schema `20260907.01`, `project.json`, PNG
assets and `shape-origin.json`. Native page settings use **numeric page-index
keys**, including `print_only`; asset-path keys are not interpreted by Storybook.
Repeated artwork has independent page paths. The origin pins the Shape project,
input revisions and exact content digests without machine paths or credentials.
Exports are standalone snapshots; later Shape changes cannot modify them.
After export, **Continue in Storybook** reveals the export folder. Open that
`.scproj` using Storybook's own Open Project command; its current desktop does
not expose a verified command-line file-open contract.

Storybook has its own typography and full-page artwork layout, so its preview is
not pixel-identical to Shape's reading PDF. Export supplies dark text over the
native white translucent wash, using Storybook's 0–100 effect scale (strength 55,
feather 80), a 90% text width, and separate cover-title and author baselines. These
remain ordinary editable native text settings. Review native typography for
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

The project schema is `20261018.1`; prior supported bundles, including
`20261004.1` and `20260811.5`, migrate on opening. Older Shape binaries do not
understand the new schema. Do not open a migrated project with an older build.

Recoverable desktop drafts use `shape.picture-book-draft/1` in the existing
Artifact Working Graph. Saves compare the exact prior graph digest; accepted
book/source heads and the reviewed draft are checked again at adoption. The
unadopted script is stored as a verified immutable CAS checkpoint object so its
256 KiB bound does not enlarge the 64 KiB graph-configuration bound. It is not an
accepted text revision. Accepting a book atomically rebases its saved draft to the
new accepted head. Merely saving the selected page/language does not invalidate a
content-identical reviewed book; changed composition or unadopted script does.

Limits: 64 ordered pages, 8 languages, 64 distinct materials, one revision per
material, 256 KiB per script/manifest, 128 MiB total material reads and canvas
axes up to 8192. Desktop reading layout requires axes of at least 256 and text
size 12–96. Saved Artifact names follow the existing 120-byte name bound;
legacy snapshot export retains its 256-byte title bound. Font availability can
differ by machine. This release was checked with installed PingFang SC on macOS.

`shape-domain::{picture_book,picture_book_draft}` owns the portable contracts; store validation owns
durable provenance; `shape-core::project::picture_book::{script,version}` owns
script interpretation and version use cases, and `picture_book::draft` owns
checkpoint/recovery commands. The bridge is a terminal adapter with the
ordinary candidate shelf. `PictureBookController` owns desktop drafts/commands,
and `picture_book_renderer` supplies shared preview/PDF layout.

## Validation boundary

Adjacent tests cover transient review/cancel, stale output and source heads,
repeated edits, reopen, unchanged originals, missing translations, exact returned
material bytes, no-clobber exports and indexed native page settings. The packaged
`shape-desktop-picture-book-smoke` also exercises QML page movement, bilingual
layout, three-page reading PDFs, a four-page inclusive proof and a real QQuickWindow
capture. Its synthetic fixture never edits a user's existing book.

The book UI also checks delayed native/PDF callbacks and a switch between copied
bundles with identical project/book/material identities. Image-generation results
carry the originating canonical bundle in their opaque bridge owner; both streamed
and batch adoption reject a different bundle before changing its candidate shelf.
The editable page prompt follows the same case-insensitive role and numeric-page
mapping as the script interpreter, without adopting the prompt text as a script.

The native interop harness runs the original Storybook model and archive
open/write/reopen code, plus original page-settings/move helpers. This is not a
claim of a completed Storybook GUI publishing round trip. The synthetic book workflow also passes in a visible Cocoa window using Metal,
with an inspected actual screenshot. Native Storybook publication UI, print
imposition, Windows and font portability remain unverified in this slice.
