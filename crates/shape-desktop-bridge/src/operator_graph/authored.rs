//! Stable authored topology; accepted revisions supply content, never node identity.

use super::{
    Artifact, OperatorGraphProjection, ShapeProject, VALUE_INPUT_PORT, VALUE_OUTPUT_PORT, ffi,
};
use shape_domain::{ArtifactWorkingGraph, RevisionId};

pub(super) fn project_authored(
    project: &ShapeProject,
    artifact: &Artifact,
    artifacts: &[Artifact],
    graph: &ArtifactWorkingGraph,
) -> Result<OperatorGraphProjection, String> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for draft in graph.operators() {
        if let Some(input) = draft.input() {
            let source = artifacts
                .iter()
                .find(|a| a.id == input.artifact_id)
                .ok_or("missing_graph_input")?;
            let id = format!("source.{}", source.id);
            if !nodes
                .iter()
                .any(|n: &ffi::OperatorGraphNodeWire| n.node_id == id)
            {
                let mut node =
                    content_node(project, source, Some(input.revision_id), "source", &id)?;
                node.output_ports.push(port(
                    VALUE_OUTPUT_PORT,
                    draft
                        .input_data_type()
                        .ok_or("missing_input_type")?
                        .as_str(),
                ));
                nodes.push(node);
            }
            edges.push(edge(
                &id,
                draft.id().as_str(),
                draft
                    .input_data_type()
                    .ok_or("missing_input_type")?
                    .as_str(),
            ));
        }
        let mut node = content_node(project, artifact, None, "operator", draft.id().as_str())?;
        node.operator_type_key = draft.operator_type().as_str().into();
        if let Some(input_type) = draft.input_data_type() {
            node.input_ports
                .push(port(VALUE_INPUT_PORT, input_type.as_str()));
        }
        node.output_ports
            .push(port(VALUE_OUTPUT_PORT, draft.output_data_type().as_str()));
        nodes.push(node);
        let output_id = format!("output.{}", artifact.id);
        let mut output = content_node(
            project,
            artifact,
            artifact.accepted_revision,
            "output",
            &output_id,
        )?;
        output
            .input_ports
            .push(port(VALUE_INPUT_PORT, draft.output_data_type().as_str()));
        nodes.push(output);
        edges.push(edge(
            draft.id().as_str(),
            &output_id,
            draft.output_data_type().as_str(),
        ));
    }
    Ok(OperatorGraphProjection { nodes, edges })
}

fn content_node(
    project: &ShapeProject,
    artifact: &Artifact,
    revision: Option<RevisionId>,
    role: &str,
    id: &str,
) -> Result<ffi::OperatorGraphNodeWire, String> {
    let mut node = ffi::OperatorGraphNodeWire {
        node_id: id.into(),
        role_key: role.into(),
        operator_type_key: format!(
            "{role}.{}",
            super::artifact_data_type(artifact.kind)?.as_str()
        ),
        artifact_id: artifact.id.to_string(),
        artifact_name: artifact.name.clone(),
        revision_id: revision.map_or_else(String::new, |r| r.to_string()),
        transformation_id: String::new(),
        intent: String::new(),
        media_type: String::new(),
        byte_length: 0,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
        input_ports: Vec::new(),
        output_ports: Vec::new(),
    };
    if let Some(revision_id) = revision {
        let content = project
            .read_revision_content(revision_id)
            .map_err(|e| e.to_string())?;
        node.media_type = content.revision.content.media_type;
        node.byte_length = content.revision.content.byte_length;
        if node.media_type.starts_with("text/")
            && let Some((preview, truncated)) = crate::bounded_text_preview(&content.bytes)
        {
            node.has_text_preview = true;
            node.text_preview = preview;
            node.text_preview_truncated = truncated;
        }
    }
    Ok(node)
}

fn port(id: &str, data_type: &str) -> ffi::OperatorPortWire {
    ffi::OperatorPortWire {
        port_id: id.into(),
        data_type_key: data_type.into(),
    }
}

fn edge(source: &str, target: &str, data_type: &str) -> ffi::OperatorGraphEdgeWire {
    ffi::OperatorGraphEdgeWire {
        source_node_id: source.into(),
        source_port_id: VALUE_OUTPUT_PORT.into(),
        target_node_id: target.into(),
        target_port_id: VALUE_INPUT_PORT.into(),
        data_type_key: data_type.into(),
    }
}

pub(super) fn is_authored_text_output(graph: &ArtifactWorkingGraph) -> bool {
    graph.operators().len() == 1
        && graph.operators().iter().any(|d| {
            d.operator_type().as_str() == "audio.speech_synthesize"
                || (matches!(d.operator_type().as_str(), "text.create" | "text.edit")
                    && d.configuration().is_some_and(|c| {
                        c.schema().as_str() == crate::operator_catalog::text_authoring::SCHEMA
                    }))
        })
}
