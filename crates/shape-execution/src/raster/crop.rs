//! Physical execution owner for the deterministic `image.crop` Operator.

use image::{ColorType, ImageFormat, metadata::Orientation};
use shape_domain::{IMAGE_CROP_PARAMETERS_REVISION, ImageCropOperator, RasterCrop};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement the creative `image.crop` Operator.
pub const RASTER_CROP_CAPABILITY: &str = "image.raster.crop";

const INVALID_INPUT_CODE: &str = "invalid_crop_input";
const INVALID_PARAMETERS_CODE: &str = "invalid_crop_parameters";
const OUT_OF_BOUNDS_CODE: &str = "crop_out_of_bounds";

/// Portable, deterministic pixel executor for one canonical raster crop.
#[derive(Debug)]
pub struct RasterCropExecutor {
    identity: ExecutorIdentity,
}

impl RasterCropExecutor {
    /// Creates the first versioned raster-crop executor.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new() -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.crop",
                env!("CARGO_PKG_VERSION"),
                IMAGE_CROP_PARAMETERS_REVISION,
            )?,
        })
    }
}

impl Executor for RasterCropExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_CROP_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_CROP_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster crop executor does not support this capability",
            ));
        }
        execute_crop(request)
    }
}

pub(super) fn execute_crop(
    request: &ExecutionRequest,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster crop requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster crop input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster crop requires canonical Shape PNG input and output",
        ));
    }
    let parameters: RasterCrop = serde_json::from_slice(&request.instruction).map_err(|_| {
        failure(
            INVALID_PARAMETERS_CODE,
            "raster crop parameters do not match the frozen contract",
        )
    })?;
    let decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster crop input is not a normalized RGBA8 top-left Shape PNG",
        ));
    }
    let source_contract = contract(
        decoded.image.width(),
        decoded.image.height(),
        decoded.icc.as_deref(),
    )?;
    let prepared = ImageCropOperator::prepare(parameters, &source_contract).map_err(|_| {
        failure(
            OUT_OF_BOUNDS_CODE,
            "raster crop exceeds the accepted source image",
        )
    })?;
    let parameters = prepared.parameters();
    let cropped = image::imageops::crop_imm(
        &decoded.image,
        parameters.x,
        parameters.y,
        parameters.width,
        parameters.height,
    )
    .to_image();
    let bytes = encode_png(&cropped, decoded.icc.as_deref())?;
    Ok(output(bytes, prepared.output_contract().clone()))
}

#[cfg(test)]
mod tests;
