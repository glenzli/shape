//! Authenticated Infer text generation prepared outside the live desktop
//! session, then adopted through its stale-head Candidate Shelf boundary.

use shape_core::{
    CoreError, ShapeProject, TextCandidate, TextTransformMode, TextTransformParameters,
};
use shape_domain::{ArtifactId, ArtifactKind, OperatorNodeId};
use shape_execution::{ExecutionError, InferRuntimeExecutor};

use crate::operator_catalog::{
    compiled_instruction_from_draft, is_text_workspace_operator, mode_from_draft,
};

/// Opaque ownership of one fully executed but still transient candidate.
#[derive(Debug)]
pub struct InferTextCandidate {
    candidate: TextCandidate,
    pub(super) request_draft: Option<shape_domain::WorkingOperatorDraft>,
}

impl InferTextCandidate {
    pub(super) fn into_candidate(self) -> TextCandidate {
        self.candidate
    }
}

pub(super) fn generate_infer_text_candidate(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferTextCandidate>, String> {
    let artifact_id = artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let draft_id =
        OperatorNodeId::new(draft_id).map_err(|_| "invalid_operator_draft".to_owned())?;
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
    let expected_head = artifact.accepted_revision;
    let graph = project
        .artifact_working_graphs()
        .map_err(|_| "project_unavailable".to_owned())?
        .into_iter()
        .find(|graph| graph.context_artifact_id() == artifact_id)
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    if graph.expected_revision_id() != expected_head {
        return Err("stale_candidate".to_owned());
    }
    let draft = graph
        .operators()
        .iter()
        .find(|draft| {
            draft.id() == &draft_id && is_text_workspace_operator(draft.operator_type().as_str())
        })
        .ok_or_else(|| "invalid_operator_draft".to_owned())?;
    let authoring = crate::operator_catalog::text_authoring::from_draft(draft)?;
    if expected_head.is_none() && authoring.is_none() {
        return Err("missing_accepted_revision".into());
    }
    let authoring_instruction = authoring
        .as_ref()
        .map(|state| state.compiled_instruction_for_input(draft.input().is_some()))
        .transpose()?;
    let mode = TextTransformMode::from_key(
        &mode_from_draft(draft).map_err(|_| "invalid_prompt".to_owned())?,
    )
    .ok_or_else(|| "invalid_prompt".to_owned())?;
    let instruction = authoring_instruction
        .clone()
        .map_or_else(|| compiled_instruction_from_draft(draft), Ok)
        .map_err(|_| "invalid_prompt".to_owned())?;
    let parameters =
        TextTransformParameters::new(mode, instruction).map_err(|_| "invalid_prompt".to_owned())?;
    let credential_path = crate::infer_runtime_access::sdk_credential_path(credential_path)?;
    let executor = InferRuntimeExecutor::new(explicit_override, credential_path)
        .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = if let Some(instruction) = authoring_instruction {
        project.propose_text_node(
            artifact_id,
            draft,
            &instruction,
            authoring
                .as_ref()
                .expect("authoring instruction")
                .content_contract(),
            &executor,
        )
    } else {
        project.propose_text_transform(
            artifact_id,
            expected_head.ok_or("missing_accepted_revision")?,
            &parameters,
            Vec::new(),
            &executor,
        )
    }
    .map_err(core_error_code)?;
    // A user may change a durable draft or accept another result while the
    // provider is running. Reject obsolete intent before adopting its output.
    let fresh = project
        .artifact_working_graphs()
        .map_err(|_| "project_unavailable")?;
    if !fresh.iter().any(|g| {
        g.context_artifact_id() == artifact_id
            && g.expected_revision_id() == expected_head
            && g.operators().iter().any(|d| d == draft)
    }) {
        return Err("stale_candidate".into());
    }
    Ok(Box::new(InferTextCandidate {
        candidate,
        request_draft: Some(draft.clone()),
    }))
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
        | CoreError::InvalidSpeechSource { .. }
        | CoreError::UnsupportedSpeechVoiceReference
        | CoreError::MissingAudioOutputContract
        | CoreError::AudioOutputContractMismatch
        | CoreError::ImageGenerationOutputContractMismatch
        | CoreError::InvalidImageGenerationTarget { .. }
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}

#[cfg(test)]
mod tests;
