use super::*;
#[test]
fn exact_authored_request_and_named_local_policy() {
    for (kind, model) in [
        (SoundGenerationKind::SoundEffect, SoundModelChoice::SmallSfx),
        (
            SoundGenerationKind::ShortMusic,
            SoundModelChoice::SmallMusic,
        ),
    ] {
        let op = SoundGenerationOperation::new("rain against a window", kind, 7, u32::MAX).unwrap();
        let r = sound_request(&op);
        assert_eq!(r.prompt, op.prompt);
        assert_eq!(r.duration_seconds, 7);
        assert_eq!(r.seed, Some(u32::MAX));
        assert_eq!(r.model_choice, Some(model));
        assert!(!r.metadata.contains_key("infer.deployment_ids"));
        assert_eq!(r.metadata["infer.fallback"], "none");
        assert_eq!(r.metadata["infer.placement"], "local_only");
    }
}

use super::super::{job_provenance::tests::text_job, test_support::FakeSdk};
use infer_runtime_client::{JobSnapshot, SoundGenerationResponse};
use serde_json::json;
fn job(kind: SoundGenerationKind) -> JobSnapshot {
    let mut j = text_job();
    let d = deployment(kind);
    j.intent = SOUND_INTENT.into();
    j.capability_contract = Some(INFER_RUNTIME_SOUND_GENERATION_CAPABILITY.into());
    j.deployment = d.into();
    j.model_build = d.into();
    j.physical_model = model_choice(kind).physical_model().unwrap().into();
    j.constraints["latency"] = json!("balanced");
    j.constraints["named_route"] = serde_json::Value::Null;
    j.routing.named_route = None;
    j.routing.candidates[0].deployment = d.into();
    j.attempts[0].deployment = d.into();
    j
}
fn wav() -> Vec<u8> {
    let data = 44_100_u32 * 4;
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend((36 + data).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16_u32.to_le_bytes());
    b.extend(1_u16.to_le_bytes());
    b.extend(2_u16.to_le_bytes());
    b.extend(44_100_u32.to_le_bytes());
    b.extend(176_400_u32.to_le_bytes());
    b.extend(4_u16.to_le_bytes());
    b.extend(16_u16.to_le_bytes());
    b.extend(b"data");
    b.extend(data.to_le_bytes());
    b.resize(b.len() + data as usize, 0);
    b
}
fn response(kind: SoundGenerationKind) -> SoundGenerationResponse {
    let j = job(kind);
    SoundGenerationResponse {
        wav: wav(),
        sha256: String::new(),
        job_id: j.id,
        logical_model: SOUND_INTENT.into(),
        model_choice: model_choice(kind),
        provider: j.provider,
        deployment: j.deployment,
        model_build: j.model_build,
        physical_model: j.physical_model,
        placement: j.placement,
        seed: 42,
        duration_seconds: 1,
    }
}
fn input(kind: SoundGenerationKind) -> ExecutionRequest {
    ExecutionRequest::new(
        shape_domain::TransformationId::new(),
        CapabilityId::new(AUDIO_GENERATE_CAPABILITY).unwrap(),
        vec![],
        serde_json::to_vec(&SoundGenerationOperation::new("rain", kind, 1, 42).unwrap()).unwrap(),
        "audio/wav",
    )
    .unwrap()
}
#[test]
fn both_sound_models_bind_response_headers_job_and_exact_wav() {
    for kind in [
        SoundGenerationKind::SoundEffect,
        SoundGenerationKind::ShortMusic,
    ] {
        let sdk = FakeSdk::new().job(job(kind));
        sdk.sounds.lock().unwrap().push_back(Ok(response(kind)));
        let executor =
            InferRuntimeSoundExecutor::with_sdk(Box::new(sdk), SoundGenerationControl::default());
        let output = executor.execute(&input(kind)).unwrap();
        assert_eq!(output.bytes, wav());
        assert_eq!(
            output
                .external_provenance
                .unwrap()
                .sound_prompt
                .unwrap()
                .effective_prompt,
            "rain"
        );
    }
}
#[test]
fn response_seed_duration_model_and_local_job_cannot_drift() {
    let kind = SoundGenerationKind::SoundEffect;
    let mutations: Vec<fn(&mut SoundGenerationResponse)> = vec![
        |r| r.seed = 43,
        |r| r.duration_seconds = 2,
        |r| r.model_choice = SoundModelChoice::SmallMusic,
        |r| r.wav.truncate(12),
        |r| r.job_id = "wrong".into(),
    ];
    for mutate in mutations {
        let mut r = response(kind);
        mutate(&mut r);
        let sdk = FakeSdk::new().job(job(kind));
        sdk.sounds.lock().unwrap().push_back(Ok(r));
        let executor =
            InferRuntimeSoundExecutor::with_sdk(Box::new(sdk), SoundGenerationControl::default());
        assert!(executor.execute(&input(kind)).is_err());
    }
    let mut j = job(kind);
    j.constraints["fallback"] = json!("allow");
    let sdk = FakeSdk::new().job(j);
    sdk.sounds.lock().unwrap().push_back(Ok(response(kind)));
    assert!(
        InferRuntimeSoundExecutor::with_sdk(Box::new(sdk), SoundGenerationControl::default())
            .execute(&input(kind))
            .is_err()
    );
}
#[test]
fn stop_prevents_io_and_prepared_prompt_is_cached_only_for_identical_original() {
    let control = SoundGenerationControl::default();
    control.cancel();
    let executor = InferRuntimeSoundExecutor::with_sdk(Box::new(FakeSdk::new()), control.clone());
    assert_eq!(
        executor
            .execute(&input(SoundGenerationKind::SoundEffect))
            .unwrap_err()
            .code,
        "generation_cancelled"
    );
    control.resume();
    let sdk = FakeSdk::new();
    let first = control.prepare(&sdk, "rain").unwrap();
    assert_eq!(first.effective_prompt, "rain");
    let second = control.prepare(&sdk, "wind").unwrap();
    assert_eq!(second.original_prompt, "wind");
    assert_eq!(
        control
            .0
            .prepared
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .original_prompt,
        "wind"
    );
}

