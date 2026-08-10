use std::{fs, sync::Mutex};

use shape_domain::{ArtifactKind, IntentSpec};
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
        capability.as_str() == "text.generate"
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        *self.instruction.lock().expect("instruction locks") =
            Some(String::from_utf8(request.instruction.clone()).expect("instruction is text"));
        Ok(ExecutionOutput {
            bytes: b"A generated replacement.".to_vec(),
            media_type: request.output_media_type.clone(),
            executor_job_id: Some("resp_core_test".to_owned()),
            content_contract: None,
        })
    }
}

#[test]
fn generated_text_keeps_runtime_output_transient_and_links_external_job() {
    let root = std::env::temp_dir().join(format!("shape-core-generated-{}", Uuid::now_v7()));
    let mut project = ShapeProject::create(&root, "Generated text").expect("project creates");
    let artifact = project
        .create_artifact("Draft", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let accepted = project
        .propose_text(
            artifact.id,
            None,
            "Original paragraph.",
            IntentSpec::new("Import original").expect("intent is valid"),
            Vec::new(),
        )
        .and_then(|candidate| project.accept_text(candidate))
        .expect("original accepts");
    let executor = GeneratedTextExecutor::new();

    let candidate = project
        .propose_generated_text(
            artifact.id,
            Some(accepted.id),
            "Make it clearer.",
            IntentSpec::new("Make it clearer.").expect("intent is valid"),
            Vec::new(),
            &executor,
        )
        .expect("generation succeeds");
    assert_eq!(candidate.text(), "A generated replacement.");
    assert_eq!(
        candidate.receipt().executor_job_id.as_deref(),
        Some("resp_core_test")
    );
    assert_eq!(
        project
            .read_accepted(artifact.id)
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
    assert!(instruction.contains("Original paragraph."));

    project.accept_text(candidate).expect("candidate accepts");
    assert_eq!(
        project
            .read_accepted(artifact.id)
            .expect("accepted content reads")
            .expect("accepted content exists")
            .bytes,
        b"A generated replacement."
    );
    fs::remove_dir_all(root).expect("fixture removes");
}
