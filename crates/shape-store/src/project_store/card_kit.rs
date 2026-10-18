//! Card manifests cross the same object publication and expected-head transaction as media.
use shape_domain::{
    ArtifactKind, CARD_KIT_MAX_BYTES, CardKitDocument, Transformation, TransformationKind,
};
use shape_execution::ExecutionReceipt;

use super::ProjectStore;
use crate::StoreError;

pub(super) fn document(bytes: &[u8]) -> Result<CardKitDocument, StoreError> {
    if bytes.len() > CARD_KIT_MAX_BYTES + 1024 {
        return Err(StoreError::InvalidCommit(
            "card kit manifest exceeds its bound",
        ));
    }
    let document: CardKitDocument = serde_json::from_slice(bytes)?;
    document.validate().map_err(StoreError::InvalidCommit)?;
    Ok(document)
}

pub(super) fn validate_commit(
    store: &ProjectStore,
    kind: ArtifactKind,
    bytes: &[u8],
    provenance: (&Transformation, &ExecutionReceipt),
    input_heads: &[(shape_domain::ArtifactId, shape_domain::RevisionId)],
) -> Result<(), StoreError> {
    if kind != ArtifactKind::CardKit {
        return Ok(());
    }
    let document = document(bytes)?;
    for card in &document.composition.cards {
        for (source, expected_kind) in [
            (card.artwork, ArtifactKind::ImageRaster),
            (card.text_material, ArtifactKind::TextDocument),
        ] {
            let Some(source) = source else {
                continue;
            };
            let artifact = store
                .artifact(source.artifact_id)?
                .ok_or(StoreError::UnknownArtifact(source.artifact_id))?;
            let revision = store.revision(source.revision_id)?;
            if artifact.kind != expected_kind
                || revision.artifact_id != source.artifact_id
                || (expected_kind == ArtifactKind::ImageRaster
                    && revision.content.media_type != "image/png")
                || (expected_kind == ArtifactKind::TextDocument
                    && !revision.content.media_type.starts_with("text/"))
            {
                return Err(StoreError::InvalidCommit(
                    "card material kind or revision does not match",
                ));
            }
        }
    }
    let inputs = document.composition.inputs();
    let expected: Vec<_> = inputs
        .iter()
        .map(|s| (s.artifact_id, s.revision_id))
        .collect();
    let mut observed = input_heads.to_vec();
    observed.sort_unstable();
    let mut revisions: Vec<_> = inputs.iter().map(|s| s.revision_id).collect();
    revisions.sort_unstable();
    let (transformation, receipt) = provenance;
    let mut declared = transformation.inputs.clone();
    declared.sort_unstable();
    if expected != observed
        || revisions != declared
        || transformation.kind != TransformationKind::Composite
        || transformation.operation.is_some()
        || receipt.capability.as_str() != "card_kit.compose"
        || receipt.executor.id != "shape.builtin.card-kit-composition"
    {
        return Err(StoreError::InvalidCommit(
            "card kit composition provenance does not match its pinned materials",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
