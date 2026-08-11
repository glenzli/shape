//! Authenticated candidate.3 text-to-image Responses executor.
//!
//! The Runtime contract is deliberately narrower than Shape's provider-neutral
//! AI Image domain. This first adapter accepts only source-less `image.generate`,
//! one unary Candidate, and the exact single-field `image_generation` tool.

use std::{collections::BTreeMap, io::Read, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{
    Url,
    blocking::Client,
    header::{CONTENT_TYPE, HeaderMap},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use shape_domain::{AiImageGenerateParameters, ArtifactContentContract, ImageRasterContract};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    raster::normalize_generated_png,
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient, InferRuntimeClientError,
    InferRuntimeContractRevision, InferRuntimeCredential, InferRuntimeEndpointResolver,
    ResolvedInferRuntimeEndpoint,
    job_provenance::{JobPolicyProfile, JobProvenanceError, parse_job_snapshot, valid_job_id},
    should_retry_endpoint, validate_discovered_contract,
};

/// Shape-side logical capability implemented by Runtime `image.generate`.
pub const IMAGE_GENERATE_CAPABILITY: &str = "image.generate";

const RESPONSES_PATH: &str = "v1/responses";
const RESPONSES_ROUTE: &str = "/v1/responses";
const JOB_PATH: &str = "infer/v1/jobs/";
const IMAGE_INTENT: &str = "image.generate";
const IMAGE_MEDIA_TYPE: &str = "image/png";
const MAX_INSTRUCTION_BYTES: usize = 32 * 1024;
const MAX_GENERATED_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MAX_GENERATED_IMAGE_DIMENSION: u32 = 4096;
const MAX_GENERATED_IMAGE_PIXELS: u64 = 16_777_216;
const MAX_BASE64_CHARS: usize = MAX_GENERATED_IMAGE_BYTES.div_ceil(3) * 4;
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_RESPONSE_READ_BYTES: u64 = 32 * 1024 * 1024;
const MAX_JOB_BYTES: usize = 1024 * 1024;
const MAX_JOB_READ_BYTES: u64 = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const REQUEST_TIMEOUT: Duration = Duration::from_mins(10);

/// Exact candidate.3 Consumer for one source-less generated raster.
pub struct InferRuntimeImageGenerationExecutor {
    identity: ExecutorIdentity,
    resolver: InferRuntimeEndpointResolver,
    credential: InferRuntimeCredential,
}

impl std::fmt::Debug for InferRuntimeImageGenerationExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeImageGenerationExecutor")
            .field("identity", &self.identity)
            .field("resolver", &self.resolver)
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

impl InferRuntimeImageGenerationExecutor {
    /// Creates a live Consumer whose endpoint is resolved for each execution.
    ///
    /// # Errors
    ///
    /// Returns an error only if the built-in executor identity is invalid.
    pub fn new(
        explicit_override: &str,
        credential: InferRuntimeCredential,
    ) -> Result<Self, crate::ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-image-generation-consumer",
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
                "shape.infer-runtime-image-generation-consumer",
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
        parameters: &AiImageGenerateParameters,
    ) -> Result<ExecutionOutput, ImageAttemptFailure> {
        let contract = InferRuntimeClient::new(&endpoint.origin)
            .and_then(|client| client.probe_contract_for_route("POST", RESPONSES_ROUTE))
            .map_err(|error| ImageAttemptFailure::from_contract(&error))?;
        let revision = validate_discovered_contract(endpoint, &contract)
            .map_err(|error| ImageAttemptFailure::from_contract(&error))?;
        if revision != InferRuntimeContractRevision::Candidate3 {
            return Err(ImageAttemptFailure::new(
                "infer_incompatible_contract",
                false,
                false,
            ));
        }
        ImageGenerationClient::new(&endpoint.origin)?.create_image(
            &self.credential,
            parameters,
            revision,
        )
    }
}

impl Executor for InferRuntimeImageGenerationExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == IMAGE_GENERATE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != IMAGE_GENERATE_CAPABILITY
            || request.output_media_type != IMAGE_MEDIA_TYPE
            || !request.inputs.is_empty()
            || request.instruction.is_empty()
            || request.instruction.len() > MAX_INSTRUCTION_BYTES
        {
            return Err(failure("invalid_image_generation_request", false));
        }
        let parameters: AiImageGenerateParameters = serde_json::from_slice(&request.instruction)
            .map_err(|_| failure("invalid_image_generation_request", false))?;
        if parameters.candidate_count() != 1 {
            return Err(failure("unsupported_image_candidate_count", false));
        }

        let endpoint = self
            .resolver
            .resolve()
            .map_err(|_| failure("infer_invalid_endpoint", false))?;
        match self.execute_resolved(&endpoint, &parameters) {
            Ok(output) => Ok(output),
            Err(first) if first.transport => {
                let rediscovered = self
                    .resolver
                    .resolve_after_connection_failure(&endpoint)
                    .map_err(|_| first.failure.clone())?;
                if should_retry_endpoint(&endpoint, &rediscovered) {
                    self.execute_resolved(&rediscovered, &parameters)
                        .map_err(|error| error.failure)
                } else {
                    Err(first.failure)
                }
            }
            Err(error) => Err(error.failure),
        }
    }
}

struct ImageGenerationClient {
    origin: Url,
    responses_endpoint: Url,
    client: Client,
}

impl ImageGenerationClient {
    fn new(origin: &str) -> Result<Self, ImageAttemptFailure> {
        let origin = super::canonical_loopback_url(origin)
            .map_err(|error| ImageAttemptFailure::from_contract(&error))?;
        let responses_endpoint = origin
            .join(RESPONSES_PATH)
            .map_err(|_| ImageAttemptFailure::new("infer_invalid_endpoint", false, false))?;
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .no_proxy()
            .user_agent("shape/0.1 infer-image-generation-consumer")
            .build()
            .map_err(|_| ImageAttemptFailure::new("infer_client_invalid", false, false))?;
        Ok(Self {
            origin,
            responses_endpoint,
            client,
        })
    }

    fn create_image(
        &self,
        credential: &InferRuntimeCredential,
        parameters: &AiImageGenerateParameters,
        revision: InferRuntimeContractRevision,
    ) -> Result<ExecutionOutput, ImageAttemptFailure> {
        let request = ImageGenerationRequest::candidate_three(parameters.instruction());
        let response = self
            .client
            .post(self.responses_endpoint.clone())
            .header(CONTENT_TYPE, "application/json")
            .bearer_auth(credential.expose())
            .json(&request)
            .send()
            .map_err(|_| ImageAttemptFailure::new("infer_unavailable", true, true))?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ImageAttemptFailure::new("infer_invalid_response", false, false))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(ImageAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        if content_type(&headers) != Some("application/json") {
            return Err(ImageAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        let parsed = parse_image_response(&bytes, parameters)?;
        let provenance = self.job_provenance(credential, &parsed.job_id, revision)?;
        Ok(ExecutionOutput {
            bytes: parsed.png,
            media_type: IMAGE_MEDIA_TYPE.to_owned(),
            executor_job_id: Some(parsed.job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::ImageRaster(parsed.contract)),
        })
    }

    fn job_provenance(
        &self,
        credential: &InferRuntimeCredential,
        job_id: &str,
        revision: InferRuntimeContractRevision,
    ) -> Result<crate::ExternalExecutionProvenance, ImageAttemptFailure> {
        let endpoint = self
            .origin
            .join(&format!("{JOB_PATH}{job_id}"))
            .map_err(|_| ImageAttemptFailure::new("infer_invalid_response", false, false))?;
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(credential.expose())
            .send()
            .map_err(|_| ImageAttemptFailure::new("infer_unavailable", true, false))?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut bytes = Vec::new();
        response
            .take(MAX_JOB_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ImageAttemptFailure::new("infer_invalid_response", false, false))?;
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        if bytes.len() > MAX_JOB_BYTES || content_type(&headers) != Some("application/json") {
            return Err(ImageAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        parse_job_snapshot(
            revision,
            job_id,
            IMAGE_INTENT,
            JobPolicyProfile::ShapeCloudImageInteractive,
            &bytes,
        )
        .map_err(ImageAttemptFailure::from_job_provenance)
    }
}

#[derive(Serialize)]
struct ImageGenerationRequest<'a> {
    model: &'static str,
    input: &'a str,
    tools: [ImageGenerationTool; 1],
    stream: bool,
    background: bool,
    metadata: BTreeMap<&'static str, &'static str>,
}

impl<'a> ImageGenerationRequest<'a> {
    fn candidate_three(input: &'a str) -> Self {
        Self {
            model: IMAGE_INTENT,
            input,
            tools: [ImageGenerationTool {
                kind: "image_generation",
            }],
            stream: false,
            background: false,
            metadata: BTreeMap::from([
                ("infer.capability_floor", "capable"),
                ("infer.fallback", "none"),
                ("infer.max_cost_usd", "0"),
                ("infer.placement", "cloud_only"),
                ("infer.policy", "balanced"),
                ("infer.prefer", "cloud"),
                ("infer.priority", "interactive"),
                ("infer.provider_access_class", "subscription"),
            ]),
        }
    }
}

#[derive(Serialize)]
struct ImageGenerationTool {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ResponseEnvelope {
    id: String,
    object: String,
    created_at: i64,
    model: String,
    status: String,
    output: Vec<ImageGenerationCall>,
}

#[derive(Deserialize)]
struct ImageGenerationCall {
    #[serde(rename = "type")]
    kind: String,
    status: String,
    result: String,
}

#[derive(Debug)]
struct ParsedImageResponse {
    job_id: String,
    png: Vec<u8>,
    contract: ImageRasterContract,
}

fn parse_image_response(
    bytes: &[u8],
    parameters: &AiImageGenerateParameters,
) -> Result<ParsedImageResponse, ImageAttemptFailure> {
    let response: ResponseEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| ImageAttemptFailure::new("infer_invalid_response", false, false))?;
    let [image] = response.output.as_slice() else {
        return Err(ImageAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ));
    };
    if response.object != "response"
        || response.model != IMAGE_INTENT
        || response.status != "completed"
        || response.created_at < 0
        || !valid_response_id(&response.id)
        || image.kind != "image_generation_call"
        || image.status != "completed"
        || image.result.is_empty()
        || image.result.len() > MAX_BASE64_CHARS
    {
        return Err(ImageAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ));
    }
    let decoded = STANDARD
        .decode(&image.result)
        .map_err(|_| ImageAttemptFailure::new("invalid_generated_image", false, false))?;
    if decoded.is_empty()
        || decoded.len() > MAX_GENERATED_IMAGE_BYTES
        || STANDARD.encode(&decoded) != image.result
    {
        return Err(ImageAttemptFailure::new(
            "invalid_generated_image",
            false,
            false,
        ));
    }
    let (png, contract) = normalize_generated_png(
        &decoded,
        MAX_GENERATED_IMAGE_BYTES,
        MAX_GENERATED_IMAGE_DIMENSION,
        MAX_GENERATED_IMAGE_PIXELS,
    )
    .map_err(|_| ImageAttemptFailure::new("invalid_generated_image", false, false))?;
    let expected = parameters.output();
    if contract.width != expected.width() || contract.height != expected.height() {
        return Err(ImageAttemptFailure::new(
            "generated_image_geometry_mismatch",
            false,
            false,
        ));
    }
    Ok(ParsedImageResponse {
        job_id: response.id,
        png,
        contract,
    })
}

fn valid_response_id(value: &str) -> bool {
    value.starts_with("resp_") && valid_job_id(value)
}

fn content_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
}

