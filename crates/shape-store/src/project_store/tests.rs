use std::fs;

use shape_domain::{Constraint, IntentSpec, TransformationKind};
use shape_execution::{CapabilityId, ExecutionJob, ExecutorIdentity};

use super::*;

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("shape-project-{label}-{}", Uuid::now_v7()))
}

fn successful_commit(artifact: &Artifact, expected_head: Option<RevisionId>) -> AcceptedCommit {
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
        output_bytes: b"accepted content".to_vec(),
        output_media_type: "text/plain; charset=utf-8".to_owned(),
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
        transformation,
        receipt,
        output_bytes: b"branched content".to_vec(),
        output_media_type: "text/plain; charset=utf-8".to_owned(),
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
