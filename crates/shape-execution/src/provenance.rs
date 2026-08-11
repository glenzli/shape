//! Bounded, payload-free provenance returned by external executors.
//!
//! These values remain part of a transient [`crate::ExecutionOutput`] until a
//! future project-schema migration defines how acceptance persists them.

use serde::{Deserialize, Serialize};

const MAX_PROVENANCE_TEXT_BYTES: usize = 256;
const MAX_ROUTING_CANDIDATES: usize = 64;
const MAX_ATTEMPTS: usize = 16;
const MAX_REASON_CODES: usize = 32;

/// One payload-free physical attempt returned by an external runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAttemptProvenance {
    pub number: u32,
    pub provider: String,
    pub deployment: String,
    pub outcome: String,
    pub trigger: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
}

/// One bounded routing candidate considered by an external runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRoutingCandidate {
    pub provider: String,
    pub deployment: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    pub reason_codes: Vec<String>,
}

/// Payload-free physical facts copied from a runtime-owned Job snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalExecutionProvenance {
    pub contract_revision: String,
    pub app_id: String,
    pub intent: String,
    pub provider: String,
    pub deployment: String,
    pub model_profile: String,
    pub model_build: String,
    pub physical_model: String,
    pub placement: String,
    #[serde(alias = "quality_grade")]
    pub capability_level: String,
    #[serde(alias = "rating_status")]
    pub evaluation_status: String,
    pub resource_class: String,
    pub policy: String,
    pub priority: String,
    pub requested_policy: String,
    pub requested_priority: String,
    pub requested_provider_access_class: Option<String>,
    pub requested_placement: String,
    pub requested_preference: String,
    pub offline_required: bool,
    pub requested_latency: Option<String>,
    pub fallback: String,
    pub requested_deadline_ms: Option<u64>,
    pub max_cost_microusd: u64,
    #[serde(alias = "quality_floor")]
    pub capability_floor: String,
    pub routing_candidates: Vec<ExternalRoutingCandidate>,
    pub attempts: Vec<ExternalAttemptProvenance>,
}

impl ExternalExecutionProvenance {
    /// Returns whether this fixed-schema provenance is bounded and payload-free.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        let scalar_fields = [
            self.contract_revision.as_str(),
            self.app_id.as_str(),
            self.intent.as_str(),
            self.provider.as_str(),
            self.deployment.as_str(),
            self.model_profile.as_str(),
            self.model_build.as_str(),
            self.physical_model.as_str(),
            self.placement.as_str(),
            self.capability_level.as_str(),
            self.evaluation_status.as_str(),
            self.resource_class.as_str(),
            self.policy.as_str(),
            self.priority.as_str(),
            self.requested_policy.as_str(),
            self.requested_priority.as_str(),
            self.requested_placement.as_str(),
            self.requested_preference.as_str(),
            self.fallback.as_str(),
            self.capability_floor.as_str(),
        ];
        scalar_fields.iter().all(|value| bounded_text(value))
            && self
                .requested_provider_access_class
                .as_deref()
                .is_none_or(bounded_text)
            && self.requested_latency.as_deref().is_none_or(bounded_text)
            && self.routing_candidates.len() <= MAX_ROUTING_CANDIDATES
            && !self.attempts.is_empty()
            && self.attempts.len() <= MAX_ATTEMPTS
            && self.routing_candidates.iter().all(|candidate| {
                bounded_text(&candidate.provider)
                    && bounded_text(&candidate.deployment)
                    && matches!(
                        candidate.status.as_str(),
                        "eligible" | "fallback_eligible" | "rejected"
                    )
                    && candidate.reason_codes.len() <= MAX_REASON_CODES
                    && candidate.reason_codes.iter().all(|code| bounded_text(code))
            })
            && self.attempts.iter().all(|attempt| {
                attempt.number > 0
                    && bounded_text(&attempt.provider)
                    && bounded_text(&attempt.deployment)
                    && matches!(
                        attempt.outcome.as_str(),
                        "running" | "succeeded" | "failed" | "interrupted"
                    )
                    && matches!(
                        attempt.trigger.as_str(),
                        "initial" | "retry" | "fallback" | "recovery"
                    )
                    && attempt.error_kind.as_deref().is_none_or(bounded_text)
            })
            && self
                .attempts
                .windows(2)
                .all(|pair| pair[0].number < pair[1].number)
            && self.attempts.last().is_some_and(|attempt| {
                attempt.outcome == "succeeded"
                    && attempt.provider == self.provider
                    && attempt.deployment == self.deployment
            })
            && self.routing_candidates.iter().any(|candidate| {
                candidate.provider == self.provider
                    && candidate.deployment == self.deployment
                    && matches!(candidate.status.as_str(), "eligible" | "fallback_eligible")
            })
    }
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROVENANCE_TEXT_BYTES && value.is_ascii()
}

#[cfg(test)]
mod tests;
