//! JSON request/response adapter for the same material-edit stages used by the desktop.

use serde::Deserialize;
use serde_json::{Value, json};
use shape_core::{MaterialEditTicket, ShapeProject};
use shape_domain::{ArtifactId, ContentDigest, MaterialEditor, WorkingInput};
use std::{
    error::Error,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Begin {
        source: WorkingInput,
        name: String,
        editor: MaterialEditor,
    },
    Status {
        output_artifact_id: ArtifactId,
    },
    Export {
        ticket: MaterialEditTicket,
        destination: PathBuf,
    },
    Review {
        handoff: PathBuf,
        returned_file: PathBuf,
        #[serde(default)]
        allow_pcm24_conversion: bool,
    },
    Accept {
        handoff: PathBuf,
        returned_file: PathBuf,
        #[serde(default)]
        allow_pcm24_conversion: bool,
        returned_blake3: String,
    },
    ReviewShadow {
        handoff: PathBuf,
        result_file: PathBuf,
    },
    AcceptShadow {
        handoff: PathBuf,
        result_file: PathBuf,
        returned_blake3: String,
    },
    Cancel {
        ticket: MaterialEditTicket,
    },
    Refresh {
        ticket: MaterialEditTicket,
    },
}

pub(super) fn run(project: &Path, request: &Path) -> u8 {
    match execute(project, request) {
        Ok(result) => {
            println!(
                "{}",
                json!({"schema":"shape.material-edit-result/1", "status":"ok", "result": result})
            );
            0
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema":"shape.material-edit-result/1", "status":"error", "message":error.to_string()})
            );
            1
        }
    }
}

fn execute(project: &Path, request: &Path) -> Result<Value, Box<dyn Error>> {
    if !std::fs::symlink_metadata(request)?.file_type().is_file() {
        return Err("request must be a regular JSON file".into());
    }
    let file = File::open(request)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > 256 * 1024 {
        return Err("request must be a regular JSON file up to 256 KiB".into());
    }
    let mut bytes = Vec::new();
    file.take(256 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 256 * 1024 {
        return Err("request too large".into());
    }
    let request: Request = serde_json::from_slice(&bytes)?;
    let mut project = ShapeProject::open(project)?;
    Ok(match request {
        Request::Begin {
            source,
            name,
            editor,
        } => json!({"ticket":project.create_material_edit(source, &name, editor)?}),
        Request::Status { output_artifact_id } => {
            json!({"ticket":project.material_edit_ticket(output_artifact_id)?})
        }
        Request::Export {
            ticket,
            destination,
        } => json!({"handoff":project.export_material_edit(&ticket, &destination)?}),
        Request::Review {
            handoff,
            returned_file,
            allow_pcm24_conversion,
        } => {
            let handoff = ShapeProject::read_material_handoff(&handoff)?;
            let candidate = project.material_return_file_with_options(
                &handoff.ticket,
                &returned_file,
                None,
                allow_pcm24_conversion,
            )?;
            let returned_digest = candidate.returned_digest().to_string();
            json!({"ticket":candidate.ticket(), "returned_blake3":returned_digest,
                "normalized_blake3":ContentDigest::from_bytes(candidate.bytes()).to_string(),
                "output_contract":candidate.contract(), "normalization":candidate.normalization(), "byte_length":candidate.bytes().len(), "accepted":false})
        }
        Request::Accept {
            handoff,
            returned_file,
            allow_pcm24_conversion,
            returned_blake3,
        } => {
            let handoff = ShapeProject::read_material_handoff(&handoff)?;
            let candidate = project.material_return_file_with_options(
                &handoff.ticket,
                &returned_file,
                Some(returned_blake3.parse()?),
                allow_pcm24_conversion,
            )?;
            json!({"accepted_revision": project.accept_material_return(candidate)?})
        }
        Request::ReviewShadow {
            handoff,
            result_file,
        } => {
            let handoff = ShapeProject::read_material_handoff(&handoff)?;
            let candidate = project.propose_shadow_return(&handoff, &result_file, None)?;
            json!({"ticket":candidate.ticket(),"returned_blake3":candidate.returned_digest().to_string(),
                "normalized_blake3":ContentDigest::from_bytes(candidate.bytes()).to_string(),
                "output_contract":candidate.contract(),"accepted":false,"completed_shadow_receipt_verified":true})
        }
        Request::AcceptShadow {
            handoff,
            result_file,
            returned_blake3,
        } => {
            let handoff = ShapeProject::read_material_handoff(&handoff)?;
            let candidate = project.propose_shadow_return(
                &handoff,
                &result_file,
                Some(returned_blake3.parse()?),
            )?;
            json!({"accepted_revision":project.accept_material_return(candidate)?})
        }
        Request::Cancel { ticket } => {
            json!({"ticket":project.restart_material_edit(&ticket, false)?, "previous_session_cancelled":true})
        }
        Request::Refresh { ticket } => {
            json!({"ticket":project.restart_material_edit(&ticket, true)?, "input_refreshed":true})
        }
    })
}
