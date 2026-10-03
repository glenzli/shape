use super::*;
use crate::project_store::tests::test_root;
use shape_domain::{Artifact, ArtifactKind, OperatorDataTypeId, OperatorTypeId};
use std::fs;

#[test]
fn sees_live_wal_state_without_changing_manifest_database_or_wal_bytes() {
    let root = test_root("readonly-inspection");
    let store = ProjectStore::create(&root, "Inspect").unwrap();
    let artifact = Artifact::new("Live unsaved output", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let paths = [
        root.join(MANIFEST_FILE),
        root.join(DATABASE_FILE),
        root.join("project.sqlite-wal"),
    ];
    let before: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    assert_eq!(
        ProjectStore::inspect(&root).unwrap().snapshot.artifacts,
        vec![artifact]
    );
    for (path, bytes) in paths.iter().zip(before) {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    let reader = open_for_inspection(&root).unwrap();
    assert!(
        reader
            .connection
            .execute("DELETE FROM artifacts", [])
            .is_err()
    );
    drop(reader);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_oversize_collections_and_graph_payloads_before_deserialization() {
    let root = test_root("bounded-inspection");
    let mut store = ProjectStore::create(&root, "Limits").unwrap();
    let transaction = store.connection.transaction().unwrap();
    for i in 0..=MAX_COLLECTION_ROWS {
        let artifact = Artifact::new(format!("Item {i}"), ArtifactKind::TextDocument).unwrap();
        transaction
            .execute(
                "INSERT INTO artifacts VALUES (?1, ?2, ?3, NULL)",
                rusqlite::params![
                    artifact.id.to_string(),
                    artifact.name,
                    serde_json::to_string(&artifact.kind).unwrap()
                ],
            )
            .unwrap();
    }
    transaction.commit().unwrap();
    assert!(matches!(
        ProjectStore::inspect(&root),
        Err(StoreError::InspectionLimit {
            resource: "collection rows",
            ..
        })
    ));
    store
        .connection
        .execute("DELETE FROM artifacts", [])
        .unwrap();
    let artifact = Artifact::new("Large graph", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    store
        .connection
        .execute(
            "INSERT INTO artifact_working_graphs VALUES (?1, ?2)",
            rusqlite::params![
                artifact.id.to_string(),
                " ".repeat(usize::try_from(MAX_METADATA_BYTES + 1).unwrap())
            ],
        )
        .unwrap();
    assert!(matches!(
        ProjectStore::inspect(&root),
        Err(StoreError::InspectionLimit {
            resource: "metadata bytes",
            ..
        })
    ));
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_metadata_disagreement_without_repairing_it() {
    let root = test_root("metadata-inspection");
    let store = ProjectStore::create(&root, "Metadata").unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(MANIFEST_FILE)).unwrap()).unwrap();
    manifest["metadata"]["name"] = "Changed only in manifest".into();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    fs::write(root.join(MANIFEST_FILE), &bytes).unwrap();
    assert!(matches!(
        ProjectStore::inspect(&root),
        Err(StoreError::MetadataMismatch)
    ));
    assert_eq!(fs::read(root.join(MANIFEST_FILE)).unwrap(), bytes);
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspection_keeps_heads_and_saved_graphs_in_one_read_snapshot() {
    let root = test_root("coherent-inspection");
    let mut store = ProjectStore::create(&root, "Concurrent reader").unwrap();
    let artifact = Artifact::new("0", ArtifactKind::TextDocument).unwrap();
    let mut graph = ArtifactWorkingGraph::new_source(artifact.id);
    graph
        .add_source_operator(
            OperatorTypeId::new("text.create").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    store
        .insert_source_artifact_with_working_graph(&artifact, &graph)
        .unwrap();
    let writer_root = root.clone();
    let writer = std::thread::spawn(move || {
        let mut writer = ProjectStore::open(writer_root).unwrap();
        for i in 1..=40 {
            let transaction = writer.connection.transaction().unwrap();
            transaction
                .execute(
                    "UPDATE artifacts SET name=?1 WHERE id=?2",
                    rusqlite::params![i.to_string(), artifact.id.to_string()],
                )
                .unwrap();
            // A matching scene name provides a second table in the same accepted transaction.
            transaction.execute("DELETE FROM scenes", []).unwrap();
            let scene = shape_domain::Scene::new(i.to_string()).unwrap();
            transaction
                .execute(
                    "INSERT INTO scenes VALUES (?1,?2,NULL)",
                    rusqlite::params![scene.id.to_string(), scene.name],
                )
                .unwrap();
            transaction.commit().unwrap();
        }
    });
    for _ in 0..40 {
        let state = ProjectStore::inspect(&root).unwrap();
        if let Some(scene) = state.snapshot.scenes.first() {
            assert_eq!(state.snapshot.artifacts[0].name, scene.name);
        } else {
            assert_eq!(state.snapshot.artifacts[0].name, "0");
        }
        assert_eq!(state.working_graphs.len(), 1);
    }
    writer.join().unwrap();
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
