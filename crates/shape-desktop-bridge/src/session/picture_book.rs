//! Bounded book authoring adapter over the shared candidate shelf and immutable materials.
use super::DesktopSession;
use shape_domain::{ArtifactKind, WorkingInput};

fn target(value: &str) -> Result<WorkingInput, String> {
    serde_json::from_str(value).map_err(|_| "invalid_book_identity".into())
}
fn json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

impl DesktopSession {
    pub fn session_book_draft_export(
        &self,
        artifact_id: &str,
        draft_digest: &str,
        destination: &str,
    ) -> Result<String, String> {
        json(
            &self
                .project
                .export_legacy_picture_book_draft(
                    artifact_id.parse().map_err(|_| "invalid_book_identity")?,
                    draft_digest,
                    std::path::Path::new(destination),
                )
                .map_err(|e| e.to_string())?,
        )
    }
    pub fn session_book_read(&self, source: &str) -> Result<String, String> {
        json(
            &self
                .project
                .read_picture_book(target(source)?)
                .map_err(|e| e.to_string())?,
        )
    }
    pub fn session_book_export(&self, source: &str, destination: &str) -> Result<String, String> {
        json(
            &self
                .project
                .export_accepted_picture_book(target(source)?, std::path::Path::new(destination))
                .map_err(|e| e.to_string())?,
        )
    }
    pub fn session_book_material(&self, source: &str, kind: &str) -> Result<Vec<u8>, String> {
        let source = target(source)?;
        let expected_kind = match kind {
            "text" => ArtifactKind::TextDocument,
            "image" => ArtifactKind::ImageRaster,
            _ => return Err("invalid_book_material_kind".into()),
        };
        let artifact = self
            .project
            .snapshot()
            .map_err(|e| e.to_string())?
            .artifacts
            .into_iter()
            .find(|a| a.id == source.artifact_id)
            .ok_or("missing_book_material")?;
        let content = self
            .project
            .read_revision_content(source.revision_id)
            .map_err(|e| e.to_string())?;
        if artifact.kind != expected_kind
            || content.revision.artifact_id != artifact.id
            || (kind == "text" && content.bytes.len() > 256 * 1024)
            || (kind == "image" && content.revision.content.media_type != "image/png")
        {
            return Err("invalid_book_material".into());
        }
        Ok(content.bytes)
    }
}
#[cfg(test)]
mod tests;
