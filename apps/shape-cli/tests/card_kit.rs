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
    reason = "One process-per-action lifecycle proves recovery, print planning and material export preserve the same accepted source"
)]
fn public_cli_preserves_recovery_acceptance_print_and_material_export_boundaries() {
    let root = std::env::temp_dir().join(format!("shape-card-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("cards.shape");
    let mut project = ShapeProject::create(&bundle, "CLI cards").unwrap();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&[20, 40, 80, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    let art = project.import_raster_bytes(png, "Card art").unwrap();
    drop(project);
    let composition = json!({"name":"CLI kit","brief":"One card","style":"Ink","cards":[{"id":"fox","title":"Moon fox","rules":"Draw one card.","quantity":7,"artwork":{"artifact_id":art.artifact_id,"revision_id":art.id}}]});
    let created = run(
        &root,
        &bundle,
        &json!({"action":"draft_new","composition":composition}),
        true,
    );
    let draft = &created["draft"];
    let list = run(&root, &bundle, &json!({"action":"draft_list"}), true);
    assert_eq!(list["drafts"][0], *draft);
    let mut edited = composition.clone();
    edited["cards"][0]["flavor"] = json!("Under the moon.");
    let saved = run(
        &root,
        &bundle,
        &json!({"action":"draft_save","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"composition":edited}),
        true,
    );
    run(
        &root,
        &bundle,
        &json!({"action":"draft_save","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"composition":composition}),
        false,
    );
    let draft = &saved["draft"];
    let preview = run(
        &root,
        &bundle,
        &json!({"action":"preview","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"card_id":"fox"}),
        true,
    );
    assert_eq!(preview["preview"]["ready_to_export"], true);
    assert!(
        preview["preview"]["svg"]
            .as_str()
            .unwrap()
            .contains("data:image/png;base64,")
    );
    let accepted = run(
        &root,
        &bundle,
        &json!({"action":"accept","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"review_digest":preview["review_digest"]}),
        true,
    );
    let read = run(
        &root,
        &bundle,
        &json!({"action":"read","source":accepted["source"]}),
        true,
    );
    assert_eq!(
        serde_json::from_value::<shape_domain::CardKitRequest>(read["composition"].clone())
            .unwrap(),
        serde_json::from_value::<shape_domain::CardKitRequest>(edited).unwrap()
    );
    let printed = run(
        &root,
        &bundle,
        &json!({"action":"print_plan","source":accepted["source"],"options":{"paper":"letter"}}),
        true,
    );
    let plan = &printed["print_plan"];
    assert_eq!(plan["schema"], "shape.card-print-plan/1");
    assert_eq!(plan["source"], accepted["source"]);
    assert_eq!(plan["total_copies"], 7);
    assert_eq!(plan["cards"].as_array().unwrap().len(), 1);
    assert_eq!(plan["page_count"], 2);
    assert_eq!(plan["pages"][0]["slots"].as_array().unwrap().len(), 6);
    assert_eq!(plan["pages"][1]["slots"].as_array().unwrap().len(), 1);
    assert_eq!(plan["pages"][1]["slots"][0]["copy_index"], 7);
    assert_eq!(plan["commercial_prepress"], false);
    run(
        &root,
        &bundle,
        &json!({"action":"print_plan","source":accepted["source"],"options":{"bleed_mm":3}}),
        false,
    );
    let exported = run(
        &root,
        &bundle,
        &json!({"action":"export","source":accepted["source"],"destination":root.join("cards.zip")}),
        true,
    );
    assert_eq!(exported["cards"], 1);
    assert_eq!(exported["print_ready"], false);
    assert!(root.join("cards.zip").is_file());
    run(
        &root,
        &bundle,
        &json!({"action":"export","source":accepted["source"],"destination":root.join("cards.zip")}),
        false,
    );
    fs::remove_dir_all(root).unwrap();
}
