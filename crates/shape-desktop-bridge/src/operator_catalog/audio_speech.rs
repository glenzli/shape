//! Versioned authored-state contract for the real `audio.speech_synthesize` consumer.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, PresetVoiceAlias, PresetVoiceSelection,
    SpeechSynthesisOperation, SpeechVoiceSelection, WorkingOperatorConfiguration,
    WorkingOperatorDraft,
};
use shape_execution::{
    INFER_SPEECH_VOICE_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
};

use super::AUDIO_SPEECH_OPERATOR;

const LEGACY_SCHEMA: &str = "shape.operator-draft.audio-speech@20260811.1";
const AUDIO_SPEECH_DRAFT_SCHEMA: &str = "shape.operator-draft.audio-speech@20260922.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioSpeechDraftConfiguration {
    preset_alias: String,
    preset_catalog_revision: String,
    language: String,
    speed_milli: u16,
    synthetic_disclosure_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    script: Option<shape_domain::speech_script::SpeechScriptOptions>,
}

impl AudioSpeechDraftConfiguration {
    fn operation(&self) -> Result<SpeechSynthesisOperation, String> {
        let mut operation = SpeechSynthesisOperation::new(
            self.language.clone(),
            SpeechVoiceSelection::Preset(
                PresetVoiceSelection::new(
                    PresetVoiceAlias::new(self.preset_alias.clone())
                        .map_err(|error| error.to_string())?,
                    self.preset_catalog_revision.clone(),
                )
                .map_err(|error| error.to_string())?,
            ),
            self.speed_milli,
            self.synthetic_disclosure_required,
        )
        .map_err(|error| error.to_string())?;
        operation.script.clone_from(&self.script);
        operation.validate().map_err(|e| e.to_string())?;
        if let Some(options) = &operation.script {
            for voice in options.roles.values() {
                let selection = SpeechSynthesisOperation::new(
                    "auto",
                    SpeechVoiceSelection::Preset(voice.clone()),
                    self.speed_milli,
                    true,
                )
                .map_err(|e| e.to_string())?;
                if !shape_execution::supported_speech_operation(&selection) {
                    return Err("unsupported script voice".into());
                }
            }
        }
        if !shape_execution::supported_speech_operation(&operation) {
            return Err("audio speech draft uses an unsupported preset selection".to_owned());
        }
        Ok(operation)
    }
}

pub(crate) fn default_audio_speech_configuration() -> Result<WorkingOperatorConfiguration, String> {
    configuration_for_audio_speech(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
        INFER_SPEECH_VOICE_CATALOG_REVISION,
        "auto",
        1_000,
        true,
    )
}

pub(crate) fn configuration_for_audio_speech(
    preset_alias: &str,
    preset_catalog_revision: &str,
    language: &str,
    speed_milli: u16,
    synthetic_disclosure_required: bool,
) -> Result<WorkingOperatorConfiguration, String> {
    let configuration = AudioSpeechDraftConfiguration {
        preset_alias: preset_alias.to_owned(),
        preset_catalog_revision: preset_catalog_revision.to_owned(),
        language: language.to_owned(),
        speed_milli,
        synthetic_disclosure_required,
        script: None,
    };
    configuration.operation()?;
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(AUDIO_SPEECH_DRAFT_SCHEMA)
            .map_err(|error| error.to_string())?,
        serde_json::to_string(&configuration).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn audio_speech_operation_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<Option<SpeechSynthesisOperation>, String> {
    if draft.operator_type().as_str() != AUDIO_SPEECH_OPERATOR {
        return Ok(None);
    }
    let Some(configuration) = draft.configuration() else {
        return Ok(None);
    };
    decode(configuration)
        .map(|configuration| Some(configuration.operation()))
        .and_then(Option::transpose)
}

pub(super) fn validate_audio_speech_configuration(
    configuration: Option<&WorkingOperatorConfiguration>,
) -> Result<(), String> {
    configuration.map_or(Ok(()), |configuration| {
        decode(configuration)?.operation().map(|_| ())
    })
}

fn decode(
    configuration: &WorkingOperatorConfiguration,
) -> Result<AudioSpeechDraftConfiguration, String> {
    if ![AUDIO_SPEECH_DRAFT_SCHEMA, LEGACY_SCHEMA].contains(&configuration.schema().as_str()) {
        return Err("audio speech draft configuration schema is unsupported".to_owned());
    }
    let decoded: AudioSpeechDraftConfiguration = serde_json::from_str(configuration.json())
        .map_err(|_| {
            "audio speech draft configuration does not match its exact schema".to_owned()
        })?;
    if configuration.schema().as_str() == LEGACY_SCHEMA && decoded.script.is_some() {
        return Err("legacy speech drafts cannot contain script options".into());
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests;

/// Rebuilds the versioned authored configuration without losing ordinary voice controls.
pub(crate) fn configuration_with_script(
    configuration: &WorkingOperatorConfiguration,
    script: Option<shape_domain::speech_script::SpeechScriptOptions>,
) -> Result<WorkingOperatorConfiguration, String> {
    let mut decoded = decode(configuration)?;
    decoded.script = script;
    decoded.operation()?;
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(AUDIO_SPEECH_DRAFT_SCHEMA).map_err(|e| e.to_string())?,
        serde_json::to_string(&decoded).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
