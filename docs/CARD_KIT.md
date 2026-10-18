# Card material kits and creation templates

Shape's creation templates route to concrete workspaces: picture books, card material kits,
and accepted-material editing. They do not introduce a second graph engine or execute every step
automatically. The desktop catalog is `shape.workflow-templates/1`; identifiers remain stable while
the desktop owns translated names and guidance.

## Card workflow

A card kit keeps shared brief/style direction and an ordered list of cards. Each card has a stable
`id`, independent title/rules/flavor/type/cost/stats, an artwork prompt, a quantity, and optional
exact accepted artwork/text bindings. Reordering a card preserves its bindings. When `text_material`
is bound, that accepted UTF-8 material supplies the rules and the authored `rules` field must be
empty. An artwork prompt records intent; it does not imply that an image was generated or accepted.
Generate/import/edit a material through its existing workspace, explicitly accept it, then bind
that exact accepted version to the selected card. Replacing one card's pin affects only that card.

Drafts use the existing `ArtifactWorkingGraph` configuration and atomically reserve a `CardKit`
output. Saving requires the last observed `draft_digest`. Unchanged saves preserve that digest;
changed drafts or accepted heads reject stale saves and discards. A draft restores the complete
brief, style, card list, ordering and exact material bindings after reopening. Previewing produces
a transient candidate; only explicit acceptance advances the immutable kit revision. Acceptance
rechecks the exact authored node, target head and all material heads in the store transaction,
and rebases the saved draft to the accepted head. No AI result is automatically bound or accepted.

An incomplete list can be accepted as a reviewable checklist version. Preview reports
`missing_artwork_ids`, `missing_text_ids`, and `overflow_card_ids`; `ready_to_export` requires all
three lists to be empty. A selected card's SVG has separate text and artwork and embeds its PNG,
with no scripts or network resources. Long text is explicitly reported as layout overflow.
Artwork is fitted and centered using its accepted raster dimensions, preserving its proportions
in both Qt and browser SVG renderers without stretching or decoding the PNG a second time.
The preview keeps ordinary English word boundaries and reserves a conservative width per
character (1.25 em for ASCII and common CJK ranges, 2 em for other Unicode). CJK phrases and
oversized tokens fill available line space before continuing; ordinary English words remain whole.
Wide `W`/`M` sequences and mixed Chinese/English copy use the same line budget for preview and export.

## Shared desktop and CLI actions

The desktop calls `session_card_kit_action_json(request_json)` and receives a JSON result (errors
cross CXX as an error result). The CLI opens the same core use case:

```sh
shape-cli card-kit /path/to/cards.shape /path/to/request.json
```

CLI output wraps the result as
`{"schema":"shape.card-kit-result/1","status":"ok","result":{...}}`; errors use status
`error`, a `message`, and exit 1. The action request must be a regular UTF-8 JSON file of at most
96 KiB. Unknown fields are rejected.

Create a complete recoverable draft; defaults are 750 × 1050 pixels and quantity 1:

```json
{
  "action": "draft_new",
  "composition": {
    "name": "Moon cards",
    "brief": "A small cooperative exploration game",
    "style": "Ink, quiet moonlight, readable silhouettes",
    "canvas_width": 750,
    "canvas_height": 1050,
    "cards": [
      {
        "id": "moon-fox",
        "title": "Moon fox",
        "card_type": "Creature",
        "cost": "2",
        "rules": "Draw one card.",
        "stats": "2 / 3",
        "flavor": "",
        "art_prompt": "A silver fox beneath a crescent moon",
        "quantity": 1,
        "artwork": null,
        "text_material": null
      }
    ]
  }
}
```

The response is `{"draft":{"artifact_id":"<UUID>","expected_head":null,
"draft_digest":"<BLAKE3>","composition":{...}}}`. The digest belongs to this exact output,
head, operator identity and configuration; never synthesize it from the visible fields.

