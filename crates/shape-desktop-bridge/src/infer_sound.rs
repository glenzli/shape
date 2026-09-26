//! Background sound generation re-reads the checkpointed authored request before credential access.
use shape_core::{ShapeProject, SoundCandidate};
use shape_domain::{ArtifactId, ArtifactKind};
use shape_execution::InferRuntimeSoundExecutor;
#[derive(Debug, Default)]
pub struct SoundGenerationControl(shape_execution::SoundGenerationControl);
#[derive(Debug)]
pub struct InferSoundCandidate {
    pub(super) candidate: SoundCandidate,
    pub(super) draft_id: String,
}
#[allow(clippy::unnecessary_box_returns)] // CXX opaque ownership.
pub(super) fn new_sound_control() -> Box<SoundGenerationControl> {
    Box::default()
}
pub(super) fn sound_control_resume(control: &SoundGenerationControl) {
    control.0.resume();
}
pub(super) fn sound_control_stage(control: &SoundGenerationControl) -> u8 {
    control.0.stage()
}
pub(super) fn sound_control_cancel(control: &SoundGenerationControl) {
    control.0.cancel();
}
pub(super) fn generate_infer_sound_candidate(
    project_path: &str,
    artifact_id: &str,
    draft_id: &str,
    credential_path: &str,
    explicit_override: &str,
    control: &SoundGenerationControl,
) -> Result<Box<InferSoundCandidate>, String> {
    let artifact_id: ArtifactId = artifact_id.parse().map_err(|_| "invalid_artifact")?;
    let project = ShapeProject::open(project_path).map_err(|_| "project_unavailable")?;
    let snapshot = project.snapshot().map_err(|_| "project_unavailable")?;
    if !snapshot.artifacts.iter().any(|a| {
        a.id == artifact_id && a.kind == ArtifactKind::AudioClip && a.accepted_revision.is_none()
    }) {
        return Err("stale_candidate".into());
    }
    let graph = project
        .artifact_working_graphs()
        .map_err(|_| "invalid_sound_draft")?
        .into_iter()
        .find(|g| g.context_artifact_id() == artifact_id)
        .ok_or("invalid_sound_draft")?;
    let draft = graph
        .operators()
        .iter()
        .find(|d| d.id().as_str() == draft_id)
        .ok_or("invalid_sound_draft")?;
    let parameters = crate::operator_catalog::sound_generation::parameters(draft)?;
    let credential_path = crate::infer_runtime_access::sdk_credential_path(credential_path)?;
    let executor =
        InferRuntimeSoundExecutor::new(explicit_override, credential_path, control.0.clone())
            .map_err(|_| "executor_invalid")?;
    let candidate = project
        .propose_generated_sound(artifact_id, &parameters, &executor)
        .map_err(|e| match e {
            shape_core::CoreError::Execution(shape_execution::ExecutionError::ExecutorFailed {
                failure,
                ..
            }) => failure.code,
            _ => "sound_generation_failed".into(),
        })?;
    Ok(Box::new(InferSoundCandidate {
        candidate,
        draft_id: draft_id.into(),
    }))
}
