//! Bounded validation for materialized audio executor outputs.

use shape_domain::{ArtifactContentContract, AudioOriginDisclosure, AudioValueContract};

use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Imports exact externally supplied WAV bytes without asserting how they were made.
pub const AUDIO_IMPORT_CAPABILITY: &str = "audio.clip.import";

/// Exact, bounded adapter for an external audio file selected by the user.
#[derive(Debug)]
pub struct AudioImportExecutor {
    identity: ExecutorIdentity,
}

impl AudioImportExecutor {
    /// Creates the built-in import executor.
    /// # Errors
    /// Returns an invalid identity error if the built-in constants drift.
    pub fn new() -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.audio-import",
                env!("CARGO_PKG_VERSION"),
                "20260925.1",
            )?,
        })
    }
}

impl Executor for AudioImportExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == AUDIO_IMPORT_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if !request.inputs.is_empty() || request.output_media_type != "audio/wav" {
            return Err(ExecutionFailure::new(
                "invalid_audio_import",
                "Audio import requires one exact WAV payload and no accepted inputs",
                false,
            ));
        }
        let contract = parse_pcm_s16le_wav(
            &request.instruction,
            AudioOriginDisclosure::ImportedUnverified,
        )?;
        Ok(ExecutionOutput {
            bytes: request.instruction.clone(),
            media_type: "audio/wav".to_owned(),
            executor_job_id: None,
            external_provenance: None,
            content_contract: Some(ArtifactContentContract::AudioClip(contract)),
        })
    }
}

pub(crate) const MAX_AUDIO_OUTPUT_BYTES: usize = 128 * 1024 * 1024;

/// Validates one exact PCM S16 LE WAV value and returns its portable contract.
///
/// # Errors
///
/// Rejects oversized, malformed, empty, or unsupported WAV values.
pub fn parse_pcm_s16le_wav(
    bytes: &[u8],
    origin: AudioOriginDisclosure,
) -> Result<AudioValueContract, ExecutionFailure> {
    Ok(decode_pcm_s16le_wav(bytes, origin)?.0)
}

pub(crate) fn decode_pcm_s16le_wav(
    bytes: &[u8],
    origin: AudioOriginDisclosure,
) -> Result<(AudioValueContract, &[u8]), ExecutionFailure> {
    if bytes.len() < 44 || bytes.len() > MAX_AUDIO_OUTPUT_BYTES {
        return Err(invalid_audio());
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(invalid_audio());
    }
    let declared_size = read_u32(&bytes[4..8])
        .checked_add(8)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or_else(invalid_audio)?;
    if declared_size != bytes.len() {
        return Err(invalid_audio());
    }

    let mut position = 12_usize;
    let mut format = None;
    let mut data_bytes = None;
    while position < bytes.len() {
        let header_end = position.checked_add(8).ok_or_else(invalid_audio)?;
        if header_end > bytes.len() {
            return Err(invalid_audio());
        }
        let chunk_id = &bytes[position..position + 4];
        let chunk_size = usize::try_from(read_u32(&bytes[position + 4..header_end]))
            .map_err(|_| invalid_audio())?;
        let payload_end = header_end
            .checked_add(chunk_size)
            .ok_or_else(invalid_audio)?;
        if payload_end > bytes.len() {
            return Err(invalid_audio());
        }
        let payload = &bytes[header_end..payload_end];
        if chunk_id == b"fmt " {
            if format.is_some() || payload.len() < 16 {
                return Err(invalid_audio());
            }
            format = Some(WaveFormat::parse(payload)?);
        } else if chunk_id == b"data" && data_bytes.replace(payload).is_some() {
            return Err(invalid_audio());
        }
        position = payload_end
            .checked_add(chunk_size % 2)
            .ok_or_else(invalid_audio)?;
        if position > bytes.len() {
            return Err(invalid_audio());
        }
    }

    let format = format.ok_or_else(invalid_audio)?;
    let data_bytes = data_bytes.ok_or_else(invalid_audio)?;
    if data_bytes.is_empty() || data_bytes.len() % usize::from(format.block_align) != 0 {
        return Err(invalid_audio());
    }
    let frame_count = u64::try_from(data_bytes.len() / usize::from(format.block_align))
        .map_err(|_| invalid_audio())?;
    let contract = AudioValueContract::pcm_s16le_wav(
        format.sample_rate_hz,
        format.channels,
        frame_count,
        origin,
    )
    .map_err(|_| invalid_audio())?;
    Ok((contract, data_bytes))
}

