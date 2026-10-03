//! Payload-free JSON projection of the Core read-only project inspection use case.
use serde_json::json;
use shape_core::ShapeProject;
use shape_domain::ContentDigest;
use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, Write},
    path::Path,
};

const SCHEMA: &str = "shape.cli.project-status@20261003.1";

pub fn run(path: &Path) -> Result<u8, Box<dyn Error>> {
    let (report, code) = match ShapeProject::inspect(path) {
        Ok(state) => {
            let heads: BTreeMap<_, _> = state
                .snapshot
                .artifacts
                .iter()
                .map(|a| (a.id, a.accepted_revision))
                .collect();
            let mut drafts = Vec::new();
            for graph in &state.working_graphs {
                let current_output = heads.get(&graph.context_artifact_id()).copied().flatten();
                let output_status = if current_output == graph.expected_revision_id() {
                    "current"
                } else {
                    "changed"
                };
                let graph_digest =
                    ContentDigest::from_bytes(&serde_json::to_vec(graph)?).to_string();
                for draft in graph.operators() {
                    let current_input = draft
                        .input()
                        .and_then(|input| heads.get(&input.artifact_id).copied().flatten());
                    let input_status = match (draft.input(), draft.input_data_type()) {
                        (None, None) => "not_required",
                        (None, Some(_)) => "unbound",
                        (Some(_), _) if current_input.is_none() => "missing",
                        (Some(input), _) if current_input == Some(input.revision_id) => "current",
                        _ => "changed",
                    };
                    let next_action = if output_status == "current" {
                        match input_status {
                            "changed" => "refresh_input_and_review",
                            "missing" | "unbound" => "bind_accepted_input",
                            _ => "review_in_desktop",
                        }
                    } else {
                        "reopen_output_and_review"
                    };
                    drafts.push(json!({"id":draft.id(),"operator_type":draft.operator_type(),
                        "output_artifact_id":graph.context_artifact_id(),"expected_output_revision":graph.expected_revision_id(),
                        "current_output_revision":current_output,"output_status":output_status,
                        "input":draft.input(),"current_input_revision":current_input,"input_status":input_status,
                        "graph_digest":graph_digest,"execution_checked":false,"next_action":next_action}));
                }
            }
            let mut report = json!({"schema":SCHEMA,"status":"ok","read_only":true,
                "project":state.snapshot.metadata,"artifacts":state.snapshot.artifacts,"scenes":state.snapshot.scenes,
                "drafts":drafts,"candidates":{"observed":false,"visibility":"desktop_session_only"},
                "acceptance":"explicit_session_action_required"});
            report["snapshot_digest"] = ContentDigest::from_bytes(&serde_json::to_vec(&report)?)
                .to_string()
                .into();
            (report, 0)
        }
        Err(error) => (
            json!({"schema":SCHEMA,"status":"error","read_only":true,"code":"project_unavailable","message":error.to_string()}),
            1,
        ),
    };
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, &report)?;
    writeln!(output)?;
    Ok(code)
}
