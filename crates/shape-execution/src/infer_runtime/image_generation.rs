//! Official-SDK-backed source-less `image.generate` Responses executor.
//!
//! This remains distinct from `image.edit`: Runtime publishes no stable typed
//! raster-edit capability, so material-conditioned image editing stays gated.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use infer_runtime_client::{ResponsesRequest, ResponsesResult};
use serde_json::{Value, json};
use shape_domain::{AiImageGenerateParameters, ArtifactContentContract, ImageRasterContract};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
    raster::normalize_generated_png,
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, parse_job_snapshot, valid_job_id},
    official_sdk,
    sdk::{InferRuntimeSdk, SdkAdapterError, execution_failure},
};

/// Shape-side logical capability implemented by Runtime `image.generate`.
pub const IMAGE_GENERATE_CAPABILITY: &str = "image.generate";

const IMAGE_INTENT: &str = "image.generate";
pub(super) const IMAGE_DEPLOYMENT: &str = "codex_gpt_5_6_luna";
const IMAGE_MEDIA_TYPE: &str = "image/png";
const MAX_INSTRUCTION_BYTES: usize = 32 * 1024;
const MAX_GENERATED_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MAX_GENERATED_IMAGE_DIMENSION: u32 = 4096;
const MAX_GENERATED_IMAGE_PIXELS: u64 = 16_777_216;
const MAX_BASE64_CHARS: usize = MAX_GENERATED_IMAGE_BYTES.div_ceil(3) * 4;
const REQUEST_TIMEOUT: Duration = Duration::from_mins(10);

/// Stable Core/Responses Consumer for one source-less generated raster.
pub struct InferRuntimeImageGenerationExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
    deployment: &'static str,
    effort: Option<&'static str>,
}

impl std::fmt::Debug for InferRuntimeImageGenerationExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeImageGenerationExecutor")
            .field("identity", &self.identity)
            .field("deployment", &self.deployment)
            .field("effort", &self.effort)
            .field("sdk", &"official-infer-runtime-client")
            .finish()
    }
}

impl InferRuntimeImageGenerationExecutor {
    /// Creates an official SDK Consumer using Shape's managed credential file.
    ///
    /// # Errors
    ///
    /// Returns an error only if the built-in identity or SDK adapter is invalid.
    pub fn new(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
    ) -> Result<Self, crate::ExecutionError> {
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk(Box::new(sdk)))
    }

    /// Selects an explicit, allowlisted image deployment for one request.
    ///
    /// # Errors
    ///
    /// Rejects unknown model keys or an invalid SDK adapter.
    pub fn new_with_model(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
        model_key: &str,
        effort_key: &str,
    ) -> Result<Self, crate::ExecutionError> {
        let deployment =
            image_deployment(model_key).ok_or(crate::ExecutionError::InvalidExecutorIdentity)?;
        let effort = image_effort(model_key, effort_key)
            .map_err(|()| crate::ExecutionError::InvalidExecutorIdentity)?;
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk_and_deployment(
            Box::new(sdk),
            deployment,
            effort,
        ))
    }

    fn with_sdk(sdk: Box<dyn InferRuntimeSdk>) -> Self {
        Self::with_sdk_and_deployment(sdk, IMAGE_DEPLOYMENT, None)
    }

    fn with_sdk_and_deployment(
        sdk: Box<dyn InferRuntimeSdk>,
        deployment: &'static str,
        effort: Option<&'static str>,
    ) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-runtime-image-generation-consumer",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("built-in Infer Runtime identity is valid"),
            sdk,
            deployment,
            effort,
        }
    }

    fn create_image(
        &self,
        parameters: &AiImageGenerateParameters,
    ) -> Result<ExecutionOutput, ExecutionFailure> {
        let response = self
            .sdk
            .create_response(
                &image_request_for_deployment(parameters, self.deployment, self.effort),
                REQUEST_TIMEOUT,
            )
            .map_err(map_failure)?;
        let parsed = parse_image_response(response)?;
        let job = self.sdk.job(&parsed.job_id).map_err(map_failure)?;
        let provenance = parse_job_snapshot(
            &parsed.job_id,
            IMAGE_INTENT,
            JobPolicyProfile::CloudImageInteractive(self.deployment),
            job,
        )
        .map_err(|error| failure(error.code(), false))?;
        Ok(ExecutionOutput {
            bytes: parsed.png,
            media_type: IMAGE_MEDIA_TYPE.to_owned(),
            executor_job_id: Some(parsed.job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::ImageRaster(parsed.contract)),
        })
    }
}

impl Executor for InferRuntimeImageGenerationExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == IMAGE_GENERATE_CAPABILITY
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.capability.as_str() != IMAGE_GENERATE_CAPABILITY
            || request.output_media_type != IMAGE_MEDIA_TYPE
            || !request.inputs.is_empty()
            || request.instruction.is_empty()
            || request.instruction.len() > MAX_INSTRUCTION_BYTES
        {
            return Err(failure("invalid_image_generation_request", false));
        }
        let parameters: AiImageGenerateParameters = serde_json::from_slice(&request.instruction)
            .map_err(|_| failure("invalid_image_generation_request", false))?;
        if parameters.candidate_count() != 1 {
            return Err(failure("unsupported_image_candidate_count", false));
        }
        self.create_image(&parameters)
    }
}

