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
fn execute(
    project: &Path,
    request: &Path,
) -> Result<shape_core::PictureBookExport, Box<dyn Error>> {
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
    let request: Request = serde_json::from_slice(&bytes)?;
    Ok(ShapeProject::open(project)?
        .export_picture_book(&request.composition, &request.destination)?)
}
