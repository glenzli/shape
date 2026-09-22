use super::*;

#[test]
fn empty_drafts_are_valid_but_generation_requires_authored_intent() {
    let state = TextAuthoring::new("listening").unwrap();
    assert!(state.configuration().is_ok());
    assert!(
        !serde_json::to_string(&state)
            .unwrap()
            .contains("repeat_count")
    );
    assert!(state.compiled_instruction().is_err());
    assert!(TextAuthoring::new("unknown").is_err());
}

#[test]
fn templates_compile_the_canonical_grammar_and_optional_material() {
    let mut state = TextAuthoring::new("listening").unwrap();
    state.instruction = "五年级，周末活动。".into();
    state.material = "tai chi, park, Sunday".into();
    let prompt = state.compiled_instruction().unwrap();
    assert!(prompt.contains("[role: Narrator"));
    assert!(prompt.contains("[repeat: 2; gap: 2s]"));
    assert!(prompt.contains("[pause: 5s]"));
    assert!(prompt.contains("tai chi, park, Sunday"));
    assert!(!prompt.contains("每题恰好一个重复块"));
    assert!(!prompt.contains("[pause: 8s]"));
    assert!(!prompt.contains("【在这里填写"));
    state.entry = WritingEntry::Adapt;
    state.material.clear();
    assert!(state.compiled_instruction().is_err());
}

#[test]
fn format_errors_are_visible_and_block_script_acceptance_only() {
    let script = TextAuthoring::new("narration").unwrap();
    assert!(validate_output(&script, "[repeat: 2]\nHello.").is_err());
    assert!(
        validate_output(
            &script,
            "[role: Narrator]\n# Title\n[角色：Narrator]\nHello.\n[停顿：2秒]"
        )
        .is_ok()
    );
    assert!(validate_output(&script, "# Just a heading").is_err());
    assert!(validate_output(&TextAuthoring::new("plain").unwrap(), "[repeat: 2]").is_ok());
    let projection: serde_json::Value =
        serde_json::from_str(&preview("narration", "Hello.\n[pause: nope]").unwrap()).unwrap();
    assert_eq!(projection["valid"], false);
    assert_eq!(projection["plan"]["issues"][0]["line"], 2);
}

#[test]
fn script_source_controls_ai_output_without_duplicate_form_constraints() {
    let state = TextAuthoring::new("listening").unwrap();
    let source = "[production: listening]\n[role: Narrator]\n[role: Reader]\n[cue: answer; sound: beep]\n[scene: q1]\n[speaker: Narrator]\nNumber one.\n[repeat: 2; gap: 2s]\n[speaker: Reader]\nHello.\n[end-repeat]\n[audio: answer]\n[pause: 5s]";
    assert!(validate_output(&state, source).is_ok());
    for valid in [
        source.replace("repeat: 2", "repeat: 3"),
        source.replace("gap: 2s", "gap: 3s"),
        source.replace("pause: 5s", "pause: 3s"),
        source.replace("[audio: answer]\n", ""),
        source.replace("sound: beep", "sound: chime"),
        source.replace("[scene: q1]\n", ""),
    ] {
        assert!(validate_output(&state, &valid).is_ok(), "{valid}");
    }
    for (bad, code) in [
        (source.replace("[end-repeat]", ""), "unclosed_repeat"),
        (
            source.replace("[audio: answer]", "[audio: missing]"),
            "undeclared_cue",
        ),
    ] {
        let preview: serde_json::Value = serde_json::from_str(
            &configured_preview(&serde_json::to_string(&state).unwrap(), &bad).unwrap(),
        )
        .unwrap();
        assert_eq!(preview["valid"], false, "{bad}");
        assert!(
            preview["plan"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["code"] == code && e["line"].as_u64().unwrap() > 0)
        );
        assert!(validate_output(&state, &bad).is_err());
    }
    let old_draft = serde_json::json!({"profile":"listening","entry":"generate","instruction":"write","material":"","text":""});
    assert!(TextAuthoring::from_json(&old_draft.to_string()).is_ok());
}

#[test]
fn repair_feedback_is_separate_from_the_original_user_request() {
    let mut state = TextAuthoring::new("listening").unwrap();
    state.instruction = "中文必须说：请听录音。".into();
    state.repair_feedback = "line 12: answer_pause_mismatch".into();
    state.material = "A draft needing a pause.".into();
    state.entry = WritingEntry::Adapt;
    let prompt = state.compiled_instruction().unwrap();
    assert!(prompt.contains(&state.instruction));
    assert!(prompt.contains(&state.repair_feedback));
    assert_eq!(
        TextAuthoring::from_json(&serde_json::to_string(&state).unwrap()).unwrap(),
        state
    );
    state.repair_feedback = "x".repeat(4097);
    assert!(state.configuration().is_err());
}
