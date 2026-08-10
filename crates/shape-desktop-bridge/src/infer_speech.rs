//! Authenticated preset speech synthesis prepared outside the live desktop
//! session, then adopted through its stale-source Candidate Shelf boundary.

use shape_core::{AudioCandidate, CoreError, ShapeProject};
use shape_domain::{
    ArtifactId, PresetVoiceAlias, PresetVoiceSelection, SpeechSynthesisOperation,
    SpeechVoiceSelection,
};
use shape_execution::{
    ExecutionError, INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
    InferRuntimeCredentialStore, InferRuntimeSpeechExecutor,
};

/// Opaque ownership of one fully executed but still transient speech Candidate.
#[derive(Debug)]
pub struct InferSpeechCandidate {
    candidate: AudioCandidate,
}

impl InferSpeechCandidate {
    pub(super) fn new(candidate: AudioCandidate) -> Self {
        Self { candidate }
    }

    pub(super) fn into_candidate(self) -> AudioCandidate {
        self.candidate
    }
}

pub(super) fn generate_infer_speech_candidate(
    project_path: &str,
    source_artifact_id: &str,
    artifact_name: &str,
    speed_milli: u16,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferSpeechCandidate>, String> {
    let source_artifact_id = source_artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let project = ShapeProject::open(project_path).map_err(|_| "project_unavailable".to_owned())?;
    let source = project
        .snapshot()
        .map_err(|_| "project_unavailable".to_owned())?
        .artifacts
        .into_iter()
        .find(|artifact| artifact.id == source_artifact_id)
        .ok_or_else(|| "invalid_artifact".to_owned())?;
    let expected_source_head = source
        .accepted_revision
        .ok_or_else(|| "missing_accepted_revision".to_owned())?;
    let operation = SpeechSynthesisOperation::new(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1)
                    .map_err(|_| "speech_preset_invalid".to_owned())?,
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            )
            .map_err(|_| "speech_preset_invalid".to_owned())?,
        ),
        speed_milli,
        true,
    )
    .map_err(|_| "invalid_speech_request".to_owned())?;
    let credential = InferRuntimeCredentialStore::new(credential_path)
        .load()
        .map_err(|error| crate::infer_runtime_access::credential_error_code(&error).to_owned())?;
    let executor = InferRuntimeSpeechExecutor::new(explicit_override, credential)
        .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = project
        .propose_speech_synthesis(
            source_artifact_id,
            expected_source_head,
            artifact_name,
            &operation,
            Vec::new(),
            &executor,
        )
        .map_err(core_error_code)?;
    Ok(Box::new(InferSpeechCandidate::new(candidate)))
}

fn core_error_code(error: CoreError) -> String {
    match error {
        CoreError::Execution(ExecutionError::ExecutorFailed { failure, .. }) => failure.code,
        CoreError::Execution(ExecutionError::UnsupportedCapability { .. }) => {
            "unsupported_capability".to_owned()
        }
        CoreError::StaleCandidate { .. } | CoreError::StaleSceneCandidate { .. } => {
            "stale_candidate".to_owned()
        }
        CoreError::InvalidSpeechSource { .. } => "unsupported_artifact".to_owned(),
        CoreError::UnsupportedSpeechVoiceReference => "voice_reference_not_supported".to_owned(),
        CoreError::MissingAudioOutputContract | CoreError::AudioOutputContractMismatch => {
            "invalid_audio_output".to_owned()
        }
        CoreError::Domain(_) | CoreError::Json(_) => "invalid_speech_request".to_owned(),
        CoreError::Store(_) | CoreError::MissingAcceptedRevision { .. } => {
            "project_unavailable".to_owned()
        }
        CoreError::BranchRequiresAcceptedSource { .. }
        | CoreError::InvalidTextCandidate
        | CoreError::InvalidTextArtifact { .. }
        | CoreError::InvalidTextTransformInstruction
        | CoreError::InvalidRasterSource { .. }
        | CoreError::InvalidRasterContent { .. }
        | CoreError::NoOpRasterCrop
        | CoreError::MissingRasterOutputContract
        | CoreError::RasterCropOutputContractMismatch
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}

#[cfg(test)]
mod tests;
