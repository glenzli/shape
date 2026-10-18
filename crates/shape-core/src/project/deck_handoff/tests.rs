use super::*;
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::Value;

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("shape-deck-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn task(&self) -> std::path::PathBuf {
        let path = self.0.join("task.json");
        fs::write(&path,json!({"schema":"deck.material-task/1","request_id":"task-1","target":{"project_id":"deck-project","kit_id":"deck-kit","card_id":"stable-card","draft_digest":"a".repeat(64),"artwork":null},"prompt":"A tree","canvas":{"width":750,"height":1050}}).to_string()).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn digest(path: &Path) -> String {
    ContentDigest::from_bytes(&fs::read(path).unwrap()).to_string()
}

#[test]
fn opaque_foreign_card_snapshot_is_preserved_and_bound_to_the_target() {
    let fixture = Fixture::new();
    let task = fixture.task();
    let result = fixture.0.join("result.json");
    let mut request: Value = serde_json::from_slice(&fs::read(&task).unwrap()).unwrap();
    request["target"]["card_snapshot"] =
        json!({"id":"stable-card","title":"Pending text","artwork":null});
    fs::write(&task, request.to_string()).unwrap();
    let read: Value =
        serde_json::from_str(&ShapeProject::read_deck_material_task(&task, &result).unwrap())
            .unwrap();
    assert_eq!(
        read["request"]["target"]["card_snapshot"],
        request["target"]["card_snapshot"]
    );
    request["target"]["card_snapshot"]["id"] = json!("other-card");
    fs::write(&task, request.to_string()).unwrap();
    assert!(ShapeProject::read_deck_material_task(&task, &result).is_err());
    assert!(!result.exists());
}

#[test]
fn image_task_requires_explicit_current_accepted_image_and_never_overwrites_result() {
    let fixture = Fixture::new();
    let task = fixture.task();
    let result = fixture.0.join("result.json");
    let mut project = ShapeProject::create(fixture.0.join("task.shape"), "Task").unwrap();
    assert!(ShapeProject::read_deck_material_task(&task, &result).is_ok());
    let text = project
        .create_text_document("Text", "not an image")
        .unwrap();
    assert!(
        project
            .return_deck_material(
                &task,
                &result,
                WorkingInput {
                    artifact_id: text.artifact_id,
                    revision_id: text.id
                },
                &digest(&task),
            )
            .is_err()
    );
    assert!(!result.exists());
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&[20, 40, 60, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    let image = project.import_raster_bytes(png, "Accepted").unwrap();
    let source = WorkingInput {
        artifact_id: image.artifact_id,
        revision_id: image.id,
    };
    let returned: Value = serde_json::from_str(
        &project
            .return_deck_material(&task, &result, source, &digest(&task))
            .unwrap(),
    )
    .unwrap();
    let receipt = &returned["result"];
    assert_eq!(
        receipt["request_blake3"],
        ContentDigest::from_bytes(&fs::read(&task).unwrap()).to_string()
    );
    let output = fs::read(receipt["output"]["path"].as_str().unwrap()).unwrap();
    assert_eq!(
        receipt["output"]["blake3"],
        ContentDigest::from_bytes(&output).to_string()
    );
    assert_eq!(
        output,
        project.read_revision_content(image.id).unwrap().bytes
    );
    let exact = fs::read(&result).unwrap();
    assert!(
        ShapeProject::finish_deck_material_task(&task, &result, &digest(&task), "cancelled", "")
            .is_err()
    );
    assert_eq!(exact, fs::read(&result).unwrap());
    assert!(ShapeProject::read_deck_material_task(&task, &result).is_err());
}

#[test]
fn cancelled_task_is_terminal_and_bad_request_does_not_create_result() {
    let fixture = Fixture::new();
    let task = fixture.task();
    let result = fixture.0.join("cancelled.json");
    let value: Value = serde_json::from_str(
        &ShapeProject::finish_deck_material_task(&task, &result, &digest(&task), "cancelled", "")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["result"]["outcome"], "cancelled");
    assert!(value["result"]["output"].is_null());
    let mut request: Value = serde_json::from_slice(&fs::read(&task).unwrap()).unwrap();
    request["reference_image"] = json!("unsupported");
    fs::write(&task, request.to_string()).unwrap();
    assert!(
        ShapeProject::finish_deck_material_task(
            &task,
            &fixture.0.join("bad.json"),
            &digest(&task),
            "failed",
            "unsupported"
        )
        .is_err()
    );
    assert!(!fixture.0.join("bad.json").exists());
}

#[test]
fn deck_outputs_bind_exact_request_and_recheck_file_bytes_without_mutating_shape() {
    let fixture = Fixture::new();
    let mut project = ShapeProject::create(fixture.0.join("source.shape"), "Source").unwrap();
    let composition =
        serde_json::from_value(json!({"name":"Kit","cards":[{"id":"one","title":"ONE"}]})).unwrap();
    let draft = project.create_card_kit_draft(&composition).unwrap();
    let prepared: Value = serde_json::from_str(
        &project
            .prepare_deck_handoff(
                draft.artifact_id,
                &draft.draft_digest,
                "edit-1",
                &fixture.0.join("edit"),
                "",
            )
            .unwrap(),
    )
    .unwrap();
    let request = Path::new(prepared["request_path"].as_str().unwrap());
    let result = Path::new(prepared["result_path"].as_str().unwrap());
    let expected_digest = prepared["request_blake3"].as_str().unwrap();
    let document = fixture.0.join("local.deck");
    let mut deck = ShapeProject::create(&document, "Independent Deck").unwrap();
    let local = deck.create_card_kit_draft(&composition).unwrap();
    let candidate = deck
        .propose_card_kit(local.artifact_id, &local.draft_digest)
        .unwrap();
    let accepted = deck.accept_card_kit(candidate).unwrap();
    let document_id = deck.snapshot().unwrap().metadata.id;
    fs::write(document.join("deck-transfer.json"),json!({"schema":"deck.transfer-import/1","request_id":"edit-1","manifest_blake3":prepared["request"]["input"]["manifest_blake3"],"origin":prepared["request"]["origin"],"project_id":document_id,"kit_id":local.artifact_id}).to_string()).unwrap();
    let pdf = fixture.0.join("output.pdf");
    let bytes = b"%PDF-1.7\nfixture\n";
    fs::write(&pdf, bytes).unwrap();
    let receipt = json!({"schema":"deck.external-edit-result/1","request_id":"edit-1","request_blake3":prepared["request_blake3"],"outcome":"completed","document":{"project_path":document,"project_id":document_id,"kit_id":local.artifact_id,"accepted_revision_id":accepted.id},"outputs":[{"kind":"print_pdf","path":pdf,"byte_length":bytes.len(),"blake3":ContentDigest::from_bytes(bytes).to_string(),"media_type":"application/pdf"}],"error":null});
    fs::write(result, receipt.to_string()).unwrap();
    let before = project.snapshot().unwrap();
    assert!(ShapeProject::verify_deck_output_return(request, result, expected_digest).is_ok());
    assert_eq!(before, project.snapshot().unwrap());
    fs::write(&pdf, b"%PDF-1.7\nchanged\n").unwrap();
    assert!(ShapeProject::verify_deck_output_return(request, result, expected_digest).is_err());
    fs::write(&pdf, bytes).unwrap();
    for field in ["project_id", "kit_id", "accepted_revision_id"] {
        let mut wrong = receipt.clone();
        wrong["document"][field] = json!(uuid::Uuid::now_v7());
        fs::write(result, wrong.to_string()).unwrap();
        assert!(ShapeProject::verify_deck_output_return(request, result, expected_digest).is_err());
    }
    // A later accepted Deck version does not invalidate the exact historical output source.
    let updated = deck.open_card_kit_draft(local.artifact_id).unwrap();
    let candidate = deck
        .propose_card_kit(local.artifact_id, &updated.draft_digest)
        .unwrap();
    deck.accept_card_kit(candidate).unwrap();
    fs::write(result, receipt.to_string()).unwrap();
    assert!(ShapeProject::verify_deck_output_return(request, result, expected_digest).is_ok());
    let mut wrong = receipt;
    wrong["request_id"] = json!("other");
    fs::write(result, wrong.to_string()).unwrap();
    assert!(ShapeProject::verify_deck_output_return(request, result, expected_digest).is_err());
}
