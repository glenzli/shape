//! Application use-case failures.

use shape_domain::{ArtifactId, RevisionId};
use thiserror::Error;

/// A failure in a Shape project use case.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("domain contract failed: {0}")]
    Domain(#[from] shape_domain::DomainError),
    #[error("execution failed: {0}")]
    Execution(#[from] shape_execution::ExecutionError),
    #[error("project persistence failed: {0}")]
    Store(#[from] shape_store::StoreError),
    #[error("artifact {artifact_id} head changed: expected {expected:?}, actual {actual:?}")]
    StaleCandidate {
        artifact_id: ArtifactId,
        expected: Option<RevisionId>,
        actual: Option<RevisionId>,
    },
    #[error("artifact {artifact_id} points to missing accepted revision {revision_id}")]
    MissingAcceptedRevision {
        artifact_id: ArtifactId,
        revision_id: RevisionId,
    },
    #[error("text executor returned bytes that are not valid UTF-8")]
    InvalidTextCandidate,
}