fn error_response(status: u16, bytes: &[u8]) -> ImageAttemptFailure {
    let parsed_code = serde_json::from_slice::<ErrorEnvelope>(bytes)
        .ok()
        .map(|envelope| envelope.error.code)
        .filter(|code| valid_error_code(code));
    let code = parsed_code
        .as_deref()
        .filter(|code| known_error(status, code))
        .map_or_else(|| format!("infer_http_{status}"), ToOwned::to_owned);
    let retryable = matches!(
        (status, code.as_str()),
        (
            429,
            "queue_full" | "app_queue_full" | "upstream_rate_limited"
        ) | (503, "provider_unavailable" | "upstream_unavailable")
            | (504, "deadline_exceeded" | "upstream_timeout")
    );
    ImageAttemptFailure {
        failure: failure(&code, retryable),
        transport: false,
    }
}

fn known_error(status: u16, code: &str) -> bool {
    matches!(
        (status, code),
        (400, "invalid_request_error")
            | (401, "invalid_api_key")
            | (403, "intent_forbidden" | "policy_violation")
            | (409, "no_candidate" | "cancelled")
            | (
                429,
                "queue_full" | "app_queue_full" | "quota_exceeded" | "upstream_rate_limited"
            )
            | (502, "upstream_authentication" | "upstream_protocol")
            | (503, "provider_unavailable" | "upstream_unavailable")
            | (504, "deadline_exceeded" | "upstream_timeout")
    )
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
        "Infer Runtime could not produce an image candidate",
        retryable,
    )
}

#[derive(Debug)]
struct ImageAttemptFailure {
    failure: ExecutionFailure,
    transport: bool,
}

impl ImageAttemptFailure {
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
