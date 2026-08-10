//! Authenticated Infer text generation prepared outside the live desktop
//! session, then adopted through its stale-head Candidate Shelf boundary.

use std::io::ErrorKind;

use shape_core::{CoreError, ShapeProject, TextCandidate};
use shape_domain::{ArtifactId, ArtifactKind, IntentSpec};
use shape_execution::{
    ExecutionError, InferRuntimeCredentialError, InferRuntimeCredentialStore, InferRuntimeExecutor,
};

use crate::ffi;

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

pub(super) fn infer_runtime_credential_status(path: &str) -> ffi::InferRuntimeCredentialStatusWire {
    match InferRuntimeCredentialStore::new(path).is_available() {
        Ok(configured) => ffi::InferRuntimeCredentialStatusWire {
            configured,
            error_code: String::new(),
        },
        Err(error) => ffi::InferRuntimeCredentialStatusWire {
            configured: false,
            error_code: credential_error_code(&error).to_owned(),
        },
    }
}

pub(super) fn install_infer_runtime_credential(path: &str, token: &str) -> Result<(), String> {
    InferRuntimeCredentialStore::new(path)
        .install(token)
        .map_err(|error| credential_error_code(&error).to_owned())
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
    let intent = IntentSpec::new(prompt).map_err(|_| "invalid_prompt".to_owned())?;
    let credential = InferRuntimeCredentialStore::new(credential_path)
        .load()
        .map_err(|error| credential_error_code(&error).to_owned())?;
    let executor = InferRuntimeExecutor::new(explicit_override, credential)
        .map_err(|_| "executor_invalid".to_owned())?;
    let candidate = project
        .propose_generated_text(
            artifact_id,
            artifact.accepted_revision,
            prompt,
            intent,
            Vec::new(),
            &executor,
        )
        .map_err(core_error_code)?;
    Ok(Box::new(InferTextCandidate { candidate }))
}

fn credential_error_code(error: &InferRuntimeCredentialError) -> &'static str {
    match error {
        InferRuntimeCredentialError::Io { source, .. } if source.kind() == ErrorKind::NotFound => {
            "credential_missing"
        }
        InferRuntimeCredentialError::InvalidPath => "credential_path_invalid",
        InferRuntimeCredentialError::InvalidToken => "credential_invalid",
        InferRuntimeCredentialError::UnsafeObject => "credential_unsafe",
        InferRuntimeCredentialError::UnsupportedPlatform => "credential_unsupported",
        InferRuntimeCredentialError::Io { .. } => "credential_unavailable",
    }
}

fn core_error_code(error: CoreError) -> String {
    match error {
        CoreError::Execution(ExecutionError::ExecutorFailed { failure, .. }) => failure.code,
        CoreError::Execution(ExecutionError::UnsupportedCapability { .. }) => {
            "unsupported_capability".to_owned()
        }
        CoreError::StaleCandidate { .. } => "stale_candidate".to_owned(),
        CoreError::InvalidTextCandidate => "invalid_text_output".to_owned(),
        CoreError::Domain(_) => "invalid_prompt".to_owned(),
        CoreError::Store(_) | CoreError::MissingAcceptedRevision { .. } => {
            "project_unavailable".to_owned()
        }
        CoreError::BranchRequiresAcceptedSource { .. } => "project_unavailable".to_owned(),
        CoreError::Execution(_) => "execution_invalid".to_owned(),
    }
}

#[cfg(test)]
mod tests;
