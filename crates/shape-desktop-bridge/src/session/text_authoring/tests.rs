use super::*;
use crate::{create_desktop_project, open_desktop_session};

#[test]
fn empty_writing_reopens_then_explicit_accept_preserves_profile_and_enables_speech() {
    let path = std::env::temp_dir().join(format!("shape-authoring-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(path.to_str().unwrap(), "Writing").unwrap();
    let snapshot = session
        .session_create_text_authoring("Production script", "script")
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
    assert_eq!(
        session
            .session_begin_operator_draft(&id, "audio.speech_synthesize")
            .unwrap()
            .draft_id,
        speech.draft_id,
        "the graph node library must continue the existing speech step"
    );
    assert_eq!(
        session
            .session_discard_operator_draft(&retained[0].draft_id)
            .unwrap_err(),
        "an accepted Source cannot be discarded as a draft"
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

#[test]
fn concurrent_draft_save_preserves_the_other_sessions_authored_text() {
    let path = std::env::temp_dir().join(format!("shape-draft-conflict-{}", uuid::Uuid::now_v7()));
    let mut first = create_desktop_project(path.to_str().unwrap(), "Concurrent writing").unwrap();
    first
        .session_create_text_authoring("Story", "plain")
        .unwrap();
    let mut second = open_desktop_session(path.to_str().unwrap()).unwrap();
    let draft = &first.session_operator_drafts()[0];
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "A human wrote this newer draft.".into();
    first
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let saved = state.clone();
    state.text = "An AI session still holds the older draft.".into();
    assert!(
        second
            .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
            .is_err(),
        "a stale session must not silently overwrite a newer draft at the same accepted head"
    );
    assert_eq!(
        second.session_operator_drafts()[0].text_authoring_json,
        draft.text_authoring_json,
        "a rejected write must restore the session's local draft"
    );
    let mut reopened = open_desktop_session(path.to_str().unwrap()).unwrap();
    assert_eq!(
        TextAuthoring::from_json(&reopened.session_operator_drafts()[0].text_authoring_json)
            .unwrap(),
        saved
    );
    let candidate = reopened
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    let duplicate = reopened
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    assert_eq!(candidate.candidate_id, duplicate.candidate_id);
    reopened
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert!(
        reopened
            .session_accept_candidate(&candidate.candidate_id)
            .is_err()
    );
    drop((first, second, reopened));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn concurrent_draft_discard_preserves_the_other_sessions_authored_text() {
    let path =
        std::env::temp_dir().join(format!("shape-discard-conflict-{}", uuid::Uuid::now_v7()));
    let mut first = create_desktop_project(path.to_str().unwrap(), "Concurrent writing").unwrap();
    first
        .session_create_text_authoring("Story", "plain")
        .unwrap();
    let mut second = open_desktop_session(path.to_str().unwrap()).unwrap();
    let draft = &first.session_operator_drafts()[0];
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "This draft was edited after the other session opened.".into();
    first
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    assert!(
        second
            .session_discard_operator_draft(&draft.draft_id)
            .is_err(),
        "a stale discard must not remove another session's draft and reserved output"
    );
    assert_eq!(second.session_operator_drafts().len(), 1);
    let mut reopened = open_desktop_session(path.to_str().unwrap()).unwrap();
    assert_eq!(
        TextAuthoring::from_json(&reopened.session_operator_drafts()[0].text_authoring_json)
            .unwrap(),
        state
    );
    reopened
        .session_discard_operator_draft(&draft.draft_id)
        .unwrap();
    assert!(reopened.session_snapshot().unwrap().artifacts.is_empty());
    drop((first, second, reopened));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn accepting_literal_candidate_does_not_write_back_stale_authored_intent() {
    let path = std::env::temp_dir().join(format!("shape-accept-conflict-{}", uuid::Uuid::now_v7()));
    let mut first = create_desktop_project(path.to_str().unwrap(), "Concurrent writing").unwrap();
    let created = first
        .session_create_text_authoring("Story", "plain")
        .unwrap();
    let id = created.artifacts[0].id.clone();
    let draft = &first.session_operator_drafts()[0];
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "Initial accepted text.".into();
    first
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let initial = first
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    first
        .session_accept_candidate(&initial.candidate_id)
        .unwrap();
    let candidate = first
        .session_propose_text(&id, "New accepted text.")
        .unwrap();
    let mut second = open_desktop_session(path.to_str().unwrap()).unwrap();
    state.text = "Keep this other editor's newer work in progress.".into();
    second
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    first
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    let reopened = open_desktop_session(path.to_str().unwrap()).unwrap();
    assert_eq!(
        reopened.session_text_authoring_content(&id, "").unwrap(),
        "New accepted text."
    );
    assert_eq!(
        TextAuthoring::from_json(&reopened.session_operator_drafts()[0].text_authoring_json)
            .unwrap(),
        state
    );
    drop((first, second, reopened));
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn repeated_current_input_refresh_preserves_candidate_and_stale_refresh_invalidates_it() {
    let path = std::env::temp_dir().join(format!("shape-refresh-review-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(path.to_str().unwrap(), "Source refresh").unwrap();
    let source = session
        .session_create_text_document("Original", "Keep these facts.")
        .unwrap()
        .artifacts[0]
        .id
        .clone();
    let draft = session
        .session_begin_text_authoring(&source, "plain")
        .unwrap();
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "Reviewed candidate, waiting for a human.".into();
    session
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let candidate = session
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    for _ in 0..3 {
        session.session_refresh_text_input(&draft.draft_id).unwrap();
        let candidates = session.session_candidates();
        assert_eq!(
            candidates.len(),
            1,
            "a repeated refresh at the same source must preserve the reviewed candidate"
        );
        assert_eq!(candidates[0].candidate_id, candidate.candidate_id);
    }
    // Another session advances the actual accepted source; refreshing now must retire stale results.
    let mut peer = open_desktop_session(path.to_str().unwrap()).unwrap();
    let updated = peer
        .session_propose_text(&source, "A human updated the facts.")
        .unwrap();
    peer.session_accept_candidate(&updated.candidate_id)
        .unwrap();
    assert!(
        session
            .session_accept_candidate(&candidate.candidate_id)
            .is_err()
    );
    session.session_refresh_text_input(&draft.draft_id).unwrap();
    assert!(session.session_candidates().is_empty());
    let refreshed = session.session_operator_drafts();
    let retained = refreshed
        .iter()
        .find(|d| d.draft_id == draft.draft_id)
        .unwrap();
    assert_eq!(
        TextAuthoring::from_json(&retained.text_authoring_json).unwrap(),
        state
    );
    let current = session
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    session.session_refresh_text_input(&draft.draft_id).unwrap();
    session
        .session_accept_candidate(&current.candidate_id)
        .unwrap();
    assert_eq!(
        session
            .session_text_authoring_content(&draft.context_artifact_id, "")
            .unwrap(),
        state.text
    );
    assert_eq!(
        session.session_text_authoring_content(&source, "").unwrap(),
        "A human updated the facts."
    );
    drop((peer, session));
    std::fs::remove_dir_all(path).unwrap();
}
