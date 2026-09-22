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

fn request_text(text: &str) -> ExecutionRequest {
    let bytes = text.as_bytes().to_vec();
    let content = ContentRef::new(
        ContentDigest::from_bytes(&bytes),
        "text/plain",
        bytes.len() as u64,
    )
    .unwrap();
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(AUDIO_SPEECH_SYNTHESIZE_CAPABILITY).unwrap(),
        vec![ExecutionInput::materialized(content, bytes).unwrap()],
        serde_json::to_vec(&operation(SpeechVoiceSelection::Preset(preset()))).unwrap(),
        AUDIO_MEDIA_TYPE,
    )
    .unwrap()
}

fn response(index: usize) -> AudioBytesResponse {
    AudioBytesResponse {
        bytes: wav(24_000, 1, 240),
        content_type: "audio/wav".into(),
        job_id: format!("resp_segment_{index}"),
        logical_model: "speech.synthesize".into(),
    }
}

#[test]
fn long_speech_preserves_text_pcm_and_each_runtime_receipt() {
    let text = "一句普通话旁白。".repeat(65);
    let parts = super::narration::split_text(&text);
    let mut fake = FakeSdk::new();
    for i in 0..parts.len() {
        let mut job = speech_job();
        job.id = format!("resp_segment_{i}");
        fake = fake.speech(response(i)).job(job);
    }
    let seen = fake.seen_speeches.clone();
    let output = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .execute(&request_text(&text))
        .unwrap();
    let provenance = output.external_provenance.unwrap();
    assert_eq!(provenance.speech_segments.len(), parts.len());
    assert!(provenance.is_bounded());
    assert_eq!(
        seen.lock()
            .unwrap()
            .iter()
            .map(|r| r.input.as_str())
            .collect::<Vec<_>>()
            .concat(),
        text
    );
    assert_eq!(output.bytes.len(), 44 + parts.len() * 480);
    assert_eq!(
        provenance.speech_segments.last().unwrap().input_end as usize,
        text.len()
    );
    for (part, receipt) in parts.iter().zip(&provenance.speech_segments) {
        assert_eq!(
            receipt.input_digest,
            ContentDigest::from_bytes(part.as_bytes())
        );
        assert_eq!(receipt.runtime.deployment, SPEECH_DEPLOYMENT);
    }
}

#[test]
fn retry_reuses_completed_segments_but_changed_text_invalidates_cache() {
    use crate::infer_runtime::sdk::SdkAdapterError;
    let text = "旁白内容。".repeat(70);
    let parts = super::narration::split_text(&text);
    assert_eq!(parts.len(), 2);
    let mut first_job = speech_job();
    first_job.id = "resp_segment_0".into();
    let fake = FakeSdk::new().speech(response(0)).job(first_job);
    fake.speeches
        .lock()
        .unwrap()
        .push_back(Err(SdkAdapterError::Timeout));
    let control = super::SpeechSynthesisControl::default();
    let error = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .with_control(control.clone())
        .execute(&request_text(&text))
        .unwrap_err();
    assert_eq!(error.code, "infer_unavailable");
    assert_eq!(control.completed(), 1);
    let mut second_job = speech_job();
    second_job.id = "resp_segment_1".into();
    let fake = FakeSdk::new().speech(response(1)).job(second_job);
    let seen = fake.seen_speeches.clone();
    let output = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .with_control(control.clone())
        .execute(&request_text(&text))
        .unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(output.external_provenance.unwrap().speech_segments.len(), 2);
    // Success drops retained PCM. A changed request always calls Runtime.
    let mut job = speech_job();
    job.id = "resp_segment_2".into();
    let fake = FakeSdk::new().speech(response(2)).job(job);
    let seen = fake.seen_speeches.clone();
    InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .with_control(control)
        .execute(&request_text("新的旁白。"))
        .unwrap();
    assert_eq!(seen.lock().unwrap()[0].input, "新的旁白。");
}

#[test]
fn all_presets_support_auto_and_each_explicit_language_with_expanded_catalog() {
    for preset in super::SPEECH_PRESETS {
        let mut op = operation(SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(preset.alias).unwrap(),
                super::INFER_SPEECH_VOICE_CATALOG_REVISION,
            )
            .unwrap(),
        ));
        op.language = preset.language.into();
        assert!(super::supported_speech_operation(&op));
        for language in super::voices::SPEECH_LANGUAGES {
            op.language = (*language).into();
            assert!(super::supported_speech_operation(&op));
        }
        op.language = "invalid".into();
        assert!(!super::supported_speech_operation(&op));
    }
}

