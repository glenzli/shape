//! Mutable desktop lifecycle over one project and its transient Candidate Shelf.

mod candidate_shelf;

use shape_core::{ImageCandidate, ShapeProject, TextCandidate};
use shape_domain::{ArtifactContentContract, ArtifactId, ArtifactKind, IntentSpec, RasterCrop};

use crate::{bounded_text_preview, ffi, project_snapshot};
use candidate_shelf::{Candidate, CandidateShelf};

use crate::infer_text::InferTextCandidate;

const USER_AUTHORED_TEXT_INTENT: &str = "Replace text with a user-authored draft";

/// One open desktop project and its transient cross-media candidates.
///
/// The session never persists preview state. Proposals accumulate only after
/// successful execution. Consequential operations address one exact candidate;
/// a failed durable commit leaves the shelf available for retry.
#[derive(Debug)]
pub struct DesktopSession {
    project: ShapeProject,
    bundle_path: String,
    candidates: CandidateShelf,
}

/// Opens a validated project for mutable desktop use.
///
/// # Errors
///
/// Returns a user-safe message when the project cannot be opened.
pub fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>, String> {
    let project = ShapeProject::open(path).map_err(|error| error.to_string())?;
    Ok(Box::new(DesktopSession {
        project,
        bundle_path: super::project_path(path),
        candidates: CandidateShelf::default(),
    }))
}

impl DesktopSession {
    /// Returns durable accepted state; pending previews are excluded.
    pub fn session_snapshot(&self) -> Result<ffi::ProjectSnapshotWire, String> {
        project_snapshot(&self.project, &self.bundle_path)
    }

    /// Imports one user-selected PNG/JPEG as an atomic accepted raster origin.
    pub fn session_import_raster(
        &mut self,
        source_path: &str,
        artifact_name: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        self.project
            .import_raster(source_path, artifact_name)
            .map_err(|error| error.to_string())?;
        self.session_snapshot()
    }

    /// Executes a literal user-authored text draft as a transient candidate.
    pub fn session_propose_text(
        &mut self,
        artifact_id: &str,
        replacement_text: &str,
    ) -> Result<ffi::CandidateWire, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        let snapshot = self.project.snapshot().map_err(|error| error.to_string())?;
        let artifact = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or_else(|| "artifact does not exist in this project".to_owned())?;
        if artifact.kind != ArtifactKind::TextDocument {
            return Err("only text documents support text candidates".to_owned());
        }
        if self
            .project
            .read_accepted(artifact_id)
            .map_err(|error| error.to_string())?
            .is_some_and(|accepted| accepted.bytes == replacement_text.as_bytes())
        {
            return Err("candidate text matches the current accepted text".to_owned());
        }
        if self.candidates.contains_text(artifact_id, replacement_text) {
            return Err("candidate text already exists on the shelf".to_owned());
        }

