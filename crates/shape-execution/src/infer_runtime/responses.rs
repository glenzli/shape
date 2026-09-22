//! Official-SDK-backed local text-edit executor.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use infer_runtime_client::{ResponsesRequest, ResponsesResult};
use serde_json::{Value, json};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    ExternalExecutionProvenance,
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, TEXT_EDIT_DEPLOYMENT, parse_job_snapshot, valid_job_id},
    official_sdk,
    sdk::{InferRuntimeSdk, SdkAdapterError, execution_failure},
};

const TEXT_GENERATE_CAPABILITY: &str = "text.generate";
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";
const TEXT_EDIT_INTENT: &str = "text.edit";
const MAX_PROMPT_BYTES: usize = 256 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_mins(2);

/// Shape's authenticated adapter for the stable Runtime Responses capability.
/// Runtime success remains a transient Shape Candidate until explicit Accept.
pub struct InferRuntimeExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
    deployment: &'static str,
}

impl std::fmt::Debug for InferRuntimeExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeExecutor")
            .field("identity", &self.identity)
            .field("deployment", &self.deployment)
            .field("sdk", &"official-infer-runtime-client")
            .finish()
    }
}

impl InferRuntimeExecutor {
    /// Creates one official SDK consumer using Discovery or an explicit
    /// development endpoint and Shape's existing managed credential file.
    ///
    /// # Errors
    ///
    /// Returns an error only if the built-in identity or SDK adapter is invalid.
    pub fn new(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
    ) -> Result<Self, crate::ExecutionError> {
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk(Box::new(sdk)))
    }

    /// Selects an explicit, allowlisted text deployment for one request.
    ///
    /// # Errors
    ///
    /// Rejects unknown model keys or an invalid SDK adapter.
    pub fn new_with_model(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
        model_key: &str,
    ) -> Result<Self, crate::ExecutionError> {
        let deployment =
            text_deployment(model_key).ok_or(crate::ExecutionError::InvalidExecutorIdentity)?;
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk_and_deployment(Box::new(sdk), deployment))
    }

    fn with_sdk(sdk: Box<dyn InferRuntimeSdk>) -> Self {
        Self::with_sdk_and_deployment(sdk, TEXT_EDIT_DEPLOYMENT)
    }

    fn with_sdk_and_deployment(sdk: Box<dyn InferRuntimeSdk>, deployment: &'static str) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("built-in Infer Runtime identity is valid"),
            sdk,
            deployment,
        }
    }

    fn execute_text(&self, prompt: &str) -> Result<ExecutionOutput, ExecutionFailure> {
        let response = self
            .sdk
            .create_response(&text_request(prompt, self.deployment), REQUEST_TIMEOUT)
            .map_err(map_failure)?;
        if !valid_job_id(&response.id) {
            return Err(failure("infer_invalid_response", false));
        }
        let job = self.sdk.job(&response.id).map_err(map_failure)?;
        let provenance = parse_job_snapshot(
            &response.id,
            TEXT_EDIT_INTENT,
            if self.deployment == TEXT_EDIT_DEPLOYMENT {
                JobPolicyProfile::LocalTextEdit
            } else {
                JobPolicyProfile::CloudTextEdit(self.deployment)
            },
            job,
        )
        .map_err(|error| failure(error.code(), false))?;
        parse_response(response, provenance)
    }
}

impl Executor for InferRuntimeExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == TEXT_GENERATE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.output_media_type != TEXT_MEDIA_TYPE {
            return Err(failure("unsupported_media_type", false));
        }
        let prompt = std::str::from_utf8(&request.instruction)
            .map_err(|_| failure("invalid_utf8", false))?;
        if prompt.is_empty() || prompt.len() > MAX_PROMPT_BYTES {
            return Err(failure("invalid_prompt", false));
        }
        self.execute_text(prompt)
    }
}

#[cfg(test)]
fn local_text_request(prompt: &str) -> ResponsesRequest {
    text_request(prompt, TEXT_EDIT_DEPLOYMENT)
}

fn text_deployment(model_key: &str) -> Option<&'static str> {
    match model_key {
        "local_qwen" => Some(TEXT_EDIT_DEPLOYMENT),
        "gpt_6_luna" => Some("codex_gpt_6_luna"),
        "gpt_6_sol" => Some("codex_gpt_6_sol"),
        _ => None,
    }
}

fn text_request(prompt: &str, deployment: &'static str) -> ResponsesRequest {
    let cloud = deployment != TEXT_EDIT_DEPLOYMENT;
    let mut metadata = BTreeMap::from([
        (
            "infer.capability_floor".to_owned(),
            "foundational".to_owned(),
        ),
        ("infer.deployment_ids".to_owned(), deployment.to_owned()),
        ("infer.fallback".to_owned(), "none".to_owned()),
        ("infer.max_cost_usd".to_owned(), "0".to_owned()),
        ("infer.offline_required".to_owned(), (!cloud).to_string()),
        (
            "infer.placement".to_owned(),
            if cloud { "cloud_only" } else { "local_only" }.to_owned(),
        ),
        (
            "infer.policy".to_owned(),
            if cloud { "balanced" } else { "local-first" }.to_owned(),
        ),
        (
            "infer.prefer".to_owned(),
            if cloud { "cloud" } else { "local" }.to_owned(),
        ),
        ("infer.priority".to_owned(), "interactive".to_owned()),
    ]);
    if cloud {
        metadata.insert(
            "infer.provider_access_class".to_owned(),
            "subscription".to_owned(),
        );
    } else {
        metadata.insert("infer.latency".to_owned(), "interactive".to_owned());
    }
    ResponsesRequest {
        model: TEXT_EDIT_INTENT.to_owned(),
        input: Value::String(prompt.to_owned()),
        instructions: None,
        stream: false,
        background: false,
        metadata,
        tools: Vec::new(),
        reasoning: cloud.then(|| json!({"effort": "low"})),
        max_output_tokens: None,
    }
}

fn parse_response(
    response: ResponsesResult,
    provenance: ExternalExecutionProvenance,
) -> Result<ExecutionOutput, ExecutionFailure> {
    if response.object != "response"
        || response.model != TEXT_EDIT_INTENT
        || response.status != "completed"
        || !valid_job_id(&response.id)
    {
        return Err(failure("infer_invalid_response", false));
    }
    let mut text = String::new();
    for item in &response.output {
        if item.get("type").and_then(Value::as_str) != Some("message") {
            continue;
        }
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            if part.get("type").and_then(Value::as_str) == Some("output_text")
                && let Some(part_text) = part.get("text").and_then(Value::as_str)
            {
                if text.len().saturating_add(part_text.len()) > MAX_RESPONSE_BYTES {
                    return Err(failure("infer_invalid_response", false));
                }
                text.push_str(part_text);
            }
        }
    }
    if text.is_empty() {
        return Err(failure("infer_empty_output", false));
    }
    Ok(ExecutionOutput {
        bytes: text.into_bytes(),
        media_type: TEXT_MEDIA_TYPE.to_owned(),
        executor_job_id: Some(response.id),
        external_provenance: Some(provenance),
        content_contract: None,
    })
}

fn map_failure(error: SdkAdapterError) -> ExecutionFailure {
    let (code, retryable) = execution_failure(error);
    failure(&code, retryable)
}

fn failure(code: &str, retryable: bool) -> ExecutionFailure {
    ExecutionFailure::new(
        code,
        "Infer Runtime could not produce a text candidate",
        retryable,
    )
}

#[cfg(test)]
mod tests;
