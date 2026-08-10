//! Strict Infra Discovery resolver for Infer Runtime's Consumer API.

use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[cfg(any(target_os = "macos", windows))]
use std::process::Command;

use serde::Deserialize;
use thiserror::Error;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use super::{INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClientError, canonical_loopback_url};

/// Fixed endpoint retained only while existing Consumers migrate to Discovery.
pub const INFER_RUNTIME_COMPATIBILITY_ENDPOINT: &str = "http://127.0.0.1:8787";

const DISCOVERY_SCHEMA: &str = "infra.discovery.registration";
const DISCOVERY_SCHEMA_VERSION: &str = "20260810.1";
const SERVICE_KIND: &str = "infer-runtime";
const SERVICE_INSTANCE_ID: &str = "local";
const CONSUMER_PROTOCOL: &str = "infer-runtime.consumer";
const CONSUMER_BINDING: &str = "infer-runtime.http-loopback";
const MANIFEST_FILENAME: &str = "infer-runtime--local.json";
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_OFFERS: usize = 64;
const MAX_PROTOCOL_VERSIONS: usize = 16;

/// Authority used to select an Infer Runtime Consumer endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferRuntimeEndpointSource {
    /// An explicit Shape development or diagnostics override.
    ExplicitOverride,
    /// A live, owner-only Infra Discovery registration.
    Discovery,
    /// The temporary fixed-port migration fallback.
    CompatibilityFallback,
}

impl InferRuntimeEndpointSource {
    /// Returns a language-neutral source identity for diagnostics and bridges.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExplicitOverride => "explicit_override",
            Self::Discovery => "discovery",
            Self::CompatibilityFallback => "compatibility_fallback",
        }
    }
}

/// One validated endpoint selection and its Discovery restart identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInferRuntimeEndpoint {
    /// Canonical numeric-loopback HTTP origin without a trailing slash.
    pub origin: String,
    /// Authority from which this endpoint was selected.
    pub source: InferRuntimeEndpointSource,
    /// Stable Discovery instance when `source` is Discovery.
    pub instance_id: Option<String>,
    /// Opaque per-process Discovery generation when available.
    pub generation: Option<String>,
    /// Validated lease expiry as a Unix timestamp when discovered.
    pub lease_expires_at_unix: Option<i64>,
}

/// Resolves Infer Runtime through explicit override, Infra Discovery, then the
/// temporary fixed-port compatibility fallback.
///
/// Discovery never carries credentials. A connection failure causes at most
/// one immediate re-read; an unchanged endpoint is not retried indefinitely.
#[derive(Debug)]
pub struct InferRuntimeEndpointResolver {
    explicit_override: Option<String>,
    runtime_root: Option<PathBuf>,
    fallback: String,
    cached: Mutex<Option<ResolvedInferRuntimeEndpoint>>,
}

