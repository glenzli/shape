//! Execution validation, lifecycle, and adapter failures.

use thiserror::Error;

use crate::{AttemptId, ExecutionReceipt, JobState};

/// A provider- or executor-reported failure safe for application handling.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("executor failed with {code}: {message}")]
pub struct ExecutionFailure {
    /// Stable machine-readable category.
    pub code: String,
    /// Bounded user-safe explanation. It must not contain payloads or credentials.
    pub message: String,
    /// Whether the same physical plan may be attempted again.
    pub retryable: bool,
}

impl ExecutionFailure {
    /// Creates a normalized executor failure.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}

/// A failure to validate or run a Shape execution job.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// A stable capability identifier was malformed.
    #[error("capability id must be a bounded ASCII identifier containing a dot")]
    InvalidCapabilityId,
    /// An executor identity field was empty, non-ASCII, or oversized.
    #[error("executor identity fields must be bounded non-empty ASCII strings")]
    InvalidExecutorIdentity,
    /// The request media type was not a portable media type.
    #[error("output media type must be bounded ASCII and contain a slash")]
    InvalidMediaType,
    /// A state transition is not legal for the current job state.
    #[error("cannot perform {operation} while execution job is {state:?}")]
    InvalidTransition {
        operation: &'static str,
        state: JobState,
    },
    /// Completion did not refer to the currently running attempt.
    #[error("attempt {actual} does not match active attempt {expected}")]
    StaleAttempt {
        expected: AttemptId,
        actual: AttemptId,
    },
    /// The selected executor cannot run the requested capability.
    #[error("executor does not support capability {capability}")]
    UnsupportedCapability { capability: String },
    /// The executor failed after a receipt was produced.
    #[error("execution attempt failed: {failure}")]
    ExecutorFailed {
        /// Durable, payload-free provenance for the failed attempt.
        receipt: Box<ExecutionReceipt>,
        /// Normalized executor failure returned to the caller.
        failure: Box<ExecutionFailure>,
    },
    /// The system clock could not be represented by the portable receipt.
    #[error("system clock is before the Unix epoch")]
    InvalidSystemClock,
}
