//! Bounded desktop session and projection over a real Shape project.
//!
//! Rust owns project validation, `SQLite`, object verification, and domain
//! interpretation. The session owns a transient candidate shelf; C++
//! receives explicit presence flags and presentation-safe values through one
//! generated CXX contract. QML never reads or writes project files directly.

mod infer_runtime_access;
mod infer_speech;
mod infer_text;
mod operator_graph;
mod session;

use shape_core::ShapeProject;
use shape_domain::{
    Artifact, ArtifactContentContract, ArtifactKind, AudioOriginDisclosure, TransformationKind,
};
use shape_execution::{InferRuntimeClientError, InferRuntimeProbe, probe_infer_runtime_contract};

use infer_runtime_access::{infer_runtime_credential_status, install_infer_runtime_credential};
use infer_speech::{InferSpeechCandidate, generate_infer_speech_candidate};
use infer_text::{InferTextCandidate, generate_infer_text_candidate};
use operator_graph::project_operator_graph;
use session::{DesktopSession, create_desktop_project, open_desktop_session};

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

    /// One typed input or output port in the user-visible Operator Graph.
    #[derive(Debug)]
    struct OperatorPortWire {
        port_id: String,
        data_type_key: String,
    }

    /// One accepted Source, Operator, or Output node in a scene-compatible graph.
    #[derive(Debug)]
    struct OperatorGraphNodeWire {
        node_id: String,
        role_key: String,
        operator_type_key: String,
        artifact_id: String,
        artifact_name: String,
        revision_id: String,
        transformation_id: String,
        intent: String,
        input_ports: Vec<OperatorPortWire>,
        output_ports: Vec<OperatorPortWire>,
    }

    /// One exact typed connection between visible Operator Graph ports.
    #[derive(Debug)]
    struct OperatorGraphEdgeWire {
        source_node_id: String,
        source_port_id: String,
        target_node_id: String,
        target_port_id: String,
        data_type_key: String,
    }

    /// One session-local Operator entry awaiting its first real Candidate.
    #[derive(Debug)]
    struct OperatorDraftWire {
        draft_id: String,
        context_artifact_id: String,
        operator_type_key: String,
        input_data_type_key: String,
        output_data_type_key: String,
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
        has_image_preview: bool,
        image_width: u32,
        image_height: u32,
        has_audio_preview: bool,
        audio_duration_millis: u64,
        audio_sample_rate_hz: u32,
        audio_channels: u16,
        audio_origin_key: String,
        operator_graph_nodes: Vec<OperatorGraphNodeWire>,
        operator_graph_edges: Vec<OperatorGraphEdgeWire>,
    }

    /// Transient cross-media candidate projection. Large image bytes are
    /// fetched separately only for the selected preview.
    #[derive(Debug)]
    struct CandidateWire {
        candidate_id: String,
        artifact_id: String,
        context_artifact_id: String,
        artifact_name: String,
        kind_key: String,
        has_expected_head: bool,
        expected_head: String,
        can_branch: bool,
        has_text_preview: bool,
        text_preview_truncated: bool,
        text_preview: String,
        has_image_preview: bool,
        image_width: u32,
        image_height: u32,
        has_audio_preview: bool,
        audio_duration_millis: u64,
        audio_sample_rate_hz: u32,
        audio_channels: u16,
        audio_origin_key: String,
    }

    /// One on-demand selected raster preview. Bytes never enter QML strings.
    #[derive(Debug)]
    struct ImagePreviewWire {
        identity: String,
        width: u32,
        height: u32,
        png_bytes: Vec<u8>,
    }

    /// One on-demand selected audio preview. Exact WAV bytes cross only for
    /// the accepted clip or transient Candidate currently being auditioned.
    #[derive(Debug)]
    struct AudioPreviewWire {
        identity: String,
        duration_millis: u64,
        sample_rate_hz: u32,
        channels: u16,
        wav_bytes: Vec<u8>,
    }

    /// Public Infer Runtime contract availability. Error codes are stable and
    /// language-neutral; credentials and response payloads never cross here.
    #[derive(Debug)]
    struct InferRuntimeProbeWire {
        reachable: bool,
        compatible: bool,
        contract_version: String,
        error_code: String,
        endpoint_origin: String,
        endpoint_source: String,
        runtime_instance_id: String,
        runtime_generation: String,
    }

    /// Non-secret readiness of Shape's managed Infer credential copy.
    #[derive(Debug)]
    struct InferRuntimeCredentialStatusWire {
        configured: bool,
        error_code: String,
    }

    extern "Rust" {
        type DesktopSession;
        type InferSpeechCandidate;
        type InferTextCandidate;

        /// Opens and validates one `.shape` bundle, then returns a bounded
        /// read-only snapshot for the desktop shell.
        fn load_project_snapshot(path: &str) -> Result<ProjectSnapshotWire>;

        /// Probes only the unauthenticated public consumer contract endpoint.
        fn probe_infer_runtime(base_url: &str) -> InferRuntimeProbeWire;

        /// Checks only owner/permission/format status; token bytes never cross.
        fn infer_runtime_credential_status(path: &str) -> InferRuntimeCredentialStatusWire;

        /// Atomically installs a one-time managed token in Shape's secret store.
        fn install_infer_runtime_credential(path: &str, token: &str) -> Result<()>;

        /// Runs authenticated generation outside the live desktop session.
        fn generate_infer_text_candidate(
            project_path: &str,
            artifact_id: &str,
            prompt: &str,
            credential_path: &str,
            explicit_override: &str,
        ) -> Result<Box<InferTextCandidate>>;

        /// Runs preset-only speech synthesis outside the live desktop session.
        fn generate_infer_speech_candidate(
            project_path: &str,
            source_artifact_id: &str,
            artifact_name: &str,
            speed_milli: u16,
            credential_path: &str,
            explicit_override: &str,
        ) -> Result<Box<InferSpeechCandidate>>;

        /// Opens one mutable desktop session. The session remains the sole
        /// owner of transient candidates and the underlying project.
        fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>>;

        /// Creates one new empty `.shape` bundle and opens its desktop session.
        fn create_desktop_project(path: &str, name: &str) -> Result<Box<DesktopSession>>;

        fn session_snapshot(self: &DesktopSession) -> Result<ProjectSnapshotWire>;
        fn session_create_text_document(
            self: &mut DesktopSession,
            artifact_name: &str,
            initial_text: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_begin_operator_draft(
            self: &mut DesktopSession,
            artifact_id: &str,
            operator_type: &str,
        ) -> Result<OperatorDraftWire>;
        fn session_operator_drafts(self: &DesktopSession) -> Vec<OperatorDraftWire>;
        fn session_discard_operator_draft(self: &mut DesktopSession, draft_id: &str) -> Result<()>;
        fn session_import_raster(
            self: &mut DesktopSession,
            source_path: &str,
            artifact_name: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_propose_text(
            self: &mut DesktopSession,
            artifact_id: &str,
            replacement_text: &str,
        ) -> Result<CandidateWire>;
        fn session_propose_raster_crop(
            self: &mut DesktopSession,
            artifact_id: &str,
            x: u32,
            y: u32,
            width: u32,
            height: u32,
        ) -> Result<CandidateWire>;
        fn session_candidates(self: &DesktopSession) -> Vec<CandidateWire>;
        fn session_accept_candidate(
            self: &mut DesktopSession,
            candidate_id: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_branch_candidate(
            self: &mut DesktopSession,
            candidate_id: &str,
            artifact_name: &str,
        ) -> Result<ProjectSnapshotWire>;
        fn session_discard_candidate(self: &mut DesktopSession, candidate_id: &str) -> Result<()>;
        fn session_image_preview(
            self: &DesktopSession,
            artifact_id: &str,
            candidate_id: &str,
        ) -> Result<ImagePreviewWire>;
        fn session_audio_preview(
            self: &DesktopSession,
            artifact_id: &str,
            candidate_id: &str,
        ) -> Result<AudioPreviewWire>;
        /// Adopts one completed background result only if its target head is
        /// still current and it is not already on the Candidate Shelf.
        fn session_adopt_infer_text(
            self: &mut DesktopSession,
            candidate: Box<InferTextCandidate>,
        ) -> Result<CandidateWire>;
        /// Adopts one completed speech result only if its source head remains current.
        fn session_adopt_infer_speech(
            self: &mut DesktopSession,
            candidate: Box<InferSpeechCandidate>,
        ) -> Result<CandidateWire>;
    }
}

fn probe_infer_runtime(explicit_override: &str) -> ffi::InferRuntimeProbeWire {
    infer_runtime_probe_wire(probe_infer_runtime_contract(explicit_override))
}

fn infer_runtime_probe_wire(probe: InferRuntimeProbe) -> ffi::InferRuntimeProbeWire {
    let endpoint_origin = probe
        .endpoint
        .as_ref()
        .map_or_else(String::new, |endpoint| endpoint.origin.clone());
    let endpoint_source = probe
        .endpoint
        .as_ref()
        .map_or_else(String::new, |endpoint| endpoint.source.code().to_owned());
    let runtime_instance_id = probe
        .endpoint
        .as_ref()
        .and_then(|endpoint| endpoint.instance_id.clone())
        .unwrap_or_default();
    let runtime_generation = probe
        .endpoint
        .as_ref()
        .and_then(|endpoint| endpoint.generation.clone())
        .unwrap_or_default();
    match probe.contract {
        Ok(contract) => ffi::InferRuntimeProbeWire {
            reachable: true,
            compatible: true,
            contract_version: contract.contract_version,
            error_code: String::new(),
            endpoint_origin,
            endpoint_source,
            runtime_instance_id,
            runtime_generation,
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
                endpoint_origin,
                endpoint_source,
                runtime_instance_id,
                runtime_generation,
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
        has_image_preview: false,
        image_width: 0,
        image_height: 0,
        has_audio_preview: false,
        audio_duration_millis: 0,
        audio_sample_rate_hz: 0,
        audio_channels: 0,
        audio_origin_key: String::new(),
        operator_graph_nodes: Vec::new(),
        operator_graph_edges: Vec::new(),
    };
    if let Some(revision_id) = artifact.accepted_revision {
        let revision = project
            .revision(revision_id)
            .map_err(|error| error.to_string())?;
        let transformation = project
            .transformation(revision.transformation_id)
            .map_err(|error| error.to_string())?;
        wire.has_accepted_revision = true;
        wire.accepted_revision_id = revision.id.to_string();
        wire.accepted_parent_revision_ids =
            revision.parents.iter().map(ToString::to_string).collect();
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
        wire.content_digest = revision.content.digest.to_string();
        wire.media_type.clone_from(&revision.content.media_type);
        wire.byte_length = revision.content.byte_length;
        if wire.media_type.starts_with("text/") {
            let content = project
                .read_accepted(artifact.id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "accepted revision disappeared while projecting it".to_owned())?;
            if let Some((preview, truncated)) = bounded_text_preview(&content.bytes) {
                wire.has_text_preview = true;
                wire.text_preview_truncated = truncated;
                wire.text_preview = preview;
            }
        }
        project_content_contract(&mut wire, revision.content_contract.as_ref());
        let graph = project_operator_graph(project, artifact, project_artifacts)?;
        wire.operator_graph_nodes = graph.nodes;
        wire.operator_graph_edges = graph.edges;
    }
    Ok(wire)
}

fn project_content_contract(
    wire: &mut ffi::ArtifactSummaryWire,
    contract: Option<&ArtifactContentContract>,
) {
    match contract {
        Some(ArtifactContentContract::ImageRaster(contract)) => {
            wire.has_image_preview = true;
            wire.image_width = contract.width;
            wire.image_height = contract.height;
        }
        Some(ArtifactContentContract::AudioClip(contract)) => {
            wire.has_audio_preview = true;
            wire.audio_duration_millis = contract.duration_millis();
            wire.audio_sample_rate_hz = contract.sample_rate_hz;
            wire.audio_channels = contract.channels;
            audio_origin_key(contract.origin).clone_into(&mut wire.audio_origin_key);
        }
        None => {}
    }
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
        ArtifactKind::AudioClip => "audio_clip",
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

const fn audio_origin_key(origin: AudioOriginDisclosure) -> &'static str {
    match origin {
        AudioOriginDisclosure::RecordedSource => "recorded_source",
        AudioOriginDisclosure::SyntheticSpeech => "synthetic_speech",
        AudioOriginDisclosure::SyntheticSound => "synthetic_sound",
        AudioOriginDisclosure::TransformedAudio => "transformed_audio",
    }
}

fn project_path(path: &str) -> String {
    std::path::Path::new(path).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
