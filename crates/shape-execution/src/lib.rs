//! Physical execution contracts for creative transformations.
//!
//! The domain crate explains *why* content changes. This crate records *how* one
//! attempt ran, without importing provider-specific HTTP or model types. Its
//! Infer Runtime consumer resolves an owner-only Infra Discovery registration
//! and verifies the public contract gate. Shape's owner-only credential store,
//! local-first Responses adapter, and preset-only speech synthesis adapter
//! remain behind the same narrow [`Executor`] interface as deterministic
//! built-ins.

mod audio;
mod coordinator;
mod error;
mod executor;
mod ids;
mod infer_runtime;
mod job;
mod provenance;
mod raster;

pub use audio::parse_pcm_s16le_wav;
pub use coordinator::{ExecutedCandidate, ExecutionCoordinator};
pub use error::{ExecutionError, ExecutionFailure};
pub use executor::{
    CapabilityId, ExecutionInput, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
};
pub use ids::{AttemptId, JobId};
pub use infer_runtime::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, INFER_RUNTIME_COMPATIBILITY_ENDPOINT,
    INFER_RUNTIME_CONTRACT_VERSION, INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
    InferRuntimeClient, InferRuntimeClientError, InferRuntimeContract, InferRuntimeCredential,
    InferRuntimeCredentialError, InferRuntimeCredentialStore, InferRuntimeEndpointResolver,
    InferRuntimeEndpointSource, InferRuntimeExecutor, InferRuntimeProbe,
    InferRuntimeSpeechExecutor, ResolvedInferRuntimeEndpoint, probe_infer_runtime_contract,
};
pub use job::{ExecutionJob, ExecutionOutcome, ExecutionReceipt, JobState};
pub use provenance::{
    ExternalAttemptProvenance, ExternalExecutionProvenance, ExternalRoutingCandidate,
};
pub use raster::{
    RASTER_CROP_CAPABILITY, RASTER_IMPORT_CAPABILITY, RASTER_RESIZE_CAPABILITY, RasterCropExecutor,
    RasterExecutor, RasterResizeExecutor,
};
