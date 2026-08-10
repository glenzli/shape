use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{
    ArtifactContentContract, ArtifactKind, AudioOriginDisclosure, AudioValueContract, IntentSpec,
    PresetVoiceAlias, PresetVoiceSelection, SpeechSynthesisOperation, SpeechVoiceSelection,
};
use shape_execution::{
    AUDIO_SPEECH_SYNTHESIZE_CAPABILITY, CapabilityId, ExecutionFailure, ExecutionOutput,
    ExecutionRequest, Executor, ExecutorIdentity, ExternalAttemptProvenance,
    ExternalExecutionProvenance, ExternalRoutingCandidate, INFER_RUNTIME_CONTRACT_VERSION,
    INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION, INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE,
    INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
};
use uuid::Uuid;

use super::{create_desktop_project, open_desktop_session};
use crate::infer_speech::InferSpeechCandidate;
use crate::operator_catalog::AUDIO_SPEECH_OPERATOR;

#[derive(Debug)]
struct BridgeSpeechExecutor {
    identity: ExecutorIdentity,
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
        contract_revision: INFER_RUNTIME_CONTRACT_VERSION.to_owned(),
        app_id: "shape".to_owned(),
        intent: "speech.synthesize".to_owned(),
        provider: "mlx-audio-local".to_owned(),
        deployment: "mlx_qwen3_tts_custom_voice_1_7b".to_owned(),
        model_profile: "qwen3_tts_custom_voice_1_7b".to_owned(),
        model_build: "qwen3_tts_custom_voice_1_7b_8bit".to_owned(),
        physical_model: "qwen3-tts-custom-voice".to_owned(),
        placement: "local".to_owned(),
        quality_grade: "general".to_owned(),
        rating_status: "provisional".to_owned(),
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
        quality_floor: "general".to_owned(),
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
    assert_eq!(draft.context_artifact_id, artifact.id);
    assert_eq!(draft.operator_type_key, "text.edit");
    assert_eq!(session.session_operator_drafts().len(), 1);
    let repeated = session
        .session_begin_operator_draft(&artifact.id, "text.edit")
        .expect("draft reuses");
    assert_eq!(repeated.draft_id, draft.draft_id);
    assert_eq!(
        session
            .session_operator_descriptors(&artifact.id)
            .expect("catalog projects")
            .len(),
        3
    );

    drop(session);
    let mut session = open_desktop_session(path).expect("project reopens with Working Graph");
    let restored = session.session_operator_drafts();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].draft_id, draft.draft_id);
    assert_eq!(restored[0].operator_type_key, "text.edit");

    session
        .session_propose_text(&artifact.id, "A revised first line.")
        .expect("candidate executes");
    assert!(session.session_operator_drafts().is_empty());
    assert_eq!(session.session_candidates().len(), 1);
    drop(session);

    let reopened = open_desktop_session(path).expect("project reopens");
    assert!(reopened.session_operator_drafts().is_empty());
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
        .expect("transform draft begins");
    let updated = session
        .session_update_text_transform_draft(
            &draft.draft_id,
            "polish",
            "  Make it warmer, but preserve the title.  ",
        )
        .expect("instruction saves");
    assert_eq!(updated.draft_id, draft.draft_id);
    assert_eq!(updated.text_transform_mode, "polish");
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
        .session_update_text_transform_draft(&draft.draft_id, "rewrite", "")
        .expect("instruction clears");
    assert_eq!(cleared.text_transform_mode, "rewrite");
    assert!(cleared.text_transform_instruction.is_empty());
    assert!(cleared.configuration_schema.is_empty());
    fs::remove_dir_all(root).expect("fixture removes");
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
        INFER_SPEECH_VOICE_ALIAS_CATALOG_REVISION
    );
    assert_eq!(
        draft.audio_speech_language,
        INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_LANGUAGE
    );
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
            .is_err()
    );
    assert_eq!(
        reopened.session_operator_drafts()[0].audio_speech_speed_milli,
        1_150,
        "a failed durable save restores the complete in-memory draft"
    );
    fs::remove_dir_all(root).expect("fixture removes");
}

#[test]
fn candidate_is_transient_until_acceptance_and_survives_reopen_after_commit() {
    let root = test_root();
    let artifact_id = seeded_project(&root);
    let path = root.to_str().expect("portable path");
    let mut session = open_desktop_session(path).expect("session opens");
    let before = session.session_snapshot().expect("snapshot reads");
    let before_artifact = before.artifacts.first().expect("artifact projected");
    let before_revision = before_artifact.accepted_revision_id.clone();
    session
        .session_begin_operator_draft(&artifact_id.to_string(), "audio.speech_synthesize")
        .expect("parallel speech draft begins");

    let candidate = session
        .session_propose_text(&artifact_id.to_string(), "A quiet summer afternoon.")
        .expect("candidate executes");
    assert_eq!(candidate.artifact_id, artifact_id.to_string());
    assert_eq!(candidate.text_preview, "A quiet summer afternoon.");
    assert!(!candidate.text_preview_truncated);
    assert_eq!(session.session_operator_drafts().len(), 1);

    let still_accepted = session
        .session_snapshot()
        .expect("snapshot remains readable");
    assert_eq!(
        still_accepted.artifacts[0].accepted_revision_id,
        before_revision
    );
    assert_eq!(
        still_accepted.artifacts[0].text_preview,
        "A summer afternoon."
    );

    let accepted = session
        .session_accept_candidate(&candidate.candidate_id)
        .expect("candidate accepts");
    assert!(accepted.graph_edges.is_empty());
    assert_ne!(accepted.artifacts[0].accepted_revision_id, before_revision);
    assert_eq!(
        accepted.artifacts[0].accepted_parent_revision_ids,
        vec![before_revision.clone()]
    );
    assert_eq!(
        accepted.artifacts[0].transformation_kind_key,
        "text_rewrite"
    );
    assert_eq!(
        accepted.artifacts[0].transformation_input_revision_ids,
        vec![before_revision]
    );
    assert_eq!(
        accepted.artifacts[0].text_preview,
        "A quiet summer afternoon."
    );
    assert_eq!(accepted.artifacts[0].operator_graph_nodes.len(), 3);
    assert_eq!(accepted.artifacts[0].operator_graph_edges.len(), 2);
    assert!(session.session_operator_drafts().is_empty());
    assert_eq!(
        accepted.artifacts[0].operator_graph_nodes[1].operator_type_key,
        "text.edit"
    );
    drop(session);

    let reopened = ShapeProject::open(&root).expect("project reopens");
    assert!(
        reopened
            .artifact_working_graphs()
            .expect("Working Graphs load")
            .is_empty()
    );
    let content = reopened
        .read_accepted(artifact_id)
        .expect("accepted head reads")
        .expect("accepted content exists");
    assert_eq!(content.bytes, b"A quiet summer afternoon.");
    fs::remove_dir_all(root).expect("test project removes");
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

    let candidate = session
        .session_propose_raster_crop(&artifact.id, 1, 1, 4, 3)
        .unwrap();
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
