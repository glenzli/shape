use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, AuthorizedVoiceReference, ContentDigest,
    ContentRef, PresetVoiceAlias, PresetVoiceSelection, RevisionId, SpeechSynthesisOperation,
    SpeechVoiceSelection, TransformationId, VoiceAuthorizationScope,
};

use super::super::INFER_RUNTIME_CAPABILITY_SCALE_VERSION;
use super::*;
use crate::{ExecutionCoordinator, ExecutionInput, ExecutionRequest};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout configures");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut content_length = None;
    loop {
        let read = stream.read(&mut buffer).expect("request reads");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_end = header_end + 4;
            if content_length.is_none() {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = headers.lines().find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                });
            }
            if bytes.len() >= header_end + content_length.unwrap_or(0) {
                break;
            }
        }
    }
    bytes
}

fn write_json(stream: &mut TcpStream, status: u16, body: &Value) {
    let body = body.to_string();
    let reason = if status == 200 { "OK" } else { "Error" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("JSON response writes");
}

fn write_wav(stream: &mut TcpStream, job_id: &str, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nX-Infer-Job-Id: {job_id}\r\nX-Infer-Model: {SPEECH_INTENT}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("WAV headers write");
    stream.write_all(body).expect("WAV body writes");
}

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

fn compatible_contract(revision: InferRuntimeContractRevision) -> Value {
    json!({
        "contract_version": revision.as_str(),
        "capability_scale_version": INFER_RUNTIME_CAPABILITY_SCALE_VERSION,
        "consumer_routes": [{"method": "POST", "path": SPEECH_ROUTE}]
    })
}

fn succeeded_job(job_id: &str, revision: InferRuntimeContractRevision) -> Value {
    let mut job = json!({
        "id": job_id,
        "app_id": "shape",
        "intent": SPEECH_INTENT,
        "provider": "mlx-audio",
        "deployment": "qwen3-tts-custom-voice-local",
        "model_profile": "qwen3-tts-custom-voice",
        "model_build": "qwen3_tts_custom_voice_1_7b_8bit",
        "physical_model": "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
        "placement": "local",
        "resource_class": "standard",
        "state": "succeeded",
        "policy": "local-first",
        "priority": "interactive",
        "constraints": {
            "policy": "local-first",
            "priority": "interactive",
            "provider_access_class": null,
            "placement": "local_only",
            "prefer": "local",
            "offline_required": true,
            "latency": "interactive",
            "fallback": "none",
            "max_cost_usd": 0.0,
            "deadline_ms": null
        },
        "routing": {
            "candidates": [{
                "provider": "mlx-audio",
                "deployment": "qwen3-tts-custom-voice-local",
                "status": "eligible",
                "rank": 1,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": "mlx-audio",
            "deployment": "qwen3-tts-custom-voice-local",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }],
        "error": null
    });
    let (level_key, status_key) = match revision {
        InferRuntimeContractRevision::Candidate2 => ("quality_grade", "rating_status"),
        InferRuntimeContractRevision::Candidate3 => ("capability_level", "evaluation_status"),
    };
    job[level_key] = json!(revision.capable_level());
    job[status_key] = json!("provisional");
    job["constraints"][revision.job_capability_floor_key()] = json!(revision.capable_level());
    job["routing"][revision.job_capability_floor_key()] = json!(revision.capable_level());
    job
}

fn operation(voice: SpeechVoiceSelection) -> SpeechSynthesisOperation {
    SpeechSynthesisOperation::new(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        voice,
        1_000,
        true,
    )
    .expect("operation is valid")
}

fn request(operation: &SpeechSynthesisOperation) -> ExecutionRequest {
    let text = "这是一条普通的合成旁白。".as_bytes().to_vec();
    let content = ContentRef::new(
        ContentDigest::from_bytes(&text),
        "text/plain; charset=utf-8",
        text.len() as u64,
    )
    .expect("text content is valid");
    ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new(AUDIO_SPEECH_SYNTHESIZE_CAPABILITY).expect("capability is valid"),
        vec![ExecutionInput::materialized(content, text).expect("text bytes verify")],
        serde_json::to_vec(operation).expect("operation serializes"),
        AUDIO_MEDIA_TYPE,
    )
    .expect("request is valid")
}

fn executor(origin: &str) -> InferRuntimeSpeechExecutor {
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        origin,
        std::env::temp_dir(),
        "http://127.0.0.1:9",
    );
    InferRuntimeSpeechExecutor::with_resolver(
        InferRuntimeCredential::from_test_token(TOKEN),
        resolver,
    )
}

