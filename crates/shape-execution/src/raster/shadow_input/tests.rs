use super::*;
use image::RgbaImage;

fn profile() -> Vec<u8> {
    // Complete Qt QColorSpace::SRgb ICC bytes observed in the actual Shadow return.
    const HEX: &str = "000001e000000000044000006d6e74725247422058595a2000000000000000000000000061637370000000000000000000000000000000000000000000000000000000000000f6d6000100000000d32d5174060b00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a7258595a000000fc000000146758595a00000110000000146258595a0000012400000014777470740000013800000014637072740000014c0000002263686164000001700000002c725452430000019c00000020675452430000019c00000020625452430000019c0000002064657363000001bc0000002458595a200000000000006f9f000038f40000039158595a2000000000000062960000b787000018dc58595a2000000000000024a100000f850000b6d358595a20000000000000f34f00010000000116c26d6c756300000000000000010000000c656e5553000000060000001c004e002f00410000736633320000000000010c40000005ddfffff32a000007920000fd8ffffffba2fffffda3000003da0000c087706172610000000000030000000266660000f2a700000d59000013d000000a5b6d6c756300000000000000010000000c656e5553000000080000001c0073005200470042";
    HEX.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn pixels(alpha: u8) -> RgbaImage {
    RgbaImage::from_raw(
        2,
        2,
        vec![
            10, 80, 200, alpha, 0, 127, 255, alpha, 255, 20, 70, alpha, 31, 67, 129, alpha,
        ],
    )
    .unwrap()
}

#[test]
fn exact_known_profile_changes_only_working_encoding_and_preserves_every_rgba_sample() {
    let image = pixels(255);
    let source = encode_png(&image, Some(&profile())).unwrap();
    let (working, record) = materialize(&source).unwrap();
    assert_ne!(working, source);
    assert!(matches!(
        record,
        MaterialReturnNormalization::KnownSrgbIccInput { .. }
    ));
    let MaterialReturnNormalization::KnownSrgbIccInput {
        profile_digest,
        materialized_digest,
    } = record
    else {
        unreachable!()
    };
    assert_eq!(profile_digest.to_string(), QT_SRGB_PROFILE);
    assert_eq!(materialized_digest, ContentDigest::from_bytes(&working));
    let decoded =
        decode_bounded(&working, ImageFormat::Png, MAX_BYTES, 16_384, 16_777_216).unwrap();
    assert!(decoded.icc.is_none());
    assert_eq!(decoded.image.as_raw(), image.as_raw());
    assert_eq!(
        materialize(&working).unwrap(),
        (working, MaterialReturnNormalization::Canonical)
    );
    // The original keeps its complete ICC bytes.
    assert_eq!(
        decode_bounded(&source, ImageFormat::Png, MAX_BYTES, 16_384, 16_777_216)
            .unwrap()
            .icc
            .unwrap(),
        profile()
    );
}

#[test]
fn one_profile_byte_change_transparency_and_non_png_fail_closed() {
    let mut arbitrary = profile();
    arbitrary[479] ^= 1;
    let source = encode_png(&pixels(255), Some(&arbitrary)).unwrap();
    assert!(materialize(&source).is_err());
    assert!(materialize(&encode_png(&pixels(254), Some(&profile())).unwrap()).is_err());
    assert!(materialize(b"not a PNG").is_err());
    let ordinary = encode_png(&pixels(255), None).unwrap();
    assert_eq!(
        materialize(&ordinary).unwrap(),
        (ordinary, MaterialReturnNormalization::Canonical)
    );
}
