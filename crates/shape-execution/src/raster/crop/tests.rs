use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_domain::{ContentDigest, ContentRef, TransformationId};

use super::*;
use crate::{ExecutionInput, Executor, RASTER_IMPORT_CAPABILITY};

fn png(color_type: ColorType) -> Vec<u8> {
    let channels = usize::from(color_type.channel_count());
    let pixels = vec![127_u8; 4 * 3 * channels];
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, 4, 3, color_type.into())
        .unwrap();
    bytes
}

fn content(bytes: &[u8], media_type: &str) -> ContentRef {
    ContentRef::new(
        ContentDigest::from_bytes(bytes),
        media_type,
        bytes.len() as u64,
    )
    .unwrap()
}

fn request(
    inputs: Vec<ExecutionInput>,
    instruction: Vec<u8>,
    output_media_type: &str,
) -> ExecutionRequest {
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap(),
        inputs,
        instruction,
        output_media_type,
    )
    .unwrap()
}

fn valid_request() -> ExecutionRequest {
    let bytes = png(ColorType::Rgba8);
    request(
        vec![ExecutionInput::materialized(content(&bytes, RASTER_MEDIA_TYPE), bytes).unwrap()],
        serde_json::to_vec(&RasterCrop::new(1, 1, 2, 2, 4, 3).unwrap()).unwrap(),
        RASTER_MEDIA_TYPE,
    )
}

#[test]
fn executor_has_a_single_versioned_physical_capability() {
    let executor = RasterCropExecutor::new().unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.crop");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_CROP_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_IMPORT_CAPABILITY).unwrap()));
}

#[test]
fn parameter_json_is_exact_and_rejects_unknown_fields() {
    assert_eq!(
        serde_json::to_string(&RasterCrop::new(10, 20, 30, 40, 100, 100).unwrap()).unwrap(),
        r#"{"x":10,"y":20,"width":30,"height":40}"#
    );
    assert!(
        serde_json::from_str::<RasterCrop>(
            r#"{"x":10,"y":20,"width":30,"height":40,"future":true}"#
        )
        .is_err()
    );
}

#[test]
fn crop_executes_exact_pixels_and_contract() {
    let output = RasterCropExecutor::new()
        .unwrap()
        .execute(&valid_request())
        .unwrap();
    let image = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!((image.width(), image.height()), (2, 2));
    assert_eq!(image.get_pixel(0, 0).0, [127, 127, 127, 127]);
    let Some(shape_domain::ArtifactContentContract::ImageRaster(contract)) =
        output.content_contract
    else {
        panic!("crop output contract missing");
    };
    assert_eq!((contract.width, contract.height), (2, 2));
    assert_eq!(
        contract,
        shape_domain::ImageRasterContract::rgba8(2, 2, shape_domain::ImageColorProfile::Srgb)
            .unwrap()
    );
}

#[test]
fn failure_codes_freeze_input_parameter_and_boundary_semantics() {
    let executor = RasterCropExecutor::new().unwrap();
    let no_input = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap(),
        Vec::new(),
        b"{}".to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    let failure = executor.execute(&no_input).unwrap_err();
    assert_eq!(failure.code, INVALID_INPUT_CODE);
    assert!(!failure.retryable);

    let bytes = png(ColorType::Rgba8);
    let malformed = request(
        vec![ExecutionInput::materialized(content(&bytes, RASTER_MEDIA_TYPE), bytes).unwrap()],
        br#"{"x":0,"y":0,"width":1,"height":1,"future":true}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    );
    assert_eq!(
        executor.execute(&malformed).unwrap_err().code,
        INVALID_PARAMETERS_CODE
    );

    let bytes = png(ColorType::Rgba8);
    let out_of_bounds = request(
        vec![ExecutionInput::materialized(content(&bytes, RASTER_MEDIA_TYPE), bytes).unwrap()],
        br#"{"x":3,"y":2,"width":2,"height":2}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    );
    assert_eq!(
        executor.execute(&out_of_bounds).unwrap_err().code,
        OUT_OF_BOUNDS_CODE
    );
}

#[test]
fn crop_rejects_noncanonical_or_unmaterialized_input() {
    let executor = RasterCropExecutor::new().unwrap();
    let rgb = png(ColorType::Rgb8);
    let noncanonical = request(
        vec![ExecutionInput::materialized(content(&rgb, RASTER_MEDIA_TYPE), rgb).unwrap()],
        br#"{"x":0,"y":0,"width":2,"height":2}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    );
    assert_eq!(
        executor.execute(&noncanonical).unwrap_err().code,
        INVALID_INPUT_CODE
    );

    let rgba = png(ColorType::Rgba8);
    let reference_only = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap(),
        vec![content(&rgba, RASTER_MEDIA_TYPE)],
        br#"{"x":0,"y":0,"width":2,"height":2}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&reference_only).unwrap_err().code,
        INVALID_INPUT_CODE
    );
}
