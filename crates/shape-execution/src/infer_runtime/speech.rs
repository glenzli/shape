//! Official-SDK-backed local unary WAV speech synthesis executor.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use infer_runtime_client::{ExecutionMode, SpeechFormat, SpeechRequest};
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, ContentDigest, SpeechSynthesisOperation,
    SpeechVoiceSelection,
};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    SpeechSegmentProvenance,
    audio::{MAX_AUDIO_OUTPUT_BYTES, SpeechWaveAssembly, parse_pcm_s16le_wav},
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, SPEECH_DEPLOYMENT, parse_job_snapshot, valid_job_id},
    official_sdk,
    sdk::{InferRuntimeSdk, SdkAdapterError, execution_failure},
};

pub(crate) mod narration;
mod script;
mod voices;
pub use narration::SpeechSynthesisControl;
use narration::{CachedNarration, Segment, split_text};
pub use voices::{
    INFER_SPEECH_VOICE_CATALOG_REVISION, SPEECH_PRESETS, SpeechPreset, supported_speech_operation,
};

/// Shape-side creative capability implemented by Runtime `speech.synthesize`.
pub const AUDIO_SPEECH_SYNTHESIZE_CAPABILITY: &str = "audio.speech_synthesize";
/// Runtime-owned voice catalog revision required by the first executable Operator.
pub const INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION: &str = "infer.speech.voice-aliases@20260811.1";
/// Stable Runtime voice identity for ordinary synthetic Mandarin narration.
pub const INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1: &str = "speech.voice.zh.bright_female.v1";
/// Exact Runtime language value paired with the first stable voice alias.
pub const INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE: &str = "Chinese";

const SPEECH_INTENT: &str = "speech.synthesize";
const AUDIO_MEDIA_TYPE: &str = "audio/wav";
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_INSTRUCTION_BYTES: usize = 64 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_mins(5);

/// Shape's typed unary WAV adapter for the stable Runtime speech capability.
pub struct InferRuntimeSpeechExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
    control: SpeechSynthesisControl,
}

impl std::fmt::Debug for InferRuntimeSpeechExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeSpeechExecutor")
            .field("identity", &self.identity)
            .field("sdk", &"official-infer-runtime-client")
            .field("control", &"shared-narration-progress")
            .finish()
    }
}

impl InferRuntimeSpeechExecutor {
    /// Creates an official SDK Consumer using Shape's managed credential file.
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

