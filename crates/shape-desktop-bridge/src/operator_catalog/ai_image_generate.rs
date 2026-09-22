//! Versioned authored draft for the zero-input `image.generate` consumer.

use serde::{Deserialize, Serialize};
use shape_domain::{
    AI_IMAGE_GENERATE_OPERATOR_TYPE, AiImageGenerateParameters, AiImageOutputCanvas, Constraint,
    OperatorConfigurationSchemaId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

const AI_IMAGE_GENERATE_DRAFT_SCHEMA: &str = "shape.operator-draft.ai-image-generate@20260811.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AiImageGenerateDraftWire {
    instruction: String,
    output: AiImageOutputCanvas,
    candidate_count: u8,
    constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiImageGenerateDraftState {
    pub(crate) instruction: String,
    pub(crate) output: AiImageOutputCanvas,
    pub(crate) candidate_count: u8,
}

pub(crate) fn configuration_for_ai_image_generate(
    instruction: &str,
    output_width: u32,
    output_height: u32,
    candidate_count: u8,
) -> Result<WorkingOperatorConfiguration, String> {
    if !instruction.is_empty() && instruction.trim().is_empty() {
        return Err("AI image instruction cannot contain only whitespace".to_owned());
    }
    let output =
        AiImageOutputCanvas::new(output_width, output_height).map_err(|error| error.to_string())?;
    validate_parameters_or_empty(instruction, output, candidate_count, &[])?;
    let wire = AiImageGenerateDraftWire {
        instruction: instruction.to_owned(),
        output,
        candidate_count,
        constraints: Vec::new(),
    };
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(AI_IMAGE_GENERATE_DRAFT_SCHEMA)
            .map_err(|error| error.to_string())?,
        serde_json::to_string(&wire).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn ai_image_generate_state_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<Option<AiImageGenerateDraftState>, String> {
    if draft.operator_type().as_str() != AI_IMAGE_GENERATE_OPERATOR_TYPE {
        return Ok(None);
    }
    let wire = decode(
        draft
            .configuration()
            .ok_or_else(|| "AI image generation draft configuration is required".to_owned())?,
    )?;
    Ok(Some(AiImageGenerateDraftState {
        instruction: wire.instruction,
        output: wire.output,
        candidate_count: wire.candidate_count,
    }))
}

pub(crate) fn ai_image_generate_parameters_from_draft(
    draft: &WorkingOperatorDraft,
) -> Result<Option<AiImageGenerateParameters>, String> {
    if draft.operator_type().as_str() != AI_IMAGE_GENERATE_OPERATOR_TYPE {
        return Ok(None);
    }
    let wire = decode(
        draft
            .configuration()
            .ok_or_else(|| "AI image generation draft configuration is required".to_owned())?,
    )?;
    if wire.instruction.is_empty() {
        return Err("AI image generation draft has no authored instruction".to_owned());
    }
    AiImageGenerateParameters::new(
        wire.instruction,
        wire.output,
        wire.candidate_count,
        wire.constraints,
    )
    .map(Some)
    .map_err(|error| error.to_string())
}

pub(super) fn validate_ai_image_generate_configuration(
    configuration: Option<&WorkingOperatorConfiguration>,
) -> Result<(), String> {
    decode(
        configuration
            .ok_or_else(|| "AI image generation draft configuration is required".to_owned())?,
    )
    .map(|_| ())
}

fn decode(
    configuration: &WorkingOperatorConfiguration,
) -> Result<AiImageGenerateDraftWire, String> {
    if configuration.schema().as_str() != AI_IMAGE_GENERATE_DRAFT_SCHEMA {
        return Err("AI image generation draft configuration schema is unsupported".to_owned());
    }
    let wire: AiImageGenerateDraftWire =
        serde_json::from_str(configuration.json()).map_err(|_| {
            "AI image generation draft configuration does not match its exact schema".to_owned()
        })?;
    validate_parameters_or_empty(
        &wire.instruction,
        wire.output,
        wire.candidate_count,
        &wire.constraints,
    )?;
    Ok(wire)
}

fn validate_parameters_or_empty(
    instruction: &str,
    output: AiImageOutputCanvas,
    candidate_count: u8,
    constraints: &[Constraint],
) -> Result<(), String> {
    if !instruction.is_empty() && instruction.trim().is_empty() {
        return Err("AI image instruction cannot contain only whitespace".to_owned());
    }
    let validation_instruction = if instruction.is_empty() {
        "unconfigured source draft"
    } else {
        instruction
    };
    AiImageGenerateParameters::new(
        validation_instruction,
        output,
        candidate_count,
        constraints.to_vec(),
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
