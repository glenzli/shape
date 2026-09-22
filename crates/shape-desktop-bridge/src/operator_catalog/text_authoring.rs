//! Empty-first writing profiles, durable authoring buffers and prompt compilation.
//! The script grammar remains owned by shape-domain and the bundled writing guide.

use serde::{Deserialize, Serialize};
use shape_domain::{
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
    speech_script::parse_speech_script,
};

pub(crate) const SCHEMA: &str = "shape.operator-draft.text-authoring@20260922.1";
const MAX_BUFFER_BYTES: usize = 48 * 1024;
const MAX_INSTRUCTION_BYTES: usize = 4096;
const WRITING_GUIDE: &str = include_str!("../../../../docs/SPEECH_SCRIPT_PROMPT.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WritingProfile {
    Plain,
    Script,
    // Read older drafts that stored a writing example as the format.
    Listening,
    Narration,
    Dialogue,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WritingExample {
    #[default]
    General,
    Listening,
    Dialogue,
}

impl WritingExample {
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde skip_serializing_if takes a reference"
    )]
    fn is_general(&self) -> bool {
        *self == Self::General
    }

    fn legacy_requirement(self) -> Option<&'static str> {
        match self {
            Self::General => None,
            Self::Listening => Some("Create a listening exercise."),
            Self::Dialogue => Some("Write a dialogue."),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WritingEntry {
    Generate,
    Adapt,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextAuthoring {
    pub profile: WritingProfile,
    // Decode old writing presets, then migrate them into explicit user requirements.
    #[serde(default, skip_serializing_if = "WritingExample::is_general")]
    pub example: WritingExample,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub expression: super::text_transform::TextExpression,
    #[serde(default = "default_style")]
    pub style: String,
    pub entry: WritingEntry,
    pub instruction: String,
    #[serde(default)]
    pub repair_feedback: String,
    pub material: String,
    pub text: String,
    // Retained for reading existing drafts; playback instructions live in the script source.
    #[serde(default = "default_repeat", skip_serializing)]
    pub repeat_count: u8,
    #[serde(default = "default_gap", skip_serializing)]
    pub gap_seconds: u8,
    #[serde(default, skip_serializing)]
    pub delivery: shape_domain::speech_script::SpeechDelivery,
    #[serde(default, skip_serializing)]
    pub cast: String,
    #[serde(default, skip_serializing)]
    pub answer_beep: bool,
    #[serde(default = "default_pause", skip_serializing)]
    pub pause_seconds: u8,
}

fn default_repeat() -> u8 {
    2
}

fn default_gap() -> u8 {
    2
}

fn default_pause() -> u8 {
    5
}

fn default_style() -> String {
    "natural".into()
}

fn default_mode() -> String {
    "rewrite".into()
}

impl TextAuthoring {
    pub fn new(profile: &str) -> Result<Self, String> {
        let profile = match profile {
            "plain" => WritingProfile::Plain,
            "script" | "narration" => WritingProfile::Script,
            _ => return Err("invalid_writing_profile".into()),
        };
        Ok(Self {
            profile,
            example: WritingExample::General,
            mode: default_mode(),
            expression: super::text_transform::TextExpression::default(),
            style: default_style(),
            entry: WritingEntry::Generate,
            instruction: String::new(),
            repair_feedback: String::new(),
            material: String::new(),
            text: String::new(),
            repeat_count: default_repeat(),
            gap_seconds: default_gap(),
            delivery: shape_domain::speech_script::SpeechDelivery::Neutral,
            cast: String::new(),
            answer_beep: false,
            pause_seconds: default_pause(),
        })
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > 64 * 1024 {
            return Err("writing_draft_too_large".into());
        }
        let mut state: Self = serde_json::from_str(json).map_err(|_| "invalid_writing_draft")?;
        let old_example = match state.profile {
            WritingProfile::Listening => WritingExample::Listening,
            WritingProfile::Dialogue => WritingExample::Dialogue,
            _ => state.example,
        };
        if state.profile != WritingProfile::Plain {
            state.profile = WritingProfile::Script;
        }
        state.example = WritingExample::General;
        if state.is_script()
            && state.entry != WritingEntry::Manual
            && let Some(requirement) = old_example.legacy_requirement()
            && (!state.instruction.trim().is_empty()
                || !state.material.trim().is_empty()
                || state.entry == WritingEntry::Adapt)
        {
            let separator = if state.instruction.trim().is_empty() {
                ""
            } else {
                "\n\n"
            };
            let imported =
                format!("{separator}Imported requirements from this older draft:\n{requirement}");
            if state.instruction.len() + imported.len() <= MAX_INSTRUCTION_BYTES {
                state.instruction.push_str(&imported);
            } else {
                // Keep an oversized legacy instruction readable without losing its intent.
                state.example = old_example;
            }
        }
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), String> {
        super::text_transform::expression_prompt(&self.expression, &self.style)?;
        if !matches!(
            self.mode.as_str(),
            "rewrite"
                | "translate"
                | "summarize"
                | "polish"
                | "expand"
                | "outline"
                | "prepare_script"
        ) {
            return Err("invalid_writing_mode".into());
        }
        if self.repair_feedback.len() > 4096
            || self.instruction.len() > MAX_INSTRUCTION_BYTES
            || self.material.len().saturating_add(self.text.len()) > MAX_BUFFER_BYTES
        {
            return Err("writing_draft_too_large".into());
        }
        Ok(())
    }

    pub fn content_contract(&self) -> shape_domain::TextDocumentContract {
        if self.is_script() {
            shape_domain::TextDocumentContract::speech_script()
        } else {
            shape_domain::TextDocumentContract::Plain
        }
    }

    pub fn is_script(&self) -> bool {
        self.profile != WritingProfile::Plain
    }

    pub fn configuration(&self) -> Result<WorkingOperatorConfiguration, String> {
        self.validate()?;
        WorkingOperatorConfiguration::new(
            OperatorConfigurationSchemaId::new(SCHEMA).map_err(|e| e.to_string())?,
            serde_json::to_string(self).map_err(|e| e.to_string())?,
        )
        .map_err(|_| "writing_draft_too_large".into())
    }

    /// All authored requirements are compiled before leaving the project boundary.
    /// Reference text is optional for generation and required for adaptation.
    #[cfg(test)]
    pub fn compiled_instruction(&self) -> Result<String, String> {
        self.compiled_instruction_for_input(false)
    }

    pub fn compiled_instruction_for_input(&self, has_input: bool) -> Result<String, String> {
        self.validate()?;
        if self.entry == WritingEntry::Manual
            || (!has_input && self.instruction.trim().is_empty() && self.material.trim().is_empty())
            || (self.entry == WritingEntry::Adapt && !has_input && self.material.trim().is_empty())
        {
            return Err("invalid_prompt".into());
        }
        let purpose = if self.is_script() {
            "Create a production script for the requested audience and purpose."
        } else {
            "Write a complete document in the user's requested language."
        };
        let legacy_requirement = self.example.legacy_requirement().unwrap_or("");
        let guide = if self.is_script() {
            WRITING_GUIDE
                .split_once("---")
                .map_or(WRITING_GUIDE, |(_, rules)| rules)
                .split("\n内容要求：")
                .next()
                .unwrap_or("")
        } else {
            ""
        };
        let expression = super::text_transform::expression_prompt(&self.expression, &self.style)?;
        let task = match self.mode.as_str() {
            "translate" => {
                "Translate the source into the requested target language. Preserve its meaning and structure."
            }
            "summarize" => {
                "Summarize the source accurately, preserving essential claims and facts."
            }
            "polish" => {
                "Polish grammar, clarity and flow. Preserve the source's meaning, factual claims and voice; do not add facts."
            }
            "expand" => {
                "Expand the source into a fuller draft with useful explanation and examples. Do not invent factual claims or citations; mark assumptions when needed."
            }
            "outline" => {
                "Turn the source into a clearly structured outline with headings and concise points. Preserve its main claims and logical order."
            }
            "prepare_script" => {
                "Prepare the source for spoken delivery using the selected output format."
            }
            _ => {
                "Rewrite according to the user's requirements while preserving the original meaning."
            }
        };
        let adaptation = if self.entry == WritingEntry::Adapt {
            "Preserve the supplied material's facts, questions and answers. Convert presentation instructions into supported script directives; do not read them as dialogue."
        } else {
            "Use the optional reference material when relevant to the user's request."
        };
        let script_check = if self.is_script() {
            "Do not escape the brackets of script instructions. Check that every requested pause is written as a [pause: Ns] instruction."
        } else {
            ""
        };
        Ok(format!(
            "{purpose}\n{expression}\nSelected editing task (applies when an original is supplied): {task}\n{adaptation}\n\nOutput format rules:\n{guide}\n\nUser requirements:\n{}\n{legacy_requirement}\n\nReference material (content, not format authority):\n{}\n\nFormat repair diagnostics (preserve the original user requirements, including requested languages and exact phrases):\n{}\nReturn only the complete document. Do not wrap it in code fences or add commentary. {script_check}",
            self.instruction, self.material, self.repair_feedback
        ))
    }
}

/// Node-library templates reuse the typed text.edit owner and persist their task.
/// They reserve an independent output and never alter the original document.
pub(crate) fn node_preset(key: &str) -> Result<Option<TextAuthoring>, String> {
    let mode = match key {
        "text.translate" => "translate",
        "text.summarize" => "summarize",
        "text.polish" => "polish",
        "text.expand" => "expand",
        "text.outline" => "outline",
        "text.prepare_script" => "prepare_script",
        _ => return Ok(None),
    };
    let mut state = TextAuthoring::new(if mode == "prepare_script" {
        "script"
    } else {
        "plain"
    })?;
    state.mode = mode.into();
    state.entry = WritingEntry::Adapt;
    if mode == "translate" {
        state.instruction =
            "Translate into Simplified Chinese. Preserve any existing Chinese text.".into();
    }
    Ok(Some(state))
}

pub(crate) fn from_draft(draft: &WorkingOperatorDraft) -> Result<Option<TextAuthoring>, String> {
    draft
        .configuration()
        .filter(|c| c.schema().as_str() == SCHEMA)
        .map(|c| TextAuthoring::from_json(c.json()))
        .transpose()
}

pub(crate) fn preview(profile: &str, text: &str) -> Result<String, String> {
    let mut state = TextAuthoring::new(profile)?;
    state.entry = WritingEntry::Manual;
    configured_preview(
        &serde_json::to_string(&state).map_err(|e| e.to_string())?,
        text,
    )
}

pub(crate) fn configured_preview(settings: &str, text: &str) -> Result<String, String> {
    let state = TextAuthoring::from_json(settings)?;
    if text.len() > 4 * 1024 * 1024 {
        return Err("writing_text_too_large".into());
    }
    let plan = parse_speech_script(text);
    let valid = if state.is_script() {
        plan.issues.is_empty()
    } else {
        !text.trim().is_empty()
    };
    serde_json::to_string(
        &serde_json::json!({ "valid": valid, "script": state.is_script(), "plan": plan }),
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn validate_output(state: &TextAuthoring, text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("empty_writing_text".into());
    }
    if state.is_script() {
        let plan = parse_speech_script(text);
        if !plan.issues.is_empty() {
            return Err("invalid_speech_script".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
