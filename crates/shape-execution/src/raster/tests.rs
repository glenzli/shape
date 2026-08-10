use image::{ColorType, ImageEncoder, codecs::jpeg::JpegEncoder, codecs::png::PngEncoder};
use shape_domain::{
    ArtifactContentContract, ArtifactId, IntentSpec, Transformation, TransformationKind,
};

use super::*;
use crate::{ExecutionCoordinator, ExecutionRequest};

fn transformation() -> Transformation {
    let artifact = ArtifactId::new();
    Transformation::new(
        TransformationKind::Import,
        artifact,
        Vec::new(),
        IntentSpec::new("test raster").unwrap(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
}

fn rgba_fixture() -> Vec<u8> {
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 10, 20, 30, 255, 40,
        50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255, 1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255,
        10, 11, 12, 255,
    ];
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, 4, 3, ColorType::Rgba8.into())
        .unwrap();
    bytes
}

#[test]
fn import_normalizes_png_and_declares_contract() {
    let transformation = transformation();
    let request = ExecutionRequest::new(
        transformation.id,
        CapabilityId::new(RASTER_IMPORT_CAPABILITY).unwrap(),
        Vec::new(),
        rgba_fixture(),
        RASTER_MEDIA_TYPE,
    )
    .unwrap();
    let executed =
        ExecutionCoordinator::execute(&RasterExecutor::new().unwrap(), &request).unwrap();
    let Some(ArtifactContentContract::ImageRaster(contract)) = executed.output.content_contract
    else {
        panic!("raster contract missing");
    };
    assert_eq!((contract.width, contract.height), (4, 3));
    assert_eq!(
        image::load_from_memory(&executed.output.bytes)
            .unwrap()
            .width(),
        4
    );
}

#[test]
fn import_accepts_jpeg_and_materializes_canonical_png() {
    let rgb = vec![128_u8; 4 * 3 * 3];
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .write_image(&rgb, 4, 3, ColorType::Rgb8.into())
        .unwrap();
    let transformation = transformation();
    let imported = ExecutionCoordinator::execute(
        &RasterExecutor::new().unwrap(),
        &ExecutionRequest::new(
            transformation.id,
            CapabilityId::new(RASTER_IMPORT_CAPABILITY).unwrap(),
            Vec::new(),
            jpeg,
            RASTER_MEDIA_TYPE,
        )
        .unwrap(),
    )
    .unwrap();
    let image = image::load_from_memory(&imported.output.bytes)
        .unwrap()
        .to_rgba8();
    assert_eq!((image.width(), image.height()), (4, 3));
}

#[test]
fn compatibility_executor_still_routes_the_crop_capability() {
    let executor = RasterExecutor::new().unwrap();
    assert!(executor.supports(&CapabilityId::new(RASTER_IMPORT_CAPABILITY).unwrap()));
    assert!(executor.supports(&CapabilityId::new(RASTER_CROP_CAPABILITY).unwrap()));
}
