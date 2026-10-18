# Legacy specialist state

Card making and printing belong to Deck, book production to Storybook Co-Editor, and scripted
speech production to Vox. Shape retains accepted material history and recoverable old drafts.
Existing schemas, revision identities, objects and source bytes are not removed or rewritten.

The public card-kit channel permits `draft_list`, `draft_load`, exact accepted `read`, material ZIP
`export`, saved `deck_transfer`, and `deck_accepted_transfer` with an explicit accepted source pin.
New/save/discard/preview/accept/print actions report that authoring moved to Deck. A draft read
never opens a new editor checkpoint. Accepted transfer uses exactly that revision's composition and
materials even when a different saved draft exists. It does not create a synthetic persisted graph.
Its existing v1 `origin.draft_digest` field carries the accepted document's exact content digest.

Desktop tracked handoffs have distinct functions: `prepare_deck_handoff` selects the exact saved
draft digest; `prepare_deck_accepted_handoff` selects the exact `WorkingInput` JSON pin. Both open
Shape read-only, recheck the selection before publication, and use the unchanged Deck v1 file
protocol. Continuation into an associated independent Deck project remains a separate choice.

The public picture-book channel permits exact accepted `read`/`export`, `draft_list`, `draft_load`
and `draft_export`. A direct accepted-material composition-to-`.scproj` snapshot remains a narrow
external adapter. Preview/accept/script-adoption actions report that authoring moved to Storybook.
The desktop bridge no longer exposes book proposal or script-adoption functions.

`draft_export` requires `artifact_id`, exact `draft_digest` and a new `.scproj` destination. The
archive retains the complete saved state in `shape-draft.json` with `status: "unaccepted"`.
Unaccepted active-language script bytes are copied exactly into Storybook's editable script;
incomplete syntax is preserved and `unaccepted_script_requires_review_in_storybook` is returned.
The sidecar also preserves page selection, unfinished artwork linkage, original pins and dirty state.
This never creates an accepted text or book revision in Shape. Accepted export reads only the exact
accepted composition and never substitutes the current draft.

Image entries in `project.json.visible_images` are basenames, with the corresponding PNG files at
the ZIP root. Storybook preserves archive paths during extraction, normalizes image identities to
basenames, and serves images from the workspace root. An `images/` ZIP subdirectory would therefore
produce broken images after that normalization. Each page retains its ordered index in its filename,
so repeated artwork has independent page identities; blank-page sentinels and page order remain intact.

The desktop bridge no longer exposes speech begin/configure/import-cue/infer/adopt producer
functions. Its generic operator creation channel explicitly rejects `audio.speech_synthesize`,
and the advertised creation catalog excludes it. Old settings and immutable speech receipts remain
readable; full-text syntax diagnostics remain a read-only utility. [Vox transfer](VOX_TRANSFER.md)
preserves the exact source, unfinished writing and speech choices, cue bytes and old accepted audio.

Core card/book legacy owners still decode and materialize their stable formats; their historical
authoring helpers remain for compatibility fixtures and existing pinned consumers. They are not
reachable through Shape's current user-facing creation/acceptance/print adapters. No change in this
checkout changes Deck's separately pinned Shape dependency.

General text editing keeps legal older format labels, unknown saved fields and authored requirements.
Opening a project does not migrate those choices into prompt text or insert default speech settings.
Transient candidate shelves are not persisted or exported as accepted content.
