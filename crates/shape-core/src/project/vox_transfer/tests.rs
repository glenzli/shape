use std::{fs, path::Path};

use shape_domain::{
    ArtifactContentContract, ArtifactId, ArtifactKind, ArtifactRevision, AudioOriginDisclosure,
    AudioValueContract, Constraint, IntentSpec, PresetVoiceAlias, PresetVoiceSelection,
    SpeechSynthesisOperation, SpeechVoiceSelection,
};
use shape_execution::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, CapabilityId, ExecutionFailure, ExecutionOutput,
    ExecutionRequest, Executor, ExecutorIdentity, ExternalAttemptProvenance,
    ExternalExecutionProvenance, ExternalRoutingCandidate, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
};
use uuid::Uuid;

use super::*;

#[derive(Debug)]
struct SpeechExecutor {
    identity: ExecutorIdentity,
    bytes: Vec<u8>,
}

impl SpeechExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.test.speech",
                "1",
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .unwrap(),
            bytes: wav(24_000, 1, 24_000),
        }
    }
}

impl Executor for SpeechExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == AUDIO_SPEECH_SYNTHESIZE_CAPABILITY
    }

    fn execute(&self, _request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        Ok(ExecutionOutput {
            bytes: self.bytes.clone(),
            media_type: "audio/wav".to_owned(),
            executor_job_id: Some("job_shape_audio_core_1".to_owned()),
            external_provenance: Some(provenance()),
            content_contract: Some(ArtifactContentContract::AudioClip(
                AudioValueContract::pcm_s16le_wav(
                    24_000,
                    1,
                    24_000,
                    AudioOriginDisclosure::SyntheticSpeech,
                )
                .unwrap(),
            )),
        })
    }
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("shape-core-audio-{label}-{}", Uuid::now_v7()))
}

fn wav(sample_rate_hz: u32, channels: u16, frames: u32) -> Vec<u8> {
    let block_align = channels * 2;
    let data_size = frames * u32::from(block_align);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate_hz.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate_hz * u32::from(block_align)).to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.resize(bytes.len() + data_size as usize, 0);
    bytes
}

fn operation() -> SpeechSynthesisOperation {
    SpeechSynthesisOperation::new(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1).unwrap(),
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            )
            .unwrap(),
        ),
        1_000,
        true,
    )
    .unwrap()
}

fn provenance() -> ExternalExecutionProvenance {
    ExternalExecutionProvenance {
        sound_prompt: None,
        speech_segments: Vec::new(),
        speech_script: None,
        contract_revision: INFER_RUNTIME_CONTRACT_VERSION.to_owned(),
        capability_contract: Some(shape_execution::INFER_RUNTIME_SPEECH_CAPABILITY.to_owned()),
        app_id: "shape".to_owned(),
        intent: "speech.synthesize".to_owned(),
        provider: "mlx-audio-local".to_owned(),
        deployment: "mlx_qwen3_tts_custom_voice_1_7b".to_owned(),
        model_profile: "qwen3_tts_custom_voice_1_7b".to_owned(),
        model_build: "qwen3_tts_custom_voice_1_7b_8bit".to_owned(),
        physical_model: "qwen3-tts-custom-voice".to_owned(),
        placement: "local".to_owned(),
        capability_level: "capable".to_owned(),
        evaluation_status: "provisional".to_owned(),
        resource_class: "standard".to_owned(),
        policy: "local-first".to_owned(),
        priority: "interactive".to_owned(),
        requested_policy: "local-first".to_owned(),
        requested_priority: "interactive".to_owned(),
        requested_provider_access_class: None,
        requested_placement: "local_only".to_owned(),
        requested_preference: "local".to_owned(),
        offline_required: true,
        requested_latency: Some("interactive".to_owned()),
        fallback: "none".to_owned(),
        requested_deadline_ms: None,
        max_cost_microusd: 0,
        capability_floor: "capable".to_owned(),
        named_route: None,
        routing_candidates: vec![ExternalRoutingCandidate {
            provider: "mlx-audio-local".to_owned(),
            deployment: "mlx_qwen3_tts_custom_voice_1_7b".to_owned(),
            status: "eligible".to_owned(),
            rank: Some(1),
            reason_codes: Vec::new(),
        }],
        attempts: vec![ExternalAttemptProvenance {
            number: 1,
            provider: "mlx-audio-local".to_owned(),
            deployment: "mlx_qwen3_tts_custom_voice_1_7b".to_owned(),
            outcome: "succeeded".to_owned(),
            trigger: "initial".to_owned(),
            error_kind: None,
        }],
    }
}

