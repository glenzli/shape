//! Book workspace checkpoints stay distinct from accepted scripts and the transient review shelf.
use super::DesktopSession;

impl DesktopSession {
    pub fn session_book_draft_action_json(&mut self, request_json: &str) -> Result<String, String> {
        let result = self
            .project
            .picture_book_draft_action_json(request_json)
            .map_err(|e| e.to_string())?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
