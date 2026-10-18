//! Read-only, identity-bound material publication for the independent Vox application.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::{Value, json};
use shape_domain::{
    ArtifactContentContract, ArtifactId, ArtifactKind, ArtifactWorkingGraph, ContentDigest,
    ContentRef, PresetVoiceAlias, PresetVoiceSelection, RevisionId, SpeechSynthesisOperation,
    SpeechVoiceSelection, TextDocumentContract, TransformationOperation, WorkingInput,
    WorkingOperatorDraft,
};
use thiserror::Error;

use super::ShapeProject;
use super::deck_handoff::files::write_new;
use crate::CoreError;

const AUTHORING_SCHEMA: &str = "shape.operator-draft.text-authoring@20260922.1";
const SPEECH_SCHEMAS: &[&str] = &[
    "shape.operator-draft.audio-speech@20260811.1",
    "shape.operator-draft.audio-speech@20260922.1",
];
const MAX_TEXT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TRANSFER_BYTES: u64 = 168 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum VoxTransferError {
    #[error("invalid Vox transfer: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("Vox transfer JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Vox transfer file operation failed: {0}")]
    Io(#[from] std::io::Error),
}

fn invalid(message: &'static str) -> VoxTransferError {
    VoxTransferError::Invalid(message)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoxTransferRequest {
    request_id: String,
    text_artifact_id: ArtifactId,
    expected_text_head: Option<RevisionId>,
    authoring_graph_digest: Option<String>,
    speech_target_artifact_id: Option<ArtifactId>,
    speech_graph_digest: Option<String>,
    accepted_audio: Option<WorkingInput>,
    destination: PathBuf,
}

#[derive(Debug, Default)]
struct Payloads {
    files: BTreeMap<String, Vec<u8>>,
    total: u64,
}
impl Payloads {
    fn add(&mut self, content: &ContentRef, bytes: Vec<u8>) -> Result<String, VoxTransferError> {
        if bytes.len() as u64 != content.byte_length
            || ContentDigest::from_bytes(&bytes) != content.digest
        {
            return Err(invalid(
                "material bytes do not match their exact content identity",
            ));
        }
        let extension = if content.media_type == "audio/wav" {
            "wav"
        } else {
            "txt"
        };
        let relative_path = format!("materials/{}.{extension}", content.digest);
        if !self.files.contains_key(&relative_path) {
            self.total = self
                .total
                .checked_add(content.byte_length)
                .filter(|total| *total <= MAX_TRANSFER_BYTES)
                .ok_or_else(|| invalid("material bytes exceed the transfer limit"))?;
            self.files.insert(relative_path.clone(), bytes);
        }
        Ok(relative_path)
    }
}

pub(super) fn graph_digest(graph: &ArtifactWorkingGraph) -> Result<String, serde_json::Error> {
    Ok(ContentDigest::from_bytes(&serde_json::to_vec(graph)?).to_string())
}

pub(super) fn speech_operation(
    node: &WorkingOperatorDraft,
) -> Result<SpeechSynthesisOperation, VoxTransferError> {
    let settings = node
        .configuration()
        .ok_or_else(|| invalid("speech draft has no settings"))?;
    if !SPEECH_SCHEMAS.contains(&settings.schema().as_str()) {
        return Err(invalid("unsupported saved speech settings schema"));
    }
    let value: Value = serde_json::from_str(settings.json())?;
    let string = |field| {
        value[field]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| invalid("invalid saved speech settings"))
    };
    let mut operation = SpeechSynthesisOperation::new(
        string("language")?,
        SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(string("preset_alias")?).map_err(CoreError::from)?,
                string("preset_catalog_revision")?,
            )
            .map_err(CoreError::from)?,
        ),
        value["speed_milli"]
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| invalid("invalid saved speech speed"))?,
        value["synthetic_disclosure_required"]
            .as_bool()
            .ok_or_else(|| invalid("invalid saved speech disclosure"))?,
    )
    .map_err(CoreError::from)?;
    operation.script = serde_json::from_value(value.get("script").cloned().unwrap_or(Value::Null))?;
    if settings.schema().as_str() == SPEECH_SCHEMAS[0] && operation.script.is_some() {
        return Err(invalid(
            "legacy speech settings cannot contain script bindings",
        ));
    }
    operation.validate().map_err(CoreError::from)?;
    Ok(operation)
}

