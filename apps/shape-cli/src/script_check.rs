//! Bounded file intake and machine-readable diagnostics over the canonical domain parser.

use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

use serde_json::json;
use shape_domain::speech_script::parse_speech_script_for_review;

const MAX_SOURCE_BYTES: u64 = 65_536;

pub fn run(path: &Path) -> Result<u8, Box<dyn Error>> {
    let source = read_source(path)?;
    let (plan, note_lines) = parse_speech_script_for_review(&source);
    let valid = plan.issues.is_empty();
    let issues: Vec<_> = plan
        .issues
        .iter()
        .map(|issue| json!({"line": issue.line, "code": issue.code}))
        .collect();
    let report = json!({
        "schema": "shape.cli.script-check@20261002.1",
        "grammar_revision": plan.revision,
        "source_digest": plan.source_digest.to_string(),
        "source_bytes": source.len(),
        "valid": valid,
        "synthesis_checked": false,
        "roles": plan.roles.keys().collect::<Vec<_>>(),
        "cues": plan.cues.keys().collect::<Vec<_>>(),
        "note_lines": note_lines,
        "explicit_pause_ms": plan.pause_millis(),
        "issues": issues,
    });
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, &report)?;
    writeln!(output)?;
    Ok(if valid { 0 } else { 2 })
}

fn read_source(path: &Path) -> io::Result<String> {
    // Reject special files before opening them (in particular, a pipe with no writer).
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "script must be a regular file",
        ));
    }
    if metadata.len() > MAX_SOURCE_BYTES {
        return Err(too_large());
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "script must be a regular file",
        ));
    }
    // Recheck the bytes as well as metadata so an expanding file cannot bypass the bound.
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(too_large());
    }
    String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "script must contain valid UTF-8",
        )
    })
}

fn too_large() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "script exceeds the 65536-byte limit",
    )
}
