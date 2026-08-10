#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use uuid::Uuid;

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn credential_path(label: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("shape-infer-secret-{label}-{}", Uuid::now_v7()))
        .join("infer-runtime.token")
}

#[test]
fn install_creates_owner_only_storage_and_loads_a_redacted_credential() {
    let path = credential_path("install");
    let store = InferRuntimeCredentialStore::new(&path);
    assert!(!store.is_available().expect("absence is valid"));

    store.install(TOKEN).expect("credential installs");
    assert!(store.is_available().expect("credential validates"));
    assert_eq!(store.load().expect("credential loads").expose(), TOKEN);
    assert!(!format!("{:?}", store.load().expect("credential loads")).contains(TOKEN));
    assert_eq!(
        fs::metadata(path.parent().expect("secret has parent"))
            .expect("directory metadata reads")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&path)
            .expect("secret metadata reads")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}

#[test]
fn malformed_insecure_and_symlink_credentials_fail_closed() {
    let path = credential_path("unsafe");
    let store = InferRuntimeCredentialStore::new(&path);
    assert!(matches!(
        store.install("not-a-token"),
        Err(InferRuntimeCredentialError::InvalidToken)
    ));

    store.install(TOKEN).expect("credential installs");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("permissions change");
    assert!(matches!(
        store.load(),
        Err(InferRuntimeCredentialError::UnsafeObject)
    ));

    fs::remove_file(&path).expect("secret removes");
    let target = path.with_file_name("target");
    fs::write(&target, TOKEN).expect("target writes");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).expect("target secures");
    symlink(&target, &path).expect("symlink creates");
    assert!(store.load().is_err());

    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}

#[test]
fn rotation_replaces_exact_bytes_without_leaving_temporary_files() {
    let path = credential_path("rotation");
    let store = InferRuntimeCredentialStore::new(&path);
    store.install(TOKEN).expect("initial credential installs");
    let rotated = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    store.install(rotated).expect("credential rotates");
    assert_eq!(store.load().expect("credential loads").expose(), rotated);
    assert_eq!(
        fs::read_dir(path.parent().expect("secret has parent"))
            .expect("directory reads")
            .count(),
        1
    );

    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}