/// Bounded assembly of validated, format-identical speech segments. No header bytes
/// or lossy resampling enter the PCM timeline.
#[derive(Debug, Default)]
pub(crate) struct SpeechWaveAssembly {
    format: Option<(u32, u16)>,
    bytes: Vec<u8>,
}

impl SpeechWaveAssembly {
    pub(crate) fn push(&mut self, wav: &[u8]) -> Result<(), ExecutionFailure> {
        let (contract, pcm) = decode_pcm_s16le_wav(wav, AudioOriginDisclosure::SyntheticSpeech)?;
        self.push_pcm(contract.sample_rate_hz, contract.channels, pcm)
    }

    pub(crate) fn push_pcm(
        &mut self,
        rate: u32,
        channels: u16,
        pcm: &[u8],
    ) -> Result<(), ExecutionFailure> {
        let format = (rate, channels);
        if self.format.is_some_and(|expected| expected != format) {
            return Err(ExecutionFailure::new(
                "speech_format_changed",
                "Speech segments have incompatible sample formats",
                false,
            ));
        }
        if self.format.is_none() {
            self.bytes.resize(44, 0);
            self.format = Some(format);
        }
        if self.bytes.len().saturating_add(pcm.len()) > MAX_AUDIO_OUTPUT_BYTES {
            return Err(ExecutionFailure::new(
                "speech_audio_too_large",
                "Narration exceeds the supported audio size",
                false,
            ));
        }
        self.bytes.extend_from_slice(pcm);
        Ok(())
    }

    pub(crate) fn pcm_len(&self) -> usize {
        self.bytes.len().saturating_sub(44)
    }

    /// Replays an already assembled PCM range without a second temporary audio buffer.
    pub(crate) fn replay_pcm(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<(), ExecutionFailure> {
        if range.start > range.end || range.end > self.pcm_len() {
            return Err(invalid_audio());
        }
        if self.bytes.len().saturating_add(range.len()) > MAX_AUDIO_OUTPUT_BYTES {
            return Err(ExecutionFailure::new(
                "speech_audio_too_large",
                "Narration exceeds the supported audio size",
                false,
            ));
        }
        self.bytes
            .extend_from_within(range.start + 44..range.end + 44);
        Ok(())
    }

    pub(crate) fn finish(mut self) -> Result<Vec<u8>, ExecutionFailure> {
        let (sample_rate, channels) = self.format.ok_or_else(invalid_audio)?;
        let data_size = u32::try_from(self.bytes.len() - 44).map_err(|_| invalid_audio())?;
        let mut header = Vec::with_capacity(44);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&(data_size + 36).to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
        header.extend_from_slice(&(channels * 2).to_le_bytes());
        header.extend_from_slice(&16_u16.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_size.to_le_bytes());
        self.bytes[..44].copy_from_slice(&header);
        Ok(self.bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaveFormat {
    channels: u16,
    sample_rate_hz: u32,
    block_align: u16,
}

impl WaveFormat {
    fn parse(bytes: &[u8]) -> Result<Self, ExecutionFailure> {
        let audio_format = read_u16(&bytes[0..2]);
        let channels = read_u16(&bytes[2..4]);
        let sample_rate_hz = read_u32(&bytes[4..8]);
        let byte_rate = read_u32(&bytes[8..12]);
        let block_align = read_u16(&bytes[12..14]);
        let bits_per_sample = read_u16(&bytes[14..16]);
        let expected_block_align = channels.checked_mul(2).ok_or_else(invalid_audio)?;
        let expected_byte_rate = sample_rate_hz
            .checked_mul(u32::from(expected_block_align))
            .ok_or_else(invalid_audio)?;
        if audio_format != 1
            || !(1..=32).contains(&channels)
            || !(8_000..=384_000).contains(&sample_rate_hz)
            || bits_per_sample != 16
            || block_align != expected_block_align
            || byte_rate != expected_byte_rate
        {
            return Err(invalid_audio());
        }
        Ok(Self {
            channels,
            sample_rate_hz,
            block_align,
        })
    }
}

fn read_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn invalid_audio() -> ExecutionFailure {
    ExecutionFailure::new(
        "invalid_audio_output",
        "audio executor returned an invalid or unsupported WAV value",
        false,
    )
}

#[cfg(test)]
mod tests;
