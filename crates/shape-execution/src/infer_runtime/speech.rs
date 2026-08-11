//! Authenticated local-only Infer Runtime speech synthesis executor.

use std::{collections::BTreeMap, io::Read, time::Duration};

use reqwest::{
    Url,
    blocking::Client,
    header::{CONTENT_TYPE, HeaderMap},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, SpeechSynthesisOperation, SpeechVoiceSelection,
};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    ExternalAttemptProvenance, ExternalExecutionProvenance, ExternalRoutingCandidate,
    audio::{MAX_AUDIO_OUTPUT_BYTES, parse_pcm_s16le_wav},
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient, InferRuntimeClientError,
    InferRuntimeContractRevision, InferRuntimeCredential, InferRuntimeEndpointResolver,
    ResolvedInferRuntimeEndpoint, should_retry_endpoint, validate_discovered_contract,
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
const MAX_PROVENANCE_TEXT_BYTES: usize = 256;
const MAX_ATTEMPTS: usize = 16;
const MAX_ROUTING_CANDIDATES: usize = 64;
const MAX_REASON_CODES: usize = 32;
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
        parse_job_snapshot(revision, job_id, &bytes)
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

#[derive(Deserialize)]
struct JobSnapshot {
    id: String,
    app_id: String,
    intent: String,
    provider: String,
    deployment: String,
    model_profile: String,
    model_build: String,
    physical_model: String,
    placement: String,
    #[serde(alias = "quality_grade")]
    capability_level: String,
    #[serde(alias = "rating_status")]
    evaluation_status: String,
    resource_class: String,
    state: String,
    policy: String,
    priority: String,
    constraints: Value,
    routing: RoutingDecision,
    attempts: Vec<Attempt>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RoutingDecision {
    #[serde(alias = "quality_floor")]
    capability_floor: String,
    candidates: Vec<RoutingCandidate>,
}

#[derive(Deserialize)]
struct RoutingCandidate {
    deployment: String,
    provider: String,
    status: String,
    rank: Option<u32>,
    #[serde(default)]
    reason_codes: Vec<String>,
}

#[derive(Deserialize)]
struct Attempt {
    number: u32,
    provider: String,
    deployment: String,
    outcome: String,
    trigger: String,
    error_kind: Option<String>,
}

fn parse_job_snapshot(
    revision: InferRuntimeContractRevision,
    expected_job_id: &str,
    bytes: &[u8],
) -> Result<ExternalExecutionProvenance, SpeechAttemptFailure> {
    let raw: Value = serde_json::from_slice(bytes)
        .map_err(|_| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    validate_job_vocabulary(revision, &raw)?;
    let snapshot: JobSnapshot = serde_json::from_value(raw)
        .map_err(|_| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let scalar_fields = [
        snapshot.id.as_str(),
        snapshot.app_id.as_str(),
        snapshot.intent.as_str(),
        snapshot.provider.as_str(),
        snapshot.deployment.as_str(),
        snapshot.model_profile.as_str(),
        snapshot.model_build.as_str(),
        snapshot.physical_model.as_str(),
        snapshot.placement.as_str(),
        snapshot.capability_level.as_str(),
        snapshot.evaluation_status.as_str(),
        snapshot.resource_class.as_str(),
        snapshot.policy.as_str(),
        snapshot.priority.as_str(),
        snapshot.routing.capability_floor.as_str(),
    ];
    if snapshot.id != expected_job_id
        || snapshot.app_id != "shape"
        || snapshot.intent != SPEECH_INTENT
        || snapshot.state != "succeeded"
        || snapshot.error.is_some()
        || scalar_fields.iter().any(|value| !bounded_text(value))
        || !revision.valid_capability_level(&snapshot.capability_level)
        || !matches!(
            snapshot.evaluation_status.as_str(),
            "provisional" | "benchmarked"
        )
        || snapshot.attempts.is_empty()
        || snapshot.attempts.len() > MAX_ATTEMPTS
        || snapshot.routing.candidates.len() > MAX_ROUTING_CANDIDATES
    {
        return Err(SpeechAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ));
    }
    let requested = validate_shape_policy(revision, &snapshot)?;

    let attempts = external_attempts(snapshot.attempts)?;
    if attempts.last().map(|attempt| attempt.outcome.as_str()) != Some("succeeded") {
        return Err(SpeechAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ));
    }
    if attempts.iter().any(|attempt| attempt.trigger == "fallback") {
        return Err(SpeechAttemptFailure::new(
            "infer_policy_violation",
            false,
            false,
        ));
    }

    let routing_candidates = external_routing_candidates(snapshot.routing.candidates)?;

    Ok(ExternalExecutionProvenance {
        contract_revision: revision.as_str().to_owned(),
        app_id: snapshot.app_id,
        intent: snapshot.intent,
        provider: snapshot.provider,
        deployment: snapshot.deployment,
        model_profile: snapshot.model_profile,
        model_build: snapshot.model_build,
        physical_model: snapshot.physical_model,
        placement: snapshot.placement,
        capability_level: snapshot.capability_level,
        evaluation_status: snapshot.evaluation_status,
        resource_class: snapshot.resource_class,
        policy: snapshot.policy,
        priority: snapshot.priority,
        requested_policy: requested.policy,
        requested_priority: requested.priority,
        requested_provider_access_class: requested.provider_access_class,
        requested_placement: requested.placement,
        requested_preference: requested.preference,
        offline_required: requested.offline_required,
        requested_latency: requested.latency,
        fallback: requested.fallback,
        requested_deadline_ms: requested.deadline_ms,
        max_cost_microusd: 0,
        capability_floor: snapshot.routing.capability_floor,
        routing_candidates,
        attempts,
    })
}

struct ValidatedShapePolicy {
    policy: String,
    priority: String,
    provider_access_class: Option<String>,
    placement: String,
    preference: String,
    offline_required: bool,
    latency: Option<String>,
    fallback: String,
    deadline_ms: Option<u64>,
}

fn validate_shape_policy(
    revision: InferRuntimeContractRevision,
    snapshot: &JobSnapshot,
) -> Result<ValidatedShapePolicy, SpeechAttemptFailure> {
    let constraints = snapshot
        .constraints
        .as_object()
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let policy = constraint_string(constraints, "policy")?;
    let priority = constraint_string(constraints, "priority")?;
    let provider_access_class = optional_constraint_string(constraints, "provider_access_class")?;
    let placement = constraint_string(constraints, "placement")?;
    let preference = constraint_string(constraints, "prefer")?;
    let offline_required = constraints
        .get("offline_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let fallback = constraint_string(constraints, "fallback")?;
    let capability_floor = constraint_string(constraints, revision.job_capability_floor_key())?;
    let latency = optional_constraint_string(constraints, "latency")?;
    let deadline_ms = optional_constraint_u64(constraints, "deadline_ms")?;
    let max_cost_usd = constraints
        .get("max_cost_usd")
        .and_then(Value::as_f64)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    if policy != "local-first"
        || priority != "interactive"
        || placement != "local_only"
        || preference != "local"
        || !offline_required
        || fallback != "none"
        || max_cost_usd != 0.0
        || capability_floor != revision.capable_level()
        || provider_access_class.is_some()
        || latency.as_deref() != Some("interactive")
        || deadline_ms.is_some()
        || capability_floor != snapshot.routing.capability_floor
        || snapshot.placement != "local"
        || snapshot.policy != "local-first"
        || snapshot.priority != "interactive"
    {
        return Err(SpeechAttemptFailure::new(
            "infer_policy_violation",
            false,
            false,
        ));
    }
    Ok(ValidatedShapePolicy {
        policy,
        priority,
        provider_access_class,
        placement,
        preference,
        offline_required,
        latency,
        fallback,
        deadline_ms,
    })
}

fn validate_job_vocabulary(
    revision: InferRuntimeContractRevision,
    snapshot: &Value,
) -> Result<(), SpeechAttemptFailure> {
    let object = snapshot
        .as_object()
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let constraints = object
        .get("constraints")
        .and_then(Value::as_object)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let routing = object
        .get("routing")
        .and_then(Value::as_object)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))?;
    let valid = match revision {
        InferRuntimeContractRevision::Candidate2 => {
            object.contains_key("quality_grade")
                && object.contains_key("rating_status")
                && !object.contains_key("capability_level")
                && !object.contains_key("evaluation_status")
                && constraints.contains_key("quality_floor")
                && !constraints.contains_key("capability_floor")
                && routing.contains_key("quality_floor")
                && !routing.contains_key("capability_floor")
        }
        InferRuntimeContractRevision::Candidate3 => {
            object.contains_key("capability_level")
                && object.contains_key("evaluation_status")
                && !object.contains_key("quality_grade")
                && !object.contains_key("rating_status")
                && constraints.contains_key("capability_floor")
                && !constraints.contains_key("quality_floor")
                && routing.contains_key("capability_floor")
                && !routing.contains_key("quality_floor")
        }
    };
    if valid {
        Ok(())
    } else {
        Err(SpeechAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        ))
    }
}

