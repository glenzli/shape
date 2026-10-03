use super::*;
use shape_domain::{AudioOriginDisclosure, AudioValueContract};

fn wave() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(40_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(24_000_u32.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(4_u32.to_le_bytes());
    bytes.extend([1, 0, 2, 0]);
    bytes
}

#[test]
fn edited_audio_retains_each_original_disclosure() {
    for origin in [
        AudioOriginDisclosure::SyntheticSpeech,
        AudioOriginDisclosure::SyntheticSound,
        AudioOriginDisclosure::ImportedUnverified,
    ] {
        let source = ArtifactContentContract::AudioClip(
            AudioValueContract::pcm_s16le_wav(24_000, 1, 2, origin).unwrap(),
        );
        let output = normalize_material_return(&source, &wave()).unwrap();
        assert_eq!(output.content_contract, Some(source));
        assert_eq!(output.bytes, wave());
        assert!(output.external_provenance.is_none());
        assert!(output.executor_job_id.is_none());
    }
}

#[test]
fn pcm24_and_truncated_wav_are_not_silently_converted() {
    let source = ArtifactContentContract::AudioClip(
        AudioValueContract::pcm_s16le_wav(24_000, 1, 2, AudioOriginDisclosure::ImportedUnverified)
            .unwrap(),
    );
    let mut bytes = wave();
    bytes[34] = 24;
    assert!(normalize_material_return(&source, &bytes).is_err());
    assert!(normalize_material_return(&source, &wave()[..44]).is_err());
}
