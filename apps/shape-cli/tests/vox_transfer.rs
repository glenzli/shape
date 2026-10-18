use serde_json::{Value, json};
use shape_core::ShapeProject;
use shape_domain::{
    ArtifactKind, ArtifactWorkingGraph, ContentDigest, OperatorConfigurationSchemaId,
    OperatorDataTypeId, OperatorTypeId, WorkingOperatorConfiguration,
};
use std::{fs, path::Path, process::Command};

fn run(project: &Path, command: &str, request: Option<&Path>, success: bool) -> Value {
    let mut process = Command::new(env!("CARGO_BIN_EXE_shape-cli"));
    process.arg(command).arg(project);
    if let Some(request) = request {
        process.arg(request);
    }
    let output = process.output().unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn fresh_cli_preserves_full_text_draft_and_rejects_stale_selection_and_corrupt_source() {
    let root = std::env::temp_dir().join(format!("shape-vox-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("source.shape");
    let mut project = ShapeProject::create(&bundle, "CLI migration").unwrap();
    let text = project
        .create_text_document("Accepted document", "Accepted text. 完整文稿。")
        .unwrap();
    let mut graph = ArtifactWorkingGraph::new(text.artifact_id, text.id);
    let node = graph
        .add_operator(
            OperatorTypeId::new("text.edit").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    let raw = "Unaccepted text. 未采用原文。";
    graph.set_operator_configuration(node.id(),Some(WorkingOperatorConfiguration::new(OperatorConfigurationSchemaId::new("shape.operator-draft.text-authoring@20260922.1").unwrap(),json!({"profile":"plain","entry":"manual","instruction":"Preserve exact bytes","material":"","text":raw,"future_setting":7}).to_string()).unwrap()));
    project.save_artifact_working_graph(&graph).unwrap();
    drop(project);
    let before = fs::read(bundle.join("project.sqlite")).unwrap();
    let snapshot = run(&bundle, "legacy-snapshot", None, true);
    let digest = snapshot["texts"][0]["authoring_graph_digest"].clone();
    let request = json!({"request_id":"cli-contract","text_artifact_id":text.artifact_id,"expected_text_head":text.id,"authoring_graph_digest":digest,"destination":root.join("transfer")});
    let request_path = root.join("request.json");
    fs::write(&request_path, request.to_string()).unwrap();
    let result = run(&bundle, "vox-transfer", Some(&request_path), true);
    let bytes = fs::read(result["manifest_path"].as_str().unwrap()).unwrap();
    assert_eq!(
        ContentDigest::from_bytes(&bytes).to_string(),
        result["manifest_blake3"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    let draft = root.join("transfer").join(
        manifest["authoring_draft"]["text"]["relative_path"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(fs::read(draft).unwrap(), raw.as_bytes());
    assert_eq!(manifest["authoring_draft"]["status"], "unaccepted");
    assert_eq!(
        manifest["accepted_text"]["source"]["revision_id"],
        text.id.to_string()
    );
    assert_eq!(fs::read(bundle.join("project.sqlite")).unwrap(), before);
    run(&bundle, "vox-transfer", Some(&request_path), false);
    let mut stale = request.clone();
    stale["destination"] = json!(root.join("stale"));
    stale["authoring_graph_digest"] = json!("0".repeat(64));
    fs::write(&request_path, stale.to_string()).unwrap();
    run(&bundle, "vox-transfer", Some(&request_path), false);
    assert!(!root.join("stale").exists());
    let digest = text.content.digest.to_string();
    let object = bundle
        .join("objects/b3")
        .join(&digest[..2])
        .join(&digest[2..]);
    fs::write(object, b"corrupt").unwrap();
    let mut corrupt = request;
    corrupt["destination"] = json!(root.join("corrupt"));
    fs::write(&request_path, corrupt.to_string()).unwrap();
    run(&bundle, "vox-transfer", Some(&request_path), false);
    assert!(!root.join("corrupt").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_cli_exports_an_unaccepted_text_with_explicit_null_head() {
    let root = std::env::temp_dir().join(format!("shape-vox-only-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("only.shape");
    let project = ShapeProject::create(&bundle, "Only draft").unwrap();
    let text = project
        .create_artifact("Only draft", ArtifactKind::TextDocument)
        .unwrap();
    let mut graph = ArtifactWorkingGraph::new_source(text.id);
    let node = graph
        .add_source_operator(
            OperatorTypeId::new("text.create").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    graph.set_operator_configuration(node.id(),Some(WorkingOperatorConfiguration::new(OperatorConfigurationSchemaId::new("shape.operator-draft.text-authoring@20260922.1").unwrap(),json!({"profile":"dialogue","entry":"manual","instruction":"","material":"","text":"unfinished"}).to_string()).unwrap()));
    project.save_artifact_working_graph(&graph).unwrap();
    drop(project);
    let snapshot = run(&bundle, "legacy-snapshot", None, true);
    let request = json!({"request_id":"only-draft","text_artifact_id":text.id,"expected_text_head":null,"authoring_graph_digest":snapshot["texts"][0]["authoring_graph_digest"],"destination":root.join("transfer")});
    let request_path = root.join("request.json");
    fs::write(&request_path, request.to_string()).unwrap();
    let result = run(&bundle, "vox-transfer", Some(&request_path), true);
    let manifest: Value =
        serde_json::from_slice(&fs::read(result["manifest_path"].as_str().unwrap()).unwrap())
            .unwrap();
    assert!(manifest["accepted_text"].is_null());
    assert_eq!(manifest["authoring_draft"]["status"], "unaccepted");
    fs::remove_dir_all(root).unwrap();
}
