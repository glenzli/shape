use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ContentRef, ImageColorProfile, ImageRasterContract,
    ImageResizeOperator, RasterResize, RasterResizeAspectPolicy, RasterResizeDimensions,
    RasterResizeResampling, TransformationId,
};

use super::*;
use crate::{ExecutionInput, Executor, RASTER_CROP_CAPABILITY};

fn png(width: u32, height: u32) -> Vec<u8> {
    let pixels: Vec<u8> = (0..width * height)
        .flat_map(|index| {
            let value = u8::try_from(index % 251).unwrap();
            [value, value.wrapping_add(1), value.wrapping_add(2), 127]
        })
        .collect();
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, width, height, ColorType::Rgba8.into())
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

fn prepared(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
    aspect_policy: RasterResizeAspectPolicy,
    resampling: RasterResizeResampling,
) -> PreparedImageResize {
    ImageResizeOperator::prepare(
        RasterResize::new(
            RasterResizeDimensions::new(target_width, target_height).unwrap(),
            aspect_policy,
            resampling,
        ),
        &ImageRasterContract::rgba8(source_width, source_height, ImageColorProfile::Srgb).unwrap(),
    )
    .unwrap()
}

fn request(bytes: Vec<u8>) -> ExecutionRequest {
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_RESIZE_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes, RASTER_MEDIA_TYPE), bytes).unwrap()],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn executor_binds_one_prepared_versioned_capability() {
    let executor = RasterResizeExecutor::new(prepared(
        7,
        5,
        4,
        4,
        RasterResizeAspectPolicy::FitWithin,
        RasterResizeResampling::Lanczos3,
    ))
    .unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.resize");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_RESIZE_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_RESIZE_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap()));
}

#[test]
fn odd_fit_executes_exact_dimensions_and_preserves_raster_contract() {
    let executor = RasterResizeExecutor::new(prepared(
        7,
        5,
        4,
        4,
        RasterResizeAspectPolicy::FitWithin,
        RasterResizeResampling::Lanczos3,
    ))
    .unwrap();
    let output = executor.execute(&request(png(7, 5))).unwrap();
    let decoded = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (4, 3));
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("resize output contract missing");
    };
    assert_eq!(
        contract,
        ImageRasterContract::rgba8(4, 3, ImageColorProfile::Srgb).unwrap()
    );
    assert_eq!(decoded.get_pixel(0, 0).0[3], 127);
}

#[test]
fn source_contract_mismatch_fails_before_resampling() {
    let executor = RasterResizeExecutor::new(prepared(
        5,
        3,
        2,
        2,
        RasterResizeAspectPolicy::Stretch,
        RasterResizeResampling::Triangle,
    ))
    .unwrap();
    let failure = executor.execute(&request(png(4, 3))).unwrap_err();
    assert_eq!(failure.code, CONTRACT_MISMATCH_CODE);
    assert!(!failure.retryable);
}

#[test]
fn identity_unmaterialized_and_serialized_plan_requests_fail_closed() {
    let identity = RasterResizeExecutor::new(prepared(
        4,
        3,
        4,
        3,
        RasterResizeAspectPolicy::Stretch,
        RasterResizeResampling::Nearest,
    ))
    .unwrap();
    assert_eq!(
        identity.execute(&request(png(4, 3))).unwrap_err().code,
        NO_OP_CODE
    );

    let bytes = png(4, 3);
    let reference_only = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(RASTER_RESIZE_CAPABILITY).unwrap(),
        vec![content(&bytes, RASTER_MEDIA_TYPE)],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    let executor = RasterResizeExecutor::new(prepared(
        4,
        3,
        2,
        2,
        RasterResizeAspectPolicy::Stretch,
        RasterResizeResampling::Nearest,
    ))
    .unwrap();
    assert_eq!(
        executor.execute(&reference_only).unwrap_err().code,
        INVALID_INPUT_CODE
    );

    let bytes = png(4, 3);
    let serialized_plan = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_RESIZE_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes, RASTER_MEDIA_TYPE), bytes).unwrap()],
        br#"{"target":{"width":2,"height":2}}"#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&serialized_plan).unwrap_err().code,
        INVALID_PLAN_CODE
    );
}
