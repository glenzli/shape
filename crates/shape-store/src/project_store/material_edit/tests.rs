use super::super::{ProjectStore, tests::successful_commit};
use shape_domain::{
    Artifact, ArtifactKind, ContentDigest, IntentSpec, MaterialEditOperation, MaterialEditor,
    OperatorNodeId, RevisionId, Transformation, TransformationKind, TransformationOperation,
    WorkingInput,
};

#[test]
fn external_return_cannot_bypass_the_authored_stage_requirement() {
    let root = std::env::temp_dir().join(format!("shape-store-material-{}", uuid::Uuid::now_v7()));
    let mut store = ProjectStore::create(&root, "Store defense").unwrap();
    let artifact = Artifact::new("Target", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let source = WorkingInput {
        artifact_id: shape_domain::ArtifactId::new(),
        revision_id: RevisionId::new(),
    };
    let mut commit = successful_commit(&artifact, None);
    commit.transformation = Transformation::new_with_operation(
        TransformationKind::ExternalRoundTrip,
        artifact.id,
        vec![source.revision_id],
        IntentSpec::new("forged").unwrap(),
        vec![],
        vec![],
        Some(TransformationOperation::MaterialEdit(
            MaterialEditOperation {
                editor: MaterialEditor::Shadow,
                stage_id: OperatorNodeId::new("draft.test").unwrap(),
                session: 1,
                source,
                input_digest: ContentDigest::from_bytes(b"input"),
                returned_digest: ContentDigest::from_bytes(b"output"),
                normalization: shape_domain::MaterialReturnNormalization::Canonical,
            },
        )),
    )
    .unwrap();
    commit.receipt.transformation_id = commit.transformation.id;
    assert!(store.accept(commit).is_err());
    assert!(
        store
            .artifact(artifact.id)
            .unwrap()
            .unwrap()
            .accepted_revision
            .is_none()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
