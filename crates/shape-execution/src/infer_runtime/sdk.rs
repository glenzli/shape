//! Synchronous adaptation of the official asynchronous Infer Runtime SDK.

use std::{future::Future, path::PathBuf, sync::Mutex, time::Duration};

use infer_runtime_client::{
    AgentTaskRequest, AgentTaskResult, AudioBytesResponse, CapabilityCatalog, Client,
    ContractManifest, DiscoveryResolver, Error, JobSnapshot, PreparedSoundPrompt, ResponsesRequest,
    ResponsesResult, SoundGenerationRequest, SoundGenerationResponse, SpeechRequest,
};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

const CORE_TIMEOUT: Duration = Duration::from_secs(5);
const JOB_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub(super) enum SdkAdapterError {
    Sdk(Error),
    Timeout,
    Cancelled,
    InvalidEndpoint,
    RuntimeUnavailable,
}

pub(super) fn execution_failure(error: SdkAdapterError) -> (String, bool) {
    match error {
        SdkAdapterError::Cancelled => ("generation_cancelled".to_owned(), false),
        SdkAdapterError::InvalidEndpoint | SdkAdapterError::Sdk(Error::Discovery(_)) => {
            ("infer_invalid_endpoint".to_owned(), false)
        }
        SdkAdapterError::Timeout | SdkAdapterError::Sdk(Error::Transport(_)) => {
            ("infer_unavailable".to_owned(), true)
        }
        SdkAdapterError::RuntimeUnavailable => ("infer_client_invalid".to_owned(), false),
        SdkAdapterError::Sdk(Error::Credential(_)) => ("credential_unavailable".to_owned(), false),
        SdkAdapterError::Sdk(Error::Input(_)) => ("infer_invalid_request".to_owned(), false),
        SdkAdapterError::Sdk(Error::ContractMismatch) => {
            ("infer_incompatible_contract".to_owned(), false)
        }
        SdkAdapterError::Sdk(Error::MalformedResponse(_)) => {
            ("infer_invalid_response".to_owned(), false)
        }
        SdkAdapterError::Sdk(Error::Api { status, code, .. }) => {
            let stable = if valid_error_code(&code) {
                code
            } else {
                "infer_invalid_response".to_owned()
            };
            let retryable = matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
                || matches!(
                    stable.as_str(),
                    "provider_unavailable" | "upstream_rate_limited"
                );
            (stable, retryable)
        }
    }
}

