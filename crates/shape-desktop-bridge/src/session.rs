//! Mutable desktop lifecycle over one project and one transient text candidate.

use shape_core::{ShapeProject, TextCandidate};
use shape_domain::{ArtifactId, ArtifactKind, IntentSpec};

use crate::{bounded_text_preview, ffi, project_snapshot};

const USER_AUTHORED_TEXT_INTENT: &str = "Replace text with a user-authored draft";

/// One open desktop project and its single replaceable transient candidate.
///
/// The session never persists preview state. A proposal replaces the previous
/// candidate only after successful execution; acceptance clones the pending
/// value so a failed durable commit leaves the preview available for retry.
#[derive(Debug)]
pub struct DesktopSession {
    project: ShapeProject,
    bundle_path: String,
    pending_text: Option<TextCandidate>,
}

/// Opens a validated project for mutable desktop use.
///
/// # Errors
///
/// Returns a user-safe message when the project cannot be opened.
pub fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>, String> {
    let project = ShapeProject::open(path).map_err(|error| error.to_string())?;
    Ok(Box::new(DesktopSession {
        project,
        bundle_path: super::project_path(path),
        pending_text: None,
    }))
}

impl DesktopSession {
    /// Returns the durable accepted project state; pending preview is excluded.
    pub fn session_snapshot(&self) -> Result<ffi::ProjectSnapshotWire, String> {
        project_snapshot(&self.project, &self.bundle_path)
    }

    /// Executes a literal user-authored text draft as a transient candidate.
    ///
    /// # Errors
    ///
    /// Rejects invalid identities, non-text artifacts, no-op drafts, stale
    /// accepted heads, and execution failures without replacing an existing
    /// pending candidate.
    pub fn session_propose_text(
        &mut self,
        artifact_id: &str,
        replacement_text: &str,
    ) -> Result<ffi::TextCandidateWire, String> {
        let artifact_id = artifact_id
            .parse::<ArtifactId>()
            .map_err(|_| "artifact identity is invalid".to_owned())?;
        let snapshot = self.project.snapshot().map_err(|error| error.to_string())?;
        let artifact = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or_else(|| "artifact does not exist in this project".to_owned())?;
        if artifact.kind != ArtifactKind::TextDocument {
            return Err("only text documents support this desktop candidate flow".to_owned());
        }
        if self
            .project
            .read_accepted(artifact_id)
            .map_err(|error| error.to_string())?
            .is_some_and(|accepted| accepted.bytes == replacement_text.as_bytes())
        {
            return Err("candidate text matches the current accepted text".to_owned());
        }

        let candidate = self
            .project
            .propose_text(
                artifact_id,
                artifact.accepted_revision,
                replacement_text,
                IntentSpec::new(USER_AUTHORED_TEXT_INTENT).map_err(|error| error.to_string())?,
                Vec::new(),
            )
            .map_err(|error| error.to_string())?;
        let wire = candidate_wire(&candidate);
        self.pending_text = Some(candidate);
        Ok(wire)
    }

    /// Explicitly accepts the pending candidate and returns the new snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when no candidate exists or the durable CAS commit
    /// fails. Failed acceptance retains the pending preview for inspection.
    pub fn session_accept_text(&mut self) -> Result<ffi::ProjectSnapshotWire, String> {
        let candidate = self
            .pending_text
            .clone()
            .ok_or_else(|| "there is no text candidate to accept".to_owned())?;
        self.project
            .accept_text(candidate)
            .map_err(|error| error.to_string())?;
        self.pending_text = None;
        self.session_snapshot()
    }

    /// Accepts the pending candidate as the first revision of a new artifact.
    ///
    /// Failed branching retains the candidate so the user can rename or retry.
    pub fn session_branch_text(
        &mut self,
        artifact_name: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let candidate = self
            .pending_text
            .clone()
            .ok_or_else(|| "there is no text candidate to branch".to_owned())?;
        self.project
            .branch_text_candidate(candidate, artifact_name)
            .map_err(|error| error.to_string())?;
        self.pending_text = None;
        self.session_snapshot()
    }

    /// Discards transient preview without touching durable history.
    pub fn session_discard_text(&mut self) {
        self.pending_text = None;
    }
}

fn candidate_wire(candidate: &TextCandidate) -> ffi::TextCandidateWire {
    let (text_preview, text_preview_truncated) =
        bounded_text_preview(candidate.text().as_bytes()).expect("text candidates are valid UTF-8");
    let expected_head = candidate.expected_head();
    ffi::TextCandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        has_expected_head: expected_head.is_some(),
        expected_head: expected_head.map_or_else(String::new, |head| head.to_string()),
        text_preview_truncated,
        text_preview,
    }
}

#[cfg(test)]
mod tests;
