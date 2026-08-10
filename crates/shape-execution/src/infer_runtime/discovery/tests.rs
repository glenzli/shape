#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
};

use serde_json::json;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
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

    fn write_registration(
        &self,
        now: OffsetDateTime,
        generation: &str,
        endpoint: &str,
        version: &str,
    ) {
        self.write_value(&registration_value(now, generation, endpoint, version));
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
    let now = OffsetDateTime::now_utc();
    fixture.write_registration(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "http://127.0.0.1:9222",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    let endpoint = resolver.resolve_at(now).expect("override resolves");
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
        invalid.resolve_at(now),
        Err(InferRuntimeClientError::InvalidEndpoint)
    );
}

#[test]
fn discovery_tracks_generation_and_lease_before_falling_back() {
    let fixture = DiscoveryFixture::new("generation");
    let now = OffsetDateTime::now_utc();
    fixture.write_registration(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    let first = resolver.resolve_at(now).expect("first generation resolves");
    assert_eq!(first.source, InferRuntimeEndpointSource::Discovery);
    assert_eq!(first.instance_id.as_deref(), Some("local"));
    assert_eq!(first.generation.as_deref(), Some("generation-a"));
    assert!(first.lease_expires_at_unix.is_some());

    fixture.write_registration(
        now,
        "generation-b",
        "http://127.0.0.1:9222",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let second = resolver
        .resolve_at(now)
        .expect("second generation resolves");
    assert_eq!(second.origin, "http://127.0.0.1:9222");
    assert_eq!(second.generation.as_deref(), Some("generation-b"));

    let expired = resolver
        .resolve_at(now + Duration::seconds(121))
        .expect("fallback remains available");
    assert_eq!(
        expired.source,
        InferRuntimeEndpointSource::CompatibilityFallback
    );
    assert_eq!(expired.origin, "http://127.0.0.1:9333");
}

#[test]
fn connection_failure_retries_only_new_identity_then_uses_fallback() {
    let fixture = DiscoveryFixture::new("failure");
    let now = OffsetDateTime::now_utc();
    fixture.write_registration(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );
    let failed = resolver.resolve_at(now).expect("initial endpoint resolves");

    fixture.write_registration(
        OffsetDateTime::now_utc(),
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
    let now = OffsetDateTime::now_utc();
    let resolver = InferRuntimeEndpointResolver::with_runtime_root(
        "",
        fixture.root.clone(),
        "http://127.0.0.1:9333",
    );

    fixture.write_registration(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        "0.1.0-candidate.1",
    );
    assert_fallback(&resolver, now);

    let mut unknown = registration_value(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    unknown
        .as_object_mut()
        .expect("object")
        .insert("future".to_owned(), json!(true));
    fixture.write_value(&unknown);
    assert_fallback(&resolver, now);

    let manifest = serde_json::to_string(&registration_value(
        now,
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
    assert_fallback(&resolver, now);
}

#[test]
fn owner_only_and_nofollow_boundaries_fail_closed_to_fallback() {
    let fixture = DiscoveryFixture::new("filesystem");
    let now = OffsetDateTime::now_utc();
    fixture.write_registration(
        now,
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
    assert_fallback(&resolver, now);

    fixture.write_registration(
        now,
        "generation-a",
        "http://127.0.0.1:9111",
        INFER_RUNTIME_CONTRACT_VERSION,
    );
    let target = fixture.root.join("owned-registration.json");
    fs::rename(fixture.manifest(), &target).expect("manifest becomes target");
    symlink(&target, fixture.manifest()).expect("manifest symlink creates");
    assert_fallback(&resolver, now);
}

fn assert_fallback(resolver: &InferRuntimeEndpointResolver, now: OffsetDateTime) {
    assert_eq!(
        resolver
            .resolve_at(now)
            .expect("fallback remains valid")
            .source,
        InferRuntimeEndpointSource::CompatibilityFallback
    );
}

fn registration_value(
    now: OffsetDateTime,
    generation: &str,
    endpoint: &str,
    version: &str,
) -> serde_json::Value {
    let renewed_at = (now - Duration::seconds(5))
        .format(&Rfc3339)
        .expect("renewal time formats");
    let expires_at = (now + Duration::seconds(40))
        .format(&Rfc3339)
        .expect("expiration time formats");
    json!({
        "schema": DISCOVERY_SCHEMA,
        "schema_version": DISCOVERY_SCHEMA_VERSION,
        "service": {
            "kind": SERVICE_KIND,
            "instance_id": SERVICE_INSTANCE_ID,
            "generation": generation
        },
        "lease": {
            "renewed_at": renewed_at,
            "expires_at": expires_at
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
