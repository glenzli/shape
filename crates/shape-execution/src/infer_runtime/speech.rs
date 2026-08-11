//! Authenticated local-only Infer Runtime speech synthesis executor.

use std::{collections::BTreeMap, io::Read, time::Duration};

use reqwest::{
    Url,
    blocking::Client,
    header::{CONTENT_TYPE, HeaderMap},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, SpeechSynthesisOperation, SpeechVoiceSelection,
};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    ExternalExecutionProvenance,
    audio::{MAX_AUDIO_OUTPUT_BYTES, parse_pcm_s16le_wav},
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient, InferRuntimeClientError,
    InferRuntimeContractRevision, InferRuntimeCredential, InferRuntimeEndpointResolver,
    ResolvedInferRuntimeEndpoint,
    job_provenance::{
        JobPolicyProfile, JobProvenanceError, bounded_text, parse_job_snapshot, valid_job_id,
    },
    should_retry_endpoint, validate_discovered_contract,
};

/// Shape-side creative capability implemented by Runtime `speech.synthesize`.
pub const AUDIO_SPEECH_SYNTHESIZE_CAPABILITY: &str = "audio.speech_synthesize";
/// Runtime-owned voice catalog revision required by the first executable Operator.
pub const INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION: &str = "infer.speech.voice-aliases@20260811.1";
/// Stable Runtime voice identity for ordinary synthetic Mandarin narration.
pub const INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1: &str = "speech.voice.zh.bright_female.v1";
/// Exact Runtime language value paired with the first stable voice alias.
pub const INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE: &str = "Chinese";

const SPEECH_PATH: &str = "v1/audio/speech";
const JOB_PATH: &str = "infer/v1/jobs/";
const SPEECH_ROUTE: &str = "/v1/audio/speech";
const SPEECH_INTENT: &str = "speech.synthesize";
const AUDIO_MEDIA_TYPE: &str = "audio/wav";
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_INSTRUCTION_BYTES: usize = 16 * 1024;
const MAX_JOB_BYTES: usize = 1024 * 1024;
const MAX_JOB_READ_BYTES: u64 = 1024 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const REQUEST_TIMEOUT: Duration = Duration::from_mins(5);

/// Shape's typed unary WAV adapter for Infer Runtime speech synthesis.
pub struct InferRuntimeSpeechExecutor {
    identity: ExecutorIdentity,
    resolver: InferRuntimeEndpointResolver,
    credential: InferRuntimeCredential,
}

impl std::fmt::Debug for InferRuntimeSpeechExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeSpeechExecutor")
            .field("identity", &self.identity)
            .field("resolver", &self.resolver)
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

impl InferRuntimeSpeechExecutor {
    /// Creates a live Consumer adapter whose endpoint is resolved per attempt.
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
                "shape.infer-runtime-speech-consumer",
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
                "shape.infer-runtime-speech-consumer",
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
        text: &str,
        operation: &SpeechSynthesisOperation,
    ) -> Result<ExecutionOutput, SpeechAttemptFailure> {
        let contract = InferRuntimeClient::new(&endpoint.origin)
            .and_then(|client| client.probe_contract_for_route("POST", SPEECH_ROUTE))
            .map_err(|error| SpeechAttemptFailure::from_contract(&error))?;
        let revision = validate_discovered_contract(endpoint, &contract)
            .map_err(|error| SpeechAttemptFailure::from_contract(&error))?;
        SpeechClient::new(&endpoint.origin)?.create_speech(
            &self.credential,
            text,
            operation,
            revision,
        )
    }
}

impl Executor for InferRuntimeSpeechExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == AUDIO_SPEECH_SYNTHESIZE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.output_media_type != AUDIO_MEDIA_TYPE
            || request.inputs.len() != 1
            || request.instruction.is_empty()
            || request.instruction.len() > MAX_INSTRUCTION_BYTES
        {
            return Err(failure("invalid_speech_request", false));
        }
        let input = &request.inputs[0];
        if !input.content().media_type.starts_with("text/") {
            return Err(failure("invalid_speech_source", false));
        }
        let text = input
            .bytes()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .filter(|text| !text.is_empty() && text.len() <= MAX_TEXT_BYTES)
            .ok_or_else(|| failure("invalid_speech_source", false))?;
        let operation: SpeechSynthesisOperation = serde_json::from_slice(&request.instruction)
            .map_err(|_| failure("invalid_speech_request", false))?;
        operation
            .validate()
            .map_err(|_| failure("invalid_speech_request", false))?;
        let SpeechVoiceSelection::Preset(voice) = &operation.voice else {
            return Err(failure("voice_reference_not_supported", false));
        };
        if !operation.synthetic_disclosure_required
            || voice.alias.as_str() != INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1
            || voice.catalog_revision != INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION
            || operation.language != INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE
        {
            return Err(failure("unsupported_speech_preset", false));
        }

        let endpoint = self
            .resolver
            .resolve()
            .map_err(|_| failure("infer_invalid_endpoint", false))?;
        match self.execute_resolved(&endpoint, text, &operation) {
            Ok(output) => Ok(output),
            Err(first) if first.transport => {
                let rediscovered = self
                    .resolver
                    .resolve_after_connection_failure(&endpoint)
                    .map_err(|_| first.failure.clone())?;
                if should_retry_endpoint(&endpoint, &rediscovered) {
                    self.execute_resolved(&rediscovered, text, &operation)
                        .map_err(|error| error.failure)
                } else {
                    Err(first.failure)
                }
            }
            Err(error) => Err(error.failure),
        }
    }
}

struct SpeechClient {
    origin: Url,
    speech_endpoint: Url,
    client: Client,
}

