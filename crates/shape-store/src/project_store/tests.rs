use std::fs;

use shape_domain::{
    Constraint, IntentSpec, NamedSceneOutput, OperatorDataTypeId, OperatorGraph, OperatorGraphEdge,
    OperatorGraphNode, OperatorNodeBinding, OperatorNodeId, OperatorNodeRole, OperatorPort,
    OperatorPortId, OperatorTypeId, Scene, SceneOutputName, TransformationKind,
};
use shape_execution::{CapabilityId, ExecutionJob, ExecutorIdentity};

use super::*;

pub(super) fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("shape-project-{label}-{}", Uuid::now_v7()))
}

pub(super) fn successful_commit(
    artifact: &Artifact,
    expected_head: Option<RevisionId>,
) -> AcceptedCommit {
    let transformation = Transformation::new(
        if expected_head.is_some() {
            TransformationKind::TextRewrite
        } else {
            TransformationKind::Import
        },
        artifact.id,
        expected_head.into_iter().collect(),
        IntentSpec::new("store test").expect("valid intent"),
        Vec::<Constraint>::new(),
        Vec::new(),
    )
    .expect("valid transformation");
    let capability = CapabilityId::new("text.test").expect("valid capability");
    let mut job = ExecutionJob::new(transformation.id, capability);
    let attempt = job
        .start(
            ExecutorIdentity::new("shape.test", "1", "20260810.1").expect("valid identity"),
            10,
        )
        .expect("job starts");
    let receipt = job.succeed(attempt, 20).expect("job succeeds");
    AcceptedCommit {
        artifact_id: artifact.id,
        expected_head,
        transformation,
        receipt,
        output_bytes: b"accepted content".to_vec().into(),
        output_media_type: "text/plain; charset=utf-8".to_owned(),
        content_contract: None,
    }
}

fn successful_new_artifact_commit(
    artifact: Artifact,
    source_revision: RevisionId,
) -> NewArtifactCommit {
    let transformation = Transformation::new(
        TransformationKind::TextRewrite,
        artifact.id,
        vec![source_revision],
        IntentSpec::new("branch store test").expect("valid intent"),
        Vec::<Constraint>::new(),
        Vec::new(),
    )
    .expect("valid transformation");
    let capability = CapabilityId::new("text.test").expect("valid capability");
    let mut job = ExecutionJob::new(transformation.id, capability);
    let attempt = job
        .start(
            ExecutorIdentity::new("shape.test", "1", "20260810.1").expect("valid identity"),
            30,
        )
        .expect("job starts");
    let receipt = job.succeed(attempt, 40).expect("job succeeds");
    NewArtifactCommit {
        artifact,
        expected_input_heads: Vec::new(),
        transformation,
        receipt,
        output_bytes: b"branched content".to_vec().into(),
        output_media_type: "text/plain; charset=utf-8".to_owned(),
        content_contract: None,
    }
}

fn scene_commit(
    scene: &Scene,
    expected_head: Option<SceneRevisionId>,
    artifact: &Artifact,
    revision: &ArtifactRevision,
) -> AcceptedSceneRevisionCommit {
    let source_id = OperatorNodeId::source(revision.id);
    let output_id = OperatorNodeId::output(artifact.id);
    let data_type = OperatorDataTypeId::new("text.document").unwrap();
    AcceptedSceneRevisionCommit {
        scene_id: scene.id,
        expected_head,
        graph: OperatorGraph::new(
            vec![
                OperatorGraphNode::new(
                    source_id.clone(),
                    OperatorNodeRole::Source,
                    OperatorTypeId::new("source.text.document").unwrap(),
                    OperatorNodeBinding::Source {
                        artifact_id: artifact.id,
                        revision_id: revision.id,
                    },
                    Vec::new(),
                    vec![OperatorPort::new(
                        OperatorPortId::new("output.value").unwrap(),
                        data_type.clone(),
                    )],
                )
                .unwrap(),
                OperatorGraphNode::new(
                    output_id.clone(),
                    OperatorNodeRole::Output,
                    OperatorTypeId::new("output.text.document").unwrap(),
                    OperatorNodeBinding::Output {
                        artifact_id: artifact.id,
                        revision_id: revision.id,
                    },
                    vec![OperatorPort::new(
                        OperatorPortId::new("input.value").unwrap(),
                        data_type,
                    )],
                    Vec::new(),
                )
                .unwrap(),
            ],
            vec![OperatorGraphEdge::new(
                source_id,
                OperatorPortId::new("output.value").unwrap(),
                output_id.clone(),
                OperatorPortId::new("input.value").unwrap(),
            )],
        )
        .unwrap(),
        outputs: vec![NamedSceneOutput::new(
            SceneOutputName::new("main").unwrap(),
            output_id,
        )],
    }
}

