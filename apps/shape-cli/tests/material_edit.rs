use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::{Value, json};
use shape_core::ShapeProject;
use std::{fs, path::Path, process::Command};
fn png(c: u8) -> Vec<u8> {
    let mut b = Vec::new();
    PngEncoder::new(&mut b)
        .write_image(&[c, 20, 30, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    b
}
fn run(root: &Path, project: &Path, request: &Value, success: bool) -> Value {
    let file = root.join("request.json");
    fs::write(&file, serde_json::to_vec(request).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
        .arg("material-edit")
        .arg(project)
        .arg(file)
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
    assert_eq!(value["schema"], "shape.material-edit-result/1");
    value["result"].clone()
}

#[test]
fn cli_roundtrip_is_review_then_explicit_digest_pinned_accept_and_duplicate_is_rejected() {
    let root = std::env::temp_dir().join(format!("shape-material-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let path = root.join("fixture.shape");
    let mut project = ShapeProject::create(&path, "CLI").unwrap();
    let source = project.import_raster_bytes(png(10), "Source").unwrap();
    let begin = run(
        &root,
        &path,
        &json!({"action":"begin","source":{"artifact_id":source.artifact_id,"revision_id":source.id},"name":"Edited","editor":"shadow"}),
        true,
    );
    let ticket = &begin["ticket"];
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        true,
    );
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        false,
    );
    let returned = root.join("edited.png");
    fs::write(&returned, png(80)).unwrap();
    let handoff = root.join("handoff/handoff.json");
    let review = run(
        &root,
        &path,
        &json!({"action":"review","handoff":handoff,"returned_file":returned}),
        true,
    );
    assert_eq!(review["accepted"], false);
    let artifact = serde_json::from_value(ticket["output_artifact_id"].clone()).unwrap();
    assert!(project.read_accepted(artifact).unwrap().is_none());
    let accept = json!({"action":"accept","handoff":handoff,"returned_file":returned,"returned_blake3":review["returned_blake3"]});
    fs::write(&returned, png(81)).unwrap();
    run(&root, &path, &accept, false);
    fs::write(&returned, png(80)).unwrap();
    run(&root, &path, &accept, true);
    run(&root, &path, &accept, false);
    assert_eq!(
        project.read_accepted(artifact).unwrap().unwrap().bytes,
        png(80)
    );
    assert_eq!(
        project
            .read_accepted(source.artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id,
        source.id
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_action_fields_and_cancelled_handoffs_cannot_mutate_accepted_heads() {
    let root = std::env::temp_dir().join(format!(
        "shape-material-cli-cancel-{}",
        uuid::Uuid::now_v7()
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("fixture.shape");
    let mut project = ShapeProject::create(&path, "CLI").unwrap();
    let source = project.import_raster_bytes(png(1), "Source").unwrap();
    let req = json!({"action":"begin","source":{"artifact_id":source.artifact_id,"revision_id":source.id},"name":"Edited","editor":"shadow"});
    let mut unknown = req.clone();
    unknown["surprise"] = json!(true);
    run(&root, &path, &unknown, false);
    assert_eq!(project.snapshot().unwrap().artifacts.len(), 1);
    let ticket = run(&root, &path, &req, true)["ticket"].clone();
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        true,
    );
    run(
        &root,
        &path,
        &json!({"action":"cancel","ticket":ticket}),
        true,
    );
    let returned = root.join("edited.png");
    fs::write(&returned, png(2)).unwrap();
    run(
        &root,
        &path,
        &json!({"action":"review","handoff":root.join("handoff/handoff.json"),"returned_file":returned}),
        false,
    );
    assert!(
        project
            .read_accepted(serde_json::from_value(ticket["output_artifact_id"].clone()).unwrap())
            .unwrap()
            .is_none()
    );
    fs::remove_dir_all(root).unwrap();
}
