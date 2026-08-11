//! Bounded decoding of authenticated Infer Runtime Job provenance.
//!
//! This owner validates the shared candidate.2/candidate.3 Job vocabulary and
//! the exact Shape request policy that produced a result. Media adapters own
//! transport and payload handling; this module never sees prompts, media, or
//! credentials.

use serde::Deserialize;
use serde_json::Value;

use crate::{ExternalAttemptProvenance, ExternalExecutionProvenance, ExternalRoutingCandidate};

use super::InferRuntimeContractRevision;

const MAX_PROVENANCE_TEXT_BYTES: usize = 256;
const MAX_ATTEMPTS: usize = 16;
const MAX_ROUTING_CANDIDATES: usize = 64;
const MAX_REASON_CODES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum JobPolicyProfile {
    ShapeLocalInteractive,
    ShapeCloudImageInteractive,
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

pub(super) fn parse_job_snapshot(
    revision: InferRuntimeContractRevision,
    expected_job_id: &str,
    expected_intent: &str,
    profile: JobPolicyProfile,
    bytes: &[u8],
) -> Result<ExternalExecutionProvenance, JobProvenanceError> {
    let raw: Value =
        serde_json::from_slice(bytes).map_err(|_| JobProvenanceError::InvalidResponse)?;
    validate_job_vocabulary(revision, &raw)?;
    let snapshot: JobSnapshot =
        serde_json::from_value(raw).map_err(|_| JobProvenanceError::InvalidResponse)?;
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
        || snapshot.intent != expected_intent
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
        || snapshot.routing.candidates.is_empty()
        || snapshot.routing.candidates.len() > MAX_ROUTING_CANDIDATES
    {
        return Err(JobProvenanceError::InvalidResponse);
    }
    let requested = validate_shape_policy(revision, &snapshot, profile)?;
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
    let routing_candidates = external_routing_candidates(snapshot.routing.candidates)?;
    if !routing_candidates.iter().any(|candidate| {
        candidate.provider == snapshot.provider
            && candidate.deployment == snapshot.deployment
            && candidate.status == "eligible"
    }) {
        return Err(JobProvenanceError::InvalidResponse);
    }

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
    profile: JobPolicyProfile,
) -> Result<ValidatedShapePolicy, JobProvenanceError> {
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
    let capability_floor = constraint_string(constraints, revision.job_capability_floor_key())?;
    let latency = optional_constraint_string(constraints, "latency")?;
    let deadline_ms = optional_constraint_u64(constraints, "deadline_ms")?;
    let max_cost_usd = constraints
        .get("max_cost_usd")
        .and_then(Value::as_f64)
        .ok_or(JobProvenanceError::InvalidResponse)?;
    let valid_profile = match profile {
        JobPolicyProfile::ShapeLocalInteractive => {
            policy == "local-first"
                && priority == "interactive"
                && provider_access_class.is_none()
                && placement == "local_only"
                && preference == "local"
                && offline_required
                && latency.as_deref() == Some("interactive")
                && snapshot.placement == "local"
                && snapshot.policy == "local-first"
                && snapshot.priority == "interactive"
        }
        JobPolicyProfile::ShapeCloudImageInteractive => {
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
    };
    if !valid_profile
        || fallback != "none"
        || max_cost_usd != 0.0
        || capability_floor != revision.capable_level()
        || deadline_ms.is_some()
        || capability_floor != snapshot.routing.capability_floor
    {
        return Err(JobProvenanceError::PolicyViolation);
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
) -> Result<(), JobProvenanceError> {
    let object = snapshot
        .as_object()
        .ok_or(JobProvenanceError::InvalidResponse)?;
    let constraints = object
        .get("constraints")
        .and_then(Value::as_object)
        .ok_or(JobProvenanceError::InvalidResponse)?;
    let routing = object
        .get("routing")
        .and_then(Value::as_object)
        .ok_or(JobProvenanceError::InvalidResponse)?;
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
    valid
        .then_some(())
        .ok_or(JobProvenanceError::InvalidResponse)
}

fn external_attempts(
    attempts: Vec<Attempt>,
) -> Result<Vec<ExternalAttemptProvenance>, JobProvenanceError> {
    let mut previous_number = 0;
    attempts
        .into_iter()
        .map(|attempt| {
            if attempt.number == 0
                || attempt.number <= previous_number
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
            previous_number = attempt.number;
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
mod tests;
