use image::{ColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ContentRef, ImageBlurOperator, ImageColorProfile,
    ImageRasterContract, RasterGaussianBlur, TransformationId,
};

use super::*;
use crate::{ExecutionInput, RASTER_CROP_CAPABILITY};

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

fn prepared(width: u32, height: u32, radius: u16) -> PreparedImageBlur {
    ImageBlurOperator::prepare(
        RasterGaussianBlur::new(radius).unwrap(),
        &ImageRasterContract::rgba8(width, height, ImageColorProfile::Srgb).unwrap(),
    )
    .unwrap()
}

fn request(image: &RgbaImage) -> ExecutionRequest {
    let bytes = png(image);
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_BLUR_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn executor_binds_one_prepared_versioned_capability() {
    let executor = RasterBlurExecutor::new(prepared(5, 3, 2)).unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.blur");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_BLUR_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_BLUR_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap()));
}

#[test]
fn blur_is_alpha_aware_and_preserves_dimensions_and_contract() {
    let mut image = RgbaImage::from_pixel(5, 3, Rgba([255, 0, 0, 0]));
    image.put_pixel(2, 1, Rgba([0, 0, 255, 255]));
    let output = RasterBlurExecutor::new(prepared(5, 3, 1))
        .unwrap()
        .execute(&request(&image))
        .unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (5, 3));
    assert!(decoded.pixels().all(|pixel| pixel.0[0] == 0));
    assert!(
        decoded
            .pixels()
            .all(|pixel| pixel.0[2] == 255 || pixel.0[3] == 0)
    );
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("blur output contract missing");
    };
    assert_eq!(
        contract,
        ImageRasterContract::rgba8(5, 3, ImageColorProfile::Srgb).unwrap()
    );
}

#[test]
fn exact_golden_output_is_stable_for_a_small_impulse() {
    let mut image = RgbaImage::from_pixel(3, 1, Rgba([0, 0, 0, 0]));
    image.put_pixel(1, 0, Rgba([255, 255, 255, 255]));
    let output = RasterBlurExecutor::new(prepared(3, 1, 1))
        .unwrap()
        .execute(&request(&image))
        .unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!(
        decoded.pixels().map(|pixel| pixel.0).collect::<Vec<_>>(),
        vec![
            [255, 255, 255, 85],
            [255, 255, 255, 85],
            [255, 255, 255, 85],
        ]
    );
}

#[test]
fn source_contract_and_serialized_plan_mismatches_fail_closed() {
    let executor = RasterBlurExecutor::new(prepared(4, 3, 2)).unwrap();
    let wrong = RgbaImage::from_pixel(3, 3, Rgba([0, 0, 0, 255]));
    assert_eq!(
        executor.execute(&request(&wrong)).unwrap_err().code,
        CONTRACT_MISMATCH_CODE
    );

    let image = RgbaImage::from_pixel(4, 3, Rgba([0, 0, 0, 255]));
    let bytes = png(&image);
    let serialized = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_BLUR_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        br#"{"radius":2}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&serialized).unwrap_err().code,
        INVALID_PLAN_CODE
    );
}
