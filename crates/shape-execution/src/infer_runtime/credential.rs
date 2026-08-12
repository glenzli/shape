//! Owner-only storage for Shape's Infer Runtime bearer credential.

use std::{
    fs,
    io::{ErrorKind, Read, Write},
    path::{Path, PathBuf},
};

use thiserror::Error;
use uuid::Uuid;

const MANAGED_TOKEN_LENGTH: usize = 64;

/// Shape-owned location for the one-time managed token copied from Infer
/// Console. The path itself is configuration; token bytes never enter project
/// bundles, settings exports, command lines, environment variables, or logs.
#[derive(Debug, Clone)]
pub struct InferRuntimeCredentialStore {
    path: PathBuf,
}

impl InferRuntimeCredentialStore {
    /// Creates a credential owner at an application-selected absolute path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Reports whether one valid owner-only credential exists.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe path or malformed token. Absence is a
    /// normal unconfigured state.
    pub fn is_available(&self) -> Result<bool, InferRuntimeCredentialError> {
        match self.validate_installed() {
            Ok(()) => Ok(true),
            Err(InferRuntimeCredentialError::Io { source, .. })
                if source.kind() == ErrorKind::NotFound =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    /// Returns the absolute managed credential path for the official SDK.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured path is not absolute. The SDK owns
    /// opening, owner/mode verification, loading, and HTTP authentication.
    pub fn credential_path(&self) -> Result<PathBuf, InferRuntimeCredentialError> {
        validate_absolute_secret_path(&self.path)?;
        Ok(self.path.clone())
    }

    fn validate_installed(&self) -> Result<(), InferRuntimeCredentialError> {
        validate_absolute_secret_path(&self.path)?;
        let bytes = read_owner_only_secret(&self.path)?;
        let token =
            String::from_utf8(bytes).map_err(|_| InferRuntimeCredentialError::InvalidToken)?;
        validate_token(&token)?;
        Ok(())
    }

    /// Atomically installs or rotates the one-time managed token in Shape's
    /// owner-only secret directory.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid token, unsafe path, or durable write
    /// failure. The token is never included in an error.
    pub fn install(&self, token: &str) -> Result<(), InferRuntimeCredentialError> {
        validate_absolute_secret_path(&self.path)?;
        validate_token(token)?;
        write_owner_only_secret(&self.path, token.as_bytes())?;
        self.validate_installed()
    }
}

fn validate_absolute_secret_path(path: &Path) -> Result<(), InferRuntimeCredentialError> {
    if !path.is_absolute() || path.file_name().is_none() || path.parent().is_none() {
        return Err(InferRuntimeCredentialError::InvalidPath);
    }
    Ok(())
}

fn validate_token(token: &str) -> Result<(), InferRuntimeCredentialError> {
    if token.len() != MANAGED_TOKEN_LENGTH || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(InferRuntimeCredentialError::InvalidToken);
    }
    Ok(())
}

#[cfg(unix)]
fn read_owner_only_secret(path: &Path) -> Result<Vec<u8>, InferRuntimeCredentialError> {
    use rustix::fs::{Mode, OFlags, open, openat};

    let parent = path
        .parent()
        .ok_or(InferRuntimeCredentialError::InvalidPath)?;
    let filename = path
        .file_name()
        .ok_or(InferRuntimeCredentialError::InvalidPath)?;
    let directory = fs::File::from(
        open(
            parent,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|source| io_error("open secret directory", path, source.into()))?,
    );
    validate_unix_directory(&directory)?;
    let secret = fs::File::from(
        openat(
            &directory,
            filename,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|source| io_error("open secret", path, source.into()))?,
    );
    validate_unix_secret(&secret)?;
    let mut bytes = Vec::new();
    secret
        .take((MANAGED_TOKEN_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("read secret", path, source))?;
    if bytes.len() > MANAGED_TOKEN_LENGTH {
        return Err(InferRuntimeCredentialError::InvalidToken);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn write_owner_only_secret(path: &Path, bytes: &[u8]) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::PermissionsExt;

    use rustix::fs::{AtFlags, Mode, OFlags, open, openat, renameat, unlinkat};

    let parent = path
        .parent()
        .ok_or(InferRuntimeCredentialError::InvalidPath)?;
    fs::create_dir_all(parent)
        .map_err(|source| io_error("create secret directory", path, source))?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|source| io_error("secure secret directory", path, source))?;
    let directory = fs::File::from(
        open(
            parent,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|source| io_error("open secret directory", path, source.into()))?,
    );
    validate_unix_directory(&directory)?;

    let filename = path
        .file_name()
        .ok_or(InferRuntimeCredentialError::InvalidPath)?;
    let temporary = format!(".infer-runtime-token-{}.tmp", Uuid::now_v7());
    let write_result = (|| {
        let mut file = fs::File::from(
            openat(
                &directory,
                temporary.as_str(),
                OFlags::WRONLY | OFlags::CLOEXEC | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|source| io_error("create temporary secret", path, source.into()))?,
        );
        file.write_all(bytes)
            .map_err(|source| io_error("write temporary secret", path, source))?;
        file.sync_all()
            .map_err(|source| io_error("sync temporary secret", path, source))?;
        renameat(&directory, temporary.as_str(), &directory, filename)
            .map_err(|source| io_error("publish secret", path, source.into()))?;
        directory
            .sync_all()
            .map_err(|source| io_error("sync secret directory", path, source))
    })();
    if write_result.is_err() {
        let _ = unlinkat(&directory, temporary.as_str(), AtFlags::empty());
    }
    write_result
}

#[cfg(unix)]
fn validate_unix_directory(file: &fs::File) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file
        .metadata()
        .map_err(|source| io_error("inspect secret directory", Path::new("[secret]"), source))?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(InferRuntimeCredentialError::UnsafeObject);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_unix_secret(file: &fs::File) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file
        .metadata()
        .map_err(|source| io_error("inspect secret", Path::new("[secret]"), source))?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() != MANAGED_TOKEN_LENGTH as u64
    {
        return Err(InferRuntimeCredentialError::UnsafeObject);
    }
    Ok(())
}

#[cfg(windows)]
fn read_owner_only_secret(path: &Path) -> Result<Vec<u8>, InferRuntimeCredentialError> {
    validate_windows_private_object(path, false)?;
    let bytes = fs::read(path).map_err(|source| io_error("read secret", path, source))?;
    if bytes.len() > MANAGED_TOKEN_LENGTH {
        return Err(InferRuntimeCredentialError::InvalidToken);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn write_owner_only_secret(path: &Path, bytes: &[u8]) -> Result<(), InferRuntimeCredentialError> {
    let parent = path
        .parent()
        .ok_or(InferRuntimeCredentialError::InvalidPath)?;
    fs::create_dir_all(parent)
        .map_err(|source| io_error("create secret directory", path, source))?;
    secure_windows_object(parent, true)?;
    fs::write(path, bytes).map_err(|source| io_error("write secret", path, source))?;
    secure_windows_object(path, false)
}

#[cfg(windows)]
fn validate_windows_private_object(
    path: &Path,
    expect_directory: bool,
) -> Result<(), InferRuntimeCredentialError> {
    run_windows_acl_script(path, expect_directory, false)
}

#[cfg(windows)]
fn secure_windows_object(
    path: &Path,
    expect_directory: bool,
) -> Result<(), InferRuntimeCredentialError> {
    run_windows_acl_script(path, expect_directory, true)
}

#[cfg(windows)]
fn run_windows_acl_script(
    path: &Path,
    expect_directory: bool,
    repair: bool,
) -> Result<(), InferRuntimeCredentialError> {
    use std::process::Command;

    const ACL_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$item = Get-Item -LiteralPath $env:SHAPE_SECRET_PATH -Force
if ([bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { exit 10 }
if ($item.PSIsContainer -ne ($env:SHAPE_SECRET_DIRECTORY -eq '1')) { exit 11 }
$current = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
if ($env:SHAPE_SECRET_REPAIR -eq '1') {
  $acl = Get-Acl -LiteralPath $item.FullName
  $acl.SetAccessRuleProtection($true, $false)
  foreach ($rule in @($acl.Access)) { $acl.RemoveAccessRuleAll($rule) }
  $rights = if ($item.PSIsContainer) { 'FullControl' } else { 'Read,Write' }
  $rule = New-Object Security.AccessControl.FileSystemAccessRule($current, $rights, 'Allow')
  $acl.SetAccessRule($rule)
  Set-Acl -LiteralPath $item.FullName -AclObject $acl
}
$acl = Get-Acl -LiteralPath $item.FullName
$owner = $acl.Owner
try { $owner = (New-Object Security.Principal.NTAccount($owner)).Translate([Security.Principal.SecurityIdentifier]).Value } catch {}
if ($owner -ne $current) { exit 12 }
foreach ($rule in $acl.Access) {
  if ($rule.AccessControlType -ne [Security.AccessControl.AccessControlType]::Allow) { continue }
  $sid = $rule.IdentityReference
  try { $sid = $sid.Translate([Security.Principal.SecurityIdentifier]).Value } catch { exit 13 }
  if ($sid -ne $current -and $sid -ne 'S-1-5-18' -and $sid -ne 'S-1-5-32-544') { exit 14 }
}
"#;
    let executable = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| {
            root.join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe")
        })
        .unwrap_or_else(|| PathBuf::from("powershell.exe"));
    let status = Command::new(executable)
        .args(["-NoProfile", "-NonInteractive", "-Command", ACL_SCRIPT])
        .env("SHAPE_SECRET_PATH", path)
        .env(
            "SHAPE_SECRET_DIRECTORY",
            if expect_directory { "1" } else { "0" },
        )
        .env("SHAPE_SECRET_REPAIR", if repair { "1" } else { "0" })
        .status()
        .map_err(|source| io_error("validate secret ACL", path, source))?;
    if !status.success() {
        return Err(InferRuntimeCredentialError::UnsafeObject);
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn read_owner_only_secret(_: &Path) -> Result<Vec<u8>, InferRuntimeCredentialError> {
    Err(InferRuntimeCredentialError::UnsupportedPlatform)
}

#[cfg(not(any(unix, windows)))]
fn write_owner_only_secret(_: &Path, _: &[u8]) -> Result<(), InferRuntimeCredentialError> {
    Err(InferRuntimeCredentialError::UnsupportedPlatform)
}

fn io_error(
    operation: &'static str,
    path: &Path,
    source: std::io::Error,
) -> InferRuntimeCredentialError {
    InferRuntimeCredentialError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

/// Fail-closed credential storage failures. Token bytes are excluded from every
/// variant and display representation.
#[derive(Debug, Error)]
pub enum InferRuntimeCredentialError {
    #[error("Infer Runtime credential path must be absolute")]
    InvalidPath,
    #[error("Infer Runtime credential is not one managed 256-bit hexadecimal token")]
    InvalidToken,
    #[error("Infer Runtime credential storage object is not owner-only")]
    UnsafeObject,
    #[error("Infer Runtime credential storage is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("{operation} `{path}`: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests;
