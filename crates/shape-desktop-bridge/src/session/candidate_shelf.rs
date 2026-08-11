//! Transient cross-media candidate collection with exact identity mutation.

use shape_core::{
    AiImageCandidate, AudioCandidate, ImageCandidate, ImageResizeCandidate, TextCandidate,
};
use shape_domain::ArtifactId;

/// One executed payload awaiting explicit user acceptance.
#[derive(Debug, Clone)]
pub(super) enum Candidate {
    Text(TextCandidate),
    Image(ImageCandidate),
    ImageResize(ImageResizeCandidate),
    AiImage(AiImageCandidate),
    Audio(AudioCandidate),
}

impl Candidate {
    pub(super) fn id(&self) -> String {
        match self {
            Self::Text(candidate) => candidate.receipt().attempt_id.to_string(),
            Self::Image(candidate) => candidate.receipt().attempt_id.to_string(),
            Self::ImageResize(candidate) => candidate.receipt().attempt_id.to_string(),
            Self::AiImage(candidate) => candidate.receipt().attempt_id.to_string(),
            Self::Audio(candidate) => candidate.receipt().attempt_id.to_string(),
        }
    }

    pub(super) const fn artifact_id(&self) -> ArtifactId {
        match self {
            Self::Text(candidate) => candidate.artifact_id(),
            Self::Image(candidate) => candidate.artifact_id(),
            Self::ImageResize(candidate) => candidate.artifact_id(),
            Self::AiImage(candidate) => candidate.artifact_id(),
            Self::Audio(candidate) => candidate.artifact_id(),
        }
    }
}

/// One in-memory shelf, projected newest first across media families.
#[derive(Debug, Default)]
pub(super) struct CandidateShelf {
    candidates: Vec<Candidate>,
}

impl CandidateShelf {
    pub(super) fn push(&mut self, candidate: Candidate) {
        debug_assert!(
            self.candidates
                .iter()
                .all(|existing| existing.id() != candidate.id())
        );
        self.candidates.push(candidate);
    }

    pub(super) fn newest_first(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates.iter().rev()
    }

    pub(super) fn contains_text(&self, artifact_id: ArtifactId, text: &str) -> bool {
        self.candidates.iter().any(|candidate| {
            matches!(
                candidate,
                Candidate::Text(candidate)
                    if candidate.artifact_id() == artifact_id && candidate.text() == text
            )
        })
    }

    pub(super) fn contains_image_crop(
        &self,
        artifact_id: ArtifactId,
        crop: shape_domain::RasterCrop,
    ) -> bool {
        self.candidates.iter().any(|candidate| {
            let Candidate::Image(candidate) = candidate else {
                return false;
            };
            candidate.artifact_id() == artifact_id
                && matches!(
                    candidate.operation(),
                    Some(shape_domain::TransformationOperation::RasterCrop(existing))
                        if *existing == crop
                )
        })
    }

    pub(super) fn contains_image_resize(
        &self,
        artifact_id: ArtifactId,
        resize: shape_domain::RasterResize,
    ) -> bool {
        self.candidates.iter().any(|candidate| {
            let Candidate::ImageResize(candidate) = candidate else {
                return false;
            };
            candidate.artifact_id() == artifact_id
                && matches!(
                    candidate.operation(),
                    Some(shape_domain::TransformationOperation::RasterResize(existing))
                        if *existing == resize
                )
        })
    }

    pub(super) fn candidate(&self, candidate_id: &str) -> Result<&Candidate, String> {
        self.candidates
            .iter()
            .find(|candidate| candidate.id() == candidate_id)
            .ok_or_else(|| "candidate does not exist in this session".to_owned())
    }

    pub(super) fn clone_candidate(&self, candidate_id: &str) -> Result<Candidate, String> {
        self.candidate(candidate_id).cloned()
    }

    pub(super) fn discard(&mut self, candidate_id: &str) -> Result<(), String> {
        let index = self
            .candidates
            .iter()
            .position(|candidate| candidate.id() == candidate_id)
            .ok_or_else(|| "candidate does not exist in this session".to_owned())?;
        self.candidates.remove(index);
        Ok(())
    }

    pub(super) fn discard_artifact(&mut self, artifact_id: ArtifactId) {
        self.candidates
            .retain(|candidate| candidate.artifact_id() != artifact_id);
    }
}

#[cfg(test)]
mod tests;
