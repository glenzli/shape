use shape_core::ShapeProject;
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
