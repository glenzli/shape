use std::fs;

use shape_core::ShapeProject;
use shape_domain::{
    AI_IMAGE_RASTER_DATA_TYPE, Artifact, ArtifactKind, ArtifactWorkingGraph, OperatorDataTypeId,
    OperatorTypeId,
};
use uuid::Uuid;

use super::*;
use crate::operator_catalog::configuration_for_ai_image_generate;

fn project_with_draft(instruction: &str) -> (std::path::PathBuf, ArtifactId, String) {
    let root = std::env::temp_dir().join(format!("shape-infer-image-{}", Uuid::now_v7()));
    let mut project = ShapeProject::create(&root, "Image Draft").unwrap();
    let artifact = Artifact::new("Concept", ArtifactKind::ImageRaster).unwrap();
    let mut graph = ArtifactWorkingGraph::new_source(artifact.id);
    let draft = graph
        .add_source_operator(
            OperatorTypeId::new(IMAGE_GENERATE_OPERATOR).unwrap(),
            OperatorDataTypeId::new(AI_IMAGE_RASTER_DATA_TYPE).unwrap(),
        )
        .unwrap();
    let draft_id = draft.id().to_string();
    assert!(graph.set_operator_configuration(
        draft.id(),
        Some(configuration_for_ai_image_generate(instruction, 1024, 1024).unwrap()),
    ));
    project
        .create_source_artifact_draft(&artifact, &graph)
        .unwrap();
    (root, artifact.id, draft_id)
}

#[test]
fn unconfigured_draft_fails_before_credential_access() {
    let (root, artifact_id, draft_id) = project_with_draft("");
    let missing_credential = root.with_extension("missing-token");
    let error = generate_infer_image_candidate(
        root.to_str().unwrap(),
        &artifact_id.to_string(),
        &draft_id,
        missing_credential.to_str().unwrap(),
        "",
        "gpt_5_6_luna",
        "",
    )
    .unwrap_err();
    assert_eq!(error, "invalid_image_request");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn executable_draft_reaches_the_owner_only_credential_boundary() {
    let (root, artifact_id, draft_id) = project_with_draft("A cobalt glass bird");
    let missing_credential = root.with_extension("missing-token");
    let error = generate_infer_image_candidate(
        root.to_str().unwrap(),
        &artifact_id.to_string(),
        &draft_id,
        missing_credential.to_str().unwrap(),
        "",
        "gpt_5_6_luna",
        "",
    )
    .unwrap_err();
    assert_eq!(error, "credential_missing");
    fs::remove_dir_all(root).unwrap();
}