impl InferRuntimeEndpointResolver {
    /// Creates a resolver from Shape's explicit endpoint override and the
    /// platform Infra Protocol runtime-root contract.
    #[must_use]
    pub fn from_environment(explicit_override: &str) -> Self {
        Self {
            explicit_override: (!explicit_override.is_empty())
                .then(|| explicit_override.to_owned()),
            runtime_root: runtime_root_from_environment().ok(),
            fallback: INFER_RUNTIME_COMPATIBILITY_ENDPOINT.to_owned(),
            cached: Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub(super) fn with_runtime_root(
        explicit_override: &str,
        runtime_root: PathBuf,
        fallback: &str,
    ) -> Self {
        Self {
            explicit_override: (!explicit_override.is_empty())
                .then(|| explicit_override.to_owned()),
            runtime_root: Some(runtime_root),
            fallback: fallback.to_owned(),
            cached: Mutex::new(None),
        }
    }

    /// Resolves the currently authoritative endpoint and validates explicit
    /// configuration before any HTTP request is built.
    ///
    /// # Errors
    ///
    /// Returns [`InferRuntimeClientError::InvalidEndpoint`] when an explicit
    /// override or the compile-time compatibility endpoint is not canonical.
    pub fn resolve(&self) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeClientError> {
        self.resolve_at(OffsetDateTime::now_utc())
    }

    /// Re-reads Discovery once after a connection failure. A new generation or
    /// endpoint is selected; otherwise the compatibility fallback is returned.
    ///
    /// # Errors
    ///
    /// Returns [`InferRuntimeClientError::InvalidEndpoint`] only for invalid
    /// explicit or compile-time endpoint configuration.
    pub fn resolve_after_connection_failure(
        &self,
        failed: &ResolvedInferRuntimeEndpoint,
    ) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeClientError> {
        if self.explicit_override.is_some() {
            return self.resolve();
        }
        let now = OffsetDateTime::now_utc();
        let rediscovered = self
            .runtime_root
            .as_deref()
            .and_then(|root| discover_endpoint(root, now).ok());
        let endpoint = match rediscovered {
            Some(candidate) if candidate != *failed => candidate,
            Some(_) | None => self.fallback()?,
        };
        self.remember(endpoint.clone());
        Ok(endpoint)
    }

    fn resolve_at(
        &self,
        now: OffsetDateTime,
    ) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeClientError> {
        if let Some(origin) = &self.explicit_override {
            canonical_loopback_url(origin)?;
            let endpoint = ResolvedInferRuntimeEndpoint {
                origin: origin.clone(),
                source: InferRuntimeEndpointSource::ExplicitOverride,
                instance_id: None,
                generation: None,
                lease_expires_at_unix: None,
            };
            self.remember(endpoint.clone());
            return Ok(endpoint);
        }

        let endpoint = self
            .runtime_root
            .as_deref()
            .and_then(|root| discover_endpoint(root, now).ok())
            .map_or_else(|| self.fallback(), Ok)?;
        self.remember(endpoint.clone());
        Ok(endpoint)
    }

    fn fallback(&self) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeClientError> {
        canonical_loopback_url(&self.fallback)?;
        Ok(ResolvedInferRuntimeEndpoint {
            origin: self.fallback.clone(),
            source: InferRuntimeEndpointSource::CompatibilityFallback,
            instance_id: None,
            generation: None,
            lease_expires_at_unix: None,
        })
    }

    fn remember(&self, endpoint: ResolvedInferRuntimeEndpoint) {
        *self
            .cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(endpoint);
    }

    #[cfg(test)]
    fn cached_endpoint(&self) -> Option<ResolvedInferRuntimeEndpoint> {
        self.cached
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

fn discover_endpoint(
    runtime_root: &Path,
    now: OffsetDateTime,
) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeDiscoveryError> {
    let registration = read_registration(runtime_root)?;
    registration.select_consumer(now)
}

#[cfg(unix)]
fn read_registration(runtime_root: &Path) -> Result<Registration, InferRuntimeDiscoveryError> {
    use rustix::fs::{Mode, OFlags, open, openat};

    let directory_flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW;
    let root = fs::File::from(
        open(runtime_root, directory_flags, Mode::empty())
            .map_err(|source| InferRuntimeDiscoveryError::Io(source.into()))?,
    );
    validate_unix_directory(&root)?;
    let registrations = fs::File::from(
        openat(&root, "registrations", directory_flags, Mode::empty())
            .map_err(|source| InferRuntimeDiscoveryError::Io(source.into()))?,
    );
    validate_unix_directory(&registrations)?;
    let sockets = fs::File::from(
        openat(&root, "sockets", directory_flags, Mode::empty())
            .map_err(|source| InferRuntimeDiscoveryError::Io(source.into()))?,
    );
    validate_unix_directory(&sockets)?;

    let mut manifest = fs::File::from(
        openat(
            &registrations,
            MANIFEST_FILENAME,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|source| InferRuntimeDiscoveryError::Io(source.into()))?,
    );
    validate_unix_manifest(&manifest)?;
    parse_manifest(&mut manifest)
}

#[cfg(unix)]
fn validate_unix_directory(file: &fs::File) -> Result<(), InferRuntimeDiscoveryError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(InferRuntimeDiscoveryError::Io)?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(InferRuntimeDiscoveryError::UnsafeObject);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_unix_manifest(file: &fs::File) -> Result<(), InferRuntimeDiscoveryError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(InferRuntimeDiscoveryError::Io)?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o600
    {
        return Err(InferRuntimeDiscoveryError::UnsafeObject);
    }
    if metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(InferRuntimeDiscoveryError::ManifestTooLarge);
    }
    Ok(())
}

#[cfg(windows)]
fn read_registration(runtime_root: &Path) -> Result<Registration, InferRuntimeDiscoveryError> {
    validate_windows_private_object(runtime_root, true)?;
    let registrations = runtime_root.join("registrations");
    validate_windows_private_object(&registrations, true)?;
    let manifest_path = registrations.join(MANIFEST_FILENAME);
    validate_windows_private_object(&manifest_path, false)?;
    let mut manifest = fs::File::open(&manifest_path).map_err(InferRuntimeDiscoveryError::Io)?;
    let metadata = manifest
        .metadata()
        .map_err(InferRuntimeDiscoveryError::Io)?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(InferRuntimeDiscoveryError::UnsafeObject);
    }
    parse_manifest(&mut manifest)
}

#[cfg(windows)]
fn validate_windows_private_object(
    path: &Path,
    expect_directory: bool,
) -> Result<(), InferRuntimeDiscoveryError> {
    const VALIDATE_ACL: &str = r#"
$ErrorActionPreference = 'Stop'
$item = Get-Item -LiteralPath $env:SHAPE_INFRA_DISCOVERY_PATH -Force
if ([bool]($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { exit 10 }
if ($item.PSIsContainer -ne ($env:SHAPE_INFRA_DISCOVERY_DIRECTORY -eq '1')) { exit 11 }
$current = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$system = 'S-1-5-18'
$administrators = 'S-1-5-32-544'
$acl = Get-Acl -LiteralPath $item.FullName
$owner = $acl.Owner
try { $owner = (New-Object Security.Principal.NTAccount($owner)).Translate([Security.Principal.SecurityIdentifier]).Value } catch {}
if ($owner -ne $current) { exit 12 }
foreach ($rule in $acl.Access) {
  if ($rule.AccessControlType -ne [Security.AccessControl.AccessControlType]::Allow) { continue }
  $sid = $rule.IdentityReference
  try { $sid = $sid.Translate([Security.Principal.SecurityIdentifier]).Value } catch { exit 13 }
  if ($sid -ne $current -and $sid -ne $system -and $sid -ne $administrators) { exit 14 }
}
"#;
    let status = powershell()
        .args(["-NoProfile", "-NonInteractive", "-Command", VALIDATE_ACL])
        .env("SHAPE_INFRA_DISCOVERY_PATH", path)
        .env(
            "SHAPE_INFRA_DISCOVERY_DIRECTORY",
            if expect_directory { "1" } else { "0" },
        )
        .status()
        .map_err(InferRuntimeDiscoveryError::Io)?;
    if !status.success() {
        return Err(InferRuntimeDiscoveryError::UnsafeObject);
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn read_registration(_: &Path) -> Result<Registration, InferRuntimeDiscoveryError> {
    Err(InferRuntimeDiscoveryError::RuntimeRootUnavailable)
}

fn parse_manifest(file: &mut fs::File) -> Result<Registration, InferRuntimeDiscoveryError> {
    let mut bytes = Vec::new();
    file.take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(InferRuntimeDiscoveryError::Io)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(InferRuntimeDiscoveryError::ManifestTooLarge);
    }
    serde_json::from_slice(&bytes).map_err(InferRuntimeDiscoveryError::InvalidJson)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema: String,
    schema_version: String,
    service: Service,
    lease: Lease,
    offers: Vec<Offer>,
}

impl Registration {
    fn select_consumer(
        &self,
        now: OffsetDateTime,
    ) -> Result<ResolvedInferRuntimeEndpoint, InferRuntimeDiscoveryError> {
        if self.schema != DISCOVERY_SCHEMA
            || self.schema_version != DISCOVERY_SCHEMA_VERSION
            || self.service.kind != SERVICE_KIND
            || self.service.instance_id != SERVICE_INSTANCE_ID
            || !valid_service_kind(&self.service.kind)
            || !valid_file_token(&self.service.instance_id, 96)
            || !valid_file_token(&self.service.generation, 96)
            || self.offers.is_empty()
            || self.offers.len() > MAX_OFFERS
        {
            return Err(InferRuntimeDiscoveryError::InvalidRegistration);
        }

        let renewed_at = parse_time(&self.lease.renewed_at)?;
        let expires_at = parse_time(&self.lease.expires_at)?;
        if renewed_at >= expires_at
            || expires_at - renewed_at > Duration::seconds(120)
            || renewed_at > now + Duration::seconds(15)
            || expires_at > now + Duration::seconds(120)
            || expires_at <= now
        {
            return Err(InferRuntimeDiscoveryError::InvalidLease);
        }

        for offer in &self.offers {
            offer.validate()?;
        }
        let offer = self
            .offers
            .iter()
            .find(|offer| {
                offer.protocol == CONSUMER_PROTOCOL
                    && offer.binding == CONSUMER_BINDING
                    && offer
                        .protocol_versions
                        .iter()
                        .any(|version| version == INFER_RUNTIME_CONTRACT_VERSION)
            })
            .ok_or(InferRuntimeDiscoveryError::NoCompatibleOffer)?;
        canonical_loopback_url(&offer.endpoint)
            .map_err(|_| InferRuntimeDiscoveryError::InvalidEndpoint)?;
        Ok(ResolvedInferRuntimeEndpoint {
            origin: offer.endpoint.clone(),
            source: InferRuntimeEndpointSource::Discovery,
            instance_id: Some(self.service.instance_id.clone()),
            generation: Some(self.service.generation.clone()),
            lease_expires_at_unix: Some(expires_at.unix_timestamp()),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Service {
    kind: String,
    instance_id: String,
    generation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lease {
    renewed_at: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    protocol: String,
    protocol_versions: Vec<String>,
    binding: String,
    endpoint: String,
}

impl Offer {
    fn validate(&self) -> Result<(), InferRuntimeDiscoveryError> {
        if !valid_contract_id(&self.protocol)
            || !valid_contract_id(&self.binding)
            || self.endpoint.is_empty()
            || self.endpoint.len() > 512
            || self.protocol_versions.is_empty()
            || self.protocol_versions.len() > MAX_PROTOCOL_VERSIONS
        {
            return Err(InferRuntimeDiscoveryError::InvalidRegistration);
        }
        let mut unique_versions = HashSet::new();
        if self.protocol_versions.iter().any(|version| {
            !valid_contract_version(version) || !unique_versions.insert(version.as_str())
        }) {
            return Err(InferRuntimeDiscoveryError::InvalidRegistration);
        }
        if (self.binding == "infra.local.unix-socket"
            && !valid_unix_socket_endpoint(&self.endpoint))
            || (self.binding == "infra.local.windows-named-pipe"
                && !valid_windows_pipe_endpoint(&self.endpoint))
            || (self.binding == CONSUMER_BINDING && canonical_loopback_url(&self.endpoint).is_err())
        {
            return Err(InferRuntimeDiscoveryError::InvalidRegistration);
        }
        Ok(())
    }
}

fn parse_time(value: &str) -> Result<OffsetDateTime, InferRuntimeDiscoveryError> {
    if value.len() > 40 {
        return Err(InferRuntimeDiscoveryError::InvalidLease);
    }
    OffsetDateTime::parse(value, &Rfc3339).map_err(|_| InferRuntimeDiscoveryError::InvalidLease)
}

fn valid_service_kind(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let mut segments = value.split(['.', '-']);
    segments.next().is_some_and(|segment| {
        segment
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) && segments.all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

fn valid_file_token(value: &str, maximum: usize) -> bool {
    valid_token(value, maximum, b"._-")
}

fn valid_contract_id(value: &str) -> bool {
    valid_token(value, 128, b"._:+/@%-")
}

fn valid_contract_version(value: &str) -> bool {
    valid_token(value, 64, b"._:+-")
}

fn valid_token(value: &str, maximum: usize, punctuation: &[u8]) -> bool {
    (1..=maximum).contains(&value.len())
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || punctuation.contains(&byte))
}

fn valid_unix_socket_endpoint(value: &str) -> bool {
    value
        .strip_prefix("sockets/")
        .and_then(|name| name.strip_suffix(".sock"))
        .is_some_and(|opaque| valid_file_token(opaque, 16))
}

fn valid_windows_pipe_endpoint(value: &str) -> bool {
    value
        .strip_prefix(r"\\.\pipe\infra-protocol\")
        .is_some_and(|opaque| valid_file_token(opaque, 64))
}

fn runtime_root_from_environment() -> Result<PathBuf, InferRuntimeDiscoveryError> {
    if let Some(override_root) = std::env::var_os("INFRA_PROTOCOL_RUNTIME_DIR") {
        let root = PathBuf::from(override_root);
        return root
            .is_absolute()
            .then_some(root)
            .ok_or(InferRuntimeDiscoveryError::RuntimeRootUnavailable);
    }
    platform_runtime_root()
}

#[cfg(target_os = "macos")]
fn platform_runtime_root() -> Result<PathBuf, InferRuntimeDiscoveryError> {
    let output = Command::new("/usr/bin/getconf")
        .arg("DARWIN_USER_TEMP_DIR")
        .output()
        .map_err(InferRuntimeDiscoveryError::Io)?;
    if !output.status.success() {
        return Err(InferRuntimeDiscoveryError::RuntimeRootUnavailable);
    }
    let base = String::from_utf8(output.stdout)
        .map_err(|_| InferRuntimeDiscoveryError::RuntimeRootUnavailable)?;
    let base = PathBuf::from(base.trim());
    base.is_absolute()
        .then(|| base.join("infra-protocol"))
        .ok_or(InferRuntimeDiscoveryError::RuntimeRootUnavailable)
}

#[cfg(target_os = "linux")]
fn platform_runtime_root() -> Result<PathBuf, InferRuntimeDiscoveryError> {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(InferRuntimeDiscoveryError::RuntimeRootUnavailable)?;
    Ok(base.join("infra-protocol"))
}

#[cfg(windows)]
fn platform_runtime_root() -> Result<PathBuf, InferRuntimeDiscoveryError> {
    let output = powershell()
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::GetFolderPath('LocalApplicationData')",
        ])
        .output()
        .map_err(InferRuntimeDiscoveryError::Io)?;
    if !output.status.success() {
        return Err(InferRuntimeDiscoveryError::RuntimeRootUnavailable);
    }
    let base = String::from_utf8(output.stdout)
        .map_err(|_| InferRuntimeDiscoveryError::RuntimeRootUnavailable)?;
    let base = PathBuf::from(base.trim());
    base.is_absolute()
        .then(|| base.join("Infra Protocol").join("Runtime"))
        .ok_or(InferRuntimeDiscoveryError::RuntimeRootUnavailable)
}

#[cfg(windows)]
fn powershell() -> Command {
    let executable = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .map(|root| {
            root.join("System32")
                .join("WindowsPowerShell")
                .join("v1.0")
                .join("powershell.exe")
        })
        .unwrap_or_else(|| PathBuf::from("powershell.exe"));
    Command::new(executable)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn platform_runtime_root() -> Result<PathBuf, InferRuntimeDiscoveryError> {
    Err(InferRuntimeDiscoveryError::RuntimeRootUnavailable)
}

#[derive(Debug, Error)]
enum InferRuntimeDiscoveryError {
    #[error("Infra Discovery runtime root is unavailable")]
    RuntimeRootUnavailable,
    #[error("Infra Discovery filesystem object is not owner-only")]
    UnsafeObject,
    #[error("Infra Discovery manifest exceeds 64 KiB")]
    ManifestTooLarge,
    #[error("Infra Discovery I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("Infra Discovery manifest is not strict contract JSON: {0}")]
    InvalidJson(#[source] serde_json::Error),
    #[error("Infra Discovery registration shape is invalid")]
    InvalidRegistration,
    #[error("Infra Discovery lease is invalid or expired")]
    InvalidLease,
    #[error("Infer Runtime has no compatible Consumer offer")]
    NoCompatibleOffer,
    #[error("Infer Runtime Consumer endpoint is invalid")]
    InvalidEndpoint,
}

#[cfg(test)]
mod tests;
