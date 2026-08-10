use shape_domain::{Artifact, ArtifactKind};

use super::*;
use crate::operator_catalog::{
    AUDIO_SPEECH_OPERATOR, IMAGE_CROP_OPERATOR, TEXT_EDIT_OPERATOR, TEXT_TRANSFORM_OPERATOR,
    audio_speech_operation_from_draft, descriptor_for, instruction_from_draft, mode_from_draft,
};
use shape_execution::{
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
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
    drafts
        .update_text_transform_configuration(
            draft.id().as_str(),
            "polish",
            "Make it warmer, but preserve the title.",
        )
        .unwrap();
    let graph = drafts.graph(text.id).unwrap().clone();
    let restored = OperatorDrafts::from_graphs(vec![graph]).unwrap();
    let entries = restored.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, text.id);
    assert_eq!(entries[0].1.id(), draft.id());
    assert_eq!(
        instruction_from_draft(entries[0].1).unwrap(),
        "Make it warmer, but preserve the title."
    );
    assert_eq!(mode_from_draft(entries[0].1).unwrap(), "polish");
    assert!(
        restored
            .clone()
            .update_text_transform_configuration(draft.id().as_str(), "rewrite", "")
            .is_ok()
    );
}

#[test]
fn legacy_unconfigured_speech_draft_receives_executable_preset_defaults() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut graph = ArtifactWorkingGraph::new(text.id, text.accepted_revision.unwrap());
    let legacy = graph
        .add_operator(
            OperatorTypeId::new(AUDIO_SPEECH_OPERATOR).unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("audio.clip").unwrap(),
        )
        .unwrap();
    assert!(legacy.configuration().is_none());
    let mut drafts = OperatorDrafts::from_graphs(vec![graph]).unwrap();
    let changed = drafts.initialize_audio_speech_defaults().unwrap();
    assert_eq!(changed.len(), 1);
    let restored = drafts.entries().next().unwrap().1;
    assert_eq!(restored.id(), legacy.id());
    assert_eq!(
        audio_speech_operation_from_draft(restored)
            .unwrap()
            .unwrap()
            .speed_milli,
        1_000
    );
}

#[test]
fn configuration_is_rejected_for_the_wrong_operator_family() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut drafts = OperatorDrafts::default();
    let edit = drafts
        .begin(
            &text,
            descriptor_for(text.kind, TEXT_EDIT_OPERATOR).unwrap(),
        )
        .unwrap();
    assert!(
        drafts
            .update_text_transform_configuration(edit.id().as_str(), "rewrite", "Make it warmer.",)
            .is_err()
    );
}

#[test]
fn speech_draft_starts_configured_and_restores_exact_authored_pace() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut drafts = OperatorDrafts::default();
    let draft = drafts
        .begin(
            &text,
            descriptor_for(text.kind, AUDIO_SPEECH_OPERATOR).unwrap(),
        )
        .unwrap();
    let initial = audio_speech_operation_from_draft(&draft)
        .unwrap()
        .expect("new speech draft has executable defaults");
    assert_eq!(initial.speed_milli, 1_000);
    let (_, updated) = drafts
        .update_audio_speech_configuration(
            draft.id().as_str(),
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
            INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
            1_200,
            true,
        )
        .unwrap();
    assert_eq!(
        audio_speech_operation_from_draft(&updated)
            .unwrap()
            .unwrap()
            .speed_milli,
        1_200
    );

    let restored = OperatorDrafts::from_graphs(vec![drafts.graph(text.id).unwrap().clone()])
        .expect("configured speech draft restores");
    let restored = restored.entries().next().unwrap().1;
    assert_eq!(restored.id(), draft.id());
    assert_eq!(
        audio_speech_operation_from_draft(restored)
            .unwrap()
            .unwrap()
            .speed_milli,
        1_200
    );
}

#[test]
fn speech_configuration_rejects_other_operator_families() {
    let text = accepted_artifact(ArtifactKind::TextDocument);
    let mut drafts = OperatorDrafts::default();
    let edit = drafts
        .begin(
            &text,
            descriptor_for(text.kind, TEXT_EDIT_OPERATOR).unwrap(),
        )
        .unwrap();
    assert!(
        drafts
            .update_audio_speech_configuration(
                edit.id().as_str(),
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
                1_000,
                true,
            )
            .is_err()
    );
}
