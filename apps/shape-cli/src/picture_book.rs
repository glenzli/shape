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
    let mut project = ShapeProject::open_read_only(project)?;
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
    DraftList,
    DraftLoad {
        artifact_id: shape_domain::ArtifactId,
    },
    DraftExport {
        artifact_id: shape_domain::ArtifactId,
        draft_digest: String,
        destination: PathBuf,
    },
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
        Action::DraftList => Ok(json!({"drafts":project.picture_book_drafts()?})),
        Action::DraftLoad { artifact_id } => {
            Ok(json!({"draft":project.load_picture_book_draft(artifact_id)?}))
        }
        Action::DraftExport {
            artifact_id,
            draft_digest,
            destination,
        } => Ok(project.export_legacy_picture_book_draft(
            artifact_id,
            &draft_digest,
            &destination,
        )?),
        Action::Preview {
            composition,
            target,
        } => {
            let _ = (composition, target);
            Err(
                "book authoring moved to Storybook Co-Editor; use legacy reads or .scproj export"
                    .into(),
            )
        }
        Action::Accept {
            composition,
            target,
            review_digest,
        } => {
            let _ = (composition, target, review_digest);
            Err(
                "book authoring moved to Storybook Co-Editor; use legacy reads or .scproj export"
                    .into(),
            )
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
        Action::AdoptScript { name, source, text } => {
            let _ = (name, source, text);
            Err("book script adoption moved to Storybook Co-Editor".into())
        }
    }
}