fn valid_error_code(code: &str) -> bool {
    (1..=96).contains(&code.len())
        && code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

impl From<Error> for SdkAdapterError {
    fn from(error: Error) -> Self {
        Self::Sdk(error)
    }
}

pub(super) trait InferRuntimeSdk: Send + Sync {
    fn contract(&self) -> Result<ContractManifest, SdkAdapterError>;
    fn capabilities(&self) -> Result<CapabilityCatalog, SdkAdapterError>;
    fn create_response(
        &self,
        request: &ResponsesRequest,
        timeout: Duration,
    ) -> Result<ResponsesResult, SdkAdapterError>;
    fn synthesize_speech(
        &self,
        request: &SpeechRequest,
        timeout: Duration,
    ) -> Result<AudioBytesResponse, SdkAdapterError>;
    fn prepare_sound_prompt(
        &self,
        prompt: &str,
        timeout: Duration,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<PreparedSoundPrompt, SdkAdapterError>;
    fn sound_job(
        &self,
        id: &str,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<JobSnapshot, SdkAdapterError>;
    fn generate_sound(
        &self,
        request: &SoundGenerationRequest,
        timeout: Duration,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<SoundGenerationResponse, SdkAdapterError>;
    fn job(&self, job_id: &str) -> Result<JobSnapshot, SdkAdapterError>;
    fn create_agent_task(
        &self,
        request: &AgentTaskRequest,
        timeout: Duration,
    ) -> Result<AgentTaskResult, SdkAdapterError>;
}

pub(super) struct OfficialSdkClient {
    client: Client,
    runtime: Mutex<Runtime>,
}

impl std::fmt::Debug for OfficialSdkClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OfficialSdkClient")
            .field("client", &self.client)
            .field("runtime", &"tokio-current-thread")
            .finish()
    }
}

impl OfficialSdkClient {
    pub(super) fn new(
        resolver: DiscoveryResolver,
        credential_path: Option<PathBuf>,
    ) -> Result<Self, SdkAdapterError> {
        let builder = Client::with_discovery(resolver);
        let builder = match credential_path {
            Some(path) => builder.credential_file(path),
            None => builder,
        };
        let client = builder.build().map_err(SdkAdapterError::Sdk)?;
        let runtime = RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| SdkAdapterError::RuntimeUnavailable)?;
        Ok(Self {
            client,
            runtime: Mutex::new(runtime),
        })
    }

    fn run<T>(
        &self,
        timeout: Duration,
        future: impl Future<Output = infer_runtime_client::Result<T>>,
    ) -> Result<T, SdkAdapterError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| SdkAdapterError::RuntimeUnavailable)?;
        runtime.block_on(async {
            tokio::time::timeout(timeout, future)
                .await
                .map_err(|_| SdkAdapterError::Timeout)?
                .map_err(SdkAdapterError::Sdk)
        })
    }
    fn run_controlled<T>(
        &self,
        timeout: Duration,
        control: &super::sound_generation::SoundGenerationControl,
        future: impl Future<Output = infer_runtime_client::Result<T>>,
    ) -> Result<T, SdkAdapterError> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| SdkAdapterError::RuntimeUnavailable)?;
        runtime.block_on(async {
            let mut future = std::pin::pin!(future);
            let deadline = tokio::time::Instant::now() + timeout;
            loop {
                if control.cancelled() {
                    return Err(SdkAdapterError::Cancelled);
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(SdkAdapterError::Timeout);
                }
                if let Ok(result) =
                    tokio::time::timeout(Duration::from_millis(50), &mut future).await
                {
                    return result.map_err(SdkAdapterError::Sdk);
                }
            }
        })
    }
}

impl InferRuntimeSdk for OfficialSdkClient {
    fn contract(&self) -> Result<ContractManifest, SdkAdapterError> {
        self.run(CORE_TIMEOUT, self.client.contract())
    }

    fn capabilities(&self) -> Result<CapabilityCatalog, SdkAdapterError> {
        self.run(CORE_TIMEOUT, self.client.capabilities())
    }

    fn create_response(
        &self,
        request: &ResponsesRequest,
        timeout: Duration,
    ) -> Result<ResponsesResult, SdkAdapterError> {
        self.run(timeout, self.client.create_response(request))
    }

    fn synthesize_speech(
        &self,
        request: &SpeechRequest,
        timeout: Duration,
    ) -> Result<AudioBytesResponse, SdkAdapterError> {
        self.run(timeout, self.client.synthesize_speech(request))
    }

    fn generate_sound(
        &self,
        request: &SoundGenerationRequest,
        timeout: Duration,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<SoundGenerationResponse, SdkAdapterError> {
        self.run_controlled(timeout, control, self.client.generate_sound_effect(request))
    }
    fn prepare_sound_prompt(
        &self,
        prompt: &str,
        timeout: Duration,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<PreparedSoundPrompt, SdkAdapterError> {
        self.run_controlled(timeout, control, self.client.prepare_sound_prompt(prompt))
    }
    fn sound_job(
        &self,
        id: &str,
        control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<JobSnapshot, SdkAdapterError> {
        self.run_controlled(JOB_TIMEOUT, control, self.client.job(id))
    }

    fn job(&self, job_id: &str) -> Result<JobSnapshot, SdkAdapterError> {
        self.run(JOB_TIMEOUT, self.client.job(job_id))
    }

    fn create_agent_task(
        &self,
        request: &AgentTaskRequest,
        timeout: Duration,
    ) -> Result<AgentTaskResult, SdkAdapterError> {
        self.run(timeout, self.client.create_agent_task(request))
    }
}
