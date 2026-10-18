use crate::session::{create_desktop_project, open_desktop_session};
use serde_json::{Value, json};

#[test]
fn legacy_book_draft_reads_without_checkpoint_and_exports_unaccepted_script() {
    let root = std::env::temp_dir().join(format!(
        "shape-book-draft-compat-{}",
        shape_domain::ArtifactId::new()
    ));
    std::fs::create_dir(&root).unwrap();
    let bundle = root.join("legacy.shape");
    let mut project = shape_core::ShapeProject::create(&bundle, "Legacy").unwrap();
    let state = serde_json::from_value(json!({"composition":{"name":"Book","default_language":"en","languages":{},"pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768},"active_language":"en","page_index":1,"script":"unfinished raw draft [with notes]","script_dirty":true})).unwrap();
    let draft = project.create_picture_book_draft(&state).unwrap();
    drop(project);
    let mut session = open_desktop_session(bundle.to_str().unwrap()).unwrap();
    let before = std::fs::read(bundle.join("project.sqlite")).unwrap();
    let loaded: Value = serde_json::from_str(
        &session
            .session_book_draft_action_json(
                &json!({"action":"draft_load","artifact_id":draft.artifact_id}).to_string(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        loaded["draft"]["state"]["script"],
        "unfinished raw draft [with notes]"
    );
    assert!(
        session
            .session_book_draft_action_json(
                &json!({"action":"draft_new","state":state}).to_string()
            )
            .unwrap_err()
            .contains("moved to Storybook")
    );
    let result: Value = serde_json::from_str(
        &session
            .session_book_draft_export(
                &draft.artifact_id.to_string(),
                &draft.draft_digest,
                root.join("draft.scproj").to_str().unwrap(),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["unaccepted_script_preserved"], true);
    assert_eq!(
        std::fs::read(bundle.join("project.sqlite")).unwrap(),
        before
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
