//! File contracts for independent Deck editing and one isolated Shape image-generation task.
pub(super) mod files;
mod types;

use super::{CardKitDraft, ShapeProject};
use crate::CoreError;
use serde_json::json;
use shape_domain::{
    ArtifactContentContract, ArtifactId, ArtifactKind, ContentDigest, WorkingInput,
};
use std::{fs, path::Path};
use thiserror::Error;
use types::{
    Document, EditRequest, EditResult, Input, MaterialOutput, MaterialRequest, MaterialResult,
    Origin, Outcome,
};

#[derive(Debug, Error)]
pub enum DeckHandoffError {
    #[error("invalid Deck handoff: {0}")]
    Invalid(&'static str),
    #[error("Deck handoff file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Deck handoff JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    CardKit(#[from] super::card_kit::CardKitError),
}
fn invalid(message: &'static str) -> DeckHandoffError {
    DeckHandoffError::Invalid(message)
}
pub(super) fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn check_error(outcome: &Outcome, error: Option<&str>) -> Result<(), DeckHandoffError> {
    if match outcome {
        Outcome::Failed => error.is_none_or(|s| s.trim().is_empty() || s.len() > 2048),
        _ => error.is_some(),
    } {
        return Err(invalid("terminal outcome and error do not match"));
    }
    Ok(())
}
fn material_request(path: &Path) -> Result<(MaterialRequest, String), DeckHandoffError> {
    let bytes = files::read(path, 64 * 1024)?;
    let request: MaterialRequest = serde_json::from_slice(&bytes)?;
    if request.schema != "deck.material-task/1"
        || !valid_token(&request.request_id)
        || !valid_token(&request.target.project_id)
        || !valid_token(&request.target.kit_id)
        || !valid_token(&request.target.card_id)
        || !valid_digest(&request.target.draft_digest)
        || request.prompt.trim().is_empty()
        || request.prompt.len() > 16 * 1024
        || request
            .prompt
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        || request.canvas.width == 0
        || request.canvas.height == 0
        || request.canvas.width > 4096
        || request.canvas.height > 4096
        || u64::from(request.canvas.width) * u64::from(request.canvas.height) > 16_777_216
    {
        return Err(invalid("unsupported single-image task or target"));
    }
    if let Some(snapshot) = &request.target.card_snapshot {
        if serde_json::to_vec(snapshot)?.len() > 24 * 1024 {
            return Err(invalid("material task card snapshot exceeds 24 KiB"));
        }
        let card: shape_domain::CardKitCard = serde_json::from_value(snapshot.clone())?;
        if card.id != request.target.card_id || card.artwork != request.target.artwork {
            return Err(invalid(
                "material task card snapshot does not match its target",
            ));
        }
    }
    Ok((request, ContentDigest::from_bytes(&bytes).to_string()))
}

impl ShapeProject {
    /// Creates a tracked external edit request from one exact saved Shape `CardKit` draft.
    /// # Errors
    /// Rejects stale identities, existing/internal destinations and invalid continuation projects.
    pub fn prepare_deck_handoff(
        &mut self,
        artifact_id: ArtifactId,
        draft_digest: &str,
        request_id: &str,
        directory: &Path,
        document_json: &str,
    ) -> Result<String, DeckHandoffError> {
        let draft = self.load_card_kit_draft(artifact_id)?;
        if draft.draft_digest != draft_digest {
            return Err(invalid("card set draft changed; reload and retry"));
        }
        self.prepare_deck_snapshot_handoff(&draft, request_id, directory, document_json, None)
    }

    /// Prepares a handoff from the selected accepted version, independently of saved draft edits.
    /// # Errors
    /// Rejects changed pins, existing/internal destinations and invalid continuation projects.
    pub fn prepare_deck_accepted_handoff(
        &self,
        source: WorkingInput,
        request_id: &str,
        directory: &Path,
        document_json: &str,
    ) -> Result<String, DeckHandoffError> {
        let draft = self.accepted_deck_card_snapshot(source)?;
        self.prepare_deck_snapshot_handoff(
            &draft,
            request_id,
            directory,
            document_json,
            Some(source),
        )
    }

