//! Bounded desktop session and projection over a real Shape project.
//!
//! Rust owns project validation, `SQLite`, object verification, and domain
//! interpretation. The session owns one transient candidate at a time; C++
//! receives explicit presence flags and presentation-safe values through one
//! generated CXX contract. QML never reads or writes project files directly.

mod session;

use shape_core::ShapeProject;
use shape_domain::{Artifact, ArtifactKind};

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
    }

    /// Explicit desktop projection of one current artifact head.
    #[derive(Debug)]
    struct ArtifactSummaryWire {
        id: String,
        name: String,
        kind_key: String,
        has_accepted_revision: bool,
        accepted_revision_id: String,
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

    extern "Rust" {
        type DesktopSession;

        /// Opens and validates one `.shape` bundle, then returns a bounded
        /// read-only snapshot for the desktop shell.
        fn load_project_snapshot(path: &str) -> Result<ProjectSnapshotWire>;

        /// Opens one mutable desktop session. The session remains the sole
        /// owner of any transient candidate and the underlying project.
        fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>>;

        fn session_snapshot(self: &DesktopSession) -> Result<ProjectSnapshotWire>;
        fn session_propose_text(
            self: &mut DesktopSession,
            artifact_id: &str,
            replacement_text: &str,
        ) -> Result<TextCandidateWire>;
        fn session_accept_text(self: &mut DesktopSession) -> Result<ProjectSnapshotWire>;
        fn session_discard_text(self: &mut DesktopSession);
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
        .map(|artifact| project_artifact(project, artifact))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ffi::ProjectSnapshotWire {
        project_id: snapshot.metadata.id.to_string(),
        project_name: snapshot.metadata.name,
        schema_revision: snapshot.metadata.schema_revision,
        bundle_path: project_path(path),
        artifacts,
    })
}

fn project_artifact(
    project: &ShapeProject,
    artifact: &Artifact,
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
        has_content: false,
        content_digest: String::new(),
        media_type: String::new(),
        byte_length: 0,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
    };
    if let Some(content) = accepted {
        wire.has_accepted_revision = true;
        wire.accepted_revision_id = content.revision.id.to_string();
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

fn project_path(path: &str) -> String {
    std::path::Path::new(path).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests;
