use serde::{Deserialize, Serialize};
use shape_domain::{ArtifactContentContract, ArtifactId, ProjectId, RevisionId, WorkingInput};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Origin {
    pub producer: String,
    pub project_id: ProjectId,
    pub kit_id: ArtifactId,
    pub expected_head: Option<RevisionId>,
    pub draft_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Input {
    Transfer {
        manifest_path: PathBuf,
        manifest_blake3: String,
    },
    Project {
        project_path: PathBuf,
        project_id: ProjectId,
        kit_id: ArtifactId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditRequest {
    pub schema: String,
    pub request_id: String,
    pub origin: Origin,
    pub input: Input,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    pub project_path: PathBuf,
    pub project_id: ProjectId,
    pub kit_id: ArtifactId,
    pub accepted_revision_id: Option<RevisionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Output {
    pub kind: String,
    pub path: PathBuf,
    pub byte_length: u64,
    pub blake3: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditResult {
    pub schema: String,
    pub request_id: String,
    pub request_blake3: String,
    pub outcome: Outcome,
    pub document: Option<Document>,
    pub outputs: Vec<Output>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MaterialTarget {
    pub project_id: String,
    pub kit_id: String,
    pub card_id: String,
    pub draft_digest: String,
    pub artwork: Option<WorkingInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Canvas {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MaterialRequest {
    pub schema: String,
    pub request_id: String,
    pub target: MaterialTarget,
    pub prompt: String,
    pub canvas: Canvas,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MaterialOutput {
    pub path: PathBuf,
    pub byte_length: u64,
    pub blake3: String,
    pub media_type: String,
    pub source: WorkingInput,
    pub contract: ArtifactContentContract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MaterialResult {
    pub schema: String,
    pub request_id: String,
    pub request_blake3: String,
    pub outcome: Outcome,
    pub output: Option<MaterialOutput>,
    pub error: Option<String>,
}