#[cfg(test)]
fn image_request(parameters: &AiImageGenerateParameters) -> ResponsesRequest {
    image_request_for_deployment(parameters, IMAGE_DEPLOYMENT, None)
}

fn image_deployment(model_key: &str) -> Option<&'static str> {
    match model_key {
        "gpt_5_6_luna" => Some(IMAGE_DEPLOYMENT),
        "gpt_6_luna" => Some("codex_gpt_6_luna"),
        "gpt_6_sol" => Some("codex_gpt_6_sol"),
        _ => None,
    }
}

fn image_effort(model_key: &str, effort_key: &str) -> Result<Option<&'static str>, ()> {
    let effort = match effort_key {
        "" => return Ok(None),
        "low" => "low",
        "medium" => "medium",
        "high" => "high",
        "xhigh" => "xhigh",
        "max" => "max",
        "ultra" if model_key == "gpt_6_sol" => "ultra",
        _ => return Err(()),
    };
    Ok(Some(effort))
}

fn image_request_for_deployment(
    parameters: &AiImageGenerateParameters,
    deployment: &'static str,
    effort: Option<&str>,
) -> ResponsesRequest {
    ResponsesRequest {
        model: IMAGE_INTENT.to_owned(),
        input: Value::String(parameters.instruction().to_owned()),
        instructions: Some(Value::String(format!(
            "Generate exactly one PNG image with a canvas of {} by {} pixels. Use the requested aspect ratio and size. Return the generated image through the image_generation tool.",
            parameters.output().width(),
            parameters.output().height()
        ))),
        stream: false,
        background: false,
        metadata: BTreeMap::from([
            ("infer.deployment_ids".to_owned(), deployment.to_owned()),
            ("infer.capability_floor".to_owned(), "capable".to_owned()),
            ("infer.fallback".to_owned(), "none".to_owned()),
            ("infer.max_cost_usd".to_owned(), "0".to_owned()),
            ("infer.offline_required".to_owned(), "false".to_owned()),
            ("infer.placement".to_owned(), "cloud_only".to_owned()),
            ("infer.policy".to_owned(), "balanced".to_owned()),
            ("infer.prefer".to_owned(), "cloud".to_owned()),
            ("infer.priority".to_owned(), "interactive".to_owned()),
            (
                "infer.provider_access_class".to_owned(),
                "subscription".to_owned(),
            ),
        ]),
        tools: vec![json!({"type": "image_generation"})],
        reasoning: effort.map(|key| json!({"effort": key})),
        max_output_tokens: None,
    }
}

#[derive(Debug)]
struct ParsedImageResponse {
    job_id: String,
    png: Vec<u8>,
    contract: ImageRasterContract,
}

fn parse_image_response(
    response: ResponsesResult,
) -> Result<ParsedImageResponse, ExecutionFailure> {
    let [image] = response.output.as_slice() else {
        return Err(failure("infer_invalid_response", false));
    };
    let kind = image.get("type").and_then(Value::as_str);
    let status = image.get("status").and_then(Value::as_str);
    let result = image.get("result").and_then(Value::as_str);
    if response.object != "response"
        || response.model != IMAGE_INTENT
        || response.status != "completed"
        || !valid_response_id(&response.id)
        || kind != Some("image_generation_call")
        || status != Some("completed")
        || result.is_none_or(|value| value.is_empty() || value.len() > MAX_BASE64_CHARS)
    {
        return Err(failure("infer_invalid_response", false));
    }
    let result = result.expect("validated image result exists");
    let decoded = STANDARD
        .decode(result)
        .map_err(|_| failure("invalid_generated_image", false))?;
    if decoded.is_empty()
        || decoded.len() > MAX_GENERATED_IMAGE_BYTES
        || STANDARD.encode(&decoded) != result
    {
        return Err(failure("invalid_generated_image", false));
    }
    let (png, contract) = normalize_generated_png(
        &decoded,
        MAX_GENERATED_IMAGE_BYTES,
        MAX_GENERATED_IMAGE_DIMENSION,
        MAX_GENERATED_IMAGE_PIXELS,
    )
    .map_err(|_| failure("invalid_generated_image", false))?;
    // The canvas is a generation request, not a deterministic resize contract.
    // Preserve native pixels; an explicit image.resize node owns exact sizing.
    Ok(ParsedImageResponse {
        job_id: response.id,
        png,
        contract,
    })
}

fn valid_response_id(value: &str) -> bool {
    value.starts_with("resp_") && valid_job_id(value)
}

fn map_failure(error: SdkAdapterError) -> ExecutionFailure {
    let (code, retryable) = execution_failure(error);
    failure(&code, retryable)
}

fn failure(code: &str, retryable: bool) -> ExecutionFailure {
    ExecutionFailure::new(
        code,
        "Infer Runtime could not produce an image candidate",
        retryable,
    )
}

#[cfg(test)]
mod tests;
