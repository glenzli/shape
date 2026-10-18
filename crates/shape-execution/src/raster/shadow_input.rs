//! Exact-profile working-copy materialization for Shadow's independent PNG consumer.
//! This is an identity on RGB8 samples, never a general ICC color conversion.
use super::{decode_bounded, encode_png, failure};
use crate::ExecutionFailure;
use image::{ColorType, ImageFormat, metadata::Orientation};
use shape_domain::{ContentDigest, MaterialReturnNormalization};

const MAX_BYTES: usize = 64 * 1024 * 1024;
// QColorSpace::SRgb's complete 480-byte ICC profile emitted by the verified Shadow exporter.
// SHA-256: ef087eece0b3a8ca7b79e56b8e82578bea636870a4bd547c44c25722605ff838.
// Pin all bytes through BLAKE3; descriptions, tag names or declared primaries are insufficient.
const QT_SRGB_PROFILE: &str = "fc9a5af37a9933652f8992e652b8a4cd303eb58318240f1d97dff993a34e567a";

pub(super) fn materialize(
    bytes: &[u8],
) -> Result<(Vec<u8>, MaterialReturnNormalization), ExecutionFailure> {
    if image::guess_format(bytes).ok() != Some(ImageFormat::Png) {
        return Err(invalid());
    }
    let source = decode_bounded(bytes, ImageFormat::Png, MAX_BYTES, 16_384, 16_777_216)?;
    if !matches!(source.source_color_type, ColorType::Rgb8 | ColorType::Rgba8)
        || source.source_orientation != Orientation::NoTransforms
        || source.image.pixels().any(|p| p.0[3] != 255)
    {
        return Err(invalid());
    }
    let Some(profile) = source.icc else {
        return Ok((bytes.to_vec(), MaterialReturnNormalization::Canonical));
    };
    let profile_digest = ContentDigest::from_bytes(&profile);
    if profile.len() != 480 || profile_digest.to_string() != QT_SRGB_PROFILE {
        return Err(invalid());
    }
    let materialized = encode_png(&source.image, None)?;
    if materialized.len() > MAX_BYTES {
        return Err(invalid());
    }
    let materialized_digest = ContentDigest::from_bytes(&materialized);
    Ok((
        materialized,
        MaterialReturnNormalization::KnownSrgbIccInput {
            profile_digest,
            materialized_digest,
        },
    ))
}
fn invalid() -> ExecutionFailure {
    failure(
        "shadow_input_unsupported",
        "Shadow input requires opaque RGB8 sRGB pixels; arbitrary ICC profiles are preserved and unsupported",
    )
}

#[cfg(test)]
mod tests;
