use shape_domain::{Artifact, ArtifactKind};

use super::*;

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
        .begin(&text, TEXT_TRANSFORM_OPERATOR)
        .expect("text transform begins");
    let repeated = drafts
        .begin(&text, TEXT_TRANSFORM_OPERATOR)
        .expect("same draft reuses");
    let speech = drafts
        .begin(&text, AUDIO_SPEECH_OPERATOR)
        .expect("speech begins");
    assert_eq!(first, repeated);
    assert_ne!(first.id, speech.id);
    assert_eq!(drafts.entries().count(), 2);
    assert_eq!(speech.input_data_type, TEXT_DOCUMENT_DATA);
    assert_eq!(speech.output_data_type, AUDIO_CLIP_DATA);

    drafts.finish(text.id, TEXT_TRANSFORM_OPERATOR);
    assert_eq!(drafts.entries().collect::<Vec<_>>(), vec![&speech]);
    drafts.discard(&speech.id).expect("speech draft discards");
    assert_eq!(drafts.entries().count(), 0);
}

#[test]
fn drafts_reject_missing_sources_and_incompatible_operator_types() {
    let text = Artifact::new("Empty", ArtifactKind::TextDocument).expect("artifact is valid");
    let raster = accepted_artifact(ArtifactKind::ImageRaster);
    let mut drafts = OperatorDrafts::default();
    assert!(drafts.begin(&text, TEXT_EDIT_OPERATOR).is_err());
    assert!(drafts.begin(&raster, TEXT_EDIT_OPERATOR).is_err());
    assert!(drafts.begin(&raster, "image.unknown").is_err());
    assert!(drafts.begin(&raster, IMAGE_CROP_OPERATOR).is_ok());
}
