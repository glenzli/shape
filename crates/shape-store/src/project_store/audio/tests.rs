use shape_domain::{ArtifactContentContract, AudioOriginDisclosure, AudioValueContract};

use super::validate_audio_content;

fn wav(sample_rate_hz: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block_align = channels * 2;
    let data_size = frames * u32::from(block_align);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
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
fn exact_wav_bytes_must_match_the_declared_audio_contract() {
    let bytes = wav(24_000, 1, 24_000);
    let contract = ArtifactContentContract::AudioClip(
        AudioValueContract::pcm_s16le_wav(
            24_000,
            1,
            24_000,
            AudioOriginDisclosure::SyntheticSpeech,
        )
        .unwrap(),
    );
    validate_audio_content("audio/wav", Some(&contract), &bytes).unwrap();

    let mismatched = ArtifactContentContract::AudioClip(
        AudioValueContract::pcm_s16le_wav(
            48_000,
            1,
            24_000,
            AudioOriginDisclosure::SyntheticSpeech,
        )
        .unwrap(),
    );
    assert!(validate_audio_content("audio/wav", Some(&mismatched), &bytes).is_err());
    assert!(validate_audio_content("audio/mpeg", Some(&contract), &bytes).is_err());
    assert!(validate_audio_content("audio/wav", Some(&contract), b"not wav").is_err());
}
