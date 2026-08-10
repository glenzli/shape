//! Image-raster import, deterministic crop candidates, and explicit acceptance.

use std::{fmt, fs::File, io::Read, path::Path, sync::Arc};

use shape_domain::{
    Artifact, ArtifactContentContract, ArtifactId, ArtifactKind, ArtifactRevision,
    ImageRasterContract, IntentSpec, RasterCrop, RevisionId, Transformation, TransformationKind,
    TransformationOperation,
};
use shape_execution::{
    CapabilityId, ExecutedCandidate, ExecutionCoordinator, ExecutionInput, ExecutionReceipt,
    ExecutionRequest, RASTER_CROP_CAPABILITY, RASTER_IMPORT_CAPABILITY, RasterExecutor,
};
use shape_store::{AcceptedCommit, NewArtifactCommit};

use super::ShapeProject;
use crate::CoreError;

const RASTER_MEDIA_TYPE: &str = "image/png";
const MAX_RASTER_SOURCE_BYTES: u64 = 128 * 1024 * 1024;
const IMPORT_INTENT: &str = "Import raster image";

/// Transient canonical PNG produced by a deterministic raster edit.
#[derive(Clone)]
pub struct ImageCandidate {
    artifact_id: ArtifactId,
    expected_head: RevisionId,
    transformation: Transformation,
    receipt: ExecutionReceipt,
    output_bytes: Arc<[u8]>,
    output_media_type: String,
    output_contract: ImageRasterContract,
}

impl fmt::Debug for ImageCandidate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageCandidate")
            .field("artifact_id", &self.artifact_id)
            .field("expected_head", &self.expected_head)
            .field("transformation_id", &self.transformation.id)
            .field("receipt", &self.receipt)
            .field("output_byte_length", &self.output_bytes.len())
            .field("output_media_type", &self.output_media_type)
            .field("output_contract", &self.output_contract)
            .finish()
    }
}

impl ImageCandidate {
    #[must_use]
    pub const fn artifact_id(&self) -> ArtifactId {
        self.artifact_id
    }

    #[must_use]
    pub const fn expected_head(&self) -> RevisionId {
        self.expected_head
    }

    #[must_use]
    pub const fn contract(&self) -> &ImageRasterContract {
        &self.output_contract
    }

    #[must_use]
    pub fn png_bytes(&self) -> &[u8] {
        &self.output_bytes
    }

    #[must_use]
    pub const fn receipt(&self) -> &ExecutionReceipt {
        &self.receipt
    }

    #[must_use]
    pub const fn operation(&self) -> Option<&TransformationOperation> {
        self.transformation.operation.as_ref()
    }
}

impl ShapeProject {
    /// Imports PNG/JPEG bytes as a normalized, accepted `image.raster` artifact.
    ///
    /// Artifact, import Transformation, receipt, canonical PNG, and accepted
    /// revision are published atomically. Source paths are never persisted.
    ///
    /// # Errors
    ///
    /// Rejects non-regular, empty, oversized, unsupported, or unsafe image sources.
    pub fn import_raster(
        &mut self,
        source_path: impl AsRef<Path>,
        artifact_name: impl Into<String>,
    ) -> Result<ArtifactRevision, CoreError> {
        let source = read_source(source_path.as_ref())?;
        let artifact = Artifact::new(artifact_name, ArtifactKind::ImageRaster)?;
        let transformation = Transformation::new(
            TransformationKind::Import,
            artifact.id,
            Vec::new(),
            IntentSpec::new(IMPORT_INTENT)?,
            Vec::new(),
            Vec::new(),
        )?;
        let request = ExecutionRequest::new(
            transformation.id,
            CapabilityId::new(RASTER_IMPORT_CAPABILITY)?,
            Vec::new(),
            source,
            RASTER_MEDIA_TYPE,
        )?;
        let ExecutedCandidate { output, receipt } =
            ExecutionCoordinator::execute(&RasterExecutor::new()?, &request)?;
        let contract = output_contract(output.content_contract)?;
        Ok(self.store.accept_new_artifact(NewArtifactCommit {
            artifact,
            transformation,
            receipt,
            output_bytes: output.bytes.into(),
            output_media_type: output.media_type,
            content_contract: Some(ArtifactContentContract::ImageRaster(contract)),
        })?)
    }

