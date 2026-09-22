//! Deterministic script compilation and sample-accurate WAV timeline validation.
//! Runtime only renders spoken actions; Shape owns silence and sequential cues.
use crate::{
    ExecutionFailure, ExecutionInput, ExternalExecutionProvenance, audio::decode_pcm_s16le_wav,
};
use serde::{Deserialize, Serialize};
use shape_domain::{
    AudioOriginDisclosure, ContentDigest, SpeechSynthesisOperation, SpeechVoiceSelection,
    speech_script::{SpeechCueAction, SpeechScriptEventKind, parse_speech_script},
};

pub const SCRIPT_SAMPLE_RATE: u32 = 24_000;
pub const MAX_CUE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechScriptAssembly {
    pub source_digest: ContentDigest,
    pub operation_digest: ContentDigest,
    pub plan_digest: ContentDigest,
    pub pieces: Vec<SpeechScriptPiece>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechScriptPiece {
    pub event: u32,
    pub frames: u64,
    pub pcm_digest: ContentDigest,
    pub speech_segment: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_of: Option<u32>,
}
impl SpeechScriptAssembly {
    pub(crate) fn is_bounded(&self) -> bool {
        !self.pieces.is_empty()
            && self.pieces.len() <= 2560
            && self.pieces.iter().all(|p| {
                p.event < 2048
                    && p.frames <= 67_108_864
                    && p.speech_segment.is_none_or(|s| s < 512)
                    && p.replay_of.is_none_or(|i| i < 2560)
            })
    }
    #[must_use]
    pub fn frames(&self) -> Option<u64> {
        self.pieces
            .iter()
            .try_fold(0_u64, |sum, p| sum.checked_add(p.frames))
    }
}

pub(crate) struct ScriptAction {
    pub event: u32,
    pub kind: ScriptActionKind,
}
pub(crate) enum ScriptActionKind {
    Replay {
        piece: u32,
    },
    Speech {
        text: String,
        operation: SpeechSynthesisOperation,
    },
    Local {
        pcm: Vec<u8>,
    },
}

// Keep the complete control-to-action dispatch in one auditable compiler.
#[allow(clippy::too_many_lines)]
pub(crate) fn compile(
    source: &str,
    operation: &SpeechSynthesisOperation,
    inputs: &[ExecutionInput],
) -> Result<Vec<ScriptAction>, ExecutionFailure> {
    let options = operation.script.as_ref().ok_or_else(invalid_script)?;
    let plan = parse_speech_script(source);
    if !plan.ready(options) {
        return Err(invalid_script());
    }
    let mut actions = Vec::new();
    let mut speech_count = 0;
    let mut local_bytes = 0_usize;
    let mut repeat: Option<(usize, u8, u32)> = None;
    for (event, item) in plan.events.iter().enumerate() {
        let event = u32::try_from(event).map_err(|_| invalid_script())?;
        match &item.kind {
            SpeechScriptEventKind::Speech {
                role,
                language,
                delivery,
                text,
            } => {
                let mut selected = operation.clone();
                selected.script = None;
                selected.delivery = Some(*delivery);
                if let Some(voice) = options.roles.get(role) {
                    selected.voice = SpeechVoiceSelection::Preset(voice.clone());
                }
                if let Some(language) = language {
                    selected.language.clone_from(language);
                }
                if !crate::supported_speech_operation(&selected) {
                    return Err(invalid_script());
                }
                for part in crate::infer_runtime::speech::narration::split_text(text) {
                    speech_count += 1;
                    if speech_count > 512 {
                        return Err(invalid_script());
                    }
                    actions.push(ScriptAction {
                        event,
                        kind: ScriptActionKind::Speech {
                            text: part.to_owned(),
                            operation: selected.clone(),
                        },
                    });
                }
            }
            SpeechScriptEventKind::Pause { milliseconds } => {
                let bytes =
                    usize::try_from(u64::from(*milliseconds) * 48).map_err(|_| invalid_script())?;
                local_bytes = local_bytes.checked_add(bytes).ok_or_else(invalid_script)?;
                if local_bytes > 128 * 1024 * 1024 - 44 {
                    return Err(invalid_script());
                }
                actions.push(ScriptAction {
                    event,
                    kind: ScriptActionKind::Local {
                        pcm: vec![0; bytes],
                    },
                });
            }
            SpeechScriptEventKind::Cue { label } => {
                let pcm = match plan.cue(label, options).ok_or_else(invalid_script)? {
                    SpeechCueAction::Skip => Vec::new(),
                    SpeechCueAction::Chime => chime(),
                    SpeechCueAction::Beep {
                        milliseconds,
                        level,
                    } => beep(*milliseconds, *level),
                    SpeechCueAction::Audio { content } => {
                        let bytes = inputs
                            .iter()
                            .find(|input| input.content() == content)
                            .and_then(ExecutionInput::bytes)
                            .ok_or_else(invalid_script)?;
                        validate_cue(bytes)?;
                        decode_pcm_s16le_wav(bytes, AudioOriginDisclosure::RecordedSource)?
                            .1
                            .to_vec()
                    }
                };
                local_bytes = local_bytes
                    .checked_add(pcm.len())
                    .ok_or_else(invalid_script)?;
                if local_bytes > 128 * 1024 * 1024 - 44 {
                    return Err(invalid_script());
                }
                actions.push(ScriptAction {
                    event,
                    kind: ScriptActionKind::Local { pcm },
                });
            }
            SpeechScriptEventKind::RepeatStart { count, gap_ms } => {
                if repeat.replace((actions.len(), *count, *gap_ms)).is_some() {
                    return Err(invalid_script());
                }
            }
            SpeechScriptEventKind::RepeatEnd => {
                let (start, count, gap_ms) = repeat.take().ok_or_else(invalid_script)?;
                let end = actions.len();
                let additions = (end - start + usize::from(gap_ms > 0)) * usize::from(count - 1);
                if actions.len().saturating_add(additions) > 2560 {
                    return Err(invalid_script());
                }
                for _ in 1..count {
                    if gap_ms > 0 {
                        let bytes = gap_ms as usize * 48;
                        local_bytes = local_bytes.checked_add(bytes).ok_or_else(invalid_script)?;
                        if local_bytes > 128 * 1024 * 1024 - 44 {
                            return Err(invalid_script());
                        }
                        actions.push(ScriptAction {
                            event,
                            kind: ScriptActionKind::Local {
                                pcm: vec![0; bytes],
                            },
                        });
                    }
                    for piece in start..end {
                        actions.push(ScriptAction {
                            event,
                            kind: ScriptActionKind::Replay {
                                piece: u32::try_from(piece).map_err(|_| invalid_script())?,
                            },
                        });
                    }
                }
            }
            SpeechScriptEventKind::Heading { .. }
            | SpeechScriptEventKind::Note { .. }
            | SpeechScriptEventKind::Scene { .. } => {}
        }
    }
    if repeat.is_some() || actions.len() > 2560 {
        return Err(invalid_script());
    }
    let known_bytes = actions.iter().try_fold(0_usize, |sum, action| {
        let bytes = match &action.kind {
            ScriptActionKind::Local { pcm } => pcm.len(),
            ScriptActionKind::Replay { piece } => match &actions[*piece as usize].kind {
                ScriptActionKind::Local { pcm } => pcm.len(),
                _ => 0,
            },
            ScriptActionKind::Speech { .. } => 0,
        };
        sum.checked_add(bytes)
            .filter(|n| *n <= 128 * 1024 * 1024 - 44)
    });
    if known_bytes.is_none() {
        return Err(invalid_script());
    }
    Ok(actions)
}

/// First revision uses exact, lossless 24 kHz mono PCM16 cues, matching local speech.
/// # Errors
/// Rejects unsupported formats or a cue larger than 8 MiB.
pub fn validate_cue(bytes: &[u8]) -> Result<(), ExecutionFailure> {
    let (contract, _) = decode_pcm_s16le_wav(bytes, AudioOriginDisclosure::RecordedSource)?;
    if bytes.len() > MAX_CUE_BYTES
        || contract.sample_rate_hz != SCRIPT_SAMPLE_RATE
        || contract.channels != 1
    {
        return Err(invalid_script());
    }
    Ok(())
}

fn beep(milliseconds: u32, level: shape_domain::speech_script::CueLevel) -> Vec<u8> {
    use shape_domain::speech_script::CueLevel;
    let frames = milliseconds * 24;
    let amplitude: i32 = match level {
        CueLevel::Soft => 1800,
        CueLevel::Normal => 3600,
    };
    let mut pcm = Vec::with_capacity(frames as usize * 2);
    for frame in 0..frames {
        // A fixed 1 kHz triangle with 5 ms edge ramps; integer arithmetic is portable.
        let phase = (frame % 24).cast_signed();
        let triangle = (phase * 4 - 48).abs() - 24;
        let ramp = frame.min(frames - 1 - frame).min(120).cast_signed();
        let sample = i16::try_from(triangle * amplitude * ramp / (24 * 120))
            .expect("beep amplitude is bounded by 3600");
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    pcm
}

fn chime() -> Vec<u8> {
    // Fixed integer oscillators make accepted bytes reproducible across platforms.
    let mut pcm = Vec::with_capacity(48_000);
    for frame in 0_i32..24_000 {
        let (local, period) = if frame < 12_000 {
            (frame, 30)
        } else {
            (frame - 12_000, 40)
        };
        let phase = local % period;
        let triangle = (phase * 4 - period * 2).abs() - period;
        let amplitude =
            i64::from(triangle) * 5000 * i64::from(12_000 - local) * i64::from(local.min(240));
        let sample = i16::try_from(amplitude / (i64::from(period) * 12_000 * 240))
            .expect("chime amplitude is at most 5000");
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    pcm
}

/// Recompiles exact accepted script bytes and verifies all timeline PCM ranges.
/// Both transient proposal and durable acceptance call this same contract check.
/// # Errors
/// Rejects source, plan, timeline, voice binding, cue, duration or PCM mismatches.
pub fn validate_output(
    source: &str,
    operation: &SpeechSynthesisOperation,
    inputs: &[ExecutionInput],
    provenance: &ExternalExecutionProvenance,
    output: &[u8],
) -> Result<(), ExecutionFailure> {
    let plan = parse_speech_script(source);
    let assembly = provenance
        .speech_script
        .as_ref()
        .ok_or_else(invalid_script)?;
    if assembly.operation_digest
        != ContentDigest::from_bytes(&serde_json::to_vec(operation).map_err(|_| invalid_script())?)
        || assembly.source_digest != plan.source_digest
        || assembly.plan_digest != plan.digest()
        || !assembly.is_bounded()
    {
        return Err(invalid_script());
    }
    let actions = compile(source, operation, inputs)?;
    let (contract, pcm) = decode_pcm_s16le_wav(output, AudioOriginDisclosure::SyntheticSpeech)?;
    if contract.sample_rate_hz != SCRIPT_SAMPLE_RATE
        || contract.channels != 1
        || assembly.frames() != Some(contract.frame_count)
        || actions.len() != assembly.pieces.len()
    {
        return Err(invalid_script());
    }
    let mut offset = 0_usize;
    let mut input_start = 0_u32;
    let mut speech_index = 0_usize;
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    for (action, piece) in actions.iter().zip(&assembly.pieces) {
        let count = usize::try_from(piece.frames)
            .ok()
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(invalid_script)?;
        let end = offset.checked_add(count).ok_or_else(invalid_script)?;
        let bytes = pcm.get(offset..end).ok_or_else(invalid_script)?;
        if action.event != piece.event || ContentDigest::from_bytes(bytes) != piece.pcm_digest {
            return Err(invalid_script());
        }
        match &action.kind {
            ScriptActionKind::Local { pcm } => {
                if piece.replay_of.is_some() || piece.speech_segment.is_some() || bytes != pcm {
                    return Err(invalid_script());
                }
            }
            ScriptActionKind::Replay { piece: original } => {
                let index = *original as usize;
                let range = ranges.get(index).ok_or_else(invalid_script)?;
                let original_piece = assembly.pieces.get(index).ok_or_else(invalid_script)?;
                if piece.replay_of != Some(*original)
                    || piece.speech_segment != original_piece.speech_segment
                    || bytes != &pcm[range.clone()]
                {
                    return Err(invalid_script());
                }
            }
            ScriptActionKind::Speech { text, .. } => {
                if piece.replay_of.is_some() {
                    return Err(invalid_script());
                }
                let segment = provenance
                    .speech_segments
                    .get(speech_index)
                    .ok_or_else(invalid_script)?;
                let end = input_start + u32::try_from(text.len()).map_err(|_| invalid_script())?;
                if piece.speech_segment
                    != Some(u32::try_from(speech_index).map_err(|_| invalid_script())?)
                    || segment.frames != piece.frames
                    || segment.input_start != input_start
                    || segment.input_end != end
                    || segment.input_digest != ContentDigest::from_bytes(text.as_bytes())
                {
                    return Err(invalid_script());
                }
                speech_index += 1;
                input_start = end;
            }
        }
        ranges.push(offset..end);
        offset = end;
    }
    if offset != pcm.len() || speech_index != provenance.speech_segments.len() {
        return Err(invalid_script());
    }
    Ok(())
}

pub(crate) fn invalid_script() -> ExecutionFailure {
    ExecutionFailure::new(
        "invalid_speech_script",
        "Resolve script diagnostics, voice selections and audio cues before synthesis",
        false,
    )
}

#[cfg(test)]
mod tests;
