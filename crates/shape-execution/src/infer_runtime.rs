//! Infer Runtime endpoint discovery and public consumer-contract validation.

use std::{io::Read, net::SocketAddr, time::Duration};

use reqwest::{Url, blocking::Client, redirect::Policy};
use serde::Deserialize;
use thiserror::Error;

mod credential;
mod discovery;
mod responses;
mod speech;

pub use credential::{
    InferRuntimeCredential, InferRuntimeCredentialError, InferRuntimeCredentialStore,
};
pub use discovery::{
    INFER_RUNTIME_COMPATIBILITY_ENDPOINT, InferRuntimeEndpointResolver, InferRuntimeEndpointSource,
    ResolvedInferRuntimeEndpoint,
};
pub use responses::InferRuntimeExecutor;
pub use speech::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
    InferRuntimeSpeechExecutor,
};

/// Preferred Infer Runtime wire contract implemented by this Shape build.
pub const INFER_RUNTIME_CONTRACT_VERSION: &str = "0.1.0-candidate.3";

/// Previous Infer Runtime contract retained only for the coordinated migration.
const INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION: &str = "0.1.0-candidate.2";
const INFER_RUNTIME_CAPABILITY_SCALE_VERSION: &str = "20260811.1";

const CONTRACT_PATH: &str = "infer/v1/contract";
const MAX_CONTRACT_BYTES: u64 = 64 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const REQUEST_TIMEOUT: Duration = Duration::from_millis(1_200);

/// Validated public capabilities of one compatible Infer Runtime instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferRuntimeContract {
    /// Exact version returned by the runtime contract manifest.
    pub contract_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferRuntimeContractRevision {
    Candidate2,
    Candidate3,
}

impl InferRuntimeContractRevision {
    fn parse(value: &str) -> Option<Self> {
        match value {
            INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION => Some(Self::Candidate2),
            INFER_RUNTIME_CONTRACT_VERSION => Some(Self::Candidate3),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Candidate2 => INFER_RUNTIME_PREVIOUS_CONTRACT_VERSION,
            Self::Candidate3 => INFER_RUNTIME_CONTRACT_VERSION,
        }
    }

    const fn text_intent(self) -> &'static str {
        match self {
            Self::Candidate2 => "assistant.general",
            Self::Candidate3 => "language.respond",
        }
    }

    const fn capability_floor_metadata_key(self) -> &'static str {
        match self {
            Self::Candidate2 => "infer.quality_floor",
            Self::Candidate3 => "infer.capability_floor",
        }
    }

    const fn capable_level(self) -> &'static str {
        match self {
            Self::Candidate2 => "general",
            Self::Candidate3 => "capable",
        }
    }

    const fn job_capability_floor_key(self) -> &'static str {
        match self {
            Self::Candidate2 => "quality_floor",
            Self::Candidate3 => "capability_floor",
        }
    }

    fn valid_capability_level(self, value: &str) -> bool {
        match self {
            Self::Candidate2 => matches!(value, "basic" | "general" | "advanced" | "frontier"),
            Self::Candidate3 => matches!(
                value,
                "foundational" | "capable" | "advanced" | "expert" | "exceptional"
            ),
        }
    }
}

/// Final endpoint identity and public contract result from one bounded probe.
#[derive(Debug)]
pub struct InferRuntimeProbe {
    /// Endpoint ultimately attempted, absent only for invalid explicit config.
    pub endpoint: Option<ResolvedInferRuntimeEndpoint>,
    /// Compatible public contract or a payload-free stable failure.
    pub contract: Result<InferRuntimeContract, InferRuntimeClientError>,
}

/// Stable failures for public contract discovery. Response payloads and URLs
/// are deliberately excluded so diagnostics cannot capture credentials later.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InferRuntimeClientError {
    /// Only a canonical numeric-loopback HTTP origin is accepted.
    #[error("Infer Runtime endpoint must be a canonical numeric-loopback HTTP origin")]
    InvalidEndpoint,
    /// No valid HTTP response arrived within the bounded request window.
    #[error("Infer Runtime is unavailable")]
    Unavailable,
    /// The public contract endpoint returned a non-success status.
    #[error("Infer Runtime contract endpoint returned HTTP {status}")]
    UnexpectedStatus { status: u16 },
    /// The response was oversized, malformed, or lacked the required route.
    #[error("Infer Runtime returned an invalid contract manifest")]
    InvalidContract,
    /// The runtime is reachable but speaks a different contract revision.
    #[error("Infer Runtime contract {actual} is not supported")]
    IncompatibleContract { actual: String },
}

