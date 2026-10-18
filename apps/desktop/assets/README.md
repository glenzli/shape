# Shape application icon

Generated with **built-in imagegen** on 2026-10-18. The selected original artwork is
`app-icon-source.png`, preserved byte-for-byte from `exec-af2231be-71d7-427a-85bd-a328fcc3b969.png`.

- Original SHA-256: `9f0949e2059dfbfcf0c1c8c7d10d7414b3522a16e14f7144b9407eec3b7d2e2f`
- Design meaning: A solid square frame with a circular opening represents creating and transforming material. The angular exterior, smooth inner curve and pale blue edge distinguish Shape from the card and audio applications.

## Original generation prompt

```text
Use case: logo-brand. Create one finished macOS application icon for Shape, an app for general AI creation and transformation of creative material.
Square 1024x1024 artwork. Pure deep black background extending edge to edge, no drawn rounded-square app tile. One centered sculptural geometric object occupying about 72% of the canvas, generous negative space.
The object is a thick continuous square frame with a perfectly circular opening, like a small monolithic sculpture whose outside is angular and whose inside is smoothly curved. A softly chamfered square outer boundary, large circular aperture, simple balanced proportions. The frame is seen almost frontally with a slight three-quarter turn so its thickness is subtly visible. It must read as one coherent solid geometric form, never as several nested rings, a letter, or a box containing another icon.
Material and lighting: satin silver-white metal on the upper-left planes, deep graphite on the lower-right sides, one very restrained pale ice-blue luminous edge along the inner curve. Fine subtle grain, crisp geometry, soft controlled gallery lighting. The center opening stays black. Quiet black, ivory and cool silver palette with a tiny touch of ice blue, no colorful gradients or neon wash.
This belongs to the same icon family as Shadow's black-and-champagne geometric planes with a glowing seam and Echo's silver-white sound rings on black. Match their restrained materiality and luminous edges while giving Shape a stronger solid angular silhouette.
No text, letters, numbers, nodes, arrows, workflow diagram, sparkles, AI symbols, wireframes, extra shapes, floor, scenery, platform logo, watermark or decorative border. Sophisticated simple emblem, instantly recognizable at 32px.
```

## Mechanical outputs

`app-icon.png` is 1024×1024. `app-icon.icns` contains the standard macOS 16, 32, 64, 128,
256, 512 and 1024 pixel representations, including the paired Retina entries.
Both derive directly from the opaque RGB original: macOS supplies the native outer presentation,
avoiding a second frame around a precomposed transparent tile. No repainting, cropping, filtering,
masking or background removal is applied. No Windows binary or ICO is claimed.

From this asset directory on macOS, reproduce the outputs with:

```sh
sips --resampleHeightWidth 1024 1024 app-icon-source.png --out app-icon.png
ICON_WORK=$(mktemp -d)
mkdir "$ICON_WORK/app-icon.iconset"
for ICON_ENTRY in 16x16:16 16x16@2x:32 32x32:32 32x32@2x:64 128x128:128 128x128@2x:256 256x256:256 256x256@2x:512 512x512:512 512x512@2x:1024; do
    ICON_LABEL=${ICON_ENTRY%:*}
    ICON_SIZE=${ICON_ENTRY#*:}
    sips --resampleHeightWidth "$ICON_SIZE" "$ICON_SIZE" app-icon.png --out "$ICON_WORK/app-icon.iconset/icon_$ICON_LABEL.png"
done
iconutil --convert icns "$ICON_WORK/app-icon.iconset" --output app-icon.icns
```

The temporary iconset stays outside the repository. These commands never regenerate the original.
