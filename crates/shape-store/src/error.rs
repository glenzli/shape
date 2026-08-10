//! Project bundle and durable object failures.

use std::path::PathBuf;

use shape_domain::{ArtifactId, RevisionId, TransformationId};
use thiserror::Error;

/// A failure to create, validate, read, or atomically update a Shape project.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("project bundle already exists at {0}")]
    BundleExists(PathBuf),
    #[error("project bundle does not exist at {0}")]
    BundleMissing(PathBuf),
    #[error("project schema {actual} is incompatible with required {required}")]
    SchemaMismatch {
        actual: String,
        required: &'static str,
    },
    #[error("project manifest and SQLite metadata disagree")]
    MetadataMismatch,
    #[error("artifact {0} does not exist")]
    UnknownArtifact(ArtifactId),
    #[error("artifact {0} already exists")]
    ArtifactAlreadyExists(ArtifactId),
    #[error("transformation {0} does not exist")]
    UnknownTransformation(TransformationId),
    #[error("accepted head changed: expected {expected:?}, actual {actual:?}")]
    RevisionConflict {
        expected: Option<RevisionId>,
        actual: Option<RevisionId>,
    },
    #[error("invalid accepted commit: {0}")]
    InvalidCommit(&'static str),
    #[error("content object {path} failed its BLAKE3 or length verification")]
    CorruptObject { path: PathBuf },
    #[error("integer value cannot be represented by SQLite")]
    IntegerOutOfRange,
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("project JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("domain contract is invalid: {0}")]
    Domain(#[from] shape_domain::DomainError),
}
