# Deterministic raster algorithms

This directory contains Shape's implemented portable raster algorithms. This page is the
integration index; exact numerical behavior and golden outputs remain authoritative in the adjacent
source and tests.

All edits consume one accepted `image.raster`, produce a transient PNG Candidate, and advance
immutable history only after explicit Accept.

| Creative capability | Parameters and result | Main boundary | Entry points |
| --- | --- | --- | --- |
| `image.crop` | Pixel rectangle; smaller raster | Rectangle must remain inside the source | [Domain](../../../shape-domain/src/image_crop.rs) · [Execution](crop.rs) · [Core](../../../shape-core/src/project/image/raster_crop.rs) |
| `image.resize` | Target size, aspect policy, resampling; resized raster | Bounded dimensions and pixel count | [Domain](../../../shape-domain/src/image_resize.rs) · [Execution](resize.rs) · [Core](../../../shape-core/src/project/image/raster_resize.rs) |
| `image.transform` | Quarter turns and flips; rearranged pixels | Canonical normalized raster | [Domain](../../../shape-domain/src/image_transform.rs) · [Execution](transform.rs) · [Core](../../../shape-core/src/project/image/raster_transform.rs) |
| `image.blur` | Radius; same-size raster | sRGB, radius 1–64, bounded working set | [Domain](../../../shape-domain/src/image_blur.rs) · [Execution](blur.rs) · [Core](../../../shape-core/src/project/image/raster_blur.rs) |
| `image.drop_shadow` | Offset, blur radius, RGBA tint; expanded flattened raster | sRGB and bounded expanded canvas | [Domain](../../../shape-domain/src/image_drop_shadow.rs) · [Execution](drop_shadow.rs) · [Core](../../../shape-core/src/project/image/raster_drop_shadow.rs) |
| `image.unsharp_mask` | Radius, amount, threshold; same-size sharpened raster | sRGB and bounded working set | [Domain](../../../shape-domain/src/image_unsharp_mask.rs) · [Execution](unsharp_mask.rs) · [Core](../../../shape-core/src/project/image/raster_unsharp_mask.rs) |

## Adding a consumer

1. Use the typed Domain parameters; do not reconstruct rules in UI code.
2. Call the matching Core `propose_raster_*` use case with the accepted head.
3. Present the returned Candidate for review, then use the existing explicit Accept path.
4. Add the exact Operator route to the consuming bridge/UI and cover it in the packaged smoke test.

The shared [pixel math](pixels.rs) is an internal execution detail, not a public Filter SDK.
Composite/layers/masks, creative color grading, RAW processing, and model-backed editing are outside
this inventory and require their own product contracts.
