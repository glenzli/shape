//! Product use cases over the domain, execution, and persistence owners.
//!
//! This crate demonstrates the critical foundation invariant: execution and
//! Scene graph editing create transient candidates, while explicit acceptance
//! alone advances a durable Artifact or Scene head.

mod error;
mod project;

pub use error::CoreError;
pub use project::{
    AcceptedArtifactContent, AgentTextCandidate, AiImageCandidate, AudioCandidate, ImageCandidate,
    ImageEditCandidate, ImageResizeCandidate, SceneGraphCandidate, ShapeProject,
    TEXT_DOCUMENT_DATA_TYPE, TEXT_EDIT_OPERATOR_TYPE, TEXT_TRANSFORM_OPERATOR_TYPE, TextCandidate,
    TextEditParameters, TextTransformMode, TextTransformParameters,
};
