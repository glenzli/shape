//! Durable validation for accepted audio values and speech synthesis receipts.

use shape_domain::{
    ArtifactContentContract, AudioOriginDisclosure, SpeechVoiceSelection, TransformationKind,
    TransformationOperation,
};
use shape_execution::AUDIO_IMPORT_CAPABILITY;
use shape_execution::parse_pcm_s16le_wav;

use super::NewArtifactCommit;
use crate::StoreError;

pub(super) fn validate_audio_import_commit(commit: &NewArtifactCommit) -> Result<(), StoreError> {
    let Some(ArtifactContentContract::AudioClip(contract)) = commit.content_contract.as_ref()
    else {
        return Err(StoreError::InvalidCommit(
            "audio import requires an audio clip contract",
        ));
    };
    if commit.transformation.operation.is_some()
        || !commit.transformation.inputs.is_empty()
        || !commit.expected_input_heads.is_empty()
        || contract.origin != AudioOriginDisclosure::ImportedUnverified
        || commit.output_media_type != "audio/wav"
        || commit.receipt.capability.as_str() != AUDIO_IMPORT_CAPABILITY
        || commit.receipt.executor.id != "shape.builtin.audio-import"
        || commit.receipt.executor_job_id.is_some()
        || commit.receipt.external_provenance.is_some()
    {
        return Err(StoreError::InvalidCommit(
            "audio import must preserve an unverified external origin",
        ));
    }
    Ok(())
}

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
    if !provenance.speech_segments.is_empty()
        && (provenance.speech_script.as_ref().map_or_else(
            || {
                provenance
                    .speech_segments
                    .iter()
                    .try_fold(0_u64, |sum, part| sum.checked_add(part.frames))
            },
            shape_execution::speech_script::SpeechScriptAssembly::frames,
        ) != Some(contract.frame_count)
            || provenance.speech_segments.first().is_none_or(|part| {
                Some(part.job_id.as_str()) != commit.receipt.executor_job_id.as_deref()
            }))
    {
        return Err(StoreError::InvalidCommit(
            "speech segment receipts do not match the assembled audio",
        ));
    }
    if operation.script.is_some() != provenance.speech_script.is_some() {
        return Err(StoreError::InvalidCommit(
            "script operation and assembly receipt disagree",
        ));
    }
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

impl super::ProjectStore {
    /// Stores an immutable script cue without accepting an audio artifact.
    /// # Errors
    /// Rejects unsupported cues or content publication failures.
    pub fn import_speech_cue(&self, bytes: &[u8]) -> Result<shape_domain::ContentRef, StoreError> {
        shape_execution::speech_script::validate_cue(bytes).map_err(|_| {
            StoreError::InvalidCommit("script cues require a PCM16 mono 24 kHz WAV up to 8 MiB")
        })?;
        self.objects.publish(bytes, "audio/wav")
    }

    pub(super) fn validate_script_commit(
        &self,
        commit: &NewArtifactCommit,
    ) -> Result<(), StoreError> {
        let Some(TransformationOperation::AudioSpeechSynthesis(operation)) =
            &commit.transformation.operation
        else {
            return Ok(());
        };
        let Some(options) = &operation.script else {
            return Ok(());
        };
        let invalid =
            || StoreError::InvalidCommit("script source or timeline does not match accepted audio");
        let revision = self.revision(*commit.transformation.inputs.first().ok_or_else(invalid)?)?;
        let source = self.read_content(&revision.content)?;
        let source = std::str::from_utf8(&source).map_err(|_| invalid())?;
        let mut inputs = Vec::new();
        for action in options.cues.values() {
            if let shape_domain::speech_script::SpeechCueAction::Audio { content } = action {
                inputs.push(
                    shape_execution::ExecutionInput::materialized(
                        content.clone(),
                        self.read_content(content)?,
                    )
                    .map_err(|_| invalid())?,
                );
            }
        }
        shape_execution::speech_script::validate_output(
            source,
            operation,
            &inputs,
            commit
                .receipt
                .external_provenance
                .as_ref()
                .ok_or_else(invalid)?,
            &commit.output_bytes,
        )
        .map_err(|_| invalid())
    }
}
