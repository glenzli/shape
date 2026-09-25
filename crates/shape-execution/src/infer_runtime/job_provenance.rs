//! Shape-policy validation of official SDK Job provenance.
//!
//! The SDK owns Core decoding and additive response compatibility. This owner
//! enforces only Shape's App identity, request narrowing, no-fallback policy,
//! and the successful physical Attempt copied into a payload-free receipt.

use infer_runtime_client::{JobSnapshot, NamedRouteDecision};
use serde_json::Value;

use crate::{
    ExternalAttemptProvenance, ExternalExecutionProvenance, ExternalNamedRouteProvenance,
    ExternalRoutingCandidate,
};

use super::{
    INFER_RUNTIME_AGENT_TASK_CAPABILITY, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_RUNTIME_RESPONSES_CAPABILITY, INFER_RUNTIME_SPEECH_CAPABILITY,
};

const MAX_PROVENANCE_TEXT_BYTES: usize = 256;
const MAX_ATTEMPTS: usize = 16;
const MAX_ROUTING_CANDIDATES: usize = 64;
const MAX_REASON_CODES: usize = 32;

pub(super) const TEXT_EDIT_DEPLOYMENT: &str = "ollama_qwen3_5_4b";
pub(super) const SPEECH_DEPLOYMENT: &str = "mlx_qwen3_tts_custom_voice_1_7b";
const AGENT_SOL_DEPLOYMENT: &str = "codex_agent_gpt_6_sol";
const AGENT_SOL_BUILD: &str = "codex_gpt_6_sol_agent";
const AGENT_LUNA_DEPLOYMENT: &str = "codex_agent_gpt_6_luna";
const AGENT_LUNA_BUILD: &str = "codex_gpt_6_luna_agent";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobPolicyProfile {
    LocalTextEdit,
    CloudTextEdit(&'static str),
    LocalSpeech,
    CloudImageInteractive(&'static str),
    AgentFileTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobProvenanceError {
    InvalidResponse,
    PolicyViolation,
}

impl JobProvenanceError {
    pub(super) const fn code(self) -> &'static str {
        match self {
            Self::InvalidResponse => "infer_invalid_response",
            Self::PolicyViolation => "infer_policy_violation",
        }
    }
}

pub(super) fn parse_job_snapshot(
    expected_job_id: &str,
    expected_intent: &str,
    profile: JobPolicyProfile,
    snapshot: JobSnapshot,
) -> Result<ExternalExecutionProvenance, JobProvenanceError> {
    let expected_capability = profile.capability_contract();
    let scalar_fields = [
        snapshot.id.as_str(),
        snapshot.app_id.as_str(),
        snapshot.intent.as_str(),
        snapshot.consumer_core_contract.as_str(),
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
        || snapshot.intent != expected_intent
        || snapshot.consumer_core_contract != INFER_RUNTIME_CONTRACT_VERSION
        || snapshot.capability_contract.as_deref() != Some(expected_capability)
        || snapshot.state != "succeeded"
        || snapshot.error.is_some()
        || scalar_fields.iter().any(|value| !bounded_text(value))
        || !valid_capability_level(&snapshot.capability_level)
        || !matches!(
            snapshot.evaluation_status.as_str(),
            "provisional" | "benchmarked"
        )
        || snapshot.attempts.is_empty()
        || snapshot.attempts.len() > MAX_ATTEMPTS
        || snapshot.routing.candidates.is_empty()
        || snapshot.routing.candidates.len() > MAX_ROUTING_CANDIDATES
    {
        return Err(JobProvenanceError::InvalidResponse);
    }

    let requested = validate_shape_policy(&snapshot, profile)?;
    let attempts = external_attempts(snapshot.attempts)?;
    let Some(final_attempt) = attempts.last() else {
        return Err(JobProvenanceError::InvalidResponse);
    };
    if final_attempt.outcome != "succeeded"
        || final_attempt.provider != snapshot.provider
        || final_attempt.deployment != snapshot.deployment
    {
        return Err(JobProvenanceError::InvalidResponse);
    }
    if attempts.iter().any(|attempt| attempt.trigger == "fallback") {
        return Err(JobProvenanceError::PolicyViolation);
    }

    let named_route = external_named_route(snapshot.routing.named_route.as_ref());
    let routing_candidates = external_routing_candidates(snapshot.routing.candidates)?;
    if !routing_candidates.iter().any(|candidate| {
        candidate.provider == snapshot.provider
            && candidate.deployment == snapshot.deployment
            && candidate.status == "eligible"
    }) {
        return Err(JobProvenanceError::InvalidResponse);
    }

    Ok(ExternalExecutionProvenance {
        speech_segments: Vec::new(),
        speech_script: None,
        contract_revision: snapshot.consumer_core_contract,
        capability_contract: snapshot.capability_contract,
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
        named_route,
        routing_candidates,
        attempts,
    })
}

fn external_named_route(
    route: Option<&NamedRouteDecision>,
) -> Option<ExternalNamedRouteProvenance> {
    route.map(|route| ExternalNamedRouteProvenance {
        kind: route.kind.clone(),
        ordered_ids: route.ordered_ids.clone(),
    })
}

impl JobPolicyProfile {
    const fn capability_contract(self) -> &'static str {
        match self {
            Self::LocalTextEdit | Self::CloudTextEdit(_) | Self::CloudImageInteractive(_) => {
                INFER_RUNTIME_RESPONSES_CAPABILITY
            }
            Self::LocalSpeech => INFER_RUNTIME_SPEECH_CAPABILITY,
            Self::AgentFileTask => INFER_RUNTIME_AGENT_TASK_CAPABILITY,
        }
    }

    fn named_deployment(self) -> &'static str {
        match self {
            Self::LocalTextEdit => TEXT_EDIT_DEPLOYMENT,
            Self::CloudTextEdit(deployment) | Self::CloudImageInteractive(deployment) => deployment,
            Self::LocalSpeech => SPEECH_DEPLOYMENT,
            Self::AgentFileTask => unreachable!("Agent tasks do not request a named route"),
        }
    }

    const fn capability_floor(self) -> &'static str {
        match self {
            Self::LocalTextEdit | Self::CloudTextEdit(_) | Self::AgentFileTask => "foundational",
            Self::LocalSpeech | Self::CloudImageInteractive(_) => "capable",
        }
    }
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
    snapshot: &JobSnapshot,
    profile: JobPolicyProfile,
) -> Result<ValidatedShapePolicy, JobProvenanceError> {
    if profile == JobPolicyProfile::AgentFileTask {
        return validate_agent_file_policy(snapshot);
    }
    let constraints = snapshot
        .constraints
        .as_object()
        .ok_or(JobProvenanceError::InvalidResponse)?;
    let policy = constraint_string(constraints, "policy")?;
    let priority = constraint_string(constraints, "priority")?;
    let provider_access_class = optional_constraint_string(constraints, "provider_access_class")?;
    let placement = constraint_string(constraints, "placement")?;
    let preference = constraint_string(constraints, "prefer")?;
    let offline_required = constraints
        .get("offline_required")
        .and_then(Value::as_bool)
        .ok_or(JobProvenanceError::InvalidResponse)?;
    let fallback = constraint_string(constraints, "fallback")?;
    let capability_floor = constraint_string(constraints, "capability_floor")?;
    let latency = optional_constraint_string(constraints, "latency")?;
    let deadline_ms = optional_constraint_u64(constraints, "deadline_ms")?;
    let max_cost_usd = constraints
        .get("max_cost_usd")
        .and_then(Value::as_f64)
        .ok_or(JobProvenanceError::InvalidResponse)?;

    let valid_profile = match profile {
        JobPolicyProfile::LocalTextEdit | JobPolicyProfile::LocalSpeech => {
            policy == "local-first"
                && priority == "interactive"
                && provider_access_class
                    .as_deref()
                    .is_none_or(|value| value == "standard")
                && placement == "local_only"
                && preference == "local"
                && offline_required
                && latency.as_deref() == Some("interactive")
                && snapshot.placement == "local"
                && snapshot.policy == "local-first"
                && snapshot.priority == "interactive"
        }
        JobPolicyProfile::CloudTextEdit(_) | JobPolicyProfile::CloudImageInteractive(_) => {
            policy == "balanced"
                && priority == "interactive"
                && provider_access_class.as_deref() == Some("subscription")
                && placement == "cloud_only"
                && preference == "cloud"
                && !offline_required
                && latency.is_none()
                && snapshot.placement == "cloud"
                && snapshot.policy == "balanced"
                && snapshot.priority == "interactive"
        }
        JobPolicyProfile::AgentFileTask => unreachable!("Agent policy returned above"),
    };
    if !valid_profile
        || fallback != "none"
        || max_cost_usd != 0.0
        || capability_floor != profile.capability_floor()
        || deadline_ms.is_some()
        || capability_floor != snapshot.routing.capability_floor
    {
        return Err(JobProvenanceError::PolicyViolation);
    }

    validate_named_route(
        constraints.get("named_route"),
        snapshot.routing.named_route.as_ref(),
        Some(profile.named_deployment()),
        &snapshot.deployment,
    )?;

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

fn validate_agent_file_policy(
    snapshot: &JobSnapshot,
) -> Result<ValidatedShapePolicy, JobProvenanceError> {
    let constraints = snapshot
        .constraints
        .as_object()
        .ok_or(JobProvenanceError::InvalidResponse)?;
    if constraints.get("deadline_ms").and_then(Value::as_u64) != Some(300_000)
        || constraints
            .iter()
            .any(|(key, value)| key != "deadline_ms" && !value.is_null())
        || snapshot.policy != "balanced"
        || snapshot.priority != "normal"
        || snapshot.placement != "cloud"
        || snapshot.provider != "codex-agent"
        || !approved_agent_deployment_build(&snapshot.deployment, &snapshot.model_build)
        || snapshot.routing.capability_floor != "foundational"
        || snapshot.routing.named_route.is_some()
    {
        return Err(JobProvenanceError::PolicyViolation);
    }
    Ok(ValidatedShapePolicy {
        policy: "service_default".into(),
        priority: "service_default".into(),
        provider_access_class: None,
        placement: "service_default".into(),
        preference: "service_default".into(),
        offline_required: false,
        latency: None,
        fallback: "service_default".into(),
        deadline_ms: Some(300_000),
    })
}

pub(super) fn approved_agent_deployment_build(deployment: &str, model_build: &str) -> bool {
    matches!(
        (deployment, model_build),
        (AGENT_SOL_DEPLOYMENT, AGENT_SOL_BUILD) | (AGENT_LUNA_DEPLOYMENT, AGENT_LUNA_BUILD)
    )
}

fn validate_named_route(
    constraint: Option<&Value>,
    routing: Option<&NamedRouteDecision>,
    expected_deployment: Option<&str>,
    selected_deployment: &str,
) -> Result<(), JobProvenanceError> {
    let constraint = constraint.filter(|value| !value.is_null());
    match expected_deployment {
        Some(expected) => {
            let constraint = constraint
                .and_then(Value::as_object)
                .ok_or(JobProvenanceError::PolicyViolation)?;
            let constraint_kind = constraint.get("kind").and_then(Value::as_str);
            let constraint_ids = constraint.get("ordered_ids").and_then(Value::as_array);
            let expected_ids = [expected];
            let constraint_matches = constraint_kind == Some("deployment")
                && constraint_ids
                    .is_some_and(|ids| ids.len() == 1 && ids[0].as_str() == Some(expected));
            let routing_matches = routing.is_some_and(|route| {
                route.kind == "deployment"
                    && route
                        .ordered_ids
                        .iter()
                        .map(String::as_str)
                        .eq(expected_ids)
            });
            if !constraint_matches || !routing_matches || selected_deployment != expected {
                return Err(JobProvenanceError::PolicyViolation);
            }
        }
        None if constraint.is_some() || routing.is_some() => {
            return Err(JobProvenanceError::PolicyViolation);
        }
        None => {}
    }
    Ok(())
}

fn external_attempts(
    attempts: Vec<infer_runtime_client::AttemptSnapshot>,
) -> Result<Vec<ExternalAttemptProvenance>, JobProvenanceError> {
    let mut previous_number = 0_u32;
    attempts
        .into_iter()
        .map(|attempt| {
            let number =
                u32::try_from(attempt.number).map_err(|_| JobProvenanceError::InvalidResponse)?;
            if number == 0
                || number <= previous_number
                || !bounded_text(&attempt.provider)
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
                return Err(JobProvenanceError::InvalidResponse);
            }
            previous_number = number;
            Ok(ExternalAttemptProvenance {
                number,
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
    candidates: Vec<infer_runtime_client::CandidateDecision>,
) -> Result<Vec<ExternalRoutingCandidate>, JobProvenanceError> {
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
                return Err(JobProvenanceError::InvalidResponse);
            }
            let rank = candidate
                .rank
                .map(u32::try_from)
                .transpose()
                .map_err(|_| JobProvenanceError::InvalidResponse)?;
            Ok(ExternalRoutingCandidate {
                provider: candidate.provider,
                deployment: candidate.deployment,
                status: candidate.status,
                rank,
                reason_codes: candidate.reason_codes,
            })
        })
        .collect()
}

fn constraint_string(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, JobProvenanceError> {
    constraints
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| bounded_text(value))
        .map(ToOwned::to_owned)
        .ok_or(JobProvenanceError::InvalidResponse)
}

fn optional_constraint_string(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, JobProvenanceError> {
    match constraints.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if bounded_text(value) => Ok(Some(value.clone())),
        Some(_) => Err(JobProvenanceError::InvalidResponse),
    }
}

fn optional_constraint_u64(
    constraints: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<u64>, JobProvenanceError> {
    match constraints.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or(JobProvenanceError::InvalidResponse),
    }
}

fn valid_capability_level(value: &str) -> bool {
    matches!(
        value,
        "foundational" | "capable" | "advanced" | "expert" | "exceptional"
    )
}

pub(super) fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROVENANCE_TEXT_BYTES && value.is_ascii()
}

pub(super) fn valid_job_id(value: &str) -> bool {
    (1..=160).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
pub(crate) mod tests;
