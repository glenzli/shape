use super::*;
use crate::project_store::tests::{successful_commit, test_root};
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

fn accepted_image(store: &mut ProjectStore) -> WorkingInput {
    let artifact = Artifact::new("Picture", ArtifactKind::ImageRaster).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let mut commit = successful_commit(&artifact, None);
    commit.output_media_type = "image/png".into();
    commit.output_bytes = vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 80, 8, 168, 248, 15,
        0, 3, 100, 1, 232, 53, 225, 87, 172, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ]
    .into();
    commit.content_contract = Some(ArtifactContentContract::ImageRaster(
        shape_domain::ImageRasterContract::rgba8(1, 1, shape_domain::ImageColorProfile::Srgb)
            .unwrap(),
    ));
    let revision = store.accept(commit).unwrap();
    WorkingInput {
        artifact_id: artifact.id,
        revision_id: revision.id,
    }
}

#[test]
fn exact_material_read_preserves_live_files_and_refuses_old_schema_without_migrating() {
    let root = test_root("readonly-material");
    let mut store = ProjectStore::create(&root, "Material").unwrap();
    let source = accepted_image(&mut store);
    let paths = [
        root.join(MANIFEST_FILE),
        root.join(DATABASE_FILE),
        root.join("project.sqlite-wal"),
    ];
    let before: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    let material = ProjectStore::inspect_material(&root, source).unwrap();
    assert_eq!(
        material.revision,
        store.revision(source.revision_id).unwrap()
    );
    assert_eq!(
        material.artifact.accepted_revision,
        Some(source.revision_id)
    );
    assert!(material.material_edit.is_none());
    for (path, bytes) in paths.iter().zip(&before) {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
    let mut manifest: serde_json::Value = serde_json::from_slice(&before[0]).unwrap();
    manifest["metadata"]["schema_revision"] = "20260811.1".into();
    let old_manifest = serde_json::to_vec(&manifest).unwrap();
    fs::write(&paths[0], &old_manifest).unwrap();
    assert!(matches!(
        ProjectStore::inspect_material(&root, source),
        Err(StoreError::SchemaMismatch { .. })
    ));
    assert_eq!(fs::read(&paths[0]).unwrap(), old_manifest);
    for (path, bytes) in paths[1..].iter().zip(&before[1..]) {
        assert_eq!(&fs::read(path).unwrap(), bytes);
    }
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_material_read_bounds_each_record_before_deserializing() {
    let root = test_root("bounded-material");
    let mut store = ProjectStore::create(&root, "Material limits").unwrap();
    let source = accepted_image(&mut store);
    let revision = store.revision(source.revision_id).unwrap();
    for (table, column, id) in [
        (
            "artifacts",
            "accepted_revision",
            source.artifact_id.to_string(),
        ),
        (
            "artifact_revisions",
            "revision_json",
            source.revision_id.to_string(),
        ),
        (
            "transformations",
            "transformation_json",
            revision.transformation_id.to_string(),
        ),
    ] {
        let select = format!("SELECT {column} FROM {table} WHERE id = ?1");
        let original: String = store
            .connection
            .query_row(&select, [&id], |r| r.get(0))
            .unwrap();
        let update = format!("UPDATE {table} SET {column} = ?1 WHERE id = ?2");
        store
            .connection
            .execute(
                &update,
                rusqlite::params![
                    " ".repeat(usize::try_from(MAX_METADATA_BYTES + 1).unwrap()),
                    id
                ],
            )
            .unwrap();
        assert!(
            matches!(
                ProjectStore::inspect_material(&root, source),
                Err(StoreError::InspectionLimit {
                    resource: "metadata bytes",
                    ..
                })
            ),
            "{table}.{column}"
        );
        store
            .connection
            .execute(&update, rusqlite::params![original, id])
            .unwrap();
    }
    ProjectStore::inspect_material(&root, source).unwrap();
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exact_material_read_rejects_embedded_revision_and_transformation_identity_mismatch() {
    let root = test_root("material-record-identity");
    let mut store = ProjectStore::create(&root, "Material identity").unwrap();
    let source = accepted_image(&mut store);
    let revision = store.revision(source.revision_id).unwrap();
    for (table, column, id) in [
        (
            "artifact_revisions",
            "revision_json",
            source.revision_id.to_string(),
        ),
        (
            "transformations",
            "transformation_json",
            revision.transformation_id.to_string(),
        ),
    ] {
        let select = format!("SELECT {column} FROM {table} WHERE id = ?1");
        let original: String = store
            .connection
            .query_row(&select, [&id], |r| r.get(0))
            .unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(&original).unwrap();
        changed["id"] = shape_domain::RevisionId::new().to_string().into();
        let update = format!("UPDATE {table} SET {column} = ?1 WHERE id = ?2");
        store
            .connection
            .execute(&update, rusqlite::params![changed.to_string(), id])
            .unwrap();
        assert!(
            matches!(
                ProjectStore::inspect_material(&root, source),
                Err(StoreError::InvalidInspection(_))
            ),
            "{table}"
        );
        store
            .connection
            .execute(&update, rusqlite::params![original, id])
            .unwrap();
    }
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
