//! Physical execution owner for deterministic alpha-aware `image.blur`.

use image::{ColorType, ImageFormat, metadata::Orientation};
use shape_domain::{IMAGE_BLUR_PARAMETERS_REVISION, PreparedImageBlur};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement `image.blur`.
pub const RASTER_BLUR_CAPABILITY: &str = "image.raster.blur";

const INVALID_INPUT_CODE: &str = "invalid_blur_input";
const INVALID_PLAN_CODE: &str = "invalid_blur_plan";
const CONTRACT_MISMATCH_CODE: &str = "blur_contract_mismatch";

/// Portable fixed-point blur executor bound to one prepared plan.
#[derive(Debug)]
pub struct RasterBlurExecutor {
    identity: ExecutorIdentity,
    prepared: PreparedImageBlur,
}

impl RasterBlurExecutor {
    /// Creates a versioned executor from one already-validated plan.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new(prepared: PreparedImageBlur) -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.blur",
                env!("CARGO_PKG_VERSION"),
                IMAGE_BLUR_PARAMETERS_REVISION,
            )?,
            prepared,
        })
    }
}

impl Executor for RasterBlurExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_BLUR_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_BLUR_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster blur executor does not support this capability",
            ));
        }
        execute_blur(request, &self.prepared)
    }
}

fn execute_blur(
    request: &ExecutionRequest,
    prepared: &PreparedImageBlur,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster blur requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster blur input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster blur requires canonical Shape PNG input and output",
        ));
    }
    if !request.instruction.is_empty() {
        return Err(failure(
            INVALID_PLAN_CODE,
            "raster blur parameters must come from the prepared plan",
        ));
    }

    let mut decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
        || decoded.icc.is_some()
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster blur input must be normalized RGBA8 top-left sRGB without an embedded profile",
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
    let pixels = super::pixels::linear_premultiplied(&decoded.image);
    let pixels =
        super::pixels::gaussian_blur_clamped(pixels, width, height, prepared.parameters().radius());
    super::pixels::write_straight_srgb(&mut decoded.image, pixels);
    let output_contract = contract(width, height, None)?;
    if &output_contract != prepared.output_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "blurred raster does not match the prepared output contract",
        ));
    }
    let bytes = encode_png(&decoded.image, None)?;
    Ok(output(bytes, output_contract))
}

#[cfg(test)]
mod tests;
