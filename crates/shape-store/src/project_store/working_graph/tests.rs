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

#[test]
fn concurrent_graph_replacement_has_one_winner_and_preserves_the_winner() {
    use std::sync::{Arc, Barrier};
    let root = test_root("parallel-draft-cas");
    let mut store = ProjectStore::create(&root, "Concurrent drafts").unwrap();
    let artifact = Artifact::new("Image", ArtifactKind::ImageRaster).unwrap();
    let mut original = ArtifactWorkingGraph::new_source(artifact.id);
    original
        .add_source_operator(
            OperatorTypeId::new("image.generate").unwrap(),
            OperatorDataTypeId::new("image.raster").unwrap(),
        )
        .unwrap();
    store
        .insert_source_artifact_with_working_graph(&artifact, &original)
        .unwrap();
    for contenders in [2, 4, 8] {
        let original = store.artifact_working_graph(artifact.id).unwrap().unwrap();
        let barrier = Arc::new(Barrier::new(contenders));
        let mut workers = Vec::new();
        for _ in 0..contenders {
            let root = root.clone();
            let original = original.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                let store = ProjectStore::open(root).unwrap();
                let mut next = original.clone();
                assert!(
                    next.set_operator_configuration(
                        original.operators()[0].id(),
                        Some(
                            shape_domain::WorkingOperatorConfiguration::new(
                                shape_domain::OperatorConfigurationSchemaId::new(
                                    "shape.test.concurrent"
                                )
                                .unwrap(),
                                serde_json::json!({"writer": uuid::Uuid::now_v7().to_string()})
                                    .to_string(),
                            )
                            .unwrap()
                        ),
                    )
                );
                barrier.wait();
                match store.replace_artifact_working_graph(
                    artifact.id,
                    Some(&original),
                    Some(&next),
                ) {
                    Ok(()) => Some(next),
                    Err(StoreError::WorkingGraphConflict) => None,
                    other => panic!("unexpected replacement result: {other:?}"),
                }
            }));
        }
        let winners: Vec<_> = workers
            .into_iter()
            .filter_map(|worker| worker.join().unwrap())
            .collect();
        assert_eq!(winners.len(), 1);
        assert_eq!(
            store.artifact_working_graph(artifact.id).unwrap().as_ref(),
            winners.first()
        );
        assert!(
            store
                .artifact(artifact.id)
                .unwrap()
                .unwrap()
                .accepted_revision
                .is_none()
        );
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn graph_replacement_rejects_stale_creation_and_invalid_identity_without_mutation() {
    let root = test_root("graph-cas-rollback");
    let mut store = ProjectStore::create(&root, "Concurrent drafts").unwrap();
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
    store
        .replace_artifact_working_graph(artifact.id, None, Some(&graph))
        .unwrap();
    assert!(matches!(
        store.replace_artifact_working_graph(artifact.id, None, Some(&graph)),
        Err(StoreError::WorkingGraphConflict)
    ));
    let other = Artifact::new("Other", ArtifactKind::TextDocument).unwrap();
    assert!(
        store
            .replace_artifact_working_graph(other.id, Some(&graph), None)
            .is_err()
    );
    assert_eq!(
        store.artifact_working_graph(artifact.id).unwrap(),
        Some(graph.clone())
    );
    store
        .replace_artifact_working_graph(artifact.id, Some(&graph), None)
        .unwrap();
    assert!(store.artifact_working_graph(artifact.id).unwrap().is_none());
    assert_eq!(
        store
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(revision.id)
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn graph_replacement_cannot_restore_or_discard_a_graph_after_acceptance_rebases_it() {
    let root = test_root("graph-cas-accepted-head");
    let mut store = ProjectStore::create(&root, "Accepted draft").unwrap();
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let first = store.accept(successful_commit(&artifact, None)).unwrap();
    let mut graph = ArtifactWorkingGraph::new(artifact.id, first.id);
    graph
        .add_operator(
            OperatorTypeId::new("text.transform").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    store
        .replace_artifact_working_graph(artifact.id, None, Some(&graph))
        .unwrap();
    let second = store
        .accept(successful_commit(&artifact, Some(first.id)))
        .unwrap();
    assert!(matches!(
        store.replace_artifact_working_graph(artifact.id, Some(&graph), None),
        Err(StoreError::WorkingGraphConflict)
    ));
    assert!(matches!(
        store.replace_artifact_working_graph(artifact.id, Some(&graph), Some(&graph)),
        Err(StoreError::WorkingGraphConflict)
    ));
    assert_eq!(
        store
            .artifact_working_graph(artifact.id)
            .unwrap()
            .unwrap()
            .expected_revision_id(),
        Some(second.id)
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
