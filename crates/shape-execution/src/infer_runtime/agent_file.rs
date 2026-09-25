//! Exact accepted-file materialization through Infer's official Agent task SDK.

use std::{path::PathBuf, time::Duration};

use infer_runtime_client::{
    AGENT_TASK_INTENT, AgentTaskInputFile, AgentTaskRequest, AgentTaskResult, Error as SdkError,
};

use crate::{
    CapabilityId, ExecutionFailure, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
};

use super::{
    INFER_RUNTIME_CONTRACT_VERSION,
    job_provenance::{JobPolicyProfile, parse_job_snapshot, valid_job_id},
    official_sdk,
    sdk::{InferRuntimeSdk, SdkAdapterError, execution_failure},
};

const CAPABILITY: &str = "agent.file_task";
const MEDIA_TYPE: &str = "text/plain; charset=utf-8";
const TASK_TIMEOUT: Duration = Duration::from_mins(10);
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// An Agent-capable physical executor for one text/code file task.
pub struct InferRuntimeAgentFileExecutor {
    identity: ExecutorIdentity,
    sdk: Box<dyn InferRuntimeSdk>,
    extension: &'static str,
}

impl std::fmt::Debug for InferRuntimeAgentFileExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InferRuntimeAgentFileExecutor")
            .field("identity", &self.identity)
            .field("extension", &self.extension)
            .field("sdk", &"official-infer-runtime-client")
            .finish()
    }
}

impl InferRuntimeAgentFileExecutor {
    /// Creates an authenticated client. The extension is a bounded file hint
    /// inside Infer's private task workspace, never a host path.
    /// # Errors
    /// Rejects an unknown extension or unavailable SDK setup.
    pub fn new(
        explicit_override: &str,
        credential_path: impl Into<PathBuf>,
        extension: &str,
    ) -> Result<Self, crate::ExecutionError> {
        let extension =
            safe_extension(extension).ok_or(crate::ExecutionError::InvalidExecutorIdentity)?;
        let sdk = official_sdk(explicit_override, credential_path.into())
            .map_err(|_| crate::ExecutionError::InvalidExecutorIdentity)?;
        Ok(Self::with_sdk(Box::new(sdk), extension))
    }

    fn with_sdk(sdk: Box<dyn InferRuntimeSdk>, extension: &'static str) -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.infer-agent-file-task",
                env!("CARGO_PKG_VERSION"),
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("built-in Agent executor identity is valid"),
            sdk,
            extension,
        }
    }

    fn execute_file(
        &self,
        source: &[u8],
        instruction: &str,
    ) -> Result<ExecutionOutput, ExecutionFailure> {
        let input_path = format!("source.{}", self.extension);
        let output_path = format!("result.{}", self.extension);
        let input = AgentTaskInputFile::from_bytes(input_path, source)
            .map_err(|_| failure("infer_invalid_request", false))?;
        let task = AgentTaskRequest {
            model: AGENT_TASK_INTENT.into(),
            instruction: instruction.into(),
            input_files: vec![input],
            output_paths: vec![output_path.clone()],
        };
        let result = self
            .sdk
            .create_agent_task(&task, TASK_TIMEOUT)
            .map_err(map_dispatch_failure)?;
        let job_id = result.job_id.clone();
        self.validate_result(result, &output_path).map_err(|error| {
            if valid_job_id(&job_id) {
                error.with_executor_job_id(job_id)
            } else {
                error
            }
        })
    }

    fn validate_result(
        &self,
        result: AgentTaskResult,
        output_path: &str,
    ) -> Result<ExecutionOutput, ExecutionFailure> {
        if !valid_job_id(&result.job_id)
            || result.outputs.len() != 1
            || result.outputs[0].path != output_path
        {
            return Err(failure("infer_invalid_response", false));
        }
        let bytes = result.outputs[0]
            .decoded_bytes()
            .map_err(|_| failure("infer_invalid_response", false))?;
        if bytes.len() > MAX_OUTPUT_BYTES || std::str::from_utf8(&bytes).is_err() {
            return Err(failure("infer_invalid_response", false));
        }
        let job = self.sdk.job(&result.job_id).map_err(map_failure)?;
        let provenance = parse_job_snapshot(
            &result.job_id,
            AGENT_TASK_INTENT,
            JobPolicyProfile::AgentFileTask,
            job,
        )
        .map_err(|error| failure(error.code(), false))?;
        if result.provenance.provider != provenance.provider
            || result.provenance.deployment != provenance.deployment
            || result.provenance.model_build != provenance.model_build
        {
            return Err(failure("infer_invalid_response", false));
        }
        Ok(ExecutionOutput {
            bytes,
            media_type: MEDIA_TYPE.into(),
            executor_job_id: Some(result.job_id),
            external_provenance: Some(provenance),
            content_contract: None,
        })
    }
}

impl Executor for InferRuntimeAgentFileExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }
    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == CAPABILITY
    }
    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        if request.output_media_type != MEDIA_TYPE || request.inputs.len() != 1 {
            return Err(failure("infer_invalid_request", false));
        }
        let source = request.inputs[0]
            .bytes()
            .ok_or_else(|| failure("infer_invalid_request", false))?;
        let instruction = std::str::from_utf8(&request.instruction)
            .map_err(|_| failure("infer_invalid_request", false))?;
        self.execute_file(source, instruction)
    }
}

fn safe_extension(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "txt" => Some("txt"),
        "md" => Some("md"),
        "js" => Some("js"),
        "mjs" => Some("mjs"),
        "html" | "htm" => Some("html"),
        "css" => Some("css"),
        "json" => Some("json"),
        "svg" => Some("svg"),
        "py" => Some("py"),
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "rs" => Some("rs"),
        "sh" => Some("sh"),
        "xml" => Some("xml"),
        "yaml" => Some("yaml"),
        "yml" => Some("yml"),
        "csv" => Some("csv"),
        _ => None,
    }
}

fn map_failure(error: SdkAdapterError) -> ExecutionFailure {
    let (code, retryable) = execution_failure(error);
    failure(&code, retryable)
}

fn map_dispatch_failure(error: SdkAdapterError) -> ExecutionFailure {
    if matches!(
        error,
        SdkAdapterError::Timeout | SdkAdapterError::Sdk(SdkError::Transport(_))
    ) {
        failure("infer_outcome_unknown", false)
    } else {
        map_failure(error)
    }
}

fn failure(code: &str, retryable: bool) -> ExecutionFailure {
    ExecutionFailure::new(code, "Infer Runtime Agent file task failed", retryable)
}

#[cfg(test)]
mod tests;
