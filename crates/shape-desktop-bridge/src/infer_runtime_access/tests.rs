use std::{fs, path::PathBuf};

use uuid::Uuid;

use super::*;

fn credential_path() -> PathBuf {
    std::env::temp_dir()
        .join(format!("shape-bridge-credential-{}", Uuid::now_v7()))
        .join("infer-runtime.token")
}

#[test]
fn credential_status_and_install_expose_only_stable_non_secret_state() {
    let path = credential_path();
    let path_text = path.to_str().expect("portable path");
    let missing = infer_runtime_credential_status(path_text);
    assert!(!missing.configured);
    assert!(missing.error_code.is_empty());

    assert_eq!(
        install_infer_runtime_credential(path_text, "invalid"),
        Err("credential_invalid".to_owned())
    );
    let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    install_infer_runtime_credential(path_text, token).expect("credential installs");
    let configured = infer_runtime_credential_status(path_text);
    assert!(configured.configured);
    assert!(configured.error_code.is_empty());
    assert!(!format!("{configured:?}").contains(token));

    fs::remove_dir_all(path.parent().expect("secret has parent")).expect("fixture removes");
}
