//! Loopback-only Infer Runtime consumer-contract discovery.

use std::{io::Read, net::IpAddr, time::Duration};

use reqwest::{Url, blocking::Client, redirect::Policy};
use serde::Deserialize;
use thiserror::Error;

/// Infer Runtime wire contract implemented by this Shape build.
pub const INFER_RUNTIME_CONTRACT_VERSION: &str = "0.1.0-candidate.2";

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

/// Stable failures for public contract discovery. Response payloads and URLs
/// are deliberately excluded so diagnostics cannot capture credentials later.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InferRuntimeClientError {
    /// Only a bare loopback HTTP origin is accepted for the first integration.
    #[error("Infer Runtime endpoint must be a loopback HTTP origin")]
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

/// Bounded client for Infer Runtime's unauthenticated public contract surface.
///
/// This owner does not load credentials or submit inference. Redirects are
/// disabled and the origin must be loopback so a configuration mistake cannot
/// turn a future probe into an arbitrary network request.
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
    /// is a bare `http://localhost`, IPv4 loopback, or IPv6 loopback origin.
    pub fn new(base_url: &str) -> Result<Self, InferRuntimeClientError> {
        let base_url =
            Url::parse(base_url).map_err(|_| InferRuntimeClientError::InvalidEndpoint)?;
        if base_url.scheme() != "http"
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
            || base_url.path() != "/"
            || !is_loopback_host(&base_url)
        {
            return Err(InferRuntimeClientError::InvalidEndpoint);
        }
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
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
        if manifest.contract_version != INFER_RUNTIME_CONTRACT_VERSION {
            return Err(InferRuntimeClientError::IncompatibleContract {
                actual: manifest.contract_version,
            });
        }
        let has_text_responses = manifest.consumer_routes.iter().any(|route| {
            route.method.eq_ignore_ascii_case("POST") && route.path == "/v1/responses"
        });
        if !has_text_responses {
            return Err(InferRuntimeClientError::InvalidContract);
        }
        Ok(InferRuntimeContract {
            contract_version: manifest.contract_version,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ContractManifest {
    contract_version: String,
    #[serde(default)]
    consumer_routes: Vec<ConsumerRoute>,
}

#[derive(Debug, Deserialize)]
struct ConsumerRoute {
    method: String,
    path: String,
}

fn is_loopback_host(url: &Url) -> bool {
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback()),
        None => false,
    }
}

#[cfg(test)]
mod tests;