#[test]
fn changed_settings_invalidate_a_failed_narrations_completed_segment() {
    use crate::infer_runtime::SdkAdapterError;
    let text = "旁白内容。".repeat(70);
    for change_voice in [false, true] {
        let mut job = speech_job();
        job.id = "resp_segment_0".into();
        let fake = FakeSdk::new().speech(response(0)).job(job);
        fake.speeches
            .lock()
            .unwrap()
            .push_back(Err(SdkAdapterError::Timeout));
        let control = super::SpeechSynthesisControl::default();
        InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
            .with_control(control.clone())
            .execute(&request_text(&text))
            .unwrap_err();
        assert_eq!(control.completed(), 1);
        let mut changed = request_text(&text);
        if change_voice {
            let mut op = operation(SpeechVoiceSelection::Preset(
                PresetVoiceSelection::new(
                    PresetVoiceAlias::new(super::SPEECH_PRESETS[2].alias).unwrap(),
                    super::INFER_SPEECH_VOICE_CATALOG_REVISION,
                )
                .unwrap(),
            ));
            op.language = "auto".into();
            changed.instruction = serde_json::to_vec(&op).unwrap();
        } else {
            changed = request_text(&text.replace("内容", "新稿"));
        }
        let mut fake = FakeSdk::new();
        for i in 10..12 {
            let mut job = speech_job();
            job.id = format!("resp_segment_{i}");
            fake = fake.speech(response(i)).job(job);
        }
        let seen = fake.seen_speeches.clone();
        let output = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
            .with_control(control)
            .execute(&changed)
            .unwrap();
        assert_eq!(seen.lock().unwrap().len(), 2);
        let provenance = output.external_provenance.unwrap();
        assert_eq!(provenance.speech_segments[0].job_id, "resp_segment_10");
        for damage in 0..4 {
            let mut invalid = provenance.clone();
            match damage {
                0 => invalid.speech_segments[1].input_start += 1,
                1 => invalid.speech_segments[1].job_id = invalid.speech_segments[0].job_id.clone(),
                2 => invalid.speech_segments[1].runtime.requested_policy = "cloud-first".into(),
                _ => {
                    invalid.speech_segments[1].runtime.speech_segments =
                        provenance.speech_segments.clone();
                }
            }
            assert!(!invalid.is_bounded());
        }
    }
}

