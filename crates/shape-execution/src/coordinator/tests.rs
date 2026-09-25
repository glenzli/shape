use shape_domain::TransformationId;

use super::*;
use crate::{CapabilityId, ExecutionFailure, ExecutionOutcome, ExecutorIdentity};

#[derive(Debug)]
struct LiteralExecutor {
    identity: ExecutorIdentity,
}

impl LiteralExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new("shape.test.literal", "1", "20260810.1")
                .expect("valid identity"),
        }
    }
}

impl Executor for LiteralExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == "text.literal"
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        Ok(ExecutionOutput {
            bytes: request.instruction.clone(),
            media_type: request.output_media_type.clone(),
            executor_job_id: None,
            external_provenance: None,
            content_contract: None,
        })
    }
}

#[test]
fn success_is_still_only_a_candidate() {
    let request = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new("text.literal").expect("valid capability"),
        Vec::new(),
        b"candidate".to_vec(),
        "text/plain; charset=utf-8",
    )
    .expect("valid request");

    let candidate = ExecutionCoordinator::execute(&LiteralExecutor::new(), &request)
        .expect("execution succeeds");

    assert_eq!(candidate.output.bytes, b"candidate");
    assert_eq!(candidate.receipt.outcome, ExecutionOutcome::Succeeded);
}

#[test]
fn capability_is_checked_before_executor_invocation() {
    let request = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new("image.generate").expect("valid capability"),
        Vec::new(),
        Vec::new(),
        "image/png",
    )
    .expect("valid request");

    assert!(matches!(
        ExecutionCoordinator::execute(&LiteralExecutor::new(), &request),
        Err(ExecutionError::UnsupportedCapability { .. })
    ));
}

#[derive(Debug)]
struct CompletedUpstreamButRejectedLocally {
    identity: ExecutorIdentity,
}

impl Executor for CompletedUpstreamButRejectedLocally {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }
    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == "agent.file_task"
    }
    fn execute(&self, _: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        Err(
            ExecutionFailure::new("infer_invalid_response", "invalid receipt", false)
                .with_executor_job_id("agent_completed_123"),
        )
    }
}

#[test]
fn failed_local_validation_retains_completed_upstream_job_identity() {
    let request = ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new("agent.file_task").unwrap(),
        Vec::new(),
        Vec::new(),
        "text/plain; charset=utf-8",
    )
    .unwrap();
    let executor = CompletedUpstreamButRejectedLocally {
        identity: ExecutorIdentity::new("test.agent", "1", "test").unwrap(),
    };
    let Err(ExecutionError::ExecutorFailed { receipt, failure }) =
        ExecutionCoordinator::execute(&executor, &request)
    else {
        panic!("must fail");
    };
    assert_eq!(
        receipt.executor_job_id.as_deref(),
        Some("agent_completed_123")
    );
    assert_eq!(
        failure.executor_job_id.as_deref(),
        Some("agent_completed_123")
    );
    assert!(!failure.retryable);
}
