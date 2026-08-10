//! Platform-independent Creative Document contracts.
//!
//! This crate is the stable vocabulary shared by persistence, execution,
//! application services, and future desktop/media bridges:
//!
//! - [`ids`] owns strongly typed persistent identities;
//! - [`content`] owns immutable content references and media contracts;
//! - [`artifact`] owns creative identity and immutable accepted revisions;
//! - [`audio`] owns portable audio values, family ports, and voice authorization;
//! - [`image_crop`] owns the first stable image Operator contract;
//! - [`operator_graph`] owns typed Source/Operator/Output scene structure;
//! - [`scene`] owns stable Scene identity, immutable accepted graph revisions,
//!   and named output publication;
//! - [`transformation`] owns creative meaning, constraints, and references;
//! - [`working_graph`] owns durable mutable Operator entries outside accepted history;
//! - [`project`] owns project identity and schema revision.
//!
//! Filesystem, `SQLite`, Qt, C++, provider, and network types do not belong here.

mod artifact;
mod audio;
mod content;
mod error;
mod ids;
mod image_crop;
mod image_raster;
mod operator_graph;
mod project;
mod scene;
mod transformation;
mod working_graph;

pub use artifact::{Artifact, ArtifactContentContract, ArtifactKind, ArtifactRevision};
pub use audio::{
    AUDIO_CLIP_DATA_TYPE, AUDIO_VALUE_CONTRACT_REVISION, AUTHORIZED_VOICE_REFERENCE_DATA_TYPE,
    AudioContainer, AudioOperatorContract, AudioOperatorFamily, AudioOriginDisclosure,
    AudioPortCardinality, AudioPortContract, AudioPortDataType, AudioSampleFormat,
    AudioValueContract, AuthorizedVoiceReference, PresetVoiceAlias, PresetVoiceSelection,
    SpeechSynthesisOperation, SpeechVoiceSelection, VoiceAuthorizationScope,
};
pub use content::{ContentDigest, ContentDigestParseError, ContentRef};
pub use error::DomainError;
pub use ids::{ArtifactId, ProjectId, RevisionId, SceneId, SceneRevisionId, TransformationId};
pub use image_crop::{
    IMAGE_CROP_DATA_TYPE, IMAGE_CROP_OPERATOR_TYPE, IMAGE_CROP_PARAMETERS_REVISION,
    ImageCropContractError, ImageCropOperator, PreparedImageCrop, RasterCrop,
};
pub use image_raster::{
    IMAGE_RASTER_CONTRACT_REVISION, ImageAlphaMode, ImageColorPrimaries, ImageColorProfile,
    ImageLightReference, ImageOrientation, ImagePixelFormat, ImageRasterContract,
    ImageTransferFunction,
};
pub use operator_graph::{
    OperatorDataTypeId, OperatorGraph, OperatorGraphEdge, OperatorGraphNode, OperatorNodeBinding,
    OperatorNodeId, OperatorNodeRole, OperatorPort, OperatorPortId, OperatorTypeId,
};
pub use project::{ProjectMetadata, SHAPE_PROJECT_SCHEMA_REVISION};
pub use scene::{NamedSceneOutput, Scene, SceneOutputName, SceneRevision};
pub use transformation::{
    Constraint, ConstraintKind, ConstraintStrength, IntentSpec, ReferenceBinding, ReferenceRole,
    Transformation, TransformationKind, TransformationOperation,
};
pub use working_graph::{ArtifactWorkingGraph, WorkingOperatorDraft};
