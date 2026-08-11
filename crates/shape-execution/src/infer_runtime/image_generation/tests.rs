use std::{
    io::{Cursor, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::{Value, json};
use shape_domain::{AiImageOutputCanvas, ArtifactContentContract, TransformationId};

use super::super::INFER_RUNTIME_CAPABILITY_SCALE_VERSION;
use super::*;
use crate::{ExecutionCoordinator, ExecutionRequest};

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

fn png(width: u32, height: u32, color_type: ColorType) -> Vec<u8> {
    let pixels =
        vec![127_u8; width as usize * height as usize * color_type.channel_count() as usize];
    let mut bytes = Cursor::new(Vec::new());
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, width, height, color_type.into())
        .unwrap();
    bytes.into_inner()
}

fn parameters(width: u32, height: u32, count: u8) -> AiImageGenerateParameters {
    AiImageGenerateParameters::new(
        "A pale blue circle on white, without text",
        AiImageOutputCanvas::new(width, height).unwrap(),
        count,
        Vec::new(),
    )
    .unwrap()
}

fn request(parameters: &AiImageGenerateParameters) -> ExecutionRequest {
    ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(IMAGE_GENERATE_CAPABILITY).unwrap(),
        Vec::new(),
        serde_json::to_vec(parameters).unwrap(),
        IMAGE_MEDIA_TYPE,
    )
    .unwrap()
}

fn executor(origin: &str) -> InferRuntimeImageGenerationExecutor {
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        origin,
        std::env::temp_dir(),
        "http://127.0.0.1:9",
    );
    InferRuntimeImageGenerationExecutor::with_resolver(
        InferRuntimeCredential::from_test_token(TOKEN),
        resolver,
    )
}

fn contract(revision: InferRuntimeContractRevision) -> Value {
    json!({
        "contract_version": revision.as_str(),
        "capability_scale_version": INFER_RUNTIME_CAPABILITY_SCALE_VERSION,
        "consumer_routes": [{"method": "POST", "path": RESPONSES_ROUTE}]
    })
}

fn image_response(job_id: &str, encoded: &str) -> Value {
    json!({
        "id": job_id,
        "object": "response",
        "created_at": 1_786_456_800,
        "model": IMAGE_INTENT,
        "status": "completed",
        "output": [{
            "id": "image_1",
            "type": "image_generation_call",
            "status": "completed",
            "result": encoded,
            "revised_prompt": "a pale blue circle",
            "future_item_field": true
        }],
        "future_envelope_field": {"ignored": true}
    })
}