    fn prepare_deck_snapshot_handoff(
        &self,
        draft: &CardKitDraft,
        request_id: &str,
        directory: &Path,
        document_json: &str,
        accepted_source: Option<WorkingInput>,
    ) -> Result<String, DeckHandoffError> {
        let artifact_id = draft.artifact_id;
        let draft_digest = &draft.draft_digest;
        self.verify_deck_snapshot_selection(draft, accepted_source)?;
        if !valid_token(request_id) {
            return Err(invalid("invalid request id"));
        }
        let directory = files::new_path(directory)?;
        if directory.starts_with(self.store.root().canonicalize()?) {
            return Err(invalid("handoff must be outside the Shape project"));
        }
        fs::create_dir(&directory)?;
        let origin = Origin {
            producer: "shape".into(),
            project_id: self.snapshot()?.metadata.id,
            kit_id: artifact_id,
            expected_head: draft.expected_head,
            draft_digest: draft_digest.into(),
        };
        let input = if document_json.is_empty() {
            let transfer = if let Some(source) = accepted_source {
                self.export_deck_accepted_card_transfer(
                    source,
                    request_id,
                    &directory.join("transfer"),
                )?
            } else {
                self.export_deck_card_transfer(
                    artifact_id,
                    draft_digest,
                    request_id,
                    &directory.join("transfer"),
                )?
            };
            Input::Transfer {
                manifest_path: serde_json::from_value(transfer["manifest_path"].clone())?,
                manifest_blake3: transfer["manifest_blake3"]
                    .as_str()
                    .ok_or_else(|| invalid("transfer digest missing"))?
                    .into(),
            }
        } else {
            let mut document: Document = serde_json::from_str(document_json)?;
            document.project_path = checked_document(&document)?;
            let current = ShapeProject::inspect(&document.project_path)?;
            if current.snapshot.metadata.id != document.project_id
                || !current
                    .snapshot
                    .artifacts
                    .iter()
                    .any(|a| a.id == document.kit_id && a.kind == ArtifactKind::CardKit)
            {
                return Err(invalid("linked Deck project or card set changed"));
            }
            Input::Project {
                project_path: document.project_path,
                project_id: document.project_id,
                kit_id: document.kit_id,
            }
        };
        let request = EditRequest {
            schema: "deck.external-edit/1".into(),
            request_id: request_id.into(),
            origin,
            input,
        };
        let bytes = serde_json::to_vec_pretty(&request)?;
        let request_path = directory.join("request.json");
        self.verify_deck_snapshot_selection(draft, accepted_source)?;
        files::write_new(&request_path, &bytes)?;
        Ok(serde_json::to_string(
            &json!({"request_path":request_path,"result_path":directory.join("result.json"),"request_blake3":ContentDigest::from_bytes(&bytes).to_string(),"request":request}),
        )?)
    }

    fn verify_deck_snapshot_selection(
        &self,
        draft: &CardKitDraft,
        accepted_source: Option<WorkingInput>,
    ) -> Result<(), DeckHandoffError> {
        let current = if let Some(source) = accepted_source {
            self.accepted_deck_card_snapshot(source)?
        } else {
            self.load_card_kit_draft(draft.artifact_id)?
        };
        let head = self
            .snapshot()?
            .artifacts
            .into_iter()
            .find(|artifact| artifact.id == draft.artifact_id)
            .ok_or_else(|| invalid("card set disappeared"))?
            .accepted_revision;
        if current.draft_digest != draft.draft_digest
            || current.expected_head != draft.expected_head
            || head != draft.expected_head
        {
            return Err(invalid(
                "selected card snapshot changed; reload before handoff",
            ));
        }
        Ok(())
    }

