//! Projection of accepted Artifact history as one scene-compatible Operator Graph.
//!
//! The current foundation treats each Artifact as a single-output scene boundary.
//! Same-artifact accepted history expands into visible Operators; revisions owned
//! by another Artifact stop at a pure Source boundary. Persistent Scene and
//! `GraphComponent` aggregates remain a later contract.

use std::collections::{HashMap, HashSet};

use shape_core::{
    ShapeProject, TEXT_DOCUMENT_DATA_TYPE, TEXT_EDIT_OPERATOR_TYPE, TEXT_TRANSFORM_OPERATOR_TYPE,
};
use shape_domain::{
    AUDIO_CLIP_DATA_TYPE, Artifact, ArtifactId, ArtifactKind, IMAGE_BLUR_OPERATOR_TYPE,
    IMAGE_CROP_OPERATOR_TYPE, IMAGE_DROP_SHADOW_OPERATOR_TYPE, IMAGE_RESIZE_OPERATOR_TYPE,
    IMAGE_TRANSFORM_OPERATOR_TYPE, IMAGE_UNSHARP_MASK_OPERATOR_TYPE, OperatorDataTypeId,
    OperatorGraph, OperatorGraphEdge, OperatorGraphNode, OperatorNodeBinding, OperatorNodeId,
    OperatorNodeRole, OperatorPort, OperatorPortId, OperatorTypeId, RevisionId, Transformation,
    TransformationKind, TransformationOperation,
};

use crate::ffi;
mod authored;

const VALUE_INPUT_PORT: &str = "input.value";
const VALUE_OUTPUT_PORT: &str = "output.value";

pub(crate) struct OperatorGraphProjection {
    pub nodes: Vec<ffi::OperatorGraphNodeWire>,
    pub edges: Vec<ffi::OperatorGraphEdgeWire>,
}

pub(crate) fn project_operator_graph(
    project: &ShapeProject,
    artifact: &Artifact,
    project_artifacts: &[Artifact],
) -> Result<OperatorGraphProjection, String> {
    if let Some(graph) = project
        .artifact_working_graphs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|g| g.context_artifact_id() == artifact.id && authored::is_authored_text_output(g))
    {
        return authored::project_authored(project, artifact, project_artifacts, &graph);
    }
    let Some(head) = artifact.accepted_revision else {
        return Ok(OperatorGraphProjection {
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    };
    let mut builder = GraphBuilder {
        project,
        target_artifact_id: artifact.id,
        project_artifacts,
        nodes: Vec::new(),
        edges: Vec::new(),
        endpoints: HashMap::new(),
        operator_outputs: HashMap::new(),
        active_revisions: HashSet::new(),
    };
    let terminal = builder.build_revision(head, true)?;
    let output_id = OperatorNodeId::output(artifact.id);
    builder.nodes.push(
        OperatorGraphNode::new(
            output_id.clone(),
            OperatorNodeRole::Output,
            OperatorTypeId::new(format!("output.{}", terminal.data_type.as_str()))
                .map_err(|error| error.to_string())?,
            OperatorNodeBinding::Output {
                artifact_id: artifact.id,
                revision_id: head,
            },
            vec![OperatorPort::new(
                OperatorPortId::new(VALUE_INPUT_PORT).map_err(|error| error.to_string())?,
                terminal.data_type.clone(),
            )],
            Vec::new(),
        )
        .map_err(|error| error.to_string())?,
    );
    builder.edges.push(OperatorGraphEdge::new(
        terminal.node_id,
        terminal.port_id,
        output_id,
        OperatorPortId::new(VALUE_INPUT_PORT).map_err(|error| error.to_string())?,
    ));
    let graph = OperatorGraph::new(builder.nodes, builder.edges)
        .map_err(|error| format!("accepted operator graph is invalid: {error}"))?;
    graph_wire(
        project,
        project_artifacts,
        &graph,
        &builder.operator_outputs,
    )
}

#[derive(Clone)]
struct Endpoint {
    node_id: OperatorNodeId,
    port_id: OperatorPortId,
    data_type: OperatorDataTypeId,
}

struct GraphBuilder<'a> {
    project: &'a ShapeProject,
    target_artifact_id: ArtifactId,
    project_artifacts: &'a [Artifact],
    nodes: Vec<OperatorGraphNode>,
    edges: Vec<OperatorGraphEdge>,
    endpoints: HashMap<RevisionId, Endpoint>,
    operator_outputs: HashMap<OperatorNodeId, (ArtifactId, RevisionId)>,
    active_revisions: HashSet<RevisionId>,
}

