//! Authenticated zero-input image generation prepared outside the live desktop
//! session, then adopted through its exact source-draft Candidate boundary.

use shape_core::{AiImageCandidate, CoreError, ShapeProject};
use shape_domain::{ArtifactId, ArtifactKind, OperatorNodeId};
use shape_execution::{
    ExecutionError, InferRuntimeCredentialStore, InferRuntimeImageGenerationExecutor,
};

use crate::operator_catalog::{IMAGE_GENERATE_OPERATOR, ai_image_generate_parameters_from_draft};

/// Opaque ownership of one fully executed but still transient AI Image Candidate.
#[derive(Debug)]
pub struct InferImageCandidate {
    candidate: AiImageCandidate,
    draft_id: String,
}

impl InferImageCandidate {
    pub(super) fn new(candidate: AiImageCandidate, draft_id: impl Into<String>) -> Self {
        Self {
            candidate,
            draft_id: draft_id.into(),
        }
    }

    pub(super) fn into_parts(self) -> (AiImageCandidate, String) {
        (self.candidate, self.draft_id)
    }
}

pub(super) fn generate_infer_image_candidate(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferImageCandidate>, String> {
    let artifact_id = artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let draft_identity =
        OperatorNodeId::new(draft_id).map_err(|_| "invalid_operator_draft".to_owned())?;
    let project = ShapeProject::open(project_path).map_err(|_| "project_unavailable".to_owned())?;
    let artifact = project
        .snapshot()
        .map_err(|_| "project_unavailable".to_owned())?
        .artifacts
        .into_iter()
        .find(|artifact| artifact.id == artifact_id)
        .ok_or_else(|| "invalid_artifact".to_owned())?;
    if artifact.kind != ArtifactKind::ImageRaster || artifact.accepted_revision.is_some() {
        return Err("invalid_image_generation_target".to_owned());
    }
    let graph = project
        .artifact_working_graphs()
        .map_err(|_| "project_unavailable".to_owned())?
        .into_iter()
        .find(|graph| graph.context_artifact_id() == artifact_id)
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    if graph.expected_revision_id().is_some() {
        return Err("invalid_operator_draft".to_owned());
    }
    let draft = graph
        .operators()
        .iter()
        .find(|draft| {
            draft.id() == &draft_identity
                && draft.operator_type().as_str() == IMAGE_GENERATE_OPERATOR
                && draft.input_data_type().is_none()
        })
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    let parameters = ai_image_generate_parameters_from_draft(draft)
        .map_err(|_| "invalid_image_request".to_owned())?
        .ok_or_else(|| "invalid_image_request".to_owned())?;

    // Credential access deliberately occurs only after the exact persisted
    // draft and zero-input target have been revalidated.
    let credential = InferRuntimeCredentialStore::new(credential_path)
        .load()
        .map_err(|error| crate::infer_runtime_access::credential_error_code(&error).to_owned())?;
    let executor = InferRuntimeImageGenerationExecutor::new(explicit_override, credential)
        .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = project
        .propose_generated_image(artifact_id, &parameters, &executor)
        .map_err(core_error_code)?;
    Ok(Box::new(InferImageCandidate::new(candidate, draft_id)))
}

#[cfg(test)]
mod tests;

fn core_error_code(error: CoreError) -> String {
    match error {
        CoreError::Execution(ExecutionError::ExecutorFailed { failure, .. }) => failure.code,
        CoreError::Execution(ExecutionError::UnsupportedCapability { .. }) => {
            "unsupported_capability".to_owned()
        }
        CoreError::InvalidImageGenerationTarget { .. } => {
            "invalid_image_generation_target".to_owned()
        }
        CoreError::ImageGenerationOutputContractMismatch => "invalid_image_output".to_owned(),
        CoreError::Domain(_) | CoreError::Json(_) => "invalid_image_request".to_owned(),
        CoreError::Store(_) | CoreError::MissingAcceptedRevision { .. } => {
            "project_unavailable".to_owned()
        }
        CoreError::StaleCandidate { .. } | CoreError::StaleSceneCandidate { .. } => {
            "stale_candidate".to_owned()
        }
        CoreError::BranchRequiresAcceptedSource { .. }
        | CoreError::InvalidTextCandidate
        | CoreError::InvalidTextArtifact { .. }
        | CoreError::InvalidTextTransformInstruction
        | CoreError::InvalidSpeechSource { .. }
        | CoreError::UnsupportedSpeechVoiceReference
        | CoreError::MissingAudioOutputContract
        | CoreError::AudioOutputContractMismatch
        | CoreError::InvalidRasterSource { .. }
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
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}