#[test]
fn unary_wav_becomes_typed_transient_candidate_with_runtime_job_provenance() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake runtime binds");
    let address = listener.local_addr().expect("runtime has address");
    let expected_wav = wav(24_000, 1, 24_000);
    let served_wav = expected_wav.clone();
    let worker = thread::spawn(move || {
        let (mut contract_stream, _) = listener.accept().expect("contract probe connects");
        let contract_request = String::from_utf8(read_request(&mut contract_stream)).unwrap();
        assert!(contract_request.starts_with("GET /infer/v1/contract HTTP/1.1\r\n"));
        write_json(
            &mut contract_stream,
            200,
            &compatible_contract(InferRuntimeContractRevision::Candidate3),
        );

        let (mut speech_stream, _) = listener.accept().expect("speech request connects");
        let speech_request = read_request(&mut speech_stream);
        let request_text = String::from_utf8_lossy(&speech_request);
        assert!(request_text.starts_with("POST /v1/audio/speech HTTP/1.1\r\n"));
        assert!(request_text.contains(&format!("authorization: Bearer {TOKEN}")));
        let body = speech_request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| &speech_request[index + 4..])
            .expect("speech request has body");
        let request: Value = serde_json::from_slice(body).expect("speech request parses");
        assert_eq!(request["model"], SPEECH_INTENT);
        assert_eq!(request["voice"], INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1);
        assert_eq!(
            request["language"],
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE
        );
        assert_eq!(request["response_format"], "wav");
        assert_eq!(request["execution_mode"], "unary");
        assert_eq!(request["metadata"]["infer.placement"], "local_only");
        assert_eq!(request["metadata"]["infer.offline_required"], "true");
        assert_eq!(request["metadata"]["infer.latency"], "interactive");
        assert_eq!(request["metadata"]["infer.fallback"], "none");
        assert_eq!(request["metadata"]["infer.max_cost_usd"], "0");
        assert_eq!(request["metadata"]["infer.capability_floor"], "capable");
        assert!(request["metadata"].get("infer.quality_floor").is_none());
        write_wav(&mut speech_stream, "job_shape_speech_1", &served_wav);

        let (mut job_stream, _) = listener.accept().expect("Job request connects");
        let job_request = String::from_utf8(read_request(&mut job_stream)).unwrap();
        assert!(job_request.starts_with("GET /infer/v1/jobs/job_shape_speech_1 HTTP/1.1\r\n"));
        assert!(job_request.contains(&format!("authorization: Bearer {TOKEN}")));
        write_json(
            &mut job_stream,
            200,
            &succeeded_job(
                "job_shape_speech_1",
                InferRuntimeContractRevision::Candidate3,
            ),
        );
    });

    let voice = PresetVoiceSelection::new(
        PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1).unwrap(),
        INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    )
    .unwrap();
    let executed = ExecutionCoordinator::execute(
        &executor(&format!("http://{address}")),
        &request(&operation(SpeechVoiceSelection::Preset(voice))),
    )
    .expect("speech execution succeeds");

    assert_eq!(executed.output.bytes, expected_wav);
    assert_eq!(
        ContentDigest::from_bytes(&executed.output.bytes),
        ContentDigest::from_bytes(&expected_wav)
    );
    let ArtifactContentContract::AudioClip(contract) =
        executed.output.content_contract.expect("audio is typed")
    else {
        panic!("speech output must be an audio clip");
    };
    assert_eq!(contract.sample_rate_hz, 24_000);
    assert_eq!(contract.channels, 1);
    assert_eq!(contract.duration_millis(), 1_000);
    assert_eq!(contract.origin, AudioOriginDisclosure::SyntheticSpeech);
    let provenance = executed
        .output
        .external_provenance
        .expect("Job provenance is copied");
    assert_eq!(provenance.intent, SPEECH_INTENT);
    assert_eq!(provenance.model_build, "qwen3_tts_custom_voice_1_7b_8bit");
    assert_eq!(provenance.fallback, "none");
    assert_eq!(provenance.requested_policy, "local-first");
    assert_eq!(provenance.requested_priority, "interactive");
    assert_eq!(provenance.requested_provider_access_class, None);
    assert_eq!(provenance.requested_preference, "local");
    assert_eq!(provenance.requested_latency.as_deref(), Some("interactive"));
    assert_eq!(provenance.requested_deadline_ms, None);
    assert!(provenance.offline_required);
    assert_eq!(provenance.attempts.len(), 1);
    worker.join().expect("fake runtime exits");
}

