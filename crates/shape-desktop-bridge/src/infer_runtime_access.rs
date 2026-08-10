//! Shared owner-only Infer credential access for authenticated desktop consumers.

use std::io::ErrorKind;

use shape_execution::{InferRuntimeCredentialError, InferRuntimeCredentialStore};

use crate::ffi;

pub(super) fn infer_runtime_credential_status(path: &str) -> ffi::InferRuntimeCredentialStatusWire {
    match InferRuntimeCredentialStore::new(path).is_available() {
        Ok(configured) => ffi::InferRuntimeCredentialStatusWire {
            configured,
            error_code: String::new(),
        },
        Err(error) => ffi::InferRuntimeCredentialStatusWire {
            configured: false,
            error_code: credential_error_code(&error).to_owned(),
        },
    }
}

pub(super) fn install_infer_runtime_credential(path: &str, token: &str) -> Result<(), String> {
    InferRuntimeCredentialStore::new(path)
        .install(token)
        .map_err(|error| credential_error_code(&error).to_owned())
}

pub(super) fn credential_error_code(error: &InferRuntimeCredentialError) -> &'static str {
    match error {
        InferRuntimeCredentialError::Io { source, .. } if source.kind() == ErrorKind::NotFound => {
            "credential_missing"
        }
        InferRuntimeCredentialError::InvalidPath => "credential_path_invalid",
        InferRuntimeCredentialError::InvalidToken => "credential_invalid",
        InferRuntimeCredentialError::UnsafeObject => "credential_unsafe",
        InferRuntimeCredentialError::UnsupportedPlatform => "credential_unsupported",
        InferRuntimeCredentialError::Io { .. } => "credential_unavailable",
    }
}

#[cfg(test)]
mod tests;
