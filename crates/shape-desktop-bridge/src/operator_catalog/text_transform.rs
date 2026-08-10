//! Versioned authored-state contract for the real `text.transform` consumer.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::TEXT_TRANSFORM_OPERATOR;

const TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.1";
const MAX_INSTRUCTION_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextTransformDraftConfiguration {
    instruction: String,
}

impl TextTransformDraftConfiguration {
    fn new(instruction: impl Into<String>) -> Result<Self, String> {
        let instruction = instruction.into();
        if instruction.trim().is_empty() || instruction.len() > MAX_INSTRUCTION_BYTES {
            return Err("text transform instruction is empty or too large".to_owned());
        }
        Ok(Self { instruction })
    }

    fn validate(&self) -> Result<(), String> {
        Self::new(self.instruction.clone()).map(|_| ())
    }
}

pub(crate) fn configuration_for_instruction(
    instruction: &str,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    if instruction.trim().is_empty() {
        return Ok(None);
    }
    let configuration = TextTransformDraftConfiguration::new(instruction)?;
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(TEXT_TRANSFORM_DRAFT_SCHEMA)
            .map_err(|error| error.to_string())?,
        serde_json::to_string(&configuration).map_err(|error| error.to_string())?,
    )
    .map(Some)
    .map_err(|error| error.to_string())
}

pub(crate) fn instruction_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    if draft.operator_type().as_str() != TEXT_TRANSFORM_OPERATOR {
        return Ok(String::new());
    }
    let Some(configuration) = draft.configuration() else {
        return Ok(String::new());
    };
    decode(configuration).map(|configuration| configuration.instruction)
}

pub(super) fn validate_text_transform_configuration(
    configuration: Option<&WorkingOperatorConfiguration>,
) -> Result<(), String> {
    configuration.map_or(Ok(()), |configuration| decode(configuration).map(|_| ()))
}

fn decode(
    configuration: &WorkingOperatorConfiguration,
) -> Result<TextTransformDraftConfiguration, String> {
    if configuration.schema().as_str() != TEXT_TRANSFORM_DRAFT_SCHEMA {
        return Err("text transform draft configuration schema is unsupported".to_owned());
    }
    let configuration: TextTransformDraftConfiguration = serde_json::from_str(configuration.json())
        .map_err(|_| {
            "text transform draft configuration does not match its exact schema".to_owned()
        })?;
    configuration.validate()?;
    Ok(configuration)
}

#[cfg(test)]
mod tests;
