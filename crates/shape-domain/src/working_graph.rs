//! Durable, mutable Operator entries that have not crossed acceptance.
//!
//! The current desktop adapts one accepted Artifact as one compatibility
//! Scene. This contract preserves its unexecuted Operator entries across
//! sessions without placing them in immutable Artifact or Scene history.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ArtifactId, DomainError, OperatorDataTypeId, OperatorNodeId, OperatorTypeId, RevisionId,
};

const MAX_WORKING_OPERATORS: usize = 128;

/// One configured Operator that has not produced an accepted revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingOperatorDraft {
    id: OperatorNodeId,
    operator_type: OperatorTypeId,
    input_data_type: OperatorDataTypeId,
    output_data_type: OperatorDataTypeId,
}

impl WorkingOperatorDraft {
    /// Creates a new durable draft identity around one typed Operator contract.
    ///
    /// # Panics
    ///
    /// Panics only if the fixed `draft.` prefix plus a generated UUID is no
    /// longer accepted by the Operator node identity contract.
    #[must_use]
    pub fn new(
        operator_type: OperatorTypeId,
        input_data_type: OperatorDataTypeId,
        output_data_type: OperatorDataTypeId,
    ) -> Self {
        Self {
            id: OperatorNodeId::new(format!("draft.{}", Uuid::now_v7().simple()))
                .expect("UUID-backed working Operator identity is portable"),
            operator_type,
            input_data_type,
            output_data_type,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &OperatorNodeId {
        &self.id
    }

    #[must_use]
    pub const fn operator_type(&self) -> &OperatorTypeId {
        &self.operator_type
    }

    #[must_use]
    pub const fn input_data_type(&self) -> &OperatorDataTypeId {
        &self.input_data_type
    }

    #[must_use]
    pub const fn output_data_type(&self) -> &OperatorDataTypeId {
        &self.output_data_type
    }

    fn validate(&self) -> Result<(), DomainError> {
        OperatorNodeId::new(self.id.as_str())?;
        OperatorTypeId::new(self.operator_type.as_str())?;
        OperatorDataTypeId::new(self.input_data_type.as_str())?;
        OperatorDataTypeId::new(self.output_data_type.as_str())?;
        Ok(())
    }
}

/// Project-backed mutable state for one compatibility Scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactWorkingGraph {
    context_artifact_id: ArtifactId,
    expected_revision_id: RevisionId,
    operators: Vec<WorkingOperatorDraft>,
}

impl ArtifactWorkingGraph {
    #[must_use]
    pub const fn new(context_artifact_id: ArtifactId, expected_revision_id: RevisionId) -> Self {
        Self {
            context_artifact_id,
            expected_revision_id,
            operators: Vec::new(),
        }
    }

    #[must_use]
    pub const fn context_artifact_id(&self) -> ArtifactId {
        self.context_artifact_id
    }

    #[must_use]
    pub const fn expected_revision_id(&self) -> RevisionId {
        self.expected_revision_id
    }

    #[must_use]
    pub fn operators(&self) -> &[WorkingOperatorDraft] {
        &self.operators
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operators.is_empty()
    }

    /// Adds or reuses one Operator type in the current single-route editor.
    ///
    /// Repeated same-type nodes become legal once execution requests address a
    /// draft identity rather than only an Artifact and Operator type.
    ///
    /// # Errors
    ///
    /// Returns an error if the bounded Working Graph is already full.
    pub fn add_operator(
        &mut self,
        operator_type: OperatorTypeId,
        input_data_type: OperatorDataTypeId,
        output_data_type: OperatorDataTypeId,
    ) -> Result<WorkingOperatorDraft, DomainError> {
        if let Some(existing) = self
            .operators
            .iter()
            .find(|operator| operator.operator_type == operator_type)
        {
            return Ok(existing.clone());
        }
        if self.operators.len() >= MAX_WORKING_OPERATORS {
            return Err(DomainError::CollectionTooLarge {
                collection: "working graph operators",
                actual: self.operators.len() + 1,
                maximum: MAX_WORKING_OPERATORS,
            });
        }
        let draft = WorkingOperatorDraft::new(operator_type, input_data_type, output_data_type);
        self.operators.push(draft.clone());
        Ok(draft)
    }

    /// Removes one exact draft identity.
    pub fn remove_operator(&mut self, draft_id: &OperatorNodeId) -> bool {
        let before = self.operators.len();
        self.operators.retain(|operator| operator.id != *draft_id);
        self.operators.len() != before
    }

    /// Removes the current single-route draft for one Operator type.
    pub fn remove_operator_type(&mut self, operator_type: &str) -> bool {
        let before = self.operators.len();
        self.operators
            .retain(|operator| operator.operator_type.as_str() != operator_type);
        self.operators.len() != before
    }

    /// Revalidates JSON-loaded identifiers and collection invariants.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed identifiers, duplicate draft identities,
    /// or an oversized deserialized collection.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.operators.len() > MAX_WORKING_OPERATORS {
            return Err(DomainError::CollectionTooLarge {
                collection: "working graph operators",
                actual: self.operators.len(),
                maximum: MAX_WORKING_OPERATORS,
            });
        }
        for operator in &self.operators {
            operator.validate()?;
        }
        let identities = self
            .operators
            .iter()
            .map(WorkingOperatorDraft::id)
            .collect::<HashSet<_>>();
        if identities.len() != self.operators.len() {
            return Err(DomainError::DuplicateOperatorNode);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
