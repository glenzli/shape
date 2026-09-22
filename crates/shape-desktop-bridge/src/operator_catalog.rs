//! Rust-owned catalog of desktop-exercised creative Operators.
//!
//! Machine identity, type compatibility, and workspace/icon routing live here.
//! QML remains responsible for localized display labels and search presentation.

use shape_domain::ArtifactKind;

mod ai_image_generate;
mod audio_speech;
mod image_resize;
pub(crate) mod text_authoring;
mod text_transform;

use ai_image_generate::validate_ai_image_generate_configuration;
pub(crate) use ai_image_generate::{
    ai_image_generate_parameters_from_draft, ai_image_generate_state_from_draft,
    configuration_for_ai_image_generate,
};
use audio_speech::validate_audio_speech_configuration;
pub(crate) use audio_speech::{
    audio_speech_operation_from_draft, configuration_for_audio_speech, configuration_with_script,
    default_audio_speech_configuration,
};
use image_resize::validate_image_resize_configuration;
pub(crate) use image_resize::{
    aspect_policy_key, configuration_for_image_resize, image_resize_from_draft, resampling_key,
};
use text_transform::validate_text_transform_configuration;
pub(crate) use text_transform::{
    compiled_instruction_from_draft, configuration_for_expression_studio, configuration_for_studio,
    expression_json_from_draft, instruction_from_draft, mode_from_draft, style_from_draft,
    tone_from_draft, variant_count_from_draft,
};

pub(crate) const AUDIO_SPEECH_OPERATOR: &str = "audio.speech_synthesize";
pub(crate) const IMAGE_CROP_OPERATOR: &str = "image.crop";
pub(crate) const IMAGE_GENERATE_OPERATOR: &str = "image.generate";
pub(crate) const IMAGE_RESIZE_OPERATOR: &str = "image.resize";
pub(crate) const TEXT_CREATE_OPERATOR: &str = "text.create";
pub(crate) const TEXT_EDIT_OPERATOR: &str = "text.edit";
pub(crate) const TEXT_TRANSFORM_OPERATOR: &str = "text.transform";

const AUDIO_CLIP_DATA: &str = "audio.clip";
const IMAGE_RASTER_DATA: &str = "image.raster";
const TEXT_DOCUMENT_DATA: &str = "text.document";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OperatorDescriptor {
    pub(crate) type_key: &'static str,
    pub(crate) source_kind: ArtifactKind,
    pub(crate) input_data_type: &'static str,
    pub(crate) output_data_type: &'static str,
    pub(crate) category_key: &'static str,
    pub(crate) icon_key: &'static str,
}

const DESCRIPTORS: [OperatorDescriptor; 5] = [
    OperatorDescriptor {
        type_key: TEXT_CREATE_OPERATOR,
        source_kind: ArtifactKind::TextDocument,
        input_data_type: "",
        output_data_type: TEXT_DOCUMENT_DATA,
        category_key: "text",
        icon_key: "edit",
    },
    OperatorDescriptor {
        type_key: TEXT_EDIT_OPERATOR,
        source_kind: ArtifactKind::TextDocument,
        input_data_type: TEXT_DOCUMENT_DATA,
        output_data_type: TEXT_DOCUMENT_DATA,
        category_key: "text",
        icon_key: "sparkle",
    },
    OperatorDescriptor {
        type_key: AUDIO_SPEECH_OPERATOR,
        source_kind: ArtifactKind::TextDocument,
        input_data_type: TEXT_DOCUMENT_DATA,
        output_data_type: AUDIO_CLIP_DATA,
        category_key: "ai_audio",
        icon_key: "waveform",
    },
    OperatorDescriptor {
        type_key: IMAGE_CROP_OPERATOR,
        source_kind: ArtifactKind::ImageRaster,
        input_data_type: IMAGE_RASTER_DATA,
        output_data_type: IMAGE_RASTER_DATA,
        category_key: "image",
        icon_key: "crop",
    },
    OperatorDescriptor {
        type_key: IMAGE_RESIZE_OPERATOR,
        source_kind: ArtifactKind::ImageRaster,
        input_data_type: IMAGE_RASTER_DATA,
        output_data_type: IMAGE_RASTER_DATA,
        category_key: "image",
        icon_key: "fit",
    },
];

pub(crate) fn compatible_descriptors(
    source_kind: ArtifactKind,
) -> impl Iterator<Item = &'static OperatorDescriptor> {
    DESCRIPTORS.iter().filter(move |descriptor| {
        descriptor.source_kind == source_kind || descriptor.input_data_type.is_empty()
    })
}

pub(crate) fn source_descriptors() -> impl Iterator<Item = &'static OperatorDescriptor> {
    DESCRIPTORS
        .iter()
        .filter(|descriptor| descriptor.input_data_type.is_empty())
}

pub(crate) fn descriptor_for(
    source_kind: ArtifactKind,
    operator_type: &str,
) -> Result<&'static OperatorDescriptor, String> {
    let operator_type =
        if source_kind == ArtifactKind::TextDocument && operator_type == TEXT_TRANSFORM_OPERATOR {
            TEXT_EDIT_OPERATOR
        } else {
            operator_type
        };
    compatible_descriptors(source_kind)
        .find(|descriptor| descriptor.type_key == operator_type)
        .ok_or_else(|| "this Operator is incompatible with the selected Scene source".to_owned())
}

pub(crate) fn is_text_workspace_operator(operator_type: &str) -> bool {
    matches!(
        operator_type,
        TEXT_CREATE_OPERATOR | TEXT_EDIT_OPERATOR | TEXT_TRANSFORM_OPERATOR
    )
}

pub(crate) fn is_image_edit_workspace_operator(operator_type: &str) -> bool {
    matches!(operator_type, IMAGE_CROP_OPERATOR | IMAGE_RESIZE_OPERATOR)
}

pub(crate) fn validate_draft_configuration(
    draft: &shape_domain::WorkingOperatorDraft,
) -> Result<(), String> {
    match (draft.operator_type().as_str(), draft.configuration()) {
        (AUDIO_SPEECH_OPERATOR, configuration) => {
            validate_audio_speech_configuration(configuration)
        }
        (TEXT_CREATE_OPERATOR | TEXT_EDIT_OPERATOR | TEXT_TRANSFORM_OPERATOR, configuration) => {
            if let Some(configuration) = configuration
                && configuration.schema().as_str() == text_authoring::SCHEMA
            {
                return text_authoring::TextAuthoring::from_json(configuration.json()).map(|_| ());
            }
            validate_text_transform_configuration(configuration)
        }
        (IMAGE_RESIZE_OPERATOR, configuration) => {
            validate_image_resize_configuration(configuration)
        }
        (IMAGE_GENERATE_OPERATOR, configuration) => {
            validate_ai_image_generate_configuration(configuration)
        }
        (_, None) => Ok(()),
        _ => Err("this Operator does not support persisted draft configuration".to_owned()),
    }
}

#[cfg(test)]
mod tests;
