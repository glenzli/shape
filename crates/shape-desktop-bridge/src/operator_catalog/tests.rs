use super::*;

#[test]
fn text_and_raster_catalogs_expose_only_exercised_compatible_operators() {
    let text = compatible_descriptors(ArtifactKind::TextDocument).collect::<Vec<_>>();
    for key in [
        "text.translate",
        "text.summarize",
        "text.polish",
        "text.expand",
        "text.outline",
        "text.prepare_script",
    ] {
        let descriptor = text.iter().find(|d| d.type_key == key).unwrap();
        assert_eq!(descriptor.input_data_type, TEXT_DOCUMENT_DATA);
        assert_eq!(descriptor.output_data_type, TEXT_DOCUMENT_DATA);
        assert!(text_authoring::node_preset(key).unwrap().is_some());
        assert!(descriptor_for(ArtifactKind::ImageRaster, key).is_err());
    }
    assert_eq!(
        descriptor_for(ArtifactKind::TextDocument, TEXT_TRANSFORM_OPERATOR)
            .unwrap()
            .type_key,
        TEXT_EDIT_OPERATOR
    );
    assert_eq!(
        descriptor_for(ArtifactKind::TextDocument, AUDIO_SPEECH_OPERATOR)
            .unwrap()
            .output_data_type,
        AUDIO_CLIP_DATA
    );
    assert_eq!(
        descriptor_for(ArtifactKind::ImageRaster, IMAGE_CROP_OPERATOR)
            .unwrap()
            .input_data_type,
        IMAGE_RASTER_DATA
    );
    assert_eq!(
        descriptor_for(ArtifactKind::ImageRaster, IMAGE_RESIZE_OPERATOR)
            .unwrap()
            .output_data_type,
        IMAGE_RASTER_DATA
    );
    let image = source_descriptors()
        .find(|d| d.type_key == IMAGE_GENERATE_OPERATOR)
        .unwrap();
    assert!(image.input_data_type.is_empty());
    assert_eq!(image.output_data_type, IMAGE_RASTER_DATA);
}

#[test]
fn unsupported_families_and_unknown_types_fail_closed() {
    assert_eq!(
        compatible_descriptors(ArtifactKind::AudioClip)
            .map(|d| d.type_key)
            .collect::<Vec<_>>(),
        [TEXT_CREATE_OPERATOR, IMAGE_GENERATE_OPERATOR]
    );
    assert!(descriptor_for(ArtifactKind::TextDocument, "image.crop").is_err());
    assert!(descriptor_for(ArtifactKind::ImageRaster, "image.unknown").is_err());
}
