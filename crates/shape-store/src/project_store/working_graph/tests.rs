use shape_domain::{
    Artifact, ArtifactKind, ArtifactWorkingGraph, OperatorDataTypeId, OperatorTypeId,
};

use super::*;
use crate::project_store::tests::{successful_commit, test_root};

#[test]
fn working_graph_round_trips_without_advancing_accepted_history() {
    let root = test_root("working-graph");
    let mut store = ProjectStore::create(&root, "Working Graph").unwrap();
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let revision = store.accept(successful_commit(&artifact, None)).unwrap();
    let mut graph = ArtifactWorkingGraph::new(artifact.id, revision.id);
    graph
        .add_operator(
            OperatorTypeId::new("text.transform").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();

    store.save_artifact_working_graph(&graph).unwrap();
    assert_eq!(
        store.artifact_working_graph(artifact.id).unwrap(),
        Some(graph.clone())
    );
    assert_eq!(
        store
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(revision.id)
    );
    drop(store);

    let reopened = ProjectStore::open(&root).unwrap();
    assert_eq!(reopened.artifact_working_graphs().unwrap(), vec![graph]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn working_graph_save_rejects_a_stale_accepted_head() {
    let root = test_root("stale-working-graph");
    let mut store = ProjectStore::create(&root, "Working Graph").unwrap();
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let first = store.accept(successful_commit(&artifact, None)).unwrap();
    let mut graph = ArtifactWorkingGraph::new(artifact.id, first.id);
    graph
        .add_operator(
            OperatorTypeId::new("text.edit").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    store.save_artifact_working_graph(&graph).unwrap();
    store
        .accept(successful_commit(&artifact, Some(first.id)))
        .unwrap();
    assert!(matches!(
        store.save_artifact_working_graph(&graph),
        Err(StoreError::RevisionConflict { .. })
    ));
    let retained = store.artifact_working_graphs().unwrap();
    assert_eq!(retained.len(), 1);
    assert_ne!(
        retained[0].expected_revision_id(),
        graph.expected_revision_id()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_artifact_and_zero_input_graph_publish_atomically_and_reopen() {
    let root = test_root("source-working-graph");
    let mut store = ProjectStore::create(&root, "Source Working Graph").unwrap();
    let artifact = Artifact::new("Generated image", ArtifactKind::ImageRaster).unwrap();
    let mut graph = ArtifactWorkingGraph::new_source(artifact.id);
    graph
        .add_source_operator(
            OperatorTypeId::new("image.generate").unwrap(),
            OperatorDataTypeId::new("image.raster").unwrap(),
        )
        .unwrap();

    store
        .insert_source_artifact_with_working_graph(&artifact, &graph)
        .unwrap();
    assert_eq!(
        store
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        None
    );
    assert_eq!(
        store.artifact_working_graphs().unwrap(),
        vec![graph.clone()]
    );
    assert!(
        store
            .insert_source_artifact_with_working_graph(&artifact, &graph)
            .is_err()
    );
    drop(store);

    let reopened = ProjectStore::open(&root).unwrap();
    assert_eq!(reopened.artifact_working_graphs().unwrap(), vec![graph]);
    std::fs::remove_dir_all(root).unwrap();
}
