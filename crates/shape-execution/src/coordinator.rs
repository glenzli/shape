//! Synchronous execution coordinator for one physical attempt.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    ExecutionError, ExecutionJob, ExecutionOutput, ExecutionReceipt, ExecutionRequest, Executor,
};

/// Successful transient candidate plus its durable, payload-free receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedCandidate {
    /// Candidate bytes. Acceptance remains a separate application operation.
    pub output: ExecutionOutput,
    /// Physical provenance that may be persisted on acceptance.
    pub receipt: ExecutionReceipt,
}

/// Owns the common validation and lifecycle around one executor call.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExecutionCoordinator;

impl ExecutionCoordinator {
    /// Runs one physical executor attempt.
    ///
    /// # Errors
    ///
    /// Returns an error when capability admission, the clock, lifecycle, or executor fails.
    pub fn execute(
        executor: &dyn Executor,
        request: &ExecutionRequest,
    ) -> Result<ExecutedCandidate, ExecutionError> {
        if !executor.supports(&request.capability) {
            return Err(ExecutionError::UnsupportedCapability {
                capability: request.capability.to_string(),
            });
        }

        let mut job = ExecutionJob::new(request.transformation_id, request.capability.clone());
        let attempt = job.start(executor.identity().clone(), unix_time_ms()?)?;
        match executor.execute(request) {
            Ok(output) => {
                let receipt = job.succeed(attempt, unix_time_ms()?)?;
                Ok(ExecutedCandidate { output, receipt })
            }
            Err(failure) => {
                let receipt = job.fail(attempt, unix_time_ms()?, &failure)?;
                Err(ExecutionError::ExecutorFailed {
                    receipt: Box::new(receipt),
                    failure: Box::new(failure),
                })
            }
        }
    }
}

fn unix_time_ms() -> Result<u64, ExecutionError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ExecutionError::InvalidSystemClock)?
        .as_millis();
    u64::try_from(milliseconds).map_err(|_| ExecutionError::InvalidSystemClock)
}

#[cfg(test)]
mod tests;
