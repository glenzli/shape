use crate::{
    DomainError, IMAGE_RASTER_CONTRACT_REVISION, ImageColorPrimaries, ImageColorProfile,
    ImageRasterContract, ImageTransferFunction, RasterCrop,
};

#[test]
fn raster_contract_declares_normalized_display_pixels() {
    let contract = ImageRasterContract::rgba8(1920, 1080, ImageColorProfile::Srgb).unwrap();
    assert_eq!(contract.schema_revision, IMAGE_RASTER_CONTRACT_REVISION);
    assert_eq!(contract.bit_depth, 8);
    assert_eq!(contract.color_primaries, ImageColorPrimaries::Srgb);
    assert_eq!(contract.transfer_function, ImageTransferFunction::Srgb);
    assert!(!contract.premultiplied);
}

#[test]
fn crop_rejects_empty_overflowing_and_out_of_bounds_regions() {
    assert_eq!(
        RasterCrop::new(0, 0, 0, 10, 100, 100).unwrap_err(),
        DomainError::InvalidRasterCrop {
            x: 0,
            y: 0,
            width: 0,
            height: 10,
            source_width: 100,
            source_height: 100,
        }
    );
    assert!(RasterCrop::new(90, 0, 11, 10, 100, 100).is_err());
    assert!(RasterCrop::new(u32::MAX, 0, 2, 10, u32::MAX, 100).is_err());
    assert_eq!(
        RasterCrop::new(10, 20, 30, 40, 100, 100).unwrap(),
        RasterCrop {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        }
    );
}
