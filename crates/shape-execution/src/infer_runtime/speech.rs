//! Official-SDK-backed local unary WAV speech synthesis executor.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use infer_runtime_client::{ExecutionMode, SpeechFormat, SpeechRequest};
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, SpeechSynthesisOperation, SpeechVoiceSelection,
};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    audio::{MAX_AUDIO_OUTPUT_BYTES, parse_pcm_s16le_wav},
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, SPEECH_DEPLOYMENT, parse_job_snapshot, valid_job_id},
    official_sdk,
    sdk::{InferRuntimeSdk, SdkAdapterError, execution_failure},
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
const MAX_INSTRUCTION_BYTES: usize = 16 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_mins(5);

/// Shape's typed unary WAV adapter for the stable Runtime speech capability.
pub struct InferRuntimeSpeechExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
}

impl std::fmt::Debug for InferRuntimeSpeechExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeSpeechExecutor")
            .field("identity", &self.identity)
            .field("sdk", &"official-infer-runtime-client")
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
        }
    }

    fn create_speech(
        &self,
        text: &str,
        operation: &SpeechSynthesisOperation,
    ) -> Result<ExecutionOutput, ExecutionFailure> {
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
        let contract = parse_pcm_s16le_wav(&response.bytes, AudioOriginDisclosure::SyntheticSpeech)
            .map_err(|_| failure("invalid_audio_output", false))?;
        let job = self.sdk.job(&response.job_id).map_err(map_failure)?;
        let provenance = parse_job_snapshot(
            &response.job_id,
            SPEECH_INTENT,
            JobPolicyProfile::LocalSpeech,
            job,
        )
        .map_err(|error| failure(error.code(), false))?;
        Ok(ExecutionOutput {
            bytes: response.bytes,
            media_type: AUDIO_MEDIA_TYPE.to_owned(),
            executor_job_id: Some(response.job_id),
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
        self.create_speech(text, &operation)
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
        instructions: None,
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
