//! Versioned authored-state contract for the real `text.transform` consumer.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::TEXT_TRANSFORM_OPERATOR;

const LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.1";
const TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.2";
const MAX_INSTRUCTION_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTextTransformDraftConfiguration {
    instruction: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextTransformDraftMode {
    Rewrite,
    Expand,
    Polish,
    Shorten,
}

impl TextTransformDraftMode {
    fn from_key(key: &str) -> Result<Self, String> {
        match key {
            "rewrite" => Ok(Self::Rewrite),
            "expand" => Ok(Self::Expand),
            "polish" => Ok(Self::Polish),
            "shorten" => Ok(Self::Shorten),
            _ => Err("text transform draft mode is unsupported".to_owned()),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Rewrite => "rewrite",
            Self::Expand => "expand",
            Self::Polish => "polish",
            Self::Shorten => "shorten",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextTransformDraftConfiguration {
    mode: TextTransformDraftMode,
    instruction: String,
}

impl TextTransformDraftConfiguration {
    fn new(mode: TextTransformDraftMode, instruction: impl Into<String>) -> Result<Self, String> {
        let instruction = instruction.into();
        if instruction.trim().is_empty() || instruction.len() > MAX_INSTRUCTION_BYTES {
            return Err("text transform instruction is empty or too large".to_owned());
        }
        Ok(Self { mode, instruction })
    }

    fn validate(&self) -> Result<(), String> {
        Self::new(self.mode, self.instruction.clone()).map(|_| ())
    }
}

#[cfg(test)]
pub(crate) fn configuration_for_instruction(
    instruction: &str,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    configuration_for_mode_and_instruction(TextTransformDraftMode::Rewrite.as_str(), instruction)
}

pub(crate) fn configuration_for_mode_and_instruction(
    mode_key: &str,
    instruction: &str,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    let mode = TextTransformDraftMode::from_key(mode_key)?;
    if instruction.trim().is_empty() {
        return Ok(None);
    }
    let configuration = TextTransformDraftConfiguration::new(mode, instruction)?;
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

pub(crate) fn mode_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    if draft.operator_type().as_str() != TEXT_TRANSFORM_OPERATOR {
        return Ok(String::new());
    }
    let Some(configuration) = draft.configuration() else {
        return Ok(TextTransformDraftMode::Rewrite.as_str().to_owned());
    };
    decode(configuration).map(|configuration| configuration.mode.as_str().to_owned())
}

pub(super) fn validate_text_transform_configuration(
    configuration: Option<&WorkingOperatorConfiguration>,
) -> Result<(), String> {
    configuration.map_or(Ok(()), |configuration| decode(configuration).map(|_| ()))
}

fn decode(
    configuration: &WorkingOperatorConfiguration,
) -> Result<TextTransformDraftConfiguration, String> {
    let configuration = match configuration.schema().as_str() {
        TEXT_TRANSFORM_DRAFT_SCHEMA => {
            serde_json::from_str(configuration.json()).map_err(|_| {
                "text transform draft configuration does not match its exact schema".to_owned()
            })?
        }
        LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let legacy: LegacyTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            TextTransformDraftConfiguration {
                mode: TextTransformDraftMode::Rewrite,
                instruction: legacy.instruction,
            }
        }
        _ => {
            return Err("text transform draft configuration schema is unsupported".to_owned());
        }
    };
    configuration.validate()?;
    Ok(configuration)
}

#[cfg(test)]
mod tests;
