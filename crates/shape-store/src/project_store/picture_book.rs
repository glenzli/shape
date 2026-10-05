//! Book manifests use the ordinary object/receipt/CAS transaction with exact source pins.
use crate::{ProjectStore, StoreError};
use shape_domain::{
    ArtifactId, ArtifactKind, PictureBookDocument, RevisionId, Transformation, TransformationKind,
};
use shape_execution::ExecutionReceipt;

pub(super) fn document(bytes: &[u8]) -> Result<PictureBookDocument, StoreError> {
    if bytes.len() > 256 * 1024 {
        return Err(StoreError::InvalidCommit("book manifest exceeds 256 KiB"));
    }
    let document: PictureBookDocument = serde_json::from_slice(bytes)?;
    document.validate().map_err(StoreError::InvalidCommit)?;
    Ok(document)
}
pub(super) fn validate_commit(
    store: &ProjectStore,
    kind: ArtifactKind,
    bytes: &[u8],
    provenance: (&Transformation, &ExecutionReceipt),
    input_heads: &[(ArtifactId, RevisionId)],
) -> Result<(), StoreError> {
    if kind != ArtifactKind::PictureBook {
        return Ok(());
    }
    let (transformation, receipt) = provenance;
    let document = document(bytes)?;
    let composition = &document.composition;
    for (source, kind) in composition
        .languages
        .values()
        .copied()
        .map(|s| (s, ArtifactKind::TextDocument))
        .chain(
            composition
                .pages
                .iter()
                .filter_map(shape_domain::PictureBookPage::source)
                .map(|s| (s, ArtifactKind::ImageRaster)),
        )
    {
        let artifact = store
            .artifact(source.artifact_id)?
            .ok_or(StoreError::UnknownArtifact(source.artifact_id))?;
        let revision = store.revision(source.revision_id)?;
        if artifact.kind != kind
            || revision.artifact_id != source.artifact_id
            || (kind == ArtifactKind::ImageRaster && revision.content.media_type != "image/png")
        {
            return Err(StoreError::InvalidCommit(
                "book material has the wrong artifact kind or revision",
            ));
        }
    }
    let inputs = composition.inputs();
    let expected: Vec<_> = inputs
        .iter()
        .map(|i| (i.artifact_id, i.revision_id))
        .collect();
    let mut observed = input_heads.to_vec();
    observed.sort_unstable();
    let mut revisions: Vec<_> = inputs.iter().map(|i| i.revision_id).collect();
    revisions.sort_unstable();
    let mut declared = transformation.inputs.clone();
    declared.sort_unstable();
    if expected != observed
        || revisions != declared
        || transformation.kind != TransformationKind::Composite
        || transformation.operation.is_some()
        || receipt.capability.as_str() != "book.compose"
        || receipt.executor.id != "shape.builtin.book-composition"
    {
        return Err(StoreError::InvalidCommit(
            "book composition provenance does not match its pinned materials",
        ));
    }
    Ok(())
}
