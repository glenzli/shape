//! Bounded book authoring adapter over the shared candidate shelf and immutable materials.
use super::{Candidate, DesktopSession, ffi};
use shape_core::{PictureBookCandidate, PictureBookRequest};
use shape_domain::{ArtifactKind, WorkingInput};

fn request(value: &str) -> Result<PictureBookRequest, String> {
    if value.len() > 256 * 1024 {
        return Err("book_request_too_large".into());
    }
    serde_json::from_str(value).map_err(|_| "invalid_book_request".into())
}
fn target(value: &str) -> Result<WorkingInput, String> {
    serde_json::from_str(value).map_err(|_| "invalid_book_identity".into())
}
fn json(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

impl DesktopSession {
    pub fn session_book_preview(&self, composition: &str) -> Result<String, String> {
        json(
            &self
                .project
                .preview_picture_book(&request(composition)?)
                .map_err(|e| e.to_string())?,
        )
    }
    pub fn session_book_propose(
        &mut self,
        source: &str,
        composition: &str,
    ) -> Result<String, String> {
        let source = if source.is_empty() {
            None
        } else {
            Some(target(source)?)
        };
        let candidate = self
            .project
            .propose_picture_book(source, request(composition)?)
            .map_err(|e| e.to_string())?;
        let id = candidate.receipt().attempt_id.to_string();
        self.candidates.push(Candidate::Book(candidate));
        Ok(id)
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
    pub fn session_book_script(
        &mut self,
        name: &str,
        source: &str,
        text: &str,
    ) -> Result<String, String> {
        if text.len() > 256 * 1024 {
            return Err("book_script_too_large".into());
        }
        let source = if source.is_empty() {
            None
        } else {
            Some(target(source)?)
        };
        let revision = self
            .project
            .adopt_picture_book_script(name, source, text)
            .map_err(|e| e.to_string())?;
        json(&WorkingInput {
            artifact_id: revision.artifact_id,
            revision_id: revision.id,
        })
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
pub(super) fn candidate_wire(candidate: &PictureBookCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: candidate.name().into(),
        kind_key: "picture_book".into(),
        has_expected_head: candidate.expected_head().is_some(),
        expected_head: candidate
            .expected_head()
            .map_or_else(String::new, |h| h.to_string()),
        can_branch: false,
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
    }
}

#[cfg(test)]
mod tests;
