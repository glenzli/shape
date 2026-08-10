//! Transient candidate collection with stable identity-addressed mutation.

use shape_core::TextCandidate;
use shape_domain::ArtifactId;

/// One in-memory shelf of executed candidates, newest first at projection time.
///
/// The shelf never persists candidate bytes. Exact candidate identity is required
/// for consequential operations so a selection change cannot accept or discard
/// a different preview.
#[derive(Debug, Default)]
pub(super) struct CandidateShelf {
    candidates: Vec<TextCandidate>,
}

impl CandidateShelf {
    pub(super) fn push(&mut self, candidate: TextCandidate) {
        debug_assert!(
            self.candidates
                .iter()
                .all(|existing| candidate_id(existing) != candidate_id(&candidate))
        );
        self.candidates.push(candidate);
    }

    pub(super) fn newest_first(&self) -> impl Iterator<Item = &TextCandidate> {
        self.candidates.iter().rev()
    }

    pub(super) fn contains_text(&self, artifact_id: ArtifactId, text: &str) -> bool {
        self.candidates
            .iter()
            .any(|candidate| candidate.artifact_id() == artifact_id && candidate.text() == text)
    }

    pub(super) fn clone_candidate(&self, candidate_id: &str) -> Result<TextCandidate, String> {
        self.candidates
            .iter()
            .find(|candidate| self::candidate_id(candidate) == candidate_id)
            .cloned()
            .ok_or_else(|| "text candidate does not exist in this session".to_owned())
    }

    pub(super) fn discard(&mut self, candidate_id: &str) -> Result<(), String> {
        let index = self
            .candidates
            .iter()
            .position(|candidate| self::candidate_id(candidate) == candidate_id)
            .ok_or_else(|| "text candidate does not exist in this session".to_owned())?;
        self.candidates.remove(index);
        Ok(())
    }

    pub(super) fn discard_artifact(&mut self, artifact_id: ArtifactId) {
        self.candidates
            .retain(|candidate| candidate.artifact_id() != artifact_id);
    }
}

fn candidate_id(candidate: &TextCandidate) -> String {
    candidate.receipt().attempt_id.to_string()
}

#[cfg(test)]
mod tests;
