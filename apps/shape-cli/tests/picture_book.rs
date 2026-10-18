use shape_core::ShapeProject;
use std::io::Read;
use std::{fs, process::Command};

#[test]
fn accepted_script_becomes_native_book_and_repeated_publication_is_not_overwritten() {
    let root = std::env::temp_dir().join(format!("shape-book-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let path = root.join("source.shape");
    let mut project = ShapeProject::create(&path, "Book").unwrap();
    let source = project
        .create_text_document("Script", "[Cover] Night\n[1] Stars")
        .unwrap();
    let request = serde_json::json!({"composition":{"name":"Night","default_language":"en",
        "languages":{"en":{"artifact_id":source.artifact_id,"revision_id":source.id}},
        "pages":[{"kind":"blank"},{"kind":"blank","print_only":true}],"canvas_width":1024,"canvas_height":1024},
        "destination":root.join("book.scproj")});
    let request_path = root.join("request.json");
    fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_shape-cli"))
            .arg("picture-book")
            .arg(&path)
            .arg(&request_path)
            .output()
            .unwrap()
    };
    let output = run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["schema"], "shape.picture-book-result/1");
    assert_eq!(receipt["result"]["pages"], 2);
    let original = fs::read(root.join("book.scproj")).unwrap();
    let again = run();
    assert!(!again.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&again.stdout).unwrap()["status"],
        "error"
    );
    assert_eq!(fs::read(root.join("book.scproj")).unwrap(), original);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_book_exports_selected_accepted_or_full_unaccepted_draft_without_source_changes() {
    let root = std::env::temp_dir().join(format!("shape-book-cli-legacy-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let path = root.join("source.shape");
    let mut project = ShapeProject::create(&path, "Legacy book").unwrap();
    let text = project
        .create_text_document("Script", "[Cover] Accepted moon\n[1] Accepted stars")
        .unwrap();
    let composition=serde_json::from_value(serde_json::json!({"name":"Moon","default_language":"en","languages":{"en":{"artifact_id":text.artifact_id,"revision_id":text.id}},"pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768})).unwrap();
    let candidate = project.propose_picture_book(None, composition).unwrap();
    let accepted = project.accept_picture_book(candidate).unwrap();
    let draft = project
        .open_picture_book_draft(accepted.artifact_id)
        .unwrap();
    let mut state = draft.state;
    state.script = "unfinished raw author script [do not silently discard]".into();
    state.script_dirty = true;
    let draft = project
        .save_picture_book_draft(draft.artifact_id, &draft.draft_digest, &state)
        .unwrap();
    drop(project);
    let before = fs::read(path.join("project.sqlite")).unwrap();
    let request_path = root.join("request.json");
    let run = |request: serde_json::Value, success: bool| {
        fs::write(&request_path, request.to_string()).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
            .arg("picture-book")
            .arg(&path)
            .arg(&request_path)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    let exported = run(
        serde_json::json!({"action":"draft_export","artifact_id":draft.artifact_id,"draft_digest":draft.draft_digest,"destination":root.join("draft.scproj")}),
        true,
    );
    assert_eq!(exported["result"]["unaccepted_script_preserved"], true);
    let mut archive =
        zip::ZipArchive::new(fs::File::open(root.join("draft.scproj")).unwrap()).unwrap();
    let mut bytes = String::new();
    archive
        .by_name("project.json")
        .unwrap()
        .read_to_string(&mut bytes)
        .unwrap();
    let document: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(document["languages"]["en"]["script"], state.script);
    bytes.clear();
    archive
        .by_name("shape-draft.json")
        .unwrap()
        .read_to_string(&mut bytes)
        .unwrap();
    let sidecar: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(sidecar["status"], "unaccepted");
    assert_eq!(sidecar["draft"]["state"]["script"], state.script);
    let source = serde_json::json!({"artifact_id":accepted.artifact_id,"revision_id":accepted.id});
    run(
        serde_json::json!({"action":"export","source":source,"destination":root.join("accepted.scproj")}),
        true,
    );
    let mut archive =
        zip::ZipArchive::new(fs::File::open(root.join("accepted.scproj")).unwrap()).unwrap();
    bytes.clear();
    archive
        .by_name("project.json")
        .unwrap()
        .read_to_string(&mut bytes)
        .unwrap();
    let document: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(
        document["languages"]["en"]["script"],
        "[Cover] Accepted moon\n[1] Accepted stars"
    );
    for request in [
        serde_json::json!({"action":"preview","composition":state.composition}),
        serde_json::json!({"action":"accept","composition":state.composition,"review_digest":"old"}),
        serde_json::json!({"action":"adopt_script","name":"Changed","text":"[Cover] New"}),
    ] {
        run(request, false);
    }
    assert_eq!(fs::read(path.join("project.sqlite")).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}
