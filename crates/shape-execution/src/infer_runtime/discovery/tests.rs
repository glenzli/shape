#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use serde_json::json;
use uuid::Uuid;

use super::*;

struct DiscoveryFixture {
    root: PathBuf,
}

impl DiscoveryFixture {
    fn new(label: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("shape-infer-discovery-{label}-{}", Uuid::now_v7()));
        fs::create_dir_all(root.join("registrations")).expect("registration directory creates");
        fs::create_dir_all(root.join("sockets")).expect("socket directory creates");
        set_mode(&root, 0o700);
        set_mode(&root.join("registrations"), 0o700);
        set_mode(&root.join("sockets"), 0o700);
        Self { root }
    }

    fn manifest(&self) -> PathBuf {
        self.root.join("registrations").join(MANIFEST_FILENAME)
    }

    fn write_registration(&self, generation: &str, endpoint: &str, version: &str) {
        self.write_value(&registration_value(generation, endpoint, version));
    }

    fn write_value(&self, value: &serde_json::Value) {
        fs::write(
            self.manifest(),
            serde_json::to_vec_pretty(value).expect("registration serializes"),
        )
        .expect("registration writes");
        set_mode(&self.manifest(), 0o600);
    }
}

impl Drop for DiscoveryFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("discovery fixture removes");
    }
}

