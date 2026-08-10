use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use super::{INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient, InferRuntimeClientError};

fn fake_runtime(status: u16, body: String) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("fake runtime binds");
    let address = listener.local_addr().expect("fake runtime has address");
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("probe connects");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout configures");
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /infer/v1/contract HTTP/1.1\r\n"));
        let reason = if status == 200 { "OK" } else { "Error" };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("response writes");
    });
    (format!("http://{address}"), worker)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 512];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut buffer).expect("request reads");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(bytes).expect("request is HTTP text")
}

fn compatible_manifest() -> String {
    format!(
        r#"{{
            "contract_version":"{INFER_RUNTIME_CONTRACT_VERSION}",
            "consumer_routes":[{{"method":"POST","path":"/v1/responses","future":true}}],
            "future_manifest_field":{{"enabled":true}}
        }}"#
    )
}

#[test]
fn compatible_contract_accepts_unknown_response_fields() {
    let (base_url, worker) = fake_runtime(200, compatible_manifest());
    let contract = InferRuntimeClient::new(&base_url)
        .expect("endpoint validates")
        .probe_contract()
        .expect("contract validates");
    assert_eq!(contract.contract_version, INFER_RUNTIME_CONTRACT_VERSION);
    worker.join().expect("fake runtime exits");
}

#[test]
fn contract_revision_and_required_route_fail_closed() {
    let incompatible = r#"{
        "contract_version":"0.1.0-candidate.99",
        "consumer_routes":[{"method":"POST","path":"/v1/responses"}]
    }"#;
    let (base_url, worker) = fake_runtime(200, incompatible.to_owned());
    assert_eq!(
        InferRuntimeClient::new(&base_url)
            .expect("endpoint validates")
            .probe_contract(),
        Err(InferRuntimeClientError::IncompatibleContract {
            actual: "0.1.0-candidate.99".to_owned()
        })
    );
    worker.join().expect("fake runtime exits");

    let missing_route = format!(
        r#"{{"contract_version":"{INFER_RUNTIME_CONTRACT_VERSION}","consumer_routes":[]}}"#
    );
    let (base_url, worker) = fake_runtime(200, missing_route);
    assert_eq!(
        InferRuntimeClient::new(&base_url)
            .expect("endpoint validates")
            .probe_contract(),
        Err(InferRuntimeClientError::InvalidContract)
    );
    worker.join().expect("fake runtime exits");
}

#[test]
fn probe_rejects_non_loopback_redirects_and_oversized_manifests() {
    for endpoint in [
        "https://127.0.0.1:8787",
        "http://example.com:8787",
        "http://127.0.0.1:8787/nested",
        "http://user@127.0.0.1:8787",
    ] {
        assert!(matches!(
            InferRuntimeClient::new(endpoint),
            Err(InferRuntimeClientError::InvalidEndpoint)
        ));
    }

    let oversized = " ".repeat(65 * 1024);
    let (base_url, worker) = fake_runtime(200, oversized);
    assert_eq!(
        InferRuntimeClient::new(&base_url)
            .expect("endpoint validates")
            .probe_contract(),
        Err(InferRuntimeClientError::InvalidContract)
    );
    worker.join().expect("fake runtime exits");
}

#[test]
fn http_and_transport_failures_have_stable_codes() {
    let (base_url, worker) = fake_runtime(503, "{}".to_owned());
    let error = InferRuntimeClient::new(&base_url)
        .expect("endpoint validates")
        .probe_contract()
        .expect_err("status fails");
    assert_eq!(error.code(), "unexpected_status");
    worker.join().expect("fake runtime exits");

    let listener = TcpListener::bind("127.0.0.1:0").expect("unused port binds");
    let base_url = format!(
        "http://{}",
        listener.local_addr().expect("unused port has address")
    );
    drop(listener);
    let error = InferRuntimeClient::new(&base_url)
        .expect("endpoint validates")
        .probe_contract()
        .expect_err("closed port fails");
    assert_eq!(error.code(), "unavailable");
}
