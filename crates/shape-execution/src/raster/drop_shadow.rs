//! Physical execution owner for deterministic flattened `image.drop_shadow`.

use image::{ColorType, ImageFormat, Rgba, RgbaImage, metadata::Orientation};
use shape_domain::{IMAGE_DROP_SHADOW_PARAMETERS_REVISION, PreparedImageDropShadow};

use super::{RASTER_MEDIA_TYPE, contract, decode, encode_png, failure, output};
use crate::{
    CapabilityId, ExecutionError, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor,
    ExecutorIdentity,
};

/// Physical capability selected to implement `image.drop_shadow`.
pub const RASTER_DROP_SHADOW_CAPABILITY: &str = "image.raster.drop_shadow";

const INVALID_INPUT_CODE: &str = "invalid_drop_shadow_input";
const INVALID_PLAN_CODE: &str = "invalid_drop_shadow_plan";
const CONTRACT_MISMATCH_CODE: &str = "drop_shadow_contract_mismatch";

/// Portable fixed-point shadow executor bound to one prepared plan.
#[derive(Debug)]
pub struct RasterDropShadowExecutor {
    identity: ExecutorIdentity,
    prepared: PreparedImageDropShadow,
}

impl RasterDropShadowExecutor {
    /// Creates a versioned executor from one already-validated plan.
    ///
    /// # Errors
    ///
    /// Returns an error only if its static identity is malformed.
    pub fn new(prepared: PreparedImageDropShadow) -> Result<Self, ExecutionError> {
        Ok(Self {
            identity: ExecutorIdentity::new(
                "shape.builtin.raster.drop-shadow",
                env!("CARGO_PKG_VERSION"),
                IMAGE_DROP_SHADOW_PARAMETERS_REVISION,
            )?,
            prepared,
        })
    }
}

impl Executor for RasterDropShadowExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == RASTER_DROP_SHADOW_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != RASTER_DROP_SHADOW_CAPABILITY {
            return Err(failure(
                "unsupported_raster_capability",
                "built-in raster drop-shadow executor does not support this capability",
            ));
        }
        execute_drop_shadow(request, &self.prepared)
    }
}

fn execute_drop_shadow(
    request: &ExecutionRequest,
    prepared: &PreparedImageDropShadow,
) -> Result<ExecutionOutput, ExecutionFailure> {
    let [input] = request.inputs.as_slice() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster drop shadow requires exactly one materialized input",
        ));
    };
    let Some(bytes) = input.bytes() else {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster drop-shadow input is not materialized",
        ));
    };
    if input.content().media_type != RASTER_MEDIA_TYPE
        || request.output_media_type != RASTER_MEDIA_TYPE
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster drop shadow requires canonical Shape PNG input and output",
        ));
    }
    if !request.instruction.is_empty() {
        return Err(failure(
            INVALID_PLAN_CODE,
            "raster drop-shadow parameters must come from the prepared plan",
        ));
    }

    let decoded = decode(bytes, ImageFormat::Png)?;
    if decoded.source_color_type != ColorType::Rgba8
        || decoded.source_orientation != Orientation::NoTransforms
        || decoded.icc.is_some()
    {
        return Err(failure(
            INVALID_INPUT_CODE,
            "raster drop-shadow input must be normalized RGBA8 top-left sRGB without an embedded profile",
        ));
    }
    let source_contract = contract(decoded.image.width(), decoded.image.height(), None)?;
    if &source_contract != prepared.source_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "materialized raster does not match the prepared source contract",
        ));
    }

    let image = render_drop_shadow(&decoded.image, prepared)?;
    let output_contract = contract(image.width(), image.height(), None)?;
    if &output_contract != prepared.output_contract() {
        return Err(failure(
            CONTRACT_MISMATCH_CODE,
            "drop-shadow raster does not match the prepared output contract",
        ));
    }
    let bytes = encode_png(&image, None)?;
    Ok(output(bytes, output_contract))
}

fn render_drop_shadow(
    source: &RgbaImage,
    prepared: &PreparedImageDropShadow,
) -> Result<RgbaImage, ExecutionFailure> {
    let output_width = prepared.output_contract().width;
    let output_height = prepared.output_contract().height;
    let output_len =
        usize::try_from(u64::from(output_width) * u64::from(output_height)).map_err(|_| {
            failure(
                INVALID_PLAN_CODE,
                "drop-shadow output allocation is invalid",
            )
        })?;
    let source_width = usize::try_from(source.width())
        .map_err(|_| failure(INVALID_INPUT_CODE, "source raster width is invalid"))?;
    let source_height = usize::try_from(source.height())
        .map_err(|_| failure(INVALID_INPUT_CODE, "source raster height is invalid"))?;
    let output_width_usize = usize::try_from(output_width)
        .map_err(|_| failure(INVALID_PLAN_CODE, "drop-shadow output width is invalid"))?;
    let source_pixels = super::pixels::linear_premultiplied(source);
    let mut shadow_mask = vec![super::pixels::LinearPremultiplied::default(); output_len];
    let (shadow_x, shadow_y) = prepared.shadow_origin();
    let shadow_x = usize::try_from(shadow_x)
        .map_err(|_| failure(INVALID_PLAN_CODE, "drop-shadow x origin is invalid"))?;
    let shadow_y = usize::try_from(shadow_y)
        .map_err(|_| failure(INVALID_PLAN_CODE, "drop-shadow y origin is invalid"))?;
    for y in 0..source_height {
        for x in 0..source_width {
            shadow_mask[(shadow_y + y) * output_width_usize + shadow_x + x] =
                source_pixels[y * source_width + x].alpha_only();
        }
    }
    let parameters = prepared.parameters();
    let shadow_mask = super::pixels::gaussian_blur_transparent(
        shadow_mask,
        output_width,
        output_height,
        parameters.blur_radius(),
    );
    let color = parameters.color();
    let color = Rgba([color.red(), color.green(), color.blue(), color.alpha()]);
    let mut composed = shadow_mask
        .into_iter()
        .map(|coverage| super::pixels::LinearPremultiplied::tint(color, coverage.alpha()))
        .collect::<Vec<_>>();

    let (source_x, source_y) = prepared.source_origin();
    let source_x = usize::try_from(source_x)
        .map_err(|_| failure(INVALID_PLAN_CODE, "source x origin is invalid"))?;
    let source_y = usize::try_from(source_y)
        .map_err(|_| failure(INVALID_PLAN_CODE, "source y origin is invalid"))?;
    for y in 0..source_height {
        for x in 0..source_width {
            let target = (source_y + y) * output_width_usize + source_x + x;
            composed[target] =
                super::pixels::source_over(composed[target], source_pixels[y * source_width + x]);
        }
    }

    Ok(super::pixels::straight_srgb_image(
        composed,
        output_width,
        output_height,
    ))
}

#[cfg(test)]
mod tests;
