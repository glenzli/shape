//! Physical execution contracts for creative transformations.
//!
//! The domain crate explains *why* content changes. This crate records *how* one
//! attempt ran, without importing provider-specific HTTP or model types. A
//! future Infer Runtime adapter and deterministic built-ins both implement the
//! same narrow [`Executor`] interface.

mod coordinator;
mod error;
mod executor;
mod ids;
mod job;

pub use coordinator::{ExecutedCandidate, ExecutionCoordinator};
pub use error::{ExecutionError, ExecutionFailure};
pub use executor::{CapabilityId, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity};
pub use ids::{AttemptId, JobId};
pub use job::{ExecutionJob, ExecutionOutcome, ExecutionReceipt, JobState};