fn mixed_preparation() -> PreparedSoundPrompt {
    let mut j = text_job();
    j.provider = "ollama-local".into();
    j.deployment = "ollama_qwen3_5_4b".into();
    j.model_build = "qwen3_5_4b_mlx".into();
    j.physical_model = "qwen3.5:4b-mlx".into();
    j.priority = "background".into();
    j.constraints["priority"] = json!("background");
    j.constraints["latency"] = json!("balanced");
    j.constraints["named_route"]["ordered_ids"] = json!([j.deployment]);
    j.routing.named_route.as_mut().unwrap().ordered_ids = vec![j.deployment.clone()];
    j.routing.candidates[0].provider.clone_from(&j.provider);
    j.routing.candidates[0].deployment.clone_from(&j.deployment);
    j.attempts[0].provider.clone_from(&j.provider);
    j.attempts[0].deployment.clone_from(&j.deployment);
    PreparedSoundPrompt {
        original_prompt: "轻柔的 guqin 和 sparse piano，舒缓节奏，不要人声，不要鼓点。".into(),
        effective_prompt: "Soft guqin and sparse piano, relaxed tempo, no vocals, no drum beats."
            .into(),
        rules_revision: infer_runtime_client::SOUND_PROMPT_RULES_REVISION.into(),
        text_job: Some(j),
        preparation_elapsed_ms: 1,
    }
}

#[test]
fn legacy_or_unfaithful_cache_is_prepared_again_and_current_cache_is_reused() {
    let current = mixed_preparation();
    let mut legacy = current.clone();
    legacy.rules_revision = "infer.sound-prompt-preparation@20260926.1".into();
    let mut unfaithful = current.clone();
    unfaithful.effective_prompt.push_str(" No music.");
    for old in [legacy, unfaithful] {
        assert!(old.validate_for(&current.original_prompt, "shape").is_ok());
        let control = SoundGenerationControl::default();
        *control.0.prepared.lock().unwrap() = Some(old);
        let sdk = FakeSdk::new();
        sdk.preparations.lock().unwrap().push_back(current.clone());
        let prepared = control.prepare(&sdk, &current.original_prompt).unwrap();
        assert_eq!(prepared.rules_revision, current.rules_revision);
        assert_eq!(prepared.effective_prompt, current.effective_prompt);
        control.resume();
        control.prepare(&sdk, &current.original_prompt).unwrap();
        assert_eq!(
            sdk.seen_preparations.lock().unwrap().as_slice(),
            std::slice::from_ref(&current.original_prompt)
        );
    }
}

#[test]
fn stale_or_invented_exclusions_from_sdk_never_reach_the_audio_request() {
    let current = mixed_preparation();
    let mut legacy = current.clone();
    legacy.rules_revision = "infer.sound-prompt-preparation@20260926.1".into();
    let mut unfaithful = current.clone();
    unfaithful.effective_prompt.push_str(" No music.");
    for bad in [legacy, unfaithful] {
        let sdk = FakeSdk::new();
        sdk.preparations.lock().unwrap().push_back(bad);
        // No audio response is queued: reaching generate_sound would panic.
        let executor =
            InferRuntimeSoundExecutor::with_sdk(Box::new(sdk), SoundGenerationControl::default());
        let mut request = input(SoundGenerationKind::ShortMusic);
        request.instruction = serde_json::to_vec(
            &SoundGenerationOperation::new(
                &current.original_prompt,
                SoundGenerationKind::ShortMusic,
                1,
                42,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            executor.execute(&request).unwrap_err().code,
            "prompt_preparation_failed"
        );
    }
}

#[test]
fn durable_prompt_separates_history_reading_from_new_acceptance() {
    use crate::sound_prompt::SoundPromptProvenance;
    let current = mixed_preparation();
    let operation = SoundGenerationOperation::new(
        &current.original_prompt,
        SoundGenerationKind::ShortMusic,
        1,
        42,
    )
    .unwrap();
    let p = SoundPromptProvenance::from_prepared(current.clone(), &operation).unwrap();
    assert!(p.valid_for_generation(&operation.prompt));
    for legacy in [false, true] {
        let mut stale = current.clone();
        if legacy {
            stale.rules_revision = "infer.sound-prompt-preparation@20260926.1".into();
        } else {
            stale.effective_prompt.push_str(" No music.");
        }
        assert!(SoundPromptProvenance::from_prepared(stale.clone(), &operation).is_none());
        let mut recorded = p.clone();
        recorded.rules_revision = stale.rules_revision;
        recorded.effective_prompt = stale.effective_prompt;
        let restored: SoundPromptProvenance =
            serde_json::from_slice(&serde_json::to_vec(&recorded).unwrap()).unwrap();
        assert!(restored.valid_for(&operation.prompt));
        assert!(!restored.valid_for_generation(&operation.prompt));
    }
    let other =
        SoundGenerationOperation::new("Different", SoundGenerationKind::ShortMusic, 1, 42).unwrap();
    assert!(SoundPromptProvenance::from_prepared(current, &other).is_none());
}
