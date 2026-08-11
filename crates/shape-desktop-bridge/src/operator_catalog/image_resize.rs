//! Versioned authored-state contract for the built-in `image.resize` consumer.

use shape_domain::{
    OperatorConfigurationSchemaId, RasterResize, RasterResizeAspectPolicy, RasterResizeDimensions,
    RasterResizeResampling, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::IMAGE_RESIZE_OPERATOR;

const IMAGE_RESIZE_DRAFT_SCHEMA: &str = "shape.operator-draft.image-resize@20260811.1";

pub(crate) fn configuration_for_image_resize(
    target_width: u32,
    target_height: u32,
    aspect_policy_key: &str,
    resampling_key: &str,
) -> Result<WorkingOperatorConfiguration, String> {
    let resize = RasterResize::new(
        RasterResizeDimensions::new(target_width, target_height)
            .map_err(|error| error.to_string())?,
        aspect_policy_from_key(aspect_policy_key)?,
        resampling_from_key(resampling_key)?,
    );
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(IMAGE_RESIZE_DRAFT_SCHEMA)
            .map_err(|error| error.to_string())?,
        serde_json::to_string(&resize).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn image_resize_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<Option<RasterResize>, String> {
    if draft.operator_type().as_str() != IMAGE_RESIZE_OPERATOR {
        return Ok(None);
    }
    draft.configuration().map(decode).transpose()
}

pub(crate) const fn aspect_policy_key(policy: RasterResizeAspectPolicy) -> &'static str {
    match policy {
        RasterResizeAspectPolicy::Stretch => "stretch",
        RasterResizeAspectPolicy::FitWithin => "fit_within",
    }
}

pub(crate) const fn resampling_key(resampling: RasterResizeResampling) -> &'static str {
    match resampling {
        RasterResizeResampling::Nearest => "nearest",
        RasterResizeResampling::Triangle => "triangle",
        RasterResizeResampling::CatmullRom => "catmull_rom",
        RasterResizeResampling::Lanczos3 => "lanczos3",
    }
}

pub(super) fn validate_image_resize_configuration(
    configuration: Option<&WorkingOperatorConfiguration>,
) -> Result<(), String> {
    let configuration =
        configuration.ok_or_else(|| "image resize draft configuration is required".to_owned())?;
    decode(configuration).map(|_| ())
}

fn decode(configuration: &WorkingOperatorConfiguration) -> Result<RasterResize, String> {
    if configuration.schema().as_str() != IMAGE_RESIZE_DRAFT_SCHEMA {
        return Err("image resize draft configuration schema is unsupported".to_owned());
    }
    serde_json::from_str(configuration.json())
        .map_err(|_| "image resize draft configuration does not match its exact schema".to_owned())
}

fn aspect_policy_from_key(key: &str) -> Result<RasterResizeAspectPolicy, String> {
    match key {
        "stretch" => Ok(RasterResizeAspectPolicy::Stretch),
        "fit_within" => Ok(RasterResizeAspectPolicy::FitWithin),
        _ => Err("image resize aspect policy is unsupported".to_owned()),
    }
}

fn resampling_from_key(key: &str) -> Result<RasterResizeResampling, String> {
    match key {
        "nearest" => Ok(RasterResizeResampling::Nearest),
        "triangle" => Ok(RasterResizeResampling::Triangle),
        "catmull_rom" => Ok(RasterResizeResampling::CatmullRom),
        "lanczos3" => Ok(RasterResizeResampling::Lanczos3),
        _ => Err("image resize resampling kernel is unsupported".to_owned()),
    }
}

#[cfg(test)]
mod tests;