#[test]
fn explicit_override_wins_and_remains_strict_numeric_loopback() {
    let fixture = DiscoveryFixture::new("override");
    fixture.write_registration(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "http://127.0.0.1:9222",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    let endpoint = resolver.resolve_current().expect("override resolves");
    assert_eq!(endpoint.origin, "http://127.0.0.1:9222");
    assert_eq!(
        endpoint.source,
        InferRuntimeEndpointSource::ExplicitOverride
    );

    let invalid = InferRuntimeEndpointResolver::with_runtime_root(
        "http://localhost:9222",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );
    assert_eq!(
        invalid.resolve_current(),
        Err(InferRuntimeClientError::InvalidEndpoint)
    );
}

#[test]
fn discovery_tracks_stable_generation_without_a_manifest_clock() {
    let fixture = DiscoveryFixture::new("generation");
    fixture.write_registration(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    let first = resolver
        .resolve_current()
        .expect("first generation resolves");
    assert_eq!(first.source, InferRuntimeEndpointSource::Discovery);
    assert_eq!(first.instance_id.as_deref(), Some("local"));
    assert_eq!(first.generation.as_deref(), Some("generation-a"));
    assert_eq!(
        first.contract_version.as_deref(),
        Some(INFER_RUNTIME_CONTRACT_VERSION)
    );

    fixture.write_registration(
        "generation-b",
        "http://127.0.0.1:9222",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let second = resolver
        .resolve_current()
        .expect("second generation resolves");
    assert_eq!(second.origin, "http://127.0.0.1:9222");
    assert_eq!(second.generation.as_deref(), Some("generation-b"));

    let unchanged = resolver
        .resolve_current()
        .expect("stable declaration remains selectable without a clock");
    assert_eq!(unchanged, second);
}

#[test]
fn connection_failure_retries_only_new_identity_then_uses_fallback() {
    let fixture = DiscoveryFixture::new("failure");
    fixture.write_registration(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );
    let failed = resolver
        .resolve_current()
        .expect("initial endpoint resolves");

    fixture.write_registration(
        "generation-b",
        "http://127.0.0.1:9222",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let rediscovered = resolver
        .resolve_after_connection_failure(&failed)
        .expect("new generation resolves");
    assert_eq!(rediscovered.source, InferRuntimeEndpointSource::Discovery);
    assert_eq!(rediscovered.generation.as_deref(), Some("generation-b"));

    let unchanged = resolver
        .resolve_after_connection_failure(&rediscovered)
        .expect("unchanged generation falls back");
    assert_eq!(
        unchanged.source,
        InferRuntimeEndpointSource::CompatibilityFallback
    );
    assert_eq!(
        resolver.cached_endpoint().expect("selection caches"),
        unchanged
    );
}

#[test]
fn incompatible_or_structurally_invalid_registration_falls_back() {
    let fixture = DiscoveryFixture::new("invalid");
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    fixture.write_registration("generation-a", "http://127.0.0.1:9111", "0.1.0-candidate.1");
    assert_fallback(&resolver);

    let mut unknown = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    unknown
        .as_object_mut()
        .expect("object")
        .insert("future".to_owned(), json!(true));
    fixture.write_value(&unknown);
    assert_fallback(&resolver);

    let manifest = serde_json::to_string(&registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    ))
    .expect("registration serializes");
    let duplicate = manifest.replacen(
        "\"schema\":\"infra.discovery.registration\"",
        "\"schema\":\"infra.discovery.registration\",\"schema\":\"infra.discovery.registration\"",
        1,
    );
    fs::write(fixture.manifest(), duplicate).expect("duplicate registration writes");
    set_mode(&fixture.manifest(), 0o600);
    assert_fallback(&resolver);

    let mut removed_lease = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    removed_lease.as_object_mut().expect("object").insert(
        "lease".to_owned(),
        json!({
            "renewed_at": "2026-08-12T00:00:00Z",
            "expires_at": "2026-08-12T00:00:45Z"
        }),
    );
    fixture.write_value(&removed_lease);
    assert_fallback(&resolver);

    let mut old_schema = registration_value(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    old_schema["schema_version"] = json!("20260810.1");
    fixture.write_value(&old_schema);
    assert_fallback(&resolver);
}

#[test]
fn migration_accepts_candidate_two_but_prefers_candidate_three() {
    let fixture = DiscoveryFixture::new("candidate-migration");
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    fixture.write_registration(
        "generation-candidate-two",
        "http://127.0.0.1:9111",
        InferRuntimeContractRevision::Candidate2.as_str(),
    );
    let candidate_two = resolver
        .resolve_current()
        .expect("candidate.2 remains selectable");
    assert_eq!(
        candidate_two.contract_version.as_deref(),
        Some(InferRuntimeContractRevision::Candidate2.as_str())
    );

    let mut both = registration_value(
        "generation-both",
        "http://127.0.0.1:9222",
        InferRuntimeContractRevision::Candidate2.as_str(),
    );
    both["offers"][1]["protocol_versions"] = json!([
        InferRuntimeContractRevision::Candidate2.as_str(),
        INFER_RUNTIME_CONTRACT_VERSION
    ]);
    fixture.write_value(&both);
    let candidate_three = resolver
        .resolve_current()
        .expect("preferred candidate resolves");
    assert_eq!(
        candidate_three.contract_version.as_deref(),
        Some(INFER_RUNTIME_CONTRACT_VERSION)
    );
}

#[test]
fn owner_only_and_nofollow_boundaries_fail_closed_to_fallback() {
    let fixture = DiscoveryFixture::new("filesystem");
    fixture.write_registration(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    set_mode(&fixture.manifest(), 0o644);
    assert_fallback(&resolver);

    fixture.write_registration(
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let target = fixture.root.join("owned-registration.json");
    fs::rename(fixture.manifest(), &target).expect("manifest becomes target");
    symlink(&target, fixture.manifest()).expect("manifest symlink creates");
    assert_fallback(&resolver);
}

#[test]
#[ignore = "requires a live Infer Runtime publisher in the platform discovery root"]
fn live_publisher_resolves_through_the_strict_discovery_consumer() {
    let probe = crate::infer_runtime::probe_infer_runtime_contract("");
    let endpoint = probe.endpoint.expect("live probe resolves an endpoint");
    assert_eq!(endpoint.source, InferRuntimeEndpointSource::Discovery);
    assert_eq!(endpoint.instance_id.as_deref(), Some(SERVICE_INSTANCE_ID));
    assert!(
        endpoint
            .generation
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        probe
            .contract
            .expect("live contract probe succeeds")
            .contract_version,
        INFER_RUNTIME_CONTRACT_VERSION
    );
}

fn assert_fallback(resolver: &InferRuntimeEndpointResolver) {
    assert_eq!(
        resolver
            .resolve_current()
            .expect("fallback remains valid")
            .source,
        InferRuntimeEndpointSource::CompatibilityFallback
    );
}

fn registration_value(generation: &str, endpoint: &str, version: &str) -> serde_json::Value {
    json!({
        "schema": DISCOVERY_SCHEMA,
        "schema_version": DISCOVERY_SCHEMA_VERSION,
        "service": {
            "kind": SERVICE_KIND,
            "instance_id": SERVICE_INSTANCE_ID,
            "generation": generation
        },
        "offers": [{
            "protocol": "infer-runtime.status",
            "protocol_versions": ["20260810.1"],
            "binding": "infra.local.unix-socket",
            "endpoint": "sockets/ir-fixture.sock"
        }, {
            "protocol": CONSUMER_PROTOCOL,
            "protocol_versions": [version],
            "binding": CONSUMER_BINDING,
            "endpoint": endpoint
        }]
    })
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("fixture mode sets");
}
