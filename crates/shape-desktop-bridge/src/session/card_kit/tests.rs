use crate::session::{create_desktop_project, open_desktop_session};
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::{Value, json};

#[test]
fn desktop_card_action_recovers_drafts_review_accepts_and_projects_card_kind() {
    let root = std::env::temp_dir().join(format!(
        "shape-card-bridge-{}",
        shape_domain::ArtifactId::new()
    ));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Card project").unwrap();
    let catalog: Value =
        serde_json::from_str(&session.session_workflow_templates_json().unwrap()).unwrap();
    assert_eq!(catalog["templates"][1]["id"], "card_kit");
    let result:Value=serde_json::from_str(&session.session_card_kit_action_json(&json!({"action":"draft_new","composition":{"name":"Cards","brief":"Tiny game","style":"Ink","cards":[{"id":"fox","title":"Fox","rules":"Draw one card."}]}}).to_string()).unwrap()).unwrap();
    let draft = result["draft"].clone();
    drop(session);
    let mut session = open_desktop_session(root.to_str().unwrap()).unwrap();
    let loaded: Value = serde_json::from_str(
        &session
            .session_card_kit_action_json(&json!({"action":"draft_list"}).to_string())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(loaded["drafts"][0], draft);
    let preview:Value=serde_json::from_str(&session.session_card_kit_action_json(&json!({"action":"preview","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"card_id":"fox"}).to_string()).unwrap()).unwrap();
    assert_eq!(preview["preview"]["ready_to_export"], false);
    let accepted:Value=serde_json::from_str(&session.session_card_kit_action_json(&json!({"action":"accept","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"review_digest":preview["review_digest"]}).to_string()).unwrap()).unwrap();
    assert_eq!(
        accepted["source"]["revision_id"],
        accepted["draft"]["expected_head"]
    );
    let snapshot = session.session_snapshot().unwrap();
    assert_eq!(snapshot.artifacts[0].kind_key, "card_kit");
    assert!(snapshot.artifacts[0].has_accepted_revision);
    assert!(!snapshot.artifacts[0].operator_graph_nodes.is_empty());
    drop(session);
    let session = open_desktop_session(root.to_str().unwrap()).unwrap();
    assert!(session.session_snapshot().unwrap().artifacts[0].has_accepted_revision);
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn desktop_print_plan_pins_accepted_source_and_deduplicates_quantity_svg_resources() {
    let root = std::env::temp_dir().join(format!("shape-print-bridge-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Print project").unwrap();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&[20, 40, 80, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    let art = session
        .project
        .import_raster_bytes(png, "Card art")
        .unwrap();
    let composition = serde_json::from_value(json!({"name":"Print cards","cards":[{"id":"fox","title":"Fox","rules":"Draw one card.","quantity":10,"artwork":{"artifact_id":art.artifact_id,"revision_id":art.id}}]})).unwrap();
    let draft = session.project.create_card_kit_draft(&composition).unwrap();
    let candidate = session
        .project
        .propose_card_kit(draft.artifact_id, &draft.draft_digest)
        .unwrap();
    let accepted = session.project.accept_card_kit(candidate).unwrap();
    let source = json!({"artifact_id":accepted.artifact_id,"revision_id":accepted.id});
    drop(session);
    let mut session = open_desktop_session(root.to_str().unwrap()).unwrap();
    let before = session
        .project
        .open_card_kit_draft(accepted.artifact_id)
        .unwrap();
    let result = session
        .session_card_kit_action_json(&json!({"action":"print_plan","source":source}).to_string())
        .unwrap();
    assert_eq!(result.matches("data:image/png;base64,").count(), 1);
    let result: Value = serde_json::from_str(&result).unwrap();
    let plan = &result["print_plan"];
    assert_eq!(plan["source"], source);
    assert_eq!(plan["total_copies"], 10);
    assert_eq!(plan["page_count"], 2);
    assert_eq!(plan["pages"][0]["slots"].as_array().unwrap().len(), 9);
    assert_eq!(plan["pages"][1]["slots"].as_array().unwrap().len(), 1);
    let after = session
        .project
        .open_card_kit_draft(accepted.artifact_id)
        .unwrap();
    assert_eq!(before.draft_digest, after.draft_digest);
    assert_eq!(before.expected_head, after.expected_head);
    let error = session
        .session_card_kit_action_json(
            &json!({"action":"print_plan","source":source,"options":{"gap_mm":0}}).to_string(),
        )
        .unwrap_err();
    assert!(error.starts_with("card_print.crop_clearance:"));
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
