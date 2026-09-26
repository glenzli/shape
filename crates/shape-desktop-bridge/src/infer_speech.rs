//! Authenticated preset speech synthesis prepared outside the live desktop
//! session, then adopted through its stale-source Candidate Shelf boundary.

use shape_core::{AudioCandidate, CoreError, ShapeProject};
use shape_domain::{ArtifactId, OperatorNodeId};
use shape_execution::{ExecutionError, InferRuntimeSpeechExecutor};

/// Opaque desktop ownership of thread-safe narration progress and retry state.
#[derive(Debug, Default)]
pub struct SpeechSynthesisControl {
    inner: shape_execution::SpeechSynthesisControl,
}

use crate::operator_catalog::{AUDIO_SPEECH_OPERATOR, audio_speech_operation_from_draft};

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
    draft_id: &str,
    artifact_name: &str,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferSpeechCandidate>, String> {
    generate_infer_speech_candidate_controlled(
        project_path,
        source_artifact_id,
        draft_id,
        artifact_name,
        credential_path,
        explicit_override,
        &SpeechSynthesisControl::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn generate_infer_speech_candidate_controlled(
    project_path: &str,
    source_artifact_id: &str,
    draft_id: &str,
    _artifact_name: &str,
    credential_path: &str,
    explicit_override: &str,
    control: &SpeechSynthesisControl,
) -> Result<Box<InferSpeechCandidate>, String> {
    let source_artifact_id = source_artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let draft_id =
        OperatorNodeId::new(draft_id).map_err(|_| "invalid_operator_draft".to_owned())?;
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
    let graph = project
        .artifact_working_graphs()
        .map_err(|_| "project_unavailable".to_owned())?
        .into_iter()
        .find(|graph| graph.operators().iter().any(|d| d.id() == &draft_id))
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    let draft = graph
        .operators()
        .iter()
        .find(|draft| {
            draft.id() == &draft_id && draft.operator_type().as_str() == AUDIO_SPEECH_OPERATOR
        })
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    if draft.input().is_none_or(|i| {
        i.artifact_id != source_artifact_id || i.revision_id != expected_source_head
    }) {
        return Err("stale_candidate".into());
    }
    let operation = audio_speech_operation_from_draft(draft)
        .map_err(|_| "invalid_speech_request".to_owned())?
        .ok_or_else(|| "speech_draft_unconfigured".to_owned())?;
    let credential_path = crate::infer_runtime_access::sdk_credential_path(credential_path)?;
    let executor = InferRuntimeSpeechExecutor::new(explicit_override, credential_path)
        .map_err(|_| "executor_invalid".to_owned())?
        .with_control(control.inner.clone());
    let candidate = project
        .propose_speech_node(graph.context_artifact_id(), draft, &operation, &executor)
        .map_err(core_error_code)?;
    Ok(Box::new(InferSpeechCandidate::new(candidate)))
}

#[allow(clippy::unnecessary_box_returns)] // CXX opaque Rust ownership requires Box.
pub(super) fn new_speech_control() -> Box<SpeechSynthesisControl> {
    Box::default()
}
pub(super) fn speech_control_resume(control: &SpeechSynthesisControl) {
    control.inner.resume();
}
pub(super) fn speech_control_cancel(control: &SpeechSynthesisControl) {
    control.inner.cancel();
}
pub(super) fn speech_control_completed(control: &SpeechSynthesisControl) -> u32 {
    u32::try_from(control.inner.completed()).unwrap_or(512)
}
pub(super) fn speech_control_total(control: &SpeechSynthesisControl) -> u32 {
    u32::try_from(control.inner.total()).unwrap_or(512)
}
pub(super) fn speech_presets() -> Vec<crate::ffi::SpeechPresetWire> {
    shape_execution::SPEECH_PRESETS
        .iter()
        .map(|preset| crate::ffi::SpeechPresetWire {
            key: preset.key.to_owned(),
            alias: preset.alias.to_owned(),
            language: preset.language.to_owned(),
            catalog_revision: shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION.to_owned(),
        })
        .collect()
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
        | CoreError::InvalidImportSource { .. }
        | CoreError::InvalidRasterContent { .. }
        | CoreError::NoOpRasterCrop
        | CoreError::NoOpRasterResize
        | CoreError::MissingRasterOutputContract
        | CoreError::RasterCropOutputContractMismatch
        | CoreError::RasterResizeOutputContractMismatch
        | CoreError::RasterEditOutputContractMismatch
        | CoreError::RasterBlurUnsupportedColorProfile
        | CoreError::RasterBlurSourceTooLarge { .. }
        | CoreError::RasterUnsharpMaskUnsupportedColorProfile
        | CoreError::RasterUnsharpMaskSourceTooLarge { .. }
        | CoreError::RasterDropShadowUnsupportedColorProfile
        | CoreError::RasterDropShadowOutputTooLarge { .. }
        | CoreError::ImageGenerationOutputContractMismatch
        | CoreError::InvalidImageGenerationTarget { .. }
        | CoreError::InvalidSoundGenerationTarget { .. }
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}

#[cfg(test)]
mod tests;