fn succeeded_job(job_id: &str) -> Value {
    json!({
        "id": job_id,
        "app_id": "shape",
        "intent": IMAGE_INTENT,
        "provider": "codex-subscription",
        "deployment": "codex_gpt_5_6_luna",
        "model_profile": "codex_gpt_5_6_luna",
        "model_build": "codex_gpt_5_6_luna_subscription",
        "physical_model": "gpt-5.6-luna",
        "placement": "cloud",
        "capability_level": "advanced",
        "evaluation_status": "provisional",
        "resource_class": "standard",
        "state": "succeeded",
        "policy": "balanced",
        "priority": "interactive",
        "constraints": {
            "policy": "balanced",
            "priority": "interactive",
            "provider_access_class": "subscription",
            "placement": "cloud_only",
            "prefer": "cloud",
            "offline_required": false,
            "latency": null,
            "fallback": "none",
            "max_cost_usd": 0.0,
            "deadline_ms": null,
            "capability_floor": "capable"
        },
        "routing": {
            "capability_floor": "capable",
            "candidates": [{
                "provider": "codex-subscription",
                "deployment": "codex_gpt_5_6_luna",
                "status": "eligible",
                "rank": 1,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": "codex-subscription",
            "deployment": "codex_gpt_5_6_luna",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }],
        "error": null
    })
}

#[test]
fn exact_candidate_three_request_returns_one_canonical_transient_raster() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let source_png = png(4, 3, ColorType::Rgb8);
    let encoded = STANDARD.encode(&source_png);
    let worker = thread::spawn(move || {
        let (mut contract_stream, _) = listener.accept().unwrap();
        let contract_request = String::from_utf8(read_request(&mut contract_stream)).unwrap();
        assert!(contract_request.starts_with("GET /infer/v1/contract HTTP/1.1\r\n"));
        write_json(
            &mut contract_stream,
            200,
            &contract(InferRuntimeContractRevision::Candidate3),
        );

        let (mut image_stream, _) = listener.accept().unwrap();
        let image_request = read_request(&mut image_stream);
        let request_text = String::from_utf8_lossy(&image_request);
        assert!(request_text.starts_with("POST /v1/responses HTTP/1.1\r\n"));
        assert!(request_text.contains(&format!("authorization: Bearer {TOKEN}")));
        let body = image_request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| &image_request[index + 4..])
            .unwrap();
        let request: Value = serde_json::from_slice(body).unwrap();
        assert_eq!(
            request
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            [
                "background",
                "input",
                "metadata",
                "model",
                "stream",
                "tools"
            ]
        );
        assert_eq!(request["model"], IMAGE_INTENT);
        assert!(request["input"].as_str().unwrap().contains("blue circle"));
        assert_eq!(request["tools"], json!([{"type": "image_generation"}]));
        assert_eq!(request["stream"], false);
        assert_eq!(request["background"], false);
        assert_eq!(
            request["metadata"],
            json!({
                "infer.capability_floor": "capable",
                "infer.fallback": "none",
                "infer.max_cost_usd": "0",
                "infer.placement": "cloud_only",
                "infer.policy": "balanced",
                "infer.prefer": "cloud",
                "infer.priority": "interactive",
                "infer.provider_access_class": "subscription"
            })
        );
        assert!(request["metadata"].get("infer.offline_required").is_none());
        write_json(
            &mut image_stream,
            200,
            &image_response("resp_shape_image_1", &encoded),
        );

        let (mut job_stream, _) = listener.accept().unwrap();
        let job_request = String::from_utf8(read_request(&mut job_stream)).unwrap();
        assert!(job_request.starts_with("GET /infer/v1/jobs/resp_shape_image_1 HTTP/1.1\r\n"));
        assert!(job_request.contains(&format!("authorization: Bearer {TOKEN}")));
        write_json(&mut job_stream, 200, &succeeded_job("resp_shape_image_1"));
    });

    let executed = ExecutionCoordinator::execute(
        &executor(&format!("http://{address}")),
        &request(&parameters(4, 3, 1)),
    )
    .unwrap();
    let decoded = image::load_from_memory(&executed.output.bytes)
        .unwrap()
        .to_rgba8();
    assert_eq!((decoded.width(), decoded.height()), (4, 3));
    let Some(ArtifactContentContract::ImageRaster(contract)) = executed.output.content_contract
    else {
        panic!("generated image must be typed");
    };
    assert_eq!((contract.width, contract.height), (4, 3));
    assert_eq!(
        executed.output.executor_job_id.as_deref(),
        Some("resp_shape_image_1")
    );
    assert_eq!(
        executed.output.external_provenance.unwrap().provider,
        "codex-subscription"
    );
    worker.join().unwrap();
}

#[test]
fn candidate_two_and_multi_candidate_requests_fail_before_generation() {
    let unsupported = executor("http://127.0.0.1:9")
        .execute(&request(&parameters(4, 3, 2)))
        .unwrap_err();
    assert_eq!(unsupported.code, "unsupported_image_candidate_count");

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        write_json(
            &mut stream,
            200,
            &contract(InferRuntimeContractRevision::Candidate2),
        );
    });
    let failure = executor(&format!("http://{address}"))
        .execute(&request(&parameters(4, 3, 1)))
        .unwrap_err();
    assert_eq!(failure.code, "infer_incompatible_contract");
    worker.join().unwrap();
}

