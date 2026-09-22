use super::*;
use crate::{create_desktop_project, open_desktop_session};

#[test]
fn empty_writing_reopens_then_explicit_accept_preserves_profile_and_enables_speech() {
    let path = std::env::temp_dir().join(format!("shape-authoring-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(path.to_str().unwrap(), "Writing").unwrap();
    let snapshot = session
        .session_create_text_authoring("Listening", "listening")
        .unwrap();
    let id = snapshot.artifacts[0].id.clone();
    assert!(!snapshot.artifacts[0].has_accepted_revision);
    let draft = &session.session_operator_drafts()[0];
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text =
        "[role: Narrator]\n# Listening\n[角色：Narrator]\nHello, 世界.\n[停顿：2秒]".into();
    session
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    drop(session);
    let mut session = open_desktop_session(path.to_str().unwrap()).unwrap();
    let draft = &session.session_operator_drafts()[0];
    assert_eq!(
        TextAuthoring::from_json(&draft.text_authoring_json).unwrap(),
        state
    );
    let candidate = session
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    assert!(!session.session_snapshot().unwrap().artifacts[0].has_accepted_revision);
    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_eq!(accepted.artifacts[0].text_preview, state.text);
    assert_eq!(
        session.session_text_authoring_content(&id, "").unwrap(),
        state.text
    );
    let retained = session.session_operator_drafts();
    assert_eq!(retained.len(), 1);
    assert!(!retained[0].has_input_data_type);
    assert_eq!(retained[0].operator_type_key, "text.create");
    assert_eq!(
        TextAuthoring::from_json(&retained[0].text_authoring_json)
            .unwrap()
            .profile,
        state.profile
    );
    let speech = session.session_begin_authoring_speech(&id).unwrap();
    assert!(!speech.audio_speech_script_json.is_empty());
    assert_eq!(
        session
            .session_begin_authoring_speech(&id)
            .unwrap()
            .draft_id,
        speech.draft_id
    );
    drop(session);
    let session = open_desktop_session(path.to_str().unwrap()).unwrap();
    assert_eq!(session.session_operator_drafts().len(), 2);
    drop(session);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn invalid_script_stays_a_draft_and_full_text_never_uses_bounded_previews() {
    let path = std::env::temp_dir().join(format!("shape-authoring-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(path.to_str().unwrap(), "Writing").unwrap();
    let snapshot = session
        .session_create_text_authoring("Narration", "narration")
        .unwrap();
    let id = snapshot.artifacts[0].id.clone();
    let draft = &session.session_operator_drafts()[0];
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "[pause: invalid]\nHello.".into();
    session
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    assert!(
        session
            .session_propose_authored_text(&draft.draft_id)
            .is_err()
    );
    assert!(session.session_candidates().is_empty());
    state.text = format!(
        "{}\n[停顿：2秒]",
        "A complete long spoken sentence. ".repeat(1200)
    );
    session
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let candidate = session
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    assert!(candidate.text_preview_truncated);
    assert_eq!(
        session
            .session_text_authoring_content(&id, &candidate.candidate_id)
            .unwrap(),
        state.text
    );
    drop(session);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn common_node_templates_pin_original_and_persist_distinct_output_tasks() {
    let path = std::env::temp_dir().join(format!("shape-node-templates-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(path.to_str().unwrap(), "Templates").unwrap();
    let source = session
        .session_create_text_document("Original", "The source stays available.")
        .unwrap()
        .artifacts[0]
        .id
        .clone();
    let mut created = Vec::new();
    for mode in [
        "translate",
        "summarize",
        "polish",
        "expand",
        "outline",
        "prepare_script",
    ] {
        let draft = session
            .session_begin_operator_draft(&source, &format!("text.{mode}"))
            .unwrap();
        let state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
        assert_eq!(state.mode, mode);
        assert!(state.compiled_instruction_for_input(true).is_ok());
        assert_eq!(state.is_script(), mode == "prepare_script");
        assert_eq!(draft.operator_type_key, "text.edit");
        assert_ne!(draft.context_artifact_id, source);
        assert_eq!(
            session.session_text_node_input(&draft.draft_id).unwrap(),
            "The source stays available."
        );
        created.push((draft.draft_id, state));
    }
    assert_eq!(
        session.session_text_authoring_content(&source, "").unwrap(),
        "The source stays available."
    );
    drop(session);
    let session = open_desktop_session(path.to_str().unwrap()).unwrap();
    for (id, state) in created {
        let drafts = session.session_operator_drafts();
        let draft = drafts.iter().find(|d| d.draft_id == id).unwrap();
        assert_eq!(
            TextAuthoring::from_json(&draft.text_authoring_json).unwrap(),
            state
        );
    }
    drop(session);
    std::fs::remove_dir_all(path).unwrap();
}
