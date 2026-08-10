//! Bounded desktop session and projection over a real Shape project.
//!
//! Rust owns project validation, `SQLite`, object verification, and domain
//! interpretation. The session owns a transient candidate shelf; C++
//! receives explicit presence flags and presentation-safe values through one
//! generated CXX contract. QML never reads or writes project files directly.

mod session;

use shape_core::ShapeProject;
use shape_domain::{Artifact, ArtifactKind, TransformationKind};
use shape_execution::{InferRuntimeClient, InferRuntimeClientError, InferRuntimeContract};

use session::{DesktopSession, open_desktop_session};

const MAX_TEXT_PREVIEW_BYTES: usize = 32 * 1024;

#[cxx::bridge(namespace = "shape::desktop")]
mod ffi {
    /// Bounded project-level presentation snapshot.
    #[derive(Debug)]
    struct ProjectSnapshotWire {
        project_id: String,
        project_name: String,
        schema_revision: String,
        bundle_path: String,
        artifacts: Vec<ArtifactSummaryWire>,
        graph_edges: Vec<ProjectGraphEdgeWire>,
    }

    /// One accepted cross-artifact derivation in the current project graph.
    #[derive(Debug)]
    struct ProjectGraphEdgeWire {
        source_artifact_id: String,
        target_artifact_id: String,
        source_revision_id: String,
        target_revision_id: String,
        transformation_id: String,
        transformation_kind_key: String,
    }

    /// Explicit desktop projection of one current artifact head.
    #[derive(Debug)]
    struct ArtifactSummaryWire {
        id: String,
        name: String,
        kind_key: String,
        has_accepted_revision: bool,
        accepted_revision_id: String,
        accepted_parent_revision_ids: Vec<String>,
        transformation_id: String,
        transformation_kind_key: String,
        transformation_intent: String,
        transformation_input_revision_ids: Vec<String>,
        transformation_input_artifact_ids: Vec<String>,
        transformation_input_artifact_names: Vec<String>,
        constraint_count: u64,
        reference_count: u64,
        has_content: bool,
        content_digest: String,
        media_type: String,
        byte_length: u64,
        has_text_preview: bool,
        text_preview_truncated: bool,
        text_preview: String,
    }

    /// Transient text candidate projection. Candidate bytes are not durable
    /// history until `session_accept_text` succeeds.
    #[derive(Debug)]
    struct TextCandidateWire {
        candidate_id: String,
        artifact_id: String,
        has_expected_head: bool,
        expected_head: String,
        text_preview_truncated: bool,
        text_preview: String,
    }

    /// Public Infer Runtime contract availability. Error codes are stable and
    /// language-neutral; credentials and response payloads never cross here.
    #[derive(Debug)]
    struct InferRuntimeProbeWire {
        reachable: bool,
        compatible: bool,
        contract_version: String,
        error_code: String,
    }

