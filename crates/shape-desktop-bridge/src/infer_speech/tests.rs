use std::fs;

use shape_core::ShapeProject;
use shape_domain::{
    ArtifactKind, ArtifactWorkingGraph, IntentSpec, OperatorDataTypeId, OperatorTypeId,
};
use uuid::Uuid;

use super::*;

#[test]
fn draft_identity_and_configuration_are_authoritative_before_credential_access() {
    assert_eq!(
        generate_infer_speech_candidate(
            "/missing/project.shape",
            "not-an-artifact",
            "draft.invalid",
            "Narration",
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
    let source_head = project
        .snapshot()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|artifact| artifact.id == source.id)
        .unwrap()
        .accepted_revision
        .unwrap();
    let mut graph = ArtifactWorkingGraph::new(source.id, source_head);
    let draft = graph
        .add_operator(
            OperatorTypeId::new(AUDIO_SPEECH_OPERATOR).unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("audio.clip").unwrap(),
        )
        .unwrap();
    graph.set_operator_configuration(
        draft.id(),
        Some(crate::operator_catalog::default_audio_speech_configuration().unwrap()),
    );
    project
        .save_artifact_working_graph(&graph)
        .expect("speech Working Graph saves");
    drop(project);

    assert_eq!(
        generate_infer_speech_candidate(
            root.to_str().expect("portable path"),
            &source.id.to_string(),
            "draft.invalid",
            "Narration",
            "/missing/token",
            ""
        )
        .unwrap_err(),
        "invalid_operator_draft"
    );
    assert_eq!(
        generate_infer_speech_candidate(
            root.to_str().expect("portable path"),
            &source.id.to_string(),
            draft.id().as_str(),
            "Narration",
            "/missing/token",
            ""
        )
        .unwrap_err(),
        "credential_missing"
    );
    fs::remove_dir_all(root).expect("fixture removes");
}