fn accepted_text(project: &mut ShapeProject) -> (ArtifactId, ArtifactRevision) {
    let artifact = project
        .create_artifact("Narration", ArtifactKind::TextDocument)
        .unwrap();
    let candidate = project
        .propose_text(
            artifact.id,
            None,
            "这是一条普通的合成旁白。",
            IntentSpec::new("Write narration").unwrap(),
            Vec::<Constraint>::new(),
        )
        .unwrap();
    let revision = project.accept_text(candidate).unwrap();
    (artifact.id, revision)
}

fn configuration(schema: &str, value: &Value) -> shape_domain::WorkingOperatorConfiguration {
    shape_domain::WorkingOperatorConfiguration::new(
        shape_domain::OperatorConfigurationSchemaId::new(schema).unwrap(),
        value.to_string(),
    )
    .unwrap()
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "One retained mixed legacy fixture exercises the producer and is consumed by independent Vox and native processes"
)]
fn full_legacy_transfer_preserves_unaccepted_text_voices_cues_and_historical_audio_source() {
    use shape_domain::{ArtifactWorkingGraph, OperatorDataTypeId, OperatorTypeId};
    let retained = std::env::var_os("SHAPE_VOX_FIXTURE_ROOT").map(PathBuf::from);
    let root = retained
        .clone()
        .unwrap_or_else(|| test_root("vox-transfer"));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("legacy.shape");
    let mut project = ShapeProject::create(&bundle, "Mixed legacy migration").unwrap();
    let (text_id, old_text) = accepted_text(&mut project);
    let audio = project
        .propose_speech_synthesis(
            text_id,
            old_text.id,
            "Old Shape accepted audio",
            &operation(),
            vec![],
            &SpeechExecutor::new(),
        )
        .unwrap();
    let audio = project.accept_speech_synthesis(audio).unwrap();
    let current = project
        .propose_text(
            text_id,
            Some(old_text.id),
            "[role: Reader]\n[speaker: Reader]\nNew accepted text.",
            IntentSpec::new("Revise for another episode").unwrap(),
            vec![],
        )
        .unwrap();
    let current = project.accept_text(current).unwrap();
    let mut author = ArtifactWorkingGraph::new(text_id, current.id);
    let node = author
        .add_operator(
            OperatorTypeId::new("text.edit").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    let raw_draft = "[role: Reader]\n[speaker: Reader]\n未采用的完整稿件。 [pause: 2s]";
    author.set_operator_configuration(node.id(),Some(configuration(AUTHORING_SCHEMA,&json!({"profile":"dialogue","entry":"manual","instruction":"Keep this exact requirement","material":"source note","text":raw_draft,"future_setting":{"review":true}}))));
    project.save_artifact_working_graph(&author).unwrap();
    let cue = project.import_speech_cue(&wav(24_000, 1, 240)).unwrap();
    let voices = shape_execution::SPEECH_PRESETS;
    let roles=voices.iter().enumerate().map(|(i,voice)|(format!("Role{i}"),json!({"alias":voice.alias,"catalog_revision":shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION}))).collect::<serde_json::Map<_,_>>();
    let target = project
        .create_artifact("Saved speech settings", ArtifactKind::AudioClip)
        .unwrap();
    let mut speech = ArtifactWorkingGraph::new_source(target.id);
    let node = speech
        .add_bound_operator(
            OperatorTypeId::new("audio.speech_synthesize").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("audio.clip").unwrap(),
            WorkingInput {
                artifact_id: text_id,
                revision_id: current.id,
            },
        )
        .unwrap();
    speech.set_operator_configuration(node.id(),Some(configuration(SPEECH_SCHEMAS[1],&json!({"preset_alias":voices[6].alias,"preset_catalog_revision":shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION,"language":"auto","speed_milli":1125,"synthetic_disclosure_required":true,"script":{"revision":shape_domain::speech_script::SPEECH_SCRIPT_REVISION,"roles":roles,"cues":{"bell":{"action":"audio","content":cue}}}}))));
    project.save_artifact_working_graph(&speech).unwrap();
    let cards=serde_json::from_value(json!({"name":"Legacy cards","cards":[{"id":"moon","rules":"Visible text","art_prompt":"Saved unfinished prompt"}]})).unwrap();
    project.create_card_kit_draft(&cards).unwrap();
    let book=serde_json::from_value(json!({"composition":{"name":"Legacy unfinished book","default_language":"en","languages":{},"pages":[{"kind":"blank"},{"kind":"blank"}],"canvas_width":1024,"canvas_height":768},"active_language":"en","page_index":1,"script":"unfinished [Cover] draft","script_dirty":true})).unwrap();
    project.create_picture_book_draft(&book).unwrap();
    let accepted_cards=serde_json::from_value(json!({"name":"Legacy accepted cards","cards":[{"id":"accepted-moon","rules":"Accepted only"}]})).unwrap();
    let accepted_draft = project.create_card_kit_draft(&accepted_cards).unwrap();
    let candidate = project
        .propose_card_kit(accepted_draft.artifact_id, &accepted_draft.draft_digest)
        .unwrap();
    project.accept_card_kit(candidate).unwrap();
    let reopened = project
        .open_card_kit_draft(accepted_draft.artifact_id)
        .unwrap();
    let mut changed = accepted_cards;
    changed.cards[0].rules = "Draft only, must not leak into accepted export".into();
    project
        .save_card_kit_draft(reopened.artifact_id, &reopened.draft_digest, &changed)
        .unwrap();
    drop(project);
    let project = ShapeProject::open_read_only(&bundle).unwrap();
    let before = fs::read(bundle.join("project.sqlite")).unwrap();
    let request = json!({"request_id":"compat-full-v1","text_artifact_id":text_id,"expected_text_head":current.id,"authoring_graph_digest":graph_digest(&author).unwrap(),"speech_target_artifact_id":target.id,"speech_graph_digest":graph_digest(&speech).unwrap(),"accepted_audio":{"artifact_id":audio.artifact_id,"revision_id":audio.id},"destination":root.join("transfer")});
    fs::write(
        root.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(
        &project
            .export_vox_transfer_json(&request.to_string())
            .unwrap(),
    )
    .unwrap();
    let manifest_path = Path::new(result["manifest_path"].as_str().unwrap());
    let manifest: Value = serde_json::from_slice(&fs::read(manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["authoring_draft"]["status"], "unaccepted");
    let settings: Value = serde_json::from_str(
        manifest["authoring_draft"]["settings"]["json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(settings["future_setting"], json!({"review":true}));
    let read = |material: &Value| {
        fs::read(
            root.join("transfer")
                .join(material["relative_path"].as_str().unwrap()),
        )
        .unwrap()
    };
    assert_eq!(
        read(&manifest["authoring_draft"]["text"]),
        raw_draft.as_bytes()
    );
    assert_eq!(
        manifest["speech_draft"]["operation"]["script"]["roles"]
            .as_object()
            .unwrap()
            .len(),
        9
    );
    assert_eq!(manifest["speech_draft"]["operation"]["speed_milli"], 1125);
    assert_eq!(
        manifest["accepted_audio"]["input_text"]["source"]["revision_id"],
        old_text.id.to_string()
    );
    assert_eq!(
        manifest["accepted_text"]["source"]["revision_id"],
        current.id.to_string()
    );
    assert_eq!(
        read(&manifest["accepted_audio"]["input_text"]),
        project.read_revision_content(old_text.id).unwrap().bytes
    );
    assert_eq!(
        manifest["accepted_audio"]["receipt"]["external_provenance"]["app_id"],
        "shape"
    );
    assert_eq!(manifest["cue_materials"].as_array().unwrap().len(), 1);
    assert_eq!(fs::read(bundle.join("project.sqlite")).unwrap(), before);
    assert!(
        project
            .export_vox_transfer_json(&request.to_string())
            .is_err()
    );
    let mut stale = request.clone();
    stale["destination"] = json!(root.join("stale"));
    stale["authoring_graph_digest"] = json!("0".repeat(64));
    assert!(
        project
            .export_vox_transfer_json(&stale.to_string())
            .is_err()
    );
    assert!(!root.join("stale").exists());
    let mut internal = request.clone();
    internal["destination"] = json!(bundle.join("forbidden"));
    assert!(
        project
            .export_vox_transfer_json(&internal.to_string())
            .is_err()
    );
    assert!(!bundle.join("forbidden").exists());
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    drop(project);
    if retained.is_none() {
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn expected_head_is_required_before_opening_any_output() {
    let root = test_root("vox-empty");
    let project = ShapeProject::create(&root, "Empty").unwrap();
    assert!(
        project
            .export_vox_transfer_json("{}")
            .unwrap_err()
            .to_string()
            .contains("expected_text_head must be explicit")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn draft_only_text_and_unconfigured_speech_are_preserved_without_defaults() {
    use shape_domain::{ArtifactWorkingGraph, OperatorDataTypeId, OperatorTypeId};
    let retained = std::env::var_os("SHAPE_VOX_DRAFT_ONLY_FIXTURE_ROOT").map(PathBuf::from);
    let root = retained
        .clone()
        .unwrap_or_else(|| test_root("vox-draft-only"));
    fs::create_dir(&root).unwrap();
    let bundle = root.join("draft.shape");
    let project = ShapeProject::create(&bundle, "Unaccepted").unwrap();
    let text = project
        .create_artifact("Unaccepted text", ArtifactKind::TextDocument)
        .unwrap();
    let mut author = ArtifactWorkingGraph::new_source(text.id);
    let node = author
        .add_source_operator(
            OperatorTypeId::new("text.create").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    author.set_operator_configuration(node.id(),Some(configuration(AUTHORING_SCHEMA,&json!({"profile":"script","entry":"manual","instruction":"","material":"","text":"unfinished raw [draft]"}))));
    project.save_artifact_working_graph(&author).unwrap();
    let output = project
        .create_artifact("Unconfigured speech", ArtifactKind::AudioClip)
        .unwrap();
    let mut speech = ArtifactWorkingGraph::new_source(output.id);
    speech
        .add_source_operator(
            OperatorTypeId::new("audio.speech_synthesize").unwrap(),
            OperatorDataTypeId::new("audio.clip").unwrap(),
        )
        .unwrap();
    project.save_artifact_working_graph(&speech).unwrap();
    drop(project);
    let project = ShapeProject::open_read_only(&bundle).unwrap();
    let before = fs::read(bundle.join("project.sqlite")).unwrap();
    let request = json!({"request_id":"unconfigured-v1","text_artifact_id":text.id,"expected_text_head":null,"authoring_graph_digest":graph_digest(&author).unwrap(),"speech_target_artifact_id":output.id,"speech_graph_digest":graph_digest(&speech).unwrap(),"destination":root.join("transfer")});
    fs::write(
        root.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    let result: Value = serde_json::from_str(
        &project
            .export_vox_transfer_json(&request.to_string())
            .unwrap(),
    )
    .unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(result["manifest_path"].as_str().unwrap()).unwrap())
            .unwrap();
    assert!(manifest["accepted_text"].is_null());
    assert!(manifest["speech_draft"]["settings"].is_null());
    assert!(manifest["speech_draft"]["operation"].is_null());
    assert_eq!(
        manifest["speech_draft"]["issue"],
        "speech_settings_unconfigured"
    );
    assert_eq!(fs::read(bundle.join("project.sqlite")).unwrap(), before);
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    drop(project);
    if retained.is_none() {
        fs::remove_dir_all(root).unwrap();
    }
}