#[test]
fn authorization_and_runtime_error_paths_fail_closed_without_payload_leaks() {
    let authorized = AuthorizedVoiceReference::new(
        RevisionId::new(),
        "consent.receipt.local.1",
        VoiceAuthorizationScope::SpeechSynthesis,
        true,
        true,
    )
    .unwrap();
    let rejected = executor("http://127.0.0.1:9")
        .execute(&request(&operation(
            SpeechVoiceSelection::AuthorizedReference(authorized),
        )))
        .expect_err("authorized voice references are not executable yet");
    assert_eq!(rejected.code, "voice_reference_not_supported");

    let listener = TcpListener::bind("127.0.0.1:0").expect("fake runtime binds");
    let address = listener.local_addr().expect("runtime has address");
    let worker = thread::spawn(move || {
        let (mut contract_stream, _) = listener.accept().expect("contract probe connects");
        let _ = read_request(&mut contract_stream);
        write_json(
            &mut contract_stream,
            200,
            &compatible_contract(InferRuntimeContractRevision::Candidate3),
        );

        let (mut speech_stream, _) = listener.accept().expect("speech request connects");
        let _ = read_request(&mut speech_stream);
        write_json(
            &mut speech_stream,
            403,
            &json!({"error": {"code": "intent_forbidden", "message": TOKEN}}),
        );
    });
    let voice = PresetVoiceSelection::new(
        PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1).unwrap(),
        INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    )
    .unwrap();
    let error = ExecutionCoordinator::execute(
        &executor(&format!("http://{address}")),
        &request(&operation(SpeechVoiceSelection::Preset(voice))),
    )
    .expect_err("ACL denial fails execution");
    let rendered = error.to_string();
    assert!(rendered.contains("intent_forbidden"));
    assert!(!rendered.contains(TOKEN));
    worker.join().expect("fake runtime exits");
}

#[test]
fn job_readback_rejects_fallback_that_would_violate_the_shape_request() {
    let mut job = succeeded_job(
        "job_policy_violation",
        InferRuntimeContractRevision::Candidate3,
    );
    job["attempts"][0]["trigger"] = Value::String("fallback".to_owned());
    let bytes = serde_json::to_vec(&job).unwrap();
    let error = parse_job_snapshot(
        InferRuntimeContractRevision::Candidate3,
        "job_policy_violation",
        &bytes,
    )
    .expect_err("no-fallback execution rejects a fallback Attempt");
    assert_eq!(error.failure.code, "infer_policy_violation");
}

#[test]
fn candidate_two_job_provenance_remains_readable_during_migration() {
    let revision = InferRuntimeContractRevision::Candidate2;
    let bytes = serde_json::to_vec(&succeeded_job("job_candidate_two", revision)).unwrap();
    let provenance = parse_job_snapshot(revision, "job_candidate_two", &bytes)
        .unwrap_or_else(|_| panic!("candidate.2 provenance remains valid"));
    assert_eq!(provenance.contract_revision, revision.as_str());
    assert_eq!(provenance.capability_level, "general");
    assert_eq!(provenance.evaluation_status, "provisional");
    assert_eq!(provenance.capability_floor, "general");
}

#[test]
fn candidate_two_speech_request_uses_only_the_candidate_two_floor() {
    let revision = InferRuntimeContractRevision::Candidate2;
    let request = SpeechRequest::local_unary(
        "migration",
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        1_000,
        revision,
    );
    let request = serde_json::to_value(request).expect("speech request serializes");
    assert_eq!(request["model"], SPEECH_INTENT);
    assert_eq!(request["metadata"]["infer.quality_floor"], "general");
    assert!(request["metadata"].get("infer.capability_floor").is_none());
}

#[test]
fn job_provenance_rejects_vocabulary_that_does_not_match_the_contract() {
    let bytes = serde_json::to_vec(&succeeded_job(
        "job_wrong_vocabulary",
        InferRuntimeContractRevision::Candidate2,
    ))
    .unwrap();
    let error = parse_job_snapshot(
        InferRuntimeContractRevision::Candidate3,
        "job_wrong_vocabulary",
        &bytes,
    )
    .expect_err("candidate.2 fields cannot be decoded as candidate.3");
    assert_eq!(error.failure.code, "infer_invalid_response");
}

#[test]
fn runtime_job_identity_cannot_rewrite_the_authenticated_readback_path() {
    assert!(valid_job_id("job_shape_speech_1"));
    assert!(!valid_job_id("../jobs/operator"));
    assert!(!valid_job_id("job?redirect=elsewhere"));
    assert!(!valid_job_id("job%2fsecret"));
}
