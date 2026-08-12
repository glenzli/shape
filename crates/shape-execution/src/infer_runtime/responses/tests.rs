use std::collections::BTreeMap;

use infer_runtime_client::{Error as SdkError, ResponsesResult};
use serde_json::json;
use shape_domain::TransformationId;

use super::{InferRuntimeExecutor, TEXT_EDIT_DEPLOYMENT, local_text_request};
use crate::{CapabilityId, ExecutionRequest, Executor as _};

use crate::infer_runtime::{
    job_provenance::tests::text_job, sdk::SdkAdapterError, test_support::FakeSdk,
};

fn request() -> ExecutionRequest {
    ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new("text.generate").unwrap(),
        Vec::new(),
        b"Rewrite this paragraph clearly.".to_vec(),
        "text/plain; charset=utf-8",
    )
    .unwrap()
}

fn response() -> ResponsesResult {
    ResponsesResult {
        id: "resp_shape_job".into(),
        object: "response".into(),
        created_at: 1,
        model: "text.edit".into(),
        status: "completed".into(),
        output: vec![json!({
            "type":"message",
            "content":[{"type":"output_text","text":"A clearer paragraph."}]
        })],
        extra: BTreeMap::default(),
    }
}

#[test]
fn request_uses_text_edit_and_exact_local_named_narrowing() {
    let request = local_text_request("bounded fixture");
    assert_eq!(request.model, "text.edit");
    assert!(!request.stream);
    assert!(!request.background);
    assert!(request.tools.is_empty());
    assert_eq!(
        request
            .metadata
            .get("infer.deployment_ids")
            .map(String::as_str),
        Some(TEXT_EDIT_DEPLOYMENT)
    );
    assert_eq!(request.metadata["infer.placement"], "local_only");
    assert_eq!(request.metadata["infer.offline_required"], "true");
    assert_eq!(request.metadata["infer.fallback"], "none");
    assert_eq!(request.metadata["infer.max_cost_usd"], "0");
}

#[test]
fn sdk_response_and_typed_job_become_only_a_shape_execution_output() {
    let fake = FakeSdk::new().response(response()).job(text_job());
    let executor = InferRuntimeExecutor::with_sdk(Box::new(fake));
    let output = executor
        .execute(&request())
        .expect("fake SDK execution succeeds");
    assert_eq!(output.bytes, b"A clearer paragraph.");
    assert_eq!(output.executor_job_id.as_deref(), Some("resp_shape_job"));
    let provenance = output.external_provenance.expect("typed Job is copied");
    assert_eq!(provenance.intent, "text.edit");
    assert_eq!(provenance.deployment, TEXT_EDIT_DEPLOYMENT);
}

#[test]
fn sdk_machine_error_code_is_preserved_without_message_parsing() {
    let fake = FakeSdk::new();
    fake.responses
        .lock()
        .unwrap()
        .push_back(Err(SdkAdapterError::Sdk(SdkError::Api {
            status: reqwest::StatusCode::FORBIDDEN,
            code: "intent_forbidden".into(),
            message: "provider detail must not be parsed".into(),
        })));
    let error = InferRuntimeExecutor::with_sdk(Box::new(fake))
        .execute(&request())
        .expect_err("ACL denial fails closed");
    assert_eq!(error.code, "intent_forbidden");
    assert!(!error.retryable);
    assert!(!error.message.contains("provider detail"));
}

#[test]
fn malformed_prompt_is_rejected_before_sdk_transport() {
    let mut request = request();
    request.instruction.clear();
    let error = InferRuntimeExecutor::with_sdk(Box::new(FakeSdk::new()))
        .execute(&request)
        .expect_err("empty prompt is rejected");
    assert_eq!(error.code, "invalid_prompt");
}