| Action | Additional fields | Result |
| --- | --- | --- |
| `draft_new` | `composition` | `{draft}` |
| `draft_list` | none | `{drafts:[...]}` with complete composition values |
| `draft_load` | `artifact_id` | `{draft}`; reopens a draft or starts one from the current accepted kit |
| `draft_save` | `artifact_id`, `draft_digest`, `composition` | `{draft}` with current digest |
| `draft_discard` | `artifact_id`, `draft_digest` | `{discarded:true}`; preserves accepted revisions |
| `preview` | `artifact_id`, `draft_digest`, optional `card_id` | `{draft,review_digest,preview}` |
| `accept` | `artifact_id`, `draft_digest`, `review_digest` | `{source:{artifact_id,revision_id},draft}` with rebased head/digest |
| `read` | `source`, optional `card_id` | `{source,composition,preview}` for that exact historical version |
| `export` | `source`, `destination` | `shape.card-kit-export/1` receipt |
| `print_plan` | `source`, optional `options` | `{print_plan}` with unique SVGs and physical page geometry |

Save changes before preview. `preview` verifies current accepted source heads and returns the exact
`review_digest` needed by `accept`. The preview object has this shape:

```json
{
  "schema": "shape.card-kit-preview/1",
  "cards": [{"id":"moon-fox","title":"Moon fox","quantity":1,"has_artwork":false,"has_text":true,"artwork":null,"text_material":null,"layout_overflow":false}],
  "selected_card_id": "moon-fox",
  "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\" ...>...</svg>",
  "ready_to_export": false,
  "missing_artwork_ids": ["moon-fox"],
  "missing_text_ids": [],
  "overflow_card_ids": [],
  "stale_inputs": [],
  "output_kind": "card_material_kit",
  "print_ready": false
}
```

Each material binding is `{artifact_id,revision_id}`. `read` preserves those historical pins even
when a source or the kit has a newer accepted head. It reports `stale_inputs`; it never silently
updates them. Accepting a composition requires explicitly choosing current material versions.

## Export and bounds

Export requires a new `.zip` file outside the Shape project. Publication is atomic and refuses to
overwrite an existing destination. The archive contains `manifest.json`, `README.txt`, a local
`index.html`, a self-contained SVG for every card, and exact verified PNG/text source bytes under
`materials/`. Fixed generated paths use content identities or card positions; authored names never
become paths. All rendered text and attributes are escaped. The manifest retains stable card ids,
quantities, direction, exact artifact/revision pins and source content identities.

This is a card **material kit**, not print imposition, bleed/trim/PDF production, a game engine,
or proof of generated artwork. The brief/style guide authoring; this first renderer uses one
neutral readable frame. Inspect the exported HTML/SVG before any further production process.

The contract admits 1–64 cards, at most 64 distinct materials (one revision per material), 60 KiB
of composition JSON, a 240–2400 pixel canvas on either axis, and quantity 1–999. Accepted images
are canonical Shape PNGs of at most 20 MiB each; rules materials are at most 8 KiB UTF-8. Total
unique input bytes are limited to 128 MiB. Exported self-contained previews admit at most 96 MiB
of artwork counted once per card, including repeated artwork; split larger kits or bind smaller
accepted images. Project schema `20261018.1` additively recognizes CardKit without adding tables;
older `20261004.1` projects retain their accepted content and draft configurations on upgrade.

## Home-print PDF and crop marks

Home printing is a separate export path. The desktop consumes one Core `shape.card-print-plan/1`
for its page preview and PDF writer; neither adapter recomputes the layout. It reads an exact
accepted kit version, including historical material pins, and honors quantities in authored card
order. Print options are transient export settings: planning or writing a PDF never saves a draft,
accepts a candidate, or advances an accepted head. The `.zip` material package remains separate
and does not gain imposed pages.

The existing desktop/CLI JSON channel accepts:

