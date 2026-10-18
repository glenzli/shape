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

#[test]
fn bounded_verification_admits_exact_bytes_and_rejects_size_or_digest_mismatch() {
    let root = test_root("bounded-verification");
    let store = ObjectStore::create(&root).unwrap();
    let content = store.publish(b"original", "image/png").unwrap();
    store.verify_bounded(&content, 8).unwrap();
    assert!(matches!(
        store.verify_bounded(&content, 7),
        Err(StoreError::InspectionLimit {
            resource: "material content bytes",
            maximum: 7
        })
    ));
    let path = store.path_for(content.digest);
    fs::write(&path, b"modified").unwrap();
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::write(&path, b"original and oversized").unwrap();
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.publish(b"original", "image/png"),
        Err(StoreError::CorruptObject { .. })
    ));
    // A sparse oversized regular file is rejected from metadata instead of first reading its payload.
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024)
        .unwrap();
    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.publish(b"original", "image/png"),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn accepted_read_bounded_verification_and_dedup_reject_a_fifo_before_opening_it() {
    let root = test_root("fifo-verification");
    let store = ObjectStore::create(&root).unwrap();
    let content = store.publish(b"original", "image/png").unwrap();
    let path = store.path_for(content.digest);
    fs::remove_file(&path).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.publish(b"original", "image/png"),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_objects_remain_io_errors_and_directories_are_corrupt_objects() {
    let root = test_root("nonregular-read");
    let store = ObjectStore::create(&root).unwrap();
    let content = store.publish(b"original", "text/plain").unwrap();
    let path = store.path_for(content.digest);
    fs::remove_file(&path).unwrap();
    assert!(
        matches!(store.read(&content), Err(StoreError::Io(error)) if error.kind()==std::io::ErrorKind::NotFound)
    );
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.publish(b"original", "text/plain"),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn accepted_reads_do_not_follow_symlinks_outside_the_cas() {
    let root = test_root("symlink-read");
    let store = ObjectStore::create(&root).unwrap();
    let content = store.publish(b"original", "text/plain").unwrap();
    let path = store.path_for(content.digest);
    fs::remove_file(&path).unwrap();
    let external = root.join("outside-object");
    fs::write(&external, b"original").unwrap();
    std::os::unix::fs::symlink(&external, &path).unwrap();
    assert!(matches!(
        store.read(&content),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.verify_bounded(&content, 8),
        Err(StoreError::CorruptObject { .. })
    ));
    assert!(matches!(
        store.publish(b"original", "text/plain"),
        Err(StoreError::CorruptObject { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}
