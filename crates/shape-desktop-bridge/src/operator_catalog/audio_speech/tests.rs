use shape_domain::{
    OperatorDataTypeId, OperatorTypeId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};
use shape_execution::{
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
};

use super::*;

fn audio_speech_draft() -> WorkingOperatorDraft {
    WorkingOperatorDraft::new(
        OperatorTypeId::new(AUDIO_SPEECH_OPERATOR).unwrap(),
        OperatorDataTypeId::new("text.document").unwrap(),
        OperatorDataTypeId::new("audio.clip").unwrap(),
    )
}

fn configured_draft(speed_milli: u16) -> WorkingOperatorDraft {
    let draft = audio_speech_draft();
    let mut graph = shape_domain::ArtifactWorkingGraph::new(
        shape_domain::ArtifactId::new(),
        shape_domain::RevisionId::new(),
    );
    let stored = graph
        .add_operator(
            draft.operator_type().clone(),
            draft.input_data_type().clone(),
            draft.output_data_type().clone(),
        )
        .unwrap();
    graph.set_operator_configuration(
        stored.id(),
        Some(
            configuration_for_audio_speech(
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
                speed_milli,
                true,
            )
            .unwrap(),
        ),
    );
    graph.operators()[0].clone()
}

#[test]
fn preset_only_configuration_round_trips_through_the_domain_operation() {
    let draft = configured_draft(1_150);
    let operation = audio_speech_operation_from_draft(&draft)
        .unwrap()
        .expect("speech draft is configured");
    assert_eq!(
        operation.language,
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE
    );
    assert_eq!(operation.speed_milli, 1_150);
    assert!(operation.synthetic_disclosure_required);
    let shape_domain::SpeechVoiceSelection::Preset(selection) = operation.voice else {
        panic!("authorized voice references are never admitted by this schema");
    };
    assert_eq!(
        selection.alias.as_str(),
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1
    );
    assert_eq!(
        selection.catalog_revision,
        INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION
    );
}

#[test]
fn unsupported_preset_speed_or_disclosure_fails_closed() {
    assert!(
        configuration_for_audio_speech(
            "speech.voice.other.v1",
            INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
            1_000,
            true,
        )
        .is_err()
    );
    assert!(
        configuration_for_audio_speech(
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
            INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
            0,
            true,
        )
        .is_err()
    );
    assert!(
        configuration_for_audio_speech(
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
            INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
            1_000,
            false,
        )
        .is_err()
    );
}

#[test]
fn unknown_fields_and_authorized_reference_shapes_fail_exact_decoding() {
    let schema = shape_domain::OperatorConfigurationSchemaId::new(
        "shape.operator-draft.audio-speech@20260811.1",
    )
    .unwrap();
    for json in [
        format!(
            r#"{{"preset_alias":"{INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1}","preset_catalog_revision":"{INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION}","language":"{INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE}","speed_milli":1000,"synthetic_disclosure_required":true,"provider":"mlx"}}"#
        ),
        r#"{"voice":{"kind":"authorized_reference"},"language":"Chinese","speed_milli":1000,"synthetic_disclosure_required":true}"#.to_owned(),
    ] {
        let configuration = WorkingOperatorConfiguration::new(schema.clone(), json).unwrap();
        assert!(validate_audio_speech_configuration(Some(&configuration)).is_err());
    }
}

#[test]
fn missing_configuration_remains_compatible_but_is_not_executable() {
    let draft = audio_speech_draft();
    assert!(validate_audio_speech_configuration(None).is_ok());
    assert!(audio_speech_operation_from_draft(&draft).unwrap().is_none());
}
