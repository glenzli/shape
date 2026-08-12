use infer_runtime_client::AudioBytesResponse;
use shape_domain::{
    ArtifactContentContract, AuthorizedVoiceReference, ContentDigest, ContentRef, PresetVoiceAlias,
    PresetVoiceSelection, RevisionId, SpeechSynthesisOperation, SpeechVoiceSelection,
    TransformationId, VoiceAuthorizationScope,
};

use super::{
    AUDIO_MEDIA_TYPE, AUDIO_SPEECH_SYNTHESIZE_CAPABILITY,
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1, InferRuntimeSpeechExecutor, SPEECH_DEPLOYMENT,
    local_unary_request,
};
use crate::infer_runtime::{job_provenance::tests::speech_job, test_support::FakeSdk};
use crate::{CapabilityId, ExecutionInput, ExecutionRequest, Executor as _};

fn wav(sample_rate_hz: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block_align = channels * 2;
    let data_size = frames * u32::from(block_align);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate_hz * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.resize(bytes.len() + data_size as usize, 0);
    bytes
}

fn operation(voice: SpeechVoiceSelection) -> SpeechSynthesisOperation {
    SpeechSynthesisOperation::new(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        voice,
        1_000,
        true,
    )
    .unwrap()
}

fn preset() -> PresetVoiceSelection {
    PresetVoiceSelection::new(
        PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1).unwrap(),
        INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    )
    .unwrap()
}

fn request(operation: &SpeechSynthesisOperation) -> ExecutionRequest {
    let text = "这是一条普通的合成旁白。".as_bytes().to_vec();
    let content = ContentRef::new(
        ContentDigest::from_bytes(&text),
        "text/plain; charset=utf-8",
        text.len() as u64,
    )
    .unwrap();
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(AUDIO_SPEECH_SYNTHESIZE_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content, text).unwrap()],
        serde_json::to_vec(operation).unwrap(),
        AUDIO_MEDIA_TYPE,
    )
    .unwrap()
}

#[test]
fn speech_request_is_unary_wav_with_exact_local_named_narrowing() {
    let operation = operation(SpeechVoiceSelection::Preset(preset()));
    let request = local_unary_request(
        "bounded fixture",
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
        &operation,
    );
    assert_eq!(request.model, "speech.synthesize");
    assert_eq!(request.metadata["infer.deployment_ids"], SPEECH_DEPLOYMENT);
    assert_eq!(request.metadata["infer.placement"], "local_only");
    assert_eq!(request.metadata["infer.offline_required"], "true");
    assert_eq!(request.metadata["infer.fallback"], "none");
    assert_eq!(request.metadata["infer.max_cost_usd"], "0");
}

#[test]
fn sdk_unary_wav_and_job_become_typed_transient_output() {
    let bytes = wav(24_000, 1, 2_400);
    let fake = FakeSdk::new()
        .speech(AudioBytesResponse {
            bytes: bytes.clone(),
            content_type: "audio/wav".into(),
            job_id: "resp_shape_job".into(),
            logical_model: "speech.synthesize".into(),
        })
        .job(speech_job());
    let executor = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake));
    let output = executor
        .execute(&request(&operation(SpeechVoiceSelection::Preset(preset()))))
        .expect("fake SDK speech succeeds");
    assert_eq!(output.bytes, bytes);
    assert_eq!(output.executor_job_id.as_deref(), Some("resp_shape_job"));
    assert_eq!(
        output.content_contract,
        Some(ArtifactContentContract::AudioClip(
            shape_domain::AudioValueContract::pcm_s16le_wav(
                24_000,
                1,
                2_400,
                shape_domain::AudioOriginDisclosure::SyntheticSpeech,
            )
            .unwrap()
        ))
    );
    assert_eq!(
        output.external_provenance.unwrap().deployment,
        SPEECH_DEPLOYMENT
    );
}

#[test]
fn authorized_voice_reference_remains_dependency_gated_before_sdk() {
    let reference = AuthorizedVoiceReference::new(
        RevisionId::new(),
        "consent-receipt",
        VoiceAuthorizationScope::SpeechSynthesis,
        true,
        true,
    )
    .unwrap();
    let executor = InferRuntimeSpeechExecutor::with_sdk(Box::new(FakeSdk::new()));
    let error = executor
        .execute(&request(&operation(
            SpeechVoiceSelection::AuthorizedReference(reference),
        )))
        .expect_err("voice references are not executable");
    assert_eq!(error.code, "voice_reference_not_supported");
}

#[test]
fn non_wav_sdk_payload_fails_closed() {
    let fake = FakeSdk::new()
        .speech(AudioBytesResponse {
            bytes: b"not wav".to_vec(),
            content_type: "audio/wav".into(),
            job_id: "resp_shape_job".into(),
            logical_model: "speech.synthesize".into(),
        })
        .job(speech_job());
    let error = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .execute(&request(&operation(SpeechVoiceSelection::Preset(preset()))))
        .expect_err("invalid bytes never become a Candidate");
    assert_eq!(error.code, "invalid_audio_output");
}
