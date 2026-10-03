//! Material stage admission, exact-session adoption and bounded candidate replacement.
use super::{Candidate, DesktopSession, ffi, operator_draft_wire, parse_artifact_id};
use crate::material_edit::{PreparedMaterialReturn, material_edit_error};
use shape_core::{MaterialEditCandidate, MaterialEditTicket};
use shape_domain::{ArtifactContentContract, MaterialEditor, WorkingInput};

impl DesktopSession {
    pub(super) fn begin_material_edit(
        &mut self,
        source_id: &str,
        editor: MaterialEditor,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let source_id = parse_artifact_id(source_id)?;
        let source = self
            .project
            .snapshot()
            .map_err(material_edit_error)?
            .artifacts
            .into_iter()
            .find(|a| a.id == source_id)
            .ok_or("invalid_material_source")?;
        let source_head = source
            .accepted_revision
            .ok_or("missing_accepted_revision")?;
        let name = format!(
            "{} · {}",
            source.name.chars().take(24).collect::<String>(),
            editor.name()
        );
        let ticket = self
            .project
            .create_material_edit(
                WorkingInput {
                    artifact_id: source_id,
                    revision_id: source_head,
                },
                &name,
                editor,
            )
            .map_err(material_edit_error)?;
        self.reload_material_graph(ticket.output_artifact_id)?;
        Ok(operator_draft_wire(
            ticket.output_artifact_id,
            &ticket.draft,
        ))
    }

    pub fn session_material_edit_ticket(&self, output_id: &str) -> Result<String, String> {
        let output = parse_artifact_id(output_id)?;
        let ticket = self
            .project
            .material_edit_ticket(output)
            .map_err(material_edit_error)?;
        let observed = self
            .operator_drafts
            .graph(output)
            .ok_or("material_session_changed")?;
        if observed.expected_revision_id() != ticket.expected_output_head
            || observed.operators() != std::slice::from_ref(&ticket.draft)
        {
            return Err("material_session_changed".into());
        }
        serde_json::to_string(&ticket).map_err(|_| "material_return_invalid".into())
    }

    /// Shows the pinned original even when a downstream input has become stale.
    pub fn session_material_source_image(
        &self,
        output_id: &str,
    ) -> Result<ffi::ImagePreviewWire, String> {
        let ticket: MaterialEditTicket =
            serde_json::from_str(&self.session_material_edit_ticket(output_id)?)
                .map_err(|_| "material_return_invalid")?;
        let source = ticket.draft.input().ok_or("material_return_invalid")?;
        let content = self
            .project
            .read_revision_content(source.revision_id)
            .map_err(material_edit_error)?;
        let Some(ArtifactContentContract::ImageRaster(contract)) =
            content.revision.content_contract
        else {
            return Err("material_return_invalid".into());
        };
        Ok(ffi::ImagePreviewWire {
            identity: source.revision_id.to_string(),
            width: contract.width,
            height: contract.height,
            png_bytes: content.bytes,
        })
    }

    pub fn session_restart_material_edit(
        &mut self,
        ticket_json: &str,
        refresh: bool,
    ) -> Result<String, String> {
        let ticket: MaterialEditTicket =
            serde_json::from_str(ticket_json).map_err(|_| "material_return_invalid")?;
        let current = self
            .project
            .restart_material_edit(&ticket, refresh)
            .map_err(material_edit_error)?;
        self.reload_material_graph(ticket.output_artifact_id)?;
        self.candidates.discard_artifact(ticket.output_artifact_id);
        serde_json::to_string(&current).map_err(|_| "material_return_invalid".into())
    }

    pub fn session_adopt_material_return(
        &mut self,
        prepared: Box<PreparedMaterialReturn>,
    ) -> Result<ffi::CandidateWire, String> {
        let candidate = prepared.candidate;
        self.project
            .material_edit_source(candidate.ticket())
            .map_err(material_edit_error)?;
        let observed: MaterialEditTicket = serde_json::from_str(
            &self.session_material_edit_ticket(&candidate.artifact_id().to_string())?,
        )
        .map_err(|_| "material_return_invalid")?;
        if &observed != candidate.ticket() {
            return Err("material_session_changed".into());
        }
        for existing in self.candidates.newest_first() {
            if let Candidate::Material(existing) = existing
                && existing.ticket() == candidate.ticket()
                && existing.returned_digest() == candidate.returned_digest()
            {
                return Ok(material_candidate_wire(existing));
            }
        }
        let wire = material_candidate_wire(&candidate);
        // A valid replacement supersedes one return; failures above retain the reviewed one.
        self.candidates.discard_artifact(candidate.artifact_id());
        self.candidates.push(Candidate::Material(candidate));
        Ok(wire)
    }

    fn reload_material_graph(&mut self, output: shape_domain::ArtifactId) -> Result<(), String> {
        let graph = self
            .project
            .artifact_working_graphs()
            .map_err(material_edit_error)?
            .into_iter()
            .find(|g| g.context_artifact_id() == output)
            .ok_or("material_session_changed")?;
        self.operator_drafts.adopt_material_graph(graph)
    }
}

pub(super) fn material_candidate_wire(candidate: &MaterialEditCandidate) -> ffi::CandidateWire {
    let image = match candidate.contract() {
        ArtifactContentContract::ImageRaster(c) => Some(c),
        _ => None,
    };
    let audio = match candidate.contract() {
        ArtifactContentContract::AudioClip(c) => Some(c),
        _ => None,
    };
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: candidate.artifact_name().into(),
        kind_key: if image.is_some() {
            "image_raster"
        } else {
            "audio_clip"
        }
        .into(),
        has_expected_head: candidate.ticket().expected_output_head.is_some(),
        expected_head: candidate
            .ticket()
            .expected_output_head
            .map_or_else(String::new, |r| r.to_string()),
        can_branch: false,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
        has_image_preview: image.is_some(),
        image_width: image.map_or(0, |c| c.width),
        image_height: image.map_or(0, |c| c.height),
        has_audio_preview: audio.is_some(),
        audio_duration_millis: audio.map_or(0, shape_domain::AudioValueContract::duration_millis),
        audio_sample_rate_hz: audio.map_or(0, |c| c.sample_rate_hz),
        audio_channels: audio.map_or(0, |c| c.channels),
        audio_origin_key: audio
            .map_or_else(String::new, |c| crate::audio_origin_key(c.origin).into()),
    }
}

#[cfg(test)]
mod tests;
