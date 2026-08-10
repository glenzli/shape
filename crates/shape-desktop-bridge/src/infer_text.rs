//! Authenticated Infer text generation prepared outside the live desktop
//! session, then adopted through its stale-head Candidate Shelf boundary.

use shape_core::{
    CoreError, ShapeProject, TextCandidate, TextTransformMode, TextTransformParameters,
};
use shape_domain::{ArtifactId, ArtifactKind};
use shape_execution::{ExecutionError, InferRuntimeCredentialStore, InferRuntimeExecutor};

/// Opaque ownership of one fully executed but still transient candidate.
#[derive(Debug)]
pub struct InferTextCandidate {
    candidate: TextCandidate,
}

impl InferTextCandidate {
    pub(super) fn into_candidate(self) -> TextCandidate {
        self.candidate
    }
}

pub(super) fn generate_infer_text_candidate(
    project_path: &str,
    artifact_id: &str,
    prompt: &str,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferTextCandidate>, String> {
    let artifact_id = artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let project = ShapeProject::open(project_path).map_err(|_| "project_unavailable".to_owned())?;
    let snapshot = project
        .snapshot()
        .map_err(|_| "project_unavailable".to_owned())?;
    let artifact = snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id)
        .ok_or_else(|| "invalid_artifact".to_owned())?;
    if artifact.kind != ArtifactKind::TextDocument {
        return Err("unsupported_artifact".to_owned());
    }
    let expected_head = artifact
        .accepted_revision
        .ok_or_else(|| "missing_accepted_revision".to_owned())?;
    let parameters = TextTransformParameters::new(TextTransformMode::Rewrite, prompt)
        .map_err(|_| "invalid_prompt".to_owned())?;
    let credential = InferRuntimeCredentialStore::new(credential_path)
        .load()
        .map_err(|error| crate::infer_runtime_access::credential_error_code(&error).to_owned())?;
    let executor = InferRuntimeExecutor::new(explicit_override, credential)
        .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = project
        .propose_text_transform(
            artifact_id,
            expected_head,
            &parameters,
            Vec::new(),
            &executor,
        )
        .map_err(core_error_code)?;
    Ok(Box::new(InferTextCandidate { candidate }))
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
        CoreError::InvalidTextCandidate => "invalid_text_output".to_owned(),
        CoreError::InvalidTextArtifact { .. } => "unsupported_artifact".to_owned(),
        CoreError::InvalidTextTransformInstruction | CoreError::Domain(_) | CoreError::Json(_) => {
            "invalid_prompt".to_owned()
        }
        CoreError::Store(_) | CoreError::MissingAcceptedRevision { .. } => {
            "project_unavailable".to_owned()
        }
        CoreError::BranchRequiresAcceptedSource { .. } => "project_unavailable".to_owned(),
        CoreError::InvalidRasterSource { .. }
        | CoreError::InvalidRasterContent { .. }
        | CoreError::NoOpRasterCrop
        | CoreError::MissingRasterOutputContract
        | CoreError::RasterCropOutputContractMismatch
        | CoreError::InvalidSpeechSource { .. }
        | CoreError::UnsupportedSpeechVoiceReference
        | CoreError::MissingAudioOutputContract
        | CoreError::AudioOutputContractMismatch
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}

#[cfg(test)]
mod tests;
