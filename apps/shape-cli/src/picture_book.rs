//! AI-facing composition adapter to the same accepted Shape materials used by the desktop.
use serde::Deserialize;
use serde_json::json;
use shape_core::{PictureBookRequest, ShapeProject};
use std::{
    error::Error,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    composition: PictureBookRequest,
    destination: PathBuf,
}

pub(super) fn run(project: &Path, request: &Path) -> u8 {
    match execute(project, request) {
        Ok(value) => {
            println!(
                "{}",
                json!({"schema":"shape.picture-book-result/1","status":"ok","result":value})
            );
            0
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema":"shape.picture-book-result/1","status":"error","message":error.to_string()})
            );
            1
        }
    }
}
fn execute(project: &Path, request: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    if !std::fs::symlink_metadata(request)?.file_type().is_file() {
        return Err("request must be a regular JSON file".into());
    }
    let file = File::open(request)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > 256 * 1024 {
        return Err("request exceeds 256 KiB".into());
    }
    let mut bytes = Vec::new();
    file.take(256 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 256 * 1024 {
        return Err("request exceeds 256 KiB".into());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let mut project = ShapeProject::open(project)?;
    if value.get("action").is_some() {
        return execute_action(&mut project, serde_json::from_value(value)?);
    }
    let request: Request = serde_json::from_value(value)?;
    Ok(serde_json::to_value(project.export_picture_book(
        &request.composition,
        &request.destination,
    )?)?)
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Preview {
        composition: PictureBookRequest,
        target: Option<shape_domain::WorkingInput>,
    },
    Accept {
        composition: PictureBookRequest,
        target: Option<shape_domain::WorkingInput>,
        review_digest: String,
    },
    Read {
        source: shape_domain::WorkingInput,
    },
    Export {
        source: shape_domain::WorkingInput,
        destination: PathBuf,
    },
    AdoptScript {
        name: String,
        source: Option<shape_domain::WorkingInput>,
        text: String,
    },
}
fn execute_action(
    project: &mut ShapeProject,
    action: Action,
) -> Result<serde_json::Value, Box<dyn Error>> {
    match action {
        Action::Preview {
            composition,
            target,
        } => {
            let candidate = project.propose_picture_book(target, composition.clone())?;
            Ok(
                json!({"review_digest":candidate.review_digest(),"preview":project.preview_picture_book(&composition)?}),
            )
        }
        Action::Accept {
            composition,
            target,
            review_digest,
        } => {
            let candidate = project.propose_picture_book(target, composition)?;
            if candidate.review_digest() != review_digest {
                return Err("review digest does not match this exact book proposal".into());
            }
            Ok(serde_json::to_value(
                project.accept_picture_book(candidate)?,
            )?)
        }
        Action::Read { source } => {
            let composition = project.read_picture_book(source)?;
            Ok(project.preview_picture_book(&composition)?)
        }
        Action::Export {
            source,
            destination,
        } => Ok(serde_json::to_value(
            project.export_accepted_picture_book(source, &destination)?,
        )?),
        Action::AdoptScript { name, source, text } => Ok(serde_json::to_value(
            project.adopt_picture_book_script(&name, source, &text)?,
        )?),
    }
}
