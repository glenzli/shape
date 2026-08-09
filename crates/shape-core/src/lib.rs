//! Product use cases over the domain, execution, and persistence owners.
//!
//! This crate demonstrates the critical foundation invariant: execution creates
//! a transient candidate, while an explicit acceptance call alone advances the
//! durable artifact head.

mod error;
mod project;

pub use error::CoreError;
pub use project::{AcceptedArtifactContent, ShapeProject, TextCandidate};
