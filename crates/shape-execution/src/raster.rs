//! Bounded PNG/JPEG normalization and canonical raster materialization.

mod crop;
mod resize;

use std::io::Cursor;

use image::{
    ColorType, DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, Limits,
    RgbaImage, codecs::png::PngEncoder, metadata::Orientation,
};
use shape_domain::{
    ArtifactContentContract, ContentDigest, ImageColorProfile, ImageRasterContract,
};

use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

pub use crop::{RASTER_CROP_CAPABILITY, RasterCropExecutor};
pub use resize::{RASTER_RESIZE_CAPABILITY, RasterResizeExecutor};

pub const RASTER_IMPORT_CAPABILITY: &str = "image.raster.import";
const RASTER_MEDIA_TYPE: &str = "image/png";
const RASTER_CONTRACT_REVISION: &str = "20260811.1";
const MAX_ENCODED_BYTES: usize = 128 * 1024 * 1024;
const MAX_CANONICAL_BYTES: usize = 256 * 1024 * 1024;
const MAX_DIMENSION: u32 = 32_768;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_DECODE_ALLOC: u64 = 320 * 1024 * 1024;
const MAX_ICC_BYTES: usize = 4 * 1024 * 1024;

/// Portable import executor and compatibility entry for the raster slice.
#[derive(Debug)]
pub struct RasterExecutor {
    identity: ExecutorIdentity,
}

impl RasterExecutor {
    /// Creates the versioned built-in raster executor.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new() -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster",
                env!("CARGO_PKG_VERSION"),
                RASTER_CONTRACT_REVISION,
            )?,
        })
    }
}

impl Executor for RasterExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        matches!(
            capability.as_str(),
            RASTER_IMPORT_CAPABILITY | RASTER_CROP_CAPABILITY
        )
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        match request.capability.as_str() {
            RASTER_IMPORT_CAPABILITY => execute_import(request),
            RASTER_CROP_CAPABILITY => crop::execute_crop(request),
            _ => Err(failure(
                "unsupported_raster_capability",
                "built-in raster executor does not support this capability",
            )),
        }
    }
}

fn execute_import(request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
    if !request.inputs.is_empty() || request.instruction.len() > MAX_ENCODED_BYTES {
        return Err(failure(
            "invalid_raster_import",
            "raster import payload is missing or exceeds the local limit",
        ));
    }
    let format = image::guess_format(&request.instruction).map_err(|_| {
        failure(
            "unsupported_image_format",
            "only PNG and JPEG are supported",
        )
    })?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err(failure(
            "unsupported_image_format",
            "only PNG and JPEG are supported",
        ));
    }
    let decoded = decode(&request.instruction, format)?;
    let contract = contract(
        decoded.image.width(),
        decoded.image.height(),
        decoded.icc.as_deref(),
    )?;
    let bytes = encode_png(&decoded.image, decoded.icc.as_deref())?;
    Ok(output(bytes, contract))
}

struct DecodedRaster {
    image: RgbaImage,
    icc: Option<Vec<u8>>,
    source_color_type: ColorType,
    source_orientation: Orientation,
}

fn decode(bytes: &[u8], format: ImageFormat) -> Result<DecodedRaster, ExecutionFailure> {
    if bytes.is_empty() || bytes.len() > MAX_ENCODED_BYTES {
        return Err(failure(
            "image_too_large",
            "encoded image exceeds the local import limit",
        ));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| failure("image_decode_failed", "image could not be decoded safely"))?;
    let (width, height) = decoder.dimensions();
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| failure("image_too_large", "image dimensions exceed the local limit"))?;
    if pixels > MAX_PIXELS {
        return Err(failure(
            "image_too_large",
            "image pixel count exceeds the local import limit",
        ));
    }
    let source_color_type = decoder.color_type();
    if !matches!(
        source_color_type,
        ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8
    ) {
        return Err(failure(
            "unsupported_bit_depth",
            "the first raster slice supports 8-bit PNG and JPEG images",
        ));
    }
    let orientation = decoder.orientation().map_err(|_| {
        failure(
            "invalid_orientation",
            "image orientation metadata is invalid",
        )
    })?;
    let icc = decoder
        .icc_profile()
        .map_err(|_| failure("invalid_color_profile", "image color profile is invalid"))?;
    if icc
        .as_ref()
        .is_some_and(|profile| profile.len() > MAX_ICC_BYTES)
    {
        return Err(failure(
            "color_profile_too_large",
            "embedded color profile exceeds the local limit",
        ));
    }
    let mut image = DynamicImage::from_decoder(decoder)
        .map_err(|_| failure("image_decode_failed", "image could not be decoded safely"))?;
    image.apply_orientation(orientation);
    Ok(DecodedRaster {
        image: image.to_rgba8(),
        icc,
        source_color_type,
        source_orientation: orientation,
    })
}

fn contract(
    width: u32,
    height: u32,
    icc: Option<&[u8]>,
) -> Result<ImageRasterContract, ExecutionFailure> {
    let color_profile = icc.map_or(ImageColorProfile::Srgb, |profile| {
        ImageColorProfile::EmbeddedIcc {
            digest: ContentDigest::from_bytes(profile),
        }
    });
    ImageRasterContract::rgba8(width, height, color_profile).map_err(|_| {
        failure(
            "invalid_raster_contract",
            "decoded image does not satisfy the raster contract",
        )
    })
}

fn encode_png(image: &RgbaImage, icc: Option<&[u8]>) -> Result<Vec<u8>, ExecutionFailure> {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    if let Some(profile) = icc {
        encoder.set_icc_profile(profile.to_vec()).map_err(|_| {
            failure(
                "color_profile_encode_failed",
                "embedded color profile could not be preserved",
            )
        })?;
    }
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|_| failure("image_encode_failed", "canonical PNG could not be encoded"))?;
    if bytes.len() > MAX_CANONICAL_BYTES {
        return Err(failure(
            "image_too_large",
            "canonical PNG exceeds the local materialization limit",
        ));
    }
    Ok(bytes)
}

fn output(bytes: Vec<u8>, contract: ImageRasterContract) -> ExecutionOutput {
    ExecutionOutput {
        bytes,
        media_type: RASTER_MEDIA_TYPE.to_owned(),
        executor_job_id: None,
        external_provenance: None,
        content_contract: Some(ArtifactContentContract::ImageRaster(contract)),
    }
}

fn failure(code: &'static str, message: &'static str) -> ExecutionFailure {
    ExecutionFailure::new(code, message, false)
}

#[cfg(test)]
mod tests;
