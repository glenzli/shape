use shape_domain::{Artifact, ArtifactKind};

use super::*;
use crate::operator_catalog::{
    AUDIO_SPEECH_OPERATOR, IMAGE_CROP_OPERATOR, TEXT_EDIT_OPERATOR, TEXT_TRANSFORM_OPERATOR,
    descriptor_for,
};

fn accepted_artifact(kind: ArtifactKind) -> Artifact {
    let mut artifact = Artifact::new("Source", kind).expect("artifact is valid");
    artifact.accepted_revision = Some(shape_domain::RevisionId::new());
    artifact
}

#[test]
fn compatible_operator_drafts_are_identity_addressed_and_deduplicated() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut drafts = OperatorDrafts::default();
    let first = drafts
        .begin(
            &text,
            descriptor_for(text.kind, TEXT_TRANSFORM_OPERATOR).unwrap(),
        )
        .expect("text transform begins");
    let repeated = drafts
        .begin(
            &text,
            descriptor_for(text.kind, TEXT_TRANSFORM_OPERATOR).unwrap(),
        )
        .expect("same draft reuses");
    let speech = drafts
        .begin(
            &text,
            descriptor_for(text.kind, AUDIO_SPEECH_OPERATOR).unwrap(),
        )
        .expect("speech begins");
    assert_eq!(first, repeated);
    assert_ne!(first.id(), speech.id());
    assert_eq!(drafts.entries().count(), 2);
    assert_eq!(speech.input_data_type().as_str(), "text.document");
    assert_eq!(speech.output_data_type().as_str(), "audio.clip");

    drafts.finish(text.id, TEXT_TRANSFORM_OPERATOR);
    assert_eq!(
        drafts
            .entries()
            .map(|(_, operator)| operator)
            .collect::<Vec<_>>(),
        vec![&speech]
    );
    drafts
        .discard(speech.id().as_str())
        .expect("speech draft discards");
    assert_eq!(drafts.entries().count(), 0);
}

#[test]
fn drafts_reject_missing_sources_and_incompatible_operator_types() {
    let text = Artifact::new("Empty", ArtifactKind::TextDocument).expect("artifact is valid");
    let raster = accepted_artifact(ArtifactKind::ImageRaster);
    let mut drafts = OperatorDrafts::default();
    let text_edit = descriptor_for(ArtifactKind::TextDocument, TEXT_EDIT_OPERATOR).unwrap();
    let image_crop = descriptor_for(ArtifactKind::ImageRaster, IMAGE_CROP_OPERATOR).unwrap();
    assert!(drafts.begin(&text, text_edit).is_err());
    assert!(descriptor_for(raster.kind, TEXT_EDIT_OPERATOR).is_err());
    assert!(descriptor_for(raster.kind, "image.unknown").is_err());
    assert!(drafts.begin(&raster, image_crop).is_ok());
}

#[test]
fn project_backed_graphs_restore_exact_draft_identity() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut drafts = OperatorDrafts::default();
    let draft = drafts
        .begin(
            &text,
            descriptor_for(text.kind, TEXT_TRANSFORM_OPERATOR).unwrap(),
        )
        .unwrap();
    let graph = drafts.graph(text.id).unwrap().clone();
    let restored = OperatorDrafts::from_graphs(vec![graph]).unwrap();
    let entries = restored.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, text.id);
    assert_eq!(entries[0].1.id(), draft.id());
}
