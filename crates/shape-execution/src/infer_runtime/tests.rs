use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

#[cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
#[cfg(unix)]
use uuid::Uuid;

use super::{
    INFER_RUNTIME_CAPABILITY_SCALE_VERSION, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION, InferRuntimeClient, InferRuntimeClientError,
    InferRuntimeEndpointResolver, InferRuntimeEndpointSource, ResolvedInferRuntimeEndpoint,
    probe_with_resolver, should_retry_endpoint,
};

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
            "capability_scale_version":"{INFER_RUNTIME_CAPABILITY_SCALE_VERSION}",
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
fn failed_probe_retries_only_a_distinct_transport_or_discovery_generation() {
    let failed = ResolvedInferRuntimeEndpoint {
        origin: "http://127.0.0.1:8787".to_owned(),
        source: InferRuntimeEndpointSource::Discovery,
        instance_id: Some("local".to_owned()),
        generation: Some("generation-a".to_owned()),
        lease_expires_at_unix: Some(1),
        contract_version: Some(INFER_RUNTIME_CONTRACT_VERSION.to_owned()),
    };
    let same_address_fallback = ResolvedInferRuntimeEndpoint {
        origin: failed.origin.clone(),
        source: InferRuntimeEndpointSource::CompatibilityFallback,
        instance_id: None,
        generation: None,
        lease_expires_at_unix: None,
        contract_version: None,
    };
    assert!(!should_retry_endpoint(&failed, &same_address_fallback));

    let new_generation = ResolvedInferRuntimeEndpoint {
        generation: Some("generation-b".to_owned()),
        ..failed.clone()
    };
    assert!(should_retry_endpoint(&failed, &new_generation));

    let migrated_contract = ResolvedInferRuntimeEndpoint {
        contract_version: Some(INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION.to_owned()),
        ..failed.clone()
    };
    assert!(should_retry_endpoint(&failed, &migrated_contract));

    let new_address_fallback = ResolvedInferRuntimeEndpoint {
        origin: "http://127.0.0.1:8788".to_owned(),
        ..same_address_fallback
    };
    assert!(should_retry_endpoint(&failed, &new_address_fallback));
}

#[cfg(unix)]
#[test]
fn resolved_probe_uses_the_discovered_generation_and_contract() {
    let (base_url, worker) = fake_runtime(200, compatible_manifest());
    let root = std::env::temp_dir().join(format!("shape-live-discovery-{}", Uuid::now_v7()));
    fs::create_dir_all(root.join("registrations")).expect("registration directory creates");
    fs::create_dir_all(root.join("sockets")).expect("socket directory creates");
    set_mode(&root, 0o700);
    set_mode(&root.join("registrations"), 0o700);
    set_mode(&root.join("sockets"), 0o700);
    let now = OffsetDateTime::now_utc();
    let registration = json!({
        "schema": "infra.discovery.registration",
        "schema_version": "20260810.1",
        "service": {
            "kind": "infer-runtime",
            "instance_id": "local",
            "generation": "generation-integration"
        },
        "lease": {
            "renewed_at": (now - TimeDuration::seconds(5)).format(&Rfc3339).expect("time formats"),
            "expires_at": (now + TimeDuration::seconds(40)).format(&Rfc3339).expect("time formats")
        },
        "offers": [{
            "protocol": "infer-runtime.consumer",
            "protocol_versions": [INFER_RUNTIME_CONTRACT_VERSION],
            "binding": "infer-runtime.http-loopback",
            "endpoint": base_url
        }]
    });
    let manifest = root.join("registrations/infer-runtime--local.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&registration).expect("registration serializes"),
    )
    .expect("registration writes");
    set_mode(&manifest, 0o600);
    let resolver =
        InferRuntimeEndpointResolver::with_runtime_root("", root.clone(), "http://127.0.0.1:9");

    let probe = probe_with_resolver(&resolver);
    let endpoint = probe.endpoint.expect("probe retains endpoint identity");
    assert_eq!(endpoint.source, InferRuntimeEndpointSource::Discovery);
    assert_eq!(
        endpoint.generation.as_deref(),
        Some("generation-integration")
    );
    assert_eq!(
        probe.contract.expect("contract validates").contract_version,
        INFER_RUNTIME_CONTRACT_VERSION
    );

    worker.join().expect("fake runtime exits");
    fs::remove_dir_all(root).expect("fixture removes");
}

