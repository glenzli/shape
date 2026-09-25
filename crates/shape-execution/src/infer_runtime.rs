//! Infer Runtime official-SDK integration and Shape-owned execution adapters.

use std::path::PathBuf;

use infer_runtime_client::{
    CAPABILITY_CATALOG_SCHEMA, CAPABILITY_CATALOG_VERSION, CONSUMER_CORE, CONSUMER_CORE_PROTOCOL,
    DiscoveryResolver, Error as SdkError, ResolvedEndpoint,
};
use thiserror::Error;

mod agent_file;
mod credential;
mod image_generation;
mod job_provenance;
mod responses;
mod sdk;
pub(crate) mod speech;

#[cfg(test)]
mod test_support;

pub use agent_file::InferRuntimeAgentFileExecutor;
pub use credential::{InferRuntimeCredentialError, InferRuntimeCredentialStore};
pub use image_generation::{IMAGE_GENERATE_CAPABILITY, InferRuntimeImageGenerationExecutor};
pub use responses::InferRuntimeExecutor;
pub use speech::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    INFER_SPEECH_VOICE_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1, InferRuntimeSpeechExecutor, SPEECH_PRESETS,
    SpeechPreset, SpeechSynthesisControl, supported_speech_operation,
};

use sdk::{InferRuntimeSdk, OfficialSdkClient, SdkAdapterError};

/// Exact Consumer Core identity implemented by the frozen official SDK.
pub const INFER_RUNTIME_CONTRACT_VERSION: &str = CONSUMER_CORE;
/// Exact Capability Catalog identity required by this Shape build.
pub const INFER_RUNTIME_CAPABILITY_CATALOG: &str = "infer-runtime.capability-catalog@20260813.1";

/// Exact stable Responses capability consumed by Shape.
pub const INFER_RUNTIME_RESPONSES_CAPABILITY: &str = "infer.responses@20260812.1";
/// Exact stable unary speech capability consumed by Shape.
pub const INFER_RUNTIME_SPEECH_CAPABILITY: &str = "infer.audio.speech@20260811.1";
/// Exact stable Agent file-task capability consumed by Shape.
pub const INFER_RUNTIME_AGENT_TASK_CAPABILITY: &str = "infer.agent.task@20260925.1";

/// Validated public contracts of one compatible Infer Runtime instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferRuntimeContract {
    /// Exact dated Consumer Core identity.
    pub contract_version: String,
    /// Exact dated Capability Catalog identity.
    pub capability_catalog: String,
}

/// Final endpoint identity and public contract result from one bounded probe.
#[derive(Debug)]
pub struct InferRuntimeProbe {
    /// Endpoint selected by the official SDK resolver.
    pub endpoint: Option<ResolvedInferRuntimeEndpoint>,
    /// Compatible Core/Catalog or a payload-free stable failure.
    pub contract: Result<InferRuntimeContract, InferRuntimeClientError>,
}

/// Authority used by the official SDK to select the Consumer endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferRuntimeEndpointSource {
    /// Explicit development or diagnostics override.
    ExplicitOverride,
    /// Owner-only Infra Discovery registration.
    Discovery,
}

impl InferRuntimeEndpointSource {
    /// Returns a language-neutral discriminator for the desktop projection.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExplicitOverride => "explicit_override",
            Self::Discovery => "discovery",
        }
    }
}

/// Payload-free projection of the endpoint selected by the official SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInferRuntimeEndpoint {
    /// Canonical numeric-loopback origin.
    pub origin: String,
    /// Selection authority.
    pub source: InferRuntimeEndpointSource,
    /// Discovery instance identity, absent for an explicit override.
    pub instance_id: Option<String>,
    /// Discovery generation, absent for an explicit override.
    pub generation: Option<String>,
    /// Exact selected Consumer Core identity.
    pub contract_version: Option<String>,
}

/// Stable failures for the desktop availability projection.
///
/// SDK error messages and HTTP bodies never enter this type.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InferRuntimeClientError {
    /// The explicit endpoint or Discovery registration was rejected by the SDK.
    #[error("Infer Runtime endpoint or Discovery registration is invalid")]
    InvalidEndpoint,
    /// No valid HTTP response arrived within the bounded request window.
    #[error("Infer Runtime is unavailable")]
    Unavailable,
    /// The Runtime returned a stable public API error other than incompatibility.
    #[error("Infer Runtime contract endpoint returned HTTP {status} with code {code}")]
    UnexpectedStatus { status: u16, code: String },
    /// The response was malformed or failed an immutable schema digest check.
    #[error("Infer Runtime returned an invalid contract artifact")]
    InvalidContract,
    /// The Runtime does not implement the exact dated Consumer Core.
    #[error("Infer Runtime contract is not supported: {actual}")]
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

