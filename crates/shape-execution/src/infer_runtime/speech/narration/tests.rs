use super::*;

#[test]
fn segmentation_preserves_exact_multilingual_input_and_bounds_requests() {
    for text in [
        "一句中文。Another sentence! 👋\n".repeat(40),
        "连续无标点文字".repeat(400),
        "word ".repeat(900),
    ] {
        let parts = split_text(&text);
        assert!(parts.len() > 1);
        assert_eq!(parts.concat(), text);
        assert!(
            parts
                .iter()
                .all(|part| !part.trim().is_empty()
                    && part.chars().count() <= MAX_SEGMENT_CHARACTERS)
        );
    }
}

#[test]
fn short_text_and_terminal_whitespace_do_not_create_blank_requests() {
    assert_eq!(split_text("一句话。\n"), vec!["一句话。\n"]);
    let text = format!("{}{}", "旁白。".repeat(80), "\n".repeat(500));
    let parts = split_text(&text);
    assert_eq!(parts.concat(), text);
    assert!(parts.iter().all(|part| !part.trim().is_empty()));
    let text = format!("{}{}", "\n".repeat(500), "旁白。Hello. ".repeat(60));
    let parts = split_text(&text);
    assert_eq!(parts.concat(), text);
    assert!(parts.iter().all(|part| !part.trim().is_empty()));
}

#[test]
fn control_is_shared_and_cancellation_is_explicitly_resumable() {
    let control = SpeechSynthesisControl::default();
    let worker = control.clone();
    worker.progress(2, 7);
    assert_eq!((control.completed(), control.total()), (2, 7));
    control.cancel();
    assert_eq!(
        worker.check_cancelled().unwrap_err().code,
        "speech_cancelled"
    );
    control.resume();
    worker.check_cancelled().unwrap();
}
