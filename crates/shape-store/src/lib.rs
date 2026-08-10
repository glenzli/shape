//! Durable ownership of portable `.shape` project bundles.
//!
//! One store owns both `SQLite` metadata and the BLAKE3 object directory so an
//! Artifact revision can be published with a clear order: durable bytes first,
//! then one `SQLite` compare-and-swap transaction. Scene graph revisions use the
//! same expected-head authority without entering the object store. Mutable Artifact Working Graphs
//! are stored separately and must remain anchored to an exact accepted revision. A failed metadata
//! transaction may leave an unreachable object, but can never publish a revision whose bytes are
//! absent.

mod error;
mod objects;
mod project_store;
mod schema;

pub use error::StoreError;
pub use project_store::{
    AcceptedCommit, AcceptedSceneRevisionCommit, NewArtifactCommit, ProjectSnapshot, ProjectStore,
};
