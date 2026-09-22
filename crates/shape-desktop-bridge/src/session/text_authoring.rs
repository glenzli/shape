//! Durable empty-first writing, exact full-text review and speech handoff.
use super::{
    Candidate, DesktopSession, ffi, operator_draft_wire, parse_artifact_id, text_candidate_wire,
};
use crate::operator_catalog::{
    self,
    text_authoring::{self, TextAuthoring, WritingEntry},
};
use shape_domain::{
    Artifact, ArtifactContentContract, ArtifactKind, TextDocumentContract, WorkingInput,
};

impl DesktopSession {
    pub fn session_rename_artifact(&self, artifact_id: &str, name: &str) -> Result<(), String> {
        self.project
            .rename_artifact(parse_artifact_id(artifact_id)?, name)
            .map_err(|e| e.to_string())
    }
    pub fn session_text_node_input(&self, draft_id: &str) -> Result<String, String> {
        let (_, draft) = self
            .operator_drafts
            .entries()
            .find(|(_, d)| d.id().as_str() == draft_id)
            .ok_or("invalid_operator_draft")?;
        let Some(input) = draft.input() else {
            return Ok(String::new());
        };
        let accepted = self
            .project
            .read_revision_content(input.revision_id)
            .map_err(|e| e.to_string())?;
        if accepted.revision.artifact_id != input.artifact_id {
            return Err("invalid_text_input".into());
        }
        String::from_utf8(accepted.bytes).map_err(|_| "invalid_text_input".into())
    }

    pub fn session_refresh_text_input(&mut self, draft_id: &str) -> Result<(), String> {
        let (_, draft) = self
            .operator_drafts
            .entries()
            .find(|(_, d)| d.id().as_str() == draft_id)
            .ok_or("invalid_operator_draft")?;
        let speech = draft.operator_type().as_str() == operator_catalog::AUDIO_SPEECH_OPERATOR;
        let mut input = draft.input().ok_or("missing_accepted_revision")?;
        let accepted = self
            .project
            .read_accepted(input.artifact_id)
            .map_err(|e| e.to_string())?
            .ok_or("missing_accepted_revision")?;
        input.revision_id = accepted.revision.id;
        let previous = self.operator_drafts.clone();
        let artifact_id = self.operator_drafts.refresh_text_input(draft_id, input)?;
        if speech {
            let script = matches!(
                accepted.revision.content_contract,
                Some(ArtifactContentContract::TextDocument(
                    TextDocumentContract::SpeechScript { .. }
                ))
            );
            let current = self.operator_drafts.draft(artifact_id, draft_id)?;
            let options = operator_catalog::audio_speech_operation_from_draft(current)?
                .and_then(|o| o.script);
            self.operator_drafts.update_speech_script(
                draft_id,
                if script {
                    Some(options.unwrap_or_default())
                } else {
                    None
                },
            )?;
        }
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        self.candidates.discard_artifact(artifact_id);
        Ok(())
    }

    pub fn session_create_text_authoring(
        &mut self,
        name: &str,
        profile: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let state = TextAuthoring::new(profile)?;
        let artifact =
            Artifact::new(name, ArtifactKind::TextDocument).map_err(|e| e.to_string())?;
        let previous = self.operator_drafts.clone();
        let result = (|| {
            let draft = self
                .operator_drafts
                .begin_text_node(&artifact, None, &state)?;
            self.operator_drafts
                .configure_authoring(draft.id().as_str(), &state)?;
            let graph = self
                .operator_drafts
                .graph(artifact.id)
                .ok_or("invalid_operator_draft")?;
            self.project
                .create_source_artifact_draft(&artifact, graph)
                .map_err(|e| e.to_string())?;
            self.session_snapshot()
        })();
        if result.is_err() {
            self.operator_drafts = previous;
        }
        result
    }

    pub fn session_begin_text_authoring(
        &mut self,
        artifact_id: &str,
        profile: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let artifact = parse_artifact_id(artifact_id)?;
        if let Some((_, draft)) = self.operator_drafts.entries().find(|(id, draft)| {
            *id == artifact && text_authoring::from_draft(draft).ok().flatten().is_some()
        }) {
            return Ok(operator_draft_wire(artifact, draft));
        }
        self.create_text_derivation(artifact_id, profile)
    }

    pub(super) fn create_text_derivation(
        &mut self,
        source_id: &str,
        profile: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        self.create_text_derivation_with_state(source_id, TextAuthoring::new(profile)?)
    }

    pub(super) fn create_text_derivation_with_state(
        &mut self,
        source_id: &str,
        mut state: TextAuthoring,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let source_id = parse_artifact_id(source_id)?;
        let source = self
            .project
            .snapshot()
            .map_err(|e| e.to_string())?
            .artifacts
            .into_iter()
            .find(|a| a.id == source_id && a.kind == ArtifactKind::TextDocument)
            .ok_or("invalid_text_source")?;
        let revision_id = source
            .accepted_revision
            .ok_or("missing_accepted_revision")?;
        state.entry = WritingEntry::Adapt;
        let artifact = Artifact::new(
            format!(
                "{} · {}",
                source.name.chars().take(24).collect::<String>(),
                "Edit"
            ),
            ArtifactKind::TextDocument,
        )
        .map_err(|e| e.to_string())?;
        let previous = self.operator_drafts.clone();
        let result = (|| {
            let draft = self.operator_drafts.begin_text_node(
                &artifact,
                Some(WorkingInput {
                    artifact_id: source_id,
                    revision_id,
                }),
                &state,
            )?;
            self.project
                .create_source_artifact_draft(
                    &artifact,
                    self.operator_drafts
                        .graph(artifact.id)
                        .ok_or("invalid_operator_draft")?,
                )
                .map_err(|e| e.to_string())?;
            Ok(operator_draft_wire(artifact.id, &draft))
        })();
        if result.is_err() {
            self.operator_drafts = previous;
        }
        result
    }

