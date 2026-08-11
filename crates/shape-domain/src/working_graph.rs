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
const MAX_CONFIGURATION_SCHEMA_BYTES: usize = 160;
const MAX_CONFIGURATION_JSON_BYTES: usize = 64 * 1_024;

/// Versioned, language-neutral identity of an Operator-owned draft contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperatorConfigurationSchemaId(String);

impl OperatorConfigurationSchemaId {
    /// Creates one bounded namespaced configuration schema identity.
    ///
    /// # Errors
    ///
    /// Rejects empty, oversized, non-ASCII, or unnamespaced values.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CONFIGURATION_SCHEMA_BYTES
            || !value.is_ascii()
            || !value.contains('.')
        {
            return Err(DomainError::InvalidOperatorIdentifier {
                field: "operator configuration schema",
                max_bytes: MAX_CONFIGURATION_SCHEMA_BYTES,
            });
        }
        Ok(Self(value))
    }

    /// Returns the exact stable schema identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Bounded exact JSON owned and validated by one Operator family.
///
/// The generic Working Graph preserves this envelope without interpreting
/// media semantics. A real Operator adapter must validate its exact schema
/// before presentation or execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingOperatorConfiguration {
    schema: OperatorConfigurationSchemaId,
    json: String,
}

impl WorkingOperatorConfiguration {
    /// Creates a versioned configuration envelope around one JSON object.
    ///
    /// # Errors
    ///
    /// Rejects malformed, non-object, or oversized JSON.
    pub fn new(
        schema: OperatorConfigurationSchemaId,
        json: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let configuration = Self {
            schema,
            json: json.into(),
        };
        configuration.validate()?;
        Ok(configuration)
    }

    #[must_use]
    pub const fn schema(&self) -> &OperatorConfigurationSchemaId {
        &self.schema
    }

    /// Returns exact authored JSON for validation by its Operator owner.
    #[must_use]
    pub fn json(&self) -> &str {
        &self.json
    }

    fn validate(&self) -> Result<(), DomainError> {
        OperatorConfigurationSchemaId::new(self.schema.as_str())?;
        let value = serde_json::from_str::<serde_json::Value>(&self.json).map_err(|_| {
            DomainError::InvalidWorkingOperatorConfiguration {
                max_bytes: MAX_CONFIGURATION_JSON_BYTES,
            }
        })?;
        if self.json.len() > MAX_CONFIGURATION_JSON_BYTES || !value.is_object() {
            return Err(DomainError::InvalidWorkingOperatorConfiguration {
                max_bytes: MAX_CONFIGURATION_JSON_BYTES,
            });
        }
        Ok(())
    }
}

/// One configured Operator that has not produced an accepted revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingOperatorDraft {
    id: OperatorNodeId,
    operator_type: OperatorTypeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input_data_type: Option<OperatorDataTypeId>,
    output_data_type: OperatorDataTypeId,
    #[serde(default)]
    configuration: Option<WorkingOperatorConfiguration>,
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
            input_data_type: Some(input_data_type),
            output_data_type,
            configuration: None,
        }
    }

    /// Creates a zero-input Source Operator draft.
    ///
    /// # Panics
    ///
    /// Panics only if the generated UUID-backed node identity stops satisfying
    /// the portable Operator identifier contract.
    #[must_use]
    pub fn new_source(operator_type: OperatorTypeId, output_data_type: OperatorDataTypeId) -> Self {
        Self {
            id: OperatorNodeId::new(format!("draft.{}", Uuid::now_v7().simple()))
                .expect("UUID-backed working Operator identity is portable"),
            operator_type,
            input_data_type: None,
            output_data_type,
            configuration: None,
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
    pub const fn input_data_type(&self) -> Option<&OperatorDataTypeId> {
        self.input_data_type.as_ref()
    }

    #[must_use]
    pub const fn output_data_type(&self) -> &OperatorDataTypeId {
        &self.output_data_type
    }

    #[must_use]
    pub const fn configuration(&self) -> Option<&WorkingOperatorConfiguration> {
        self.configuration.as_ref()
    }

    fn validate(&self) -> Result<(), DomainError> {
        OperatorNodeId::new(self.id.as_str())?;
        OperatorTypeId::new(self.operator_type.as_str())?;
        if let Some(input_data_type) = &self.input_data_type {
            OperatorDataTypeId::new(input_data_type.as_str())?;
        }
        OperatorDataTypeId::new(self.output_data_type.as_str())?;
        if let Some(configuration) = &self.configuration {
            configuration.validate()?;
        }
        Ok(())
    }
}

/// Project-backed mutable state for one compatibility Scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactWorkingGraph {
    context_artifact_id: ArtifactId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_revision_id: Option<RevisionId>,
    operators: Vec<WorkingOperatorDraft>,
}