#[test]
fn script_executes_only_spoken_lines_and_validates_exact_local_timeline() {
    use shape_domain::speech_script::{SpeechCueAction, SpeechScriptOptions};
    let source = "[role: Narrator]\n[role: Reader]\n[cue: intro]\n[cue: omitted]\n# exam\n[audio: intro]\n[speaker: Narrator]\n[language: auto]\n开始。\n[pause: 1.25s]\n[speaker: Reader]\n[language: English]\nLook at the boy.\n[audio: omitted]\n[pause: 2s]";
    let mut op = operation(SpeechVoiceSelection::Preset(
        PresetVoiceSelection::new(
            PresetVoiceAlias::new(super::SPEECH_PRESETS[0].alias).unwrap(),
            super::INFER_SPEECH_VOICE_CATALOG_REVISION,
        )
        .unwrap(),
    ));
    let mut options = SpeechScriptOptions::default();
    let SpeechVoiceSelection::Preset(voice) = op.voice.clone() else {
        panic!()
    };
    options.roles.insert("Narrator".into(), voice);
    options.cues.insert("intro".into(), SpeechCueAction::Chime);
    options.cues.insert("omitted".into(), SpeechCueAction::Skip);
    options.roles.insert(
        "Reader".into(),
        PresetVoiceSelection::new(
            PresetVoiceAlias::new(super::SPEECH_PRESETS[5].alias).unwrap(),
            super::INFER_SPEECH_VOICE_CATALOG_REVISION,
        )
        .unwrap(),
    );
    op.script = Some(options);
    let mut request = request_text(source);
    request.instruction = serde_json::to_vec(&op).unwrap();
    let mut fake = FakeSdk::new();
    for i in 0..2 {
        let mut job = speech_job();
        job.id = format!("resp_segment_{i}");
        fake = fake.speech(response(i)).job(job);
    }
    let seen = fake.seen_speeches.clone();
    let output = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .execute(&request)
        .unwrap();
    let provenance = output.external_provenance.unwrap();
    assert!(provenance.is_bounded());
    assert_eq!(output.bytes.len(), 44 + (24000 + 30000 + 48000 + 480) * 2);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].input, "开始。");
    assert_eq!(seen[0].language.as_deref(), Some("auto"));
    assert_eq!(seen[1].input, "Look at the boy.");
    assert_eq!(
        seen[1].voice.as_deref(),
        Some(super::SPEECH_PRESETS[5].alias)
    );
    assert_eq!(seen[1].language.as_deref(), Some("English"));
    let validate = |p: &crate::ExternalExecutionProvenance, bytes: &[u8]| {
        crate::speech_script::validate_output(source, &op, &request.inputs, p, bytes)
    };
    assert!(validate(&provenance, &output.bytes).is_ok());
    for mutation in 0..4 {
        let mut invalid = provenance.clone();
        let assembly = invalid.speech_script.as_mut().unwrap();
        match mutation {
            0 => assembly.pieces[0].event += 1,
            1 => assembly.pieces[2].frames += 1,
            2 => assembly.source_digest = ContentDigest::from_bytes(b"other"),
            _ => invalid.speech_segments[0].input_digest = ContentDigest::from_bytes(b"wrong"),
        }
        assert!(validate(&invalid, &output.bytes).is_err());
    }
    let mut corrupted = output.bytes.clone();
    corrupted[44 + (24000 + 240) * 2] = 1;
    assert!(validate(&provenance, &corrupted).is_err());
    op.script.as_mut().unwrap().cues.remove("intro");
    request.instruction = serde_json::to_vec(&op).unwrap();
    let fake = FakeSdk::new();
    let seen = fake.seen_speeches.clone();
    assert_eq!(
        InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
            .execute(&request)
            .unwrap_err()
            .code,
        "invalid_speech_script"
    );
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn repeated_dialogue_reuses_exact_pcm_and_receipts_and_transmits_delivery() {
    use shape_domain::speech_script::SpeechScriptOptions;
    let source = "[production: listening]\n[role: A; language: auto]\n[role: B; language: English]\n[cue: turn; sound: beep; duration: 0.125s]\n[scene: question-1]\n[speaker: A]\nNumber one.\n[repeat: 3; gap: 1.25s]\n[speaker: A]\n你好。Hello.\n[pause: 0.5s]\n[speaker: B]\nGood morning.\n[audio: turn]\n[end-repeat]\n[pause: 5s]";
    let mut op = operation(SpeechVoiceSelection::Preset(preset()));
    let mut options = SpeechScriptOptions::default();
    for (role, index) in [("A", 0), ("B", 5)] {
        options.roles.insert(
            role.into(),
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(super::SPEECH_PRESETS[index].alias).unwrap(),
                super::INFER_SPEECH_VOICE_CATALOG_REVISION,
            )
            .unwrap(),
        );
    }
    op.script = Some(options);
    let mut request = request_text(source);
    request.instruction = serde_json::to_vec(&op).unwrap();
    let mut fake = FakeSdk::new();
    for i in 0..3 {
        let mut audio = response(i);
        audio.bytes[44..].fill(u8::try_from(i + 1).unwrap());
        let mut job = speech_job();
        job.id = format!("resp_segment_{i}");
        fake = fake.speech(audio).job(job);
    }
    let seen = fake.seen_speeches.clone();
    let output = InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
        .execute(&request)
        .unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        3,
        "three unique utterances, never seven generation requests"
    );
    assert!(seen.iter().all(|r| {
        r.instructions
            .as_ref()
            .is_some_and(|i| i.contains("educational listening"))
    }));
    assert_eq!(seen[1].voice, seen[0].voice);
    assert_ne!(seen[1].voice, seen[2].voice);
    assert_eq!(seen[1].language.as_deref(), Some("auto"));
    let p = output.external_provenance.unwrap();
    assert_eq!(p.speech_segments.len(), 3);
    let pieces = &p.speech_script.as_ref().unwrap().pieces;
    assert_eq!(
        pieces.iter().filter(|s| s.replay_of.is_some()).count(),
        8,
        "two replays of four pieces"
    );
    assert_eq!(
        pieces.iter().map(|p| p.frames).sum::<u64>(),
        240 + 3 * (480 + 12_000 + 3000) + 2 * 30_000 + 120_000
    );
    assert!(
        crate::speech_script::validate_output(source, &op, &request.inputs, &p, &output.bytes)
            .is_ok()
    );
    let index = pieces.iter().position(|s| s.replay_of.is_some()).unwrap();
    let offset = 44
        + pieces[..index]
            .iter()
            .map(|p| usize::try_from(p.frames).unwrap() * 2)
            .sum::<usize>();
    let count = usize::try_from(pieces[index].frames).unwrap() * 2;
    let mut bytes = output.bytes.clone();
    bytes[offset] ^= 1;
    let mut forged = p.clone();
    forged.speech_script.as_mut().unwrap().pieces[index].pcm_digest =
        ContentDigest::from_bytes(&bytes[offset..offset + count]);
    assert!(
        crate::speech_script::validate_output(source, &op, &request.inputs, &forged, &bytes)
            .is_err(),
        "even a rehashed changed replay is rejected"
    );
    op.script.as_mut().unwrap().roles.remove("B");
    request.instruction = serde_json::to_vec(&op).unwrap();
    let fake = FakeSdk::new();
    let seen = fake.seen_speeches.clone();
    assert!(
        InferRuntimeSpeechExecutor::with_sdk(Box::new(fake))
            .execute(&request)
            .is_err()
    );
    assert!(
        seen.lock().unwrap().is_empty(),
        "missing role fails before synthesis"
    );
}