    /// Produces a deterministic crop without advancing durable history.
    ///
    /// # Errors
    ///
    /// Rejects unknown, stale, non-raster, full-frame, or out-of-bounds requests.
    pub fn propose_raster_crop(
        &self,
        artifact_id: ArtifactId,
        expected_head: RevisionId,
        crop: RasterCrop,
    ) -> Result<ImageCandidate, CoreError> {
        let artifact = self
            .store
            .artifact(artifact_id)?
            .ok_or(shape_store::StoreError::UnknownArtifact(artifact_id))?;
        if artifact.kind != ArtifactKind::ImageRaster {
            return Err(CoreError::InvalidRasterContent { artifact_id });
        }
        if artifact.accepted_revision != Some(expected_head) {
            return Err(CoreError::StaleCandidate {
                artifact_id,
                expected: Some(expected_head),
                actual: artifact.accepted_revision,
            });
        }
        let accepted =
            self.read_accepted(artifact_id)?
                .ok_or(CoreError::MissingAcceptedRevision {
                    artifact_id,
                    revision_id: expected_head,
                })?;
        let source_contract = raster_contract(&accepted.revision, artifact_id)?;
        let crop = RasterCrop::new(
            crop.x,
            crop.y,
            crop.width,
            crop.height,
            source_contract.width,
            source_contract.height,
        )?;
        if crop.x == 0
            && crop.y == 0
            && crop.width == source_contract.width
            && crop.height == source_contract.height
        {
            return Err(CoreError::NoOpRasterCrop);
        }
        let transformation = Transformation::new_with_operation(
            TransformationKind::DeterministicEdit,
            artifact_id,
            vec![expected_head],
            IntentSpec::new(format!(
                "Crop raster to x={}, y={}, width={}, height={}",
                crop.x, crop.y, crop.width, crop.height
            ))?,
            Vec::new(),
            Vec::new(),
            Some(TransformationOperation::RasterCrop(crop)),
        )?;
        let input = ExecutionInput::materialized(accepted.revision.content, accepted.bytes)?;
        let request = ExecutionRequest::new_materialized(
            transformation.id,
            CapabilityId::new(RASTER_CROP_CAPABILITY)?,
            vec![input],
            serde_json::to_vec(&crop).map_err(shape_store::StoreError::Json)?,
            RASTER_MEDIA_TYPE,
        )?;
        let ExecutedCandidate { output, receipt } =
            ExecutionCoordinator::execute(&RasterExecutor::new()?, &request)?;
        let output_contract = output_contract(output.content_contract)?;
        Ok(ImageCandidate {
            artifact_id,
            expected_head,
            transformation,
            receipt,
            output_bytes: output.bytes.into(),
            output_media_type: output.media_type,
            output_contract,
        })
    }

    /// Explicitly accepts one deterministic raster candidate.
    ///
    /// # Errors
    ///
    /// Returns an error when its expected head is stale or publication fails.
    pub fn accept_raster_crop(
        &mut self,
        candidate: ImageCandidate,
    ) -> Result<ArtifactRevision, CoreError> {
        let contract = candidate.output_contract.clone();
        Ok(self.store.accept(AcceptedCommit {
            artifact_id: candidate.artifact_id,
            expected_head: Some(candidate.expected_head),
            transformation: candidate.transformation,
            receipt: candidate.receipt,
            output_bytes: candidate.output_bytes,
            output_media_type: candidate.output_media_type,
            content_contract: Some(ArtifactContentContract::ImageRaster(contract)),
        })?)
    }
}

fn read_source(path: &Path) -> Result<Vec<u8>, CoreError> {
    let file = File::open(path).map_err(source_io)?;
    let metadata = file.metadata().map_err(source_io)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_RASTER_SOURCE_BYTES {
        return Err(CoreError::InvalidRasterSource {
            maximum_bytes: MAX_RASTER_SOURCE_BYTES,
        });
    }
    let capacity = usize::try_from(metadata.len()).map_err(|_| CoreError::InvalidRasterSource {
        maximum_bytes: MAX_RASTER_SOURCE_BYTES,
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_RASTER_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(source_io)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(CoreError::InvalidRasterSource {
            maximum_bytes: MAX_RASTER_SOURCE_BYTES,
        });
    }
    Ok(bytes)
}

fn source_io(error: std::io::Error) -> CoreError {
    CoreError::Store(shape_store::StoreError::Io(error))
}

fn output_contract(
    contract: Option<ArtifactContentContract>,
) -> Result<ImageRasterContract, CoreError> {
    match contract {
        Some(ArtifactContentContract::ImageRaster(contract)) => Ok(contract),
        None => Err(CoreError::MissingRasterOutputContract),
    }
}

fn raster_contract(
    revision: &ArtifactRevision,
    artifact_id: ArtifactId,
) -> Result<&ImageRasterContract, CoreError> {
    match revision.content_contract.as_ref() {
        Some(ArtifactContentContract::ImageRaster(contract)) => Ok(contract),
        None => Err(CoreError::InvalidRasterContent { artifact_id }),
    }
}

#[cfg(test)]
mod tests;