        let candidate = self
            .project
            .propose_text(
                artifact_id,
                artifact.accepted_revision,
                replacement_text,
                IntentSpec::new(USER_AUTHORED_TEXT_INTENT).map_err(|error| error.to_string())?,
                Vec::new(),
            )
            .map_err(|error| error.to_string())?;
        let wire = text_candidate_wire(&candidate);
        self.candidates.push(Candidate::Text(candidate));
        Ok(wire)
    }

    /// Executes a bounded pixel crop as a transient image candidate.
    pub fn session_propose_raster_crop(
        &mut self,
        artifact_id: &str,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<ffi::CandidateWire, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        let artifact = self
            .project
            .snapshot()
            .map_err(|error| error.to_string())?
            .artifacts
            .into_iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or_else(|| "artifact does not exist in this project".to_owned())?;
        if artifact.kind != ArtifactKind::ImageRaster {
            return Err("only raster images support crop candidates".to_owned());
        }
        let expected_head = artifact
            .accepted_revision
            .ok_or_else(|| "raster artifact has no accepted revision".to_owned())?;
        let revision = self
            .project
            .revision(expected_head)
            .map_err(|error| error.to_string())?;
        let Some(ArtifactContentContract::ImageRaster(contract)) = revision.content_contract else {
            return Err("raster artifact content contract is missing".to_owned());
        };
        let crop = RasterCrop::new(x, y, width, height, contract.width, contract.height)
            .map_err(|error| error.to_string())?;
        if self.candidates.contains_image_crop(artifact_id, crop) {
            return Err("crop candidate already exists on the shelf".to_owned());
        }
        let candidate = self
            .project
            .propose_raster_crop(artifact_id, expected_head, crop)
            .map_err(|error| error.to_string())?;
        let wire = image_candidate_wire(&candidate);
        self.candidates.push(Candidate::Image(candidate));
        Ok(wire)
    }

    /// Returns all transient candidates in newest-first presentation order.
    pub fn session_candidates(&self) -> Vec<ffi::CandidateWire> {
        self.candidates.newest_first().map(candidate_wire).collect()
    }

    /// Explicitly accepts one text or image candidate.
    pub fn session_accept_candidate(
        &mut self,
        candidate_id: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let candidate = self.candidates.clone_candidate(candidate_id)?;
        let artifact_id = candidate.artifact_id();
        match candidate {
            Candidate::Text(candidate) => self
                .project
                .accept_text(candidate)
                .map_err(|error| error.to_string())?,
            Candidate::Image(candidate) => self
                .project
                .accept_raster_crop(candidate)
                .map_err(|error| error.to_string())?,
        };
        self.candidates.discard_artifact(artifact_id);
        self.session_snapshot()
    }

    /// Branches a text candidate. Image branching remains deferred until the
    /// first image-derived-artifact product path exists.
    pub fn session_branch_candidate(
        &mut self,
        candidate_id: &str,
        artifact_name: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let Candidate::Text(candidate) = self.candidates.clone_candidate(candidate_id)? else {
            return Err("this candidate type cannot branch yet".to_owned());
        };
        self.project
            .branch_text_candidate(candidate, artifact_name)
            .map_err(|error| error.to_string())?;
        self.candidates.discard(candidate_id)?;
        self.session_snapshot()
    }

    /// Discards one transient preview without touching durable history.
    pub fn session_discard_candidate(&mut self, candidate_id: &str) -> Result<(), String> {
        self.candidates.discard(candidate_id)
    }

    /// Returns canonical PNG bytes only for the requested accepted head or
    /// exact transient image candidate.
    pub fn session_image_preview(
        &self,
        artifact_id: &str,
        candidate_id: &str,
    ) -> Result<ffi::ImagePreviewWire, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        if !candidate_id.is_empty() {
            let Candidate::Image(candidate) = self.candidates.candidate(candidate_id)? else {
                return Err("candidate is not an image preview".to_owned());
            };
            if candidate.artifact_id() != artifact_id {
                return Err("candidate does not belong to the selected artifact".to_owned());
            }
            return Ok(ffi::ImagePreviewWire {
                identity: candidate_id.to_owned(),
                width: candidate.contract().width,
                height: candidate.contract().height,
                png_bytes: candidate.png_bytes().to_vec(),
            });
        }
        let accepted = self
            .project
            .read_accepted(artifact_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "raster artifact has no accepted revision".to_owned())?;
        let Some(ArtifactContentContract::ImageRaster(contract)) =
            accepted.revision.content_contract
        else {
            return Err("accepted content is not an image raster".to_owned());
        };
        Ok(ffi::ImagePreviewWire {
            identity: accepted.revision.id.to_string(),
            width: contract.width,
            height: contract.height,
            png_bytes: accepted.bytes,
        })
    }

    /// Adopts a completed Infer text candidate after rechecking the live head.
    pub fn session_adopt_infer_text(
        &mut self,
        candidate: Box<InferTextCandidate>,
    ) -> Result<ffi::CandidateWire, String> {
        let candidate = (*candidate).into_candidate();
        let artifact_id = candidate.artifact_id();
        let snapshot = self.project.snapshot().map_err(|_| "project_unavailable")?;
        let artifact = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or("invalid_artifact")?;
        if artifact.accepted_revision != candidate.expected_head() {
            return Err("stale_candidate".to_owned());
        }
        if self.candidates.contains_text(artifact_id, candidate.text()) {
            return Err("duplicate_candidate".to_owned());
        }
        let wire = text_candidate_wire(&candidate);
        self.candidates.push(Candidate::Text(candidate));
        Ok(wire)
    }
}

fn parse_artifact_id(value: &str) -> Result<ArtifactId, String> {
    value
        .parse::<ArtifactId>()
        .map_err(|_| "artifact identity is invalid".to_owned())
}

fn candidate_wire(candidate: &Candidate) -> ffi::CandidateWire {
    match candidate {
        Candidate::Text(candidate) => text_candidate_wire(candidate),
        Candidate::Image(candidate) => image_candidate_wire(candidate),
    }
}

fn text_candidate_wire(candidate: &TextCandidate) -> ffi::CandidateWire {
    let (text_preview, text_preview_truncated) =
        bounded_text_preview(candidate.text().as_bytes()).expect("text candidates are valid UTF-8");
    let expected_head = candidate.expected_head();
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        kind_key: "text_document".to_owned(),
        has_expected_head: expected_head.is_some(),
        expected_head: expected_head.map_or_else(String::new, |head| head.to_string()),
        can_branch: expected_head.is_some(),
        has_text_preview: true,
        text_preview_truncated,
        text_preview,
        has_image_preview: false,
        image_width: 0,
        image_height: 0,
    }
}

fn image_candidate_wire(candidate: &ImageCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        kind_key: "image_raster".to_owned(),
        has_expected_head: true,
        expected_head: candidate.expected_head().to_string(),
        can_branch: false,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
        has_image_preview: true,
        image_width: candidate.contract().width,
        image_height: candidate.contract().height,
    }
}

#[cfg(test)]
mod tests;
