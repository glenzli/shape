//! Read-only legacy navigation and the bounded Vox material-transfer adapter.
use serde_json::json;
use shape_core::ShapeProject;
use std::{error::Error, fs::File, io::Read, path::Path};

pub(super) fn snapshot(path: &Path) -> u8 {
    report("shape.legacy-snapshot/1", || {
        Ok(ShapeProject::open_read_only(path)?.legacy_snapshot_json()?)
    })
}

pub(super) fn vox_transfer(path: &Path, request_path: &Path) -> u8 {
    report("shape.vox-transfer-result/1", || {
        if !std::fs::symlink_metadata(request_path)?
            .file_type()
            .is_file()
        {
            return Err("Vox transfer request must be a regular JSON file".into());
        }
        let file = File::open(request_path)?;
        if !file.metadata()?.is_file() || file.metadata()?.len() > 16 * 1024 {
            return Err("Vox transfer request exceeds 16 KiB".into());
        }
        let mut bytes = Vec::new();
        file.take(16 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 {
            return Err("Vox transfer request exceeds 16 KiB".into());
        }
        Ok(ShapeProject::open_read_only(path)?
            .export_vox_transfer_json(std::str::from_utf8(&bytes)?)?)
    })
}

fn report(schema: &str, action: impl FnOnce() -> Result<String, Box<dyn Error>>) -> u8 {
    match action() {
        Ok(value) => {
            println!("{value}");
            0
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema":schema,"status":"error","message":error.to_string()})
            );
            1
        }
    }
}
