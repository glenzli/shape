//! Verifies a selected editor return; never asserts that an external editor ran.

use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity, RASTER_IMPORT_CAPABILITY, RasterExecutor, parse_pcm_s16le_wav,
};
mod pcm24;

use shape_domain::{ArtifactContentContract, MaterialEditor};

pub const MATERIAL_RETURN_EXECUTOR: &str = "shape.builtin.material-return";
pub const MATERIAL_RETURN_REVISION: &str = "20261003.1";

/// Local byte validation/normalization with the original audio disclosure retained.
#[derive(Debug)]
pub struct MaterialReturnExecutor {
    identity: ExecutorIdentity,
    source_contract: ArtifactContentContract,
    editor: MaterialEditor,
    allow_pcm24: bool,
}

impl MaterialReturnExecutor {
    /// Enables an explicitly requested reduction in sample precision; original files stay intact.
    #[must_use]
    pub const fn with_pcm24_conversion(mut self, allow: bool) -> Self {
        self.allow_pcm24 = allow;
        self
    }

    /// Creates a verifier for one typed source.
    /// # Errors
    /// Rejects a source/editor family mismatch.
    pub fn new(
        editor: MaterialEditor,
        source_contract: ArtifactContentContract,
    ) -> Result<Self, ExecutionError> {
        if !matches!(
            (editor, &source_contract),
            (
                MaterialEditor::Shadow,
                ArtifactContentContract::ImageRaster(_)
            ) | (MaterialEditor::Echo, ArtifactContentContract::AudioClip(_))
        ) {
            return Err(ExecutionError::InvalidInput);
        }
        Ok(Self {
            identity: ExecutorIdentity::new(
                MATERIAL_RETURN_EXECUTOR,
                env!("CARGO_PKG_VERSION"),
                MATERIAL_RETURN_REVISION,
            )?,
            source_contract,
            editor,
            allow_pcm24: false,
        })
    }
}

impl Executor for MaterialReturnExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }
    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == self.editor.operator_type()
    }
    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if !self.supports(&request.capability)
            || request.inputs.len() != 1
            || request.output_media_type != self.editor.media_type()
            || request.inputs[0].content().media_type != self.editor.media_type()
            || request.inputs[0].bytes().is_none()
        {
            return Err(invalid());
        }
        if self.allow_pcm24
            && self.editor == MaterialEditor::Echo
            && pcm24::is_pcm24(&request.instruction)
        {
            return normalize_material_return(
                &self.source_contract,
                &pcm24::convert(&request.instruction)?,
            );
        }
        normalize_material_return(&self.source_contract, &request.instruction)
    }
}

/// Normalizes PNG or verifies exact PCM16 WAV while preserving source disclosure.
/// # Errors
/// Rejects unsupported, malformed or oversized bytes; there is no lossy fallback.
pub fn normalize_material_return(
    source: &ArtifactContentContract,
    bytes: &[u8],
) -> Result<ExecutionOutput, ExecutionFailure> {
    match source {
        ArtifactContentContract::ImageRaster(_) => {
            if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err(invalid());
            }
            let request = ExecutionRequest::new(
                shape_domain::TransformationId::new(),
                CapabilityId::new(RASTER_IMPORT_CAPABILITY).map_err(|_| invalid())?,
                Vec::new(),
                bytes.to_vec(),
                "image/png",
            )
            .map_err(|_| invalid())?;
            RasterExecutor::new()
                .map_err(|_| invalid())?
                .execute(&request)
        }
        ArtifactContentContract::AudioClip(source) => {
            let contract = parse_pcm_s16le_wav(bytes, source.origin)?;
            Ok(ExecutionOutput {
                bytes: bytes.to_vec(),
                media_type: "audio/wav".into(),
                content_contract: Some(ArtifactContentContract::AudioClip(contract)),
                executor_job_id: None,
                external_provenance: None,
            })
        }
        ArtifactContentContract::TextDocument(_) => Err(invalid()),
    }
}

/// Returns whether the exact bytes satisfy the bounded PCM24 conversion input contract.
#[must_use]
pub fn is_pcm24_material_return(bytes: &[u8]) -> bool {
    pcm24::is_pcm24(bytes)
}

fn invalid() -> ExecutionFailure {
    ExecutionFailure::new(
        "invalid_material_return",
        "Editor returns require a valid PNG or PCM16 WAV matching the source family",
        false,
    )
}

#[cfg(test)]
mod tests;
