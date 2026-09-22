use shape_domain::AudioOriginDisclosure;

#[test]
fn speech_assembly_preserves_pcm_order_and_rejects_format_changes() {
    let mut first = wav(24_000, 1, 3);
    let mut second = wav(24_000, 1, 2);
    first[44..].fill(17);
    second[44..].fill(29);
    let mut assembly = super::SpeechWaveAssembly::default();
    assembly.push(&first).unwrap();
    assembly.push(&second).unwrap();
    let combined = assembly.finish().unwrap();
    assert_eq!(&combined[44..], &[17, 17, 17, 17, 17, 17, 29, 29, 29, 29]);
    assert_eq!(
        super::parse_pcm_s16le_wav(&combined, AudioOriginDisclosure::SyntheticSpeech)
            .unwrap()
            .frame_count,
        5
    );
    let mut assembly = super::SpeechWaveAssembly::default();
    assembly.push(&first).unwrap();
    assert_eq!(
        assembly.push(&wav(48_000, 1, 3)).unwrap_err().code,
        "speech_format_changed"
    );
}

use super::parse_pcm_s16le_wav;

fn wav(sample_rate_hz: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block_align = channels * 2;
    let data_size = frames * u32::from(block_align);
    let riff_size = 36 + data_size;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate_hz * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.resize(bytes.len() + data_size as usize, 0);
    bytes
}

#[test]
fn pcm_wav_projects_exact_frames_and_duration() {
    let bytes = wav(24_000, 1, 36_000);
    let contract = parse_pcm_s16le_wav(&bytes, AudioOriginDisclosure::SyntheticSpeech).unwrap();
    assert_eq!(contract.sample_rate_hz, 24_000);
    assert_eq!(contract.channels, 1);
    assert_eq!(contract.frame_count, 36_000);
    assert_eq!(contract.duration_millis(), 1_500);
}

#[test]
fn malformed_lengths_formats_and_empty_audio_fail_closed() {
    let mut truncated = wav(24_000, 1, 8);
    truncated.pop();
    assert!(parse_pcm_s16le_wav(&truncated, AudioOriginDisclosure::SyntheticSpeech).is_err());

    let mut float_format = wav(24_000, 1, 8);
    float_format[20..22].copy_from_slice(&3_u16.to_le_bytes());
    assert!(parse_pcm_s16le_wav(&float_format, AudioOriginDisclosure::SyntheticSpeech).is_err());

    assert!(
        parse_pcm_s16le_wav(&wav(24_000, 1, 0), AudioOriginDisclosure::SyntheticSpeech).is_err()
    );
}
