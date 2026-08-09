use std::fs;

use super::*;

fn test_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("shape-store-{label}-{}", Uuid::now_v7()))
}

#[test]
fn published_objects_round_trip_and_deduplicate() {
    let root = test_root("round-trip");
    let store = ObjectStore::create(&root).expect("object store creates");
    let first = store
        .publish(b"same bytes", "text/plain")
        .expect("first publish succeeds");
    let second = store
        .publish(b"same bytes", "text/plain")
        .expect("second publish succeeds");

    assert_eq!(first, second);
    assert_eq!(store.read(&first).expect("object reads"), b"same bytes");
    fs::remove_dir_all(&root).expect("test bundle removes");
}

#[test]
fn corrupted_objects_are_rejected() {
    let root = test_root("corrupt");
    let store = ObjectStore::create(&root).expect("object store creates");
    let content = store
        .publish(b"original", "text/plain")
        .expect("publish succeeds");
    fs::write(store.path_for(content.digest), b"changed").expect("test corrupts object");

    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::remove_dir_all(&root).expect("test bundle removes");
}
