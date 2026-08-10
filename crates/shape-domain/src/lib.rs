//! Platform-independent Creative Document contracts.
//!
//! This crate is the stable vocabulary shared by persistence, execution,
//! application services, and future desktop/media bridges:
//!
//! - [`ids`] owns strongly typed persistent identities;
//! - [`content`] owns immutable content references and media contracts;
//! - [`artifact`] owns creative identity and immutable accepted revisions;
//! - [`transformation`] owns creative meaning, constraints, and references;
//! - [`project`] owns project identity and schema revision.
//!
//! Filesystem, `SQLite`, Qt, C++, provider, and network types do not belong here.

mod artifact;
mod content;
mod error;
mod ids;
mod image_raster;
mod project;
mod transformation;

pub use artifact::{Artifact, ArtifactContentContract, ArtifactKind, ArtifactRevision};
pub use content::{ContentDigest, ContentDigestParseError, ContentRef};
pub use error::DomainError;
pub use ids::{ArtifactId, ProjectId, RevisionId, TransformationId};
pub use image_raster::{
    IMAGE_RASTER_CONTRACT_REVISION, ImageAlphaMode, ImageColorPrimaries, ImageColorProfile,
    ImageLightReference, ImageOrientation, ImagePixelFormat, ImageRasterContract,
    ImageTransferFunction, RasterCrop,
};
pub use project::{ProjectMetadata, SHAPE_PROJECT_SCHEMA_REVISION};
pub use transformation::{
    Constraint, ConstraintKind, ConstraintStrength, IntentSpec, ReferenceBinding, ReferenceRole,
    Transformation, TransformationKind, TransformationOperation,
};
