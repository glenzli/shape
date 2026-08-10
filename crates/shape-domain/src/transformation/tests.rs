use crate::{
    Artifact, ArtifactKind, Constraint, ConstraintKind, ConstraintStrength, IntentSpec, RasterCrop,
    RevisionId, TransformationOperation,
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

#[test]
fn typed_crop_operation_belongs_only_to_deterministic_edits() {
    let artifact = Artifact::new("Portrait", ArtifactKind::ImageRaster).unwrap();
    let operation =
        TransformationOperation::RasterCrop(RasterCrop::new(1, 2, 30, 20, 100, 80).unwrap());
    let deterministic = Transformation::new_with_operation(
        TransformationKind::DeterministicEdit,
        artifact.id,
        vec![RevisionId::new()],
        IntentSpec::new("crop the raster").unwrap(),
        Vec::new(),
        Vec::new(),
        Some(operation.clone()),
    )
    .unwrap();
    assert_eq!(deterministic.operation, Some(operation.clone()));

    let invalid = Transformation::new_with_operation(
        TransformationKind::GenerativeEdit,
        artifact.id,
        Vec::new(),
        IntentSpec::new("create a variation").unwrap(),
        Vec::new(),
        Vec::new(),
        Some(operation),
    );
    assert!(invalid.is_err());
}
