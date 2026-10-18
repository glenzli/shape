use crate::session::{create_desktop_project, open_desktop_session};
use serde_json::{Value, json};

#[test]
fn desktop_book_checkpoint_restores_script_and_selection_then_explicitly_reviews_and_accepts() {
    let root = std::env::temp_dir().join(format!(
        "shape-book-draft-bridge-{}",
        shape_domain::ArtifactId::new()
    ));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Books").unwrap();
    let mut state = json!({"composition":{"name":"Book","default_language":"en","languages":{},"pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768},"active_language":"en","page_index":1,"script":"[Cover] unfinished","script_dirty":true,"artwork_edit":null});
    let created: Value = serde_json::from_str(
        &session
            .session_book_draft_action_json(
                &json!({"action":"draft_new","state":state}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    let draft = created["draft"].clone();
    drop(session);
    let mut session = open_desktop_session(root.to_str().unwrap()).unwrap();
    let restored: Value = serde_json::from_str(
        &session
            .session_book_draft_action_json(
                &json!({"action":"draft_load","artifact_id":draft["artifact_id"]}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(restored["draft"], draft);
    assert_eq!(
        session
            .session_snapshot()
            .unwrap()
            .artifacts
            .iter()
            .filter(|a| a.kind_key == "text_document")
            .count(),
        0
    );
    assert!(
        session
            .session_book_draft_propose(
                draft["artifact_id"].as_str().unwrap(),
                draft["draft_digest"].as_str().unwrap()
            )
            .is_err()
    );
    state["script_dirty"] = json!(false);
    state["script"] = json!("[Cover] Moon\n[Author] Ada\n[1] Hello");
    state["composition"]["languages"]["en"] = serde_json::from_str::<Value>(
        &session
            .session_book_script("English", "", state["script"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let saved:Value=serde_json::from_str(&session.session_book_draft_action_json(&json!({"action":"draft_save","artifact_id":draft["artifact_id"],"draft_digest":draft["draft_digest"],"state":state}).to_string()).unwrap()).unwrap();
    let candidate = session
        .session_book_draft_propose(
            draft["artifact_id"].as_str().unwrap(),
            saved["draft"]["draft_digest"].as_str().unwrap(),
        )
        .unwrap();
    session.session_accept_candidate(&candidate).unwrap();
    let loaded: Value = serde_json::from_str(
        &session
            .session_book_draft_action_json(
                &json!({"action":"draft_load","artifact_id":draft["artifact_id"]}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert!(!loaded["draft"]["source"].is_null());
    assert_eq!(
        serde_json::from_value::<shape_domain::PictureBookDraftState>(
            loaded["draft"]["state"].clone()
        )
        .unwrap(),
        serde_json::from_value::<shape_domain::PictureBookDraftState>(state).unwrap()
    );
    assert_eq!(
        session
            .session_snapshot()
            .unwrap()
            .artifacts
            .iter()
            .filter(|a| a.kind_key == "text_document")
            .count(),
        1
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_ai_image_begin_returns_its_own_persisted_identity_and_parameters() {
    let root = std::env::temp_dir().join(format!(
        "shape-book-ai-identity-{}",
        shape_domain::ArtifactId::new()
    ));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Books").unwrap();
    let first = session
        .session_begin_ai_image_draft("First page", "A moon garden", 512, 768)
        .unwrap();
    let second = session
        .session_begin_ai_image_draft("Second page", "An owl", 256, 256)
        .unwrap();
    assert_ne!(first.context_artifact_id, second.context_artifact_id);
    assert_ne!(first.draft_id, second.draft_id);
    assert_eq!(first.ai_image_instruction, "A moon garden");
    assert_eq!(
        (first.ai_image_output_width, first.ai_image_output_height),
        (512, 768)
    );
    assert!(
        session
            .session_snapshot()
            .unwrap()
            .artifacts
            .iter()
            .all(|a| !a.has_accepted_revision)
    );
    drop(session);
    let session = open_desktop_session(root.to_str().unwrap()).unwrap();
    let drafts = session.session_operator_drafts();
    assert!(
        drafts
            .iter()
            .any(|d| d.context_artifact_id == first.context_artifact_id
                && d.draft_id == first.draft_id
                && d.ai_image_instruction == "A moon garden")
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
