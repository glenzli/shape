//! Physical execution owner for the deterministic `image.resize` Operator.

use image::{ColorType, ImageFormat, imageops::FilterType, metadata::Orientation};
use shape_domain::{IMAGE_RESIZE_PARAMETERS_REVISION, PreparedImageResize, RasterResizeResampling};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement the creative `image.resize` Operator.
pub const RASTER_RESIZE_CAPABILITY: &str = "image.raster.resize";

const INVALID_INPUT_CODE: &str = "invalid_resize_input";
const INVALID_PLAN_CODE: &str = "invalid_resize_plan";
const CONTRACT_MISMATCH_CODE: &str = "resize_contract_mismatch";
const NO_OP_CODE: &str = "resize_no_op";

/// Portable deterministic pixel executor bound to one prepared resize plan.
#[derive(Debug)]
pub struct RasterResizeExecutor {
    identity: ExecutorIdentity,
    prepared: PreparedImageResize,
}

impl RasterResizeExecutor {
    /// Creates a versioned executor that consumes one already-validated plan.
    ///
    /// The plan remains opaque to request serialization, so creative
    /// orchestration prepares dimensions and policy exactly once before the
    /// canonical source PNG enters physical execution.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new(prepared: PreparedImageResize) -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.resize",
                env!("CARGO_PKG_VERSION"),
                IMAGE_RESIZE_PARAMETERS_REVISION,
            )?,
            prepared,
        })
    }
}

impl Executor for RasterResizeExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_RESIZE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_RESIZE_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster resize executor does not support this capability",
            ));
        }
        execute_resize(request, &self.prepared)
    }
}

fn execute_resize(
    request: &ExecutionRequest,
    prepared: &PreparedImageResize,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster resize requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster resize input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster resize requires canonical Shape PNG input and output",
        ));
    }
    if !request.instruction.is_empty() {
        return Err(failure(
            INVALID_PLAN_CODE,
            "raster resize parameters must come from the prepared plan",
        ));
    }
    if prepared.is_identity() {
        return Err(failure(
            NO_OP_CODE,
            "raster resize plan would not change pixel dimensions",
        ));
    }

    let decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster resize input is not a normalized RGBA8 top-left Shape PNG",
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

    let dimensions = prepared.output_dimensions();
    let resized = image::imageops::resize(
        &decoded.image,
        dimensions.width(),
        dimensions.height(),
        filter_type(prepared.parameters().resampling()),
    );
    let output_contract = contract(resized.width(), resized.height(), decoded.icc.as_deref())?;
    if &output_contract != prepared.output_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "resized raster does not match the prepared output contract",
        ));
    }
    let bytes = encode_png(&resized, decoded.icc.as_deref())?;
    Ok(output(bytes, output_contract))
}

const fn filter_type(resampling: RasterResizeResampling) -> FilterType {
    match resampling {
        RasterResizeResampling::Nearest => FilterType::Nearest,
        RasterResizeResampling::Triangle => FilterType::Triangle,
        RasterResizeResampling::CatmullRom => FilterType::CatmullRom,
        RasterResizeResampling::Lanczos3 => FilterType::Lanczos3,
    }
}

#[cfg(test)]
mod tests;
