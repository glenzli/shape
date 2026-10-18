//! Exact saved composition and material bytes for an independent Deck project.
use std::{fs, path::Path};

use serde_json::{Value, json};
use shape_domain::{ArtifactId, ArtifactKind, ContentDigest, WorkingInput};

use super::{CardKitDraft, CardKitError, ShapeProject, invalid};
use crate::project::deck_handoff::files::write_new;

impl ShapeProject {
    /// Publishes a saved draft and every exact pinned material without accepting or rendering it.
    /// The final manifest is the publication marker. Failed partial destinations are not reused.
    /// # Errors
    /// Rejects changed drafts/heads, corrupt pins and existing or project-internal destinations.
    pub fn export_deck_card_transfer(
        &self,
        artifact_id: ArtifactId,
        draft_digest: &str,
        request_id: &str,
        destination: &Path,
    ) -> Result<Value, CardKitError> {
        let draft = self.checked_card_kit_draft(artifact_id, draft_digest)?;
        self.publish_deck_card_snapshot(&draft, request_id, destination, None)
    }

    /// Publishes one exact accepted card revision even when a newer draft exists.
    /// # Errors
    /// Rejects changed accepted heads, wrong pins, corrupt materials and unsafe destinations.
    pub fn export_deck_accepted_card_transfer(
        &self,
        source: WorkingInput,
        request_id: &str,
        destination: &Path,
    ) -> Result<Value, CardKitError> {
        let draft = self.accepted_deck_card_snapshot(source)?;
        self.publish_deck_card_snapshot(&draft, request_id, destination, Some(source))
    }

    pub(in crate::project) fn accepted_deck_card_snapshot(
        &self,
        source: WorkingInput,
    ) -> Result<CardKitDraft, CardKitError> {
        let artifact = self
            .store
            .artifact(source.artifact_id)
            .map_err(crate::CoreError::from)?
            .ok_or_else(|| invalid("card set is missing"))?;
        if artifact.accepted_revision != Some(source.revision_id) {
            return Err(invalid(
                "selected accepted card head changed; reload before exporting",
            ));
        }
        let composition = self.read_card_kit(source)?;
        let revision = self.revision(source.revision_id)?;
        Ok(CardKitDraft {
            artifact_id: source.artifact_id,
            expected_head: Some(source.revision_id),
            draft_digest: revision.content.digest.to_string(),
            composition,
        })
    }

    fn publish_deck_card_snapshot(
        &self,
        draft: &CardKitDraft,
        request_id: &str,
        destination: &Path,
        accepted_source: Option<WorkingInput>,
    ) -> Result<Value, CardKitError> {
        let artifact_id = draft.artifact_id;
        let draft_digest = &draft.draft_digest;
        if !super::super::deck_handoff::valid_token(request_id) || !destination.is_absolute() {
            return Err(invalid(
                "Deck transfer needs an absolute destination and bounded request id",
            ));
        }
        // Historical accepted material pins are preserved, including hidden content.
        let materials = self.card_kit_materials(&draft.composition, false)?;
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("destination needs a parent"))?
            .canonicalize()?;
        let destination = parent.join(
            destination
                .file_name()
                .ok_or_else(|| invalid("destination needs a name"))?,
        );
        if destination.starts_with(self.store.root().canonicalize()?) {
            return Err(invalid("Deck transfer must be outside the Shape project"));
        }
        fs::create_dir(&destination)?;
        fs::create_dir(destination.join("materials"))?;
        let mut entries = Vec::new();
        for material in materials {
            let suffix = if material.kind == ArtifactKind::ImageRaster {
                "png"
            } else {
                "txt"
            };
            let relative_path = format!(
                "materials/{}-{}.{suffix}",
                material.source.artifact_id, material.source.revision_id
            );
            write_new(&destination.join(&relative_path), &material.bytes)?;
            entries.push(json!({
                "source":material.source,"content":material.content,
                "contract":self.revision(material.source.revision_id)?.content_contract,
                "relative_path":relative_path
            }));
        }
        if let Some(source) = accepted_source {
            self.accepted_deck_card_snapshot(source)?;
        } else {
            self.checked_card_kit_draft(artifact_id, draft_digest)?;
        }
        let manifest = json!({
            "schema":"deck.card-transfer/1","request_id":request_id,
            "origin":{"producer":"shape","project_id":self.snapshot()?.metadata.id,
                "kit_id":artifact_id,"expected_head":draft.expected_head,"draft_digest":draft_digest},
            "composition":draft.composition,"materials":entries
        });
        let bytes = serde_json::to_vec_pretty(&manifest)?;
        let path = destination.join("manifest.json");
        write_new(&path, &bytes)?;
        Ok(
            json!({"manifest_path":path,"manifest_blake3":ContentDigest::from_bytes(&bytes).to_string(),
            "request_id":request_id,"origin":manifest["origin"]}),
        )
    }
}

#[cfg(test)]
mod tests;