```json
{
  "action": "print_plan",
  "source": {"artifact_id": "<UUID>", "revision_id": "<UUID>"},
  "options": {
    "paper": "a4",
    "card_width_mm": 63,
    "card_height_mm": 88,
    "margin_mm": 5,
    "gap_mm": 3,
    "crop_mark_offset_mm": 0.5,
    "crop_mark_length_mm": 1.5,
    "stroke_width_mm": 0.2
  }
}
```

All options may be omitted. Default A4 is 210 × 297 mm, with 3 columns × 3 rows of 63 × 88 mm
cards. The trim grid is 195 × 270 mm; its conservative full-stroke mark envelope is
199.2 × 274.2 mm, leaving at least the requested 5 mm outer margin. Letter is 215.9 × 279.4 mm
and fits 3 × 2 at these settings. A partial last page contains only its actual copies and marks;
an exact multiple never creates an empty trailing page.

| Setting | Admitted range in mm |
| --- | --- |
| Card width / height | 10–300; the card and readable text must also fit |
| Minimum outer margin | 0–50 |
| Inter-card gap | 0–30, additionally at least offset + mark length + rounded-up half stroke |
| Crop offset | 0–10, additionally at least half stroke |
| Crop length | 0.1–10 |
| Stroke width | 0.05–0.5 |

Values have at most 0.001 mm precision. Only `a4` and `letter` paper are supported; unknown fields
such as bleed, rotation policies, backs or duplex options are rejected. Trim and mark placement
uses integer micrometres. Marks are black, use **butt** caps, stay entirely inside the configured
paper margin including half the stroke, and never cross this or any neighboring card's trim area.
Opposing marks can overlap in a gap. Margins are the user's selected blank paper border; print
the PDF at actual size on a printer that supports that border.

The result's `print_plan` carries `source`, `name`, normalized/defaulted `options`, `units:"mm"`,
`line_cap:"butt"`, `sheet_width_mm`, `sheet_height_mm`, `columns`, `rows`, `slots_per_page`,
`total_copies`, and `page_count`. Its payload is:

- `cards:[{id,title,svg}]`: one self-contained SVG per stable card id, regardless of quantity.
- `pages:[{index,slots,crop_marks}]`: zero-based page index, stable row-major positions.
- Each slot has `card_id`, a one-based per-card `copy_index`, `rect_mm:{x,y,width,height}` for its
  white trim rectangle and `content_rect_mm` with the same shape for proportional SVG placement.
- Each crop mark is `{x1,y1,x2,y2}` in mm; paint it with `options.stroke_width_mm` and butt caps.

The fixed 750:1050 logical card frame is contained and centered inside each trim rectangle.
Its accepted artwork keeps its own proportions. The printing SVG uses physical minimum font sizes:
9 pt title, 7 pt rules/type/stats and 6 pt flavor, rounding upward in SVG units at the actual print
scale. It wraps again in fixed text areas and rejects overflow with the stable card id; it never
silently shrinks or omits copy. A longer card can remain valid for the material package while
requiring shorter copy or a larger card for printing.

Before loading artwork bytes or expanding copies, planning admits at most 4,096 total copies,
256 pages and 512 slots per page. These limits apply together (default A4 therefore permits at most
2,304 copies). Artwork is additionally bounded before reading PNGs: 32 MiB encoded bytes and
128 Mi pixels from accepted raster contracts across the unique card SVGs, alongside the existing
input limits. Distinct card SVGs sharing an image count that image again because renderers can
decode each resource separately; repeated copies of one card do not multiply the resource budget.

Stable error prefixes include `card_print.quantity_limit`, `page_limit`, `slot_limit`,
`option_range`, `crop_clearance`, `paper_fit`, `artwork_bytes`, `artwork_pixels`,
`artwork_contract`, `incomplete`, and `text_overflow` (each with `card_print.`). They let the desktop
give a specific suggestion while retaining neutral handling for corrupt or unavailable materials.

The output declares `output_kind:"home_print_layout"`, `commercial_prepress:false` and
`duplex:false`. This is a one-sided home-print PDF with cut lines, without bleed, back registration,
commercial color/prepress guarantees or a printer calibration claim.
