//! Session-local Operator drafts before a real Candidate exists.
//!
//! A draft makes the graph authoring entry visible without pretending that an
//! unexecuted Operator is already an accepted `SceneRevision`. The first real
//! Candidate replaces the matching draft; acceptance continues through the
//! existing immutable Artifact history boundary.

use shape_domain::{Artifact, ArtifactId, ArtifactKind};

pub(crate) const AUDIO_SPEECH_OPERATOR: &str = "audio.speech_synthesize";
pub(crate) const IMAGE_CROP_OPERATOR: &str = "image.crop";
pub(crate) const TEXT_EDIT_OPERATOR: &str = "text.edit";
pub(crate) const TEXT_TRANSFORM_OPERATOR: &str = "text.transform";

const AUDIO_CLIP_DATA: &str = "audio.clip";
const IMAGE_RASTER_DATA: &str = "image.raster";
const TEXT_DOCUMENT_DATA: &str = "text.document";

/// One validated, session-local creative Operator entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperatorDraft {
    pub(crate) id: String,
    pub(crate) context_artifact_id: ArtifactId,
    pub(crate) operator_type: &'static str,
    pub(crate) input_data_type: &'static str,
    pub(crate) output_data_type: &'static str,
}

/// Identity-addressed drafts owned by one open desktop session.
#[derive(Debug, Default)]
pub(crate) struct OperatorDrafts {
    next_identity: u64,
    entries: Vec<OperatorDraft>,
}

impl OperatorDrafts {
    /// Begins or reuses one compatible Operator draft.
    pub(crate) fn begin(
        &mut self,
        artifact: &Artifact,
        operator_type: &str,
    ) -> Result<OperatorDraft, String> {
        if artifact.accepted_revision.is_none() {
            return Err("an Operator draft requires an accepted source".to_owned());
        }
        let (input_data_type, output_data_type) =
            compatible_contract(artifact.kind, operator_type)?;
        if let Some(existing) = self.entries.iter().find(|draft| {
            draft.context_artifact_id == artifact.id && draft.operator_type == operator_type
        }) {
            return Ok(existing.clone());
        }
        self.next_identity = self
            .next_identity
            .checked_add(1)
            .ok_or_else(|| "Operator draft identity space is exhausted".to_owned())?;
        let draft = OperatorDraft {
            id: format!("draft.{}", self.next_identity),
            context_artifact_id: artifact.id,
            operator_type: canonical_operator(operator_type)
                .expect("compatible contracts return canonical Operators"),
            input_data_type,
            output_data_type,
        };
        self.entries.push(draft.clone());
        Ok(draft)
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &OperatorDraft> {
        self.entries.iter()
    }

    pub(crate) fn discard(&mut self, draft_id: &str) -> Result<(), String> {
        let Some(index) = self.entries.iter().position(|draft| draft.id == draft_id) else {
            return Err("Operator draft does not exist".to_owned());
        };
        self.entries.remove(index);
        Ok(())
    }

    pub(crate) fn finish(&mut self, artifact_id: ArtifactId, operator_type: &str) {
        self.entries.retain(|draft| {
            draft.context_artifact_id != artifact_id || draft.operator_type != operator_type
        });
    }
}

fn compatible_contract(
    artifact_kind: ArtifactKind,
    operator_type: &str,
) -> Result<(&'static str, &'static str), String> {
    match (artifact_kind, operator_type) {
        (ArtifactKind::TextDocument, TEXT_EDIT_OPERATOR | TEXT_TRANSFORM_OPERATOR) => {
            Ok((TEXT_DOCUMENT_DATA, TEXT_DOCUMENT_DATA))
        }
        (ArtifactKind::TextDocument, AUDIO_SPEECH_OPERATOR) => {
            Ok((TEXT_DOCUMENT_DATA, AUDIO_CLIP_DATA))
        }
        (ArtifactKind::ImageRaster, IMAGE_CROP_OPERATOR) => {
            Ok((IMAGE_RASTER_DATA, IMAGE_RASTER_DATA))
        }
        _ => Err("this Operator is incompatible with the selected Scene source".to_owned()),
    }
}

fn canonical_operator(value: &str) -> Option<&'static str> {
    match value {
        TEXT_EDIT_OPERATOR => Some(TEXT_EDIT_OPERATOR),
        TEXT_TRANSFORM_OPERATOR => Some(TEXT_TRANSFORM_OPERATOR),
        IMAGE_CROP_OPERATOR => Some(IMAGE_CROP_OPERATOR),
        AUDIO_SPEECH_OPERATOR => Some(AUDIO_SPEECH_OPERATOR),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
