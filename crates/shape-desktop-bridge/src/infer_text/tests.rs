use std::fs;

use shape_core::{ShapeProject, TextTransformMode, TextTransformParameters};
use shape_domain::{ArtifactKind, IntentSpec};
use shape_execution::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    INFER_RUNTIME_CONTRACT_VERSION,
};
use uuid::Uuid;

use super::*;
use crate::open_desktop_session;

struct FakeTextExecutor {
    identity: ExecutorIdentity,
}

impl FakeTextExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.test.fake-text-sdk",
                "1",
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .unwrap(),
        }
    }
}

impl Executor for FakeTextExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == "text.generate"
    }

    fn execute(&self, _request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        Ok(ExecutionOutput {
            bytes: b"A generated bridge candidate.".to_vec(),
            media_type: "text/plain; charset=utf-8".into(),
            executor_job_id: Some("resp_bridge_test".into()),
            external_provenance: None,
            content_contract: None,
        })
    }
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

    let mut draft_session =
        open_desktop_session(project_path.to_str().expect("portable project path"))
            .expect("draft session opens");
    let draft = draft_session
        .session_begin_operator_draft(&artifact.id.to_string(), "text.edit")
        .expect("Writing draft begins");
    draft_session
        .session_update_text_transform_draft(
            &draft.draft_id,
            "expand",
            "Make it more vivid.",
            "warm",
            "literary",
            1,
        )
        .expect("text transform draft config persists");
    drop(draft_session);

    let project = ShapeProject::open(&project_path).expect("project reopens for fake execution");
    let expected_head = project.snapshot().unwrap().artifacts[0]
        .accepted_revision
        .expect("accepted text head exists");
    let parameters = TextTransformParameters::new(
        TextTransformMode::Expand,
        "Make it more vivid.\nTone: warm at balanced intensity. Audience: general audience. Style: literary.",
    )
    .unwrap();
    let candidate = project
        .propose_text_transform(
            artifact.id,
            expected_head,
            &parameters,
            Vec::new(),
            &FakeTextExecutor::new(),
        )
        .expect("fake SDK execution prepares a transient candidate");
    let generated = Box::new(InferTextCandidate { candidate });

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
    assert_eq!(
        accepted.artifacts[0].transformation_intent,
        "Expand text: Make it more vivid.\nTone: warm at balanced intensity. Audience: general audience. Style: literary."
    );
    assert_eq!(accepted.artifacts[0].operator_graph_nodes.len(), 3);
    assert_eq!(
        accepted.artifacts[0].operator_graph_nodes[1].operator_type_key,
        "text.transform"
    );
    drop(session);
    fs::remove_dir_all(project_path).expect("fixture removes");
}