    pub fn session_update_text_authoring(
        &mut self,
        draft_id: &str,
        json: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let state = TextAuthoring::from_json(json)?;
        let previous = self.operator_drafts.clone();
        let (artifact, draft) = self.operator_drafts.configure_authoring(draft_id, &state)?;
        if let Err(error) = self.persist_operator_drafts(artifact) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact, &draft))
    }

    pub fn session_text_authoring_content(
        &self,
        artifact_id: &str,
        candidate_id: &str,
    ) -> Result<String, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        if !candidate_id.is_empty() {
            let candidate = self.candidates.clone_candidate(candidate_id)?;
            if let Candidate::Text(candidate) = candidate
                && candidate.artifact_id() == artifact_id
            {
                return Ok(candidate.text().to_owned());
            }
            return Err("invalid_candidate".into());
        }
        let accepted = self
            .project
            .read_accepted(artifact_id)
            .map_err(|e| e.to_string())?;
        accepted.map_or(Ok(String::new()), |a| {
            String::from_utf8(a.bytes).map_err(|_| "invalid_text_output".into())
        })
    }

    pub fn session_propose_authored_text(
        &mut self,
        draft_id: &str,
    ) -> Result<ffi::CandidateWire, String> {
        let (artifact_id, draft) = self
            .operator_drafts
            .entries()
            .find(|(_, d)| d.id().as_str() == draft_id)
            .ok_or("invalid_operator_draft")?;
        let state = text_authoring::from_draft(draft)?.ok_or("invalid_writing_draft")?;
        if state.entry != WritingEntry::Manual {
            return Err("invalid_writing_draft".into());
        }
        text_authoring::validate_output(&state, &state.text)?;
        let expected = self
            .operator_drafts
            .graph(artifact_id)
            .ok_or("invalid_operator_draft")?
            .expected_revision_id();
        if let Some(existing) = self.candidates.newest_first().find(|c| matches!(c, Candidate::Text(t) if t.artifact_id() == artifact_id && t.expected_head() == expected && t.text() == state.text && t.request_node() == Some(draft) && t.content_contract() == &state.content_contract())) {
            return Ok(super::candidate_wire(existing));
        }
        let candidate = self
            .project
            .propose_text_node_literal(artifact_id, draft, &state.text, state.content_contract())
            .map_err(|e| e.to_string())?;
        let wire = text_candidate_wire(&candidate);
        self.candidates.push(Candidate::Text(candidate));
        Ok(wire)
    }

    pub fn session_begin_authoring_speech(
        &mut self,
        artifact_id: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let artifact = parse_artifact_id(artifact_id)?;
        let accepted = self
            .project
            .read_accepted(artifact)
            .map_err(|e| e.to_string())?
            .ok_or("missing_accepted_revision")?;
        let script = matches!(
            accepted.revision.content_contract,
            Some(ArtifactContentContract::TextDocument(
                TextDocumentContract::SpeechScript { .. }
            ))
        );
        if let Some((target, draft)) = self.operator_drafts.entries().find(|(_, d)| {
            d.operator_type().as_str() == operator_catalog::AUDIO_SPEECH_OPERATOR
                && d.input().is_some_and(|i| i.artifact_id == artifact)
        }) {
            return Ok(operator_draft_wire(target, draft));
        }
        self.create_speech_derivation(artifact_id, script)
    }

    pub(super) fn create_speech_derivation(
        &mut self,
        source_id: &str,
        script: bool,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let source_id = parse_artifact_id(source_id)?;
        let source = self
            .project
            .snapshot()
            .map_err(|e| e.to_string())?
            .artifacts
            .into_iter()
            .find(|a| a.id == source_id && a.kind == ArtifactKind::TextDocument)
            .ok_or("invalid_speech_source")?;
        let revision_id = source
            .accepted_revision
            .ok_or("missing_accepted_revision")?;
        let target = Artifact::new(
            format!(
                "{} · Audio",
                source.name.chars().take(24).collect::<String>()
            ),
            ArtifactKind::AudioClip,
        )
        .map_err(|e| e.to_string())?;
        let previous = self.operator_drafts.clone();
        let result = (|| {
            let draft = self.operator_drafts.begin_speech_node(
                &target,
                WorkingInput {
                    artifact_id: source_id,
                    revision_id,
                },
                script,
            )?;
            self.project
                .create_source_artifact_draft(
                    &target,
                    self.operator_drafts
                        .graph(target.id)
                        .ok_or("invalid_operator_draft")?,
                )
                .map_err(|e| e.to_string())?;
            Ok(operator_draft_wire(target.id, &draft))
        })();
        if result.is_err() {
            self.operator_drafts = previous;
        }
        result
    }
}

#[cfg(test)]
mod tests;
