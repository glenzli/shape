//! Executor-neutral request, output, and capability contracts.

use serde::{Deserialize, Serialize};

use shape_domain::{ArtifactContentContract, ContentDigest, ContentRef, TransformationId};

use crate::{ExecutionError, ExecutionFailure};

const MAX_CAPABILITY_ID_BYTES: usize = 160;
const MAX_EXECUTOR_IDENTITY_BYTES: usize = 160;
const MAX_MEDIA_TYPE_BYTES: usize = 127;

/// A stable logical capability such as `text.literal` or `vision.generate`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Creates a portable capability identifier.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty, oversized, non-ASCII, or unnamespaced id.
    pub fn new(value: impl Into<String>) -> Result<Self, ExecutionError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CAPABILITY_ID_BYTES
            || !value.is_ascii()
            || !value.contains('.')
        {
            return Err(ExecutionError::InvalidCapabilityId);
        }
        Ok(Self(value))
    }

    /// Returns the stable identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Versioned identity of the physical executor selected for one attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutorIdentity {
    /// Stable executor implementation or service id.
    pub id: String,
    /// Implementation or deployment version.
    pub version: String,
    /// Wire or plugin contract revision.
    pub contract_revision: String,
}

impl ExecutorIdentity {
    /// Creates a bounded portable executor identity.
    ///
    /// # Errors
    ///
    /// Returns an error when any identity field is invalid.
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        contract_revision: impl Into<String>,
    ) -> Result<Self, ExecutionError> {
        let identity = Self {
            id: id.into(),
            version: version.into(),
            contract_revision: contract_revision.into(),
        };
        let fields = [
            identity.id.as_str(),
            identity.version.as_str(),
            identity.contract_revision.as_str(),
        ];
        if fields.iter().any(|field| {
            field.is_empty() || field.len() > MAX_EXECUTOR_IDENTITY_BYTES || !field.is_ascii()
        }) {
            return Err(ExecutionError::InvalidExecutorIdentity);
        }
        Ok(identity)
    }
}

/// Physical execution plan for one domain transformation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRequest {
    /// Creative transformation whose implementation is being attempted.
    pub transformation_id: TransformationId,
    /// Logical ability required from an executor.
    pub capability: CapabilityId,
    /// Immutable accepted inputs available to the executor.
    pub inputs: Vec<ExecutionInput>,
    /// Executor-specific bounded instruction bytes; not included in receipts.
    pub instruction: Vec<u8>,
    /// Requested candidate media type.
    pub output_media_type: String,
}

impl ExecutionRequest {
    /// Creates an execution request whose sensitive instruction remains transient.
    ///
    /// # Errors
    ///
    /// Returns an error when the output media type is not portable.
    pub fn new(
        transformation_id: TransformationId,
        capability: CapabilityId,
        inputs: Vec<ContentRef>,
        instruction: Vec<u8>,
        output_media_type: impl Into<String>,
    ) -> Result<Self, ExecutionError> {
        let output_media_type = output_media_type.into();
        if output_media_type.is_empty()
            || output_media_type.len() > MAX_MEDIA_TYPE_BYTES
            || !output_media_type.is_ascii()
            || !output_media_type.contains('/')
        {
            return Err(ExecutionError::InvalidMediaType);
        }
        Ok(Self {
            transformation_id,
            capability,
            inputs: inputs.into_iter().map(ExecutionInput::reference).collect(),
            instruction,
            output_media_type,
        })
    }

    /// Creates a request with verified, transient materialized input bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the output media type is invalid or any payload
    /// does not match its immutable content reference.
    pub fn new_materialized(
        transformation_id: TransformationId,
        capability: CapabilityId,
        inputs: Vec<ExecutionInput>,
        instruction: Vec<u8>,
        output_media_type: impl Into<String>,
    ) -> Result<Self, ExecutionError> {
        if inputs.iter().any(|input| input.bytes.is_none()) {
            return Err(ExecutionError::InvalidInput);
        }
        let request = Self::new(
            transformation_id,
            capability,
            Vec::new(),
            instruction,
            output_media_type,
        )?;
        Ok(Self { inputs, ..request })
    }
}

/// One immutable executor input with optional verified payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionInput {
    content: ContentRef,
    bytes: Option<Vec<u8>>,
}

impl ExecutionInput {
    /// Carries provenance without materializing bytes for executors that do not need them.
    #[must_use]
    pub const fn reference(content: ContentRef) -> Self {
        Self {
            content,
            bytes: None,
        }
    }

    /// Binds exact bytes to a durable content reference.
    ///
    /// # Errors
    ///
    /// Rejects length or BLAKE3 mismatches before an executor receives input.
    pub fn materialized(content: ContentRef, bytes: Vec<u8>) -> Result<Self, ExecutionError> {
        let byte_length = u64::try_from(bytes.len()).map_err(|_| ExecutionError::InvalidInput)?;
        if byte_length != content.byte_length || ContentDigest::from_bytes(&bytes) != content.digest
        {
            return Err(ExecutionError::InvalidInput);
        }
        Ok(Self {
            content,
            bytes: Some(bytes),
        })
    }

    /// Returns the immutable content identity for this input.
    #[must_use]
    pub const fn content(&self) -> &ContentRef {
        &self.content
    }

    /// Returns verified payload bytes when this input is materialized.
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

/// Candidate bytes returned by a successful executor attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutput {
    /// Exact candidate payload. It is not durable until accepted.
    pub bytes: Vec<u8>,
    /// Media type of the candidate payload.
    pub media_type: String,
    /// Executor-owned job identity used to retrieve detailed physical
    /// provenance. It must not contain payloads or credentials.
    pub executor_job_id: Option<String>,
    /// Typed media interpretation for the candidate payload, when applicable.
    pub content_contract: Option<ArtifactContentContract>,
}

/// One implementation capable of physically executing a logical transformation.
pub trait Executor: Send + Sync {
    /// Returns versioned physical identity for provenance.
    fn identity(&self) -> &ExecutorIdentity;

    /// Reports whether this executor can handle a logical capability.
    fn supports(&self, capability: &CapabilityId) -> bool;

    /// Runs one physical attempt and returns transient candidate bytes.
    ///
    /// # Errors
    ///
    /// Returns a normalized, user-safe failure. Payloads and credentials must never enter it.
    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure>;
}

#[cfg(test)]
mod tests;
