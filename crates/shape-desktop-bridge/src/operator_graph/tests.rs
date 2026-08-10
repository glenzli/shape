use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use uuid::Uuid;

use super::project_operator_graph;

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-operator-graph-{}", Uuid::now_v7()))
}

#[test]
fn accepted_text_history_projects_as_source_edit_output() {
    let root = test_root();
    let mut project = ShapeProject::create(&root, "Operator Graph").unwrap();
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .unwrap();
    let imported = project
        .propose_text(
            artifact.id,
            None,
            "First",
            IntentSpec::new("Import source").unwrap(),
            Vec::new(),
        )
        .unwrap();
    project.accept_text(imported).unwrap();
    let accepted = project
        .snapshot()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|candidate| candidate.id == artifact.id)
        .unwrap();
    let rewritten = project
        .propose_text(
            artifact.id,
            accepted.accepted_revision,
            "Second",
            IntentSpec::new("Rewrite source").unwrap(),
            Vec::new(),
        )
        .unwrap();
    project.accept_text(rewritten).unwrap();
    let artifacts = project.snapshot().unwrap().artifacts;
    let scene = artifacts
        .iter()
        .find(|candidate| candidate.id == artifact.id)
        .unwrap();
    let accepted_revision_id = scene.accepted_revision.unwrap();
    let accepted_revision = project.revision(accepted_revision_id).unwrap();
    let graph = project_operator_graph(&project, scene, &artifacts).unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
    assert_eq!(graph.nodes[0].role_key, "source");
    assert_eq!(graph.nodes[1].role_key, "operator");
    assert_eq!(graph.nodes[1].operator_type_key, "text.edit");
    assert_eq!(graph.nodes[1].artifact_id, artifact.id.to_string());
    assert_eq!(graph.nodes[1].revision_id, accepted_revision_id.to_string());
    assert_eq!(
        graph.nodes[1].transformation_id,
        accepted_revision.transformation_id.to_string()
    );
    assert_eq!(graph.nodes[2].role_key, "output");
    assert_eq!(graph.edges[0].data_type_key, "text.document");
    fs::remove_dir_all(root).unwrap();
}
