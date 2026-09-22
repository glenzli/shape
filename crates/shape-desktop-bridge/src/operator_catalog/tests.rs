use super::*;

#[test]
fn text_and_raster_catalogs_expose_only_exercised_compatible_operators() {
    let text = compatible_descriptors(ArtifactKind::TextDocument).collect::<Vec<_>>();
    assert_eq!(text.len(), 3);
    assert_eq!(text[0].type_key, TEXT_CREATE_OPERATOR);
    assert!(text[0].input_data_type.is_empty());
    let text = &text[1..];
    assert_eq!(text[0].type_key, TEXT_EDIT_OPERATOR);
    assert_eq!(text[1].type_key, AUDIO_SPEECH_OPERATOR);
    assert_eq!(text[1].output_data_type, AUDIO_CLIP_DATA);
    assert_eq!(
        descriptor_for(ArtifactKind::TextDocument, TEXT_TRANSFORM_OPERATOR)
            .expect("legacy AI text entry aliases the Writing workspace")
            .type_key,
        TEXT_EDIT_OPERATOR
    );

    let raster = compatible_descriptors(ArtifactKind::ImageRaster).collect::<Vec<_>>();
    assert_eq!(raster.len(), 3);
    let raster = &raster[1..];
    assert_eq!(raster[0].type_key, IMAGE_CROP_OPERATOR);
    assert_eq!(raster[0].input_data_type, IMAGE_RASTER_DATA);
    assert_eq!(raster[1].type_key, IMAGE_RESIZE_OPERATOR);
    assert_eq!(raster[1].output_data_type, IMAGE_RASTER_DATA);
}

#[test]
fn unsupported_families_and_unknown_types_fail_closed() {
    assert_eq!(
        compatible_descriptors(ArtifactKind::AudioClip)
            .map(|d| d.type_key)
            .collect::<Vec<_>>(),
        [TEXT_CREATE_OPERATOR]
    );
    assert!(descriptor_for(ArtifactKind::TextDocument, "image.crop").is_err());
    assert!(descriptor_for(ArtifactKind::ImageRaster, "image.unknown").is_err());
}
