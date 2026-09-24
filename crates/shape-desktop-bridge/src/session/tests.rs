use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{
    AiImageGenerateParameters, AiImageOutputCanvas, ArtifactContentContract, ArtifactKind,
    AudioOriginDisclosure, AudioValueContract, ImageColorProfile, ImageRasterContract, IntentSpec,
    PresetVoiceAlias, PresetVoiceSelection, SpeechSynthesisOperation, SpeechVoiceSelection,
};
use shape_execution::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, CapabilityId, ExecutionFailure, ExecutionOutput,
    ExecutionRequest, Executor, ExecutorIdentity, ExternalAttemptProvenance,
    ExternalExecutionProvenance, ExternalRoutingCandidate, IMAGE_GENERATE_CAPABILITY,
    INFER_RUNTIME_CONTRACT_VERSION, INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
};
use uuid::Uuid;

use super::{create_desktop_project, open_desktop_session};
use crate::infer_image::{
    ImageGenerationControl, InferImageBatch, InferImageCandidate, execute_image_batch,
    image_generation_control_cancel, image_generation_control_has_candidate,
    image_generation_control_take_candidate,
};
use crate::infer_speech::InferSpeechCandidate;
use crate::operator_catalog::{AUDIO_SPEECH_OPERATOR, IMAGE_CROP_OPERATOR, IMAGE_RESIZE_OPERATOR};

#[derive(Debug)]
struct BridgeSpeechExecutor {
    identity: ExecutorIdentity,
}

#[derive(Debug)]
struct BridgeImageExecutor {
    identity: ExecutorIdentity,
}

impl BridgeImageExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.bridge.test.image",
                "1",
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("executor identity is valid"),
        }
    }
}

impl Executor for BridgeImageExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == IMAGE_GENERATE_CAPABILITY
    }

    fn execute(&self, _request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&[120_u8; 4 * 3 * 4], 4, 3, ColorType::Rgba8.into())
            .unwrap();
        Ok(ExecutionOutput {
            bytes,
            media_type: "image/png".to_owned(),
            executor_job_id: Some("resp_shape_image_bridge_1".to_owned()),
            external_provenance: Some(bridge_image_provenance()),
            content_contract: Some(ArtifactContentContract::ImageRaster(
                ImageRasterContract::rgba8(4, 3, ImageColorProfile::Srgb).unwrap(),
            )),
        })
    }
}

struct StopImageBatchAfterFirst<'a> {
    inner: BridgeImageExecutor,
    control: &'a ImageGenerationControl,
    calls: AtomicUsize,
}

impl Executor for StopImageBatchAfterFirst<'_> {
    fn identity(&self) -> &ExecutorIdentity {
        self.inner.identity()
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        self.inner.supports(capability)
    }

    fn execute(&self, request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        let result = self.inner.execute(request);
        if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
            image_generation_control_cancel(self.control);
        }
        result
    }
}

impl BridgeSpeechExecutor {
    fn new() -> Self {
        Self {
            identity: ExecutorIdentity::new(
                "shape.bridge.test.speech",
                "1",
                INFER_RUNTIME_CONTRACT_VERSION,
            )
            .expect("executor identity is valid"),
        }
    }
}

impl Executor for BridgeSpeechExecutor {
    fn identity(&self) -> &ExecutorIdentity {
        &self.identity
    }

    fn supports(&self, capability: &CapabilityId) -> bool {
        capability.as_str() == AUDIO_SPEECH_SYNTHESIZE_CAPABILITY
    }

    fn execute(&self, _request: &ExecutionRequest) -> Result<ExecutionOutput, ExecutionFailure> {
        Ok(ExecutionOutput {
            bytes: bridge_wav(),
            media_type: "audio/wav".to_owned(),
            executor_job_id: Some("job_shape_audio_bridge_1".to_owned()),
            external_provenance: Some(bridge_provenance()),
            content_contract: Some(ArtifactContentContract::AudioClip(
                AudioValueContract::pcm_s16le_wav(
                    24_000,
                    1,
                    24_000,
                    AudioOriginDisclosure::SyntheticSpeech,
                )
                .expect("audio contract is valid"),
            )),
        })
    }
}

fn bridge_wav() -> Vec<u8> {
    let sample_rate_hz = 24_000_u32;
    let channels = 1_u16;
    let frames = 24_000_u32;
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

fn bridge_speech_operation() -> SpeechSynthesisOperation {
    SpeechSynthesisOperation::new(
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
        SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1)
                    .expect("voice alias is valid"),
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            )
            .expect("voice selection is valid"),
        ),
        1_000,
        true,
    )
    .expect("speech operation is valid")
}

