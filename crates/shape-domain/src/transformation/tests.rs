use crate::{
    Artifact, ArtifactKind, Constraint, ConstraintKind, ConstraintStrength, IntentSpec, RevisionId,
};

use super::{Transformation, TransformationKind};

#[test]
fn change_and_preserve_are_distinct_contracts() {
    let artifact = Artifact::new("Portrait", ArtifactKind::ImageComposite).unwrap();
    let constraint = Constraint::new(
        ConstraintKind::PreserveIdentity,
        ConstraintStrength::Hard,
        "character identity",
        Some("subject".to_owned()),
    )
    .unwrap();
    let transformation = Transformation::new(
        TransformationKind::GenerativeEdit,
        artifact.id,
        Vec::new(),
        IntentSpec::new("change the background to a quiet summer afternoon").unwrap(),
        vec![constraint],
        Vec::new(),
    )
    .unwrap();
    assert_eq!(transformation.constraints.len(), 1);
    assert_eq!(
        transformation.intent.as_str(),
        "change the background to a quiet summer afternoon"
    );
}

#[test]
fn duplicate_inputs_are_rejected() {
    let artifact = Artifact::new("Story", ArtifactKind::TextDocument).unwrap();
    let revision = RevisionId::new();
    let result = Transformation::new(
        TransformationKind::TextRewrite,
        artifact.id,
        vec![revision, revision],
        IntentSpec::new("make it quieter").unwrap(),
        Vec::new(),
        Vec::new(),
    );
    assert!(result.is_err());
}
