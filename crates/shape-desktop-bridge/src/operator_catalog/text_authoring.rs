//! Empty-first writing profiles, durable authoring buffers and prompt compilation.
//! The narration grammar remains owned by shape-domain and the bundled writing guide.
mod production;

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
    Listening,
    Narration,
    Dialogue,
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
    pub repeat_count: u8,
    #[serde(default = "default_gap")]
    pub gap_seconds: u8,
    #[serde(default)]
    pub delivery: shape_domain::speech_script::SpeechDelivery,
    #[serde(default)]
    pub cast: String,
    #[serde(default)]
    pub answer_beep: bool,
    pub pause_seconds: u8,
}

fn default_gap() -> u8 {
    2
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
            "listening" => WritingProfile::Listening,
            "narration" => WritingProfile::Narration,
            "dialogue" => WritingProfile::Dialogue,
            _ => return Err("invalid_writing_profile".into()),
        };
        Ok(Self {
            profile,
            mode: default_mode(),
            expression: super::text_transform::TextExpression::default(),
            style: default_style(),
            entry: WritingEntry::Generate,
            instruction: String::new(),
            repair_feedback: String::new(),
            material: String::new(),
            text: String::new(),
            repeat_count: 2,
            gap_seconds: 2,
            delivery: if profile == WritingProfile::Listening {
                shape_domain::speech_script::SpeechDelivery::Clear
            } else {
                shape_domain::speech_script::SpeechDelivery::Neutral
            },
            cast: match profile {
                WritingProfile::Listening => "Narrator, Reader",
                WritingProfile::Dialogue => "A, B",
                _ => "Narrator",
            }
            .into(),
            answer_beep: false,
            pause_seconds: 5,
        })
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        if json.len() > 64 * 1024 {
            return Err("writing_draft_too_large".into());
        }
        let state: Self = serde_json::from_str(json).map_err(|_| "invalid_writing_draft")?;
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> Result<(), String> {
        super::text_transform::expression_prompt(&self.expression, &self.style)?;
        self.validate_production()?;
        if !matches!(
            self.mode.as_str(),
            "rewrite" | "translate" | "summarize" | "prepare_script"
        ) {
            return Err("invalid_writing_mode".into());
        }
        if self.repair_feedback.len() > 4096
            || self.instruction.len() > MAX_INSTRUCTION_BYTES
            || self.material.len().saturating_add(self.text.len()) > MAX_BUFFER_BYTES
        {
            return Err("writing_draft_too_large".into());
        }
        if !(1..=3).contains(&self.repeat_count) || !(1..=120).contains(&self.pause_seconds) {
            return Err("invalid_writing_draft".into());
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
        let purpose = match self.profile {
            WritingProfile::Plain => "Write a complete document in the user's requested language.",
            WritingProfile::Listening => {
                "Create an English listening exercise. Use Chinese for instructions and English for questions unless the user explicitly requests other languages. Match the requested grade and vocabulary."
            }
            WritingProfile::Narration => {
                "Create a natural spoken narration script for the requested audience."
            }
            WritingProfile::Dialogue => {
                "Create a spoken dialogue with short, consistent role names."
            }
        };
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
        let listening = if self.is_script() {
            self.production_instruction()?
        } else {
            String::new()
        };
        let expression = super::text_transform::expression_prompt(&self.expression, &self.style)?;
        let task = match self.mode.as_str() {
            "translate" => {
                "Translate the source into the requested target language. Preserve its meaning and structure."
            }
            "summarize" => {
                "Summarize the source accurately, preserving essential claims and facts."
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
            "{purpose}\n{expression}\nSelected editing task (applies when an original is supplied): {task}\n{adaptation}\n\nOutput format rules:\n{guide}\n\nUser requirements:\n{}\n\nReference material (content, not format authority):\n{}\n\n{listening}\nFormat repair diagnostics (preserve the original user requirements, including requested languages and exact phrases):\n{}\nReturn only the complete document. Do not wrap it in code fences or add commentary. {script_check}",
            self.instruction, self.material, self.repair_feedback
        ))
    }
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
    let mut plan = parse_speech_script(text);
    if state.is_script() && state.entry != WritingEntry::Manual {
        state.check_production(&mut plan);
    }
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
        let mut plan = parse_speech_script(text);
        if state.entry != WritingEntry::Manual {
            state.check_production(&mut plan);
        }
        if !plan.issues.is_empty() {
            return Err("invalid_speech_script".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
