use crate::session::open_desktop_session;
use serde_json::{Value, json};
use shape_core::ShapeProject;

#[test]
fn accepted_historical_book_still_reads_and_exports_without_new_authoring() {
    let root = std::env::temp_dir().join(format!(
        "shape-book-compat-{}",
        shape_domain::ArtifactId::new()
    ));
    std::fs::create_dir(&root).unwrap();
    let bundle = root.join("legacy.shape");
    let mut project = ShapeProject::create(&bundle, "Legacy").unwrap();
    let text = project
        .create_text_document("Script", "[Cover] Moon\n[1] Hello")
        .unwrap();
    let request = serde_json::from_value(json!({"name":"Moon","default_language":"en","languages":{"en":{"artifact_id":text.artifact_id,"revision_id":text.id}},"pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768})).unwrap();
    let candidate = project.propose_picture_book(None, request).unwrap();
    let book = project.accept_picture_book(candidate).unwrap();
    drop(project);
    let session = open_desktop_session(bundle.to_str().unwrap()).unwrap();
    let pin = json!({"artifact_id":book.artifact_id,"revision_id":book.id}).to_string();
    let loaded: Value = serde_json::from_str(&session.session_book_read(&pin).unwrap()).unwrap();
    assert_eq!(loaded["name"], "Moon");
    let result: Value = serde_json::from_str(
        &session
            .session_book_export(&pin, root.join("saved.scproj").to_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(std::path::Path::new(result["destination"].as_str().unwrap()).is_file());
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
