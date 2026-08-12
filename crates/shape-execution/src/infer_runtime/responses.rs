//! Authenticated, local-first Infer Runtime text executor.

use std::{collections::BTreeMap, io::Read, time::Duration};

use reqwest::{Url, blocking::Client, header::CONTENT_TYPE, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
};

use super::{
    INFER_RUNTIME_CONTRACT_HEADER, INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient,
    InferRuntimeClientError, InferRuntimeCredential, InferRuntimeEndpointResolver,
    ResolvedInferRuntimeEndpoint,
    job_provenance::{JobPolicyProfile, JobProvenanceError, parse_job_snapshot},
    should_retry_endpoint, validate_discovered_contract,
};

const RESPONSES_PATH: &str = "v1/responses";
const JOB_PATH: &str = "infer/v1/jobs/";
const TEXT_GENERATE_CAPABILITY: &str = "text.generate";
const TEXT_MEDIA_TYPE: &str = "text/plain; charset=utf-8";
const TEXT_INTENT: &str = "text.edit";
const TEXT_DEPLOYMENT_ID: &str = "ollama_qwen3_5_4b";
const MAX_PROMPT_BYTES: usize = 256 * 1024;
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESPONSE_READ_BYTES: u64 = 4 * 1024 * 1024;
const MAX_JOB_BYTES: usize = 1024 * 1024;
const MAX_JOB_READ_BYTES: u64 = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const REQUEST_TIMEOUT: Duration = Duration::from_mins(2);

/// Shape's authenticated adapter for the Infer Runtime Responses data plane.
/// Runtime Job/Attempt provenance remains owned by Infer and is linked through
/// the response id captured in Shape's execution receipt.
pub struct InferRuntimeExecutor {
    identity: ExecutorIdentity,
    resolver: InferRuntimeEndpointResolver,
    credential: InferRuntimeCredential,
}

impl std::fmt::Debug for InferRuntimeExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeExecutor")
            .field("identity", &self.identity)
            .field("resolver", &self.resolver)
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

impl InferRuntimeExecutor {
    /// Creates one Consumer adapter. Endpoint resolution remains live and is
    /// repeated for every Shape execution attempt.
    ///
    /// # Errors
    ///
    /// Returns an error only if Shape's built-in adapter identity is invalid.
    pub fn new(
        explicit_override: &str,
        credential: InferRuntimeCredential,
    ) -> Result<Self, crate::ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )?,
            resolver: InferRuntimeEndpointResolver::from_environment(explicit_override),
            credential,
        })
    }

    #[cfg(test)]
    fn with_resolver(
        credential: InferRuntimeCredential,
        resolver: InferRuntimeEndpointResolver,
    ) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("adapter identity is valid"),
            resolver,
            credential,
        }
    }

    fn execute_resolved(
        &self,
        endpoint: &ResolvedInferRuntimeEndpoint,
        prompt: &str,
    ) -> Result<ExecutionOutput, ResponsesAttemptFailure> {
        let contract = InferRuntimeClient::new(&endpoint.origin)
            .and_then(|client| client.probe_contract())
            .map_err(|error| ResponsesAttemptFailure::from_contract(&error))?;
        validate_discovered_contract(endpoint, &contract)
            .map_err(|error| ResponsesAttemptFailure::from_contract(&error))?;
        let client = ResponsesClient::new(&endpoint.origin)?;
        client.create_text_response(&self.credential, prompt)
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

        let endpoint = self
            .resolver
            .resolve()
            .map_err(|_| failure("infer_invalid_endpoint", false))?;
        match self.execute_resolved(&endpoint, prompt) {
            Ok(output) => Ok(output),
            Err(first) if first.transport => {
                let rediscovered = self
                    .resolver
                    .resolve_after_connection_failure(&endpoint)
                    .map_err(|_| first.failure.clone())?;
                if should_retry_endpoint(&endpoint, &rediscovered) {
                    self.execute_resolved(&rediscovered, prompt)
                        .map_err(|error| error.failure)
                } else {
                    Err(first.failure)
                }
            }
            Err(error) => Err(error.failure),
        }
    }
}

struct ResponsesClient {
    origin: Url,
    endpoint: Url,
    client: Client,
}

impl ResponsesClient {
    fn new(origin: &str) -> Result<Self, ResponsesAttemptFailure> {
        let origin = super::canonical_loopback_url(origin)
            .map_err(|error| ResponsesAttemptFailure::from_contract(&error))?;
        let endpoint = origin
            .join(RESPONSES_PATH)
            .map_err(|_| ResponsesAttemptFailure::new("infer_invalid_endpoint", false, false))?;
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .no_proxy()
            .user_agent("shape/0.1 infer-responses-consumer")
            .build()
            .map_err(|_| ResponsesAttemptFailure::new("infer_client_invalid", false, false))?;
        Ok(Self {
            origin,
            endpoint,
            client,
        })
    }

