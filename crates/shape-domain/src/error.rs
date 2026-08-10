//! Creative contract validation failures.

use thiserror::Error;

/// A violation of a platform-independent Shape domain invariant.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    /// A project name is empty or exceeds the portable limit.
    #[error("project name must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidProjectName { max_bytes: usize },
    /// An artifact name is empty or exceeds the portable limit.
    #[error("artifact name must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidArtifactName { max_bytes: usize },
    /// A media type is empty or is too large to persist portably.
    #[error("media type must contain 1..={max_bytes} ASCII bytes")]
    InvalidMediaType { max_bytes: usize },
    /// Intent text is empty or exceeds the bounded creative contract.
    #[error("intent summary must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidIntent { max_bytes: usize },
    /// A constraint description is empty or too large.
    #[error("constraint value must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidConstraint { max_bytes: usize },
    /// A component target is empty or too large.
    #[error("component target must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidComponentTarget { max_bytes: usize },
    /// A transformation has too many inputs, constraints, or references.
    #[error("{collection} contains {actual} items; maximum is {maximum}")]
    CollectionTooLarge {
        collection: &'static str,
        actual: usize,
        maximum: usize,
    },
    /// A transformation lists the same input revision more than once.
    #[error("transformation input revisions must be unique")]
    DuplicateTransformationInput,
    /// A reference label is empty or too large.
    #[error("reference label must contain 1..={max_bytes} UTF-8 bytes")]
    InvalidReferenceLabel { max_bytes: usize },
    /// A raster contract contained zero or non-portable dimensions.
    #[error("raster dimensions must be within 1..={maximum}; received {width}x{height}")]
    InvalidRasterDimensions {
        width: u32,
        height: u32,
        maximum: u32,
    },
    /// A crop rectangle was empty or exceeded its source raster.
    #[error(
        "crop rectangle ({x}, {y}, {width}, {height}) exceeds source {source_width}x{source_height}"
    )]
    InvalidRasterCrop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        source_width: u32,
        source_height: u32,
    },
    /// A typed operation was attached to an incompatible transformation family.
    #[error("typed operation requires a deterministic edit transformation")]
    InvalidTransformationOperation,
}
