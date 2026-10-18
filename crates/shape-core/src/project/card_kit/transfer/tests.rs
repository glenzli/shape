use super::*;
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_domain::{CardKitRequest, WorkingInput};

#[test]
fn transfer_keeps_hidden_materials_legacy_layout_and_incomplete_fields_without_acceptance() {
    let root = std::env::temp_dir().join(format!("shape-deck-transfer-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let mut project = ShapeProject::create(root.join("source.shape"), "Source").unwrap();
    let text = project
        .create_text_document("Text", "hidden original\n未完成")
        .unwrap();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&[20, 40, 60, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    let image = project.import_raster_bytes(png, "Image").unwrap();
    let composition:CardKitRequest=serde_json::from_value(json!({"name":"Unfinished","cards":[{"id":"keep-stable","art_prompt":"not yet generated","text_material":WorkingInput{artifact_id:text.artifact_id,revision_id:text.id},"artwork":WorkingInput{artifact_id:image.artifact_id,revision_id:image.id}}]})).unwrap();
    let draft = project.create_card_kit_draft(&composition).unwrap();
    let before = project.snapshot().unwrap();
    let output=project.card_kit_action_json(&json!({"action":"deck_transfer","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"request_id":"exact-transfer","destination":root.join("transfer")}).to_string()).unwrap();
    let output: Value = serde_json::from_str(&output).unwrap();
    let bytes = fs::read(output["manifest_path"].as_str().unwrap()).unwrap();
    assert_eq!(
        output["manifest_blake3"],
        ContentDigest::from_bytes(&bytes).to_string()
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        manifest["composition"],
        serde_json::to_value(&composition).unwrap()
    );
    assert!(manifest["composition"].get("layout").is_none());
    assert_eq!(manifest["materials"].as_array().unwrap().len(), 2);
    for material in manifest["materials"].as_array().unwrap() {
        let exact = fs::read(
            root.join("transfer")
                .join(material["relative_path"].as_str().unwrap()),
        )
        .unwrap();
        let content: shape_domain::ContentRef =
            serde_json::from_value(material["content"].clone()).unwrap();
        assert_eq!(content.digest, ContentDigest::from_bytes(&exact));
        assert_eq!(content.byte_length, exact.len() as u64);
        if content.media_type.starts_with("text/") {
            assert!(material["contract"].is_null());
        }
    }
    assert_eq!(before, project.snapshot().unwrap());
    assert!(
        project
            .export_deck_card_transfer(
                draft.artifact_id,
                &draft.draft_digest,
                "retry",
                &root.join("transfer")
            )
            .is_err()
    );
    let mut changed = composition;
    changed.brief = "changed".into();
    project
        .save_card_kit_draft(draft.artifact_id, &draft.draft_digest, &changed)
        .unwrap();
    assert!(
        project
            .export_deck_card_transfer(
                draft.artifact_id,
                &draft.draft_digest,
                "stale",
                &root.join("stale")
            )
            .is_err()
    );
    assert!(!root.join("stale").exists());
    fs::remove_dir_all(root).unwrap();
}
