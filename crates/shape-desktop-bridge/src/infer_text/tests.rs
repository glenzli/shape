use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
    time::Duration,
};

use serde_json::json;
use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use uuid::Uuid;

use super::*;
use crate::open_desktop_session;

fn credential_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!("shape-bridge-credential-{}", Uuid::now_v7()))
        .join("infer-runtime.token")
}

#[test]
fn credential_status_and_install_expose_only_stable_non_secret_state() {
    let path = credential_path();
    let path_text = path.to_str().expect("portable path");
    let missing = infer_runtime_credential_status(path_text);
    assert!(!missing.configured);
    assert!(missing.error_code.is_empty());

    assert_eq!(
        install_infer_runtime_credential(path_text, "invalid"),
        Err("credential_invalid".to_owned())
    );
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    install_infer_runtime_credential(path_text, token).expect("credential installs");
    let configured = infer_runtime_credential_status(path_text);
    assert!(configured.configured);
    assert!(configured.error_code.is_empty());
    assert!(!format!("{configured:?}").contains(token));

    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout configures");
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("request reads");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let header_end = header_end + 4;
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .and_then(|value| value.parse::<usize>().ok())
            })
            .unwrap_or(0);
        if bytes.len() >= header_end + content_length {
            break;
        }
    }
    String::from_utf8(bytes).expect("request is UTF-8")
}

fn write_response(stream: &mut TcpStream, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("response writes");
}

fn fake_runtime() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake runtime binds");
    let address = listener.local_addr().expect("runtime has address");
    let worker = thread::spawn(move || {
        let (mut contract, _) = listener.accept().expect("contract connects");
        assert!(read_request(&mut contract).starts_with("GET /infer/v1/contract HTTP/1.1"));
        write_response(
            &mut contract,
            &json!({
                "contract_version": shape_execution::INFER_RUNTIME_CONTRACT_VERSION,
                "consumer_routes": [{"method": "POST", "path": "/v1/responses"}]
            })
            .to_string(),
        );

        let (mut responses, _) = listener.accept().expect("Responses request connects");
        assert!(read_request(&mut responses).starts_with("POST /v1/responses HTTP/1.1"));
        write_response(
            &mut responses,
            &json!({
                "id": "resp_bridge_test",
                "object": "response",
                "created_at": 1_786_383_600_u64,
                "model": "assistant.general",
                "status": "completed",
                "output": [{
                    "type": "message",
                    "content": [{"type": "output_text", "text": "A generated bridge candidate."}]
                }]
            })
            .to_string(),
        );
    });
    (format!("http://{address}"), worker)
}

#[test]
fn background_infer_result_adopts_as_transient_candidate_before_acceptance() {
    let project_path = std::env::temp_dir().join(format!("shape-bridge-infer-{}", Uuid::now_v7()));
    let mut project = ShapeProject::create(&project_path, "Infer bridge").expect("project creates");
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let initial = project
        .propose_text(
            artifact.id,
            None,
            "Accepted before Infer.",
            IntentSpec::new("Import text").expect("intent is valid"),
            Vec::new(),
        )
        .expect("initial candidate executes");
    project
        .accept_text(initial)
        .expect("initial candidate accepts");
    drop(project);

    let secret_path = credential_path();
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    InferRuntimeCredentialStore::new(&secret_path)
        .install(token)
        .expect("credential installs");
    let (origin, worker) = fake_runtime();
    let generated = generate_infer_text_candidate(
        project_path.to_str().expect("portable project path"),
        &artifact.id.to_string(),
        "Make it more vivid.",
        secret_path.to_str().expect("portable secret path"),
        &origin,
    )
    .expect("generation succeeds");
    worker.join().expect("fake runtime exits");

    let mut session = open_desktop_session(project_path.to_str().expect("portable project path"))
        .expect("session opens");
    let adopted = session
        .session_adopt_infer_text(generated)
        .expect("candidate adopts");
    assert_eq!(adopted.text_preview, "A generated bridge candidate.");
    assert_eq!(
        session
            .session_snapshot()
            .expect("snapshot reads")
            .artifacts[0]
            .text_preview,
        "Accepted before Infer."
    );
    let accepted = session
        .session_accept_candidate(&adopted.candidate_id)
        .expect("generated candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A generated bridge candidate."
    );
    assert_eq!(
        accepted.artifacts[0].transformation_kind_key,
        "generative_edit"
    );
    drop(session);
    fs::remove_dir_all(project_path).expect("fixture removes");
    fs::remove_dir_all(secret_path.parent().expect("secret has parent"))
        .expect("secret fixture removes");
}