#[test]
fn project_reopens_with_verified_accepted_content() {
    let root = test_root("reopen");
    let mut store = ProjectStore::create(&root, "Test Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).expect("artifact valid");
    store.insert_artifact(&artifact).expect("artifact inserts");
    let commit = successful_commit(&artifact, None);
    let expected_transformation = commit.transformation.clone();
    let revision = store.accept(commit).expect("commit accepts");
    drop(store);

    let reopened = ProjectStore::open(&root).expect("project reopens");
    let accepted = reopened
        .accepted_revision(artifact.id)
        .expect("revision reads")
        .expect("accepted revision exists");
    assert_eq!(accepted.id, revision.id);
    assert_eq!(
        reopened
            .transformation(accepted.transformation_id)
            .expect("transformation reads"),
        expected_transformation
    );
    assert_eq!(
        reopened
            .read_content(&accepted.content)
            .expect("content verifies"),
        b"accepted content"
    );
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn stale_acceptance_cannot_overwrite_the_head() {
    let root = test_root("conflict");
    let mut store = ProjectStore::create(&root, "Test Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).expect("artifact valid");
    store.insert_artifact(&artifact).expect("artifact inserts");
    store
        .accept(successful_commit(&artifact, None))
        .expect("first commit accepts");

    assert!(matches!(
        store.accept(successful_commit(&artifact, None)),
        Err(StoreError::RevisionConflict { .. })
    ));
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn new_artifact_commit_is_atomic_and_preserves_cross_artifact_lineage() {
    let root = test_root("branch");
    let mut store = ProjectStore::create(&root, "Test Project").expect("project creates");
    let source = Artifact::new("Story", ArtifactKind::TextDocument).expect("artifact valid");
    store.insert_artifact(&source).expect("artifact inserts");
    let source_revision = store
        .accept(successful_commit(&source, None))
        .expect("source accepts");
    let target =
        Artifact::new("Story — Branch", ArtifactKind::TextDocument).expect("target artifact valid");
    let target_id = target.id;
    let revision = store
        .accept_new_artifact(successful_new_artifact_commit(target, source_revision.id))
        .expect("branch accepts");

    assert!(revision.parents.is_empty());
    assert_eq!(revision.artifact_id, target_id);
    assert_eq!(
        store
            .accepted_revision(source.id)
            .expect("source reads")
            .expect("source head exists")
            .id,
        source_revision.id
    );
    let transformation = store
        .transformation(revision.transformation_id)
        .expect("transformation reads");
    assert_eq!(transformation.inputs, vec![source_revision.id]);
    drop(store);

    let reopened = ProjectStore::open(&root).expect("project reopens");
    let reopened_target = reopened
        .artifact(target_id)
        .expect("target reads")
        .expect("target exists");
    assert_eq!(reopened_target.accepted_revision, Some(revision.id));
    assert_eq!(
        reopened
            .read_content(
                &reopened
                    .revision(revision.id)
                    .expect("revision reads")
                    .content,
            )
            .expect("content verifies"),
        b"branched content"
    );
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn two_scenes_evolve_independently_and_stale_graphs_cannot_overwrite_heads() {
    let root = test_root("scenes");
    let mut store = ProjectStore::create(&root, "Scene Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).expect("artifact valid");
    store.insert_artifact(&artifact).expect("artifact inserts");
    let artifact_revision = store
        .accept(successful_commit(&artifact, None))
        .expect("artifact accepts");
    let first_scene = Scene::new("Opening").expect("scene valid");
    let second_scene = Scene::new("Ending").expect("scene valid");
    store
        .insert_scene(&first_scene)
        .expect("first scene inserts");
    store
        .insert_scene(&second_scene)
        .expect("second scene inserts");

    let first_revision = store
        .accept_scene_revision(scene_commit(
            &first_scene,
            None,
            &artifact,
            &artifact_revision,
        ))
        .expect("first scene graph accepts");
    let second_revision = store
        .accept_scene_revision(scene_commit(
            &second_scene,
            None,
            &artifact,
            &artifact_revision,
        ))
        .expect("second scene graph accepts");
    let next_first_revision = store
        .accept_scene_revision(scene_commit(
            &first_scene,
            Some(first_revision.id),
            &artifact,
            &artifact_revision,
        ))
        .expect("first scene advances independently");
    assert!(matches!(
        store.accept_scene_revision(scene_commit(
            &first_scene,
            Some(first_revision.id),
            &artifact,
            &artifact_revision,
        )),
        Err(StoreError::SceneRevisionConflict { .. })
    ));
    assert_eq!(
        store
            .scene(second_scene.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(second_revision.id)
    );
    drop(store);

    let reopened = ProjectStore::open(&root).expect("project reopens");
    let snapshot = reopened.snapshot().expect("snapshot reads");
    assert_eq!(snapshot.scenes.len(), 2);
    assert_eq!(
        reopened
            .scene(first_scene.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(next_first_revision.id)
    );
    assert_eq!(
        reopened
            .scene_revision(next_first_revision.id)
            .unwrap()
            .parent,
        Some(first_revision.id)
    );
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn initial_schema_is_additively_migrated_when_project_reopens() {
    let root = test_root("scene-migration");
    let store = ProjectStore::create(&root, "Migration Project").expect("project creates");
    let mut metadata = store.metadata().clone();
    drop(store);

    metadata.schema_revision = schema::INITIAL_SCHEMA_REVISION.to_owned();
    let connection = Connection::open(root.join(DATABASE_FILE)).expect("database opens");
    connection
        .execute_batch(
            "DROP TABLE artifact_working_graphs; DROP TABLE scene_revisions; DROP TABLE scenes;",
        )
        .expect("scene tables drop");
    connection
        .execute(
            "UPDATE project_singleton SET metadata_json = ?1 WHERE singleton = 1",
            [serde_json::to_string(&metadata).unwrap()],
        )
        .expect("database metadata downgrades");
    drop(connection);
    write_manifest(&root, &metadata).expect("old manifest writes");

    let reopened = ProjectStore::open(&root).expect("old project migrates");
    assert_eq!(
        reopened.metadata().schema_revision,
        SHAPE_PROJECT_SCHEMA_REVISION
    );
    let scene = Scene::new("Migrated Scene").unwrap();
    reopened.insert_scene(&scene).expect("scene table exists");
    drop(reopened);
    let reopened_again = ProjectStore::open(&root).expect("migration is durable");
    assert_eq!(reopened_again.scenes().unwrap(), vec![scene]);
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn scene_schema_upgrades_to_current_without_changing_existing_heads() {
    let root = test_root("audio-schema-migration");
    let mut store = ProjectStore::create(&root, "Migration Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let artifact_revision = store.accept(successful_commit(&artifact, None)).unwrap();
    let scene = Scene::new("Opening").unwrap();
    store.insert_scene(&scene).unwrap();
    let scene_revision = store
        .accept_scene_revision(scene_commit(&scene, None, &artifact, &artifact_revision))
        .unwrap();
    let mut metadata = store.metadata().clone();
    drop(store);

    metadata.schema_revision = schema::SCENE_SCHEMA_REVISION.to_owned();
    let connection = Connection::open(root.join(DATABASE_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE artifact_working_graphs;")
        .unwrap();
    connection
        .execute(
            "UPDATE project_singleton SET metadata_json = ?1 WHERE singleton = 1",
            [serde_json::to_string(&metadata).unwrap()],
        )
        .unwrap();
    drop(connection);
    write_manifest(&root, &metadata).unwrap();

    let mut reopened = ProjectStore::open(&root).expect("Scene schema upgrades");
    assert_eq!(
        reopened.metadata().schema_revision,
        SHAPE_PROJECT_SCHEMA_REVISION
    );
    assert_eq!(
        reopened
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(artifact_revision.id)
    );
    assert_eq!(
        reopened.scene(scene.id).unwrap().unwrap().accepted_revision,
        Some(scene_revision.id)
    );
    reopened
        .accept_scene_revision(scene_commit(
            &scene,
            Some(scene_revision.id),
            &artifact,
            &artifact_revision,
        ))
        .expect("Scene CAS still works after audio schema upgrade");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn audio_schema_adds_working_graph_storage_without_changing_artifact_heads() {
    let root = test_root("working-graph-schema-migration");
    let mut store = ProjectStore::create(&root, "Migration Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let accepted = store.accept(successful_commit(&artifact, None)).unwrap();
    let mut metadata = store.metadata().clone();
    drop(store);

    metadata.schema_revision = schema::AUDIO_SCHEMA_REVISION.to_owned();
    let connection = Connection::open(root.join(DATABASE_FILE)).unwrap();
    connection
        .execute_batch("DROP TABLE artifact_working_graphs;")
        .unwrap();
    connection
        .execute(
            "UPDATE project_singleton SET metadata_json = ?1 WHERE singleton = 1",
            [serde_json::to_string(&metadata).unwrap()],
        )
        .unwrap();
    drop(connection);
    write_manifest(&root, &metadata).unwrap();

    let reopened = ProjectStore::open(&root).expect("audio schema upgrades");
    assert_eq!(
        reopened.metadata().schema_revision,
        SHAPE_PROJECT_SCHEMA_REVISION
    );
    assert_eq!(
        reopened
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision,
        Some(accepted.id)
    );
    assert!(reopened.artifact_working_graphs().unwrap().is_empty());
    fs::remove_dir_all(&root).unwrap();
}
