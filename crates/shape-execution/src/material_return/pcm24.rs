//! Explicit PCM24-to-PCM16 normalization of bounded mono/stereo RIFF WAV returns.
//! Signed samples round to nearest (ties away from zero), then saturate. No resampling,
//! gain, channel remix, or dither occurs; ancillary chunks are retained byte-for-byte.
use crate::ExecutionFailure;

struct Wave<'a> {
    chunks: Vec<([u8; 4], &'a [u8])>,
    channels: u16,
    rate: u32,
}
fn invalid() -> ExecutionFailure {
    ExecutionFailure::new(
        "invalid_pcm24_return",
        "Expected bounded mono/stereo PCM24 RIFF WAV",
        false,
    )
}
fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}
fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn inspect(bytes: &[u8]) -> Result<Wave<'_>, ExecutionFailure> {
    if bytes.len() < 44
        || bytes.len() > 128 * 1024 * 1024
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WAVE"
        || u64::from(u32le(&bytes[4..8])) + 8 != bytes.len() as u64
    {
        return Err(invalid());
    }
    let mut offset = 12_usize;
    let mut chunks = Vec::new();
    let mut format = None;
    let mut data = None;
    let mut ancillary = 0_usize;
    while offset < bytes.len() {
        let header = bytes.get(offset..offset + 8).ok_or_else(invalid)?;
        let length = usize::try_from(u32le(&header[4..8])).map_err(|_| invalid())?;
        let end = offset
            .checked_add(8)
            .and_then(|o| o.checked_add(length))
            .ok_or_else(invalid)?;
        let payload = bytes.get(offset + 8..end).ok_or_else(invalid)?;
        let tag: [u8; 4] = header[..4].try_into().map_err(|_| invalid())?;
        if &tag == b"fmt " {
            if format.replace(payload).is_some() {
                return Err(invalid());
            }
        } else if &tag == b"data" {
            if data.replace(payload).is_some() {
                return Err(invalid());
            }
        } else {
            ancillary = ancillary.checked_add(length).ok_or_else(invalid)?;
        }
        if chunks.len() >= 4096 || ancillary > 1024 * 1024 {
            return Err(invalid());
        }
        chunks.push((tag, payload));
        offset = end.checked_add(length % 2).ok_or_else(invalid)?;
        if offset > bytes.len() {
            return Err(invalid());
        }
    }
    let fmt = format.ok_or_else(invalid)?;
    let data = data.ok_or_else(invalid)?;
    if fmt.len() < 16 || u16le(&fmt[14..16]) != 24 {
        return Err(invalid());
    }
    let channels = u16le(&fmt[2..4]);
    let rate = u32le(&fmt[4..8]);
    if !(1..=2).contains(&channels)
        || !(8000..=192_000).contains(&rate)
        || u16le(&fmt[12..14]) != channels * 3
        || u32le(&fmt[8..12]) != rate * u32::from(channels) * 3
        || data.is_empty()
        || data.len() % usize::from(channels * 3) != 0
    {
        return Err(invalid());
    }
    match u16le(&fmt[..2]) {
        1 if fmt.len() == 16 || (fmt.len() == 18 && u16le(&fmt[16..18]) == 0) => {}
        65534
            if fmt.len() == 40
                && u16le(&fmt[16..18]) == 22
                && u16le(&fmt[18..20]) == 24
                && [0, if channels == 1 { 4 } else { 3 }].contains(&u32le(&fmt[20..24]))
                && fmt[24..40] == [1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113] => {}
        _ => return Err(invalid()),
    }
    Ok(Wave {
        chunks,
        channels,
        rate,
    })
}

pub(super) fn is_pcm24(bytes: &[u8]) -> bool {
    inspect(bytes).is_ok()
}

pub(super) fn convert(bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let wave = inspect(bytes)?;
    let mut output = Vec::with_capacity(bytes.len());
    output.extend(b"RIFF\0\0\0\0WAVE");
    for (tag, payload) in wave.chunks {
        output.extend(tag);
        if &tag == b"fmt " {
            output.extend(16_u32.to_le_bytes());
            output.extend(1_u16.to_le_bytes());
            output.extend(wave.channels.to_le_bytes());
            output.extend(wave.rate.to_le_bytes());
            output.extend((wave.rate * u32::from(wave.channels) * 2).to_le_bytes());
            output.extend((wave.channels * 2).to_le_bytes());
            output.extend(16_u16.to_le_bytes());
        } else if &tag == b"data" {
            let size = u32::try_from(payload.len() / 3 * 2).map_err(|_| invalid())?;
            output.extend(size.to_le_bytes());
            for sample in payload.chunks_exact(3) {
                let value = i32::from_le_bytes([0, sample[0], sample[1], sample[2]]) >> 8;
                let rounded = if value >= 0 {
                    (value + 128) / 256
                } else {
                    (value - 128) / 256
                };
                let sample = i16::try_from(rounded.clamp(i32::from(i16::MIN), i32::from(i16::MAX)))
                    .map_err(|_| invalid())?;
                output.extend(sample.to_le_bytes());
            }
        } else {
            output.extend(
                u32::try_from(payload.len())
                    .map_err(|_| invalid())?
                    .to_le_bytes(),
            );
            output.extend(payload);
            if payload.len() % 2 != 0 {
                output.push(0);
            }
        }
    }
    let size = u32::try_from(output.len() - 8).map_err(|_| invalid())?;
    output[4..8].copy_from_slice(&size.to_le_bytes());
    Ok(output)
}

#[cfg(test)]
mod tests;
