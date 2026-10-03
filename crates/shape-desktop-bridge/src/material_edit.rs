//! Worker-side material file I/O and decoding; the live desktop session stays on its UI thread.
use shape_core::{CoreError, MaterialEditCandidate, MaterialEditTicket, ShapeProject};
use std::path::Path;

#[derive(Debug)]
pub struct PreparedMaterialReturn {
    pub(super) candidate: MaterialEditCandidate,
}

// Owned argument matches Result::map_err; provider details never cross the UI boundary.
#[allow(clippy::needless_pass_by_value)]
pub(super) fn material_edit_error(error: CoreError) -> String {
    match error {
        CoreError::StaleMaterialSession | CoreError::StaleCandidate { .. } => {
            "material_session_changed"
        }
        CoreError::UnchangedMaterialReturn => "material_unchanged",
        CoreError::Store(_) => "material_file_unavailable",
        _ => "material_return_invalid",
    }
    .into()
}

pub(super) fn export_material_copy(
    project_path: &str,
    ticket_json: &str,
    directory: &str,
) -> Result<String, String> {
    let project = ShapeProject::open(project_path).map_err(material_edit_error)?;
    let ticket: MaterialEditTicket =
        serde_json::from_str(ticket_json).map_err(|_| "material_return_invalid")?;
    let handoff = project
        .export_material_edit(&ticket, Path::new(directory))
        .map_err(material_edit_error)?;
    serde_json::to_string(&handoff).map_err(|_| "material_return_invalid".into())
}

pub(super) fn prepare_material_return(
    project_path: &str,
    ticket_json: &str,
    handoff_path: &str,
    returned_path: &str,
    allow_pcm24_conversion: bool,
) -> Result<Box<PreparedMaterialReturn>, String> {
    let ticket: MaterialEditTicket =
        serde_json::from_str(ticket_json).map_err(|_| "material_return_invalid")?;
    let handoff = ShapeProject::read_material_handoff(Path::new(handoff_path))
        .map_err(material_edit_error)?;
    if ticket != handoff.ticket {
        return Err("material_session_changed".into());
    }
    let project = ShapeProject::open(project_path).map_err(material_edit_error)?;
    let returned_path = Path::new(returned_path);
    let candidate = if returned_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        project.propose_shadow_return(&handoff, returned_path, None)
    } else {
        project.material_return_file_with_options(
            &ticket,
            returned_path,
            None,
            allow_pcm24_conversion,
        )
    }
    .map_err(material_edit_error)?;
    Ok(Box::new(PreparedMaterialReturn { candidate }))
}