#[test]
fn response_shape_base64_png_and_geometry_are_revalidated() {
    let valid_encoded = STANDARD.encode(png(4, 3, ColorType::Rgb8));
    let valid = image_response("resp_shape_image_2", &valid_encoded);
    let parsed =
        parse_image_response(&serde_json::to_vec(&valid).unwrap(), &parameters(4, 3, 1)).unwrap();
    assert_eq!((parsed.contract.width, parsed.contract.height), (4, 3));

    let mut invalid_cases = Vec::new();
    let mut wrong_id = valid.clone();
    wrong_id["id"] = json!("job_not_response");
    invalid_cases.push(wrong_id);
    let mut wrong_model = valid.clone();
    wrong_model["model"] = json!("language.respond");
    invalid_cases.push(wrong_model);
    let mut empty = valid.clone();
    empty["output"] = json!([]);
    invalid_cases.push(empty);
    let mut two = valid.clone();
    two["output"] = json!([valid["output"][0].clone(), valid["output"][0].clone()]);
    invalid_cases.push(two);
    let mut wrong_type = valid.clone();
    wrong_type["output"][0]["type"] = json!("message");
    invalid_cases.push(wrong_type);
    let mut bad_base64 = valid.clone();
    bad_base64["output"][0]["result"] = json!("not base64!");
    invalid_cases.push(bad_base64);
    let mut not_png = valid.clone();
    not_png["output"][0]["result"] = json!(STANDARD.encode(b"private prompt"));
    invalid_cases.push(not_png);
    for invalid in invalid_cases {
        let error =
            parse_image_response(&serde_json::to_vec(&invalid).unwrap(), &parameters(4, 3, 1))
                .unwrap_err();
        assert!(!error.failure.to_string().contains("private prompt"));
        assert!(!error.failure.to_string().contains(TOKEN));
    }

    let geometry_encoded = STANDARD.encode(png(3, 3, ColorType::Rgba8));
    let geometry = image_response("resp_shape_image_geometry", &geometry_encoded);
    assert_eq!(
        parse_image_response(
            &serde_json::to_vec(&geometry).unwrap(),
            &parameters(4, 3, 1)
        )
        .unwrap_err()
        .failure
        .code,
        "generated_image_geometry_mismatch"
    );

    let oversized_encoded = STANDARD.encode(png(4097, 1, ColorType::Rgba8));
    let oversized_dimension = image_response("resp_shape_image_large", &oversized_encoded);
    assert_eq!(
        parse_image_response(
            &serde_json::to_vec(&oversized_dimension).unwrap(),
            &parameters(4097, 1, 1)
        )
        .unwrap_err()
        .failure
        .code,
        "invalid_generated_image"
    );
}

#[test]
fn status_and_error_code_are_the_only_error_branch_inputs() {
    for (status, code, retryable) in [
        (400, "invalid_request_error", false),
        (401, "invalid_api_key", false),
        (403, "intent_forbidden", false),
        (403, "policy_violation", false),
        (409, "no_candidate", false),
        (429, "quota_exceeded", false),
        (429, "upstream_rate_limited", true),
        (502, "upstream_protocol", false),
        (503, "provider_unavailable", true),
        (504, "upstream_timeout", true),
    ] {
        let body = serde_json::to_vec(&json!({
            "error": {"code": code, "message": TOKEN}
        }))
        .unwrap();
        let error = error_response(status, &body);
        assert_eq!(error.failure.code, code);
        assert_eq!(error.failure.retryable, retryable);
        assert!(!error.failure.to_string().contains(TOKEN));
    }
    let mismatch = error_response(
        401,
        &serde_json::to_vec(&json!({"error": {"code": "intent_forbidden"}})).unwrap(),
    );
    assert_eq!(mismatch.failure.code, "infer_http_401");
}

#[test]
fn debug_output_redacts_credential_and_response_budget_covers_one_runtime_image() {
    let rendered = format!("{:?}", executor("http://127.0.0.1:9"));
    assert!(!rendered.contains(TOKEN));
    assert!(rendered.contains("[REDACTED]"));
    const { assert!(MAX_RESPONSE_BYTES > MAX_BASE64_CHARS) };
    assert_eq!(MAX_RESPONSE_BYTES, 32 * 1024 * 1024);
}