impl GraphBuilder<'_> {
    fn build_revision(
        &mut self,
        revision_id: RevisionId,
        expand_same_artifact: bool,
    ) -> Result<Endpoint, String> {
        if let Some(endpoint) = self.endpoints.get(&revision_id) {
            return Ok(endpoint.clone());
        }
        if !self.active_revisions.insert(revision_id) {
            return Err("accepted revision history contains a cycle".to_owned());
        }
        let revision = self
            .project
            .revision(revision_id)
            .map_err(|error| error.to_string())?;
        let artifact = self
            .project_artifacts
            .iter()
            .find(|artifact| artifact.id == revision.artifact_id)
            .ok_or_else(|| "operator input artifact is missing".to_owned())?;
        let transformation = self
            .project
            .transformation(revision.transformation_id)
            .map_err(|error| error.to_string())?;
        let data_type = artifact_data_type(artifact.kind)?;
        let endpoint = if transformation.kind == TransformationKind::Import || !expand_same_artifact
        {
            self.add_source(artifact, revision_id, data_type)?
        } else {
            let mut input_endpoints = Vec::with_capacity(transformation.inputs.len());
            for input in &transformation.inputs {
                let input_revision = self
                    .project
                    .revision(*input)
                    .map_err(|error| error.to_string())?;
                input_endpoints.push(self.build_revision(
                    *input,
                    input_revision.artifact_id == self.target_artifact_id,
                )?);
            }
            self.add_operator(
                &transformation,
                artifact.id,
                revision_id,
                artifact.kind,
                &input_endpoints,
                data_type,
            )?
        };
        self.active_revisions.remove(&revision_id);
        self.endpoints.insert(revision_id, endpoint.clone());
        Ok(endpoint)
    }

    fn add_source(
        &mut self,
        artifact: &Artifact,
        revision_id: RevisionId,
        data_type: OperatorDataTypeId,
    ) -> Result<Endpoint, String> {
        let node_id = OperatorNodeId::source(revision_id);
        let port_id = OperatorPortId::new(VALUE_OUTPUT_PORT).map_err(|error| error.to_string())?;
        self.nodes.push(
            OperatorGraphNode::new(
                node_id.clone(),
                OperatorNodeRole::Source,
                OperatorTypeId::new(format!("source.{}", data_type.as_str()))
                    .map_err(|error| error.to_string())?,
                OperatorNodeBinding::Source {
                    artifact_id: artifact.id,
                    revision_id,
                },
                Vec::new(),
                vec![OperatorPort::new(port_id.clone(), data_type.clone())],
            )
            .map_err(|error| error.to_string())?,
        );
        Ok(Endpoint {
            node_id,
            port_id,
            data_type,
        })
    }

    fn add_operator(
        &mut self,
        transformation: &Transformation,
        output_artifact_id: ArtifactId,
        output_revision_id: RevisionId,
        output_kind: ArtifactKind,
        input_endpoints: &[Endpoint],
        output_data_type: OperatorDataTypeId,
    ) -> Result<Endpoint, String> {
        let node_id = OperatorNodeId::transformation(transformation.id);
        let output_port_id =
            OperatorPortId::new(VALUE_OUTPUT_PORT).map_err(|error| error.to_string())?;
        let mut input_ports = Vec::with_capacity(input_endpoints.len());
        for (index, endpoint) in input_endpoints.iter().enumerate() {
            let input_port_id =
                OperatorPortId::new(format!("input.{index}")).map_err(|error| error.to_string())?;
            input_ports.push(OperatorPort::new(
                input_port_id.clone(),
                endpoint.data_type.clone(),
            ));
            self.edges.push(OperatorGraphEdge::new(
                endpoint.node_id.clone(),
                endpoint.port_id.clone(),
                node_id.clone(),
                input_port_id,
            ));
        }
        self.nodes.push(
            OperatorGraphNode::new(
                node_id.clone(),
                OperatorNodeRole::Operator,
                operator_type(transformation, output_kind)?,
                OperatorNodeBinding::Transformation {
                    transformation_id: transformation.id,
                },
                input_ports,
                vec![OperatorPort::new(
                    output_port_id.clone(),
                    output_data_type.clone(),
                )],
            )
            .map_err(|error| error.to_string())?,
        );
        self.operator_outputs
            .insert(node_id.clone(), (output_artifact_id, output_revision_id));
        Ok(Endpoint {
            node_id,
            port_id: output_port_id,
            data_type: output_data_type,
        })
    }
}