    /// Independently verifies a Deck terminal result and every returned PDF/ZIP exact byte identity.
    /// No Shape store or candidate is mutated, so a native worker can call this without a session.
    /// # Errors
    /// Rejects wrong requests, nonterminal or malformed results, aliases and changed output bytes.
    pub fn verify_deck_output_return(
        request_path: &Path,
        result_path: &Path,
        expected_request_blake3: &str,
    ) -> Result<String, DeckHandoffError> {
        let request_bytes = files::read(request_path, 64 * 1024)?;
        if !valid_digest(expected_request_blake3)
            || ContentDigest::from_bytes(&request_bytes).to_string() != expected_request_blake3
        {
            return Err(invalid("handoff request changed after it was prepared"));
        }
        let request: EditRequest = serde_json::from_slice(&request_bytes)?;
        if request.schema != "deck.external-edit/1"
            || request.origin.producer != "shape"
            || !valid_token(&request.request_id)
            || !valid_digest(&request.origin.draft_digest)
        {
            return Err(invalid("unsupported edit request"));
        }
        if request_path.canonicalize()? == result_path.canonicalize()? {
            return Err(invalid("request and result alias"));
        }
        let mut result: EditResult = serde_json::from_slice(&files::read(result_path, 64 * 1024)?)?;
        if result.schema != "deck.external-edit-result/1"
            || result.request_id != request.request_id
            || result.request_blake3 != ContentDigest::from_bytes(&request_bytes).to_string()
        {
            return Err(invalid("result does not belong to this exact request"));
        }
        check_error(&result.outcome, result.error.as_deref())?;
        if let Some(document) = result.document.as_mut() {
            document.project_path = checked_document(document)?;
            check_return_document(&request, document)?;
        }
        if result.outcome != Outcome::Completed {
            if !result.outputs.is_empty() {
                return Err(invalid("cancelled or failed edit cannot return outputs"));
            }
            return Ok(serde_json::to_string(&result)?);
        }
        let document = result
            .document
            .as_ref()
            .ok_or_else(|| invalid("completed edit needs a Deck document"))?;
        if document.accepted_revision_id.is_none()
            || result.outputs.is_empty()
            || result.outputs.len() > 16
        {
            return Err(invalid(
                "completed edit needs an accepted version and bounded outputs",
            ));
        }
        if let Input::Project {
            project_id, kit_id, ..
        } = &request.input
            && (*project_id != document.project_id || *kit_id != document.kit_id)
        {
            return Err(invalid("returned Deck document identity changed"));
        }
        let mut total = 0_u64;
        let mut paths = std::collections::BTreeSet::new();
        for output in &mut result.outputs {
            total = total
                .checked_add(output.byte_length)
                .ok_or_else(|| invalid("output budget overflow"))?;
            if total > 512 * 1024 * 1024 || !valid_digest(&output.blake3) {
                return Err(invalid("returned output exceeds the byte budget"));
            }
            let path = output.path.canonicalize()?;
            if path.starts_with(&document.project_path)
                || path == request_path.canonicalize()?
                || path == result_path.canonicalize()?
                || !paths.insert(path.clone())
            {
                return Err(invalid(
                    "returned output aliases a project or another handoff file",
                ));
            }
            let (digest, prefix) =
                files::output_digest(&output.path, output.byte_length, 512 * 1024 * 1024)?;
            let signature = match (output.kind.as_str(), output.media_type.as_str()) {
                ("print_pdf", "application/pdf") => prefix.starts_with(b"%PDF-"),
                ("materials_zip", "application/zip") => prefix.starts_with(b"PK\x03\x04"),
                _ => false,
            };
            if digest != output.blake3 || !signature {
                return Err(invalid("returned output bytes or media do not match"));
            }
            output.path = path;
        }
        Ok(serde_json::to_string(&result)?)
    }

    /// Validates an isolated text-to-image task before creating any local project.
    /// # Errors
    /// Rejects unsupported fields, oversized prompts, invalid target/canvas and existing results.
    pub fn read_deck_material_task(
        request_path: &Path,
        result_path: &Path,
    ) -> Result<String, DeckHandoffError> {
        files::new_path(result_path)?;
        let (request, digest) = material_request(request_path)?;
        Ok(serde_json::to_string(
            &json!({"request":request,"request_blake3":digest}),
        )?)
    }

    /// Publishes an explicit cancelled/failed result without needing an open project.
    /// # Errors
    /// Rejects completed outcome, invalid requests and attempts to overwrite a prior receipt.
    pub fn finish_deck_material_task(
        request_path: &Path,
        result_path: &Path,
        expected_request_blake3: &str,
        outcome: &str,
        error: &str,
    ) -> Result<String, DeckHandoffError> {
        let (request, digest) = material_request(request_path)?;
        if digest != expected_request_blake3 {
            return Err(invalid("material task changed after it was opened"));
        }
        let outcome = match outcome {
            "cancelled" => Outcome::Cancelled,
            "failed" => Outcome::Failed,
            _ => return Err(invalid("completed material needs an accepted image")),
        };
        let error = if error.is_empty() {
            None
        } else {
            Some(error.into())
        };
        check_error(&outcome, error.as_deref())?;
        publish_material(
            result_path,
            &MaterialResult {
                schema: "deck.material-task-result/1".into(),
                request_id: request.request_id,
                request_blake3: digest,
                outcome,
                output: None,
                error,
            },
        )
    }

