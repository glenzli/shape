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

#[test]
fn project_reopens_with_verified_accepted_content() {
    let root = test_root("reopen");
    let mut store = ProjectStore::create(&root, "Test Project").expect("project creates");
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).expect("artifact valid");
    store.insert_artifact(&artifact).expect("artifact inserts");
    let revision = store
        .accept(successful_commit(&artifact, None))
        .expect("commit accepts");
    drop(store);

    let reopened = ProjectStore::open(&root).expect("project reopens");
    let accepted = reopened
        .accepted_revision(artifact.id)
        .expect("revision reads")
        .expect("accepted revision exists");
    assert_eq!(accepted.id, revision.id);
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
