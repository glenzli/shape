//! Authenticated zero-input image generation prepared outside the live desktop
//! session, then adopted through its exact source-draft Candidate boundary.

use std::{
    collections::VecDeque,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use shape_core::{AiImageCandidate, CoreError, ShapeProject};
use shape_domain::{AiImageGenerateParameters, ArtifactId, ArtifactKind, OperatorNodeId};
use shape_execution::{ExecutionError, Executor, InferRuntimeImageGenerationExecutor};

use crate::operator_catalog::{IMAGE_GENERATE_OPERATOR, ai_image_generate_parameters_from_draft};

/// Opaque ownership of one fully executed but still transient AI Image Candidate.
#[derive(Debug)]
pub struct InferImageCandidate {
    candidate: AiImageCandidate,
    draft_id: String,
}

/// One user action with independently executed, transient image results.
#[derive(Debug)]
pub struct InferImageBatch {
    candidates: Vec<AiImageCandidate>,
    draft_id: String,
    failure_code: String,
}

/// Thread-safe progress and transfer queue for one image batch. The desktop
/// consumes each Candidate on its UI thread as soon as its request completes.
#[derive(Debug, Default)]
pub struct ImageGenerationControl {
    cancelled: AtomicBool,
    ready: Mutex<VecDeque<InferImageCandidate>>,
}

#[allow(clippy::unnecessary_box_returns)] // CXX opaque ownership requires Box.
pub(super) fn new_image_generation_control() -> Box<ImageGenerationControl> {
    Box::new(ImageGenerationControl::default())
}

pub(super) fn image_generation_control_cancel(control: &ImageGenerationControl) {
    control.cancelled.store(true, Ordering::Release);
}

pub(super) fn image_generation_control_has_candidate(control: &ImageGenerationControl) -> bool {
    !control
        .ready
        .lock()
        .expect("image candidate queue is usable")
        .is_empty()
}

pub(super) fn image_generation_control_take_candidate(
    control: &ImageGenerationControl,
) -> Result<Box<InferImageCandidate>, String> {
    control
        .ready
        .lock()
        .map_err(|_| "image_candidate_queue_failed")?
        .pop_front()
        .map(Box::new)
        .ok_or_else(|| "no_image_candidate_ready".to_owned())
}

impl InferImageBatch {
    pub(crate) fn new(
        candidates: Vec<AiImageCandidate>,
        draft_id: impl Into<String>,
        failure_code: impl Into<String>,
    ) -> Self {
        Self {
            candidates,
            draft_id: draft_id.into(),
            failure_code: failure_code.into(),
        }
    }

    pub(super) fn into_parts(self) -> (Vec<AiImageCandidate>, String, String) {
        (self.candidates, self.draft_id, self.failure_code)
    }
}

pub(super) fn infer_image_batch_completed_count(batch: &InferImageBatch) -> u8 {
    u8::try_from(batch.candidates.len()).expect("image batch is bounded to four candidates")
}

pub(super) fn infer_image_batch_failure_code(batch: &InferImageBatch) -> String {
    batch.failure_code.clone()
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
    model_key: &str,
    effort_key: &str,
) -> Result<Box<InferImageCandidate>, String> {
    let batch = generate_infer_image_batch(
        project_path,
        artifact_id,
        draft_id,
        credential_path,
        explicit_override,
        model_key,
        effort_key,
        1,
    )?;
    let (mut candidates, _, _) = batch.into_parts();
    Ok(Box::new(InferImageCandidate::new(
        candidates.remove(0),
        draft_id,
    )))
}

#[allow(clippy::too_many_arguments)] // CXX passes explicit draft identity and expected count.
pub(super) fn generate_infer_image_batch(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
    model_key: &str,
    effort_key: &str,
    requested_count: u8,
) -> Result<Box<InferImageBatch>, String> {
    generate_infer_image_batch_impl(
        project_path,
        artifact_id,
        draft_id,
        credential_path,
        explicit_override,
        model_key,
        effort_key,
        requested_count,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // CXX passes explicit draft identity and expected count.
pub(super) fn generate_infer_image_batch_controlled(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
    model_key: &str,
    effort_key: &str,
    requested_count: u8,
    control: &ImageGenerationControl,
) -> Result<Box<InferImageBatch>, String> {
    generate_infer_image_batch_impl(
        project_path,
        artifact_id,
        draft_id,
        credential_path,
        explicit_override,
        model_key,
        effort_key,
        requested_count,
        Some(control),
    )
}

#[allow(clippy::too_many_arguments)] // Exact persisted request and optional progress owner.
fn generate_infer_image_batch_impl(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
    model_key: &str,
    effort_key: &str,
    requested_count: u8,
    control: Option<&ImageGenerationControl>,
) -> Result<Box<InferImageBatch>, String> {
    if !matches!(model_key, "gpt_5_6_luna" | "gpt_6_luna" | "gpt_6_sol") {
        return Err("invalid_model_choice".to_owned());
    }
    if !(matches!(effort_key, "" | "low" | "medium" | "high" | "xhigh" | "max")
        || (model_key == "gpt_6_sol" && effort_key == "ultra"))
    {
        return Err("invalid_effort_choice".to_owned());
    }
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
    if !(1..=4).contains(&requested_count) || parameters.candidate_count() != requested_count {
        return Err("invalid_image_candidate_count".to_owned());
    }

    // Credential access deliberately occurs only after the exact persisted
    // draft and zero-input target have been revalidated.
    let credential_path = crate::infer_runtime_access::sdk_credential_path(credential_path)?;
    let executor = InferRuntimeImageGenerationExecutor::new_with_model(
        explicit_override,
        credential_path,
        model_key,
        effort_key,
    )
    .map_err(|_| "executor_invalid".to_owned())?;
    execute_image_batch(
        &project,
        artifact_id,
        draft_id,
        &parameters,
        requested_count,
        &executor,
        control,
    )
}

pub(crate) fn execute_image_batch(
    project: &ShapeProject,
    artifact_id: ArtifactId,
    draft_id: &str,
    parameters: &AiImageGenerateParameters,
    requested_count: u8,
    executor: &dyn Executor,
    control: Option<&ImageGenerationControl>,
) -> Result<Box<InferImageBatch>, String> {
    let mut candidates = Vec::with_capacity(usize::from(requested_count));
    let mut failure_code = String::new();
    for _ in 0..requested_count {
        if control.is_some_and(|control| control.cancelled.load(Ordering::Acquire)) {
            if candidates.is_empty() {
                return Err("generation_cancelled".to_owned());
            }
            "generation_cancelled".clone_into(&mut failure_code);
            break;
        }
        match project.propose_generated_image(artifact_id, parameters, executor) {
            Ok(candidate) => {
                if let Some(control) = control {
                    control
                        .ready
                        .lock()
                        .map_err(|_| "image_candidate_queue_failed")?
                        .push_back(InferImageCandidate::new(candidate.clone(), draft_id));
                }
                candidates.push(candidate);
            }
            Err(error) if candidates.is_empty() => return Err(core_error_code(error)),
            Err(error) => {
                failure_code = core_error_code(error);
                break;
            }
        }
    }
    Ok(Box::new(InferImageBatch::new(
        candidates,
        draft_id,
        failure_code,
    )))
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
        | CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}
