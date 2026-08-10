//! Durable ownership of portable `.shape` project bundles.
//!
//! One store owns both `SQLite` metadata and the BLAKE3 object directory so an
//! accepted revision can be published with a clear order: durable bytes first,
//! then one `SQLite` compare-and-swap transaction. A failed metadata transaction
//! may leave an unreachable object, but can never publish a revision whose bytes
//! are absent.

mod error;
mod objects;
mod project_store;
mod schema;

pub use error::StoreError;
pub use project_store::{AcceptedCommit, NewArtifactCommit, ProjectSnapshot, ProjectStore};
