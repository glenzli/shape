use image::{ColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ContentRef, ImageColorProfile, ImageDropShadowOperator,
    ImageRasterContract, RasterDropShadow, RasterShadowColor, TransformationId,
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
    offset_x: i32,
    offset_y: i32,
    radius: u16,
    color: RasterShadowColor,
) -> PreparedImageDropShadow {
    ImageDropShadowOperator::prepare(
        RasterDropShadow::new(offset_x, offset_y, radius, color).unwrap(),
        &ImageRasterContract::rgba8(width, height, ImageColorProfile::Srgb).unwrap(),
    )
    .unwrap()
}

fn request(image: &RgbaImage) -> ExecutionRequest {
    let bytes = png(image);
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_DROP_SHADOW_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn executor_binds_one_prepared_versioned_capability() {
    let executor = RasterDropShadowExecutor::new(prepared(
        3,
        2,
        1,
        2,
        3,
        RasterShadowColor::new(0, 0, 0, 128).unwrap(),
    ))
    .unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.drop-shadow");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_DROP_SHADOW_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_DROP_SHADOW_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_BLUR_CAPABILITY).unwrap()));
}

#[test]
fn hard_shadow_expands_canvas_and_composites_source_over_shadow() {
    let image = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
    let executor = RasterDropShadowExecutor::new(prepared(
        1,
        1,
        1,
        0,
        0,
        RasterShadowColor::new(0, 0, 0, 128).unwrap(),
    ))
    .unwrap();
    let output = executor.execute(&request(&image)).unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (2, 1));
    assert_eq!(decoded.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(decoded.get_pixel(1, 0).0, [0, 0, 0, 128]);
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("drop-shadow output contract missing");
    };
    assert_eq!(
        contract,
        ImageRasterContract::rgba8(2, 1, ImageColorProfile::Srgb).unwrap()
    );
}

#[test]
fn fully_transparent_hidden_color_casts_no_shadow() {
    let image = RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 0]));
    let executor = RasterDropShadowExecutor::new(prepared(
        1,
        1,
        1,
        0,
        1,
        RasterShadowColor::new(0, 0, 255, 255).unwrap(),
    ))
    .unwrap();
    let output = executor.execute(&request(&image)).unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert!(decoded.pixels().all(|pixel| pixel.0 == [0, 0, 0, 0]));
}

#[test]
fn source_contract_and_serialized_plan_mismatches_fail_closed() {
    let color = RasterShadowColor::new(0, 0, 0, 128).unwrap();
    let executor = RasterDropShadowExecutor::new(prepared(2, 2, 1, 1, 1, color)).unwrap();
    let wrong = RgbaImage::from_pixel(3, 2, Rgba([0, 0, 0, 255]));
    assert_eq!(
        executor.execute(&request(&wrong)).unwrap_err().code,
        CONTRACT_MISMATCH_CODE
    );

    let image = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 255]));
    let bytes = png(&image);
    let serialized = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_DROP_SHADOW_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        br#"{"offset_x":1}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&serialized).unwrap_err().code,
        INVALID_PLAN_CODE
    );
}
