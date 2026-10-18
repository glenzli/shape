# Shape application icon

The current artwork was edited with **built-in imagegen** on 2026-10-19.
`app-icon-source.png` is the byte-identical selected output
`exec-6a59ef86-7384-48f1-9262-47b3d4a7616b.png`.

- Source SHA-256: `a4055ceae9a72fbb05828b9dd0151ee51adf8708990a97a768b210f876cf3bdd`
- Design: a frontal square frame and circular opening represent creating and transforming
  material. Shallow relief and a restrained pale-blue inner edge retain the Shape identity while
  reducing the previous sculpture-like depth.
- The previous original and its generation prompt remain in Git history.

## Imagegen editing briefs

The approved source followed two imagegen edits:

1. Reduce the three-dimensional appearance: face the frame forward, remove the thick visible side,
   retain only shallow relief and the restrained blue inner edge, and keep the black background.
2. Enlarge only the existing subject to about 80% of the source width, preserving its design.
   Icon Forge's default 824/1024 inset then places the subject at about 64% of the final canvas,
   close to the previous icon's apparent scale. The first-round output was a study, not a shipped
   source.

## Mechanical packaging

**Icon Forge 1.0.0**, revision `e9cd09e88e65390018b59921441843ec80ad3ca2`, generates
all outputs using `--skip-rembg` and its defaults: 1024px desktop canvas, 824px inset artwork,
186px corner radius, and extension sizes 16/32/48/128. Background removal is skipped; the supplied
source is never changed. Forge applies its standard rounded mask and transparent macOS padding.
No extra scaling, crop, filter or postprocessing is applied to the final outputs.

Shape keeps only these application assets:

| Repository file | Exact Forge output |
| --- | --- |
| `app-icon-source.png` | Original selected imagegen PNG, unchanged |
| `app-icon.png` | `rounded-source.png` (1024×1024), for the existing Qt resource contract |
| `app-icon.icns` | `tauri/icon.icns`, with ten standard macOS representations |

Forge also produces extension, ICO, Android and iOS assets in the temporary candidate directory.
Their presence is checked, but they are not additional shipped Shape platform integrations.
`tauri/icon.png` is 512×512; it is intentionally not substituted for Shape's existing 1024px Qt
resource. The ICNS and Qt PNG derive from the same Forge rounded source.

From this asset directory, reproduce with an installed Icon Forge checkout:

```sh
set -eu
SHAPE_ASSETS="$PWD"
ICON_FORGE=/path/to/icon-forge
ICON_OUTPUT=$(mktemp -d)
(cd "$ICON_FORGE" && node forge.js \
  -i "$SHAPE_ASSETS/app-icon-source.png" -o "$ICON_OUTPUT" --skip-rembg)
cp "$ICON_OUTPUT/rounded-source.png" "$SHAPE_ASSETS/app-icon.png"
cp "$ICON_OUTPUT/tauri/icon.icns" "$SHAPE_ASSETS/app-icon.icns"
```

The source, candidate outputs and extracted ICNS entries are checked by hash, dimensions and alpha.
The 1024px rounded source has a 100px transparent inset on every side; its nonzero-alpha bounds are
`(100,100)–(923,923)`. At 16/32px the standard resampling can leave edge alpha of at most 2/255;
this is retained without postprocessing. Small sizes are visually checked at 16, 32, 64 and 128px.
The existing native startup check loads the Qt resource and window icon at those same sizes.