impl ArtifactWorkingGraph {
    #[must_use]
    pub const fn new(context_artifact_id: ArtifactId, expected_revision_id: RevisionId) -> Self {
        Self {
            context_artifact_id,
            expected_revision_id: Some(expected_revision_id),
            operators: Vec::new(),
        }
    }

    /// Creates mutable state for one Source Operator whose target Artifact has
    /// no accepted head yet.
    #[must_use]
    pub const fn new_source(context_artifact_id: ArtifactId) -> Self {
        Self {
            context_artifact_id,
            expected_revision_id: None,
            operators: Vec::new(),
        }
    }

    #[must_use]
    pub const fn context_artifact_id(&self) -> ArtifactId {
        self.context_artifact_id
    }

    #[must_use]
    pub const fn expected_revision_id(&self) -> Option<RevisionId> {
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
        if self.expected_revision_id.is_none() {
            return Err(DomainError::InvalidWorkingGraphAnchor);
        }
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

    /// Adds or reuses one zero-input Source Operator in an unaccepted target.
    ///
    /// # Errors
    ///
    /// Returns an error if this graph is anchored to an accepted Revision or
    /// its bounded draft collection is full.
    pub fn add_source_operator(
        &mut self,
        operator_type: OperatorTypeId,
        output_data_type: OperatorDataTypeId,
    ) -> Result<WorkingOperatorDraft, DomainError> {
        if self.expected_revision_id.is_some() {
            return Err(DomainError::InvalidWorkingGraphAnchor);
        }
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
        let draft = WorkingOperatorDraft::new_source(operator_type, output_data_type);
        self.operators.push(draft.clone());
        Ok(draft)
    }

    /// Removes one exact draft identity.
    pub fn remove_operator(&mut self, draft_id: &OperatorNodeId) -> bool {
        let before = self.operators.len();
        self.operators.retain(|operator| operator.id != *draft_id);
        self.operators.len() != before
    }

    /// Replaces or clears one exact draft's Operator-owned configuration.
    pub fn set_operator_configuration(
        &mut self,
        draft_id: &OperatorNodeId,
        configuration: Option<WorkingOperatorConfiguration>,
    ) -> bool {
        let Some(draft) = self
            .operators
            .iter_mut()
            .find(|operator| operator.id == *draft_id)
        else {
            return false;
        };
        draft.configuration = configuration;
        true
    }

    /// Advances an accepted-input Working Graph to a newly locked head while
    /// preserving reusable Operator intent and configuration.
    ///
    /// # Errors
    ///
    /// Rejects zero-input Source graphs, whose output acceptance completes the
    /// source lifecycle instead of rebasing an input revision.
    pub fn rebase_accepted_input(
        &mut self,
        expected_revision_id: RevisionId,
    ) -> Result<(), DomainError> {
        if self.expected_revision_id.is_none() {
            return Err(DomainError::InvalidWorkingGraphAnchor);
        }
        self.expected_revision_id = Some(expected_revision_id);
        self.validate()
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
            if operator.input_data_type.is_some() != self.expected_revision_id.is_some() {
                return Err(DomainError::InvalidWorkingGraphAnchor);
            }
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
