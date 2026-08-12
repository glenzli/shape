//! Versioned AI-assistance state inside the product-visible Writing workspace.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::is_text_workspace_operator;

const LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.1";
const PREVIOUS_TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260811.2";
const PREVIOUS_STUDIO_TEXT_TRANSFORM_DRAFT_SCHEMA: &str =
    "shape.operator-draft.text-transform@20260812.1";
const PREVIOUS_EXPRESSION_TEXT_TRANSFORM_DRAFT_SCHEMA: &str =
    "shape.operator-draft.text-transform@20260812.2";
const TEXT_TRANSFORM_DRAFT_SCHEMA: &str = "shape.operator-draft.text-transform@20260813.1";
const MAX_INSTRUCTION_BYTES: usize = 4_096;
const MAX_EXPRESSION_JSON_BYTES: usize = 4_096;
const MAX_TONE_FACETS: usize = 2;
const MAX_CUSTOM_NAME_BYTES: usize = 64;
const MAX_CUSTOM_INSTRUCTION_BYTES: usize = 512;
const MAX_CUSTOM_EXAMPLE_BYTES: usize = 256;
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
enum PreviousTextTransformTone {
    Neutral,
    Warm,
    Confident,
    Playful,
    Serious,
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
struct PreviousStudioTextTransformDraftConfiguration {
    mode: TextTransformDraftMode,
    instruction: String,
    tone: PreviousTextTransformTone,
    style: TextTransformStyle,
    variant_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextTonePreset {
    Neutral,
    Warm,
    Empathetic,
    Confident,
    Playful,
    Restrained,
    Serious,
    Urgent,
}

impl TextTonePreset {
    fn from_key(key: &str) -> Result<Self, String> {
        match key {
            "neutral" => Ok(Self::Neutral),
            "warm" => Ok(Self::Warm),
            "empathetic" => Ok(Self::Empathetic),
            "confident" => Ok(Self::Confident),
            "playful" => Ok(Self::Playful),
            "restrained" => Ok(Self::Restrained),
            "serious" => Ok(Self::Serious),
            "urgent" => Ok(Self::Urgent),
            _ => Err("text transform tone preset is unsupported".to_owned()),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Warm => "warm",
            Self::Empathetic => "empathetic",
            Self::Confident => "confident",
            Self::Playful => "playful",
            Self::Restrained => "restrained",
            Self::Serious => "serious",
            Self::Urgent => "urgent",
        }
    }
}

impl From<PreviousTextTransformTone> for TextTonePreset {
    fn from(value: PreviousTextTransformTone) -> Self {
        match value {
            PreviousTextTransformTone::Neutral => Self::Neutral,
            PreviousTextTransformTone::Warm => Self::Warm,
            PreviousTextTransformTone::Confident => Self::Confident,
            PreviousTextTransformTone::Playful => Self::Playful,
            PreviousTextTransformTone::Serious => Self::Serious,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextToneVisual {
    Ripple,
    Glow,
    Embrace,
    Ascent,
    Spark,
    Frame,
    Pillar,
    Pulse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TextToneFacet {
    Preset {
        preset: TextTonePreset,
    },
    Custom {
        name: String,
        instruction: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        example: Option<String>,
        visual: TextToneVisual,
    },
}

impl TextToneFacet {
    fn validate(&self, custom_example_required: bool) -> Result<(), String> {
        match self {
            Self::Preset { .. } => Ok(()),
            Self::Custom {
                name,
                instruction,
                example,
                ..
            } => {
                validate_authored_text(name, MAX_CUSTOM_NAME_BYTES, "custom tone name is invalid")?;
                validate_authored_text(
                    instruction,
                    MAX_CUSTOM_INSTRUCTION_BYTES,
                    "custom tone instruction is invalid",
                )?;
                match example {
                    Some(example) => validate_authored_text(
                        example,
                        MAX_CUSTOM_EXAMPLE_BYTES,
                        "custom tone example is invalid",
                    ),
                    None if custom_example_required => {
                        Err("custom tone example is required".to_owned())
                    }
                    None => Ok(()),
                }
            }
        }
    }

    fn prompt(&self) -> String {
        match self {
            Self::Preset { preset } => preset.as_str().to_owned(),
            Self::Custom {
                name,
                instruction,
                example,
                ..
            } => example.as_ref().map_or_else(
                || format!("custom tone ‘{name}’ ({instruction})"),
                |example| {
                    format!("custom tone ‘{name}’ (direction: {instruction} Example: “{example}”)")
                },
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextToneIntensity {
    Subtle,
    Balanced,
    Strong,
}

impl TextToneIntensity {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Subtle => "subtle",
            Self::Balanced => "balanced",
            Self::Strong => "strong",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TextAudiencePreset {
    General,
    CloseFriend,
    Colleague,
    Customer,
    Public,
    Expert,
    Beginner,
}

impl TextAudiencePreset {
    const fn as_str(self) -> &'static str {
        match self {
            Self::General => "general audience",
            Self::CloseFriend => "a close friend",
            Self::Colleague => "a colleague",
            Self::Customer => "a customer",
            Self::Public => "the public",
            Self::Expert => "an expert audience",
            Self::Beginner => "a beginner audience",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TextAudience {
    Preset { preset: TextAudiencePreset },
    Custom { name: String, instruction: String },
}

impl TextAudience {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Preset { .. } => Ok(()),
            Self::Custom { name, instruction } => {
                validate_authored_text(
                    name,
                    MAX_CUSTOM_NAME_BYTES,
                    "custom audience name is invalid",
                )?;
                validate_authored_text(
                    instruction,
                    MAX_CUSTOM_INSTRUCTION_BYTES,
                    "custom audience instruction is invalid",
                )
            }
        }
    }

    fn prompt(&self) -> String {
        match self {
            Self::Preset { preset } => preset.as_str().to_owned(),
            Self::Custom { name, instruction } => {
                format!("custom audience ‘{name}’ ({instruction})")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextExpression {
    tones: Vec<TextToneFacet>,
    intensity: TextToneIntensity,
    audience: TextAudience,
}

impl Default for TextExpression {
    fn default() -> Self {
        Self {
            tones: vec![TextToneFacet::Preset {
                preset: TextTonePreset::Neutral,
            }],
            intensity: TextToneIntensity::Balanced,
            audience: TextAudience::Preset {
                preset: TextAudiencePreset::General,
            },
        }
    }
}

impl TextExpression {
    fn validate(&self, custom_examples_required: bool) -> Result<(), String> {
        if self.tones.is_empty() || self.tones.len() > MAX_TONE_FACETS {
            return Err("text expression tone count is unsupported".to_owned());
        }
        let mut preset_keys = Vec::new();
        let mut custom_count = 0;
        for tone in &self.tones {
            tone.validate(custom_examples_required)?;
            match tone {
                TextToneFacet::Preset { preset } => {
                    if preset_keys.contains(preset) {
                        return Err("text expression contains a duplicate tone".to_owned());
                    }
                    preset_keys.push(*preset);
                }
                TextToneFacet::Custom { .. } => custom_count += 1,
            }
        }
        if custom_count > 1 {
            return Err("text expression contains too many custom tones".to_owned());
        }
        if preset_keys.contains(&TextTonePreset::Neutral) && self.tones.len() > 1 {
            return Err("neutral tone cannot be combined with another tone".to_owned());
        }
        self.audience.validate()
    }

    fn primary_tone_key(&self) -> &'static str {
        self.tones.first().map_or("neutral", |tone| match tone {
            TextToneFacet::Preset { preset } => preset.as_str(),
            TextToneFacet::Custom { .. } => "custom",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextTransformDraftConfiguration {
    mode: TextTransformDraftMode,
    instruction: String,
    expression: TextExpression,
    style: TextTransformStyle,
    variant_count: u8,
}

impl TextTransformDraftConfiguration {
    fn new(
        mode: TextTransformDraftMode,
        instruction: impl Into<String>,
        expression: TextExpression,
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
        expression.validate(false)?;
        Ok(Self {
            mode,
            instruction,
            expression,
            style,
            variant_count,
        })
    }

    fn validate(&self, custom_examples_required: bool) -> Result<(), String> {
        if self.instruction.len() > MAX_INSTRUCTION_BYTES {
            return Err("text transform instruction is too large".to_owned());
        }
        if !(1..=4).contains(&self.variant_count) {
            return Err("text transform variant count is unsupported".to_owned());
        }
        self.expression.validate(custom_examples_required)
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
    let expression = TextExpression {
        tones: vec![TextToneFacet::Preset {
            preset: TextTonePreset::from_key(tone_key)?,
        }],
        ..TextExpression::default()
    };
    configuration_for_expression_studio(
        mode_key,
        instruction,
        &serde_json::to_string(&expression).map_err(|error| error.to_string())?,
        style_key,
        variant_count,
    )
}

pub(crate) fn configuration_for_expression_studio(
    mode_key: &str,
    instruction: &str,
    expression_json: &str,
    style_key: &str,
    variant_count: u8,
) -> Result<Option<WorkingOperatorConfiguration>, String> {
    if expression_json.len() > MAX_EXPRESSION_JSON_BYTES {
        return Err("text expression is too large".to_owned());
    }
    let expression: TextExpression = serde_json::from_str(expression_json)
        .map_err(|_| "text expression does not match its exact schema".to_owned())?;
    let configuration = TextTransformDraftConfiguration::new(
        TextTransformDraftMode::from_key(mode_key)?,
        instruction,
        expression,
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
        .map(|configuration| configuration.expression.primary_tone_key().to_owned())
}

pub(crate) fn expression_json_from_draft(draft: &WorkingOperatorDraft) -> Result<String, String> {
    let expression = studio_configuration_from_draft(draft)?.expression;
    serde_json::to_string(&expression).map_err(|error| error.to_string())
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
    let tone = if configuration.expression.tones.len() == 1 {
        configuration.expression.tones[0].prompt()
    } else {
        format!(
            "primarily {}, with {} as a supporting tone",
            configuration.expression.tones[0].prompt(),
            configuration.expression.tones[1].prompt()
        )
    };
    Ok(format!(
        "{direction}\nTone: {} at {} intensity. Audience: {}. Style: {}.",
        tone,
        configuration.expression.intensity.as_str(),
        configuration.expression.audience.prompt(),
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
    let (configuration, custom_examples_required) = match configuration.schema().as_str() {
        TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let current = serde_json::from_str(configuration.json()).map_err(|_| {
                "text transform draft configuration does not match its exact schema".to_owned()
            })?;
            (current, false)
        }
        PREVIOUS_EXPRESSION_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let previous = serde_json::from_str(configuration.json()).map_err(|_| {
                "text transform draft configuration does not match its exact schema".to_owned()
            })?;
            (previous, false)
        }
        PREVIOUS_STUDIO_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let previous: PreviousStudioTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            (
                TextTransformDraftConfiguration {
                    mode: previous.mode,
                    instruction: previous.instruction,
                    expression: TextExpression {
                        tones: vec![TextToneFacet::Preset {
                            preset: previous.tone.into(),
                        }],
                        ..TextExpression::default()
                    },
                    style: previous.style,
                    variant_count: previous.variant_count,
                },
                false,
            )
        }
        PREVIOUS_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let previous: PreviousTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            (
                TextTransformDraftConfiguration {
                    mode: previous.mode,
                    instruction: previous.instruction,
                    expression: TextExpression::default(),
                    style: TextTransformStyle::Natural,
                    variant_count: DEFAULT_VARIANT_COUNT,
                },
                false,
            )
        }
        LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA => {
            let legacy: LegacyTextTransformDraftConfiguration =
                serde_json::from_str(configuration.json()).map_err(|_| {
                    "text transform draft configuration does not match its exact schema".to_owned()
                })?;
            (
                TextTransformDraftConfiguration {
                    mode: TextTransformDraftMode::Rewrite,
                    instruction: legacy.instruction,
                    expression: TextExpression::default(),
                    style: TextTransformStyle::Natural,
                    variant_count: DEFAULT_VARIANT_COUNT,
                },
                false,
            )
        }
        _ => {
            return Err("text transform draft configuration schema is unsupported".to_owned());
        }
    };
    configuration.validate(custom_examples_required)?;
    Ok(configuration)
}

fn studio_configuration_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<TextTransformDraftConfiguration, String> {
    if !is_text_workspace_operator(draft.operator_type().as_str()) {
        return Ok(TextTransformDraftConfiguration {
            mode: TextTransformDraftMode::Rewrite,
            instruction: String::new(),
            expression: TextExpression::default(),
            style: TextTransformStyle::Natural,
            variant_count: DEFAULT_VARIANT_COUNT,
        });
    }
    draft.configuration().map_or_else(
        || {
            Ok(TextTransformDraftConfiguration {
                mode: TextTransformDraftMode::Rewrite,
                instruction: String::new(),
                expression: TextExpression::default(),
                style: TextTransformStyle::Natural,
                variant_count: DEFAULT_VARIANT_COUNT,
            })
        },
        decode,
    )
}

fn validate_authored_text(value: &str, max_bytes: usize, error: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.len() > max_bytes
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(error.to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