    fn with_sdk(sdk: Box<dyn InferRuntimeSdk>) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-speech-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("built-in Infer Runtime identity is valid"),
            sdk,
            control: SpeechSynthesisControl::default(),
        }
    }

    /// Shares segment progress and bounded retry state with the caller.
    #[must_use]
    pub fn with_control(mut self, control: SpeechSynthesisControl) -> Self {
        self.control = control;
        self
    }

    fn create_segment(
        &self,
        text: &str,
        operation: &SpeechSynthesisOperation,
    ) -> Result<Segment, ExecutionFailure> {
        let SpeechVoiceSelection::Preset(voice) = &operation.voice else {
            return Err(failure("voice_reference_not_supported", false));
        };
        let request = local_unary_request(text, voice.alias.as_str(), operation);
        let response = self
            .sdk
            .synthesize_speech(&request, REQUEST_TIMEOUT)
            .map_err(map_failure)?;
        if response.bytes.len() > MAX_AUDIO_OUTPUT_BYTES
            || response.content_type.split(';').next().map(str::trim) != Some(AUDIO_MEDIA_TYPE)
            || response.logical_model != SPEECH_INTENT
            || !valid_job_id(&response.job_id)
        {
            return Err(failure("infer_invalid_response", false));
        }
        parse_pcm_s16le_wav(&response.bytes, AudioOriginDisclosure::SyntheticSpeech)
            .map_err(|_| failure("invalid_audio_output", false))?;
        let job = self.sdk.job(&response.job_id).map_err(map_failure)?;
        let provenance = parse_job_snapshot(
            &response.job_id,
            SPEECH_INTENT,
            JobPolicyProfile::LocalSpeech,
            job,
        )
        .map_err(|error| failure(error.code(), false))?;
        Ok(Segment {
            bytes: response.bytes.into(),
            job_id: response.job_id,
            provenance,
        })
    }
    fn create_speech(
        &self,
        text: &str,
        operation: &SpeechSynthesisOperation,
    ) -> Result<ExecutionOutput, ExecutionFailure> {
        let parts = split_text(text);
        if parts.len() > 512 {
            return Err(failure("speech_text_too_long", false));
        }
        let mut key_bytes =
            serde_json::to_vec(operation).map_err(|_| failure("invalid_speech_request", false))?;
        key_bytes.extend_from_slice(text.as_bytes());
        let key = ContentDigest::from_bytes(&key_bytes);
        let mut cache_guard = self.control.cache()?;
        if cache_guard.as_ref().is_none_or(|cache| cache.key != key) {
            *cache_guard = Some(CachedNarration {
                key,
                segments: Vec::new(),
            });
        }
        let cache = cache_guard.as_mut().expect("narration initialized");
        self.control.progress(cache.segments.len(), parts.len());
        let mut assembly = SpeechWaveAssembly::default();
        let mut receipts = Vec::with_capacity(parts.len());
        let mut input_start = 0_u32;
        for (index, part) in parts.iter().enumerate() {
            self.control.check_cancelled()?;
            let segment = if let Some(cached) = cache.segments.get(index) {
                cached.clone()
            } else {
                self.create_segment(part, operation)?
            };
            assembly.push(&segment.bytes)?;
            let contract =
                parse_pcm_s16le_wav(&segment.bytes, AudioOriginDisclosure::SyntheticSpeech)?;
            if index == cache.segments.len() {
                cache.segments.push(segment.clone());
            }
            self.control.progress(cache.segments.len(), parts.len());
            self.control.check_cancelled()?;
            let input_end = input_start
                + u32::try_from(part.len()).map_err(|_| failure("invalid_speech_source", false))?;
            receipts.push(SpeechSegmentProvenance {
                job_id: segment.job_id.clone(),
                input_start,
                input_end,
                input_digest: ContentDigest::from_bytes(part.as_bytes()),
                output_digest: ContentDigest::from_bytes(&segment.bytes),
                frames: contract.frame_count,
                runtime: Box::new(segment.provenance),
            });
            input_start = input_end;
        }
        let first = receipts
            .first()
            .ok_or_else(|| failure("invalid_speech_source", false))?;
        let job_id = first.job_id.clone();
        let mut provenance = *first.runtime.clone();
        let bytes = if parts.len() == 1 {
            cache.segments[0].bytes.to_vec()
        } else {
            provenance.speech_segments = receipts;
            assembly.finish()?
        };
        let contract = parse_pcm_s16le_wav(&bytes, AudioOriginDisclosure::SyntheticSpeech)?;
        // Successful runs release the retry cache. A subsequent run creates a new option.
        *cache_guard = None;
        Ok(ExecutionOutput {
            bytes,
            media_type: AUDIO_MEDIA_TYPE.to_owned(),
            executor_job_id: Some(job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::AudioClip(contract)),
        })
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
            || request.inputs.is_empty()
            || request.inputs.len() > 129
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
            .filter(|text| !text.trim().is_empty() && text.len() <= MAX_TEXT_BYTES)
            .ok_or_else(|| failure("invalid_speech_source", false))?;
        let operation: SpeechSynthesisOperation = serde_json::from_slice(&request.instruction)
            .map_err(|_| failure("invalid_speech_request", false))?;
        operation
            .validate()
            .map_err(|_| failure("invalid_speech_request", false))?;
        let SpeechVoiceSelection::Preset(_) = &operation.voice else {
            return Err(failure("voice_reference_not_supported", false));
        };
        if !supported_speech_operation(&operation) {
            return Err(failure("unsupported_speech_preset", false));
        }
        if operation.script.is_some() {
            self.create_script(text, &operation, &request.inputs)
        } else if request.inputs.len() == 1 {
            self.create_speech(text, &operation)
        } else {
            Err(failure("invalid_speech_request", false))
        }
    }
}

fn local_unary_request(
    input: &str,
    voice: &str,
    operation: &SpeechSynthesisOperation,
) -> SpeechRequest {
    SpeechRequest {
        model: SPEECH_INTENT.to_owned(),
        input: input.to_owned(),
        voice: Some(voice.to_owned()),
        instructions: operation
            .delivery
            .map(|delivery| delivery.instruction().to_owned()),
        language: Some(operation.language.clone()),
        speed: f64::from(operation.speed_milli) / 1_000.0,
        response_format: SpeechFormat::Wav,
        execution_mode: ExecutionMode::Unary,
        metadata: BTreeMap::from([
            ("infer.capability_floor".to_owned(), "capable".to_owned()),
            (
                "infer.deployment_ids".to_owned(),
                SPEECH_DEPLOYMENT.to_owned(),
            ),
            ("infer.fallback".to_owned(), "none".to_owned()),
            ("infer.latency".to_owned(), "interactive".to_owned()),
            ("infer.max_cost_usd".to_owned(), "0".to_owned()),
            ("infer.offline_required".to_owned(), "true".to_owned()),
            ("infer.placement".to_owned(), "local_only".to_owned()),
            ("infer.policy".to_owned(), "local-first".to_owned()),
            ("infer.prefer".to_owned(), "local".to_owned()),
            ("infer.priority".to_owned(), "interactive".to_owned()),
        ]),
    }
}

fn map_failure(error: SdkAdapterError) -> ExecutionFailure {
    let (code, retryable) = execution_failure(error);
    failure(&code, retryable)
}

fn failure(code: &str, retryable: bool) -> ExecutionFailure {
    ExecutionFailure::new(
        code,
        "Infer Runtime could not synthesize an audio candidate",
        retryable,
    )
}

#[cfg(test)]
mod tests;
