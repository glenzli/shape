//! Physical execution owner for deterministic alpha-aware `image.unsharp_mask`.

use image::{ColorType, ImageFormat, metadata::Orientation};
use shape_domain::{IMAGE_UNSHARP_MASK_PARAMETERS_REVISION, PreparedImageUnsharpMask};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement `image.unsharp_mask`.
pub const RASTER_UNSHARP_MASK_CAPABILITY: &str = "image.raster.unsharp_mask";

const INVALID_INPUT_CODE: &str = "invalid_unsharp_mask_input";
const INVALID_PLAN_CODE: &str = "invalid_unsharp_mask_plan";
const CONTRACT_MISMATCH_CODE: &str = "unsharp_mask_contract_mismatch";
const AMOUNT_SCALE: i64 = 1_000;
const CHANNEL_MAX: i64 = u16::MAX as i64;

/// Decoded RGBA plus two reusable linear-premultiplied buffers.
pub const RASTER_UNSHARP_MASK_WORKING_BYTES_PER_PIXEL: u64 = 4 + 8 + 8;

/// Portable fixed-point unsharp-mask executor bound to one prepared plan.
#[derive(Debug)]
pub struct RasterUnsharpMaskExecutor {
    identity: ExecutorIdentity,
    prepared: PreparedImageUnsharpMask,
}

impl RasterUnsharpMaskExecutor {
    /// Creates a versioned executor from one already-validated plan.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new(prepared: PreparedImageUnsharpMask) -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.unsharp-mask",
                env!("CARGO_PKG_VERSION"),
                IMAGE_UNSHARP_MASK_PARAMETERS_REVISION,
            )?,
            prepared,
        })
    }
}

impl Executor for RasterUnsharpMaskExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_UNSHARP_MASK_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_UNSHARP_MASK_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster unsharp-mask executor does not support this capability",
            ));
        }
        execute_unsharp_mask(request, &self.prepared)
    }
}

fn execute_unsharp_mask(
    request: &ExecutionRequest,
    prepared: &PreparedImageUnsharpMask,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster unsharp mask requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster unsharp-mask input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster unsharp mask requires canonical Shape PNG input and output",
        ));
    }
    if !request.instruction.is_empty() {
        return Err(failure(
            INVALID_PLAN_CODE,
            "raster unsharp-mask parameters must come from the prepared plan",
        ));
    }

    let mut decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
        || decoded.icc.is_some()
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster unsharp-mask input must be normalized RGBA8 top-left sRGB without an embedded profile",
        ));
    }
    let source_contract = contract(decoded.image.width(), decoded.image.height(), None)?;
    if &source_contract != prepared.source_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "materialized raster does not match the prepared source contract",
        ));
    }

    let width = decoded.image.width();
    let height = decoded.image.height();
    let parameters = prepared.parameters();
    let blurred = super::pixels::gaussian_blur_clamped(
        super::pixels::linear_premultiplied(&decoded.image),
        width,
        height,
        parameters.radius(),
    );
    for (target, blurred) in decoded.image.pixels_mut().zip(blurred) {
        let original = super::pixels::LinearPremultiplied::from_straight_srgb(*target);
        *target = sharpen(
            original,
            blurred,
            parameters.amount_milli(),
            parameters.threshold(),
        )
        .to_straight_srgb();
    }

    let output_contract = contract(width, height, None)?;
    if &output_contract != prepared.output_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "unsharp-mask raster does not match the prepared output contract",
        ));
    }
    let bytes = encode_png(&decoded.image, None)?;
    Ok(output(bytes, output_contract))
}

fn sharpen(
    original: super::pixels::LinearPremultiplied,
    blurred: super::pixels::LinearPremultiplied,
    amount_milli: u16,
    threshold: u8,
) -> super::pixels::LinearPremultiplied {
    let threshold = i64::from(threshold) * 257;
    let maximum_difference = (0..3)
        .map(|index| (i64::from(original.0[index]) - i64::from(blurred.0[index])).abs())
        .max()
        .expect("RGB has three channels");
    if maximum_difference <= threshold {
        return original;
    }

    let alpha = i64::from(original.alpha());
    let mut output = original.0;
    for (index, channel) in output[..3].iter_mut().enumerate() {
        let difference = i64::from(original.0[index]) - i64::from(blurred.0[index]);
        let scaled = round_signed(difference * i64::from(amount_milli), AMOUNT_SCALE);
        let sharpened = (i64::from(original.0[index]) + scaled).clamp(0, alpha.min(CHANNEL_MAX));
        *channel = u16::try_from(sharpened).expect("clamped sharpened channel remains u16");
    }
    output[3] = original.alpha();
    super::pixels::LinearPremultiplied(output)
}

fn round_signed(numerator: i64, denominator: i64) -> i64 {
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        -((-numerator + denominator / 2) / denominator)
    }
}

#[cfg(test)]
mod tests;
