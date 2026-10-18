use crate::session::open_desktop_session;
use serde_json::{Value, json};
use shape_core::ShapeProject;

#[test]
fn saved_cards_remain_readable_and_transferable_while_authoring_is_rejected() {
    let root = std::env::temp_dir().join(format!(
        "shape-card-compat-{}",
        shape_domain::ArtifactId::new()
    ));
    std::fs::create_dir(&root).unwrap();
    let bundle = root.join("legacy.shape");
    let mut project = ShapeProject::create(&bundle, "Legacy").unwrap();
    let request = serde_json::from_value(
        json!({"name":"Saved cards","cards":[{"id":"word","rules":"Moon · 月"}]}),
    )
    .unwrap();
    let draft = project.create_card_kit_draft(&request).unwrap();
    drop(project);
    let mut session = open_desktop_session(bundle.to_str().unwrap()).unwrap();
    let before = std::fs::read(bundle.join("project.sqlite")).unwrap();
    let loaded: Value = serde_json::from_str(
        &session
            .session_card_kit_action_json(
                &json!({"action":"draft_load","artifact_id":draft.artifact_id}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(loaded["draft"]["draft_digest"], draft.draft_digest);
    let snapshot: Value =
        serde_json::from_str(&session.session_legacy_snapshot_json().unwrap()).unwrap();
    assert_eq!(
        snapshot["legacy_artifacts"][0]["draft"]["graph_digest"],
        draft.draft_digest
    );
    for request in [
        json!({"action":"draft_new","composition":request}),
        json!({"action":"draft_discard","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest}),
        json!({"action":"preview","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest}),
    ] {
        assert!(
            session
                .session_card_kit_action_json(&request.to_string())
                .unwrap_err()
                .contains("moved to Deck")
        );
    }
    assert!(
        session
            .session_begin_operator_draft(&draft.artifact_id.to_string(), "audio.speech_synthesize")
            .unwrap_err()
            .contains("moved to Vox")
    );
    let transfer: Value = serde_json::from_str(&session.session_card_kit_action_json(&json!({"action":"deck_transfer","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"request_id":"compat-cards","destination":root.join("deck-transfer")}).to_string()).unwrap()).unwrap();
    assert!(std::path::Path::new(transfer["manifest_path"].as_str().unwrap()).is_file());
    let catalog: Value =
        serde_json::from_str(&session.session_workflow_templates_json().unwrap()).unwrap();
    assert!(
        catalog["templates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|template| !matches!(template["id"].as_str(), Some("card_kit" | "picture_book")))
    );
    assert_eq!(
        std::fs::read(bundle.join("project.sqlite")).unwrap(),
        before
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