fn bridge_provenance() -> ExternalExecutionProvenance {
    ExternalExecutionProvenance {
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

fn bridge_image_provenance() -> ExternalExecutionProvenance {
    let mut provenance = bridge_provenance();
    provenance.capability_contract =
        Some(shape_execution::INFER_RUNTIME_RESPONSES_CAPABILITY.to_owned());
    provenance.intent = IMAGE_GENERATE_CAPABILITY.to_owned();
    provenance.provider = "codex-subscription".to_owned();
    provenance.deployment = "codex_gpt_5_6_luna".to_owned();
    provenance.model_profile = "codex_gpt_5_6_luna".to_owned();
    provenance.model_build = "codex_gpt_5_6_luna_subscription".to_owned();
    provenance.physical_model = "gpt-5.6-luna".to_owned();
    provenance.placement = "cloud".to_owned();
    provenance.capability_level = "advanced".to_owned();
    provenance.policy = "balanced".to_owned();
    provenance.requested_policy = "balanced".to_owned();
    provenance.requested_provider_access_class = Some("subscription".to_owned());
    provenance.requested_placement = "cloud_only".to_owned();
    provenance.requested_preference = "cloud".to_owned();
    provenance.offline_required = false;
    provenance.requested_latency = None;
    provenance.routing_candidates[0].provider = "codex-subscription".to_owned();
    provenance.routing_candidates[0].deployment = "codex_gpt_5_6_luna".to_owned();
    provenance.attempts[0].provider = "codex-subscription".to_owned();
    provenance.attempts[0].deployment = "codex_gpt_5_6_luna".to_owned();
    provenance
}

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-desktop-session-{}", Uuid::now_v7()))
}

fn seeded_project(root: &PathBuf) -> shape_domain::ArtifactId {
    let mut project = ShapeProject::create(root, "Desktop Session").expect("project creates");
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let initial = project
        .propose_text(
            artifact.id,
            None,
            "A summer afternoon.",
            IntentSpec::new("Import initial text").expect("intent valid"),
            Vec::new(),
        )
        .expect("candidate executes");
    project.accept_text(initial).expect("candidate accepts");
    artifact.id
}

#[test]
fn empty_project_can_create_a_text_scene_and_drive_an_operator_draft() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "Debug Entry").expect("project creates");
    assert!(
        session
            .session_snapshot()
            .expect("empty snapshot reads")
            .artifacts
            .is_empty()
    );

    let snapshot = session
        .session_create_text_document("Opening", "A first line.")
        .expect("text scene creates");
    let artifact = snapshot.artifacts.first().expect("text source projects");
    assert_eq!(artifact.name, "Opening");
    assert_eq!(artifact.text_preview, "A first line.");
    assert_eq!(artifact.operator_graph_nodes.len(), 2);

    let draft = session
        .session_begin_operator_draft(&artifact.id, "text.edit")
        .expect("draft begins");
    assert_ne!(draft.context_artifact_id, artifact.id);
    assert_eq!(draft.input_artifact_id, artifact.id);
    assert_eq!(draft.operator_type_key, "text.edit");
    assert_eq!(session.session_operator_drafts().len(), 1);
    let repeated = session
        .session_begin_operator_draft(&artifact.id, "text.edit")
        .expect("draft reuses");
    assert_ne!(repeated.draft_id, draft.draft_id);
    assert_ne!(repeated.context_artifact_id, draft.context_artifact_id);
    assert_eq!(
        session
            .session_operator_descriptors(&artifact.id)
            .expect("catalog projects")
            .len(),
        10
    );

    drop(session);
    let mut session = open_desktop_session(path).expect("project reopens with Working Graph");
    let restored = session.session_operator_drafts();
    assert_eq!(restored.len(), 2);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(restored[0].operator_type_key, "text.edit");

    session
        .session_propose_text(&artifact.id, "A revised first line.")
        .expect("candidate executes");
    assert_eq!(session.session_operator_drafts().len(), 2);
    assert_eq!(session.session_candidates().len(), 1);
    drop(session);

    let reopened = open_desktop_session(path).expect("project reopens");
    assert_eq!(reopened.session_operator_drafts().len(), 2);
    assert!(reopened.session_candidates().is_empty());
    assert_eq!(
        reopened
            .session_snapshot()
            .expect("reopened snapshot reads")
            .artifacts[0]
            .text_preview,
        "A first line."
    );
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn text_transform_configuration_restores_with_its_exact_draft_identity() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "Transform Draft").expect("project creates");
    let snapshot = session
        .session_create_text_document("Opening", "A first line.")
        .expect("text scene creates");
    let artifact_id = snapshot.artifacts[0].id.clone();
    let draft = session
        .session_begin_operator_draft(&artifact_id, "text.transform")
        .expect("legacy transform entry opens the Writing draft");
    assert_eq!(draft.operator_type_key, "text.edit");
    let updated = session
        .session_update_text_transform_draft(
            &draft.draft_id,
            "polish",
            "  Make it warmer, but preserve the title.  ",
            "warm",
            "literary",
            3,
        )
        .expect("instruction saves");
    assert_eq!(updated.draft_id, draft.draft_id);
    assert_eq!(updated.text_transform_mode, "polish");
    assert_eq!(updated.text_transform_tone, "warm");
    assert_eq!(updated.text_transform_style, "literary");
    assert_eq!(updated.text_transform_variant_count, 3);
    assert_eq!(
        updated.text_transform_instruction,
        "  Make it warmer, but preserve the title.  "
    );
    assert!(!updated.configuration_schema.is_empty());
    drop(session);

    let mut reopened = open_desktop_session(path).expect("project reopens");
    let restored = reopened.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(restored[0].text_transform_mode, "polish");
    assert_eq!(
        restored[0].text_transform_instruction,
        "  Make it warmer, but preserve the title.  "
    );
    let cleared = reopened
        .session_update_text_transform_draft(
            &draft.draft_id,
            "rewrite",
            "",
            "neutral",
            "natural",
            1,
        )
        .expect("instruction clears");
    assert_eq!(cleared.text_transform_mode, "rewrite");
    assert!(cleared.text_transform_instruction.is_empty());
    assert!(!cleared.configuration_schema.is_empty());
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn text_expression_snapshot_survives_project_reopen() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "Expression Draft").expect("project creates");
    let snapshot = session
        .session_create_text_document("Opening", "A first line.")
        .expect("text scene creates");
    let artifact_id = snapshot.artifacts[0].id.clone();
    let draft = session
        .session_begin_operator_draft(&artifact_id, "text.edit")
        .expect("Writing draft opens");
    let expression = r#"{"tones":[{"kind":"preset","preset":"empathetic"},{"kind":"custom","name":"Quiet conviction","instruction":"Stay certain without becoming forceful.","example":"This is the right direction; we can proceed carefully.","visual":"ascent"}],"intensity":"subtle","audience":{"kind":"preset","preset":"expert"}}"#;
    let updated = session
        .session_update_text_expression_draft(
            &draft.draft_id,
            "polish",
            "Keep every number.",
            expression,
            "professional",
            3,
        )
        .expect("expression saves");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.text_transform_expression_json).unwrap(),
        serde_json::from_str::<serde_json::Value>(expression).unwrap()
    );
    drop(session);

    let reopened = open_desktop_session(path).expect("project reopens");
    let restored = reopened.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(
        restored[0].configuration_schema,
        "shape.operator-draft.text-transform@20260813.1"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&restored[0].text_transform_expression_json)
            .unwrap(),
        serde_json::from_str::<serde_json::Value>(expression).unwrap()
    );
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn detached_text_editor_can_be_authored_before_any_material_is_connected() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "Detached Text").expect("project creates");
    let snapshot = session
        .session_create_detached_text_editor("Untitled AI text")
        .expect("detached text editor creates atomically");
    assert_eq!(snapshot.artifacts.len(), 1);
    let artifact = &snapshot.artifacts[0];
    assert_eq!(artifact.name, "Untitled AI text");
    assert_eq!(artifact.kind_key, "text_document");
    assert!(!artifact.has_accepted_revision);
    assert_eq!(artifact.operator_graph_nodes.len(), 2);

    let drafts = session.session_operator_drafts();
    assert_eq!(drafts.len(), 1);
    let draft = &drafts[0];
    assert_eq!(draft.context_artifact_id, artifact.id);
    assert_eq!(draft.operator_type_key, "text.create");
    assert!(!draft.has_input_data_type);
    assert_eq!(draft.output_data_type_key, "text.document");
    assert_eq!(draft.text_transform_mode, "rewrite");
    assert_eq!(draft.text_transform_tone, "neutral");
    assert_eq!(draft.text_transform_style, "natural");
    assert_eq!(draft.text_transform_variant_count, 1);
    let draft_id = draft.draft_id.clone();
    session
        .session_update_text_transform_draft(
            &draft_id,
            "summarize",
            "Keep the conclusion explicit.",
            "confident",
            "concise",
            3,
        )
        .expect("detached intent saves without a material input");
    drop(session);

    let reopened = open_desktop_session(path).expect("project reopens");
    let restored = reopened.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft_id);
    assert_eq!(restored[0].text_transform_mode, "summarize");
    assert_eq!(restored[0].text_transform_tone, "confident");
    assert_eq!(restored[0].text_transform_style, "concise");
    assert_eq!(restored[0].text_transform_variant_count, 3);
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn ai_image_scene_persists_a_zero_input_source_draft_and_exact_canvas() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "AI Image Draft").expect("project creates");
    let snapshot = session
        .session_create_ai_image_draft("Cobalt bird", "A cobalt glass bird", 1536, 1024)
        .expect("AI image source draft creates atomically");
    assert_eq!(snapshot.artifacts.len(), 1);
    let artifact = &snapshot.artifacts[0];
    assert_eq!(artifact.name, "Cobalt bird");
    assert_eq!(artifact.kind_key, "image_raster");
    assert!(!artifact.has_accepted_revision);
    assert!(artifact.operator_graph_nodes.is_empty());

    let drafts = session.session_operator_drafts();
    assert_eq!(drafts.len(), 1);
    let draft = &drafts[0];
    assert_eq!(draft.context_artifact_id, artifact.id);
    assert_eq!(draft.operator_type_key, "image.generate");
    assert!(!draft.has_input_data_type);
    assert!(draft.input_data_type_key.is_empty());
    assert_eq!(draft.output_data_type_key, "image.raster");
    assert_eq!(draft.ai_image_instruction, "A cobalt glass bird");
    assert_eq!(
        (draft.ai_image_output_width, draft.ai_image_output_height),
        (1536, 1024)
    );
    let draft_id = draft.draft_id.clone();
    session
        .session_update_ai_image_draft(&draft_id, "A cobalt paper bird", 1024, 1024, 3)
        .expect("AI image authored state saves");
    drop(session);

    let mut reopened = open_desktop_session(path).expect("project reopens");
    let restored = reopened.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft_id);
    assert!(!restored[0].has_input_data_type);
    assert_eq!(restored[0].ai_image_instruction, "A cobalt paper bird");
    reopened
        .session_discard_operator_draft(&draft_id)
        .expect("unfinished source node and empty output are removable");
    assert!(reopened.session_operator_drafts().is_empty());
    assert!(reopened.session_snapshot().unwrap().artifacts.is_empty());
    assert_eq!(
        (
            restored[0].ai_image_output_width,
            restored[0].ai_image_output_height
        ),
        (1024, 1024)
    );
    assert_eq!(restored[0].ai_image_candidate_count, 3);
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn ai_image_candidate_adopts_previews_accepts_and_reopens_as_operator_output() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "AI Image Candidate").unwrap();
    let snapshot = session
        .session_create_ai_image_draft("Cobalt bird", "A cobalt glass bird", 4, 3)
        .unwrap();
    let artifact_id = snapshot.artifacts[0]
        .id
        .parse::<shape_domain::ArtifactId>()
        .unwrap();
    let draft_id = session.session_operator_drafts()[0].draft_id.clone();

    let project = ShapeProject::open(&root).unwrap();
    let parameters = AiImageGenerateParameters::new(
        "A cobalt glass bird",
        AiImageOutputCanvas::new(4, 3).unwrap(),
        1,
        Vec::new(),
    )
    .unwrap();
    let generated = project
        .propose_generated_image(artifact_id, &parameters, &BridgeImageExecutor::new())
        .unwrap();
    let expected_bytes = generated.bytes().to_vec();
    let adopted = session
        .session_adopt_infer_image(Box::new(InferImageCandidate::new(
            generated,
            draft_id.clone(),
        )))
        .unwrap();
    assert!(adopted.has_image_preview);
    assert!(!adopted.has_expected_head);
    assert_eq!((adopted.image_width, adopted.image_height), (4, 3));
    assert_eq!(session.session_operator_drafts()[0].draft_id, draft_id);
    assert_eq!(
        session
            .session_image_preview(&artifact_id.to_string(), &adopted.candidate_id)
            .unwrap()
            .png_bytes,
        expected_bytes
    );

    let accepted = session
        .session_accept_candidate(&adopted.candidate_id)
        .unwrap();
    assert!(accepted.artifacts[0].has_accepted_revision);
    assert_eq!(accepted.artifacts[0].operator_graph_nodes.len(), 2);
    assert_eq!(
        accepted.artifacts[0].operator_graph_nodes[0].operator_type_key,
        "image.generate"
    );
    assert!(session.session_operator_drafts().is_empty());
    drop(session);

    let reopened = open_desktop_session(path).unwrap();
    assert!(reopened.session_operator_drafts().is_empty());
    assert_eq!(
        reopened
            .session_image_preview(&artifact_id.to_string(), "")
            .unwrap()
            .png_bytes,
        expected_bytes
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn image_batch_adopts_distinct_candidates_and_accepts_only_the_chosen_one() {
    let root = test_root();
    let path = root.to_str().unwrap();
    let mut session = create_desktop_project(path, "Image options").unwrap();
    let snapshot = session
        .session_create_ai_image_draft("Cobalt bird", "A cobalt glass bird", 4, 3)
        .unwrap();
    let artifact_id = snapshot.artifacts[0]
        .id
        .parse::<shape_domain::ArtifactId>()
        .unwrap();
    let draft_id = session.session_operator_drafts()[0].draft_id.clone();
    session
        .session_update_ai_image_draft(&draft_id, "A cobalt glass bird", 4, 3, 2)
        .unwrap();
    let project = ShapeProject::open(&root).unwrap();
    let parameters = AiImageGenerateParameters::new(
        "A cobalt glass bird",
        AiImageOutputCanvas::new(4, 3).unwrap(),
        2,
        Vec::new(),
    )
    .unwrap();
    let first = project
        .propose_generated_image(artifact_id, &parameters, &BridgeImageExecutor::new())
        .unwrap();
    let second = project
        .propose_generated_image(artifact_id, &parameters, &BridgeImageExecutor::new())
        .unwrap();
    let adopted = session
        .session_adopt_infer_image_batch(Box::new(InferImageBatch::new(
            vec![first, second],
            draft_id,
            "",
        )))
        .unwrap();
    assert_eq!(adopted.len(), 2);
    assert_ne!(adopted[0].candidate_id, adopted[1].candidate_id);
    assert_eq!(session.session_candidates().len(), 2);
    assert!(!session.session_snapshot().unwrap().artifacts[0].has_accepted_revision);
    session
        .session_accept_candidate(&adopted[0].candidate_id)
        .unwrap();
    assert!(session.session_candidates().is_empty());
    assert!(session.session_snapshot().unwrap().artifacts[0].has_accepted_revision);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stopped_image_batch_streams_the_completed_candidate_without_starting_another_job() {
    let root = test_root();
    let path = root.to_str().unwrap();
    let mut session = create_desktop_project(path, "Stopped image batch").unwrap();
    let snapshot = session
        .session_create_ai_image_draft("Bird", "A cobalt glass bird", 4, 3)
        .unwrap();
    let artifact_id = snapshot.artifacts[0]
        .id
        .parse::<shape_domain::ArtifactId>()
        .unwrap();
    let draft_id = session.session_operator_drafts()[0].draft_id.clone();
    session
        .session_update_ai_image_draft(&draft_id, "A cobalt glass bird", 4, 3, 3)
        .unwrap();
    let parameters = AiImageGenerateParameters::new(
        "A cobalt glass bird",
        AiImageOutputCanvas::new(4, 3).unwrap(),
        3,
        Vec::new(),
    )
    .unwrap();
    let project = ShapeProject::open(&root).unwrap();
    let control = ImageGenerationControl::default();
    let executor = StopImageBatchAfterFirst {
        inner: BridgeImageExecutor::new(),
        control: &control,
        calls: AtomicUsize::new(0),
    };
    let batch = execute_image_batch(
        &project,
        artifact_id,
        &draft_id,
        &parameters,
        3,
        &executor,
        Some(&control),
    )
    .unwrap();
    assert_eq!(executor.calls.load(Ordering::Relaxed), 1);
    assert_eq!(batch.into_parts().2, "generation_cancelled");
    assert!(image_generation_control_has_candidate(&control));
    let ready = image_generation_control_take_candidate(&control).unwrap();
    assert!(!image_generation_control_has_candidate(&control));
    let adopted = session.session_adopt_infer_image(ready).unwrap();
    assert!(adopted.has_image_preview);
    assert_eq!(session.session_candidates().len(), 1);
    assert!(!session.session_snapshot().unwrap().artifacts[0].has_accepted_revision);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn speech_draft_configuration_persists_and_failed_stale_save_rolls_back() {
    let root = test_root();
    let path = root.to_str().expect("portable path");
    let mut session = create_desktop_project(path, "Speech Draft").expect("project creates");
    let snapshot = session
        .session_create_text_document("Opening", "A first line.")
        .expect("text scene creates");
    let artifact_id = snapshot.artifacts[0].id.clone();
    let draft = session
        .session_begin_operator_draft(&artifact_id, AUDIO_SPEECH_OPERATOR)
        .expect("speech draft begins with executable defaults");
    assert_eq!(
        draft.audio_speech_preset_alias,
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1
    );
    assert_eq!(
        draft.audio_speech_preset_catalog_revision,
        shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION
    );
    assert_eq!(draft.audio_speech_language, "auto");
    assert_eq!(draft.audio_speech_speed_milli, 1_000);
    assert!(draft.audio_speech_disclosure_required);
    let updated = session
        .session_update_audio_speech_draft(
            &draft.draft_id,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
            INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
            1_150,
            true,
        )
        .expect("speech configuration saves");
    assert_eq!(updated.audio_speech_speed_milli, 1_150);
    drop(session);

    let mut reopened = open_desktop_session(path).expect("project reopens");
    let restored = reopened.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(restored[0].audio_speech_speed_milli, 1_150);
    assert!(restored[0].audio_speech_disclosure_required);

    let artifact_id = artifact_id.parse::<shape_domain::ArtifactId>().unwrap();
    let expected_head = reopened.project.snapshot().unwrap().artifacts[0]
        .accepted_revision
        .unwrap();
    let newer = reopened
        .project
        .propose_text(
            artifact_id,
            Some(expected_head),
            "A newer accepted line.",
            IntentSpec::new("Advance source behind stale Working Graph").unwrap(),
            Vec::new(),
        )
        .unwrap();
    reopened.project.accept_text(newer).unwrap();
    assert!(
        reopened
            .session_update_audio_speech_draft(
                &draft.draft_id,
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
                INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION,
                INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
                900,
                true,
            )
            .is_ok()
    );
    assert_eq!(
        reopened.session_operator_drafts()[0].audio_speech_speed_milli,
        900,
        "voice settings remain editable when the original is stale"
    );
    assert_ne!(
        reopened.session_operator_drafts()[0].input_revision_id,
        reopened
            .project
            .read_accepted(artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id
            .to_string()
    );
    reopened
        .session_refresh_text_input(&draft.draft_id)
        .unwrap();
    assert_eq!(
        reopened.session_operator_drafts()[0].input_revision_id,
        reopened
            .project
            .read_accepted(artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id
            .to_string()
    );
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn candidate_is_transient_until_acceptance_and_survives_reopen_after_commit() {
    use crate::operator_catalog::text_authoring::{TextAuthoring, WritingEntry};
    let root = test_root();
    let source_id = seeded_project(&root).to_string();
    let mut session = open_desktop_session(root.to_str().unwrap()).unwrap();
    let original_head = session.session_snapshot().unwrap().artifacts[0]
        .accepted_revision_id
        .clone();
    let draft = session
        .session_begin_operator_draft(&source_id, "text.edit")
        .unwrap();
    let mut state = TextAuthoring::from_json(&draft.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "A quiet summer afternoon.".into();
    session
        .session_update_text_authoring(&draft.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let candidate = session
        .session_propose_authored_text(&draft.draft_id)
        .unwrap();
    let before = session.session_snapshot().unwrap();
    assert!(
        !before
            .artifacts
            .iter()
            .find(|a| a.id == draft.context_artifact_id)
            .unwrap()
            .has_accepted_revision
    );
    assert_eq!(before.graph_edges.len(), 1);
    let nodes_before = before
        .artifacts
        .iter()
        .find(|a| a.id == draft.context_artifact_id)
        .unwrap()
        .operator_graph_nodes
        .iter()
        .map(|n| n.node_id.clone())
        .collect::<Vec<_>>();
    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_eq!(
        accepted
            .artifacts
            .iter()
            .find(|a| a.id == source_id)
            .unwrap()
            .accepted_revision_id,
        original_head
    );
    let output = accepted
        .artifacts
        .iter()
        .find(|a| a.id == draft.context_artifact_id)
        .unwrap();
    assert_eq!(output.text_preview, state.text);
    assert_eq!(
        output
            .operator_graph_nodes
            .iter()
            .map(|n| n.node_id.clone())
            .collect::<Vec<_>>(),
        nodes_before
    );
    drop(session);
    let reopened = open_desktop_session(root.to_str().unwrap()).unwrap();
    assert!(reopened.session_candidates().is_empty());
    assert_eq!(
        reopened.session_operator_drafts()[0].draft_id,
        draft.draft_id
    );
    assert_eq!(reopened.session_snapshot().unwrap().graph_edges.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_reproposal_does_not_replace_a_valid_pending_candidate() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A calm summer afternoon.")
        .expect("candidate executes");
    assert!(
        session
            .session_propose_text("not-an-artifact-id", "Discard me")
            .is_err()
    );

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .expect("original candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A calm summer afternoon."
    );
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn pending_candidate_can_branch_as_a_new_artifact_with_a_source_edge() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let before = session.session_snapshot().expect("snapshot reads");
    let source_revision = before.artifacts[0].accepted_revision_id.clone();
    let first = session
        .session_propose_text(&artifact_id.to_string(), "A warm summer afternoon.")
        .expect("first candidate executes");
    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A quiet summer afternoon.")
        .expect("candidate executes");
    assert_eq!(session.session_candidates().len(), 2);

    let branched = session
        .session_branch_candidate(&candidate.candidate_id, "Story — Quiet")
        .expect("candidate branches");
    assert_eq!(branched.artifacts.len(), 2);
    let source = branched
        .artifacts
        .iter()
        .find(|artifact| artifact.id == artifact_id.to_string())
        .expect("source remains");
    assert_eq!(source.accepted_revision_id, source_revision);
    let branch = branched
        .artifacts
        .iter()
        .find(|artifact| artifact.name == "Story — Quiet")
        .expect("branch appears");
    assert!(branch.accepted_parent_revision_ids.is_empty());
    assert_eq!(
        branch.transformation_input_revision_ids,
        vec![source_revision.clone()]
    );
    assert_eq!(
        branch.transformation_input_artifact_ids,
        vec![artifact_id.to_string()]
    );
    assert_eq!(branch.transformation_input_artifact_names, vec!["Story"]);
    assert_eq!(branch.text_preview, "A quiet summer afternoon.");
    assert_eq!(branched.graph_edges.len(), 1);
    let edge = &branched.graph_edges[0];
    assert_eq!(edge.source_artifact_id, artifact_id.to_string());
    assert_eq!(edge.target_artifact_id, branch.id);
    assert_eq!(edge.source_revision_id, source_revision);
    assert_eq!(edge.target_revision_id, branch.accepted_revision_id);
    assert_eq!(edge.transformation_id, branch.transformation_id);
    assert_eq!(edge.transformation_kind_key, "text_rewrite");
    let remaining = session.session_candidates();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].candidate_id, first.candidate_id);
    session
        .session_discard_candidate(&first.candidate_id)
        .expect("remaining candidate discards");
    assert!(session.session_candidates().is_empty());
    drop(session);

    let reopened = open_desktop_session(path).expect("session reopens");
    assert_eq!(
        reopened
            .session_snapshot()
            .expect("snapshot reads")
            .artifacts
            .len(),
        2
    );
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn shelf_accumulates_candidates_and_accepting_one_clears_stale_siblings() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let first = session
        .session_propose_text(&artifact_id.to_string(), "A bright summer afternoon.")
        .expect("first candidate executes");
    let second = session
        .session_propose_text(&artifact_id.to_string(), "A still summer afternoon.")
        .expect("second candidate executes");
    assert!(
        session
            .session_propose_text(&artifact_id.to_string(), "A still summer afternoon.")
            .is_err()
    );

    let projected = session.session_candidates();
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].candidate_id, second.candidate_id);
    assert_eq!(projected[1].candidate_id, first.candidate_id);
    assert!(
        session
            .session_accept_candidate("missing-candidate")
            .is_err()
    );
    assert_eq!(session.session_candidates().len(), 2);

    let accepted = session
        .session_accept_candidate(&second.candidate_id)
        .expect("selected candidate accepts");
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A still summer afternoon."
    );
    assert!(session.session_candidates().is_empty());
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn source_revision_reads_the_pinned_text_and_external_audio_after_heads_change() {
    let root = test_root();
    let source = root.with_extension("wav");
    let wav = bridge_wav();
    fs::write(&source, &wav).unwrap();
    let mut session = create_desktop_project(root.to_str().unwrap(), "Source revisions").unwrap();
    let first = session
        .session_create_text_document("Story", "Earlier text")
        .unwrap();
    let text_id = first.artifacts[0].id.clone();
    let first_revision = first.artifacts[0].accepted_revision_id.clone();
    let candidate = session
        .session_propose_text(&text_id, "Later text")
        .unwrap();
    session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_eq!(
        session.session_source_text(&first_revision).unwrap(),
        "Earlier text"
    );
    let imported = session
        .session_import_audio_wav(source.to_str().unwrap(), "External audio")
        .unwrap();
    let audio = imported.artifacts.last().unwrap();
    let audio_revision = audio.accepted_revision_id.clone();
    assert_eq!(audio.audio_origin_key, "imported_unverified");
    fs::remove_file(&source).unwrap();
    assert_eq!(
        session
            .session_audio_revision_preview(&audio_revision)
            .unwrap()
            .wav_bytes,
        wav
    );
    assert!(session.session_source_text(&audio_revision).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn raster_import_crop_candidate_accept_and_reopen_cross_the_desktop_bridge() {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    let root = test_root();
    let source = root.with_extension("png");
    let pixels: Vec<u8> = (0_u8..48).flat_map(|value| [value, 40, 80, 255]).collect();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 8, 6, ColorType::Rgba8.into())
        .unwrap();
    fs::write(&source, png).unwrap();

    ShapeProject::create(&root, "Desktop Raster").unwrap();
    let path = root.to_str().unwrap();
    let mut session = open_desktop_session(path).unwrap();
    let imported = session
        .session_import_raster(source.to_str().unwrap(), "Cover")
        .unwrap();
    let artifact = imported.artifacts.first().unwrap();
    assert!(artifact.has_image_preview);
    assert_eq!((artifact.image_width, artifact.image_height), (8, 6));
    let imported_head = artifact.accepted_revision_id.clone();
    let accepted_preview = session.session_image_preview(&artifact.id, "").unwrap();
    assert_eq!((accepted_preview.width, accepted_preview.height), (8, 6));

    session
        .session_begin_operator_draft(&artifact.id, IMAGE_CROP_OPERATOR)
        .expect("frame tool draft begins");
    session
        .session_begin_operator_draft(&artifact.id, IMAGE_RESIZE_OPERATOR)
        .expect("size tool draft begins");
    assert_eq!(session.session_operator_drafts().len(), 2);

    let candidate = session
        .session_propose_raster_crop(&artifact.id, 1, 1, 4, 3)
        .unwrap();
    assert!(session.session_operator_drafts().is_empty());
    assert_eq!(candidate.kind_key, "image_raster");
    assert_eq!((candidate.image_width, candidate.image_height), (4, 3));
    assert_eq!(
        session.session_snapshot().unwrap().artifacts[0].accepted_revision_id,
        imported_head
    );
    let candidate_preview = session
        .session_image_preview(&artifact.id, &candidate.candidate_id)
        .unwrap();
    assert_eq!((candidate_preview.width, candidate_preview.height), (4, 3));

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_ne!(accepted.artifacts[0].accepted_revision_id, imported_head);
    assert_eq!(
        (
            accepted.artifacts[0].image_width,
            accepted.artifacts[0].image_height
        ),
        (4, 3)
    );
    assert_eq!(accepted.artifacts[0].operator_graph_nodes.len(), 3);
    assert_eq!(
        accepted.artifacts[0].operator_graph_nodes[1].operator_type_key,
        "image.crop"
    );
    drop(session);

    let reopened = open_desktop_session(path).unwrap();
    let preview = reopened
        .session_image_preview(&accepted.artifacts[0].id, "")
        .unwrap();
    assert_eq!((preview.width, preview.height), (4, 3));
    fs::remove_file(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn raster_resize_draft_candidate_accept_and_reopen_cross_the_desktop_bridge() {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    let root = test_root();
    let source = root.with_extension("resize.png");
    let pixels: Vec<u8> = (0_u8..48).flat_map(|value| [value, 60, 90, 255]).collect();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 8, 6, ColorType::Rgba8.into())
        .unwrap();
    fs::write(&source, png).unwrap();

    ShapeProject::create(&root, "Desktop Resize").unwrap();
    let path = root.to_str().unwrap();
    let mut session = open_desktop_session(path).unwrap();
    let imported = session
        .session_import_raster(source.to_str().unwrap(), "Resize Cover")
        .unwrap();
    let artifact = imported.artifacts.first().unwrap();
    let imported_head = artifact.accepted_revision_id.clone();
    let draft = session
        .session_begin_operator_draft(&artifact.id, "image.resize")
        .unwrap();
    assert_eq!(
        (
            draft.image_resize_target_width,
            draft.image_resize_target_height
        ),
        (8, 6)
    );
    assert_eq!(draft.image_resize_aspect_policy, "fit_within");
    assert_eq!(draft.image_resize_resampling, "lanczos3");
    assert!(
        session
            .session_propose_raster_resize(&artifact.id, &draft.draft_id)
            .is_err(),
        "identity resize remains a draft"
    );

    let configured = session
        .session_update_image_resize_draft(&draft.draft_id, 4, 4, "fit_within", "catmull_rom")
        .unwrap();
    assert_eq!(
        (
            configured.image_resize_target_width,
            configured.image_resize_target_height
        ),
        (4, 4)
    );
    drop(session);

    let mut session = open_desktop_session(path).unwrap();
    let restored = session.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(restored[0].image_resize_aspect_policy, "fit_within");
    assert_eq!(restored[0].image_resize_resampling, "catmull_rom");
    let candidate = session
        .session_propose_raster_resize(&artifact.id, &draft.draft_id)
        .unwrap();
    assert_eq!((candidate.image_width, candidate.image_height), (4, 3));
    assert_eq!(
        session.session_snapshot().unwrap().artifacts[0].accepted_revision_id,
        imported_head
    );
    let preview = session
        .session_image_preview(&artifact.id, &candidate.candidate_id)
        .unwrap();
    assert_eq!((preview.width, preview.height), (4, 3));

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .unwrap();
    assert_eq!(
        (
            accepted.artifacts[0].image_width,
            accepted.artifacts[0].image_height
        ),
        (4, 3)
    );
    assert_eq!(
        accepted.artifacts[0].operator_graph_nodes[1].operator_type_key,
        "image.resize"
    );
    drop(session);

    let reopened = open_desktop_session(path).unwrap();
    let preview = reopened
        .session_image_preview(&accepted.artifacts[0].id, "")
        .unwrap();
    assert_eq!((preview.width, preview.height), (4, 3));
    fs::remove_file(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn transform_blur_unsharp_and_shadow_cross_the_desktop_candidate_boundary() {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    let root = test_root();
    let source = root.with_extension("effects.png");
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255, 255, 0, 255, 255, 0, 255,
        255, 255,
    ];
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 3, 2, ColorType::Rgba8.into())
        .unwrap();
    fs::write(&source, png).unwrap();

    ShapeProject::create(&root, "Desktop Effects").unwrap();
    let path = root.to_str().unwrap();
    let mut session = open_desktop_session(path).unwrap();
    let imported = session
        .session_import_raster(source.to_str().unwrap(), "Effects")
        .unwrap();
    let artifact_id = imported.artifacts[0].id.clone();

    let transform = session
        .session_propose_raster_transform(&artifact_id, "rotate90_clockwise")
        .unwrap();
    assert_eq!((transform.image_width, transform.image_height), (2, 3));
    session
        .session_accept_candidate(&transform.candidate_id)
        .unwrap();

    let blur = session
        .session_propose_raster_blur(&artifact_id, 2)
        .unwrap();
    assert_eq!((blur.image_width, blur.image_height), (2, 3));
    session
        .session_accept_candidate(&blur.candidate_id)
        .unwrap();

    let unsharp = session
        .session_propose_raster_unsharp_mask(&artifact_id, 1, 1_250, 4)
        .unwrap();
    assert_eq!((unsharp.image_width, unsharp.image_height), (2, 3));
    session
        .session_accept_candidate(&unsharp.candidate_id)
        .unwrap();

    let shadow = session
        .session_propose_raster_drop_shadow(&artifact_id, 1, 2, 0, 0, 0, 0, 128)
        .unwrap();
    assert_eq!((shadow.image_width, shadow.image_height), (3, 5));
    let preview = session
        .session_image_preview(&artifact_id, &shadow.candidate_id)
        .unwrap();
    assert_eq!((preview.width, preview.height), (3, 5));
    let accepted = session
        .session_accept_candidate(&shadow.candidate_id)
        .unwrap();
    let operator_types = accepted.artifacts[0]
        .operator_graph_nodes
        .iter()
        .map(|node| node.operator_type_key.as_str())
        .collect::<Vec<_>>();
    assert!(operator_types.contains(&"image.transform"));
    assert!(operator_types.contains(&"image.blur"));
    assert!(operator_types.contains(&"image.unsharp_mask"));
    assert!(operator_types.contains(&"image.drop_shadow"));
    drop(session);

    let reopened = open_desktop_session(path).unwrap();
    let preview = reopened.session_image_preview(&artifact_id, "").unwrap();
    assert_eq!((preview.width, preview.height), (3, 5));
    fs::remove_file(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deriving_another_text_preserves_original_audio_and_its_pending_candidates() {
    use crate::operator_catalog::text_authoring::{TextAuthoring, WritingEntry};
    let root = test_root();
    let source_id = seeded_project(&root);
    let path = root.to_str().unwrap();
    let mut session = open_desktop_session(path).unwrap();
    let writer = session
        .session_begin_text_authoring(&source_id.to_string(), "plain")
        .unwrap();
    let speech_draft = session
        .session_begin_authoring_speech(&source_id.to_string())
        .unwrap();
    let project = ShapeProject::open(&root).unwrap();
    let head = project.snapshot().unwrap().artifacts[0]
        .accepted_revision
        .unwrap();
    let make_speech = || {
        project
            .propose_speech_synthesis(
                source_id,
                head,
                "Recording",
                &bridge_speech_operation(),
                Vec::new(),
                &BridgeSpeechExecutor::new(),
            )
            .unwrap()
    };
    let accepted_candidate = make_speech();
    let audio_id = accepted_candidate.artifact_id().to_string();
    let audio_bytes = accepted_candidate.bytes().to_vec();
    let preview_candidate = make_speech();
    drop(project);
    let wire = session
        .session_adopt_infer_speech(Box::new(InferSpeechCandidate::new(accepted_candidate)))
        .unwrap();
    session
        .session_accept_candidate(&wire.candidate_id)
        .unwrap();
    session
        .session_adopt_infer_speech(Box::new(InferSpeechCandidate::new(preview_candidate)))
        .unwrap();
    assert_eq!(session.session_candidates().len(), 1);
    let mut state = TextAuthoring::from_json(&writer.text_authoring_json).unwrap();
    state.entry = WritingEntry::Manual;
    state.text = "A revised recording script.".into();
    session
        .session_update_text_authoring(&writer.draft_id, &serde_json::to_string(&state).unwrap())
        .unwrap();
    let text = session
        .session_propose_authored_text(&writer.draft_id)
        .unwrap();
    session
        .session_accept_candidate(&text.candidate_id)
        .unwrap();
    assert_eq!(session.session_candidates().len(), 1);
    let retained = session
        .session_begin_authoring_speech(&source_id.to_string())
        .unwrap();
    assert_eq!(retained.draft_id, speech_draft.draft_id);
    assert_eq!(
        retained.audio_speech_preset_alias,
        speech_draft.audio_speech_preset_alias
    );
    assert_eq!(
        session
            .session_audio_preview(&audio_id, "")
            .unwrap()
            .wav_bytes,
        audio_bytes
    );
    drop(session);
    let reopened = open_desktop_session(path).unwrap();
    assert_eq!(
        reopened
            .session_audio_preview(&audio_id, "")
            .unwrap()
            .wav_bytes,
        audio_bytes
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn speech_candidate_stays_source_scoped_until_acceptance_then_reopens_as_audio() {
    let root = test_root();
    let source_id = seeded_project(&root);
    let project = ShapeProject::open(&root).expect("project opens for background execution");
    let source_head = project.snapshot().expect("snapshot reads").artifacts[0]
        .accepted_revision
        .expect("source has accepted text");
    let speech = project
        .propose_speech_synthesis(
            source_id,
            source_head,
            "Mandarin narration",
            &bridge_speech_operation(),
            Vec::new(),
            &BridgeSpeechExecutor::new(),
        )
        .expect("speech candidate executes");
    let audio_id = speech.artifact_id();
    let expected_bytes = speech.bytes().to_vec();
    drop(project);

    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let adopted = session
        .session_adopt_infer_speech(Box::new(InferSpeechCandidate::new(speech)))
        .expect("speech candidate adopts");
    assert_eq!(adopted.artifact_id, audio_id.to_string());
    assert_eq!(adopted.context_artifact_id, source_id.to_string());
    assert_eq!(adopted.artifact_name, "Mandarin narration");
    assert!(adopted.has_audio_preview);
    assert_eq!(adopted.audio_duration_millis, 1_000);
    assert_eq!(adopted.audio_sample_rate_hz, 24_000);
    assert_eq!(adopted.audio_channels, 1);
    assert_eq!(adopted.audio_origin_key, "synthetic_speech");
    assert_eq!(session.session_snapshot().unwrap().artifacts.len(), 1);

    let transient = session
        .session_audio_preview(&source_id.to_string(), &adopted.candidate_id)
        .expect("selected transient WAV loads");
    assert_eq!(transient.wav_bytes, expected_bytes);
    assert_eq!(transient.duration_millis, 1_000);

    let accepted = session
        .session_accept_candidate(&adopted.candidate_id)
        .expect("speech candidate accepts");
    assert_eq!(accepted.artifacts.len(), 2);
    let audio = accepted
        .artifacts
        .iter()
        .find(|artifact| artifact.id == audio_id.to_string())
        .expect("accepted audio appears");
    assert!(audio.has_audio_preview);
    assert_eq!(audio.audio_duration_millis, 1_000);
    assert_eq!(audio.audio_origin_key, "synthetic_speech");
    assert_eq!(audio.operator_graph_nodes.len(), 3);
    assert_eq!(
        audio.operator_graph_nodes[1].operator_type_key,
        "audio.speech_synthesize"
    );
    let durable = session
        .session_audio_preview(&audio.id, "")
        .expect("accepted WAV loads");
    assert_eq!(durable.wav_bytes, expected_bytes);
    drop(session);

    let reopened = open_desktop_session(path).expect("session reopens");
    let reopened_audio = reopened
        .session_audio_preview(&audio_id.to_string(), "")
        .expect("reopened WAV loads");
    assert_eq!(reopened_audio.wav_bytes, expected_bytes);
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn script_preview_reads_full_source_and_bindings_and_cue_bytes_survive_reopen() {
    use shape_domain::speech_script::{SpeechCueAction, SpeechScriptOptions};
    let root = test_root();
    let path = root.to_str().unwrap();
    let mut session = create_desktop_project(path, "Script").unwrap();
    let text = format!(
        "[role: Reader]\n[cue: End]\n# Script\n[speaker: Reader]\n[repeat: 2; gap: 1s; cue: End]\n{}\n[end-repeat]\n[pause: 1.25s]\n",
        "Hello world. ".repeat(2000)
    );
    let snapshot = session
        .session_create_text_document("Source", &text)
        .unwrap();
    let artifact_id = snapshot.artifacts[0].id.clone();
    let draft = session
        .session_begin_operator_draft(&artifact_id, AUDIO_SPEECH_OPERATOR)
        .unwrap();
    session
        .session_update_speech_script(
            &draft.draft_id,
            &serde_json::to_string(&SpeechScriptOptions::default()).unwrap(),
        )
        .unwrap();
    let preview: serde_json::Value = serde_json::from_str(
        &session
            .session_speech_script_preview(&draft.draft_id)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["pause_ms"], 2250);
    assert_eq!(preview["cues"][0], "End");
    assert_eq!(preview["ready"], false);
    let mut bindings = SpeechScriptOptions::default();
    let SpeechVoiceSelection::Preset(mut voice) = bridge_speech_operation().voice else {
        panic!()
    };
    voice.catalog_revision = shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION.into();
    bindings.roles.insert("Reader".into(), voice);
    session
        .session_update_speech_script(&draft.draft_id, &serde_json::to_string(&bindings).unwrap())
        .unwrap();
    let updated = session
        .session_import_speech_cue(&draft.draft_id, "End", &bridge_wav())
        .unwrap();
    let options: SpeechScriptOptions =
        serde_json::from_str(&updated.audio_speech_script_json).unwrap();
    let SpeechCueAction::Audio { content } = &options.cues["End"] else {
        panic!("imported cue");
    };
    assert_eq!(
        content.digest,
        shape_domain::ContentDigest::from_bytes(&bridge_wav())
    );
    let updated = session
        .session_update_audio_speech_draft(
            &draft.draft_id,
            INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
            shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION,
            "auto",
            1150,
            true,
        )
        .unwrap();
    assert_eq!(
        updated.audio_speech_script_json,
        serde_json::to_string(&options).unwrap()
    );
    assert!(
        session
            .session_import_speech_cue(&draft.draft_id, "End", b"invalid WAV")
            .is_err()
    );
    drop(session);
    let reopened = open_desktop_session(path).unwrap();
    assert_eq!(
        reopened.session_operator_drafts()[0].audio_speech_script_json,
        updated.audio_speech_script_json
    );
    let preview: serde_json::Value = serde_json::from_str(
        &reopened
            .session_speech_script_preview(&draft.draft_id)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["ready"], true);
    assert_eq!(
        reopened.project.import_speech_cue(&bridge_wav()).unwrap(),
        *content
    );
    fs::remove_dir_all(root).unwrap();
}
