//! Durable, bounded original/effective prompt record owned by sound execution.
use infer_runtime_client::PreparedSoundPrompt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundPromptProvenance {
    pub authored_request_digest: shape_domain::ContentDigest,
    pub original_prompt: String,
    pub effective_prompt: String,
    pub rules_revision: String,
    /// Exact payload-free SDK Job projection, validated before every consumption.
    pub text_job: Option<Value>,
    pub preparation_elapsed_ms: u64,
}
impl SoundPromptProvenance {
    pub(crate) fn from_prepared(
        p: PreparedSoundPrompt,
        operation: &shape_domain::SoundGenerationOperation,
    ) -> Option<Self> {
        p.validate_for_generation(&operation.prompt, "shape").ok()?;
        Some(Self {
            authored_request_digest: shape_domain::ContentDigest::from_bytes(
                &serde_json::to_vec(operation).ok()?,
            ),
            original_prompt: p.original_prompt,
            effective_prompt: p.effective_prompt,
            rules_revision: p.rules_revision,
            text_job: p.text_job.map(serde_json::to_value).transpose().ok()?,
            preparation_elapsed_ms: p.preparation_elapsed_ms,
        })
    }
    /// Validates stored history without requiring today's generation rules.
    #[must_use]
    pub fn valid_for(&self, original: &str) -> bool {
        self.prepared()
            .is_some_and(|p| p.validate_for(original, "shape").is_ok())
    }

    /// Requires current preparation and exclusion checks for new Candidates and acceptance.
    #[must_use]
    pub fn valid_for_generation(&self, original: &str) -> bool {
        self.prepared()
            .is_some_and(|p| p.validate_for_generation(original, "shape").is_ok())
    }

    fn prepared(&self) -> Option<PreparedSoundPrompt> {
        if self
            .text_job
            .as_ref()
            .is_some_and(|j| j.to_string().len() > 65_536)
        {
            return None;
        }
        Some(PreparedSoundPrompt {
            original_prompt: self.original_prompt.clone(),
            effective_prompt: self.effective_prompt.clone(),
            rules_revision: self.rules_revision.clone(),
            text_job: self
                .text_job
                .clone()
                .map(serde_json::from_value)
                .transpose()
                .ok()?,
            preparation_elapsed_ms: self.preparation_elapsed_ms,
        })
    }
}

impl std::fmt::Debug for SoundPromptProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundPromptProvenance")
            .field("rules_revision", &self.rules_revision)
            .field("translated", &self.text_job.is_some())
            .finish_non_exhaustive()
    }
}
