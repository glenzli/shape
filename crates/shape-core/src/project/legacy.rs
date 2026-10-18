//! Read-only navigation of retired specialist state; migration owners keep exact bytes.
use serde_json::{Value, json};
use shape_domain::{ArtifactKind, ArtifactWorkingGraph};

use super::{
    ShapeProject,
    vox_transfer::{graph_digest, is_authoring_node, speech_operation},
};
use crate::CoreError;

impl ShapeProject {
    /// Lightweight compatibility navigation. No draft is created, normalized or checkpointed.
    /// Immutable payloads remain available through exact historical revision readers.
    /// # Errors
    /// Rejects corrupt metadata or unencodable persisted graphs.
    pub fn legacy_snapshot_json(&self) -> Result<String, CoreError> {
        let snapshot = self.snapshot()?;
        let graphs = self.artifact_working_graphs()?;
        let mut legacy_artifacts = Vec::new();
        let mut texts = Vec::new();
        let mut speech_drafts = Vec::new();
        for artifact in &snapshot.artifacts {
            let graph = graphs
                .iter()
                .find(|g| g.context_artifact_id() == artifact.id);
            if matches!(
                artifact.kind,
                ArtifactKind::CardKit | ArtifactKind::PictureBook
            ) {
                let kind = if artifact.kind == ArtifactKind::CardKit {
                    "card_kit"
                } else {
                    "picture_book"
                };
                legacy_artifacts.push(json!({"artifact_id":artifact.id,"name":artifact.name,"kind":kind,"accepted_revision":artifact.accepted_revision,"draft":graph.map(draft_summary).transpose()?,"destination":if kind == "card_kit" {"deck"} else {"storybook"},"read_only":true}));
            }
            if artifact.kind == ArtifactKind::TextDocument {
                let authored = graph.filter(|g| g.operators().iter().any(is_authoring_node));
                texts.push(json!({"artifact_id":artifact.id,"name":artifact.name,"accepted_revision":artifact.accepted_revision,"authoring_graph_digest":authored.map(graph_digest).transpose()?,"has_authoring_draft":authored.is_some()}));
            }
            if let Some(graph) = graph {
                for node in graph
                    .operators()
                    .iter()
                    .filter(|node| node.operator_type().as_str() == "audio.speech_synthesize")
                {
                    // An unreadable legacy configuration is reported, never repaired or defaulted.
                    let operation = speech_operation(node);
                    speech_drafts.push(json!({"target_artifact_id":artifact.id,"name":artifact.name,"draft_id":node.id(),"graph_digest":graph_digest(graph)?,"expected_head":graph.expected_revision_id(),"input":node.input(),"operation":operation.as_ref().ok(),"issue":operation.err().map(|e| e.to_string())}));
                }
            }
        }
        Ok(serde_json::to_string(
            &json!({"schema":"shape.legacy-snapshot/1","project_id":snapshot.metadata.id,"legacy_artifacts":legacy_artifacts,"texts":texts,"speech_drafts":speech_drafts,"candidates_observed":false,"read_only":true}),
        )?)
    }
}

fn draft_summary(graph: &ArtifactWorkingGraph) -> Result<Value, serde_json::Error> {
    Ok(
        json!({"graph_digest":graph_digest(graph)?,"expected_head":graph.expected_revision_id(),"status":"unaccepted","operators":graph.operators().iter().map(|node| json!({"draft_id":node.id(),"operator_type":node.operator_type(),"configuration_schema":node.configuration().map(shape_domain::WorkingOperatorConfiguration::schema)})).collect::<Vec<_>>()}),
    )
}
