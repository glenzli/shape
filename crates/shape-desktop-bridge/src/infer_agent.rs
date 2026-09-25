//! Background preparation of one Agent file result for desktop review.

use shape_core::{AgentTextCandidate, ShapeProject};
use shape_domain::{ArtifactId, ArtifactKind, RevisionId};
use shape_execution::{ExecutionError, InferRuntimeAgentFileExecutor};

/// Opaque executed Agent result. The UI thread must recheck and adopt it.
#[derive(Debug)]
pub struct InferAgentTextCandidate {
    candidate: AgentTextCandidate,
}

impl InferAgentTextCandidate {
    pub(super) fn into_candidate(self) -> AgentTextCandidate {
        self.candidate
    }
}

pub(super) fn generate_infer_agent_text_candidate(
    project_path: &str,
    artifact_id: &str,
    revision_id: &str,
    instruction: &str,
    credential_path: &str,
    explicit_override: &str,
) -> Result<Box<InferAgentTextCandidate>, String> {
    if instruction.trim().is_empty() || instruction.len() > 16 * 1024 {
        return Err("invalid_prompt".into());
    }
    let artifact_id = artifact_id
        .parse::<ArtifactId>()
        .map_err(|_| "invalid_artifact".to_owned())?;
    let revision_id = revision_id
        .parse::<RevisionId>()
        .map_err(|_| "missing_accepted_revision".to_owned())?;
    let project = ShapeProject::open(project_path).map_err(|_| "project_unavailable")?;
    let snapshot = project.snapshot().map_err(|_| "project_unavailable")?;
    let source = snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id)
        .ok_or("invalid_artifact")?;
    if source.kind != ArtifactKind::TextDocument {
        return Err("unsupported_artifact".into());
    }
    if source.accepted_revision != Some(revision_id) {
        return Err("stale_candidate".into());
    }
    let (name, extension) = output_identity(&source.name);
    let credential_path = crate::infer_runtime_access::sdk_credential_path(credential_path)?;
    let executor =
        InferRuntimeAgentFileExecutor::new(explicit_override, credential_path, extension)
            .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = project
        .propose_agent_text_file(artifact_id, revision_id, name, instruction, &executor)
        .map_err(|error| match error {
            shape_core::CoreError::Execution(ExecutionError::ExecutorFailed {
                failure, ..
            }) => failure.code,
            shape_core::CoreError::StaleCandidate { .. } => "stale_candidate".into(),
            shape_core::CoreError::InvalidTextCandidate => "invalid_text_output".into(),
            _ => "agent_task_failed".into(),
        })?;
    Ok(Box::new(InferAgentTextCandidate { candidate }))
}

fn output_identity(source_name: &str) -> (String, &'static str) {
    let (stem, extension) = source_name
        .rsplit_once('.')
        .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
        .unwrap_or((source_name, "txt"));
    let extension = match extension.to_ascii_lowercase().as_str() {
        "md" => "md",
        "js" => "js",
        "mjs" => "mjs",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "svg" => "svg",
        "py" => "py",
        "ts" => "ts",
        "tsx" => "tsx",
        "rs" => "rs",
        "sh" => "sh",
        "xml" => "xml",
        "yaml" => "yaml",
        "yml" => "yml",
        "csv" => "csv",
        _ => "txt",
    };
    (format!("{stem} · Agent.{extension}"), extension)
}

#[cfg(test)]
mod tests;
