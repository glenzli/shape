//! Mutable desktop lifecycle over one project and its transient Candidate Shelf.

mod candidate_shelf;
mod operator_drafts;

use shape_core::{
    AiImageCandidate, AudioCandidate, ImageCandidate, ImageResizeCandidate, ShapeProject,
    TextCandidate, TextEditParameters,
};
use shape_domain::{
    Artifact, ArtifactContentContract, ArtifactId, ArtifactKind, IntentSpec, RasterCrop,
    SpeechVoiceSelection,
};

use crate::{audio_origin_key, bounded_text_preview, ffi, project_snapshot};
use candidate_shelf::{Candidate, CandidateShelf};
use operator_drafts::OperatorDrafts;

use crate::operator_catalog::{
    AUDIO_SPEECH_OPERATOR, IMAGE_GENERATE_OPERATOR, IMAGE_RESIZE_OPERATOR,
    ai_image_generate_parameters_from_draft, ai_image_generate_state_from_draft, aspect_policy_key,
    audio_speech_operation_from_draft, compatible_descriptors, descriptor_for,
    image_resize_from_draft, instruction_from_draft, is_image_edit_workspace_operator,
    is_text_workspace_operator, mode_from_draft, resampling_key, style_from_draft, tone_from_draft,
    variant_count_from_draft,
};

use crate::infer_image::InferImageCandidate;
use crate::infer_speech::InferSpeechCandidate;
use crate::infer_text::InferTextCandidate;

const USER_AUTHORED_TEXT_INTENT: &str = "Calibrate text with a user-authored replacement";

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
    operator_drafts: OperatorDrafts,
}

/// Creates a new empty project and opens it as the sole mutable desktop session.
///
/// # Errors
///
/// Returns a user-safe message when the bundle cannot be created.
pub fn create_desktop_project(path: &str, name: &str) -> Result<Box<DesktopSession>, String> {
    let project = ShapeProject::create(path, name).map_err(|error| error.to_string())?;
    desktop_session(project, path)
}

/// Opens a validated project for mutable desktop use.
///
/// # Errors
///
/// Returns a user-safe message when the project cannot be opened.
pub fn open_desktop_session(path: &str) -> Result<Box<DesktopSession>, String> {
    let project = ShapeProject::open(path).map_err(|error| error.to_string())?;
    desktop_session(project, path)
}

fn desktop_session(project: ShapeProject, path: &str) -> Result<Box<DesktopSession>, String> {
    let mut operator_drafts = OperatorDrafts::from_graphs(
        project
            .artifact_working_graphs()
            .map_err(|error| error.to_string())?,
    )?;
    for graph in operator_drafts.initialize_audio_speech_defaults()? {
        project
            .save_artifact_working_graph(&graph)
            .map_err(|error| error.to_string())?;
    }
    Ok(Box::new(DesktopSession {
        project,
        bundle_path: super::project_path(path),
        candidates: CandidateShelf::default(),
        operator_drafts,
    }))
}

impl DesktopSession {
    /// Returns durable accepted state; pending previews are excluded.
    pub fn session_snapshot(&self) -> Result<ffi::ProjectSnapshotWire, String> {
        project_snapshot(&self.project, &self.bundle_path)
    }

    /// Atomically creates the first accepted text source for one compatibility Scene.
    pub fn session_create_text_document(
        &mut self,
        artifact_name: &str,
        initial_text: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        self.project
            .create_text_document(artifact_name, initial_text)
            .map_err(|error| error.to_string())?;
        self.session_snapshot()
    }

    /// Atomically creates one unaccepted `ImageRaster` compatibility Scene with
    /// its exact zero-input `image.generate` Working Graph.
    pub fn session_create_ai_image_draft(
        &mut self,
        artifact_name: &str,
        instruction: &str,
        output_width: u32,
        output_height: u32,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let artifact = Artifact::new(artifact_name, ArtifactKind::ImageRaster)
            .map_err(|error| error.to_string())?;
        let previous = self.operator_drafts.clone();
        if let Err(error) = self.operator_drafts.begin_ai_image_source(
            &artifact,
            instruction,
            output_width,
            output_height,
        ) {
            self.operator_drafts = previous;
            return Err(error);
        }
        let graph = self
            .operator_drafts
            .graph(artifact.id)
            .cloned()
            .ok_or_else(|| "AI image source Working Graph disappeared".to_owned())?;
        if let Err(error) = self.project.create_source_artifact_draft(&artifact, &graph) {
            self.operator_drafts = previous;
            return Err(error.to_string());
        }
        self.session_snapshot()
    }