    fn create_text_response(
        &self,
        credential: &InferRuntimeCredential,
        prompt: &str,
    ) -> Result<ExecutionOutput, ResponsesAttemptFailure> {
        let request = ResponsesRequest::local_text(prompt);
        let response = self
            .client
            .post(self.endpoint.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(
                INFER_RUNTIME_CONTRACT_HEADER,
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .bearer_auth(credential.expose())
            .json(&request)
            .send()
            .map_err(|_| ResponsesAttemptFailure::new("infer_unavailable", true, true))?;
        let status = response.status();
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ResponsesAttemptFailure::new("infer_invalid_response", false, false))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(ResponsesAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        let mut output = parse_response(&bytes)?;
        let job_id = output
            .executor_job_id
            .as_deref()
            .ok_or_else(|| ResponsesAttemptFailure::new("infer_invalid_response", false, false))?;
        output.external_provenance = Some(self.job_provenance(credential, job_id)?);
        Ok(output)
    }

    fn job_provenance(
        &self,
        credential: &InferRuntimeCredential,
        job_id: &str,
    ) -> Result<crate::ExternalExecutionProvenance, ResponsesAttemptFailure> {
        let endpoint = self
            .origin
            .join(&format!("{JOB_PATH}{job_id}"))
            .map_err(|_| ResponsesAttemptFailure::new("infer_invalid_response", false, false))?;
        let response = self
            .client
            .get(endpoint)
            .header(
                INFER_RUNTIME_CONTRACT_HEADER,
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .bearer_auth(credential.expose())
            .send()
            .map_err(|_| ResponsesAttemptFailure::new("infer_unavailable", true, true))?;
        let status = response.status();
        let mut bytes = Vec::new();
        response
            .take(MAX_JOB_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ResponsesAttemptFailure::new("infer_invalid_response", false, false))?;
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        if bytes.len() > MAX_JOB_BYTES {
            return Err(ResponsesAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        parse_job_snapshot(job_id, TEXT_INTENT, JobPolicyProfile::LocalTextEdit, &bytes)
            .map_err(ResponsesAttemptFailure::from_job_provenance)
    }
}

#[derive(Serialize)]
struct ResponsesRequest<'a> {
    model: &'static str,
    input: &'a str,
    stream: bool,
    metadata: BTreeMap<&'static str, &'static str>,
}

impl<'a> ResponsesRequest<'a> {
    fn local_text(input: &'a str) -> Self {
        let metadata = BTreeMap::from([
            ("infer.capability_floor", "foundational"),
            ("infer.deployment_ids", TEXT_DEPLOYMENT_ID),
            ("infer.fallback", "none"),
            ("infer.max_cost_usd", "0"),
            ("infer.offline_required", "true"),
            ("infer.placement", "local_only"),
            ("infer.policy", "local-first"),
            ("infer.prefer", "local"),
            ("infer.priority", "interactive"),
        ]);
        Self {
            model: TEXT_INTENT,
            input,
            stream: false,
            metadata,
        }
    }
}

#[derive(Deserialize)]
struct ResponseEnvelope {
    id: String,
    object: String,
    created_at: u64,
    model: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    output: Vec<Value>,
}

fn parse_response(bytes: &[u8]) -> Result<ExecutionOutput, ResponsesAttemptFailure> {
    let response: ResponseEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| ResponsesAttemptFailure::new("infer_invalid_response", false, false))?;
    let _ = response.created_at;
    if response.object != "response"
        || response.model != TEXT_INTENT
        || response
            .status
            .as_deref()
            .is_some_and(|status| status != "completed")
        || !valid_response_id(&response.id)
    {
        return Err(ResponsesAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ));
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
                    return Err(ResponsesAttemptFailure::new(
                        "infer_invalid_response",
                        false,
                        false,
                    ));
                }
                text.push_str(part_text);
            }
        }
    }
    if text.is_empty() {
        return Err(ResponsesAttemptFailure::new(
            "infer_empty_output",
            false,
            false,
        ));
    }
    Ok(ExecutionOutput {
        bytes: text.into_bytes(),
        media_type: TEXT_MEDIA_TYPE.to_owned(),
        executor_job_id: Some(response.id),
        external_provenance: None,
        content_contract: None,
    })
}

fn valid_response_id(value: &str) -> bool {
    (1..=160).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
}

fn error_response(status: u16, bytes: &[u8]) -> ResponsesAttemptFailure {
    let code = serde_json::from_slice::<ErrorEnvelope>(bytes)
        .ok()
        .map(|envelope| envelope.error.code)
        .filter(|code| valid_error_code(code))
        .unwrap_or_else(|| format!("infer_http_{status}"));
    let retryable = matches!(status, 429 | 500 | 502 | 503 | 504)
        || matches!(
            code.as_str(),
            "provider_unavailable" | "upstream_rate_limited"
        );
    ResponsesAttemptFailure {
        failure: failure(&code, retryable),
        transport: false,
    }
}

fn valid_error_code(code: &str) -> bool {
    (1..=96).contains(&code.len())
        && code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn failure(code: &str, retryable: bool) -> ExecutionFailure {
    ExecutionFailure::new(
        code,
        "Infer Runtime could not produce a text candidate",
        retryable,
    )
}

struct ResponsesAttemptFailure {
    failure: ExecutionFailure,
    transport: bool,
}

impl ResponsesAttemptFailure {
    fn new(code: &str, retryable: bool, transport: bool) -> Self {
        Self {
            failure: failure(code, retryable),
            transport,
        }
    }

    fn from_contract(error: &InferRuntimeClientError) -> Self {
        let transport = matches!(error, InferRuntimeClientError::Unavailable);
        let code = match error {
            InferRuntimeClientError::InvalidEndpoint => "infer_invalid_endpoint",
            InferRuntimeClientError::Unavailable => "infer_unavailable",
            InferRuntimeClientError::UnexpectedStatus { .. } => "infer_contract_http_error",
            InferRuntimeClientError::InvalidContract => "infer_invalid_contract",
            InferRuntimeClientError::IncompatibleContract { .. } => "infer_incompatible_contract",
        };
        Self::new(code, transport, transport)
    }

    fn from_job_provenance(error: JobProvenanceError) -> Self {
        Self::new(error.code(), false, false)
    }
}

#[cfg(test)]
mod tests;
