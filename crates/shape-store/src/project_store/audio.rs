//! Durable validation for accepted audio values and speech synthesis receipts.

use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, SpeechVoiceSelection, TransformationKind,
    TransformationOperation,
};
use shape_execution::parse_pcm_s16le_wav;

use super::NewArtifactCommit;
use crate::StoreError;

pub(super) fn validate_speech_synthesis_commit(
    commit: &NewArtifactCommit,
) -> Result<(), StoreError> {
    let Some(TransformationOperation::AudioSpeechSynthesis(operation)) =
        commit.transformation.operation.as_ref()
    else {
        return Err(StoreError::InvalidCommit(
            "audio clips currently require an audio speech synthesis operation",
        ));
    };
    operation.validate()?;
    let Some(ArtifactContentContract::AudioClip(contract)) = commit.content_contract.as_ref()
    else {
        return Err(StoreError::InvalidCommit(
            "speech synthesis requires an audio clip output contract",
        ));
    };
    let Some(provenance) = commit.receipt.external_provenance.as_ref() else {
        return Err(StoreError::InvalidCommit(
            "speech synthesis requires external execution provenance",
        ));
    };
    if commit.transformation.kind != TransformationKind::GenerativeEdit
        || commit.transformation.inputs.len() != 1
        || commit.expected_input_heads.len() != 1
        || commit.expected_input_heads[0].1 != commit.transformation.inputs[0]
        || !matches!(operation.voice, SpeechVoiceSelection::Preset(_))
        || !operation.synthetic_disclosure_required
        || contract.origin != AudioOriginDisclosure::SyntheticSpeech
        || commit.output_media_type != "audio/wav"
        || commit.receipt.capability.as_str() != "audio.speech_synthesize"
        || commit.receipt.executor_job_id.is_none()
        || !provenance.is_bounded()
        || provenance.app_id != "shape"
        || provenance.intent != "speech.synthesize"
        || provenance.placement != "local"
        || provenance.policy != "local-first"
        || provenance.requested_policy != "local-first"
        || provenance.requested_priority != "interactive"
        || provenance.requested_placement != "local_only"
        || provenance.requested_preference != "local"
        || !provenance.offline_required
        || provenance.fallback != "none"
        || provenance.max_cost_microusd != 0
        || provenance
            .attempts
            .last()
            .is_none_or(|attempt| attempt.outcome != "succeeded")
        || provenance
            .attempts
            .iter()
            .any(|attempt| attempt.trigger == "fallback")
    {
        return Err(StoreError::InvalidCommit(
            "speech synthesis output or provenance violates the accepted contract",
        ));
    }
    Ok(())
}

pub(super) fn validate_audio_content(
    media_type: &str,
    contract: Option<&ArtifactContentContract>,
    output_bytes: &[u8],
) -> Result<(), StoreError> {
    let Some(ArtifactContentContract::AudioClip(contract)) = contract else {
        return Err(StoreError::InvalidCommit(
            "audio revisions require an audio clip contract and audio/wav bytes",
        ));
    };
    if media_type != "audio/wav" {
        return Err(StoreError::InvalidCommit(
            "audio revisions require an audio clip contract and audio/wav bytes",
        ));
    }
    let derived = parse_pcm_s16le_wav(output_bytes, contract.origin).map_err(|_| {
        StoreError::InvalidCommit("audio revisions require exact supported PCM S16 LE WAV bytes")
    })?;
    if &derived != contract {
        return Err(StoreError::InvalidCommit(
            "audio contract does not match exact accepted WAV bytes",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
