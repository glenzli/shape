use super::*;
use crate::create_desktop_project;
use shape_domain::{PresetVoiceAlias, PresetVoiceSelection};

#[test]
fn preview_projects_the_canonical_prerequisites_after_each_explicit_binding() {
    let root = std::env::temp_dir().join(format!("shape-prerequisites-{}", uuid::Uuid::now_v7()));
    let mut session = create_desktop_project(root.to_str().unwrap(), "Synthetic dialogue").unwrap();
    let source = "[role: Guest]\n[cue: turn]\n[speaker: Guest]\n你好。\n[audio: turn]";
    let snapshot = session
        .session_create_text_document("Original", source)
        .unwrap();
    let draft = session
        .session_begin_authoring_speech(&snapshot.artifacts[0].id)
        .unwrap();
    let mut options = SpeechScriptOptions::default();
    let plan = shape_domain::speech_script::parse_speech_script(source);
    for step in 0..3 {
        match step {
            1 => {
                options.roles.insert(
                    "Guest".into(),
                    PresetVoiceSelection::new(
                        PresetVoiceAlias::new(
                            shape_execution::INFER_SPEECH_VOICE_ZH_BRIGHT_FEMALE_V1,
                        )
                        .unwrap(),
                        shape_execution::INFER_SPEECH_VOICE_CATALOG_REVISION,
                    )
                    .unwrap(),
                );
            }
            2 => {
                options.cues.insert("turn".into(), SpeechCueAction::Skip);
            }
            _ => {}
        }
        session
            .session_update_speech_script(
                &draft.draft_id,
                &serde_json::to_string(&options).unwrap(),
            )
            .unwrap();
        let preview: serde_json::Value = serde_json::from_str(
            &session
                .session_speech_script_preview(&draft.draft_id)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            preview["prerequisites"],
            serde_json::to_value(plan.readiness(&options)).unwrap()
        );
        assert_eq!(preview["ready"], step == 2);
    }
    assert_eq!(session.session_candidates().len(), 0);
    assert_eq!(
        session.session_text_node_input(&draft.draft_id).unwrap(),
        source
    );
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
