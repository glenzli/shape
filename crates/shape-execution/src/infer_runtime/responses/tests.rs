use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use uuid::Uuid;

use shape_domain::TransformationId;

use super::super::INFER_RUNTIME_CAPABILITY_SCALE_VERSION;
use super::*;
use crate::InferRuntimeCredentialStore;
use crate::{ExecutionCoordinator, ExecutionRequest};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn assert_contract_header(request: &str) {
    assert_eq!(
        request
            .matches(&format!(
                "infer-consumer-contract: {INFER_RUNTIME_CONTRACT_VERSION}\r\n"
            ))
            .count(),
        1
    );
}

fn read_request(stream: &mut TcpStream) -> String {
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
                    line.strip_prefix("content-length: ")
                        .or_else(|| line.strip_prefix("Content-Length: "))
                        .and_then(|value| value.parse::<usize>().ok())
                });
            }
            if bytes.len() >= header_end + content_length.unwrap_or(0) {
                break;
            }
        }
    }
    String::from_utf8(bytes).expect("request is UTF-8")
}

fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "Error" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("response writes");
}

fn executor_with_fake_runtime(
    response_status: u16,
    response_body: Value,
) -> (InferRuntimeExecutor, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake runtime binds");
    let address = listener.local_addr().expect("runtime has address");
    let worker = thread::spawn(move || {
        let (mut contract_stream, _) = listener.accept().expect("contract probe connects");
        let contract_request = read_request(&mut contract_stream);
        assert!(contract_request.starts_with("GET /infer/v1/contract HTTP/1.1\r\n"));
        assert_contract_header(&contract_request);
        write_response(
            &mut contract_stream,
            200,
            &json!({
                "contract_version": INFER_RUNTIME_CONTRACT_VERSION,
                "supported_contract_versions": [INFER_RUNTIME_CONTRACT_VERSION],
                "capability_scale_version": INFER_RUNTIME_CAPABILITY_SCALE_VERSION,
                "consumer_routes": [{"method": "POST", "path": "/v1/responses"}]
            })
            .to_string(),
        );

        let (mut response_stream, _) = listener.accept().expect("Responses request connects");
        let response_request = read_request(&mut response_stream);
        assert!(response_request.starts_with("POST /v1/responses HTTP/1.1\r\n"));
        assert_contract_header(&response_request);
        assert!(response_request.contains(&format!("authorization: Bearer {TOKEN}")));
        let body = response_request
            .split_once("\r\n\r\n")
            .expect("request has body")
            .1;
        let request: Value = serde_json::from_str(body).expect("request JSON parses");
        assert_eq!(request["model"], TEXT_INTENT);
        assert_eq!(request["input"], "Rewrite this paragraph clearly.");
        assert_eq!(request["stream"], false);
        assert_eq!(request["metadata"]["infer.policy"], "local-first");
        assert_eq!(request["metadata"]["infer.placement"], "local_only");
        assert_eq!(request["metadata"]["infer.offline_required"], "true");
        assert_eq!(request["metadata"]["infer.fallback"], "none");
        assert_eq!(request["metadata"]["infer.max_cost_usd"], "0");
        assert_eq!(
            request["metadata"]["infer.capability_floor"],
            "foundational"
        );
        assert_eq!(
            request["metadata"]["infer.deployment_ids"],
            "ollama_qwen3_5_4b"
        );
        let job_id = response_body
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        write_response(
            &mut response_stream,
            response_status,
            &response_body.to_string(),
        );
        if response_status == 200
            && response_body["model"] == TEXT_INTENT
            && response_body["output"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        {
            let job_id = job_id.expect("successful response has a Job id");
            let (mut job_stream, _) = listener.accept().expect("Job request connects");
            let job_request = read_request(&mut job_stream);
            assert!(job_request.starts_with(&format!("GET /infer/v1/jobs/{job_id} HTTP/1.1\r\n")));
            assert_contract_header(&job_request);
            assert!(job_request.contains(&format!("authorization: Bearer {TOKEN}")));
            write_response(
                &mut job_stream,
                200,
                &candidate_four_job(&job_id).to_string(),
            );
        }
    });
    let origin = format!("http://{address}");
    let credential = InferRuntimeCredential::from_test_token(TOKEN);
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        &origin,
        std::env::temp_dir(),
        "http://127.0.0.1:9",
    );
    (
        InferRuntimeExecutor::with_resolver(credential, resolver),
        worker,
    )
}

