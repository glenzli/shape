use super::*;

#[test]
fn text_and_raster_catalogs_expose_only_exercised_compatible_operators() {
    let text = compatible_descriptors(ArtifactKind::TextDocument).collect::<Vec<_>>();
    assert_eq!(text.len(), 3);
    assert_eq!(text[0].type_key, TEXT_EDIT_OPERATOR);
    assert_eq!(text[1].type_key, TEXT_TRANSFORM_OPERATOR);
    assert_eq!(text[2].type_key, AUDIO_SPEECH_OPERATOR);
    assert_eq!(text[2].output_data_type, AUDIO_CLIP_DATA);

    let raster = compatible_descriptors(ArtifactKind::ImageRaster).collect::<Vec<_>>();
    assert_eq!(raster.len(), 1);
    assert_eq!(raster[0].type_key, IMAGE_CROP_OPERATOR);
    assert_eq!(raster[0].input_data_type, IMAGE_RASTER_DATA);
}

#[test]
fn unsupported_families_and_unknown_types_fail_closed() {
    assert!(
        compatible_descriptors(ArtifactKind::AudioClip)
            .next()
            .is_none()
    );
    assert!(descriptor_for(ArtifactKind::TextDocument, "image.crop").is_err());
    assert!(descriptor_for(ArtifactKind::ImageRaster, "image.unknown").is_err());
}
