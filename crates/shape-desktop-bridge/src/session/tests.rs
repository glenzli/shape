use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use uuid::Uuid;

use super::open_desktop_session;

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-desktop-session-{}", Uuid::now_v7()))
}

fn seeded_project(root: &PathBuf) -> shape_domain::ArtifactId {
    let mut project = ShapeProject::create(root, "Desktop Session").expect("project creates");
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let initial = project
        .propose_text(
            artifact.id,
            None,
            "A summer afternoon.",
            IntentSpec::new("Import initial text").expect("intent valid"),
            Vec::new(),
        )
        .expect("candidate executes");
    project.accept_text(initial).expect("candidate accepts");
    artifact.id
}

#[test]
fn candidate_is_transient_until_acceptance_and_survives_reopen_after_commit() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let before = session.session_snapshot().expect("snapshot reads");
    let before_artifact = before.artifacts.first().expect("artifact projected");
    let before_revision = before_artifact.accepted_revision_id.clone();

    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A quiet summer afternoon.")
        .expect("candidate executes");
    assert_eq!(candidate.artifact_id, artifact_id.to_string());
    assert_eq!(candidate.text_preview, "A quiet summer afternoon.");
    assert!(!candidate.text_preview_truncated);

    let still_accepted = session
        .session_snapshot()
        .expect("snapshot remains readable");
    assert_eq!(
        still_accepted.artifacts[0].accepted_revision_id,
        before_revision
    );
    assert_eq!(
        still_accepted.artifacts[0].text_preview,
        "A summer afternoon."
    );

    let accepted = session.session_accept_text().expect("candidate accepts");
    assert_ne!(accepted.artifacts[0].accepted_revision_id, before_revision);
    assert_eq!(
        accepted.artifacts[0].accepted_parent_revision_ids,
        vec![before_revision.clone()]
    );
    assert_eq!(
        accepted.artifacts[0].transformation_kind_key,
        "text_rewrite"
    );
    assert_eq!(
        accepted.artifacts[0].transformation_input_revision_ids,
        vec![before_revision]
    );
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A quiet summer afternoon."
    );
    drop(session);

    let reopened = ShapeProject::open(&root).expect("project reopens");
    let content = reopened
        .read_accepted(artifact_id)
        .expect("accepted head reads")
        .expect("accepted content exists");
    assert_eq!(content.bytes, b"A quiet summer afternoon.");
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn failed_reproposal_does_not_replace_a_valid_pending_candidate() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    session
        .session_propose_text(&artifact_id.to_string(), "A calm summer afternoon.")
        .expect("candidate executes");
    assert!(
        session
            .session_propose_text("not-an-artifact-id", "Discard me")
            .is_err()
    );

    let accepted = session
        .session_accept_text()
        .expect("original candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A calm summer afternoon."
    );
    fs::remove_dir_all(root).expect("test project removes");
}
