use super::*;
use shape_domain::{PresetVoiceAlias, PresetVoiceSelection, speech_script::SpeechScriptOptions};
fn operation() -> SpeechSynthesisOperation {
    let mut op = SpeechSynthesisOperation::new(
        "auto",
        SpeechVoiceSelection::Preset(
            PresetVoiceSelection::new(
                PresetVoiceAlias::new(crate::SPEECH_PRESETS[0].alias).unwrap(),
                crate::INFER_SPEECH_VOICE_CATALOG_REVISION,
            )
            .unwrap(),
        ),
        1150,
        true,
    )
    .unwrap();
    op.script = Some(SpeechScriptOptions::default());
    op
}
#[test]
fn compile_exact_pauses_language_inheritance_and_cue_preflight() {
    let source = "[role: English]\n[cue: intro]\n# title\n[audio: intro]\n[speaker: English]\n[language: English]\nHello world.\n[pause: 1.25s]\n[language: auto]\n你好，world.";
    let mut op = operation();
    assert!(compile(source, &op, &[]).is_err());
    op.script
        .as_mut()
        .unwrap()
        .cues
        .insert("intro".into(), SpeechCueAction::Chime);
    let SpeechVoiceSelection::Preset(voice) = op.voice.clone() else {
        panic!()
    };
    op.script
        .as_mut()
        .unwrap()
        .roles
        .insert("English".into(), voice);
    let actions = compile(source, &op, &[]).unwrap();
    assert_eq!(actions.len(), 4);
    assert!(
        matches!(&actions[0].kind, ScriptActionKind::Local { pcm } if pcm.len() == 48_000 && pcm.iter().any(|s| *s != 0))
    );
    assert!(
        matches!(&actions[2].kind, ScriptActionKind::Local { pcm } if pcm.len() == 60_000 && pcm.iter().all(|s| *s == 0))
    );
    assert!(
        matches!(&actions[1].kind, ScriptActionKind::Speech { operation, .. } if operation.language == "English" && operation.speed_milli == 1150)
    );
    assert!(
        matches!(&actions[3].kind, ScriptActionKind::Speech { operation, .. } if operation.language == "auto")
    );
    assert!(
        compile(
            &format!("Hello.\n{}", "[pause: 120s]\n".repeat(24)),
            &op,
            &[]
        )
        .is_err()
    );
}

#[test]
fn expanded_repeat_size_is_rejected_before_execution() {
    let source =
        "[repeat: 8; gap: 0s]\nHello.\n[pause: 120s]\n[pause: 120s]\n[pause: 120s]\n[end-repeat]";
    assert!(compile(source, &operation(), &[]).is_err());
}
