use std::{fs, sync::Mutex};

use shape_domain::{ArtifactKind, IntentSpec, TransformationKind};
use shape_execution::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
};
use uuid::Uuid;

use super::*;

#[derive(Debug)]
struct GeneratedTextExecutor {
    identity: ExecutorIdentity,
    instruction: Mutex<Option<String>>,
}

impl GeneratedTextExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new("test.generated-text", "1", "test")
                .expect("identity is valid"),
            instruction: Mutex::new(None),
        }
    }
}

impl Executor for GeneratedTextExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == TEXT_GENERATE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        *self.instruction.lock().expect("instruction locks") =
            Some(String::from_utf8(request.instruction.clone()).expect("instruction is text"));
        Ok(ExecutionOutput {
            bytes: b"A generated replacement.".to_vec(),
            media_type: request.output_media_type.clone(),
            executor_job_id: Some("resp_core_test".to_owned()),
            external_provenance: None,
            content_contract: None,
        })
    }
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("shape-core-text-{label}-{}", Uuid::now_v7()))
}

fn seeded_text_project(root: &std::path::Path) -> (ShapeProject, ArtifactId, ArtifactRevision) {
    let mut project = ShapeProject::create(root, "Text Operator").expect("project creates");
    let artifact = project
        .create_artifact("Draft", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let origin = project
        .propose_text(
            artifact.id,
            None,
            "Original paragraph.",
            IntentSpec::new("Import original").expect("intent is valid"),
            Vec::new(),
        )
        .and_then(|candidate| project.accept_text(candidate))
        .expect("origin accepts");
    (project, artifact.id, origin)
}

#[test]
fn direct_text_edit_is_typed_preview_then_immutable_revision() {
    let root = test_root("direct");
    let (mut project, artifact_id, origin) = seeded_text_project(&root);
    let parameters = TextEditParameters::new("A clearer paragraph.");
    assert_eq!(parameters.replacement_text(), "A clearer paragraph.");
    assert_eq!(TEXT_EDIT_OPERATOR_TYPE, "text.edit");
    assert_eq!(TEXT_DOCUMENT_DATA_TYPE, "text.document");

    let candidate = project
        .propose_text_edit(
            artifact_id,
            origin.id,
            parameters,
            IntentSpec::new("Clarify the paragraph").expect("intent is valid"),
            Vec::new(),
        )
        .expect("edit executes");
    assert_eq!(candidate.expected_head(), Some(origin.id));
    assert_eq!(
        candidate.transformation_kind(),
        TransformationKind::TextRewrite
    );
    assert_eq!(
        candidate.receipt().capability.as_str(),
        TEXT_LITERAL_CAPABILITY
    );
    assert_eq!(candidate.text(), "A clearer paragraph.");
    assert_eq!(
        project
            .read_accepted(artifact_id)
            .expect("accepted content reads")
            .expect("accepted content exists")
            .bytes,
        b"Original paragraph."
    );

    let accepted = project.accept_text(candidate).expect("candidate accepts");
    assert_eq!(accepted.parents, vec![origin.id]);
    assert_eq!(project.revision(origin.id).expect("origin remains"), origin);
    let transformation = project
        .transformation(accepted.transformation_id)
        .expect("transformation persists");
    assert_eq!(transformation.kind, TransformationKind::TextRewrite);
    assert_eq!(transformation.inputs, vec![origin.id]);
    drop(project);

    let reopened = ShapeProject::open(&root).expect("project reopens");
    assert_eq!(
        reopened
            .read_accepted(artifact_id)
            .expect("accepted content reads")
            .expect("accepted content exists")
            .bytes,
        b"A clearer paragraph."
    );
    assert_eq!(
        reopened.revision(origin.id).expect("origin reopens"),
        origin
    );
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn text_edit_rejects_a_non_text_input_before_execution() {
    let root = test_root("kind");
    let project = ShapeProject::create(&root, "Text Operator").expect("project creates");
    let raster = project
        .create_artifact("Cover", ArtifactKind::ImageRaster)
        .expect("artifact creates");
    let result = project.propose_text(
        raster.id,
        None,
        "Not an image.",
        IntentSpec::new("Invalid edit").expect("intent is valid"),
        Vec::new(),
    );
    assert!(matches!(
        result,
        Err(CoreError::InvalidTextArtifact { artifact_id }) if artifact_id == raster.id
    ));
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn generated_text_keeps_physical_job_separate_from_creative_acceptance() {
    let root = test_root("generated");
    let (mut project, artifact_id, accepted) = seeded_text_project(&root);
    let executor = GeneratedTextExecutor::new();

    let parameters = TextTransformParameters::new(TextTransformMode::Rewrite, "Make it clearer.")
        .expect("parameters are valid");
    assert_eq!(parameters.mode(), TextTransformMode::Rewrite);
    assert_eq!(parameters.mode().as_str(), "rewrite");
    assert_eq!(parameters.instruction(), "Make it clearer.");
    assert_eq!(TEXT_TRANSFORM_OPERATOR_TYPE, "text.transform");
    let candidate = project
        .propose_text_transform(artifact_id, accepted.id, &parameters, Vec::new(), &executor)
        .expect("generation succeeds");
    assert_eq!(
        candidate.transformation_kind(),
        TransformationKind::GenerativeEdit
    );
    assert_eq!(
        candidate.receipt().capability.as_str(),
        TEXT_GENERATE_CAPABILITY
    );
    assert_eq!(
        candidate.receipt().executor_job_id.as_deref(),
        Some("resp_core_test")
    );
    assert_eq!(candidate.text(), "A generated replacement.");
    assert_eq!(
        project
            .read_accepted(artifact_id)
            .expect("accepted content reads")
            .expect("accepted content exists")
            .bytes,
        b"Original paragraph."
    );
    let instruction = executor
        .instruction
        .lock()
        .expect("instruction locks")
        .clone()
        .expect("instruction captured");
    assert!(instruction.contains("Make it clearer."));
    assert!(instruction.contains("Creative text transform mode: rewrite"));
    assert!(instruction.contains("Original paragraph."));

    let revision = project.accept_text(candidate).expect("candidate accepts");
    let transformation = project
        .transformation(revision.transformation_id)
        .expect("creative transformation persists");
    assert_eq!(transformation.kind, TransformationKind::GenerativeEdit);
    assert_eq!(
        transformation.intent.as_str(),
        "Rewrite text: Make it clearer."
    );
    assert_eq!(
        project
            .read_accepted(artifact_id)
            .expect("accepted content reads")
            .expect("accepted content exists")
            .bytes,
        b"A generated replacement."
    );
    fs::remove_dir_all(root).expect("fixture removes");
}