fn candidate_four_job(job_id: &str) -> Value {
    json!({
        "id": job_id,
        "app_id": "shape",
        "intent": "text.edit",
        "provider": "ollama-local",
        "deployment": "ollama_qwen3_5_4b",
        "model_profile": "qwen3_5_4b",
        "model_build": "qwen3_5_4b_q4_k_m",
        "physical_model": "qwen3.5:4b",
        "placement": "local",
        "capability_level": "foundational",
        "evaluation_status": "provisional",
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
            "latency": null,
            "fallback": "none",
            "max_cost_usd": 0.0,
            "deadline_ms": null,
            "capability_floor": "foundational",
            "named_route": {
                "kind": "deployment",
                "ordered_ids": ["ollama_qwen3_5_4b"]
            }
        },
        "routing": {
            "capability_floor": "foundational",
            "named_route": {
                "kind": "deployment",
                "ordered_ids": ["ollama_qwen3_5_4b"]
            },
            "candidates": [{
                "provider": "ollama-local",
                "deployment": "ollama_qwen3_5_4b",
                "status": "eligible",
                "rank": 1,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": "ollama-local",
            "deployment": "ollama_qwen3_5_4b",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }],
        "error": null
    })
}

fn request() -> ExecutionRequest {
    ExecutionRequest::new(
        TransformationId::new(),
        CapabilityId::new(TEXT_GENERATE_CAPABILITY).expect("capability is valid"),
        Vec::new(),
        b"Rewrite this paragraph clearly.".to_vec(),
        TEXT_MEDIA_TYPE,
    )
    .expect("request is valid")
}

#[test]
fn authenticated_local_first_response_becomes_transient_output_with_runtime_job_identity() {
    let (executor, worker) = executor_with_fake_runtime(
        200,
        json!({
            "id": "resp_shape_test",
            "object": "response",
            "created_at": 1_786_383_600_u64,
            "model": TEXT_INTENT,
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "A clearer paragraph."}]
            }],
            "future_field": true
        }),
    );
    let executed =
        ExecutionCoordinator::execute(&executor, &request()).expect("execution succeeds");
    assert_eq!(executed.output.bytes, b"A clearer paragraph.");
    assert_eq!(
        executed.receipt.executor_job_id.as_deref(),
        Some("resp_shape_test")
    );
    let provenance = executed
        .output
        .external_provenance
        .as_ref()
        .expect("candidate.4 text output carries verified Job provenance");
    assert_eq!(provenance.intent, "text.edit");
    assert_eq!(provenance.deployment, "ollama_qwen3_5_4b");
    assert_eq!(provenance.capability_floor, "foundational");
    assert_eq!(
        provenance
            .named_route
            .as_ref()
            .expect("named route is durable evidence")
            .ordered_ids,
        ["ollama_qwen3_5_4b"]
    );
    worker.join().expect("fake runtime exits");
}

#[test]
fn error_handling_uses_only_http_status_and_machine_code() {
    let (executor, worker) = executor_with_fake_runtime(
        403,
        json!({
            "error": {
                "message": format!("do not expose {TOKEN}"),
                "type": "invalid_request_error",
                "code": "route_target_forbidden"
            }
        }),
    );
    let error =
        ExecutionCoordinator::execute(&executor, &request()).expect_err("request is denied");
    let rendered = error.to_string();
    assert!(rendered.contains("route_target_forbidden"));
    assert!(!rendered.contains(TOKEN));
    assert!(!rendered.contains("do not expose"));
    worker.join().expect("fake runtime exits");
}

#[test]
fn malformed_or_empty_success_envelopes_fail_closed() {
    for response in [
        json!({
            "id": "resp_wrong_model",
            "object": "response",
            "created_at": 1,
            "model": "physical-model",
            "output": [{"type":"message","content":[{"type":"output_text","text":"bad"}]}]
        }),
        json!({
            "id": "resp_empty",
            "object": "response",
            "created_at": 1,
            "model": TEXT_INTENT,
            "output": []
        }),
    ] {
        let (executor, worker) = executor_with_fake_runtime(200, response);
        assert!(ExecutionCoordinator::execute(&executor, &request()).is_err());
        worker.join().expect("fake runtime exits");
    }
}

#[test]
fn current_text_request_uses_the_named_foundational_route() {
    let request = ResponsesRequest::local_text("Rewrite this paragraph clearly.");
    assert_eq!(request.model, TEXT_INTENT);
    assert_eq!(
        request.metadata.get("infer.capability_floor"),
        Some(&"foundational")
    );
    assert_eq!(
        request.metadata.get("infer.deployment_ids"),
        Some(&TEXT_DEPLOYMENT_ID)
    );
}

#[test]
fn credential_fixture_never_enters_the_worktree() {
    let path = std::env::temp_dir()
        .join(format!("shape-response-secret-{}", Uuid::now_v7()))
        .join("infer-runtime.token");
    let store = InferRuntimeCredentialStore::new(&path);
    store.install(TOKEN).expect("credential installs");
    assert!(store.is_available().expect("credential validates"));
    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}
