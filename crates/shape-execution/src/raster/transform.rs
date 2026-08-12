//! Physical execution owner for deterministic `image.transform` operations.

use image::{ColorType, ImageFormat, imageops, metadata::Orientation};
use shape_domain::{IMAGE_TRANSFORM_PARAMETERS_REVISION, PreparedImageTransform, RasterTransform};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement `image.transform`.
pub const RASTER_TRANSFORM_CAPABILITY: &str = "image.raster.transform";

const INVALID_INPUT_CODE: &str = "invalid_transform_input";
const INVALID_PLAN_CODE: &str = "invalid_transform_plan";
const CONTRACT_MISMATCH_CODE: &str = "transform_contract_mismatch";

/// Portable lossless pixel-rearrangement executor bound to one prepared plan.
#[derive(Debug)]
pub struct RasterTransformExecutor {
    identity: ExecutorIdentity,
    prepared: PreparedImageTransform,
}

impl RasterTransformExecutor {
    /// Creates a versioned executor from one already-validated plan.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new(prepared: PreparedImageTransform) -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.transform",
                env!("CARGO_PKG_VERSION"),
                IMAGE_TRANSFORM_PARAMETERS_REVISION,
            )?,
            prepared,
        })
    }
}

impl Executor for RasterTransformExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_TRANSFORM_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_TRANSFORM_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster transform executor does not support this capability",
            ));
        }
        execute_transform(request, &self.prepared)
    }
}

fn execute_transform(
    request: &ExecutionRequest,
    prepared: &PreparedImageTransform,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster transform requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster transform input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster transform requires canonical Shape PNG input and output",
        ));
    }
    if !request.instruction.is_empty() {
        return Err(failure(
            INVALID_PLAN_CODE,
            "raster transform parameters must come from the prepared plan",
        ));
    }

    let decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster transform input is not a normalized RGBA8 top-left Shape PNG",
        ));
    }
    let source_contract = contract(
        decoded.image.width(),
        decoded.image.height(),
        decoded.icc.as_deref(),
    )?;
    if &source_contract != prepared.source_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "materialized raster does not match the prepared source contract",
        ));
    }

    let transformed = match prepared.parameters() {
        RasterTransform::Rotate90Clockwise => imageops::rotate90(&decoded.image),
        RasterTransform::Rotate180 => imageops::rotate180(&decoded.image),
        RasterTransform::Rotate270Clockwise => imageops::rotate270(&decoded.image),
        RasterTransform::FlipHorizontal => imageops::flip_horizontal(&decoded.image),
        RasterTransform::FlipVertical => imageops::flip_vertical(&decoded.image),
    };
    let output_contract = contract(
        transformed.width(),
        transformed.height(),
        decoded.icc.as_deref(),
    )?;
    if &output_contract != prepared.output_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "transformed raster does not match the prepared output contract",
        ));
    }
    let bytes = encode_png(&transformed, decoded.icc.as_deref())?;
    Ok(output(bytes, output_contract))
}

#[cfg(test)]
mod tests;
