//! Application use-case failures.

use shape_domain::{ArtifactId, RevisionId, SceneId, SceneRevisionId};
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
    #[error("creative JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("artifact {artifact_id} head changed: expected {expected:?}, actual {actual:?}")]
    StaleCandidate {
        artifact_id: ArtifactId,
        expected: Option<RevisionId>,
        actual: Option<RevisionId>,
    },
    #[error("scene {scene_id} head changed: expected {expected:?}, actual {actual:?}")]
    StaleSceneCandidate {
        scene_id: SceneId,
        expected: Option<SceneRevisionId>,
        actual: Option<SceneRevisionId>,
    },
    #[error("artifact {artifact_id} points to missing accepted revision {revision_id}")]
    MissingAcceptedRevision {
        artifact_id: ArtifactId,
        revision_id: RevisionId,
    },
    #[error("artifact {artifact_id} needs an accepted source revision before it can branch")]
    BranchRequiresAcceptedSource { artifact_id: ArtifactId },
    #[error("text executor returned bytes that are not valid UTF-8")]
    InvalidTextCandidate,
    #[error("artifact {artifact_id} is not a text.document input")]
    InvalidTextArtifact { artifact_id: ArtifactId },
    #[error("text transform instruction must not be empty")]
    InvalidTextTransformInstruction,
    #[error("artifact {artifact_id} is not an accepted text.document speech source")]
    InvalidSpeechSource { artifact_id: ArtifactId },
    #[error("authorized Voice References are not executable in the first speech synthesis slice")]
    UnsupportedSpeechVoiceReference,
    #[error("speech executor returned no audio.clip audio/wav contract")]
    MissingAudioOutputContract,
    #[error("speech output bytes, audio contract, or Runtime provenance do not match")]
    AudioOutputContractMismatch,
    #[error(
        "raster source must be a non-empty regular PNG or JPEG no larger than {maximum_bytes} bytes"
    )]
    InvalidRasterSource { maximum_bytes: u64 },
    #[error("artifact {artifact_id} does not carry a valid image.raster content contract")]
    InvalidRasterContent { artifact_id: ArtifactId },
    #[error("crop matches the full accepted raster and would create no change")]
    NoOpRasterCrop,
    #[error("resize resolves to the accepted raster dimensions and would create no change")]
    NoOpRasterResize,
    #[error("raster executor returned no image.raster content contract")]
    MissingRasterOutputContract,
    #[error("raster crop executor output does not match the prepared image.crop contract")]
    RasterCropOutputContractMismatch,
    #[error("raster resize executor output does not match the prepared image.resize contract")]
    RasterResizeOutputContractMismatch,
}
