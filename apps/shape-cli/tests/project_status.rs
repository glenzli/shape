//! Public inspection over actual persisted creative state, including a live WAL writer.
use serde_json::{Value, json};
use shape_core::ShapeProject;
use shape_domain::{
    Artifact, ArtifactKind, ArtifactWorkingGraph, ContentDigest, IntentSpec, OperatorDataTypeId,
    OperatorTypeId, TextDocumentContract, WorkingInput,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "shape-project-status-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn project(&self) -> PathBuf {
        self.0.join("synthetic.shape")
    }
    fn status(&self, code: i32) -> Value {
        let result = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
            .arg("project-status")
            .arg(self.project())
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            result.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(report["schema"], "shape.cli.project-status@20261003.1");
        report
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn reports_persisted_state_without_claiming_to_see_in_memory_candidates() {
    let fixture = Fixture::new();
    let mut project = ShapeProject::create(fixture.project(), "双语录音审阅").unwrap();
    let source = project
        .create_text_document("Original", "Private original words.")
        .unwrap();
    let _candidate = project
        .propose_text(
            source.artifact_id,
            Some(source.id),
            "Private candidate words.",
            IntentSpec::new("Review").unwrap(),
            vec![],
        )
        .unwrap();
    let report = fixture.status(0);
    assert_eq!(report["read_only"], true);
    assert_eq!(
        report["artifacts"][0]["accepted_revision"],
        json!(source.id)
    );
    assert_eq!(report["candidates"]["observed"], false);
    assert_eq!(report["candidates"]["visibility"], "desktop_session_only");
    assert_eq!(report["acceptance"], "explicit_session_action_required");
    assert_eq!(report["drafts"], json!([]));
    assert!(!report.to_string().contains("Private"));
    for _ in 0..3 {
        assert_eq!(fixture.status(0), report);
    }
    assert_eq!(
        project.snapshot().unwrap().artifacts[0].accepted_revision,
        Some(source.id)
    );
}

fn create_branch(
    project: &mut ShapeProject,
    input: WorkingInput,
) -> (Artifact, ArtifactWorkingGraph) {
    let target = Artifact::new("Script branch", ArtifactKind::TextDocument).unwrap();
    let mut graph = ArtifactWorkingGraph::new_source(target.id);
    let data_type = OperatorDataTypeId::new("text.document").unwrap();
    graph
        .add_bound_operator(
            OperatorTypeId::new("text.edit").unwrap(),
            data_type.clone(),
            data_type,
            input,
        )
        .unwrap();
    project
        .create_source_artifact_draft(&target, &graph)
        .unwrap();
    (target, graph)
}

#[test]
fn stale_input_refresh_and_explicit_acceptance_share_actual_versions() {
    let fixture = Fixture::new();
    let mut project = ShapeProject::create(fixture.project(), "Review lifecycle").unwrap();
    let source = project
        .create_text_document("Original", "An October session.")
        .unwrap();
    let (target, mut graph) = create_branch(
        &mut project,
        WorkingInput {
            artifact_id: source.artifact_id,
            revision_id: source.id,
        },
    );
    let draft = graph.operators()[0].clone();
    let ready = fixture.status(0);
    assert_eq!(ready["drafts"][0]["input_status"], "current");
    assert_eq!(ready["drafts"][0]["output_status"], "current");
    assert_eq!(ready["drafts"][0]["execution_checked"], false);
    assert_eq!(ready["drafts"][0]["id"], draft.id().as_str());
    let script = "[note: Check the name before recording.]\nHello, 世界.\n[pause: 1s]";
    let candidate = project
        .propose_text_node_literal(
            target.id,
            &draft,
            script,
            TextDocumentContract::speech_script(),
        )
        .unwrap();
    let update = project
        .propose_text(
            source.artifact_id,
            Some(source.id),
            "A revised October session.",
            IntentSpec::new("Human source edit").unwrap(),
            vec![],
        )
        .unwrap();
    let revised = project.accept_text(update).unwrap();
    let stale = fixture.status(0);
    let stale_draft = &stale["drafts"][0];
    assert_eq!(stale_draft["input_status"], "changed");
    assert_eq!(stale_draft["input"]["revision_id"], json!(source.id));
    assert_eq!(stale_draft["current_input_revision"], json!(revised.id));
    assert_eq!(stale_draft["next_action"], "refresh_input_and_review");
    assert_ne!(stale["snapshot_digest"], ready["snapshot_digest"]);
    assert!(project.accept_text(candidate).is_err());
    assert!(project.read_accepted(target.id).unwrap().is_none());
    let before = graph.clone();
    assert!(graph.set_operator_input(
        draft.id(),
        WorkingInput {
            artifact_id: source.artifact_id,
            revision_id: revised.id
        }
    ));
    project
        .replace_artifact_working_graph(target.id, Some(&before), Some(&graph))
        .unwrap();
    let refreshed = fixture.status(0);
    assert_eq!(refreshed["drafts"][0]["input_status"], "current");
    assert_ne!(
        refreshed["drafts"][0]["graph_digest"],
        stale_draft["graph_digest"]
    );
    let candidate = project
        .propose_text_node_literal(
            target.id,
            &graph.operators()[0],
            script,
            TextDocumentContract::speech_script(),
        )
        .unwrap();
    let accepted = project.accept_text(candidate).unwrap();
    let final_state = fixture.status(0);
    assert_eq!(
        final_state["drafts"][0]["current_output_revision"],
        json!(accepted.id)
    );
    assert_eq!(final_state["drafts"][0]["output_status"], "current");
    assert_eq!(
        project
            .read_accepted(source.artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id,
        revised.id
    );
    assert_eq!(
        project
            .read_accepted(target.id)
            .unwrap()
            .unwrap()
            .revision
            .content
            .digest,
        ContentDigest::from_bytes(script.as_bytes())
    );
}

#[test]
fn missing_malformed_old_or_oversize_bundles_are_rejected_without_migration() {
    let fixture = Fixture::new();
    assert_eq!(fixture.status(1)["code"], "project_unavailable");
    assert!(!fixture.project().exists());
    let project = ShapeProject::create(fixture.project(), "Invalid inputs").unwrap();
    drop(project);
    let manifest = fixture.project().join("manifest.json");
    let database = fixture.project().join("project.sqlite");
    let original = fs::read(&manifest).unwrap();
    let database_bytes = fs::read(&database).unwrap();
    let mut older: Value = serde_json::from_slice(&original).unwrap();
    older["metadata"]["schema_revision"] = json!("20260810.1");
    for bytes in [
        serde_json::to_vec(&older).unwrap(),
        vec![0xff, 0xfe],
        vec![b' '; 65_537],
    ] {
        fs::write(&manifest, &bytes).unwrap();
        assert_eq!(fixture.status(1)["code"], "project_unavailable");
        assert_eq!(fs::read(&manifest).unwrap(), bytes);
        assert_eq!(fs::read(&database).unwrap(), database_bytes);
    }
    fs::write(&manifest, &original).unwrap();
    fs::write(&database, b"not a SQLite database").unwrap();
    assert_eq!(fixture.status(1)["code"], "project_unavailable");
    assert_eq!(fs::read(&database).unwrap(), b"not a SQLite database");
    fs::remove_file(&database).unwrap();
    assert_eq!(fixture.status(1)["code"], "project_unavailable");
    assert!(
        !database.exists(),
        "inspection must never create a missing database"
    );
}