fn graph_wire(
    project: &ShapeProject,
    artifacts: &[Artifact],
    graph: &OperatorGraph,
    operator_outputs: &HashMap<OperatorNodeId, (ArtifactId, RevisionId)>,
) -> Result<OperatorGraphProjection, String> {
    let nodes = graph
        .nodes
        .iter()
        .map(|node| node_wire(project, artifacts, operator_outputs, node))
        .collect::<Result<Vec<_>, _>>()?;
    let edges = graph
        .edges
        .iter()
        .map(|edge| {
            let source = graph
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node_id)
                .expect("validated graph source exists");
            let port = source
                .outputs
                .iter()
                .find(|port| port.id == edge.source_port_id)
                .expect("validated graph port exists");
            ffi::OperatorGraphEdgeWire {
                source_node_id: edge.source_node_id.to_string(),
                source_port_id: edge.source_port_id.to_string(),
                target_node_id: edge.target_node_id.to_string(),
                target_port_id: edge.target_port_id.to_string(),
                data_type_key: port.data_type.to_string(),
            }
        })
        .collect();
    Ok(OperatorGraphProjection { nodes, edges })
}

fn node_wire(
    project: &ShapeProject,
    artifacts: &[Artifact],
    operator_outputs: &HashMap<OperatorNodeId, (ArtifactId, RevisionId)>,
    node: &OperatorGraphNode,
) -> Result<ffi::OperatorGraphNodeWire, String> {
    let (artifact_id, artifact_name, revision_id, transformation_id, intent) = match node.binding {
        OperatorNodeBinding::Source {
            artifact_id: source_artifact_id,
            revision_id: source_revision_id,
        } => (
            source_artifact_id.to_string(),
            artifact_name_for(artifacts, source_artifact_id)?,
            source_revision_id,
            String::new(),
            String::new(),
        ),
        OperatorNodeBinding::Transformation {
            transformation_id: operator_transformation_id,
        } => {
            let (output_artifact_id, output_revision_id) = operator_outputs
                .get(&node.id)
                .ok_or_else(|| "operator output identity is missing".to_owned())?;
            let transformation = project
                .transformation(operator_transformation_id)
                .map_err(|error| error.to_string())?;
            (
                output_artifact_id.to_string(),
                artifact_name_for(artifacts, *output_artifact_id)?,
                *output_revision_id,
                operator_transformation_id.to_string(),
                transformation.intent.as_str().to_owned(),
            )
        }
        OperatorNodeBinding::Output {
            artifact_id: output_artifact_id,
            revision_id: output_revision_id,
        } => (
            output_artifact_id.to_string(),
            artifact_name_for(artifacts, output_artifact_id)?,
            output_revision_id,
            String::new(),
            String::new(),
        ),
    };
    let content = project
        .read_revision_content(revision_id)
        .map_err(|error| error.to_string())?;
    let (has_text_preview, text_preview, text_preview_truncated) =
        if content.revision.content.media_type.starts_with("text/") {
            match crate::bounded_text_preview(&content.bytes) {
                Some((preview, truncated)) => (true, preview, truncated),
                None => (false, String::new(), false),
            }
        } else {
            (false, String::new(), false)
        };
    Ok(ffi::OperatorGraphNodeWire {
        node_id: node.id.to_string(),
        role_key: node_role_key(node.role).to_owned(),
        operator_type_key: node.operator_type.to_string(),
        artifact_id,
        artifact_name,
        revision_id: revision_id.to_string(),
        transformation_id,
        intent,
        media_type: content.revision.content.media_type,
        byte_length: content.revision.content.byte_length,
        has_text_preview,
        text_preview_truncated,
        text_preview,
        input_ports: node.inputs.iter().map(port_wire).collect(),
        output_ports: node.outputs.iter().map(port_wire).collect(),
    })
}

