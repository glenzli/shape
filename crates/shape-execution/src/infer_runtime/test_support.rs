use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use infer_runtime_client::{
    AgentTaskRequest, AgentTaskResult, AudioBytesResponse, CapabilityCatalog, ContractManifest,
    JobSnapshot, ResponsesRequest, ResponsesResult, SpeechRequest,
};

use super::sdk::{InferRuntimeSdk, SdkAdapterError};

pub(super) struct FakeSdk {
    pub preparations: Mutex<VecDeque<infer_runtime_client::PreparedSoundPrompt>>,
    pub seen_preparations: Mutex<Vec<String>>,
    pub sounds:
        Mutex<VecDeque<Result<infer_runtime_client::SoundGenerationResponse, SdkAdapterError>>>,
    pub seen_sounds: Mutex<Vec<infer_runtime_client::SoundGenerationRequest>>,
    pub contract: Mutex<VecDeque<Result<ContractManifest, SdkAdapterError>>>,
    pub capabilities: Mutex<VecDeque<Result<CapabilityCatalog, SdkAdapterError>>>,
    pub responses: Mutex<VecDeque<Result<ResponsesResult, SdkAdapterError>>>,
    pub speeches: Mutex<VecDeque<Result<AudioBytesResponse, SdkAdapterError>>>,
    pub jobs: Mutex<VecDeque<Result<JobSnapshot, SdkAdapterError>>>,
    pub agent_tasks: Mutex<VecDeque<Result<AgentTaskResult, SdkAdapterError>>>,
    pub seen_agent_tasks: Mutex<Vec<AgentTaskRequest>>,
    pub seen_responses: Mutex<Vec<ResponsesRequest>>,
    pub seen_speeches: Arc<Mutex<Vec<SpeechRequest>>>,
}

impl FakeSdk {
    pub(super) fn new() -> Self {
        Self {
            preparations: Mutex::new(VecDeque::new()),
            seen_preparations: Mutex::new(Vec::new()),
            sounds: Mutex::new(VecDeque::new()),
            seen_sounds: Mutex::new(Vec::new()),
            contract: Mutex::new(VecDeque::new()),
            capabilities: Mutex::new(VecDeque::new()),
            responses: Mutex::new(VecDeque::new()),
            speeches: Mutex::new(VecDeque::new()),
            jobs: Mutex::new(VecDeque::new()),
            agent_tasks: Mutex::new(VecDeque::new()),
            seen_agent_tasks: Mutex::new(Vec::new()),
            seen_responses: Mutex::new(Vec::new()),
            seen_speeches: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(super) fn response(self, response: ResponsesResult) -> Self {
        self.responses.lock().unwrap().push_back(Ok(response));
        self
    }

    pub(super) fn speech(self, response: AudioBytesResponse) -> Self {
        self.speeches.lock().unwrap().push_back(Ok(response));
        self
    }

    pub(super) fn job(self, job: JobSnapshot) -> Self {
        self.jobs.lock().unwrap().push_back(Ok(job));
        self
    }
}

impl InferRuntimeSdk for FakeSdk {
    fn contract(&self) -> Result<ContractManifest, SdkAdapterError> {
        self.contract
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake contract response")
    }

    fn capabilities(&self) -> Result<CapabilityCatalog, SdkAdapterError> {
        self.capabilities
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake capability response")
    }

    fn create_response(
        &self,
        request: &ResponsesRequest,
        _timeout: Duration,
    ) -> Result<ResponsesResult, SdkAdapterError> {
        self.seen_responses.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake Responses result")
    }

    fn synthesize_speech(
        &self,
        request: &SpeechRequest,
        _timeout: Duration,
    ) -> Result<AudioBytesResponse, SdkAdapterError> {
        self.seen_speeches.lock().unwrap().push(request.clone());
        self.speeches
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake speech result")
    }

    fn prepare_sound_prompt(
        &self,
        prompt: &str,
        _timeout: Duration,
        _control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<infer_runtime_client::PreparedSoundPrompt, SdkAdapterError> {
        self.seen_preparations.lock().unwrap().push(prompt.into());
        if let Some(prepared) = self.preparations.lock().unwrap().pop_front() {
            return Ok(prepared);
        }
        Ok(infer_runtime_client::PreparedSoundPrompt {
            original_prompt: prompt.into(),
            effective_prompt: prompt.into(),
            rules_revision: infer_runtime_client::SOUND_PROMPT_RULES_REVISION.into(),
            text_job: None,
            preparation_elapsed_ms: 0,
        })
    }
    fn sound_job(
        &self,
        id: &str,
        _control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<JobSnapshot, SdkAdapterError> {
        self.job(id)
    }
    fn generate_sound(
        &self,
        request: &infer_runtime_client::SoundGenerationRequest,
        _timeout: Duration,
        _control: &super::sound_generation::SoundGenerationControl,
    ) -> Result<infer_runtime_client::SoundGenerationResponse, SdkAdapterError> {
        self.seen_sounds.lock().unwrap().push(request.clone());
        self.sounds
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake sound response")
    }
    fn job(&self, _job_id: &str) -> Result<JobSnapshot, SdkAdapterError> {
        self.jobs
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake Job result")
    }

    fn create_agent_task(
        &self,
        request: &AgentTaskRequest,
        _timeout: Duration,
    ) -> Result<AgentTaskResult, SdkAdapterError> {
        self.seen_agent_tasks.lock().unwrap().push(request.clone());
        self.agent_tasks
            .lock()
            .unwrap()
            .pop_front()
            .expect("fake Agent task result")
    }
}
