//! Versioned AI-assistance state inside the product-visible Writing workspace.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::is_text_workspace_operator;

const LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.1";
const PREVIOUS_TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.2";
const TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260812.1";
const MAX_INSTRUCTION_BYTES: usize = 4_096;
#[cfg(test)]
const DEFAULT_TONE: &str = "neutral";
#[cfg(test)]
const DEFAULT_STYLE: &str = "natural";
const DEFAULT_VARIANT_COUNT: u8 = 1;

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
    Summarize,
}

impl TextTransformDraftMode {
    fn from_key(key: &str) -> Result<Self, String> {
        match key {
            "rewrite" => Ok(Self::Rewrite),
            "expand" => Ok(Self::Expand),
            "polish" => Ok(Self::Polish),
            "shorten" => Ok(Self::Shorten),
            "summarize" => Ok(Self::Summarize),
            _ => Err("text transform draft mode is unsupported".to_owned()),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Rewrite => "rewrite",
            Self::Expand => "expand",
            Self::Polish => "polish",
            Self::Shorten => "shorten",
            Self::Summarize => "summarize",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousTextTransformDraftConfiguration {
    mode: TextTransformDraftMode,
    instruction: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextTransformTone {
    Neutral,
    Warm,
    Confident,
    Playful,
    Serious,
}

impl TextTransformTone {
    fn from_key(key: &str) -> Result<Self, String> {
        match key {
            "neutral" => Ok(Self::Neutral),
            "warm" => Ok(Self::Warm),
            "confident" => Ok(Self::Confident),
            "playful" => Ok(Self::Playful),
            "serious" => Ok(Self::Serious),
            _ => Err("text transform tone is unsupported".to_owned()),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Warm => "warm",
            Self::Confident => "confident",
            Self::Playful => "playful",
            Self::Serious => "serious",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextTransformStyle {
    Natural,
    Concise,
    Professional,
    Literary,
    Casual,
}

impl TextTransformStyle {
    fn from_key(key: &str) -> Result<Self, String> {
        match key {
            "natural" => Ok(Self::Natural),
            "concise" => Ok(Self::Concise),
            "professional" => Ok(Self::Professional),
            "literary" => Ok(Self::Literary),
            "casual" => Ok(Self::Casual),
            _ => Err("text transform style is unsupported".to_owned()),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Concise => "concise",
            Self::Professional => "professional",
            Self::Literary => "literary",
            Self::Casual => "casual",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextTransformDraftConfiguration {
    mode: TextTransformDraftMode,
    instruction: String,
    tone: TextTransformTone,
    style: TextTransformStyle,
    variant_count: u8,
}

impl TextTransformDraftConfiguration {
    fn new(
        mode: TextTransformDraftMode,
        instruction: impl Into<String>,
        tone: TextTransformTone,
        style: TextTransformStyle,
        variant_count: u8,
    ) -> Result<Self, String> {
        let instruction = instruction.into();
        if instruction.len() > MAX_INSTRUCTION_BYTES {
            return Err("text transform instruction is too large".to_owned());
        }
        if !(1..=4).contains(&variant_count) {
            return Err("text transform variant count is unsupported".to_owned());
        }
        Ok(Self {
            mode,
            instruction,
            tone,
            style,
            variant_count,
        })
    }

    fn validate(&self) -> Result<(), String> {
        Self::new(
            self.mode,
            self.instruction.clone(),
            self.tone,
            self.style,
            self.variant_count,
        )
        .map(|_| ())
    }
}

#[cfg(test)]
pub(crate) fn configuration_for_instruction(
    instruction: &str,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    configuration_for_mode_and_instruction(TextTransformDraftMode::Rewrite.as_str(), instruction)
}

#[cfg(test)]
pub(crate) fn configuration_for_mode_and_instruction(
    mode_key: &str,
    instruction: &str,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    let mode = TextTransformDraftMode::from_key(mode_key)?;
    if instruction.trim().is_empty() {
        return Ok(None);
    }
    configuration_for_studio(
        mode.as_str(),
        instruction,
        DEFAULT_TONE,
        DEFAULT_STYLE,
        DEFAULT_VARIANT_COUNT,
    )
}

pub(crate) fn configuration_for_studio(
    mode_key: &str,
    instruction: &str,
    tone_key: &str,
    style_key: &str,
    variant_count: u8,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    let configuration = TextTransformDraftConfiguration::new(
        TextTransformDraftMode::from_key(mode_key)?,
        instruction,
        TextTransformTone::from_key(tone_key)?,
        TextTransformStyle::from_key(style_key)?,
        variant_count,
    )?;
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(TEXT_TRANSFORM_DRAFT_SCHEMA)
            .map_err(|error| error.to_string())?,
        serde_json::to_string(&configuration).map_err(|error| error.to_string())?,
    )
    .map(Some)
    .map_err(|error| error.to_string())
}

pub(crate) fn instruction_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    if !is_text_workspace_operator(draft.operator_type().as_str()) {
        return Ok(String::new());
    }
    let Some(configuration) = draft.configuration() else {
        return Ok(String::new());
    };
    decode(configuration).map(|configuration| configuration.instruction)
}

pub(crate) fn mode_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    if !is_text_workspace_operator(draft.operator_type().as_str()) {
        return Ok(String::new());
    }
    let Some(configuration) = draft.configuration() else {
        return Ok(TextTransformDraftMode::Rewrite.as_str().to_owned());
    };
    decode(configuration).map(|configuration| configuration.mode.as_str().to_owned())
}

pub(crate) fn tone_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    studio_configuration_from_draft(draft)
        .map(|configuration| configuration.tone.as_str().to_owned())
}

pub(crate) fn style_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    studio_configuration_from_draft(draft)
        .map(|configuration| configuration.style.as_str().to_owned())
}

pub(crate) fn variant_count_from_draft(draft: &WorkingOperatorDraft) -> Result<u8, String> {
    studio_configuration_from_draft(draft).map(|configuration| configuration.variant_count)
}

pub(crate) fn compiled_instruction_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<String, String> {
    let configuration = studio_configuration_from_draft(draft)?;
    let direction = if configuration.instruction.trim().is_empty() {
        "Preserve the source's factual meaning.".to_owned()
    } else {
        configuration.instruction
    };
    Ok(format!(
        "{direction}\nTone: {}. Style: {}.",
        configuration.tone.as_str(),
        configuration.style.as_str()
    ))
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
        PREVIOUS_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let previous: PreviousTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            TextTransformDraftConfiguration {
                mode: previous.mode,
                instruction: previous.instruction,
                tone: TextTransformTone::Neutral,
                style: TextTransformStyle::Natural,
                variant_count: DEFAULT_VARIANT_COUNT,
            }
        }
        LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let legacy: LegacyTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            TextTransformDraftConfiguration {
                mode: TextTransformDraftMode::Rewrite,
                instruction: legacy.instruction,
                tone: TextTransformTone::Neutral,
                style: TextTransformStyle::Natural,
                variant_count: DEFAULT_VARIANT_COUNT,
            }
        }
        _ => {
            return Err("text transform draft configuration schema is unsupported".to_owned());
        }
    };
    configuration.validate()?;
    Ok(configuration)
}

fn studio_configuration_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<TextTransformDraftConfiguration, String> {
    if !is_text_workspace_operator(draft.operator_type().as_str()) {
        return Ok(TextTransformDraftConfiguration {
            mode: TextTransformDraftMode::Rewrite,
            instruction: String::new(),
            tone: TextTransformTone::Neutral,
            style: TextTransformStyle::Natural,
            variant_count: DEFAULT_VARIANT_COUNT,
        });
    }
    draft.configuration().map_or_else(
        || {
            Ok(TextTransformDraftConfiguration {
                mode: TextTransformDraftMode::Rewrite,
                instruction: String::new(),
                tone: TextTransformTone::Neutral,
                style: TextTransformStyle::Natural,
                variant_count: DEFAULT_VARIANT_COUNT,
            })
        },
        decode,
    )
}

#[cfg(test)]
mod tests;
