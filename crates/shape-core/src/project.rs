//! Project lifecycle and the deterministic text foundation slice.

use std::{fmt, path::Path};

use shape_domain::{
    Artifact, ArtifactId, ArtifactKind, ArtifactRevision, Constraint, IntentSpec, RevisionId,
    Transformation, TransformationId, TransformationKind,
};
use shape_execution::{
    CapabilityId, ExecutedCandidate, ExecutionCoordinator, ExecutionFailure, ExecutionOutput,
    ExecutionReceipt, ExecutionRequest, Executor, ExecutorIdentity,
};
use shape_store::{AcceptedCommit, NewArtifactCommit, ProjectSnapshot, ProjectStore};

use crate::CoreError;

const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";
const BUILTIN_CONTRACT_REVISION: &str = "20260810.1";

/// Open application facade for one Shape project bundle.
#[derive(Debug)]
pub struct ShapeProject {
    store: ProjectStore,
}

/// Transient text candidate awaiting explicit user acceptance.
#[derive(Clone)]
pub struct TextCandidate {
    artifact_id: ArtifactId,
    expected_head: Option<RevisionId>,
    transformation: Transformation,
    receipt: ExecutionReceipt,
    output_text: String,
    output_media_type: String,
}

impl fmt::Debug for TextCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextCandidate")
            .field("artifact_id", &self.artifact_id)
            .field("expected_head", &self.expected_head)
            .field("transformation_id", &self.transformation.id)
            .field("receipt", &self.receipt)
            .field("output_byte_length", &self.output_text.len())
            .field("output_media_type", &self.output_media_type)
            .finish()
    }
}