/// Resolves Infer Runtime with the official SDK and validates exact Core,
/// Catalog, Responses, and unary speech artifacts.
#[must_use]
pub fn probe_infer_runtime_contract(explicit_override: &str) -> InferRuntimeProbe {
    let (resolver, source) = match sdk_resolver(explicit_override) {
        Ok(resolved) => resolved,
        Err(error) => {
            return InferRuntimeProbe {
                endpoint: None,
                contract: Err(error),
            };
        }
    };
    let endpoint = match resolver.resolve() {
        Ok(endpoint) => endpoint_projection(endpoint, source),
        Err(error) => {
            return InferRuntimeProbe {
                endpoint: None,
                contract: Err(map_probe_sdk_error(SdkAdapterError::Sdk(error))),
            };
        }
    };
    let sdk = match OfficialSdkClient::new(resolver, None) {
        Ok(sdk) => sdk,
        Err(error) => {
            return InferRuntimeProbe {
                endpoint: Some(endpoint),
                contract: Err(map_probe_sdk_error(error)),
            };
        }
    };
    InferRuntimeProbe {
        endpoint: Some(endpoint),
        contract: probe_contract_with_sdk(&sdk),
    }
}

fn probe_contract_with_sdk(
    sdk: &dyn InferRuntimeSdk,
) -> Result<InferRuntimeContract, InferRuntimeClientError> {
    let manifest = sdk.contract().map_err(map_probe_sdk_error)?;
    let catalog = sdk.capabilities().map_err(map_probe_sdk_error)?;
    catalog
        .require_exact(INFER_RUNTIME_RESPONSES_CAPABILITY)
        .and_then(|()| catalog.require_exact(INFER_RUNTIME_SPEECH_CAPABILITY))
        .map_err(|error| map_probe_sdk_error(SdkAdapterError::Sdk(error)))?;
    Ok(InferRuntimeContract {
        contract_version: manifest.core_contract,
        capability_catalog: format!("{CAPABILITY_CATALOG_SCHEMA}@{CAPABILITY_CATALOG_VERSION}"),
    })
}

fn sdk_resolver(
    explicit_override: &str,
) -> Result<(DiscoveryResolver, InferRuntimeEndpointSource), InferRuntimeClientError> {
    if explicit_override.is_empty() {
        return Ok((
            DiscoveryResolver::local(),
            InferRuntimeEndpointSource::Discovery,
        ));
    }
    DiscoveryResolver::local()
        .with_explicit_endpoint(explicit_override)
        .map(|resolver| (resolver, InferRuntimeEndpointSource::ExplicitOverride))
        .map_err(|_| InferRuntimeClientError::InvalidEndpoint)
}

fn official_sdk(
    explicit_override: &str,
    credential_path: PathBuf,
) -> Result<OfficialSdkClient, SdkAdapterError> {
    let (resolver, _) =
        sdk_resolver(explicit_override).map_err(|_| SdkAdapterError::InvalidEndpoint)?;
    OfficialSdkClient::new(resolver, Some(credential_path))
}

fn endpoint_projection(
    endpoint: ResolvedEndpoint,
    source: InferRuntimeEndpointSource,
) -> ResolvedInferRuntimeEndpoint {
    let discovered = source == InferRuntimeEndpointSource::Discovery;
    ResolvedInferRuntimeEndpoint {
        origin: endpoint.endpoint,
        source,
        instance_id: discovered.then_some(endpoint.instance_id),
        generation: discovered.then_some(endpoint.generation),
        contract_version: Some(format!(
            "{CONSUMER_CORE_PROTOCOL}@{}",
            endpoint.core_version
        )),
    }
}

fn map_probe_sdk_error(error: SdkAdapterError) -> InferRuntimeClientError {
    match error {
        SdkAdapterError::InvalidEndpoint | SdkAdapterError::RuntimeUnavailable => {
            InferRuntimeClientError::InvalidEndpoint
        }
        SdkAdapterError::Timeout | SdkAdapterError::Sdk(SdkError::Transport(_)) => {
            InferRuntimeClientError::Unavailable
        }
        SdkAdapterError::Sdk(SdkError::Discovery(_)) => InferRuntimeClientError::InvalidEndpoint,
        SdkAdapterError::Sdk(SdkError::ContractMismatch) => {
            InferRuntimeClientError::IncompatibleContract {
                actual: "contract_mismatch".to_owned(),
            }
        }
        SdkAdapterError::Sdk(SdkError::Api { code, .. })
            if matches!(
                code.as_str(),
                "consumer_core_unsupported" | "capability_contract_unsupported"
            ) =>
        {
            InferRuntimeClientError::IncompatibleContract { actual: code }
        }
        SdkAdapterError::Sdk(SdkError::Api { status, code, .. }) => {
            InferRuntimeClientError::UnexpectedStatus {
                status: status.as_u16(),
                code,
            }
        }
        SdkAdapterError::Sdk(
            SdkError::MalformedResponse(_) | SdkError::Credential(_) | SdkError::Input(_),
        ) => InferRuntimeClientError::InvalidContract,
    }
}

#[cfg(test)]
mod tests;
