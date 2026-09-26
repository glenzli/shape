//! Local sound/music SDK adapter and cancellable response lifetime.
use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, parse_job_snapshot},
    official_sdk,
    sdk::{InferRuntimeSdk, execution_failure},
};
use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    ExternalExecutionProvenance, parse_pcm_s16le_wav,
};
use infer_runtime_client::{PreparedSoundPrompt, SoundGenerationRequest, SoundModelChoice};
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, AudioValueContract, SoundGenerationKind,
    SoundGenerationOperation,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    time::Duration,
};

pub const AUDIO_GENERATE_CAPABILITY: &str = "audio.generate";
pub const INFER_RUNTIME_SOUND_GENERATION_CAPABILITY: &str =
    "infer.audio.sound-generation@20260926.2";
const SOUND_INTENT: &str = "audio.generate_sound";

/// Stopping drops the HTTP future, not an assertion that the provider has terminated.
#[derive(Debug, Default)]
struct SoundState {
    cancelled: AtomicBool,
    stage: AtomicU8,
    prepared: Mutex<Option<PreparedSoundPrompt>>,
}
#[derive(Debug, Clone, Default)]
pub struct SoundGenerationControl(Arc<SoundState>);
impl SoundGenerationControl {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
    }
    pub fn resume(&self) {
        self.0.cancelled.store(false, Ordering::Release);
        self.0.stage.store(0, Ordering::Release);
    }
    #[must_use]
    pub fn stage(&self) -> u8 {
        self.0.stage.load(Ordering::Acquire)
    }
    #[must_use]
    pub fn cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    fn prepare(
        &self,
        sdk: &dyn InferRuntimeSdk,
        original: &str,
    ) -> Result<PreparedSoundPrompt, ExecutionFailure> {
        self.0.stage.store(1, Ordering::Release);
        let mut cached = self
            .0
            .prepared
            .lock()
            .map_err(|_| failure("prompt_preparation_failed"))?;
        if let Some(p) = cached
            .as_ref()
            .filter(|p| p.validate_for_generation(original, "shape").is_ok())
        {
            return Ok(p.clone());
        }
        *cached = None;
        let prepared = sdk
            .prepare_sound_prompt(original, Duration::from_mins(2), self)
            .map_err(|e| {
                let (code, retryable) = execution_failure(e);
                ExecutionFailure::new(code, "Sound prompt preparation did not complete", retryable)
            })?;
        prepared
            .validate_for_generation(original, "shape")
            .map_err(|_| failure("prompt_preparation_failed"))?;
        if self.cancelled() {
            return Err(failure("generation_cancelled"));
        }
        *cached = Some(prepared.clone());
        Ok(prepared)
    }
}

