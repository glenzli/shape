use crate::session::{create_desktop_project, open_desktop_session};
use serde_json::{Value, json};
#[test]
fn desktop_book_review_discard_repeated_edit_and_reopen_use_shared_shelf() {
    let root = std::env::temp_dir().join(format!(
        "shape-book-bridge-{}",
        shape_domain::ArtifactId::new()
    ));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Books").unwrap();
    let en: Value = serde_json::from_str(
        &session
            .session_book_script("English", "", "[Cover] Moon\n[Author] Ada\n[1] Hello")
            .unwrap(),
    )
    .unwrap();
    let zh: Value = serde_json::from_str(
        &session
            .session_book_script("中文", "", "[封面] 月亮\n[作者] 艾达\n[1] 你好")
            .unwrap(),
    )
    .unwrap();
    let mut request = json!({"name":"Moon / 月亮","default_language":"en","languages":{"en":en,"zh-CN":zh},
        "pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768});
    let before = session.session_snapshot().unwrap().artifacts.len();
    let discarded = session
        .session_book_propose("", &request.to_string())
        .unwrap();
    session.session_discard_candidate(&discarded).unwrap();
    assert_eq!(session.session_snapshot().unwrap().artifacts.len(), before);
    let candidate = session
        .session_book_propose("", &request.to_string())
        .unwrap();
    let artifact = session
        .candidates
        .candidate(&candidate)
        .unwrap()
        .artifact_id();
    session.session_accept_candidate(&candidate).unwrap();
    assert!(session.candidates.candidate(&candidate).is_err());
    let mut source = session
        .session_snapshot()
        .unwrap()
        .artifacts
        .into_iter()
        .find(|a| a.id == artifact.to_string())
        .unwrap();
    for size in [30, 28] {
        request["font_size"] = json!(size);
        let pin = json!({"artifact_id":source.id,"revision_id":source.accepted_revision_id});
        let candidate = session
            .session_book_propose(&pin.to_string(), &request.to_string())
            .unwrap();
        session.session_accept_candidate(&candidate).unwrap();
        source = session
            .session_snapshot()
            .unwrap()
            .artifacts
            .into_iter()
            .find(|a| a.id == artifact.to_string())
            .unwrap();
    }
    drop(session);
    let reopened = open_desktop_session(root.to_str().unwrap()).unwrap();
    let pin = json!({"artifact_id":source.id,"revision_id":source.accepted_revision_id});
    let saved: Value =
        serde_json::from_str(&reopened.session_book_read(&pin.to_string()).unwrap()).unwrap();
    assert_eq!(saved["font_size"], 28);
    assert_eq!(
        reopened.session_snapshot().unwrap().artifacts.len(),
        before + 1
    );
    assert!(
        reopened
            .session_book_preview(&saved.to_string())
            .unwrap()
            .contains("你好")
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
