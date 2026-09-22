use super::*;
#[test]
fn exam_script_separates_speech_timing_roles_and_assets() {
    let source = "[role: 播音员]\n[role: 英语朗读]\n[cue: 片头]\n# 五年级英语听力\n[音效：片头]\n[角色：播音员]\n[语言：中文]\n听力测试现在开始。每小题读两遍。\n[停顿：10秒]\n## 第一大题\n[角色：英语朗读]\n[语言：英语]\n**Number one.**\nLook at the boy.\n[停顿：2秒]\nLook at the boy.\n[停顿：5秒]\n";
    let plan = parse_speech_script(source);
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert_eq!(
        plan.spoken_text(),
        "听力测试现在开始。每小题读两遍。Number one.Look at the boy.Look at the boy."
    );
    assert_eq!(
        plan.events
            .iter()
            .filter_map(
                |e| if let SpeechScriptEventKind::Pause { milliseconds } = e.kind {
                    Some(milliseconds)
                } else {
                    None
                }
            )
            .sum::<u32>(),
        17_000
    );
    assert!(!plan.ready(&SpeechScriptOptions::default()));
    let mut options = SpeechScriptOptions::default();
    options.cues.insert("片头".into(), SpeechCueAction::Skip);
    for role in ["播音员", "英语朗读"] {
        options.roles.insert(
            role.into(),
            crate::PresetVoiceSelection::new(
                crate::PresetVoiceAlias::new("speech.voice.zh.bright_female.v1").unwrap(),
                "test.1",
            )
            .unwrap(),
        );
    }
    assert!(plan.ready(&options));
    assert_eq!(plan.digest(), parse_speech_script(source).digest());
}
#[test]
fn malformed_commands_are_not_spoken_or_silently_accepted() {
    for command in [
        "[停顿：-1秒]",
        "[停顿：NaN]",
        "[停顿：121秒]",
        "[停顿：1.0001秒]",
        "[语言：未知]",
        "[repeat: 2]",
        "[停顿：3秒",
        "```code",
        "![image](x)",
    ] {
        let plan = parse_speech_script(&format!("Hello.\n{command}"));
        assert!(!plan.issues.is_empty(), "{command}");
        assert!(!plan.ready(&SpeechScriptOptions::default()));
        assert_eq!(plan.spoken_text(), "Hello.");
    }
}
#[test]
fn exact_decimal_pauses_and_plain_formatting_do_not_rewrite_words() {
    let p = parse_speech_script(
        "# ignored\n**Number one.** &#x20;\n[停顿：1.25秒]\nHello `world`.\n[备注：不朗读]\n",
    );
    assert_eq!(p.spoken_text(), "Number one.Hello world.");
    assert!(
        p.events
            .iter()
            .any(|e| matches!(e.kind, SpeechScriptEventKind::Pause { milliseconds: 1250 }))
    );
}

#[test]
fn production_declarations_scope_delivery_and_count_only_real_gaps() {
    let source = "[production: dialogue]\n[delivery: warm]\n[role: A; language: auto]\n[role: B; language: English; delivery: clear]\n[cue: turn; sound: beep; duration: 0.125s; level: soft]\n[scene: conversation]\n[speaker: A]\n你好。Hello.\n[repeat: 3; gap: 1.25s]\n[speaker: B]\n[performance: lively]\nHow are you?\n[pause: 0.5s]\n[audio: turn]\n[end-repeat]\nFine.";
    let plan = parse_speech_script(source);
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert_eq!(plan.pause_millis(), 4000);
    let spoken: Vec<_> = plan
        .events
        .iter()
        .filter_map(|e| {
            if let SpeechScriptEventKind::Speech {
                role,
                language,
                delivery,
                ..
            } = &e.kind
            {
                Some((role.as_str(), language.as_deref(), *delivery))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        spoken,
        vec![
            ("A", Some("auto"), SpeechDelivery::Warm),
            ("B", Some("English"), SpeechDelivery::Lively),
            ("A", Some("auto"), SpeechDelivery::Warm)
        ]
    );
    assert_eq!(plan.spoken_text().matches("How are you?").count(), 1);
    assert!(
        !plan.ready(&SpeechScriptOptions::default()),
        "named roles require explicit voice bindings"
    );
    assert!(matches!(
        plan.cue("turn", &SpeechScriptOptions::default()),
        Some(SpeechCueAction::Beep {
            milliseconds: 125,
            ..
        })
    ));
}

#[test]
fn production_rejects_ambiguous_controls_and_late_or_unbound_names() {
    for source in [
        "[speaker: missing]\nHello.",
        "[audio: missing]\nHello.",
        "[role: A]\nHello.",
        "Hello.\n[role: A]",
        "[role: A]\n[role: A]\n[speaker: A]\nHello.",
        "[repeat: 0; gap: 2s]\nHello.\n[end-repeat]",
        "[repeat: 9; gap: 2s]\nHello.\n[end-repeat]",
        "[repeat: 2; gap: 121s]\nHello.\n[end-repeat]",
        "[repeat: 2]\n[repeat: 2]\nHello.\n[end-repeat]\n[end-repeat]",
        "[repeat: 2]\n[scene: q1]\nHello.\n[end-repeat]",
        "[repeat: 2]\n[pause: 1s]\n[end-repeat]\nHello.",
        "Hello.\n[end-repeat]",
        "[production: listening]\n[performance: warm]\nHello.",
        "[delivery: warm]\n[production: listening]\nHello.",
        "[role: A; delivery: warm]\n[production: listening]\n[speaker: A]\nHello.",
        "[cue: tone; sound: beep; duration: 2.001s]\nHello.",
        "[cue: tone; sound: chime; duration: 1s]\nHello.",
    ] {
        let plan = parse_speech_script(source);
        assert!(!plan.issues.is_empty(), "must reject {source}");
        assert!(!plan.ready(&SpeechScriptOptions::default()));
    }
}
