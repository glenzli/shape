//! Rust-owned catalog of desktop-exercised creative Operators.
//!
//! Machine identity, type compatibility, and workspace/icon routing live here.
//! QML remains responsible for localized display labels and search presentation.

use shape_domain::ArtifactKind;

mod text_transform;

use text_transform::validate_text_transform_configuration;
pub(crate) use text_transform::{configuration_for_instruction, instruction_from_draft};

pub(crate) const AUDIO_SPEECH_OPERATOR: &str = "audio.speech_synthesize";
pub(crate) const IMAGE_CROP_OPERATOR: &str = "image.crop";
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

const DESCRIPTORS: [OperatorDescriptor; 4] = [
    OperatorDescriptor {
        type_key: TEXT_EDIT_OPERATOR,
        source_kind: ArtifactKind::TextDocument,
        input_data_type: TEXT_DOCUMENT_DATA,
        output_data_type: TEXT_DOCUMENT_DATA,
        category_key: "text",
        icon_key: "edit",
    },
    OperatorDescriptor {
        type_key: TEXT_TRANSFORM_OPERATOR,
        source_kind: ArtifactKind::TextDocument,
        input_data_type: TEXT_DOCUMENT_DATA,
        output_data_type: TEXT_DOCUMENT_DATA,
        category_key: "ai_text",
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
];

pub(crate) fn compatible_descriptors(
    source_kind: ArtifactKind,
) -> impl Iterator<Item = &'static OperatorDescriptor> {
    DESCRIPTORS
        .iter()
        .filter(move |descriptor| descriptor.source_kind == source_kind)
}

pub(crate) fn descriptor_for(
    source_kind: ArtifactKind,
    operator_type: &str,
) -> Result<&'static OperatorDescriptor, String> {
    compatible_descriptors(source_kind)
        .find(|descriptor| descriptor.type_key == operator_type)
        .ok_or_else(|| "this Operator is incompatible with the selected Scene source".to_owned())
}

pub(crate) fn validate_draft_configuration(
    draft: &shape_domain::WorkingOperatorDraft,
) -> Result<(), String> {
    match (draft.operator_type().as_str(), draft.configuration()) {
        (TEXT_TRANSFORM_OPERATOR, configuration) => {
            validate_text_transform_configuration(configuration)
        }
        (_, None) => Ok(()),
        _ => Err("this Operator does not support persisted draft configuration".to_owned()),
    }
}

#[cfg(test)]
mod tests;