fn port_wire(port: &OperatorPort) -> ffi::OperatorPortWire {
    ffi::OperatorPortWire {
        port_id: port.id.to_string(),
        data_type_key: port.data_type.to_string(),
    }
}

fn artifact_name_for(artifacts: &[Artifact], artifact_id: ArtifactId) -> Result<String, String> {
    artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id)
        .map(|artifact| artifact.name.clone())
        .ok_or_else(|| "operator graph artifact is missing".to_owned())
}

fn artifact_data_type(kind: ArtifactKind) -> Result<OperatorDataTypeId, String> {
    OperatorDataTypeId::new(match kind {
        ArtifactKind::TextDocument => TEXT_DOCUMENT_DATA_TYPE,
        ArtifactKind::ImageRaster => "image.raster",
        ArtifactKind::ImageComposite => "image.composite",
        ArtifactKind::ReferenceSet => "reference.set",
        ArtifactKind::AudioClip => AUDIO_CLIP_DATA_TYPE,
    })
    .map_err(|error| error.to_string())
}

fn operator_type(
    transformation: &Transformation,
    output_kind: ArtifactKind,
) -> Result<OperatorTypeId, String> {
    let identifier = match (&transformation.operation, transformation.kind, output_kind) {
        (Some(TransformationOperation::RasterCrop(_)), _, _) => IMAGE_CROP_OPERATOR_TYPE,
        (Some(TransformationOperation::RasterResize(_)), _, _) => IMAGE_RESIZE_OPERATOR_TYPE,
        (Some(TransformationOperation::RasterTransform(_)), _, _) => IMAGE_TRANSFORM_OPERATOR_TYPE,
        (Some(TransformationOperation::RasterGaussianBlur(_)), _, _) => IMAGE_BLUR_OPERATOR_TYPE,
        (Some(TransformationOperation::RasterDropShadow(_)), _, _) => {
            IMAGE_DROP_SHADOW_OPERATOR_TYPE
        }
        (Some(TransformationOperation::RasterUnsharpMask(_)), _, _) => {
            IMAGE_UNSHARP_MASK_OPERATOR_TYPE
        }
        (Some(TransformationOperation::AudioSpeechSynthesis(_)), _, _) => "audio.speech_synthesize",
        (Some(TransformationOperation::AiImageGenerate(_)), _, _) => "image.generate",
        (Some(TransformationOperation::AudioGenerate(_)), _, _) => "audio.generate",
        (_, TransformationKind::TextRewrite, _) => TEXT_EDIT_OPERATOR_TYPE,
        (_, TransformationKind::GenerativeEdit, ArtifactKind::TextDocument) => {
            TEXT_TRANSFORM_OPERATOR_TYPE
        }
        (_, TransformationKind::GenerativeEdit, _) => "creative.generate",
        (_, TransformationKind::DeterministicEdit, _) => "creative.deterministic_edit",
        (_, TransformationKind::Composite, _) => "creative.composite",
        (_, TransformationKind::ExternalRoundTrip, _) => "external.round_trip",
        (_, TransformationKind::Import, _) => "source.import",
    };
    OperatorTypeId::new(identifier).map_err(|error| error.to_string())
}

const fn node_role_key(role: OperatorNodeRole) -> &'static str {
    match role {
        OperatorNodeRole::Source => "source",
        OperatorNodeRole::Operator => "operator",
        OperatorNodeRole::Output => "output",
    }
}

#[cfg(test)]
mod tests;
