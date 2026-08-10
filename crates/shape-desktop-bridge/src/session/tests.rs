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

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .expect("candidate accepts");
    assert!(accepted.graph_edges.is_empty());
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
    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A calm summer afternoon.")
        .expect("candidate executes");
    assert!(
        session
            .session_propose_text("not-an-artifact-id", "Discard me")
            .is_err()
    );

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .expect("original candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A calm summer afternoon."
    );
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn pending_candidate_can_branch_as_a_new_artifact_with_a_source_edge() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let before = session.session_snapshot().expect("snapshot reads");
    let source_revision = before.artifacts[0].accepted_revision_id.clone();
    let first = session
        .session_propose_text(&artifact_id.to_string(), "A warm summer afternoon.")
        .expect("first candidate executes");
    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A quiet summer afternoon.")
        .expect("candidate executes");
    assert_eq!(session.session_candidates().len(), 2);

    let branched = session
        .session_branch_candidate(&candidate.candidate_id, "Story — Quiet")
        .expect("candidate branches");
    assert_eq!(branched.artifacts.len(), 2);
    let source = branched
        .artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id.to_string())
        .expect("source remains");
    assert_eq!(source.accepted_revision_id, source_revision);
    let branch = branched
        .artifacts
        .iter()
        .find(|artifact| artifact.name == "Story — Quiet")
        .expect("branch appears");
    assert!(branch.accepted_parent_revision_ids.is_empty());
    assert_eq!(
        branch.transformation_input_revision_ids,
        vec![source_revision.clone()]
    );
    assert_eq!(
        branch.transformation_input_artifact_ids,
        vec![artifact_id.to_string()]
    );
    assert_eq!(branch.transformation_input_artifact_names, vec!["Story"]);
    assert_eq!(branch.text_preview, "A quiet summer afternoon.");
    assert_eq!(branched.graph_edges.len(), 1);
    let edge = &branched.graph_edges[0];
    assert_eq!(edge.source_artifact_id, artifact_id.to_string());
    assert_eq!(edge.target_artifact_id, branch.id);
    assert_eq!(edge.source_revision_id, source_revision);
    assert_eq!(edge.target_revision_id, branch.accepted_revision_id);
    assert_eq!(edge.transformation_id, branch.transformation_id);
    assert_eq!(edge.transformation_kind_key, "text_rewrite");
    let remaining = session.session_candidates();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].candidate_id, first.candidate_id);
    session
        .session_discard_candidate(&first.candidate_id)
        .expect("remaining candidate discards");
    assert!(session.session_candidates().is_empty());
    drop(session);

    let reopened = open_desktop_session(path).expect("session reopens");
    assert_eq!(
        reopened
            .session_snapshot()
            .expect("snapshot reads")
            .artifacts
            .len(),
        2
    );
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn shelf_accumulates_candidates_and_accepting_one_clears_stale_siblings() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let first = session
        .session_propose_text(&artifact_id.to_string(), "A bright summer afternoon.")
        .expect("first candidate executes");
    let second = session
        .session_propose_text(&artifact_id.to_string(), "A still summer afternoon.")
        .expect("second candidate executes");
    assert!(
        session
            .session_propose_text(&artifact_id.to_string(), "A still summer afternoon.")
            .is_err()
    );

    let projected = session.session_candidates();
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].candidate_id, second.candidate_id);
    assert_eq!(projected[1].candidate_id, first.candidate_id);
    assert!(
        session
            .session_accept_candidate("missing-candidate")
            .is_err()
    );
    assert_eq!(session.session_candidates().len(), 2);

    let accepted = session
        .session_accept_candidate(&second.candidate_id)
        .expect("selected candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A still summer afternoon."
    );
    assert!(session.session_candidates().is_empty());
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn raster_import_crop_candidate_accept_and_reopen_cross_the_desktop_bridge() {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    let root = test_root();
    let source = root.with_extension("png");
    let pixels: Vec<u8> = (0_u8..48).flat_map(|value| [value, 40, 80, 255]).collect();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 8, 6, ColorType::Rgba8.into())
        .unwrap();
    fs::write(&source, png).unwrap();

    ShapeProject::create(&root, "Desktop Raster").unwrap();
    let path = root.to_str().unwrap();
    let mut session = open_desktop_session(path).unwrap();
    let imported = session
        .session_import_raster(source.to_str().unwrap(), "Cover")
        .unwrap();
    let artifact = imported.artifacts.first().unwrap();
    assert!(artifact.has_image_preview);
    assert_eq!((artifact.image_width, artifact.image_height), (8, 6));
    let imported_head = artifact.accepted_revision_id.clone();
    let accepted_preview = session.session_image_preview(&artifact.id, "").unwrap();
    assert_eq!((accepted_preview.width, accepted_preview.height), (8, 6));

    let candidate = session
        .session_propose_raster_crop(&artifact.id, 1, 1, 4, 3)
        .unwrap();
    assert_eq!(candidate.kind_key, "image_raster");
    assert_eq!((candidate.image_width, candidate.image_height), (4, 3));
    assert_eq!(
        session.session_snapshot().unwrap().artifacts[0].accepted_revision_id,
        imported_head
    );
    let candidate_preview = session
        .session_image_preview(&artifact.id, &candidate.candidate_id)
        .unwrap();
    assert_eq!((candidate_preview.width, candidate_preview.height), (4, 3));

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_ne!(accepted.artifacts[0].accepted_revision_id, imported_head);
    assert_eq!(
        (
            accepted.artifacts[0].image_width,
            accepted.artifacts[0].image_height
        ),
        (4, 3)
    );
    drop(session);

    let reopened = open_desktop_session(path).unwrap();
    let preview = reopened
        .session_image_preview(&accepted.artifacts[0].id, "")
        .unwrap();
    assert_eq!((preview.width, preview.height), (4, 3));
    fs::remove_file(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}
