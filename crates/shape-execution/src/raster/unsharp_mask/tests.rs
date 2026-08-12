use std::time::Instant;

use image::{ColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ContentRef, ImageColorProfile, ImageRasterContract,
    ImageUnsharpMaskOperator, RasterUnsharpMask, TransformationId,
};

use super::*;
use crate::{ExecutionInput, RASTER_BLUR_CAPABILITY};

fn png(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .unwrap();
    bytes
}

fn content(bytes: &[u8]) -> ContentRef {
    ContentRef::new(
        ContentDigest::from_bytes(bytes),
        RASTER_MEDIA_TYPE,
        bytes.len() as u64,
    )
    .unwrap()
}

fn prepared(
    width: u32,
    height: u32,
    radius: u16,
    amount: u16,
    threshold: u8,
) -> PreparedImageUnsharpMask {
    ImageUnsharpMaskOperator::prepare(
        RasterUnsharpMask::new(radius, amount, threshold).unwrap(),
        &ImageRasterContract::rgba8(width, height, ImageColorProfile::Srgb).unwrap(),
    )
    .unwrap()
}

fn request(image: &RgbaImage) -> ExecutionRequest {
    let bytes = png(image);
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_UNSHARP_MASK_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn executor_binds_one_prepared_versioned_capability() {
    let executor = RasterUnsharpMaskExecutor::new(prepared(5, 3, 2, 1_000, 0)).unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.unsharp-mask");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_UNSHARP_MASK_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_UNSHARP_MASK_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_BLUR_CAPABILITY).unwrap()));
}

#[test]
fn exact_golden_output_preserves_alpha_and_never_reveals_hidden_color() {
    let mut image = RgbaImage::from_pixel(5, 1, Rgba([255, 0, 0, 0]));
    image.put_pixel(1, 0, Rgba([64, 64, 64, 128]));
    image.put_pixel(2, 0, Rgba([160, 160, 160, 255]));
    image.put_pixel(3, 0, Rgba([64, 64, 64, 128]));
    let output = RasterUnsharpMaskExecutor::new(prepared(5, 1, 1, 1_000, 0))
        .unwrap()
        .execute(&request(&image))
        .unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!(
        decoded.pixels().map(|pixel| pixel.0).collect::<Vec<_>>(),
        vec![
            [0, 0, 0, 0],
            [0, 0, 0, 128],
            [203, 203, 203, 255],
            [0, 0, 0, 128],
            [0, 0, 0, 0],
        ]
    );
    assert_eq!(
        decoded.pixels().map(|pixel| pixel.0[3]).collect::<Vec<_>>(),
        image.pixels().map(|pixel| pixel.0[3]).collect::<Vec<_>>()
    );
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("unsharp-mask output contract missing");
    };
    assert_eq!(
        contract,
        ImageRasterContract::rgba8(5, 1, ImageColorProfile::Srgb).unwrap()
    );
}

#[test]
fn threshold_suppresses_differences_at_or_below_its_linear_distance() {
    let original = super::super::pixels::LinearPremultiplied([20_000, 20_000, 20_000, u16::MAX]);
    let blurred = super::super::pixels::LinearPremultiplied([19_743, 20_257, 20_000, u16::MAX]);
    assert_eq!(sharpen(original, blurred, 4_000, 1), original);
    assert_ne!(sharpen(original, blurred, 4_000, 0), original);
}

#[test]
fn source_contract_and_serialized_plan_mismatches_fail_closed() {
    let executor = RasterUnsharpMaskExecutor::new(prepared(4, 3, 2, 1_000, 0)).unwrap();
    let wrong = RgbaImage::from_pixel(3, 3, Rgba([0, 0, 0, 255]));
    assert_eq!(
        executor.execute(&request(&wrong)).unwrap_err().code,
        CONTRACT_MISMATCH_CODE
    );

    let image = RgbaImage::from_pixel(4, 3, Rgba([0, 0, 0, 255]));
    let bytes = png(&image);
    let serialized = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_UNSHARP_MASK_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        br#"{"radius":2,"amount_milli":1000,"threshold":0}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&serialized).unwrap_err().code,
        INVALID_PLAN_CODE
    );
}

#[test]
#[ignore = "manual 4K end-to-end performance and resident-memory probe"]
fn four_k_end_to_end_performance_probe() {
    let width = 3840;
    let height = 2160;
    let image = RgbaImage::from_fn(width, height, |x, y| {
        let value = u8::try_from((x ^ y) & 255).unwrap();
        Rgba([value, value.wrapping_add(37), value.wrapping_add(91), 255])
    });
    let executor = RasterUnsharpMaskExecutor::new(prepared(width, height, 16, 1_000, 4)).unwrap();
    let started = Instant::now();
    let output = executor.execute(&request(&image)).unwrap();
    let elapsed = started.elapsed();
    let planned_bytes =
        u64::from(width) * u64::from(height) * RASTER_UNSHARP_MASK_WORKING_BYTES_PER_PIXEL;
    eprintln!(
        "4K unsharp mask: {:.3}s, planned working buffers: {} bytes, PNG: {} bytes",
        elapsed.as_secs_f64(),
        planned_bytes,
        output.bytes.len()
    );
    assert_eq!(
        output.content_contract,
        Some(ArtifactContentContract::ImageRaster(
            ImageRasterContract::rgba8(width, height, ImageColorProfile::Srgb).unwrap()
        ))
    );
}
