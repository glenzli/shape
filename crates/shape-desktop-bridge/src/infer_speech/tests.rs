use std::fs;

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use uuid::Uuid;

use super::*;

#[test]
fn malformed_identity_and_speed_fail_before_credential_or_network_access() {
    assert_eq!(
        generate_infer_speech_candidate(
            "/missing/project.shape",
            "not-an-artifact",
            "Narration",
            1_000,
            "/missing/token",
            ""
        )
        .unwrap_err(),
        "invalid_artifact"
    );

    let root = std::env::temp_dir().join(format!("shape-bridge-speech-invalid-{}", Uuid::now_v7()));
    let mut project = ShapeProject::create(&root, "Speech bridge").expect("project creates");
    let source = project
        .create_artifact("Narration", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let initial = project
        .propose_text(
            source.id,
            None,
            "一段旁白。",
            IntentSpec::new("Write narration").expect("intent is valid"),
            Vec::new(),
        )
        .expect("candidate executes");
    project.accept_text(initial).expect("candidate accepts");
    drop(project);

    assert_eq!(
        generate_infer_speech_candidate(
            root.to_str().expect("portable path"),
            &source.id.to_string(),
            "Narration",
            0,
            "/missing/token",
            ""
        )
        .unwrap_err(),
        "invalid_speech_request"
    );
    fs::remove_dir_all(root).expect("fixture removes");
}
