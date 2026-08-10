//! Bounded validation for materialized audio executor outputs.

use shape_domain::{AudioOriginDisclosure, AudioValueContract};

use crate::ExecutionFailure;

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
        } else if chunk_id == b"data" && data_bytes.replace(payload.len()).is_some() {
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
    if data_bytes == 0 || data_bytes % usize::from(format.block_align) != 0 {
        return Err(invalid_audio());
    }
    let frame_count =
        u64::try_from(data_bytes / usize::from(format.block_align)).map_err(|_| invalid_audio())?;
    AudioValueContract::pcm_s16le_wav(format.sample_rate_hz, format.channels, frame_count, origin)
        .map_err(|_| invalid_audio())
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
