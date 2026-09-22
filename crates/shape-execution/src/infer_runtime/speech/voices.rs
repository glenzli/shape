//! Versioned Runtime voice selections; provider speaker names stay in Infer.

use shape_domain::{SpeechSynthesisOperation, SpeechVoiceSelection};

use super::{INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1};

/// Expanded Runtime catalog. The original Mandarin preset also accepts its old revision.
pub const INFER_SPEECH_VOICE_CATALOG_REVISION: &str = "infer.speech.voice-aliases@20260922.1";

/// A versioned preset and its declared language, projected by the desktop adapter.
#[derive(Debug, Clone, Copy)]
pub struct SpeechPreset {
    pub key: &'static str,
    pub alias: &'static str,
    pub language: &'static str,
}

/// Runtime-owned identities supported by the local `CustomVoice` adapter.
pub const SPEECH_PRESETS: &[SpeechPreset] = &[
    SpeechPreset {
        key: "vivian",
        alias: INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
        language: "Chinese",
    },
    SpeechPreset {
        key: "serena",
        alias: "speech.voice.zh.warm_female.v1",
        language: "Chinese",
    },
    SpeechPreset {
        key: "uncle_fu",
        alias: "speech.voice.zh.mature_male.v1",
        language: "Chinese",
    },
    SpeechPreset {
        key: "dylan",
        alias: "speech.voice.zh.beijing_male.v1",
        language: "Chinese",
    },
    SpeechPreset {
        key: "eric",
        alias: "speech.voice.zh.sichuan_male.v1",
        language: "Chinese",
    },
    SpeechPreset {
        key: "ryan",
        alias: "speech.voice.en.dynamic_male.v1",
        language: "English",
    },
    SpeechPreset {
        key: "aiden",
        alias: "speech.voice.en.warm_male.v1",
        language: "English",
    },
    SpeechPreset {
        key: "ono_anna",
        alias: "speech.voice.ja.bright_female.v1",
        language: "Japanese",
    },
    SpeechPreset {
        key: "sohee",
        alias: "speech.voice.ko.warm_female.v1",
        language: "Korean",
    },
];

/// Validates a preset against its exact version and language without contacting Runtime.
#[must_use]
pub fn supported_speech_operation(operation: &SpeechSynthesisOperation) -> bool {
    let SpeechVoiceSelection::Preset(voice) = &operation.voice else {
        return false;
    };
    let known_voice = SPEECH_PRESETS
        .iter()
        .any(|preset| preset.alias == voice.alias.as_str());
    operation.synthetic_disclosure_required
        && known_voice
        && ((voice.catalog_revision == INFER_SPEECH_VOICE_CATALOG_REVISION
            && SPEECH_LANGUAGES.contains(&operation.language.as_str()))
            || (voice.catalog_revision == INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION
                && voice.alias.as_str() == INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1
                && operation.language == "Chinese"))
}

/// Explicit language controls accepted by Qwen; auto supports mixed-language text.
pub const SPEECH_LANGUAGES: &[&str] = &[
    "auto",
    "Chinese",
    "English",
    "Japanese",
    "Korean",
    "German",
    "French",
    "Russian",
    "Portuguese",
    "Spanish",
    "Italian",
];
