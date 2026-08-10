//! Physical execution contracts for creative transformations.
//!
//! The domain crate explains *why* content changes. This crate records *how* one
//! attempt ran, without importing provider-specific HTTP or model types. Its
//! Infer Runtime consumer resolves an owner-only Infra Discovery registration
//! and verifies the public contract gate. Shape's owner-only credential store
//! and local-first Responses adapter remain behind the same narrow [`Executor`]
//! interface as deterministic built-ins.

mod coordinator;
mod error;
mod executor;
mod ids;
mod infer_runtime;
mod job;
mod raster;

pub use coordinator::{ExecutedCandidate, ExecutionCoordinator};
pub use error::{ExecutionError, ExecutionFailure};
pub use executor::{
    CapabilityId, ExecutionInput, ExecutionOutput, ExecutionRequest, Executor, ExecutorIdentity,
};
pub use ids::{AttemptId, JobId};
pub use infer_runtime::{
    INFER_RUNTIME_COMPATIBILITY_ENDPOINT, INFER_RUNTIME_CONTRACT_VERSION, InferRuntimeClient,
    InferRuntimeClientError, InferRuntimeContract, InferRuntimeCredential,
    InferRuntimeCredentialError, InferRuntimeCredentialStore, InferRuntimeEndpointResolver,
    InferRuntimeEndpointSource, InferRuntimeExecutor, InferRuntimeProbe,
    ResolvedInferRuntimeEndpoint, probe_infer_runtime_contract,
};
pub use job::{ExecutionJob, ExecutionOutcome, ExecutionReceipt, JobState};
pub use raster::{RASTER_CROP_CAPABILITY, RASTER_IMPORT_CAPABILITY, RasterExecutor};