fn external_attempts(
    attempts: Vec<Attempt>,
) -> Result<Vec<ExternalAttemptProvenance>, SpeechAttemptFailure> {
    attempts
        .into_iter()
        .map(|attempt| {
            if !bounded_text(&attempt.provider)
                || !bounded_text(&attempt.deployment)
                || !matches!(
                    attempt.outcome.as_str(),
                    "running" | "succeeded" | "failed" | "interrupted"
                )
                || !matches!(
                    attempt.trigger.as_str(),
                    "initial" | "retry" | "fallback" | "recovery"
                )
                || attempt
                    .error_kind
                    .as_deref()
                    .is_some_and(|value| !bounded_text(value))
            {
                return Err(SpeechAttemptFailure::new(
                    "infer_invalid_response",
                    false,
                    false,
                ));
            }
            Ok(ExternalAttemptProvenance {
                number: attempt.number,
                provider: attempt.provider,
                deployment: attempt.deployment,
                outcome: attempt.outcome,
                trigger: attempt.trigger,
                error_kind: attempt.error_kind,
            })
        })
        .collect()
}

fn external_routing_candidates(
    candidates: Vec<RoutingCandidate>,
) -> Result<Vec<ExternalRoutingCandidate>, SpeechAttemptFailure> {
    candidates
        .into_iter()
        .map(|candidate| {
            if !bounded_text(&candidate.provider)
                || !bounded_text(&candidate.deployment)
                || !matches!(
                    candidate.status.as_str(),
                    "eligible" | "fallback_eligible" | "rejected"
                )
                || candidate.reason_codes.len() > MAX_REASON_CODES
                || candidate
                    .reason_codes
                    .iter()
                    .any(|code| !bounded_text(code))
            {
                return Err(SpeechAttemptFailure::new(
                    "infer_invalid_response",
                    false,
                    false,
                ));
            }
            Ok(ExternalRoutingCandidate {
                provider: candidate.provider,
                deployment: candidate.deployment,
                status: candidate.status,
                rank: candidate.rank,
                reason_codes: candidate.reason_codes,
            })
        })
        .collect()
}

fn constraint_string(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, SpeechAttemptFailure> {
    constraints
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| bounded_text(value))
        .map(ToOwned::to_owned)
        .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false))
}

fn optional_constraint_string(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, SpeechAttemptFailure> {
    match constraints.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if bounded_text(value) => Ok(Some(value.clone())),
        Some(_) => Err(SpeechAttemptFailure::new(
            "infer_invalid_response",
            false,
            false,
        )),
    }
}

fn optional_constraint_u64(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<u64>, SpeechAttemptFailure> {
    match constraints.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| SpeechAttemptFailure::new("infer_invalid_response", false, false)),
    }
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROVENANCE_TEXT_BYTES && value.is_ascii()
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

fn valid_job_id(value: &str) -> bool {
    (1..=160).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
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
}

#[cfg(test)]
mod tests;
