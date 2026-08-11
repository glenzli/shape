use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use shape_execution::{
    INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClientError, InferRuntimeContract,
    InferRuntimeEndpointSource, InferRuntimeProbe, ResolvedInferRuntimeEndpoint,
};
use uuid::Uuid;

use super::{infer_runtime_probe_wire, load_project_snapshot};

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-desktop-bridge-{}", Uuid::now_v7()))
}

#[test]
fn infer_probe_wire_preserves_compatibility_without_diagnostics_payloads() {
    let compatible = infer_runtime_probe_wire(InferRuntimeProbe {
        endpoint: Some(discovered_endpoint()),
        contract: Ok(InferRuntimeContract {
            contract_version: INFER_RUNTIME_CONTRACT_VERSION.to_owned(),
        }),
    });
    assert!(compatible.reachable);
    assert!(compatible.compatible);
    assert_eq!(compatible.contract_version, INFER_RUNTIME_CONTRACT_VERSION);
    assert!(compatible.error_code.is_empty());
    assert_eq!(compatible.endpoint_source, "discovery");
    assert_eq!(compatible.endpoint_origin, "http://127.0.0.1:8787");
    assert_eq!(compatible.runtime_instance_id, "local");
    assert_eq!(compatible.runtime_generation, "generation-test");

    let incompatible = infer_runtime_probe_wire(InferRuntimeProbe {
        endpoint: Some(discovered_endpoint()),
        contract: Err(InferRuntimeClientError::IncompatibleContract {
            actual: "0.1.0-candidate.99".to_owned(),
        }),
    });
    assert!(incompatible.reachable);
    assert!(!incompatible.compatible);
    assert_eq!(incompatible.contract_version, "0.1.0-candidate.99");
    assert_eq!(incompatible.error_code, "incompatible_contract");

    let unavailable = infer_runtime_probe_wire(InferRuntimeProbe {
        endpoint: Some(ResolvedInferRuntimeEndpoint {
            origin: "http://127.0.0.1:8787".to_owned(),
            source: InferRuntimeEndpointSource::CompatibilityFallback,
            instance_id: None,
            generation: None,
            contract_version: None,
        }),
        contract: Err(InferRuntimeClientError::Unavailable),
    });
    assert!(!unavailable.reachable);
    assert!(!unavailable.compatible);
    assert!(unavailable.contract_version.is_empty());
    assert_eq!(unavailable.error_code, "unavailable");
    assert_eq!(unavailable.endpoint_source, "compatibility_fallback");
    assert!(unavailable.runtime_generation.is_empty());
}

fn discovered_endpoint() -> ResolvedInferRuntimeEndpoint {
    ResolvedInferRuntimeEndpoint {
        origin: "http://127.0.0.1:8787".to_owned(),
        source: InferRuntimeEndpointSource::Discovery,
        instance_id: Some("local".to_owned()),
        generation: Some("generation-test".to_owned()),
        contract_version: Some(INFER_RUNTIME_CONTRACT_VERSION.to_owned()),
    }
}

#[test]
fn bridge_preserves_presence_identity_and_verified_text() {
    let root = test_root();
    let mut project = ShapeProject::create(&root, "Bridge Contract").expect("project creates");
    let story = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("story creates");
    project
        .create_artifact("References", ArtifactKind::ReferenceSet)
        .expect("reference set creates");
    let candidate = project
        .propose_text(
            story.id,
            None,
            "A quiet summer afternoon.",
            IntentSpec::new("Import the opening").expect("intent valid"),
            Vec::new(),
        )
        .expect("candidate executes");
    let accepted = project.accept_text(candidate).expect("candidate accepts");
    drop(project);

    let snapshot = load_project_snapshot(root.to_str().expect("portable test path"))
        .expect("bridge snapshot loads");
    assert_eq!(snapshot.project_name, "Bridge Contract");
    assert_eq!(snapshot.artifacts.len(), 2);
    assert!(snapshot.graph_edges.is_empty());

    let story_wire = snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.id == story.id.to_string())
        .expect("story is projected");
    assert!(story_wire.has_accepted_revision);
    assert_eq!(story_wire.accepted_revision_id, accepted.id.to_string());
    assert!(story_wire.accepted_parent_revision_ids.is_empty());
    assert_eq!(story_wire.transformation_kind_key, "import");
    assert_eq!(story_wire.transformation_intent, "Import the opening");
    assert!(story_wire.transformation_input_revision_ids.is_empty());
    assert!(story_wire.transformation_input_artifact_ids.is_empty());
    assert!(story_wire.transformation_input_artifact_names.is_empty());
    assert_eq!(story_wire.constraint_count, 0);
    assert_eq!(story_wire.reference_count, 0);
    assert!(story_wire.has_content);
    assert_eq!(story_wire.media_type, "text/plain; charset=utf-8");
    assert!(story_wire.has_text_preview);
    assert!(!story_wire.text_preview_truncated);
    assert_eq!(story_wire.text_preview, "A quiet summer afternoon.");
    assert_eq!(story_wire.operator_graph_nodes.len(), 2);
    assert_eq!(story_wire.operator_graph_edges.len(), 1);
    assert_eq!(story_wire.operator_graph_nodes[0].role_key, "source");
    assert_eq!(story_wire.operator_graph_nodes[1].role_key, "output");
    assert_eq!(
        story_wire.operator_graph_edges[0].data_type_key,
        "text.document"
    );

    let reference_wire = snapshot
        .artifacts
        .iter()
        .find(|artifact| artifact.kind_key == "reference_set")
        .expect("reference set is projected");
    assert!(!reference_wire.has_accepted_revision);
    assert!(!reference_wire.has_content);
    assert!(!reference_wire.has_text_preview);
    assert!(reference_wire.operator_graph_nodes.is_empty());
    fs::remove_dir_all(root).expect("test project removes");
}
