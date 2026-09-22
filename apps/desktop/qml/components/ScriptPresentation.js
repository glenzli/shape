.pragma library
function delivery(value) {
    switch (value) {
    case "clear": return qsTr("Clear and even")
    case "warm": return qsTr("Warm and friendly")
    case "lively": return qsTr("Light and lively")
    default: return qsTr("Natural and steady")
    }
}
function issue(code) {
    switch (code) {
    case "invalid_pause": return qsTr("Use a pause longer than 0 and at most 120 seconds.")
    case "unknown_directive": return qsTr("Unknown instruction. Check Rules and examples.")
    case "unsupported_language": return qsTr("Use a supported language or auto.")
    case "unsupported_markup": return qsTr("Remove code fences, tables, images and HTML.")
    case "no_spoken_text": return qsTr("Add at least one spoken line.")
    case "script_too_large": case "too_many_events": return qsTr("Keep the script within 64 KiB and 2048 events.")
    case "undeclared_role": case "invalid_role": return qsTr("Declare each unique role before the spoken content. Check its name, language and delivery.")
    case "missing_speaker": return qsTr("Select a declared speaker before this spoken line.")
    case "undeclared_cue": case "invalid_cue": return qsTr("Declare the cue before the spoken content. Beep duration must be 0.02–2 seconds.")
    case "declaration_position": return qsTr("Place each production declaration once, before playback begins.")
    case "invalid_production": return qsTr("Use listening, narration or dialogue as the production type.")
    case "invalid_delivery": return qsTr("Choose neutral, clear, warm or lively delivery.")
    case "listening_delivery": return qsTr("Listening scripts require clear, even delivery without local emotion changes.")
    case "invalid_repeat": return qsTr("Use 1–8 total plays and a gap from 0 to 120 seconds.")
    case "nested_repeat": return qsTr("Repeat blocks cannot be nested.")
    case "unclosed_repeat": return qsTr("Close this block with [end-repeat].")
    case "unexpected_repeat_end": return qsTr("This repeat end has no matching start.")
    case "empty_repeat": return qsTr("Put spoken content inside the repeat block.")
    case "scene_inside_repeat": return qsTr("Start the scene outside the repeat block.")
    case "production_mismatch": return qsTr("The script must match the selected production type and overall delivery.")
    case "cast_mismatch": return qsTr("Use exactly the role names selected in Production settings.")
    case "unused_role": return qsTr("Every selected role must have a spoken line.")
    case "question_repeat_count": return qsTr("Each question scene must contain exactly one repeat block.")
    case "question_scene_required": return qsTr("Place each question's repeat block inside a scene.")
    case "repeat_settings_mismatch": return qsTr("Match the selected play count and gap between plays.")
    case "answer_cue_definition_mismatch": return qsTr("Use one soft 0.2-second answer beep after each question, matching Production settings.")
    case "answer_cue_missing": return qsTr("Play the answer cue immediately after the repeat block.")
    case "answer_pause_mismatch": return qsTr("Add the selected answering pause after the repeat block and optional cue.")
    case "unexpected_cue": return qsTr("Sound cues are disabled in the selected listening settings.")
    default: return qsTr("Put a complete [instruction: value] on its own line. See Rules and examples.")
    }
}
function eventText(event) {
    switch (event.kind) {
    case "speech": return event.text
    case "pause": return qsTr("Silence · %1 seconds").arg(event.milliseconds / 1000)
    case "cue": return qsTr("Sound cue · %1").arg(event.label)
    case "scene": return qsTr("Scene · %1").arg(event.label)
    case "repeat_start": {
        const repeat = qsTr("Play %1 times · identical audio · %2 s between plays").arg(event.count).arg(event.gap_ms / 1000)
        return event.between_cue ? repeat + " · " + qsTr("Sound cue · %1").arg(event.between_cue) : repeat
    }
    case "repeat_end": return qsTr("End repeat · continue once")
    default: return event.text || ""
    }
}
