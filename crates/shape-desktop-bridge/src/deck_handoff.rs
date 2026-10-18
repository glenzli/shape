//! Worker-only Deck file operations; no live `DesktopSession` crosses this boundary.
use shape_core::ShapeProject;
use std::path::Path;

pub(super) fn prepare_deck_handoff(
    project: &str,
    artifact: &str,
    digest: &str,
    request_id: &str,
    directory: &str,
    document: &str,
) -> Result<String, String> {
    ShapeProject::open_read_only(project)
        .map_err(|e| e.to_string())?
        .prepare_deck_handoff(
            artifact.parse().map_err(|_| "invalid card set id")?,
            digest,
            request_id,
            Path::new(directory),
            document,
        )
        .map_err(|e| e.to_string())
}
pub(super) fn prepare_deck_accepted_handoff(
    project: &str,
    source_json: &str,
    request_id: &str,
    directory: &str,
    document: &str,
) -> Result<String, String> {
    ShapeProject::open_read_only(project)
        .map_err(|e| e.to_string())?
        .prepare_deck_accepted_handoff(
            serde_json::from_str(source_json).map_err(|_| "invalid accepted card pin")?,
            request_id,
            Path::new(directory),
            document,
        )
        .map_err(|e| e.to_string())
}
pub(super) fn verify_deck_output_return(
    request: &str,
    result: &str,
    expected_request_blake3: &str,
) -> Result<String, String> {
    ShapeProject::verify_deck_output_return(
        Path::new(request),
        Path::new(result),
        expected_request_blake3,
    )
    .map_err(|e| e.to_string())
}
pub(super) fn read_deck_material_task(request: &str, result: &str) -> Result<String, String> {
    ShapeProject::read_deck_material_task(Path::new(request), Path::new(result))
        .map_err(|e| e.to_string())
}
pub(super) fn finish_deck_material_task(
    project: &str,
    request: &str,
    result: &str,
    source: &str,
    expected_request_blake3: &str,
    outcome: &str,
    error: &str,
) -> Result<String, String> {
    if outcome == "completed" {
        ShapeProject::open(project)
            .map_err(|e| e.to_string())?
            .return_deck_material(
                Path::new(request),
                Path::new(result),
                serde_json::from_str(source).map_err(|e| e.to_string())?,
                expected_request_blake3,
            )
            .map_err(|e| e.to_string())
    } else {
        ShapeProject::finish_deck_material_task(
            Path::new(request),
            Path::new(result),
            expected_request_blake3,
            outcome,
            error,
        )
        .map_err(|e| e.to_string())
    }
}