impl SpeechClient {
    fn new(origin: &str) -> Result<Self, SpeechAttemptFailure> {
        let origin = super::canonical_loopback_url(origin)
            .map_err(|error| SpeechAttemptFailure::from_contract(&error))?;
        let speech_endpoint = origin
            .join(SPEECH_PATH)
            .map_err(|_| SpeechAttemptFailure::new("infer_invalid_endpoint", false, false))?;
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .no_proxy()
            .user_agent("shape/0.1 infer-speech-consumer")
            .build()
            .map_err(|_| SpeechAttemptFailure::new("infer_client_invalid", false, false))?;
        Ok(Self {
            origin,
            speech_endpoint,
            client,
        })
    }

    fn create_speech(
        &self,
        credential: &InferRuntimeCredential,
        text: &str,
        operation: &SpeechSynthesisOperation,
        revision: InferRuntimeContractRevision,
    ) -> Result<ExecutionOutput, SpeechAttemptFailure> {
        let SpeechVoiceSelection::Preset(voice) = &operation.voice else {
            return Err(SpeechAttemptFailure::new(
                "voice_reference_not_supported",
                false,
                false,
            ));
        };
        let request = SpeechRequest::local_unary(
            text,
            voice.alias.as_str(),
            &operation.language,
            operation.speed_milli,
            revision,
        );
        let response = self
            .client
            .post(self.speech_endpoint.clone())
            .header(CONTENT_TYPE, "application/json")
            .bearer_auth(credential.expose())
            .json(&request)
            .send()
            .map_err(|_| SpeechAttemptFailure::new("infer_unavailable", true, true))?;
        let status = response.status();
        let headers = response.headers().clone();
        let mut bytes = Vec::new();
        response
            .take((MAX_AUDIO_OUTPUT_BYTES as u64) + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        if bytes.len() > MAX_AUDIO_OUTPUT_BYTES || content_type(&headers) != Some(AUDIO_MEDIA_TYPE)
        {
            return Err(SpeechAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        let job_id = required_header(&headers, "x-infer-job-id")?;
        if !valid_job_id(&job_id) {
            return Err(SpeechAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        if required_header(&headers, "x-infer-model")? != SPEECH_INTENT {
            return Err(SpeechAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        let contract = parse_pcm_s16le_wav(&bytes, AudioOriginDisclosure::SyntheticSpeech)
            .map_err(|_| SpeechAttemptFailure::new("invalid_audio_output", false, false))?;
        let provenance = self.job_provenance(credential, &job_id, revision)?;
        Ok(ExecutionOutput {
            bytes,
            media_type: AUDIO_MEDIA_TYPE.to_owned(),
            executor_job_id: Some(job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::AudioClip(contract)),
        })
    }

    fn job_provenance(
        &self,
        credential: &InferRuntimeCredential,
        job_id: &str,
        revision: InferRuntimeContractRevision,
    ) -> Result<ExternalExecutionProvenance, SpeechAttemptFailure> {
        let endpoint = self
            .origin
            .join(&format!("{JOB_PATH}{job_id}"))
            .map_err(|_| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
        let response = self
            .client
            .get(endpoint)
            .bearer_auth(credential.expose())
            .send()
            .map_err(|_| SpeechAttemptFailure::new("infer_unavailable", true, true))?;
        let status = response.status();
        let mut bytes = Vec::new();
        response
            .take(MAX_JOB_READ_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
        if !status.is_success() {
            return Err(error_response(status.as_u16(), &bytes));
        }
        if bytes.len() > MAX_JOB_BYTES {
            return Err(SpeechAttemptFailure::new(
                "infer_invalid_response",
                false,
                false,
            ));
        }
        parse_job_snapshot(
            revision,
            job_id,
            SPEECH_INTENT,
            JobPolicyProfile::ShapeLocalInteractive,
            &bytes,
        )
        .map_err(SpeechAttemptFailure::from_job_provenance)
    }
}

#[derive(Serialize)]
struct SpeechRequest<'a> {
    model: &'static str,
    input: &'a str,
    voice: &'a str,
    language: &'a str,
    speed: f64,
    response_format: &'static str,
    execution_mode: &'static str,
    metadata: BTreeMap<&'static str, &'static str>,
}

impl<'a> SpeechRequest<'a> {
    fn local_unary(
        input: &'a str,
        voice: &'a str,
        language: &'a str,
        speed_milli: u16,
        revision: InferRuntimeContractRevision,
    ) -> Self {
        let mut metadata = BTreeMap::from([
            ("infer.fallback", "none"),
            ("infer.max_cost_usd", "0"),
            ("infer.offline_required", "true"),
            ("infer.latency", "interactive"),
            ("infer.placement", "local_only"),
            ("infer.policy", "local-first"),
            ("infer.prefer", "local"),
            ("infer.priority", "interactive"),
        ]);
        metadata.insert(
            revision.capability_floor_metadata_key(),
            revision.capable_level(),
        );
        Self {
            model: SPEECH_INTENT,
            input,
            voice,
            language,
            speed: f64::from(speed_milli) / 1_000.0,
            response_format: "wav",
            execution_mode: "unary",
            metadata,
        }
    }
}

fn content_type(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String, SpeechAttemptFailure> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| bounded_text(value))
        .map(ToOwned::to_owned)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
}

fn error_response(status: u16, bytes: &[u8]) -> SpeechAttemptFailure {
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
    SpeechAttemptFailure {
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
        "Infer Runtime could not synthesize an audio candidate",
        retryable,
    )
}

struct SpeechAttemptFailure {
    failure: ExecutionFailure,
    transport: bool,
}

impl SpeechAttemptFailure {
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
