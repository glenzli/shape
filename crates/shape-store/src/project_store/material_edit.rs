//! Store-side defense of editor return lineage, independent of the desktop/core caller.

use super::{AcceptedCommit, ProjectStore};
use crate::StoreError;
use shape_domain::{
    ArtifactContentContract, ArtifactId, MaterialEditConfiguration, MaterialReturnNormalization,
    RevisionId, TransformationKind, TransformationOperation, WorkingOperatorDraft,
};
use shape_execution::{
    MATERIAL_RETURN_EXECUTOR, MATERIAL_RETURN_REVISION, normalize_material_return,
};

impl ProjectStore {
    pub(super) fn validate_material_edit_commit(
        &self,
        commit: &AcceptedCommit,
        inputs: &[(ArtifactId, RevisionId)],
        node: Option<&WorkingOperatorDraft>,
    ) -> Result<(), StoreError> {
        let Some(TransformationOperation::MaterialEdit(operation)) =
            &commit.transformation.operation
        else {
            return Ok(());
        };
        let invalid = || {
            StoreError::InvalidCommit(
                "material return must preserve exact source, stage and media lineage",
            )
        };
        operation.validate()?;
        let node = node.ok_or_else(invalid)?;
        let config = MaterialEditConfiguration::from_draft(node)?;
        let source = self.revision(operation.source.revision_id)?;
        let source_artifact = self
            .artifact(operation.source.artifact_id)?
            .ok_or_else(invalid)?;
        let source_contract = source.content_contract.as_ref().ok_or_else(invalid)?;
        let output_contract = commit.content_contract.as_ref().ok_or_else(invalid)?;
        if commit.transformation.kind != TransformationKind::ExternalRoundTrip
            || commit.transformation.inputs != [operation.source.revision_id]
            || inputs != [(operation.source.artifact_id, operation.source.revision_id)]
            || operation.source.artifact_id == commit.artifact_id
            || source.artifact_id != operation.source.artifact_id
            || source_artifact.kind != operation.editor.artifact_kind()
            || source.content.digest != operation.input_digest
            || node.id() != &operation.stage_id
            || node.input() != Some(operation.source)
            || config.editor != operation.editor
            || config.session != operation.session
            || commit.output_media_type != operation.editor.media_type()
            || commit.receipt.capability.as_str() != operation.editor.operator_type()
            || commit.receipt.executor.id != MATERIAL_RETURN_EXECUTOR
            || commit.receipt.executor.contract_revision != MATERIAL_RETURN_REVISION
            || commit.receipt.executor_job_id.is_some()
            || commit.receipt.external_provenance.is_some()
        {
            return Err(invalid());
        }
        if let (
            ArtifactContentContract::AudioClip(source),
            ArtifactContentContract::AudioClip(output),
        ) = (source_contract, output_contract)
            && source.origin != output.origin
        {
            return Err(invalid());
        }
        if matches!(
            operation.normalization,
            MaterialReturnNormalization::KnownSrgbIccInput { .. }
        ) {
            let source_bytes = self.read_content(&source.content)?;
            let (_, expected) =
                shape_execution::RasterExecutor::materialize_shadow_input(&source_bytes)
                    .map_err(|_| invalid())?;
            if operation.normalization != expected {
                return Err(invalid());
            }
        }
        let verified = normalize_material_return(source_contract, &commit.output_bytes)
            .map_err(|_| invalid())?;
        if verified.content_contract.as_ref() != Some(output_contract)
            || verified.bytes.as_slice() != commit.output_bytes.as_ref()
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