impl ShapeProject {
    /// Publishes exact accepted text, recoverable drafts and optional existing audio to Vox.
    /// This operation never checkpoints a draft or changes the Shape project.
    /// # Errors
    /// Rejects missing/stale selection identities, malformed drafts, corrupt material bytes,
    /// non-absolute/internal/existing destinations, or failure before the final manifest.
    pub fn export_vox_transfer_json(&self, request_json: &str) -> Result<String, VoxTransferError> {
        if request_json.len() > 16 * 1024 {
            return Err(invalid("request exceeds 16 KiB"));
        }
        let value: Value = serde_json::from_str(request_json)?;
        if value.get("expected_text_head").is_none() {
            return Err(invalid(
                "expected_text_head must be explicit, including null for an unaccepted text",
            ));
        }
        let request: VoxTransferRequest = serde_json::from_value(value)?;
        if !super::deck_handoff::valid_token(&request.request_id) {
            return Err(invalid("invalid bounded request id"));
        }
        self.check_vox_selection(&request)?;
        let mut payloads = Payloads::default();
        let accepted_text = request
            .expected_text_head
            .map(|revision_id| {
                self.vox_material(
                    WorkingInput {
                        artifact_id: request.text_artifact_id,
                        revision_id,
                    },
                    ArtifactKind::TextDocument,
                    &mut payloads,
                )
            })
            .transpose()?;
        let graphs = self.artifact_working_graphs()?;
        let authoring_draft = graphs
            .iter()
            .find(|g| g.context_artifact_id() == request.text_artifact_id)
            .filter(|g| g.operators().iter().any(is_authoring_node))
            .map(|graph| Self::vox_authoring_draft(graph, &mut payloads))
            .transpose()?;
        if accepted_text.is_none() && authoring_draft.is_none() {
            return Err(invalid(
                "selected text has neither accepted content nor a recoverable writing draft",
            ));
        }
        let mut cue_materials = BTreeMap::new();
        let speech_draft = request.speech_target_artifact_id.map(|target| {
            let graph = graphs.iter().find(|g| g.context_artifact_id() == target).ok_or_else(|| invalid("selected speech graph disappeared"))?;
            let node = graph.operators().iter().find(|node| node.operator_type().as_str() == "audio.speech_synthesize").ok_or_else(|| invalid("selected graph has no speech draft"))?;
            let operation = node.configuration().map(|_| speech_operation(node)).transpose()?;
            if let Some(operation) = &operation { self.vox_cues(operation, &mut payloads, &mut cue_materials)?; }
            let input_text = node.input().map(|pin| self.vox_material(pin, ArtifactKind::TextDocument, &mut payloads)).transpose()?;
            Ok::<_, VoxTransferError>(json!({"status":"unaccepted","target_artifact_id":target,"expected_head":graph.expected_revision_id(),"graph_digest":graph_digest(graph)?,"draft_id":node.id(),"input":node.input(),"input_text":input_text,"graph":graph,"settings":node.configuration(),"operation":operation,"issue":if operation.is_none() {Some("speech_settings_unconfigured")}else {None}}))
        }).transpose()?;
        let accepted_audio = request.accepted_audio.map(|pin| {
            let material = self.vox_material(pin, ArtifactKind::AudioClip, &mut payloads)?;
            let revision = self.revision(pin.revision_id)?;
            let transformation = self.transformation(revision.transformation_id)?;
            let receipt = self.transformation_receipt(transformation.id)?;
            let operation = match &transformation.operation { Some(TransformationOperation::AudioSpeechSynthesis(op)) => Some(op), _ => None };
            if let Some(operation) = operation { self.vox_cues(operation, &mut payloads, &mut cue_materials)?; }
            let input_text = if operation.is_some() && transformation.inputs.len() == 1 {
                let input = self.revision(transformation.inputs[0])?;
                Some(self.vox_material(WorkingInput { artifact_id: input.artifact_id, revision_id: input.id }, ArtifactKind::TextDocument, &mut payloads)?)
            } else { None };
            Ok::<_, VoxTransferError>(json!({"status":"accepted_external_origin","material":material,"input_text":input_text,"receipt":receipt,"transformation":transformation,"operation":operation}))
        }).transpose()?;
        let manifest = json!({"schema":"vox.shape-transfer/1","request_id":request.request_id,
            "origin":{"producer":"shape","project_id":self.snapshot()?.metadata.id,"text_artifact_id":request.text_artifact_id,"observed_text_head":request.expected_text_head},
            "accepted_text":accepted_text,"authoring_draft":authoring_draft,"speech_draft":speech_draft,
            "cue_materials":cue_materials.into_values().collect::<Vec<_>>(),"accepted_audio":accepted_audio});
        let destination = self.vox_destination(&request.destination)?;
        create_private_directory(&destination)?;
        create_private_directory(&destination.join("materials"))?;
        for (relative, bytes) in payloads.files {
            write_new(&destination.join(relative), &bytes)?;
        }
        self.check_vox_selection(&request)?;
        if destination.canonicalize()? != destination {
            return Err(invalid("transfer destination changed before publication"));
        }
        let bytes = serde_json::to_vec_pretty(&manifest)?;
        let manifest_path = destination.join("manifest.json");
        write_new(&manifest_path, &bytes)?;
        Ok(serde_json::to_string(
            &json!({"schema":"shape.vox-transfer-result/1","manifest_path":manifest_path,"manifest_blake3":ContentDigest::from_bytes(&bytes).to_string(),"request_id":request.request_id,"origin":manifest["origin"]}),
        )?)
    }

