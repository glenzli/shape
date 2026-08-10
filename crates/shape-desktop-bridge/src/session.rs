//! Mutable desktop lifecycle over one project and its transient candidate shelf.

mod candidate_shelf;

use shape_core::{ShapeProject, TextCandidate};
use shape_domain::{ArtifactId, ArtifactKind, IntentSpec};

use crate::{bounded_text_preview, ffi, project_snapshot};
use candidate_shelf::CandidateShelf;

use crate::infer_text::InferTextCandidate;

const USER_AUTHORED_TEXT_INTENT: &str = "Replace text with a user-authored draft";

/// One open desktop project and its transient text candidates.
///
/// The session never persists preview state. Proposals accumulate only after
/// successful execution. Consequential operations address one exact candidate;
/// a failed durable commit leaves the shelf available for retry.
#[derive(Debug)]
pub struct DesktopSession {
    project: ShapeProject,
    bundle_path: String,
    candidates: CandidateShelf,
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
        candidates: CandidateShelf::default(),
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
    /// pending candidates.
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
        if self.candidates.contains_text(artifact_id, replacement_text) {
            return Err("candidate text already exists on the shelf".to_owned());
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
        self.candidates.push(candidate);
        Ok(wire)
    }

    /// Returns all transient candidates in newest-first presentation order.
    pub fn session_text_candidates(&self) -> Vec<ffi::TextCandidateWire> {
        self.candidates.newest_first().map(candidate_wire).collect()
    }

    /// Explicitly accepts one candidate and returns the new durable snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity is unknown or the durable CAS commit
    /// fails. Failed acceptance retains every preview for inspection. A
    /// successful accepted-head change clears sibling candidates for that
    /// artifact because they were prepared against the previous head.
    pub fn session_accept_text(
        &mut self,
        candidate_id: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let candidate = self.candidates.clone_candidate(candidate_id)?;
        let artifact_id = candidate.artifact_id();
        self.project
            .accept_text(candidate)
            .map_err(|error| error.to_string())?;
        self.candidates.discard_artifact(artifact_id);
        self.session_snapshot()
    }

    /// Accepts one candidate as the first revision of a new artifact.
    ///
    /// Failed branching retains the candidate so the user can rename or retry;
    /// success removes only the chosen candidate because the source head is
    /// unchanged and its sibling candidates remain valid.
    pub fn session_branch_text(
        &mut self,
        candidate_id: &str,
        artifact_name: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let candidate = self.candidates.clone_candidate(candidate_id)?;
        self.project
            .branch_text_candidate(candidate, artifact_name)
            .map_err(|error| error.to_string())?;
        self.candidates.discard(candidate_id)?;
        self.session_snapshot()
    }

    /// Discards one transient preview without touching durable history.
    pub fn session_discard_text(&mut self, candidate_id: &str) -> Result<(), String> {
        self.candidates.discard(candidate_id)
    }

    /// Adopts a completed Infer candidate after rechecking the live project
    /// head and Candidate Shelf identity on the desktop thread.
    pub fn session_adopt_infer_text(
        &mut self,
        candidate: Box<InferTextCandidate>,
    ) -> Result<ffi::TextCandidateWire, String> {
        let candidate = (*candidate).into_candidate();
        let artifact_id = candidate.artifact_id();
        let snapshot = self.project.snapshot().map_err(|_| "project_unavailable")?;
        let artifact = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or("invalid_artifact")?;
        if artifact.accepted_revision != candidate.expected_head() {
            return Err("stale_candidate".to_owned());
        }
        if self.candidates.contains_text(artifact_id, candidate.text()) {
            return Err("duplicate_candidate".to_owned());
        }
        let wire = candidate_wire(&candidate);
        self.candidates.push(candidate);
        Ok(wire)
    }
}

fn candidate_id(candidate: &TextCandidate) -> String {
    candidate.receipt().attempt_id.to_string()
}

fn candidate_wire(candidate: &TextCandidate) -> ffi::TextCandidateWire {
    let (text_preview, text_preview_truncated) =
        bounded_text_preview(candidate.text().as_bytes()).expect("text candidates are valid UTF-8");
    let expected_head = candidate.expected_head();
    ffi::TextCandidateWire {
        candidate_id: candidate_id(candidate),
        artifact_id: candidate.artifact_id().to_string(),
        has_expected_head: expected_head.is_some(),
        expected_head: expected_head.map_or_else(String::new, |head| head.to_string()),
        text_preview_truncated,
        text_preview,
    }
}

#[cfg(test)]
mod tests;
