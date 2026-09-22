//! Full-source script preview and project-backed authoring; no truncated UI text is parsed.
use super::{DesktopSession, audio_speech_operation_from_draft, ffi, operator_draft_wire};
use shape_domain::speech_script::{
    SpeechCueAction, SpeechScriptEventKind, SpeechScriptOptions, parse_speech_script,
};

impl DesktopSession {
    pub fn session_update_speech_script(
        &mut self,
        draft_id: &str,
        options_json: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        if options_json.len() > 64 * 1024 {
            return Err("script configuration exceeds limit".into());
        }
        let options: Option<SpeechScriptOptions> = if options_json.is_empty() {
            None
        } else {
            Some(serde_json::from_str(options_json).map_err(|e| e.to_string())?)
        };
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self
            .operator_drafts
            .update_speech_script(draft_id, options)?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }

    pub fn session_speech_script_preview(&self, draft_id: &str) -> Result<String, String> {
        let (_, draft) = self
            .operator_drafts
            .entries()
            .find(|(_, draft)| draft.id().as_str() == draft_id)
            .ok_or("speech draft missing")?;
        let operation =
            audio_speech_operation_from_draft(draft)?.ok_or("speech configuration missing")?;
        let accepted = self
            .project
            .read_revision_content(draft.input().ok_or("speech input missing")?.revision_id)
            .map_err(|e| e.to_string())?;
        let source = std::str::from_utf8(&accepted.bytes).map_err(|e| e.to_string())?;
        let plan = parse_speech_script(source);
        let options = operation.script.unwrap_or_default();
        let mut roles = std::collections::BTreeSet::new();
        let mut cues = std::collections::BTreeSet::new();

        for event in &plan.events {
            match &event.kind {
                SpeechScriptEventKind::Speech { role, .. } if !role.is_empty() => {
                    roles.insert(role.clone());
                }
                SpeechScriptEventKind::Cue { label } => {
                    cues.insert(label.clone());
                }

                _ => {}
            }
        }
        serde_json::to_string(&serde_json::json!({"ready": plan.ready(&options), "roles": roles, "cues": cues, "pause_ms": plan.pause_millis(), "shared_voices": plan.shared_voices(&options), "plan": plan})).map_err(|e| e.to_string())
    }

    pub fn session_import_speech_cue(
        &mut self,
        draft_id: &str,
        label: &str,
        bytes: &[u8],
    ) -> Result<ffi::OperatorDraftWire, String> {
        let (_, draft) = self
            .operator_drafts
            .entries()
            .find(|(_, draft)| draft.id().as_str() == draft_id)
            .ok_or("speech draft missing")?;
        let mut options = audio_speech_operation_from_draft(draft)?
            .and_then(|op| op.script)
            .ok_or("script mode required")?;
        // Check authored labels before publishing immutable material.
        let preview: serde_json::Value =
            serde_json::from_str(&self.session_speech_script_preview(draft_id)?)
                .map_err(|e| e.to_string())?;
        if !preview["cues"]
            .as_array()
            .is_some_and(|cues| cues.iter().any(|cue| cue.as_str() == Some(label)))
        {
            return Err("unknown script cue".into());
        }
        let content = self
            .project
            .import_speech_cue(bytes)
            .map_err(|e| e.to_string())?;
        options
            .cues
            .insert(label.into(), SpeechCueAction::Audio { content });
        self.session_update_speech_script(
            draft_id,
            &serde_json::to_string(&options).map_err(|e| e.to_string())?,
        )
    }
}
