use image::{ColorType, ImageEncoder, RgbaImage, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ContentRef, ImageColorProfile, ImageRasterContract,
    ImageTransformOperator, RasterTransform, TransformationId,
};

use super::*;
use crate::{ExecutionInput, RASTER_CROP_CAPABILITY};

fn png() -> Vec<u8> {
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255, 0, 255,
        255, 255,
    ];
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, 3, 2, ColorType::Rgba8.into())
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

fn prepared(transform: RasterTransform) -> PreparedImageTransform {
    ImageTransformOperator::prepare(
        transform,
        &ImageRasterContract::rgba8(3, 2, ImageColorProfile::Srgb).unwrap(),
    )
    .unwrap()
}

fn request(bytes: Vec<u8>) -> ExecutionRequest {
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_TRANSFORM_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        Vec::new(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn executor_binds_one_prepared_versioned_capability() {
    let executor =
        RasterTransformExecutor::new(prepared(RasterTransform::Rotate90Clockwise)).unwrap();
    assert_eq!(executor.identity().id, "shape.builtin.raster.transform");
    assert_eq!(
        executor.identity().contract_revision,
        IMAGE_TRANSFORM_PARAMETERS_REVISION
    );
    assert!(executor.supports(&CapabilityId::new(RASTER_TRANSFORM_CAPABILITY).unwrap()));
    assert!(!executor.supports(&CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap()));
}

#[test]
fn clockwise_rotation_executes_exact_pixels_and_contract() {
    let output = RasterTransformExecutor::new(prepared(RasterTransform::Rotate90Clockwise))
        .unwrap()
        .execute(&request(png()))
        .unwrap();
    let decoded: RgbaImage = image::load_from_memory(&output.bytes).unwrap().to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (2, 3));
    assert_eq!(decoded.get_pixel(0, 0).0, [255, 255, 0, 255]);
    assert_eq!(decoded.get_pixel(1, 0).0, [255, 0, 0, 255]);
    assert_eq!(decoded.get_pixel(0, 2).0, [0, 255, 255, 255]);
    assert_eq!(decoded.get_pixel(1, 2).0, [0, 0, 255, 255]);
    let Some(ArtifactContentContract::ImageRaster(contract)) = output.content_contract else {
        panic!("transform output contract missing");
    };
    assert_eq!(
        contract,
        ImageRasterContract::rgba8(2, 3, ImageColorProfile::Srgb).unwrap()
    );
}

#[test]
fn source_contract_and_serialized_plan_mismatches_fail_closed() {
    let executor = RasterTransformExecutor::new(prepared(RasterTransform::FlipVertical)).unwrap();
    let wrong_source = {
        let bytes = {
            let mut encoded = Vec::new();
            PngEncoder::new(&mut encoded)
                .write_image(&[0_u8; 16], 2, 2, ColorType::Rgba8.into())
                .unwrap();
            encoded
        };
        request(bytes)
    };
    assert_eq!(
        executor.execute(&wrong_source).unwrap_err().code,
        CONTRACT_MISMATCH_CODE
    );

    let bytes = png();
    let serialized = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(RASTER_TRANSFORM_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content(&bytes), bytes).unwrap()],
        br#""rotate180""#.to_vec(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    assert_eq!(
        executor.execute(&serialized).unwrap_err().code,
        INVALID_PLAN_CODE
    );
}
