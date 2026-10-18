//! Public CLI consumer of the same bounded draft, material export and print-plan actions as desktop.
use serde_json::json;
use shape_core::ShapeProject;
use std::{error::Error, fs::File, io::Read, path::Path};

pub(super) fn run(project: &Path, request: &Path) -> u8 {
    match execute(project, request) {
        Ok(result) => {
            println!(
                "{}",
                json!({"schema":"shape.card-kit-result/1","status":"ok","result":result})
            );
            0
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema":"shape.card-kit-result/1","status":"error","message":error.to_string()})
            );
            1
        }
    }
}

fn execute(project: &Path, request: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    if !std::fs::symlink_metadata(request)?.file_type().is_file() {
        return Err("card kit request must be a regular JSON file".into());
    }
    let file = File::open(request)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > 96 * 1024 {
        return Err("card kit request exceeds 96 KiB".into());
    }
    let mut bytes = Vec::new();
    file.take(96 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 96 * 1024 {
        return Err("card kit request exceeds 96 KiB".into());
    }
    let request = std::str::from_utf8(&bytes)?;
    let result = ShapeProject::open(project)?.card_kit_action_json(request)?;
    Ok(serde_json::from_str(&result)?)
}
