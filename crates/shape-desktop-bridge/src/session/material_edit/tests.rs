use super::*;
use crate::{create_desktop_project, material_edit::prepare_material_return, open_desktop_session};
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use std::fs;
fn png(color: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&[color, 30, 40, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    bytes
}

#[test]
fn material_stage_reopens_projects_topology_and_reviews_replaces_accepts_on_the_original_output() {
    let root = std::env::temp_dir().join(format!("shape-material-bridge-{}", uuid::Uuid::now_v7()));
    let project_path = root.join("fixture.shape");
    fs::create_dir(&root).unwrap();
    let mut session =
        create_desktop_project(project_path.to_str().unwrap(), "Material bridge").unwrap();
    let source = session
        .project
        .import_raster_bytes(png(10), "Original")
        .unwrap();
    let draft = session
        .session_begin_operator_draft(&source.artifact_id.to_string(), "image.external_edit")
        .unwrap();
    let output = draft.context_artifact_id.clone();
    let json = session.session_material_edit_ticket(&output).unwrap();
    let ticket: MaterialEditTicket = serde_json::from_str(&json).unwrap();
    assert_ne!(output, source.artifact_id.to_string());
    let handoff_dir = root.join("handoff");
    session
        .project
        .export_material_edit(&ticket, &handoff_dir)
        .unwrap();
    let returned = root.join("returned.png");
    fs::write(&returned, png(80)).unwrap();
    let prepare = || {
        prepare_material_return(
            project_path.to_str().unwrap(),
            &json,
            handoff_dir.join("handoff.json").to_str().unwrap(),
            returned.to_str().unwrap(),
            false,
        )
        .unwrap()
    };
    let a = session.session_adopt_material_return(prepare()).unwrap();
    let duplicate = session.session_adopt_material_return(prepare()).unwrap();
    assert_eq!(a.candidate_id, duplicate.candidate_id);
    assert_eq!(session.session_candidates().len(), 1);
    assert_eq!(
        session
            .session_image_preview(&output, &a.candidate_id)
            .unwrap()
            .png_bytes,
        png(80)
    );
    assert!(
        session
            .session_image_preview(&source.artifact_id.to_string(), &a.candidate_id)
            .is_err()
    );
    fs::write(&returned, b"invalid").unwrap();
    assert!(
        prepare_material_return(
            project_path.to_str().unwrap(),
            &json,
            handoff_dir.join("handoff.json").to_str().unwrap(),
            returned.to_str().unwrap(),
            false,
        )
        .is_err()
    );
    assert_eq!(session.session_candidates()[0].candidate_id, a.candidate_id);
    fs::write(&returned, png(81)).unwrap();
    let b = session.session_adopt_material_return(prepare()).unwrap();
    assert_ne!(b.candidate_id, a.candidate_id);
    assert_eq!(session.session_candidates().len(), 1);
    session.session_accept_candidate(&b.candidate_id).unwrap();
    assert!(session.session_candidates().is_empty());
    assert_eq!(
        session.session_operator_drafts()[0].draft_id,
        draft.draft_id
    );
    drop(session);
    let reopened = open_desktop_session(project_path.to_str().unwrap()).unwrap();
    assert!(reopened.session_material_edit_ticket(&output).is_ok());
    let snapshot = reopened.session_snapshot().unwrap();
    let edited = snapshot.artifacts.iter().find(|a| a.id == output).unwrap();
    assert!(
        edited
            .operator_graph_nodes
            .iter()
            .any(|n| n.node_id == draft.draft_id && n.operator_type_key == "image.external_edit")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancellation_and_foreign_project_reject_pending_worker_result() {
    let root = std::env::temp_dir().join(format!("shape-material-owner-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let project_path = root.join("a.shape");
    let mut session = create_desktop_project(project_path.to_str().unwrap(), "A").unwrap();
    let source = session
        .project
        .import_raster_bytes(png(10), "Original")
        .unwrap();
    let draft = session
        .session_begin_operator_draft(&source.artifact_id.to_string(), "image.external_edit")
        .unwrap();
    let json = session
        .session_material_edit_ticket(&draft.context_artifact_id)
        .unwrap();
    let ticket: MaterialEditTicket = serde_json::from_str(&json).unwrap();
    session
        .project
        .export_material_edit(&ticket, &root.join("handoff"))
        .unwrap();
    let returned = root.join("return.png");
    fs::write(&returned, png(85)).unwrap();
    let prepare = || {
        prepare_material_return(
            project_path.to_str().unwrap(),
            &json,
            root.join("handoff/handoff.json").to_str().unwrap(),
            returned.to_str().unwrap(),
            false,
        )
        .unwrap()
    };
    let mine = prepare();
    let wrong = prepare();
    let mut other = create_desktop_project(root.join("b.shape").to_str().unwrap(), "B").unwrap();
    assert!(other.session_adopt_material_return(wrong).is_err());
    session.session_restart_material_edit(&json, false).unwrap();
    assert!(session.session_adopt_material_return(mine).is_err());
    assert!(session.session_candidates().is_empty());
    assert!(session.session_restart_material_edit(&json, false).is_err());
    fs::remove_dir_all(root).unwrap();
}