pub struct InferRuntimeSoundExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
    control: SoundGenerationControl,
}
impl std::fmt::Debug for InferRuntimeSoundExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferRuntimeSoundExecutor")
            .field("identity", &self.identity)
            .finish_non_exhaustive()
    }
}
impl InferRuntimeSoundExecutor {
    /// Creates an authenticated local SDK consumer.
    /// # Errors
    /// Rejects an invalid SDK endpoint or credential setup.
    pub fn new(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
        control: SoundGenerationControl,
    ) -> Result<Self, crate::ExecutionError> {
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk(Box::new(sdk), control))
    }
    fn with_sdk(sdk: Box<dyn InferRuntimeSdk>, control: SoundGenerationControl) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-sound-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("built-in identity"),
            sdk,
            control,
        }
    }
}
impl Executor for InferRuntimeSoundExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }
    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == AUDIO_GENERATE_CAPABILITY
    }
    fn execute(&self, input: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if !input.inputs.is_empty()
            || input.output_media_type != "audio/wav"
            || input.instruction.len() > 4096
        {
            return Err(failure("invalid_sound_request"));
        }
        let operation: SoundGenerationOperation = serde_json::from_slice(&input.instruction)
            .map_err(|_| failure("invalid_sound_request"))?;
        operation
            .validate()
            .map_err(|_| failure("invalid_sound_request"))?;
        if self.control.cancelled() {
            return Err(failure("generation_cancelled"));
        }
        let prepared = self.control.prepare(self.sdk.as_ref(), &operation.prompt)?;
        if self.control.cancelled() {
            return Err(failure("generation_cancelled"));
        }
        let mut request = sound_request(&operation);
        request.prompt.clone_from(&prepared.effective_prompt);
        self.control.0.stage.store(2, Ordering::Release);
        let response = self
            .sdk
            .generate_sound(&request, Duration::from_mins(10), &self.control)
            .map_err(|e| {
                let (code, retryable) = execution_failure(e);
                ExecutionFailure::new(code, "Sound generation did not complete", retryable)
            })?;
        if self.control.cancelled() {
            return Err(failure("generation_cancelled"));
        }
        let job = self
            .sdk
            .sound_job(&response.job_id, &self.control)
            .map_err(|e| {
                let (code, retryable) = execution_failure(e);
                ExecutionFailure::new(code, "Sound provenance could not be verified", retryable)
            })?;
        if response.logical_model != SOUND_INTENT
            || response.model_choice != model_choice(operation.kind)
            || response.seed != operation.seed
            || response.duration_seconds != operation.duration_seconds
            || response.job_id != job.id
            || response.provider != job.provider
            || response.deployment != job.deployment
            || response.model_build != job.model_build
            || response.physical_model != job.physical_model
            || response.placement != job.placement
        {
            return Err(failure("infer_invalid_response"));
        }
        let mut provenance = parse_job_snapshot(
            &response.job_id,
            SOUND_INTENT,
            JobPolicyProfile::LocalSound(deployment(operation.kind)),
            job,
        )
        .map_err(|e| failure(e.code()))?;
        provenance.sound_prompt = Some(
            crate::sound_prompt::SoundPromptProvenance::from_prepared(prepared, &operation)
                .ok_or_else(|| failure("prompt_preparation_failed"))?,
        );
        let contract = parse_pcm_s16le_wav(&response.wav, AudioOriginDisclosure::SyntheticSound)?;
        if !valid_sound_output(&operation, &contract, &provenance) {
            return Err(failure("infer_invalid_response"));
        }
        if self.control.cancelled() {
            return Err(failure("generation_cancelled"));
        }
        Ok(ExecutionOutput {
            bytes: response.wav,
            media_type: "audio/wav".into(),
            executor_job_id: Some(response.job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::AudioClip(contract)),
        })
    }
}
pub(crate) fn sound_request(operation: &SoundGenerationOperation) -> SoundGenerationRequest {
    SoundGenerationRequest {
        model: SOUND_INTENT.into(),
        model_choice: Some(model_choice(operation.kind)),
        prompt: operation.prompt.clone(),
        duration_seconds: operation.duration_seconds,
        seed: Some(operation.seed),
        metadata: BTreeMap::from([
            ("infer.capability_floor".into(), "foundational".into()),
            ("infer.fallback".into(), "none".into()),
            ("infer.latency".into(), "balanced".into()),
            ("infer.max_cost_usd".into(), "0".into()),
            ("infer.offline_required".into(), "true".into()),
            ("infer.placement".into(), "local_only".into()),
            ("infer.policy".into(), "local-first".into()),
            ("infer.prefer".into(), "local".into()),
            ("infer.priority".into(), "interactive".into()),
        ]),
    }
}
fn model_choice(kind: SoundGenerationKind) -> SoundModelChoice {
    match kind {
        SoundGenerationKind::SoundEffect => SoundModelChoice::SmallSfx,
        SoundGenerationKind::ShortMusic => SoundModelChoice::SmallMusic,
    }
}
fn deployment(kind: SoundGenerationKind) -> &'static str {
    match kind {
        SoundGenerationKind::SoundEffect => "stable_audio_3_sm_sfx_mlx",
        SoundGenerationKind::ShortMusic => "stable_audio_3_sm_music_mlx",
    }
}
/// Current generation rules are required at proposal and new acceptance.
/// Historical receipt reads use the compatible provenance validation separately.
#[must_use]
pub fn valid_sound_output(
    operation: &SoundGenerationOperation,
    contract: &AudioValueContract,
    p: &ExternalExecutionProvenance,
) -> bool {
    operation.validate().is_ok()
        && contract.origin == AudioOriginDisclosure::SyntheticSound
        && contract.sample_rate_hz == 44_100
        && contract.channels == 2
        && contract.frame_count == u64::from(operation.duration_seconds) * 44_100
        && p.sound_prompt.as_ref().is_some_and(|p| {
            p.valid_for_generation(&operation.prompt)
                && serde_json::to_vec(operation).is_ok_and(|bytes| {
                    shape_domain::ContentDigest::from_bytes(&bytes) == p.authored_request_digest
                })
        })
        && p.is_bounded()
        && p.contract_revision == INFER_RUNTIME_CONTRACT_VERSION
        && p.capability_contract.as_deref() == Some(INFER_RUNTIME_SOUND_GENERATION_CAPABILITY)
        && p.app_id == "shape"
        && p.intent == SOUND_INTENT
        && p.deployment == deployment(operation.kind)
        && p.model_build == deployment(operation.kind)
        && Some(p.physical_model.as_str()) == model_choice(operation.kind).physical_model()
        && p.placement == "local"
        && p.policy == "local-first"
        && p.requested_policy == "local-first"
        && p.priority == "interactive"
        && p.requested_priority == "interactive"
        && p.requested_placement == "local_only"
        && p.requested_preference == "local"
        && p.offline_required
        && p.requested_latency.as_deref() == Some("balanced")
        && p.requested_deadline_ms.is_none()
        && p.requested_provider_access_class
            .as_deref()
            .is_none_or(|v| v == "standard")
        && p.fallback == "none"
        && p.max_cost_microusd == 0
        && p.capability_floor == "foundational"
        && p.named_route.is_none()
        && p.attempts.last().is_some_and(|a| {
            a.outcome == "succeeded" && a.provider == p.provider && a.deployment == p.deployment
        })
        && p.attempts.iter().all(|a| a.trigger != "fallback")
        && p.speech_segments.is_empty()
        && p.speech_script.is_none()
}
fn failure(code: &str) -> ExecutionFailure {
    ExecutionFailure::new(code, "Sound generation output could not be verified", false)
}
#[cfg(test)]
mod tests;
