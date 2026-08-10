//! Execution job state machine and payload-free provenance receipts.

use serde::{Deserialize, Serialize};

use shape_domain::TransformationId;

use crate::{AttemptId, CapabilityId, ExecutionError, ExecutionFailure, ExecutorIdentity, JobId};

/// Observable state of one Shape-side execution job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Validated but not yet sent to an executor.
    Planned,
    /// One physical attempt is active.
    Running,
    /// Candidate bytes were returned successfully.
    Succeeded,
    /// The physical attempt failed.
    Failed,
    /// The job was cancelled before acceptance.
    Cancelled,
}

/// Payload-free terminal result captured in an execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExecutionOutcome {
    /// Candidate bytes were produced. This does not imply user acceptance.
    Succeeded,
    /// The attempt failed with a normalized stable code.
    Failed { code: String, retryable: bool },
    /// The attempt was cancelled before completion.
    Cancelled,
}

/// Durable provenance for one physical attempt, excluding content and credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub job_id: JobId,
    pub attempt_id: AttemptId,
    pub transformation_id: TransformationId,
    pub capability: CapabilityId,
    pub executor: ExecutorIdentity,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    /// Optional executor-owned identity for later provenance lookup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_job_id: Option<String>,
    pub outcome: ExecutionOutcome,
}

/// In-memory lifecycle owner for one execution job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionJob {
    id: JobId,
    transformation_id: TransformationId,
    capability: CapabilityId,
    state: JobState,
    active_attempt: Option<ActiveAttempt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveAttempt {
    id: AttemptId,
    executor: ExecutorIdentity,
    started_at_unix_ms: u64,
}

impl ExecutionJob {
    /// Plans a new job for one creative transformation.
    #[must_use]
    pub fn new(transformation_id: TransformationId, capability: CapabilityId) -> Self {
        Self {
            id: JobId::new(),
            transformation_id,
            capability,
            state: JobState::Planned,
            active_attempt: None,
        }
    }

    /// Returns the stable job identity.
    #[must_use]
    pub const fn id(&self) -> JobId {
        self.id
    }

    /// Returns the observable state.
    #[must_use]
    pub const fn state(&self) -> JobState {
        self.state
    }

    /// Starts the single physical attempt represented by this job.
    ///
    /// # Errors
    ///
    /// Returns an error unless the job is planned.
    pub fn start(
        &mut self,
        executor: ExecutorIdentity,
        started_at_unix_ms: u64,
    ) -> Result<AttemptId, ExecutionError> {
        if self.state != JobState::Planned {
            return Err(ExecutionError::InvalidTransition {
                operation: "start",
                state: self.state,
            });
        }
        let attempt_id = AttemptId::new();
        self.active_attempt = Some(ActiveAttempt {
            id: attempt_id,
            executor,
            started_at_unix_ms,
        });
        self.state = JobState::Running;
        Ok(attempt_id)
    }

    /// Completes the active attempt successfully.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid state or stale attempt identity.
    pub fn succeed(
        &mut self,
        attempt_id: AttemptId,
        completed_at_unix_ms: u64,
    ) -> Result<ExecutionReceipt, ExecutionError> {
        self.succeed_with_executor_job(attempt_id, completed_at_unix_ms, None)
    }

    /// Completes the active attempt and records an executor-owned provenance
    /// identity without treating it as Shape acceptance authority.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid state or stale attempt identity.
    pub fn succeed_with_executor_job(
        &mut self,
        attempt_id: AttemptId,
        completed_at_unix_ms: u64,
        executor_job_id: Option<String>,
    ) -> Result<ExecutionReceipt, ExecutionError> {
        self.finish(
            attempt_id,
            completed_at_unix_ms,
            executor_job_id,
            ExecutionOutcome::Succeeded,
            JobState::Succeeded,
        )
    }

    /// Completes the active attempt with a normalized failure.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid state or stale attempt identity.
    pub fn fail(
        &mut self,
        attempt_id: AttemptId,
        completed_at_unix_ms: u64,
        failure: &ExecutionFailure,
    ) -> Result<ExecutionReceipt, ExecutionError> {
        self.finish(
            attempt_id,
            completed_at_unix_ms,
            None,
            ExecutionOutcome::Failed {
                code: failure.code.clone(),
                retryable: failure.retryable,
            },
            JobState::Failed,
        )
    }

    /// Cancels the active attempt.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid state or stale attempt identity.
    pub fn cancel(
        &mut self,
        attempt_id: AttemptId,
        completed_at_unix_ms: u64,
    ) -> Result<ExecutionReceipt, ExecutionError> {
        self.finish(
            attempt_id,
            completed_at_unix_ms,
            None,
            ExecutionOutcome::Cancelled,
            JobState::Cancelled,
        )
    }

    fn finish(
        &mut self,
        attempt_id: AttemptId,
        completed_at_unix_ms: u64,
        executor_job_id: Option<String>,
        outcome: ExecutionOutcome,
        terminal_state: JobState,
    ) -> Result<ExecutionReceipt, ExecutionError> {
        if self.state != JobState::Running {
            return Err(ExecutionError::InvalidTransition {
                operation: "finish",
                state: self.state,
            });
        }
        let active = self
            .active_attempt
            .as_ref()
            .expect("running jobs always have an active attempt");
        if active.id != attempt_id {
            return Err(ExecutionError::StaleAttempt {
                expected: active.id,
                actual: attempt_id,
            });
        }
        let receipt = ExecutionReceipt {
            job_id: self.id,
            attempt_id,
            transformation_id: self.transformation_id,
            capability: self.capability.clone(),
            executor: active.executor.clone(),
            started_at_unix_ms: active.started_at_unix_ms,
            completed_at_unix_ms,
            executor_job_id,
            outcome,
        };
        self.active_attempt = None;
        self.state = terminal_state;
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests;