#[cfg(unix)]
#[test]
fn discovered_offer_and_http_contract_must_match() {
    let candidate_two_manifest = format!(
        r#"{{"contract_version":"{INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION}","consumer_routes":[{{"method":"POST","path":"/v1/responses"}}]}}"#
    );
    let (base_url, worker) = fake_runtime(200, candidate_two_manifest);
    let root = std::env::temp_dir().join(format!("shape-version-mismatch-{}", Uuid::now_v7()));
    fs::create_dir_all(root.join("registrations")).expect("registration directory creates");
    fs::create_dir_all(root.join("sockets")).expect("socket directory creates");
    set_mode(&root, 0o700);
    set_mode(&root.join("registrations"), 0o700);
    set_mode(&root.join("sockets"), 0o700);
    let now = OffsetDateTime::now_utc();
    let registration = json!({
        "schema": "infra.discovery.registration",
        "schema_version": "20260810.1",
        "service": {
            "kind": "infer-runtime",
            "instance_id": "local",
            "generation": "generation-mismatch"
        },
        "lease": {
            "renewed_at": (now - TimeDuration::seconds(5)).format(&Rfc3339).unwrap(),
            "expires_at": (now + TimeDuration::seconds(40)).format(&Rfc3339).unwrap()
        },
        "offers": [{
            "protocol": "infer-runtime.consumer",
            "protocol_versions": [INFER_RUNTIME_CONTRACT_VERSION],
            "binding": "infer-runtime.http-loopback",
            "endpoint": base_url
        }]
    });
    let manifest = root.join("registrations/infer-runtime--local.json");
    fs::write(&manifest, serde_json::to_vec(&registration).unwrap()).unwrap();
    set_mode(&manifest, 0o600);
    let resolver =
        InferRuntimeEndpointResolver::with_runtime_root("", root.clone(), "http://127.0.0.1:9");

    let probe = probe_with_resolver(&resolver);
    assert_eq!(
        probe.contract,
        Err(InferRuntimeClientError::InvalidContract)
    );

    worker.join().expect("fake runtime exits");
    fs::remove_dir_all(root).expect("fixture removes");
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
        r#"{{"contract_version":"{INFER_RUNTIME_CONTRACT_VERSION}","capability_scale_version":"{INFER_RUNTIME_CAPABILITY_SCALE_VERSION}","consumer_routes":[]}}"#
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
fn previous_contract_remains_accepted_during_the_coordinated_migration() {
    let manifest = format!(
        r#"{{"contract_version":"{INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION}","consumer_routes":[{{"method":"POST","path":"/v1/responses"}}]}}"#
    );
    let (base_url, worker) = fake_runtime(200, manifest);
    let contract = InferRuntimeClient::new(&base_url)
        .expect("endpoint validates")
        .probe_contract()
        .expect("candidate.2 remains compatible");
    assert_eq!(
        contract.contract_version,
        INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION
    );
    worker.join().expect("fake runtime exits");
}

#[test]
fn candidate_three_requires_the_frozen_capability_scale_identity() {
    let manifest = format!(
        r#"{{"contract_version":"{INFER_RUNTIME_CONTRACT_VERSION}","consumer_routes":[{{"method":"POST","path":"/v1/responses"}}]}}"#
    );
    let (base_url, worker) = fake_runtime(200, manifest);
    assert_eq!(
        InferRuntimeClient::new(&base_url)
            .expect("endpoint validates")
            .probe_contract(),
        Err(InferRuntimeClientError::InvalidContract)
    );
    worker.join().expect("fake runtime exits");
}

#[test]
fn probe_rejects_noncanonical_origins_and_oversized_manifests() {
    for endpoint in [
        "https://127.0.0.1:8787",
        "http://localhost:8787",
        "http://example.com:8787",
        "http://127.0.0.1",
        "http://127.0.0.1:8787/",
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

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("fixture mode sets");
}