    /// Exports one exact accepted task image only after the user explicitly returns it.
    /// # Errors
    /// Rejects noncurrent or unaccepted images, invalid media/bounds and existing destinations.
    pub fn return_deck_material(
        &self,
        request_path: &Path,
        result_path: &Path,
        source: WorkingInput,
        expected_request_blake3: &str,
    ) -> Result<String, DeckHandoffError> {
        let (request, digest) = material_request(request_path)?;
        if digest != expected_request_blake3 {
            return Err(invalid("material task changed after it was opened"));
        }
        let result_path = files::new_path(result_path)?;
        if result_path.starts_with(self.store.root().canonicalize()?) {
            return Err(invalid("result must be outside the task project"));
        }
        let artifact = self
            .store
            .artifact(source.artifact_id)
            .map_err(CoreError::from)?
            .ok_or_else(|| invalid("task image is missing"))?;
        if artifact.kind != ArtifactKind::ImageRaster
            || artifact.accepted_revision != Some(source.revision_id)
        {
            return Err(invalid("return requires this exact current accepted image"));
        }
        let accepted = self.read_revision_content(source.revision_id)?;
        let contract = accepted
            .revision
            .content_contract
            .ok_or_else(|| invalid("task image has no raster contract"))?;
        let ArtifactContentContract::ImageRaster(raster) = &contract else {
            return Err(invalid("task output is not an image"));
        };
        if accepted.revision.artifact_id != source.artifact_id
            || accepted.revision.content.media_type != "image/png"
            || accepted.bytes.len() > 20 * 1024 * 1024
            || raster.width > 4096
            || raster.height > 4096
            || u64::from(raster.width) * u64::from(raster.height) > 16_777_216
        {
            return Err(invalid(
                "task image exceeds the supported PNG output contract",
            ));
        }
        let path = result_path
            .parent()
            .ok_or_else(|| invalid("result needs a parent"))?
            .join(format!("shape-image-{}.png", uuid::Uuid::now_v7()));
        files::write_new(&path, &accepted.bytes)?;
        let output = MaterialOutput {
            path,
            byte_length: accepted.bytes.len() as u64,
            blake3: ContentDigest::from_bytes(&accepted.bytes).to_string(),
            media_type: "image/png".into(),
            source,
            contract,
        };
        publish_material(
            &result_path,
            &MaterialResult {
                schema: "deck.material-task-result/1".into(),
                request_id: request.request_id,
                request_blake3: digest,
                outcome: Outcome::Completed,
                output: Some(output),
                error: None,
            },
        )
    }
}

fn checked_document(document: &Document) -> Result<std::path::PathBuf, DeckHandoffError> {
    let path = &document.project_path;
    if !path.is_absolute()
        || !fs::symlink_metadata(path)?.file_type().is_dir()
        || path.extension().and_then(|p| p.to_str()) != Some("deck")
    {
        return Err(invalid(
            "Deck document must be an absolute local .deck project",
        ));
    }
    Ok(path.canonicalize()?)
}
fn check_return_document(
    request: &EditRequest,
    document: &Document,
) -> Result<(), DeckHandoffError> {
    let state = ShapeProject::inspect(&document.project_path)?;
    if state.snapshot.metadata.id != document.project_id
        || !state
            .snapshot
            .artifacts
            .iter()
            .any(|a| a.id == document.kit_id && a.kind == ArtifactKind::CardKit)
    {
        return Err(invalid("returned Deck document identities do not exist"));
    }
    if let Some(revision_id) = document.accepted_revision_id {
        let (metadata, _, bytes) = shape_store::ProjectStore::inspect_card_kit_revision(
            &document.project_path,
            WorkingInput {
                artifact_id: document.kit_id,
                revision_id,
            },
        )
        .map_err(CoreError::from)?;
        if metadata.id != document.project_id {
            return Err(invalid("returned Deck project identity changed"));
        }
        let accepted: shape_domain::CardKitDocument = serde_json::from_slice(&bytes)?;
        accepted.validate().map_err(invalid)?;
    }
    match &request.input {
        Input::Project {
            project_path,
            project_id,
            kit_id,
        } => {
            if project_path.canonicalize()? != document.project_path
                || *project_id != document.project_id
                || *kit_id != document.kit_id
            {
                return Err(invalid(
                    "returned Deck continuation is another project or kit",
                ));
            }
        }
        Input::Transfer {
            manifest_blake3, ..
        } => {
            let imported: serde_json::Value = serde_json::from_slice(&files::read(
                &document.project_path.join("deck-transfer.json"),
                512 * 1024,
            )?)?;
            if imported["schema"] != "deck.transfer-import/1"
                || imported["project_id"] != json!(document.project_id)
                || imported["kit_id"] != json!(document.kit_id)
                || imported["request_id"] != request.request_id
                || imported["manifest_blake3"] != *manifest_blake3
                || imported["origin"] != serde_json::to_value(&request.origin)?
            {
                return Err(invalid(
                    "Deck import receipt does not match this exact transfer",
                ));
            }
        }
    }
    Ok(())
}
fn publish_material(path: &Path, result: &MaterialResult) -> Result<String, DeckHandoffError> {
    let path = files::new_path(path)?;
    let bytes = serde_json::to_vec_pretty(&result)?;
    files::write_new(&path, &bytes)?;
    Ok(serde_json::to_string(
        &json!({"result_path":path,"result":result}),
    )?)
}

#[cfg(test)]
mod tests;