    extern "Rust" {
        type DesktopSession;

        /// Opens and validates one `.shape` bundle, then returns a bounded
        /// read-only snapshot for the desktop shell.
        fn load_project_snapshot(path: &str) -> Result<ProjectSnapshotWire>;

        /// Probes only the unauthenticated public consumer contract endpoint.
        fn probe_infer_runtime(base_url: &str) -> InferRuntimeProbeWire;

        /// Opens one mutable desktop session. The session remains the sole
        /// owner of transient candidates and the underlying project.
        fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>>;

        fn session_snapshot(self: &DesktopSession) -> Result<ProjectSnapshotWire>;
        fn session_propose_text(
            self: &mut DesktopSession,
            artifact_id: &str,
            replacement_text: &str,
        ) -> Result<TextCandidateWire>;
        fn session_text_candidates(self: &DesktopSession) -> Vec<TextCandidateWire>;
        fn session_accept_text(
            self: &mut DesktopSession,
            candidate_id: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_branch_text(
            self: &mut DesktopSession,
            candidate_id: &str,
            artifact_name: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_discard_text(self: &mut DesktopSession, candidate_id: &str) -> Result<()>;
    }
}

fn probe_infer_runtime(base_url: &str) -> ffi::InferRuntimeProbeWire {
    let result = InferRuntimeClient::new(base_url).and_then(|client| client.probe_contract());
    infer_runtime_probe_wire(result)
}

fn infer_runtime_probe_wire(
    result: Result<InferRuntimeContract, InferRuntimeClientError>,
) -> ffi::InferRuntimeProbeWire {
    match result {
        Ok(contract) => ffi::InferRuntimeProbeWire {
            reachable: true,
            compatible: true,
            contract_version: contract.contract_version,
            error_code: String::new(),
        },
        Err(error) => {
            let reachable = !matches!(
                error,
                InferRuntimeClientError::InvalidEndpoint | InferRuntimeClientError::Unavailable
            );
            let contract_version = match &error {
                InferRuntimeClientError::IncompatibleContract { actual } => actual.clone(),
                _ => String::new(),
            };
            ffi::InferRuntimeProbeWire {
                reachable,
                compatible: false,
                contract_version,
                error_code: error.code().to_owned(),
            }
        }
    }
}

/// Opens one project and projects its current accepted state for the desktop.
///
/// # Errors
///
/// Returns a user-safe message when the bundle cannot be validated or read.
fn load_project_snapshot(path: &str) -> Result<ffi::ProjectSnapshotWire, String> {
    let project = ShapeProject::open(path).map_err(|error| error.to_string())?;
    project_snapshot(&project, path)
}

fn project_snapshot(
    project: &ShapeProject,
    path: &str,
) -> Result<ffi::ProjectSnapshotWire, String> {
    let snapshot = project.snapshot().map_err(|error| error.to_string())?;
    let artifacts = snapshot
        .artifacts
        .iter()
        .map(|artifact| project_artifact(project, artifact, &snapshot.artifacts))
        .collect::<Result<Vec<_>, _>>()?;
    let graph_edges = project_graph_edges(&artifacts);
    Ok(ffi::ProjectSnapshotWire {
        project_id: snapshot.metadata.id.to_string(),
        project_name: snapshot.metadata.name,
        schema_revision: snapshot.metadata.schema_revision,
        bundle_path: project_path(path),
        artifacts,
        graph_edges,
    })
}

fn project_graph_edges(artifacts: &[ffi::ArtifactSummaryWire]) -> Vec<ffi::ProjectGraphEdgeWire> {
    let mut edges: Vec<ffi::ProjectGraphEdgeWire> = Vec::new();
    for target in artifacts {
        for (index, source_artifact_id) in
            target.transformation_input_artifact_ids.iter().enumerate()
        {
            if source_artifact_id == &target.id {
                continue;
            }
            let Some(source_revision_id) = target.transformation_input_revision_ids.get(index)
            else {
                continue;
            };
            let edge = ffi::ProjectGraphEdgeWire {
                source_artifact_id: source_artifact_id.clone(),
                target_artifact_id: target.id.clone(),
                source_revision_id: source_revision_id.clone(),
                target_revision_id: target.accepted_revision_id.clone(),
                transformation_id: target.transformation_id.clone(),
                transformation_kind_key: target.transformation_kind_key.clone(),
            };
            let already_projected = edges.iter().any(|existing| {
                existing.source_revision_id == edge.source_revision_id
                    && existing.target_revision_id == edge.target_revision_id
                    && existing.transformation_id == edge.transformation_id
            });
            if !already_projected {
                edges.push(edge);
            }
        }
    }
    edges
}

fn project_artifact(
    project: &ShapeProject,
    artifact: &Artifact,
    project_artifacts: &[Artifact],
) -> Result<ffi::ArtifactSummaryWire, String> {
    let accepted = project
        .read_accepted(artifact.id)
        .map_err(|error| error.to_string())?;
    let mut wire = ffi::ArtifactSummaryWire {
        id: artifact.id.to_string(),
        name: artifact.name.clone(),
        kind_key: artifact_kind_key(artifact.kind).to_owned(),
        has_accepted_revision: false,
        accepted_revision_id: String::new(),
        accepted_parent_revision_ids: Vec::new(),
        transformation_id: String::new(),
        transformation_kind_key: String::new(),
        transformation_intent: String::new(),
        transformation_input_revision_ids: Vec::new(),
        transformation_input_artifact_ids: Vec::new(),
        transformation_input_artifact_names: Vec::new(),
        constraint_count: 0,
        reference_count: 0,
        has_content: false,
        content_digest: String::new(),
        media_type: String::new(),
        byte_length: 0,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
    };
    if let Some(content) = accepted {
        let transformation = project
            .transformation(content.revision.transformation_id)
            .map_err(|error| error.to_string())?;
        wire.has_accepted_revision = true;
        wire.accepted_revision_id = content.revision.id.to_string();
        wire.accepted_parent_revision_ids = content
            .revision
            .parents
            .iter()
            .map(ToString::to_string)
            .collect();
        wire.transformation_id = transformation.id.to_string();
        transformation_kind_key(transformation.kind).clone_into(&mut wire.transformation_kind_key);
        transformation
            .intent
            .as_str()
            .clone_into(&mut wire.transformation_intent);
        wire.transformation_input_revision_ids = transformation
            .inputs
            .iter()
            .map(ToString::to_string)
            .collect();
        for input in &transformation.inputs {
            let input_revision = project
                .revision(*input)
                .map_err(|error| error.to_string())?;
            let input_artifact = project_artifacts
                .iter()
                .find(|artifact| artifact.id == input_revision.artifact_id)
                .ok_or_else(|| "transformation input artifact is missing".to_owned())?;
            wire.transformation_input_artifact_ids
                .push(input_artifact.id.to_string());
            wire.transformation_input_artifact_names
                .push(input_artifact.name.clone());
        }
        wire.constraint_count = transformation.constraints.len() as u64;
        wire.reference_count = transformation.references.len() as u64;
        wire.has_content = true;
        wire.content_digest = content.revision.content.digest.to_string();
        wire.media_type = content.revision.content.media_type;
        wire.byte_length = content.revision.content.byte_length;
        if wire.media_type.starts_with("text/")
            && let Some((preview, truncated)) = bounded_text_preview(&content.bytes)
        {
            wire.has_text_preview = true;
            wire.text_preview_truncated = truncated;
            wire.text_preview = preview;
        }
    }
    Ok(wire)
}

fn bounded_text_preview(bytes: &[u8]) -> Option<(String, bool)> {
    let text = std::str::from_utf8(bytes).ok()?;
    if bytes.len() <= MAX_TEXT_PREVIEW_BYTES {
        return Some((text.to_owned(), false));
    }
    let mut end = MAX_TEXT_PREVIEW_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    Some((text[..end].to_owned(), true))
}

const fn artifact_kind_key(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::TextDocument => "text_document",
        ArtifactKind::ImageRaster => "image_raster",
        ArtifactKind::ImageComposite => "image_composite",
        ArtifactKind::ReferenceSet => "reference_set",
    }
}

const fn transformation_kind_key(kind: TransformationKind) -> &'static str {
    match kind {
        TransformationKind::Import => "import",
        TransformationKind::TextRewrite => "text_rewrite",
        TransformationKind::DeterministicEdit => "deterministic_edit",
        TransformationKind::GenerativeEdit => "generative_edit",
        TransformationKind::Composite => "composite",
        TransformationKind::ExternalRoundTrip => "external_round_trip",
    }
}

fn project_path(path: &str) -> String {
    std::path::Path::new(path).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
