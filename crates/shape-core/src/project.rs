//! Project bundle facade and routing to media-specific creative use cases.

mod audio;
mod image;
mod scene;
mod text;

pub use audio::AudioCandidate;
pub use image::ImageCandidate;
pub use scene::SceneGraphCandidate;
pub use text::{
    TEXT_DOCUMENT_DATA_TYPE, TEXT_EDIT_OPERATOR_TYPE, TEXT_TRANSFORM_OPERATOR_TYPE, TextCandidate,
    TextEditParameters, TextTransformMode, TextTransformParameters,
};

use std::path::Path;

use shape_domain::{
    Artifact, ArtifactId, ArtifactKind, ArtifactRevision, RevisionId, Transformation,
    TransformationId,
};
use shape_execution::{AttemptId, ExecutionReceipt};
use shape_store::{ProjectSnapshot, ProjectStore};

use crate::CoreError;

/// Open application facade for one Shape project bundle.
#[derive(Debug)]
pub struct ShapeProject {
    store: ProjectStore,
}

/// Verified accepted content loaded from durable history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedArtifactContent {
    pub revision: ArtifactRevision,
    pub bytes: Vec<u8>,
}

impl ShapeProject {
    /// Creates a new `.shape` project bundle.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid metadata or durable write failure.
    pub fn create(root: impl AsRef<Path>, name: impl Into<String>) -> Result<Self, CoreError> {
        Ok(Self {
            store: ProjectStore::create(root, name)?,
        })
    }

    /// Opens and validates an existing `.shape` project bundle.
    ///
    /// # Errors
    ///
    /// Returns an error for an incompatible or corrupt project.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, CoreError> {
        Ok(Self {
            store: ProjectStore::open(root)?,
        })
    }

    /// Creates a stable artifact without accepted content.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid name or durable write failure.
    pub fn create_artifact(
        &self,
        name: impl Into<String>,
        kind: ArtifactKind,
    ) -> Result<Artifact, CoreError> {
        let artifact = Artifact::new(name, kind)?;
        self.store.insert_artifact(&artifact)?;
        Ok(artifact)
    }

    /// Returns project metadata and current artifact heads.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid durable project state.
    pub fn snapshot(&self) -> Result<ProjectSnapshot, CoreError> {
        Ok(self.store.snapshot()?)
    }

    /// Loads one accepted creative transformation for inspection and lineage projection.
    ///
    /// # Errors
    ///
    /// Returns an error when durable transformation data is absent or invalid.
    pub fn transformation(
        &self,
        transformation_id: TransformationId,
    ) -> Result<Transformation, CoreError> {
        Ok(self.store.transformation(transformation_id)?)
    }

    /// Loads one accepted payload-free execution receipt for provenance inspection.
    ///
    /// # Errors
    ///
    /// Returns an error when the receipt is absent or invalid.
    pub fn execution_receipt(&self, attempt_id: AttemptId) -> Result<ExecutionReceipt, CoreError> {
        Ok(self.store.execution_receipt(attempt_id)?)
    }

    /// Loads one immutable accepted revision for lineage resolution.
    ///
    /// # Errors
    ///
    /// Returns an error when durable revision data is absent or invalid.
    pub fn revision(&self, revision_id: RevisionId) -> Result<ArtifactRevision, CoreError> {
        Ok(self.store.revision(revision_id)?)
    }

    /// Loads the current accepted revision and verifies its exact content object.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown artifact or corrupt durable object.
    pub fn read_accepted(
        &self,
        artifact_id: ArtifactId,
    ) -> Result<Option<AcceptedArtifactContent>, CoreError> {
        self.store
            .accepted_revision(artifact_id)?
            .map(|revision| {
                let bytes = self.store.read_content(&revision.content)?;
                Ok(AcceptedArtifactContent { revision, bytes })
            })
            .transpose()
    }
}
