//! Desktop sound-source authoring and exact-result adoption.
use super::{
    Artifact, ArtifactKind, Candidate, DesktopSession, audio_origin_key, ffi, operator_draft_wire,
    parse_artifact_id,
};
use crate::infer_sound::InferSoundCandidate;
use crate::operator_catalog::sound_generation::parameters;
impl DesktopSession {
    pub fn session_sound_details(
        &self,
        artifact_id: &str,
        candidate_id: &str,
    ) -> Result<String, String> {
        let id = parse_artifact_id(artifact_id)?;
        let (operation, receipt) = if candidate_id.is_empty() {
            let revision = self
                .project
                .snapshot()
                .map_err(|_| "project_unavailable")?
                .artifacts
                .into_iter()
                .find(|a| a.id == id)
                .and_then(|a| a.accepted_revision)
                .ok_or("no_accepted_audio")?;
            let revision = self
                .project
                .revision(revision)
                .map_err(|_| "invalid_sound_history")?;
            let transform = self
                .project
                .transformation(revision.transformation_id)
                .map_err(|_| "invalid_sound_history")?;
            let Some(shape_domain::TransformationOperation::AudioGenerate(op)) =
                transform.operation
            else {
                return Err("not_generated_sound".into());
            };
            let receipt = self
                .project
                .transformation_receipt(revision.transformation_id)
                .map_err(|_| "invalid_sound_history")?
                .ok_or("invalid_sound_history")?;
            (op, receipt)
        } else {
            let Candidate::Sound(candidate) = self.candidates.candidate(candidate_id)? else {
                return Err("not_generated_sound".into());
            };
            if candidate.artifact_id() != id {
                return Err("stale_candidate".into());
            }
            (candidate.parameters().clone(), candidate.receipt().clone())
        };
        serde_json::to_string(&serde_json::json!({"operation": operation, "job_id": receipt.executor_job_id, "provenance": receipt.external_provenance})).map_err(|_| "invalid_sound_history".into())
    }

    pub fn session_create_sound_draft(
        &mut self,
        name: &str,
        prompt: &str,
        kind: &str,
        seconds: u8,
        seed: u32,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let artifact = Artifact::new(name, ArtifactKind::AudioClip).map_err(|e| e.to_string())?;
        let previous = self.operator_drafts.clone();
        if let Err(error) = self
            .operator_drafts
            .begin_sound_source(&artifact, prompt, kind, seconds, seed)
        {
            self.operator_drafts = previous;
            return Err(error);
        }
        let graph = self
            .operator_drafts
            .graph(artifact.id)
            .cloned()
            .ok_or("invalid_sound_draft")?;
        if let Err(error) = self.project.create_source_artifact_draft(&artifact, &graph) {
            self.operator_drafts = previous;
            return Err(error.to_string());
        }
        self.session_snapshot()
    }
    pub fn session_update_sound_draft(
        &mut self,
        draft_id: &str,
        prompt: &str,
        kind: &str,
        seconds: u8,
        seed: u32,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self
            .operator_drafts
            .update_sound_configuration(draft_id, prompt, kind, seconds, seed)?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }
    #[allow(clippy::boxed_local)]
    pub fn session_adopt_infer_sound(
        &mut self,
        result: Box<InferSoundCandidate>,
    ) -> Result<ffi::CandidateWire, String> {
        let InferSoundCandidate {
            candidate,
            draft_id,
        } = *result;
        let id = candidate.artifact_id();
        if !self
            .project
            .snapshot()
            .map_err(|_| "project_unavailable")?
            .artifacts
            .iter()
            .any(|a| {
                a.id == id && a.kind == ArtifactKind::AudioClip && a.accepted_revision.is_none()
            })
        {
            return Err("stale_candidate".into());
        }
        let draft = self.operator_drafts.draft(id, &draft_id)?;
        if &parameters(draft)? != candidate.parameters() {
            return Err("stale_candidate".into());
        }
        let wire = sound_candidate_wire(&candidate);
        self.candidates.push(Candidate::Sound(candidate));
        Ok(wire)
    }
}
pub(super) fn sound_candidate_wire(candidate: &shape_core::SoundCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: candidate.artifact_name().into(),
        kind_key: "audio_clip".into(),
        has_expected_head: false,
        expected_head: String::new(),
        can_branch: false,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
        has_image_preview: false,
        image_width: 0,
        image_height: 0,
        has_audio_preview: true,
        audio_duration_millis: candidate.contract().duration_millis(),
        audio_sample_rate_hz: candidate.contract().sample_rate_hz,
        audio_channels: candidate.contract().channels,
        audio_origin_key: audio_origin_key(candidate.contract().origin).into(),
    }
}