impl InferRuntimeClientError {
    /// Returns a language-neutral discriminator for bridge and UI policy.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidEndpoint => "invalid_endpoint",
            Self::Unavailable => "unavailable",
            Self::UnexpectedStatus { .. } => "unexpected_status",
            Self::InvalidContract => "invalid_contract",
            Self::IncompatibleContract { .. } => "incompatible_contract",
        }
    }
}

/// Bounded HTTP client for Infer Runtime's unauthenticated public contract.
///
/// This owner does not load credentials or submit inference. Redirects are
/// disabled, proxies are bypassed, and the raw origin must be canonical
/// numeric loopback so a configuration mistake cannot turn a future probe into
/// an arbitrary network request.
#[derive(Debug, Clone)]
pub struct InferRuntimeClient {
    base_url: Url,
    client: Client,
}

impl InferRuntimeClient {
    /// Creates a loopback-only client with bounded connect and response time.
    ///
    /// # Errors
    ///
    /// Returns [`InferRuntimeClientError::InvalidEndpoint`] unless `base_url`
    /// is a canonical numeric IPv4 or IPv6 loopback HTTP origin with an
    /// explicit non-zero port and no trailing slash.
    pub fn new(base_url: &str) -> Result<Self, InferRuntimeClientError> {
        let base_url = canonical_loopback_url(base_url)?;
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .no_proxy()
            .user_agent("shape/0.1 infer-contract-probe")
            .build()
            .map_err(|_| InferRuntimeClientError::InvalidEndpoint)?;
        Ok(Self { base_url, client })
    }

    /// Reads and validates the public contract manifest.
    ///
    /// Unknown response fields are ignored for forward compatibility. The
    /// exact supported revision and the stable text Responses route must both
    /// be present before the runtime becomes eligible for later execution.
    ///
    /// # Errors
    ///
    /// Returns a stable discovery error without including response payloads.
    pub fn probe_contract(&self) -> Result<InferRuntimeContract, InferRuntimeClientError> {
        self.probe_contract_for_route("POST", "/v1/responses")
    }

    /// Validates the exact runtime revision and one required Consumer route.
    pub(crate) fn probe_contract_for_route(
        &self,
        method: &str,
        path: &str,
    ) -> Result<InferRuntimeContract, InferRuntimeClientError> {
        let endpoint = self
            .base_url
            .join(CONTRACT_PATH)
            .map_err(|_| InferRuntimeClientError::InvalidEndpoint)?;
        let response = self
            .client
            .get(endpoint)
            .send()
            .map_err(|_| InferRuntimeClientError::Unavailable)?;
        if !response.status().is_success() {
            return Err(InferRuntimeClientError::UnexpectedStatus {
                status: response.status().as_u16(),
            });
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_CONTRACT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| InferRuntimeClientError::InvalidContract)?;
        if bytes.len() as u64 > MAX_CONTRACT_BYTES {
            return Err(InferRuntimeClientError::InvalidContract);
        }
        let manifest: ContractManifest =
            serde_json::from_slice(&bytes).map_err(|_| InferRuntimeClientError::InvalidContract)?;
        if manifest.contract_version.is_empty() {
            return Err(InferRuntimeClientError::InvalidContract);
        }
        let revision =
            InferRuntimeContractRevision::parse(&manifest.contract_version).ok_or_else(|| {
                InferRuntimeClientError::IncompatibleContract {
                    actual: manifest.contract_version.clone(),
                }
            })?;
        if revision == InferRuntimeContractRevision::Candidate3
            && manifest.capability_scale_version.as_deref()
                != Some(INFER_RUNTIME_CAPABILITY_SCALE_VERSION)
        {
            return Err(InferRuntimeClientError::InvalidContract);
        }
        let has_required_route = manifest
            .consumer_routes
            .iter()
            .any(|route| route.method.eq_ignore_ascii_case(method) && route.path == path);
        if !has_required_route {
            return Err(InferRuntimeClientError::InvalidContract);
        }
        Ok(InferRuntimeContract {
            contract_version: manifest.contract_version,
        })
    }
}

