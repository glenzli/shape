//! Persisted, text-only sound source configuration.
use shape_domain::{
    OperatorConfigurationSchemaId, SoundGenerationKind, SoundGenerationOperation,
    WorkingOperatorConfiguration, WorkingOperatorDraft,
};
pub(crate) const SOUND_GENERATE_OPERATOR: &str = "audio.generate";
const SCHEMA: &str = "shape.operator-draft.sound-generation@20260926.1";
pub(crate) fn configuration(
    prompt: &str,
    kind: &str,
    seconds: u8,
    seed: u32,
) -> Result<WorkingOperatorConfiguration, String> {
    let kind = match kind {
        "sound_effect" => SoundGenerationKind::SoundEffect,
        "short_music" => SoundGenerationKind::ShortMusic,
        _ => return Err("invalid_sound_kind".into()),
    };
    let mut p = SoundGenerationOperation::new(
        if prompt.is_empty() {
            "Unconfigured sound"
        } else {
            prompt
        },
        kind,
        seconds,
        seed,
    )
    .map_err(|_| "invalid_sound_request")?;
    p.prompt = prompt.into();
    WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(SCHEMA).map_err(|e| e.to_string())?,
        serde_json::to_string(&p).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
pub(crate) fn decode(
    config: Option<&WorkingOperatorConfiguration>,
) -> Result<SoundGenerationOperation, String> {
    let config = config
        .filter(|c| c.schema().as_str() == SCHEMA)
        .ok_or("invalid_sound_draft")?;
    let p: SoundGenerationOperation =
        serde_json::from_str(config.json()).map_err(|_| "invalid_sound_draft")?;
    let mut validation = p.clone();
    if validation.prompt.is_empty() {
        validation.prompt = "Unconfigured sound".into();
    }
    validation.validate().map_err(|_| "invalid_sound_draft")?;
    Ok(p)
}
pub(crate) fn parameters(draft: &WorkingOperatorDraft) -> Result<SoundGenerationOperation, String> {
    if draft.operator_type().as_str() != SOUND_GENERATE_OPERATOR
        || draft.input_data_type().is_some()
        || draft.output_data_type().as_str() != "audio.clip"
    {
        return Err("invalid_sound_draft".into());
    }
    let p = decode(draft.configuration())?;
    p.validate().map_err(|_| "invalid_sound_request")?;
    Ok(p)
}
#[cfg(test)]
mod tests;
