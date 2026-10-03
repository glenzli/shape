use super::*;
pub(super) fn wave(values: &[i32], extensible: bool) -> Vec<u8> {
    let mut b = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
    b.extend((if extensible { 40_u32 } else { 16_u32 }).to_le_bytes());
    b.extend((if extensible { 65534_u16 } else { 1_u16 }).to_le_bytes());
    b.extend(1_u16.to_le_bytes());
    b.extend(48_000_u32.to_le_bytes());
    b.extend(144_000_u32.to_le_bytes());
    b.extend(3_u16.to_le_bytes());
    b.extend(24_u16.to_le_bytes());
    if extensible {
        b.extend(22_u16.to_le_bytes());
        b.extend(24_u16.to_le_bytes());
        b.extend(4_u32.to_le_bytes());
        b.extend([1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
    }
    b.extend(b"LIST");
    b.extend(4_u32.to_le_bytes());
    b.extend(b"INFO");
    b.extend(b"data");
    b.extend(u32::try_from(values.len() * 3).unwrap().to_le_bytes());
    for v in values {
        b.extend(&v.to_le_bytes()[..3]);
    }
    if values.len() % 2 == 1 {
        b.push(0);
    }
    let len = u32::try_from(b.len() - 8).unwrap();
    b[4..8].copy_from_slice(&len.to_le_bytes());
    b
}
#[test]
fn explicit_quantization_handles_extrema_ties_and_preserves_metadata() {
    let values = [
        -8_388_608, -384, -256, -128, -127, 0, 127, 128, 256, 384, 8_388_607,
    ];
    for extensible in [false, true] {
        let converted = convert(&wave(&values, extensible)).unwrap();
        let (contract, pcm) = crate::audio::decode_pcm_s16le_wav(
            &converted,
            shape_domain::AudioOriginDisclosure::ImportedUnverified,
        )
        .unwrap();
        assert_eq!(contract.frame_count, values.len() as u64);
        assert_eq!(contract.sample_rate_hz, 48_000);
        let samples: Vec<_> = pcm
            .chunks_exact(2)
            .map(|s| i16::from_le_bytes([s[0], s[1]]))
            .collect();
        assert_eq!(samples, [-32768, -2, -1, -1, 0, 0, 0, 1, 1, 2, 32767]);
        assert!(converted.windows(4).any(|b| b == b"INFO"));
        for (original, quantized) in values.iter().zip(samples) {
            assert!((original - i32::from(quantized) * 256).abs() <= 255);
        }
    }
}
#[test]
fn invalid_guid_bit_depth_alignment_sizes_and_duplicate_chunks_are_rejected() {
    let good = wave(&[256, 512], true);
    for (offset, value) in [(24, 0), (34, 16), (38, 16), (44, 3), (32, 6)] {
        let mut b = good.clone();
        b[offset] = value;
        assert!(!is_pcm24(&b), "offset {offset}");
    }
    let mut extra = good.clone();
    extra.extend(b"data\0\0\0\0");
    let n = u32::try_from(extra.len() - 8).unwrap();
    extra[4..8].copy_from_slice(&n.to_le_bytes());
    assert!(convert(&extra).is_err());
    assert!(convert(&good[..good.len() - 1]).is_err());
}