/// Resolves Infer Runtime and validates its public contract in one bounded
/// Consumer lifecycle.
///
/// A transport failure triggers one immediate Discovery re-read. Shape retries
/// only when that produces a different endpoint or Discovery generation; the
/// temporary migration fallback never causes a duplicate hit to the same
/// failed address.
#[must_use]
pub fn probe_infer_runtime_contract(explicit_override: &str) -> InferRuntimeProbe {
    let resolver = InferRuntimeEndpointResolver::from_environment(explicit_override);
    probe_with_resolver(&resolver)
}

fn probe_with_resolver(resolver: &InferRuntimeEndpointResolver) -> InferRuntimeProbe {
    let mut endpoint = match resolver.resolve() {
        Ok(endpoint) => endpoint,
        Err(error) => {
            return InferRuntimeProbe {
                endpoint: None,
                contract: Err(error),
            };
        }
    };
    let mut contract = probe_endpoint(&endpoint);
    if matches!(contract, Err(InferRuntimeClientError::Unavailable))
        && let Ok(rediscovered) = resolver.resolve_after_connection_failure(&endpoint)
        && should_retry_endpoint(&endpoint, &rediscovered)
    {
        endpoint = rediscovered;
        contract = probe_endpoint(&endpoint);
    }
    InferRuntimeProbe {
        endpoint: Some(endpoint),
        contract,
    }
}

fn should_retry_endpoint(
    failed: &ResolvedInferRuntimeEndpoint,
    candidate: &ResolvedInferRuntimeEndpoint,
) -> bool {
    candidate.origin != failed.origin
        || (candidate.source == InferRuntimeEndpointSource::Discovery
            && (candidate.generation != failed.generation
                || candidate.contract_version != failed.contract_version))
}

fn probe_endpoint(
    endpoint: &ResolvedInferRuntimeEndpoint,
) -> Result<InferRuntimeContract, InferRuntimeClientError> {
    let contract = InferRuntimeClient::new(&endpoint.origin)?.probe_contract()?;
    validate_discovered_contract(endpoint, &contract)?;
    Ok(contract)
}

fn validate_discovered_contract(
    endpoint: &ResolvedInferRuntimeEndpoint,
    contract: &InferRuntimeContract,
) -> Result<InferRuntimeContractRevision, InferRuntimeClientError> {
    let revision =
        InferRuntimeContractRevision::parse(&contract.contract_version).ok_or_else(|| {
            InferRuntimeClientError::IncompatibleContract {
                actual: contract.contract_version.clone(),
            }
        })?;
    if endpoint
        .contract_version
        .as_deref()
        .is_some_and(|advertised| advertised != contract.contract_version)
    {
        return Err(InferRuntimeClientError::InvalidContract);
    }
    Ok(revision)
}

#[derive(Debug, Deserialize)]
struct ContractManifest {
    contract_version: String,
    #[serde(default)]
    capability_scale_version: Option<String>,
    #[serde(default)]
    consumer_routes: Vec<ConsumerRoute>,
}

#[derive(Debug, Deserialize)]
struct ConsumerRoute {
    method: String,
    path: String,
}

fn canonical_loopback_url(origin: &str) -> Result<Url, InferRuntimeClientError> {
    if origin.is_empty() || origin.trim() != origin {
        return Err(InferRuntimeClientError::InvalidEndpoint);
    }
    let authority = origin
        .strip_prefix("http://")
        .ok_or(InferRuntimeClientError::InvalidEndpoint)?;
    if authority.is_empty()
        || authority.contains(['/', '?', '#', '@'])
        || authority.chars().any(char::is_whitespace)
    {
        return Err(InferRuntimeClientError::InvalidEndpoint);
    }
    let address = authority
        .parse::<SocketAddr>()
        .map_err(|_| InferRuntimeClientError::InvalidEndpoint)?;
    if !address.ip().is_loopback() || address.port() == 0 || format!("http://{address}") != origin {
        return Err(InferRuntimeClientError::InvalidEndpoint);
    }
    Url::parse(origin).map_err(|_| InferRuntimeClientError::InvalidEndpoint)
}

#[cfg(test)]
mod tests;
