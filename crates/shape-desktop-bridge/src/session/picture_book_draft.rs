//! Book workspace checkpoints stay distinct from accepted scripts and the transient review shelf.
use super::{Candidate, DesktopSession, OperatorDrafts};

impl DesktopSession {
    pub fn session_book_draft_action_json(&mut self, request_json: &str) -> Result<String, String> {
        let result = self
            .project
            .picture_book_draft_action_json(request_json)
            .map_err(|e| e.to_string())?;
        self.operator_drafts = OperatorDrafts::from_graphs(
            self.project
                .artifact_working_graphs()
                .map_err(|e| e.to_string())?,
        )?;
        Ok(result)
    }
    pub fn session_book_draft_propose(
        &mut self,
        artifact_id: &str,
        draft_digest: &str,
    ) -> Result<String, String> {
        let artifact_id = artifact_id
            .parse()
            .map_err(|_| "invalid_book_draft_identity")?;
        let candidate = self
            .project
            .propose_picture_book_draft(artifact_id, draft_digest)
            .map_err(|e| e.to_string())?;
        let id = candidate.receipt().attempt_id.to_string();
        self.candidates.push(Candidate::Book(candidate));
        Ok(id)
    }
}

#[cfg(test)]
mod tests;
