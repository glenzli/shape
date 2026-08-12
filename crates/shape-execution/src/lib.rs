//! Physical execution contracts for creative transformations.
//!
//! The domain crate explains *why* content changes. This crate records *how* one
//! attempt ran, without importing provider-specific HTTP or model types. Its
//! Infer Runtime consumers use the frozen official SDK for Discovery, dated
//! Core/Capability negotiation, transport, credentials, errors, and typed
//! requests. Shape-owned creative adapters remain behind the same narrow
//! [`Executor`] interface as deterministic built-ins.

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
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, IMAGE_GENERATE_CAPABILITY,
    INFER_RUNTIME_CAPABILITY_CATALOG, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_RUNTIME_RESPONSES_CAPABILITY, INFER_RUNTIME_SPEECH_CAPABILITY,
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1, InferRuntimeClientError, InferRuntimeContract,
    InferRuntimeCredentialError, InferRuntimeCredentialStore, InferRuntimeEndpointSource,
    InferRuntimeExecutor, InferRuntimeImageGenerationExecutor, InferRuntimeProbe,
    InferRuntimeSpeechExecutor, ResolvedInferRuntimeEndpoint, probe_infer_runtime_contract,
};
pub use job::{ExecutionJob, ExecutionOutcome, ExecutionReceipt, JobState};
pub use provenance::{
    ExternalAttemptProvenance, ExternalExecutionProvenance, ExternalNamedRouteProvenance,
    ExternalRoutingCandidate,
};
pub use raster::{
    RASTER_BLUR_CAPABILITY, RASTER_CROP_CAPABILITY, RASTER_DROP_SHADOW_CAPABILITY,
    RASTER_IMPORT_CAPABILITY, RASTER_RESIZE_CAPABILITY, RASTER_TRANSFORM_CAPABILITY,
    RASTER_UNSHARP_MASK_CAPABILITY, RASTER_UNSHARP_MASK_WORKING_BYTES_PER_PIXEL,
    RasterBlurExecutor, RasterCropExecutor, RasterDropShadowExecutor, RasterExecutor,
    RasterResizeExecutor, RasterTransformExecutor, RasterUnsharpMaskExecutor,
};
