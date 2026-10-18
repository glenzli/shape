//! Narrow card authoring channel; the core owns draft CAS, review and export publication.
use super::DesktopSession;

impl DesktopSession {
    pub fn session_card_kit_action_json(&mut self, request_json: &str) -> Result<String, String> {
        let result = self
            .project
            .card_kit_action_json(request_json)
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn session_legacy_snapshot_json(&self) -> Result<String, String> {
        self.project
            .legacy_snapshot_json()
            .map_err(|error| error.to_string())
    }
    pub fn session_vox_transfer_json(&self, request_json: &str) -> Result<String, String> {
        self.project
            .export_vox_transfer_json(request_json)
            .map_err(|error| error.to_string())
    }

    // The CXX terminal adapter intentionally exposes a single project-session method channel.
    #[allow(clippy::unused_self)]
    pub fn session_workflow_templates_json(&self) -> Result<String, String> {
        serde_json::to_string(&serde_json::json!({"schema":"shape.workflow-templates/1","templates":shape_domain::workflow_templates()})).map_err(|e|e.to_string())
    }
}

#[cfg(test)]
mod tests;