impl TextCandidate {
    /// Returns the target artifact identity.
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifact_id
    }

    /// Returns the head against which this candidate was prepared.
    #[must_use]
    pub const fn expected_head(&self) -> Option<RevisionId> {
        self.expected_head
    }

    /// Returns candidate text for preview. It is not durable history yet.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.output_text
    }

    /// Returns payload-free physical provenance.
    #[must_use]
    pub const fn receipt(&self) -> &ExecutionReceipt {
        &self.receipt
    }
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

    /// Loads one immutable accepted revision for lineage resolution.
    ///
    /// # Errors
    ///
    /// Returns an error when durable revision data is absent or invalid.
    pub fn revision(&self, revision_id: RevisionId) -> Result<ArtifactRevision, CoreError> {
        Ok(self.store.revision(revision_id)?)
    }

    /// Executes a deterministic text proposal without changing durable history.
    ///
    /// The candidate is tied to `expected_head`; callers must preview it and
    /// separately invoke [`Self::accept_text`] to advance the artifact.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown/stale artifact, invalid intent, or execution failure.
    pub fn propose_text(
        &self,
        artifact_id: ArtifactId,
        expected_head: Option<RevisionId>,
        text: impl Into<String>,
        intent: IntentSpec,
        constraints: Vec<Constraint>,
    ) -> Result<TextCandidate, CoreError> {
        let artifact = self
            .store
            .artifact(artifact_id)?
            .ok_or(shape_store::StoreError::UnknownArtifact(artifact_id))?;
        if artifact.accepted_revision != expected_head {
            return Err(CoreError::StaleCandidate {
                artifact_id,
                expected: expected_head,
                actual: artifact.accepted_revision,
            });
        }

        let (kind, inputs, input_content) = match expected_head {
            Some(revision_id) => {
                let revision = self.store.accepted_revision(artifact_id)?.ok_or(
                    CoreError::MissingAcceptedRevision {
                        artifact_id,
                        revision_id,
                    },
                )?;
                debug_assert_eq!(revision.id, revision_id);
                (
                    TransformationKind::TextRewrite,
                    vec![revision_id],
                    vec![revision.content],
                )
            }
            None => (TransformationKind::Import, Vec::new(), Vec::new()),
        };
        let transformation =
            Transformation::new(kind, artifact_id, inputs, intent, constraints, Vec::new())?;
        let capability = CapabilityId::new("text.literal")?;
        let request = ExecutionRequest::new(
            transformation.id,
            capability,
            input_content,
            text.into().into_bytes(),
            TEXT_MEDIA_TYPE,
        )?;
        let ExecutedCandidate { output, receipt } =
            ExecutionCoordinator::execute(&LiteralTextExecutor::new()?, &request)?;
        let output_text =
            String::from_utf8(output.bytes).map_err(|_| CoreError::InvalidTextCandidate)?;
        Ok(TextCandidate {
            artifact_id,
            expected_head,
            transformation,
            receipt,
            output_text,
            output_media_type: output.media_type,
        })
    }

    /// Explicitly accepts a previously executed text candidate.
    ///
    /// # Errors
    ///
    /// Returns an error when its expected head is stale or durable publication fails.
    pub fn accept_text(&mut self, candidate: TextCandidate) -> Result<ArtifactRevision, CoreError> {
        Ok(self.store.accept(AcceptedCommit {
            artifact_id: candidate.artifact_id,
            expected_head: candidate.expected_head,
            transformation: candidate.transformation,
            receipt: candidate.receipt,
            output_bytes: candidate.output_text.into_bytes(),
            output_media_type: candidate.output_media_type,
        })?)
    }

    /// Accepts a deterministic text candidate as a newly named artifact.
    ///
    /// The source artifact head is left unchanged. The new artifact starts with
    /// no same-artifact parent, while its transformation records the source
    /// revision as an input. The exact candidate bytes are re-executed so the
    /// new transformation receives truthful, target-specific execution evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when the source has no accepted head, its head became
    /// stale, the name is invalid, execution fails, or atomic publication fails.
    pub fn branch_text_candidate(
        &mut self,
        candidate: TextCandidate,
        artifact_name: impl Into<String>,
    ) -> Result<ArtifactRevision, CoreError> {
        let source_artifact = self.store.artifact(candidate.artifact_id)?.ok_or(
            shape_store::StoreError::UnknownArtifact(candidate.artifact_id),
        )?;
        if source_artifact.accepted_revision != candidate.expected_head {
            return Err(CoreError::StaleCandidate {
                artifact_id: candidate.artifact_id,
                expected: candidate.expected_head,
                actual: source_artifact.accepted_revision,
            });
        }
        let source_revision_id =
            candidate
                .expected_head
                .ok_or(CoreError::BranchRequiresAcceptedSource {
                    artifact_id: candidate.artifact_id,
                })?;
        let source_revision = self.store.revision(source_revision_id)?;
        let target = Artifact::new(artifact_name, source_artifact.kind)?;
        let transformation = Transformation::new(
            TransformationKind::TextRewrite,
            target.id,
            vec![source_revision_id],
            candidate.transformation.intent,
            candidate.transformation.constraints,
            candidate.transformation.references,
        )?;
        let request = ExecutionRequest::new(
            transformation.id,
            CapabilityId::new("text.literal")?,
            vec![source_revision.content],
            candidate.output_text.into_bytes(),
            TEXT_MEDIA_TYPE,
        )?;
        let ExecutedCandidate { output, receipt } =
            ExecutionCoordinator::execute(&LiteralTextExecutor::new()?, &request)?;
        Ok(self.store.accept_new_artifact(NewArtifactCommit {
            artifact: target,
            transformation,
            receipt,
            output_bytes: output.bytes,
            output_media_type: output.media_type,
        })?)
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

#[derive(Debug)]
struct LiteralTextExecutor {
    identity: ExecutorIdentity,
}

impl LiteralTextExecutor {
    fn new() -> Result<Self, shape_execution::ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.literal-text",
                env!("CARGO_PKG_VERSION"),
                BUILTIN_CONTRACT_REVISION,
            )?,
        })
    }
}

impl Executor for LiteralTextExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == "text.literal"
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        let text = std::str::from_utf8(&request.instruction).map_err(|_| {
            ExecutionFailure::new(
                "invalid_utf8",
                "text candidate instruction is not valid UTF-8",
                false,
            )
        })?;
        Ok(ExecutionOutput {
            bytes: text.as_bytes().to_vec(),
            media_type: request.output_media_type.clone(),
        })
    }
}