    fn check_vox_selection(&self, request: &VoxTransferRequest) -> Result<(), VoxTransferError> {
        let snapshot = self.snapshot()?;
        let text = snapshot
            .artifacts
            .iter()
            .find(|a| a.id == request.text_artifact_id)
            .ok_or_else(|| invalid("selected text artifact is missing"))?;
        if text.kind != ArtifactKind::TextDocument
            || text.accepted_revision != request.expected_text_head
        {
            return Err(invalid(
                "selected text head changed; reload before transferring",
            ));
        }
        let graphs = self.artifact_working_graphs()?;
        let authored = graphs.iter().find(|g| {
            g.context_artifact_id() == text.id && g.operators().iter().any(is_authoring_node)
        });
        if authored.map(graph_digest).transpose()? != request.authoring_graph_digest {
            return Err(invalid(
                "writing draft identity changed; select its exact current graph",
            ));
        }
        match (
            request.speech_target_artifact_id,
            &request.speech_graph_digest,
        ) {
            (None, None) => {}
            (Some(target), Some(digest)) => {
                let graph = graphs
                    .iter()
                    .find(|g| g.context_artifact_id() == target)
                    .ok_or_else(|| invalid("selected speech graph is missing"))?;
                let artifact = snapshot
                    .artifacts
                    .iter()
                    .find(|a| a.id == target)
                    .ok_or_else(|| invalid("selected speech output is missing"))?;
                if graph_digest(graph)? != *digest
                    || graph.expected_revision_id() != artifact.accepted_revision
                {
                    return Err(invalid(
                        "speech draft or target head changed; reload before transferring",
                    ));
                }
                let node = graph
                    .operators()
                    .iter()
                    .find(|n| n.operator_type().as_str() == "audio.speech_synthesize")
                    .ok_or_else(|| invalid("selected graph has no speech draft"))?;
                if node.input().is_some_and(|pin| pin.artifact_id != text.id) {
                    return Err(invalid(
                        "selected speech draft belongs to a different text artifact",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "speech target and graph digest must be selected together",
                ));
            }
        }
        if let Some(graph) = authored
            && graph.expected_revision_id() != text.accepted_revision
        {
            return Err(invalid(
                "writing draft target head changed; reload before transferring",
            ));
        }
        Ok(())
    }

    fn vox_destination(&self, path: &Path) -> Result<PathBuf, VoxTransferError> {
        if !path.is_absolute() || fs::symlink_metadata(path).is_ok() {
            return Err(invalid("destination must be a new absolute directory"));
        }
        let parent = path
            .parent()
            .ok_or_else(|| invalid("destination needs a parent"))?
            .canonicalize()?;
        let destination = parent.join(
            path.file_name()
                .ok_or_else(|| invalid("destination needs a name"))?,
        );
        if destination.starts_with(self.store.root().canonicalize()?) {
            return Err(invalid("destination must be outside the Shape project"));
        }
        Ok(destination)
    }

    fn vox_material(
        &self,
        source: WorkingInput,
        kind: ArtifactKind,
        payloads: &mut Payloads,
    ) -> Result<Value, VoxTransferError> {
        let artifact = self
            .snapshot()?
            .artifacts
            .into_iter()
            .find(|a| a.id == source.artifact_id)
            .ok_or_else(|| invalid("material artifact is missing"))?;
        let revision = self.revision(source.revision_id)?;
        if artifact.kind != kind || revision.artifact_id != source.artifact_id {
            return Err(invalid(
                "material pin does not match the selected artifact and media",
            ));
        }
        let maximum = if kind == ArtifactKind::AudioClip {
            128 * 1024 * 1024
        } else {
            MAX_TEXT_BYTES
        };
        if revision.content.byte_length > maximum {
            return Err(invalid("material exceeds its byte limit"));
        }
        let accepted = self.read_revision_content(source.revision_id)?;
        match kind {
            ArtifactKind::TextDocument => {
                std::str::from_utf8(&accepted.bytes)
                    .map_err(|_| invalid("text material is not UTF-8"))?;
            }
            ArtifactKind::AudioClip => {
                let Some(ArtifactContentContract::AudioClip(contract)) = &revision.content_contract
                else {
                    return Err(invalid("accepted WAV has no exact audio contract"));
                };
                let observed =
                    shape_execution::parse_pcm_s16le_wav(&accepted.bytes, contract.origin)
                        .map_err(|_| invalid("accepted WAV is invalid"))?;
                if observed != *contract || revision.content.media_type != "audio/wav" {
                    return Err(invalid("accepted WAV bytes and audio contract differ"));
                }
            }
            _ => return Err(invalid("unsupported transfer material")),
        }
        let relative_path = payloads.add(&revision.content, accepted.bytes)?;
        Ok(
            json!({"source":source,"revision":revision,"content":revision.content,"contract":revision.content_contract,"relative_path":relative_path}),
        )
    }

    fn vox_authoring_draft(
        graph: &ArtifactWorkingGraph,
        payloads: &mut Payloads,
    ) -> Result<Value, VoxTransferError> {
        let node = graph
            .operators()
            .iter()
            .find(|node| is_authoring_node(node))
            .ok_or_else(|| invalid("writing draft is missing"))?;
        let settings = node
            .configuration()
            .ok_or_else(|| invalid("writing settings are missing"))?;
        let state: Value = serde_json::from_str(settings.json())?;
        let text = state["text"]
            .as_str()
            .ok_or_else(|| invalid("writing draft text is missing"))?;
        let format = match state["profile"].as_str() {
            Some("plain") => TextDocumentContract::Plain,
            Some("script" | "listening" | "narration" | "dialogue") => {
                TextDocumentContract::speech_script()
            }
            _ => return Err(invalid("unsupported writing draft format")),
        };
        if text.len() > 48 * 1024 {
            return Err(invalid("writing draft text exceeds its saved-state limit"));
        }
        let content = ContentRef::new(
            ContentDigest::from_bytes(text.as_bytes()),
            "text/plain; charset=utf-8",
            text.len() as u64,
        )
        .map_err(CoreError::from)?;
        let relative_path = payloads.add(&content, text.as_bytes().to_vec())?;
        Ok(
            json!({"status":"unaccepted","target_artifact_id":graph.context_artifact_id(),"expected_head":graph.expected_revision_id(),"graph_digest":graph_digest(graph)?,"graph":graph,"format":format,"settings":settings,"text":{"content":content,"contract":ArtifactContentContract::TextDocument(format),"relative_path":relative_path}}),
        )
    }

    fn vox_cues(
        &self,
        operation: &SpeechSynthesisOperation,
        payloads: &mut Payloads,
        cues: &mut BTreeMap<String, Value>,
    ) -> Result<(), VoxTransferError> {
        for cue in operation
            .script
            .iter()
            .flat_map(|options| options.cues.values())
        {
            if let shape_domain::speech_script::SpeechCueAction::Audio { content } = cue {
                if content.byte_length > 8 * 1024 * 1024 || content.media_type != "audio/wav" {
                    return Err(invalid("cue material exceeds its WAV contract"));
                }
                let bytes = self.store.read_content(content).map_err(CoreError::from)?;
                let contract = shape_execution::parse_pcm_s16le_wav(
                    &bytes,
                    shape_domain::AudioOriginDisclosure::ImportedUnverified,
                )
                .map_err(|_| invalid("cue material is not a supported WAV"))?;
                let relative_path = payloads.add(content, bytes)?;
                cues.insert(content.digest.to_string(), json!({"content":content,"contract":ArtifactContentContract::AudioClip(contract),"relative_path":relative_path}));
            }
        }
        Ok(())
    }
}

pub(super) fn is_authoring_node(node: &WorkingOperatorDraft) -> bool {
    node.configuration()
        .is_some_and(|settings| settings.schema().as_str() == AUTHORING_SCHEMA)
}

fn create_private_directory(path: &Path) -> Result<(), std::io::Error> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

#[cfg(test)]
mod tests;