    /// Creates one durable, detached AI text-editing node with no material input.
    ///
    /// This is the honest compatibility path for graph-first authoring: the
    /// node and its reusable intent may exist before a Source is connected,
    /// while execution remains unavailable until an accepted text input exists.
    pub fn session_create_detached_text_editor(
        &mut self,
        artifact_name: &str,
    ) -> Result<ffi::ProjectSnapshotWire, String> {
        let artifact = Artifact::new(artifact_name, ArtifactKind::TextDocument)
            .map_err(|error| error.to_string())?;
        let previous = self.operator_drafts.clone();
        if let Err(error) = self.operator_drafts.begin_detached_text_editor(&artifact) {
            self.operator_drafts = previous;
            return Err(error);
        }
        let graph = self
            .operator_drafts
            .graph(artifact.id)
            .cloned()
            .ok_or_else(|| "detached text editor Working Graph disappeared".to_owned())?;
        if let Err(error) = self.project.create_source_artifact_draft(&artifact, &graph) {
            self.operator_drafts = previous;
            return Err(error.to_string());
        }
        self.session_snapshot()
    }

    /// Begins one project-backed Operator draft against an accepted source.
    pub fn session_begin_operator_draft(
        &mut self,
        artifact_id: &str,
        operator_type: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        let artifact = self
            .project
            .snapshot()
            .map_err(|error| error.to_string())?
            .artifacts
            .into_iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or_else(|| "artifact does not exist in this project".to_owned())?;
        let descriptor = descriptor_for(artifact.kind, operator_type)?;
        let resize_source_dimensions = if descriptor.type_key == IMAGE_RESIZE_OPERATOR {
            let revision_id = artifact
                .accepted_revision
                .ok_or_else(|| "image.resize requires an accepted image.raster input".to_owned())?;
            let revision = self
                .project
                .revision(revision_id)
                .map_err(|error| error.to_string())?;
            let Some(ArtifactContentContract::ImageRaster(contract)) = revision.content_contract
            else {
                return Err("image.resize requires an image.raster content contract".to_owned());
            };
            Some((contract.width, contract.height))
        } else {
            None
        };
        let previous = self.operator_drafts.clone();
        let mut draft = match self.operator_drafts.begin(&artifact, descriptor) {
            Ok(draft) => draft,
            Err(error) => {
                self.operator_drafts = previous;
                return Err(error);
            }
        };
        if let Some((width, height)) = resize_source_dimensions
            && draft.configuration().is_none()
        {
            draft = match self.operator_drafts.update_image_resize_configuration(
                &draft.id().to_string(),
                width,
                height,
                "fit_within",
                "lanczos3",
            ) {
                Ok((_, draft)) => draft,
                Err(error) => {
                    self.operator_drafts = previous;
                    return Err(error);
                }
            };
        }
        if let Err(error) = self.persist_operator_drafts(artifact.id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact.id, &draft))
    }

    /// Returns the Rust-owned Operator catalog compatible with one accepted source.
    ///
    /// # Errors
    ///
    /// Returns an error when the Artifact identity or project snapshot is invalid.
    pub fn session_operator_descriptors(
        &self,
        artifact_id: &str,
    ) -> Result<Vec<ffi::OperatorDescriptorWire>, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        let artifact = self
            .project
            .snapshot()
            .map_err(|error| error.to_string())?
            .artifacts
            .into_iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or_else(|| "artifact does not exist in this project".to_owned())?;
        if artifact.accepted_revision.is_none() {
            return Ok(Vec::new());
        }
        Ok(compatible_descriptors(artifact.kind)
            .map(operator_descriptor_wire)
            .collect())
    }

    /// Returns all current project-backed Operator drafts.
    pub fn session_operator_drafts(&self) -> Vec<ffi::OperatorDraftWire> {
        self.operator_drafts
            .entries()
            .map(|(artifact_id, draft)| operator_draft_wire(artifact_id, draft))
            .collect()
    }

    /// Saves the authored AI mode and instruction owned by one Writing draft.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or incompatible draft, invalid bounded
    /// instruction, stale accepted source, or persistence failure.
    pub fn session_update_text_transform_draft(
        &mut self,
        draft_id: &str,
        mode_key: &str,
        instruction: &str,
        tone_key: &str,
        style_key: &str,
        variant_count: u8,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self.operator_drafts.update_text_transform_configuration(
            draft_id,
            mode_key,
            instruction,
            tone_key,
            style_key,
            variant_count,
        )?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }

    /// Saves the preset-only authored state owned by one speech synthesis draft.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown or incompatible draft, unsupported
    /// preset, hidden synthetic disclosure, stale source, or persistence failure.
    pub fn session_update_audio_speech_draft(
        &mut self,
        draft_id: &str,
        preset_alias: &str,
        preset_catalog_revision: &str,
        language: &str,
        speed_milli: u16,
        synthetic_disclosure_required: bool,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self.operator_drafts.update_audio_speech_configuration(
            draft_id,
            preset_alias,
            preset_catalog_revision,
            language,
            speed_milli,
            synthetic_disclosure_required,
        )?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }

    /// Saves the exact dimensions, aspect policy, and resampling kernel owned
    /// by one `image.resize` draft.
    pub fn session_update_image_resize_draft(
        &mut self,
        draft_id: &str,
        target_width: u32,
        target_height: u32,
        aspect_policy_key: &str,
        resampling_key: &str,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self.operator_drafts.update_image_resize_configuration(
            draft_id,
            target_width,
            target_height,
            aspect_policy_key,
            resampling_key,
        )?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }

    /// Saves the exact prompt and output canvas owned by one zero-input
    /// `image.generate` draft.
    pub fn session_update_ai_image_draft(
        &mut self,
        draft_id: &str,
        instruction: &str,
        output_width: u32,
        output_height: u32,
    ) -> Result<ffi::OperatorDraftWire, String> {
        let previous = self.operator_drafts.clone();
        let (artifact_id, draft) = self
            .operator_drafts
            .update_ai_image_generate_configuration(
                draft_id,
                instruction,
                output_width,
                output_height,
            )?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(operator_draft_wire(artifact_id, &draft))
    }

    /// Discards one Operator draft without changing accepted history.
    pub fn session_discard_operator_draft(&mut self, draft_id: &str) -> Result<(), String> {
        let source_draft = self
            .operator_drafts
            .entries()
            .find(|(_, draft)| draft.id().to_string() == draft_id)
            .map(|(_, draft)| draft)
            .is_some_and(|draft| draft.input_data_type().is_none());
        if source_draft {
            return Err(
                "a Source Operator cannot be removed without deleting its Scene".to_owned(),
            );
        }
        let previous = self.operator_drafts.clone();
        let artifact_id = self.operator_drafts.discard(draft_id)?;
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(())
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

        let expected_head = artifact
            .accepted_revision
            .ok_or_else(|| "text edit requires an accepted text.document input".to_owned())?;
        self.validate_text_workspace_draft_head(artifact_id, expected_head)?;
        let candidate = self
            .project
            .propose_text_edit(
                artifact_id,
                expected_head,
                TextEditParameters::new(replacement_text),
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
        self.validate_image_edit_workspace_draft_head(artifact_id, expected_head)?;
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
        self.finish_image_edit_workspace_drafts(artifact_id)?;
        self.candidates.push(Candidate::Image(candidate));
        Ok(wire)
    }

    /// Executes the persisted parameters of one exact `image.resize` draft as
    /// a transient image Candidate.
    pub fn session_propose_raster_resize(
        &mut self,
        artifact_id: &str,
        draft_id: &str,
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
            return Err("only raster images support resize candidates".to_owned());
        }
        let expected_head = artifact
            .accepted_revision
            .ok_or_else(|| "raster artifact has no accepted revision".to_owned())?;
        self.validate_image_edit_workspace_draft_head(artifact_id, expected_head)?;
        let resize =
            image_resize_from_draft(self.operator_drafts.draft(artifact_id, draft_id)?)?
                .ok_or_else(|| "image.resize draft has no executable configuration".to_owned())?;
        if self.candidates.contains_image_resize(artifact_id, resize) {
            return Err("resize candidate already exists on the shelf".to_owned());
        }
        let candidate = self
            .project
            .propose_raster_resize(artifact_id, expected_head, resize)
            .map_err(|error| error.to_string())?;
        let wire = image_resize_candidate_wire(&candidate);
        self.finish_image_edit_workspace_drafts(artifact_id)?;
        self.candidates.push(Candidate::ImageResize(candidate));
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
        let preserves_text_intent = matches!(&candidate, Candidate::Text(_));
        let accepted_revision = match candidate {
            Candidate::Text(candidate) => self
                .project
                .accept_text(candidate)
                .map_err(|error| error.to_string())?,
            Candidate::Image(candidate) => self
                .project
                .accept_raster_crop(candidate)
                .map_err(|error| error.to_string())?,
            Candidate::ImageResize(candidate) => self
                .project
                .accept_raster_resize(candidate)
                .map_err(|error| error.to_string())?,
            Candidate::AiImage(candidate) => self
                .project
                .accept_generated_image(candidate)
                .map_err(|error| error.to_string())?,
            Candidate::Audio(candidate) => self
                .project
                .accept_speech_synthesis(candidate)
                .map_err(|error| error.to_string())?,
        };
        self.candidates.discard_artifact(artifact_id);
        if preserves_text_intent
            && self
                .operator_drafts
                .rebase_artifact(artifact_id, accepted_revision.id)
                .unwrap_or(false)
        {
            // Acceptance is already durable. Keep the reusable authored intent
            // pointed at the newly locked text output whenever persistence is
            // available; a later open still rejects any stale graph.
            let _ = self.persist_operator_drafts(artifact_id);
        } else if self.operator_drafts.clear_artifact(artifact_id) {
            // Acceptance already crossed the immutable commit boundary. A cleanup
            // failure must not report that accepted history failed; stale mutable
            // graphs are also excluded when the project next opens.
            let _ = self.project.delete_artifact_working_graph(artifact_id);
        }
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
            return match self.candidates.candidate(candidate_id)? {
                Candidate::Image(candidate) if candidate.artifact_id() == artifact_id => {
                    Ok(ffi::ImagePreviewWire {
                        identity: candidate_id.to_owned(),
                        width: candidate.contract().width,
                        height: candidate.contract().height,
                        png_bytes: candidate.png_bytes().to_vec(),
                    })
                }
                Candidate::ImageResize(candidate) if candidate.artifact_id() == artifact_id => {
                    Ok(ffi::ImagePreviewWire {
                        identity: candidate_id.to_owned(),
                        width: candidate.contract().width,
                        height: candidate.contract().height,
                        png_bytes: candidate.png_bytes().to_vec(),
                    })
                }
                Candidate::AiImage(candidate) if candidate.artifact_id() == artifact_id => {
                    Ok(ffi::ImagePreviewWire {
                        identity: candidate_id.to_owned(),
                        width: candidate.contract().width,
                        height: candidate.contract().height,
                        png_bytes: candidate.bytes().to_vec(),
                    })
                }
                Candidate::Image(_) | Candidate::ImageResize(_) | Candidate::AiImage(_) => {
                    Err("candidate does not belong to the selected artifact".to_owned())
                }
                Candidate::Text(_) | Candidate::Audio(_) => {
                    Err("candidate is not an image preview".to_owned())
                }
            };
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

    /// Returns exact WAV bytes only for one selected accepted `AudioClip` or
    /// transient speech Candidate. Ordinary snapshots remain payload-free.
    pub fn session_audio_preview(
        &self,
        artifact_id: &str,
        candidate_id: &str,
    ) -> Result<ffi::AudioPreviewWire, String> {
        let artifact_id = parse_artifact_id(artifact_id)?;
        if !candidate_id.is_empty() {
            let Candidate::Audio(candidate) = self.candidates.candidate(candidate_id)? else {
                return Err("candidate is not an audio preview".to_owned());
            };
            if candidate.source_artifact_id() != artifact_id {
                return Err("candidate does not belong to the selected review context".to_owned());
            }
            return Ok(ffi::AudioPreviewWire {
                identity: candidate_id.to_owned(),
                duration_millis: candidate.contract().duration_millis(),
                sample_rate_hz: candidate.contract().sample_rate_hz,
                channels: candidate.contract().channels,
                wav_bytes: candidate.bytes().to_vec(),
            });
        }
        let accepted = self
            .project
            .read_accepted(artifact_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "audio artifact has no accepted revision".to_owned())?;
        let Some(ArtifactContentContract::AudioClip(contract)) = accepted.revision.content_contract
        else {
            return Err("accepted content is not an audio clip".to_owned());
        };
        Ok(ffi::AudioPreviewWire {
            identity: accepted.revision.id.to_string(),
            duration_millis: contract.duration_millis(),
            sample_rate_hz: contract.sample_rate_hz,
            channels: contract.channels,
            wav_bytes: accepted.bytes,
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
        let expected_head = candidate.expected_head().ok_or("invalid_candidate")?;
        self.validate_text_workspace_draft_head(artifact_id, expected_head)?;
        let wire = text_candidate_wire(&candidate);
        self.candidates.push(Candidate::Text(candidate));
        Ok(wire)
    }

    /// Adopts a completed speech Candidate after rechecking its text source head.
    pub fn session_adopt_infer_speech(
        &mut self,
        candidate: Box<InferSpeechCandidate>,
    ) -> Result<ffi::CandidateWire, String> {
        let candidate = (*candidate).into_candidate();
        let source_artifact_id = candidate.source_artifact_id();
        let snapshot = self.project.snapshot().map_err(|_| "project_unavailable")?;
        let source = snapshot
            .artifacts
            .iter()
            .find(|artifact| artifact.id == source_artifact_id)
            .ok_or("invalid_artifact")?;
        if source.accepted_revision != Some(candidate.expected_source_head()) {
            return Err("stale_candidate".to_owned());
        }
        if snapshot
            .artifacts
            .iter()
            .any(|artifact| artifact.id == candidate.artifact_id())
        {
            return Err("duplicate_candidate".to_owned());
        }
        self.validate_operator_draft_head(
            source_artifact_id,
            AUDIO_SPEECH_OPERATOR,
            candidate.expected_source_head(),
        )?;
        let wire = audio_candidate_wire(&candidate);
        self.finish_operator_draft(source_artifact_id, AUDIO_SPEECH_OPERATOR)?;
        self.candidates.push(Candidate::Audio(candidate));
        Ok(wire)
    }

    /// Adopts a completed AI Image Candidate only while its exact zero-input
    /// draft and unaccepted target still match the executed parameters.
    pub fn session_adopt_infer_image(
        &mut self,
        candidate: Box<InferImageCandidate>,
    ) -> Result<ffi::CandidateWire, String> {
        let (candidate, draft_id) = (*candidate).into_parts();
        let artifact_id = candidate.artifact_id();
        let artifact = self
            .project
            .snapshot()
            .map_err(|_| "project_unavailable")?
            .artifacts
            .into_iter()
            .find(|artifact| artifact.id == artifact_id)
            .ok_or("invalid_artifact")?;
        if artifact.kind != ArtifactKind::ImageRaster || artifact.accepted_revision.is_some() {
            return Err("stale_candidate".to_owned());
        }
        let draft = self.operator_drafts.draft(artifact_id, &draft_id)?;
        if draft.operator_type().as_str() != IMAGE_GENERATE_OPERATOR
            || draft.input_data_type().is_some()
            || ai_image_generate_parameters_from_draft(draft)?
                .as_ref()
                .is_none_or(|parameters| parameters != candidate.parameters())
        {
            return Err("stale_candidate".to_owned());
        }
        let wire = ai_image_candidate_wire(&candidate);
        self.candidates.push(Candidate::AiImage(candidate));
        Ok(wire)
    }

    fn validate_operator_draft_head(
        &self,
        artifact_id: ArtifactId,
        operator_type: &str,
        current_head: shape_domain::RevisionId,
    ) -> Result<(), String> {
        let Some(graph) = self.operator_drafts.graph(artifact_id) else {
            return Ok(());
        };
        if graph
            .operators()
            .iter()
            .any(|draft| draft.operator_type().as_str() == operator_type)
            && graph.expected_revision_id() != Some(current_head)
        {
            return Err("the Working Graph is based on a stale accepted source".to_owned());
        }
        Ok(())
    }

    fn validate_text_workspace_draft_head(
        &self,
        artifact_id: ArtifactId,
        current_head: shape_domain::RevisionId,
    ) -> Result<(), String> {
        let Some(graph) = self.operator_drafts.graph(artifact_id) else {
            return Ok(());
        };
        if graph
            .operators()
            .iter()
            .any(|draft| is_text_workspace_operator(draft.operator_type().as_str()))
            && graph.expected_revision_id() != Some(current_head)
        {
            return Err("the Working Graph is based on a stale accepted source".to_owned());
        }
        Ok(())
    }

    fn validate_image_edit_workspace_draft_head(
        &self,
        artifact_id: ArtifactId,
        current_head: shape_domain::RevisionId,
    ) -> Result<(), String> {
        let Some(graph) = self.operator_drafts.graph(artifact_id) else {
            return Ok(());
        };
        if graph
            .operators()
            .iter()
            .any(|draft| is_image_edit_workspace_operator(draft.operator_type().as_str()))
            && graph.expected_revision_id() != Some(current_head)
        {
            return Err("the Working Graph is based on a stale accepted source".to_owned());
        }
        Ok(())
    }

    fn finish_operator_draft(
        &mut self,
        artifact_id: ArtifactId,
        operator_type: &str,
    ) -> Result<(), String> {
        let previous = self.operator_drafts.clone();
        if !self.operator_drafts.finish(artifact_id, operator_type) {
            return Ok(());
        }
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(())
    }

    fn finish_image_edit_workspace_drafts(
        &mut self,
        artifact_id: ArtifactId,
    ) -> Result<(), String> {
        let previous = self.operator_drafts.clone();
        if !self
            .operator_drafts
            .finish_image_edit_workspace(artifact_id)
        {
            return Ok(());
        }
        if let Err(error) = self.persist_operator_drafts(artifact_id) {
            self.operator_drafts = previous;
            return Err(error);
        }
        Ok(())
    }

    fn persist_operator_drafts(&self, artifact_id: ArtifactId) -> Result<(), String> {
        if let Some(graph) = self.operator_drafts.graph(artifact_id) {
            self.project
                .save_artifact_working_graph(graph)
                .map_err(|error| error.to_string())
        } else {
            self.project
                .delete_artifact_working_graph(artifact_id)
                .map_err(|error| error.to_string())
        }
    }
}

fn operator_draft_wire(
    context_artifact_id: ArtifactId,
    draft: &shape_domain::WorkingOperatorDraft,
) -> ffi::OperatorDraftWire {
    let configuration_schema = draft
        .configuration()
        .map_or_else(String::new, |configuration| {
            configuration.schema().as_str().to_owned()
        });
    let audio_speech_operation = audio_speech_operation_from_draft(draft)
        .expect("session admits only validated Operator draft configurations");
    let image_resize = image_resize_from_draft(draft)
        .expect("session admits only validated Operator draft configurations");
    let ai_image = ai_image_generate_state_from_draft(draft)
        .expect("session admits only validated Operator draft configurations");
    let (audio_speech_preset_alias, audio_speech_preset_catalog_revision) =
        match audio_speech_operation
            .as_ref()
            .map(|operation| &operation.voice)
        {
            Some(SpeechVoiceSelection::Preset(selection)) => (
                selection.alias.as_str().to_owned(),
                selection.catalog_revision.clone(),
            ),
            Some(SpeechVoiceSelection::AuthorizedReference(_)) => {
                unreachable!("preset-only speech draft codec rejects authorized voice references")
            }
            None => (String::new(), String::new()),
        };
    ffi::OperatorDraftWire {
        draft_id: draft.id().to_string(),
        context_artifact_id: context_artifact_id.to_string(),
        operator_type_key: draft.operator_type().to_string(),
        has_input_data_type: draft.input_data_type().is_some(),
        input_data_type_key: draft
            .input_data_type()
            .map_or_else(String::new, ToString::to_string),
        output_data_type_key: draft.output_data_type().to_string(),
        configuration_schema,
        text_transform_mode: mode_from_draft(draft)
            .expect("session admits only validated Operator draft configurations"),
        text_transform_instruction: instruction_from_draft(draft)
            .expect("session admits only validated Operator draft configurations"),
        text_transform_tone: tone_from_draft(draft)
            .expect("session admits only validated Operator draft configurations"),
        text_transform_style: style_from_draft(draft)
            .expect("session admits only validated Operator draft configurations"),
        text_transform_variant_count: variant_count_from_draft(draft)
            .expect("session admits only validated Operator draft configurations"),
        audio_speech_preset_alias,
        audio_speech_preset_catalog_revision,
        audio_speech_language: audio_speech_operation
            .as_ref()
            .map_or_else(String::new, |operation| operation.language.clone()),
        audio_speech_speed_milli: audio_speech_operation
            .as_ref()
            .map_or(0, |operation| operation.speed_milli),
        audio_speech_disclosure_required: audio_speech_operation
            .is_some_and(|operation| operation.synthetic_disclosure_required),
        image_resize_target_width: image_resize.map_or(0, |resize| resize.target().width()),
        image_resize_target_height: image_resize.map_or(0, |resize| resize.target().height()),
        image_resize_aspect_policy: image_resize.map_or_else(String::new, |resize| {
            aspect_policy_key(resize.aspect_policy()).to_owned()
        }),
        image_resize_resampling: image_resize.map_or_else(String::new, |resize| {
            resampling_key(resize.resampling()).to_owned()
        }),
        ai_image_instruction: ai_image
            .as_ref()
            .map_or_else(String::new, |state| state.instruction.clone()),
        ai_image_output_width: ai_image.as_ref().map_or(0, |state| state.output.width()),
        ai_image_output_height: ai_image.map_or(0, |state| state.output.height()),
    }
}

fn operator_descriptor_wire(
    descriptor: &crate::operator_catalog::OperatorDescriptor,
) -> ffi::OperatorDescriptorWire {
    ffi::OperatorDescriptorWire {
        operator_type: descriptor.type_key.to_owned(),
        input_data_type: descriptor.input_data_type.to_owned(),
        output_data_type: descriptor.output_data_type.to_owned(),
        category: descriptor.category_key.to_owned(),
        icon: descriptor.icon_key.to_owned(),
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
        Candidate::ImageResize(candidate) => image_resize_candidate_wire(candidate),
        Candidate::AiImage(candidate) => ai_image_candidate_wire(candidate),
        Candidate::Audio(candidate) => audio_candidate_wire(candidate),
    }
}

fn ai_image_candidate_wire(candidate: &AiImageCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: candidate.artifact_name().to_owned(),
        kind_key: "image_raster".to_owned(),
        has_expected_head: false,
        expected_head: String::new(),
        can_branch: false,
        has_text_preview: false,
        text_preview_truncated: false,
        text_preview: String::new(),
        has_image_preview: true,
        image_width: candidate.contract().width,
        image_height: candidate.contract().height,
        has_audio_preview: false,
        audio_duration_millis: 0,
        audio_sample_rate_hz: 0,
        audio_channels: 0,
        audio_origin_key: String::new(),
    }
}

fn text_candidate_wire(candidate: &TextCandidate) -> ffi::CandidateWire {
    let (text_preview, text_preview_truncated) =
        bounded_text_preview(candidate.text().as_bytes()).expect("text candidates are valid UTF-8");
    let expected_head = candidate.expected_head();
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: String::new(),
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
        has_audio_preview: false,
        audio_duration_millis: 0,
        audio_sample_rate_hz: 0,
        audio_channels: 0,
        audio_origin_key: String::new(),
    }
}

fn image_candidate_wire(candidate: &ImageCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: String::new(),
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
        has_audio_preview: false,
        audio_duration_millis: 0,
        audio_sample_rate_hz: 0,
        audio_channels: 0,
        audio_origin_key: String::new(),
    }
}

fn image_resize_candidate_wire(candidate: &ImageResizeCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.artifact_id().to_string(),
        artifact_name: String::new(),
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
        has_audio_preview: false,
        audio_duration_millis: 0,
        audio_sample_rate_hz: 0,
        audio_channels: 0,
        audio_origin_key: String::new(),
    }
}

fn audio_candidate_wire(candidate: &AudioCandidate) -> ffi::CandidateWire {
    ffi::CandidateWire {
        candidate_id: candidate.receipt().attempt_id.to_string(),
        artifact_id: candidate.artifact_id().to_string(),
        context_artifact_id: candidate.source_artifact_id().to_string(),
        artifact_name: candidate.artifact_name().to_owned(),
        kind_key: "audio_clip".to_owned(),
        has_expected_head: true,
        expected_head: candidate.expected_source_head().to_string(),
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
        audio_origin_key: audio_origin_key(candidate.contract().origin).to_owned(),
    }
}

#[cfg(test)]
mod tests;
