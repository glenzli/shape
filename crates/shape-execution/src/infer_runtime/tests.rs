use infer_runtime_client::{
    CAPABILITY_CATALOG_SCHEMA, CAPABILITY_CATALOG_VERSION, CONSUMER_CORE, CONSUMER_OPENAPI_SHA256,
    CapabilityCatalog, CapabilityEntry, CapabilityRoute, CapabilitySchemaReference,
    ContractManifest, Stability,
};

use super::{
    INFER_RUNTIME_CAPABILITY_CATALOG, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_RUNTIME_RESPONSES_CAPABILITY, INFER_RUNTIME_SPEECH_CAPABILITY, InferRuntimeClientError,
    InferRuntimeEndpointSource, probe_contract_with_sdk,
    sdk::{SdkAdapterError, execution_failure},
    sdk_resolver,
    test_support::FakeSdk,
};

fn manifest() -> ContractManifest {
    serde_json::from_value(serde_json::json!({
        "schema": "infer-runtime.consumer-core",
        "schema_version": "20260813.1",
        "core_contract": CONSUMER_CORE,
        "supported_core_contracts": [CONSUMER_CORE],
        "capability_catalog": {
            "schema": CAPABILITY_CATALOG_SCHEMA,
            "schema_version": CAPABILITY_CATALOG_VERSION,
            "url": "/infer/v1/capabilities"
        },
        "openapi_url": "/infer/v1/openapi.json",
        "openapi_sha256": CONSUMER_OPENAPI_SHA256,
        "error_codes": ["consumer_core_unsupported"],
        "consumer_routes": []
    }))
    .unwrap()
}

fn capability(id: &str, version: &str, url: &str, digest: &str, path: &str) -> CapabilityEntry {
    CapabilityEntry {
        id: id.into(),
        schema_version: version.into(),
        stability: Stability::Stable,
        schema: CapabilitySchemaReference {
            format: "openapi-3.1".into(),
            url: url.into(),
            sha256: digest.into(),
        },
        routes: vec![CapabilityRoute {
            method: "POST".into(),
            path: path.into(),
            execution_modes: vec!["unary".into()],
        }],
    }
}

fn catalog() -> CapabilityCatalog {
    CapabilityCatalog {
        schema: CAPABILITY_CATALOG_SCHEMA.into(),
        schema_version: CAPABILITY_CATALOG_VERSION.into(),
        core_contract: CONSUMER_CORE.into(),
        capabilities: vec![
            capability(
                "infer.responses",
                "20260812.1",
                "/infer/v1/capability-schemas/infer.responses/20260812.1/openapi.json",
                "abfb3b4b9a3c5d3831d56bb877ecfdd43d62b4442ba101a5ef071ec2740adbd5",
                "/v1/responses",
            ),
            capability(
                "infer.audio.speech",
                "20260811.1",
                "/infer/v1/capability-schemas/infer.audio.speech/20260811.1/openapi.json",
                "19d29d6799a6cee1a6d24a63f9a9aab73ab925dd2e79f7181fbfe922f6906c68",
                "/v1/audio/speech",
            ),
        ],
    }
}

#[test]
fn sdk_fixture_requires_exact_core_catalog_and_consumed_capabilities() {
    let fake = FakeSdk::new();
    fake.contract.lock().unwrap().push_back(Ok(manifest()));
    fake.capabilities.lock().unwrap().push_back(Ok(catalog()));
    let contract = probe_contract_with_sdk(&fake).expect("frozen SDK fixture is compatible");
    assert_eq!(contract.contract_version, INFER_RUNTIME_CONTRACT_VERSION);
    assert_eq!(
        contract.capability_catalog,
        INFER_RUNTIME_CAPABILITY_CATALOG
    );
    assert_eq!(
        INFER_RUNTIME_RESPONSES_CAPABILITY,
        "infer.responses@20260812.1"
    );
    assert_eq!(
        INFER_RUNTIME_SPEECH_CAPABILITY,
        "infer.audio.speech@20260811.1"
    );
}

#[test]
fn missing_consumed_capability_fails_closed() {
    let fake = FakeSdk::new();
    let mut catalog = catalog();
    catalog
        .capabilities
        .retain(|entry| entry.id != "infer.audio.speech");
    fake.contract.lock().unwrap().push_back(Ok(manifest()));
    fake.capabilities.lock().unwrap().push_back(Ok(catalog));
    assert_eq!(
        probe_contract_with_sdk(&fake),
        Err(InferRuntimeClientError::IncompatibleContract {
            actual: "contract_mismatch".into()
        })
    );
}

#[test]
fn explicit_override_is_strict_and_never_implies_a_fixed_port_fallback() {
    let (_, source) = sdk_resolver("http://127.0.0.1:43129").unwrap();
    assert_eq!(source, InferRuntimeEndpointSource::ExplicitOverride);
    for invalid in [
        "https://127.0.0.1:43129",
        "http://localhost:43129",
        "http://127.0.0.1:43129/",
        "http://example.com:43129",
    ] {
        assert!(matches!(
            sdk_resolver(invalid),
            Err(InferRuntimeClientError::InvalidEndpoint)
        ));
    }
    let (_, source) = sdk_resolver("").unwrap();
    assert_eq!(source, InferRuntimeEndpointSource::Discovery);
}

#[test]
fn sdk_error_mapping_uses_machine_codes_without_messages() {
    let (code, retryable) =
        execution_failure(SdkAdapterError::Sdk(infer_runtime_client::Error::Api {
            status: reqwest::StatusCode::UPGRADE_REQUIRED,
            code: "consumer_core_unsupported".into(),
            message: "old daemon detail".into(),
        }));
    assert_eq!(code, "consumer_core_unsupported");
    assert!(!retryable);
}
