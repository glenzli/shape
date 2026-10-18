use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::{Value, json};
use shape_core::ShapeProject;
use std::{fs, path::Path, process::Command};

fn run(root: &Path, project: &Path, request: &Value, success: bool) -> Value {
    let path = root.join("request.json");
    fs::write(&path, request.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
        .arg("card-kit")
        .arg(project)
        .arg(path)
        .output()
        .unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "shape.card-kit-result/1");
    if success {
        assert_eq!(value["status"], "ok");
        value["result"].clone()
    } else {
        assert_eq!(value["status"], "error");
        value
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "One new process per action proves accepted and draft selections plus all retired public writes preserve the source project"
)]
fn public_cli_reads_legacy_cards_and_transfers_drafts_but_rejects_creation_and_printing() {
    let root = std::env::temp_dir().join(format!("shape-card-cli-compat-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("cards.shape");
    let mut project = ShapeProject::create(&bundle, "Legacy cards").unwrap();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&[20, 40, 80, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    let art = project
        .import_raster_bytes(png, "Accepted card art")
        .unwrap();
    let composition:shape_domain::CardKitRequest=serde_json::from_value(json!({"name":"Saved kit","cards":[{"id":"moon","title":"Moon","rules":"Moon · 月","art_prompt":"Unfinished prompt","artwork":{"artifact_id":art.artifact_id,"revision_id":art.id}}]})).unwrap();
    let draft = project.create_card_kit_draft(&composition).unwrap();
    let candidate = project
        .propose_card_kit(draft.artifact_id, &draft.draft_digest)
        .unwrap();
    let accepted = project.accept_card_kit(candidate).unwrap();
    let draft = project.open_card_kit_draft(draft.artifact_id).unwrap();
    let mut changed = composition.clone();
    changed.cards[0].rules = "Draft text only".into();
    let draft = project
        .save_card_kit_draft(draft.artifact_id, &draft.draft_digest, &changed)
        .unwrap();
    let source = json!({"artifact_id":accepted.artifact_id,"revision_id":accepted.id});
    drop(project);
    let before = fs::read(bundle.join("project.sqlite")).unwrap();
    let loaded = run(
        &root,
        &bundle,
        &json!({"action":"draft_load","artifact_id":draft.artifact_id}),
        true,
    );
    assert_eq!(loaded["draft"]["draft_digest"], draft.draft_digest);
    let read = run(
        &root,
        &bundle,
        &json!({"action":"read","source":source}),
        true,
    );
    assert_eq!(
        read["composition"]["cards"][0]["art_prompt"],
        "Unfinished prompt"
    );
    for request in [
        json!({"action":"draft_new","composition":composition}),
        json!({"action":"draft_save","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"composition":composition}),
        json!({"action":"draft_discard","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest}),
        json!({"action":"preview","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest}),
        json!({"action":"accept","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"review_digest":"review"}),
        json!({"action":"print_plan","source":source}),
    ] {
        let failure = run(&root, &bundle, &request, false);
        assert!(
            failure["message"]
                .as_str()
                .unwrap()
                .contains("moved to Deck")
        );
    }
    let transfer = run(
        &root,
        &bundle,
        &json!({"action":"deck_transfer","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"request_id":"legacy-cli","destination":root.join("deck-transfer")}),
        true,
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(transfer["manifest_path"].as_str().unwrap()).unwrap())
            .unwrap();
    assert_eq!(
        manifest["composition"]["cards"][0]["art_prompt"],
        "Unfinished prompt"
    );
    assert_eq!(manifest["materials"].as_array().unwrap().len(), 1);
    assert_eq!(
        manifest["composition"]["cards"][0]["rules"],
        "Draft text only"
    );
    let accepted_transfer = run(
        &root,
        &bundle,
        &json!({"action":"deck_accepted_transfer","source":source,"request_id":"selected-accepted","destination":root.join("accepted-transfer")}),
        true,
    );
    let accepted_manifest: Value = serde_json::from_slice(
        &fs::read(accepted_transfer["manifest_path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        accepted_manifest["composition"]["cards"][0]["rules"],
        "Moon · 月"
    );
    assert_eq!(
        accepted_manifest["origin"]["expected_head"],
        accepted.id.to_string()
    );
    run(
        &root,
        &bundle,
        &json!({"action":"export","source":source,"destination":root.join("legacy.zip")}),
        true,
    );
    assert_eq!(fs::read(bundle.join("project.sqlite")).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}
