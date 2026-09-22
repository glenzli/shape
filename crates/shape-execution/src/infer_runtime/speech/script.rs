//! Runs script speech actions with bounded retry; assembles local cues and silence.
use super::{
    AUDIO_MEDIA_TYPE, ArtifactContentContract, AudioOriginDisclosure, CachedNarration,
    ContentDigest, ExecutionFailure, ExecutionOutput, InferRuntimeSpeechExecutor,
    SpeechSegmentProvenance, SpeechSynthesisOperation, SpeechWaveAssembly, failure,
    parse_pcm_s16le_wav,
};
use crate::{
    audio::decode_pcm_s16le_wav,
    speech_script::{
        self, SCRIPT_SAMPLE_RATE, ScriptActionKind, SpeechScriptAssembly, SpeechScriptPiece,
    },
};
use shape_domain::speech_script::parse_speech_script;

impl InferRuntimeSpeechExecutor {
    // Cache, receipt and PCM advancement form one ordered execution transaction.
    #[allow(clippy::too_many_lines)]
    pub(super) fn create_script(
        &self,
        text: &str,
        operation: &SpeechSynthesisOperation,
        inputs: &[crate::ExecutionInput],
    ) -> Result<ExecutionOutput, ExecutionFailure> {
        let actions = speech_script::compile(text, operation, inputs)?;
        let total = actions
            .iter()
            .filter(|a| matches!(a.kind, ScriptActionKind::Speech { .. }))
            .count();
        let mut key = serde_json::to_vec(operation).map_err(|_| speech_script::invalid_script())?;
        key.extend_from_slice(text.as_bytes());
        let key = ContentDigest::from_bytes(&key);
        let mut cache_guard = self.control.cache()?;
        if cache_guard.as_ref().is_none_or(|cache| cache.key != key) {
            *cache_guard = Some(CachedNarration {
                key,
                segments: Vec::new(),
            });
        }
        let cache = cache_guard.as_mut().expect("script cache initialized");
        self.control.progress(cache.segments.len(), total);
        let mut wave = SpeechWaveAssembly::default();
        let mut receipts = Vec::new();
        let plan = parse_speech_script(text);
        let mut assembly = SpeechScriptAssembly {
            source_digest: plan.source_digest,
            operation_digest: ContentDigest::from_bytes(
                &serde_json::to_vec(operation).map_err(|_| speech_script::invalid_script())?,
            ),
            plan_digest: plan.digest(),
            pieces: Vec::new(),
        };
        let mut input_start = 0_u32;
        let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
        for action in actions {
            self.control.check_cancelled()?;
            if let ScriptActionKind::Replay { piece } = action.kind {
                let original = assembly
                    .pieces
                    .get(piece as usize)
                    .ok_or_else(speech_script::invalid_script)?
                    .clone();
                let start = wave.pcm_len();
                wave.replay_pcm(
                    ranges
                        .get(piece as usize)
                        .ok_or_else(speech_script::invalid_script)?
                        .clone(),
                )?;
                ranges.push(start..wave.pcm_len());
                assembly.pieces.push(SpeechScriptPiece {
                    event: action.event,
                    replay_of: Some(piece),
                    ..original
                });
                continue;
            }
            let (pcm, speech_segment) = match action.kind {
                ScriptActionKind::Replay { .. } => unreachable!("replay is assembled above"),
                ScriptActionKind::Local { pcm } => (pcm, None),
                ScriptActionKind::Speech { text, operation } => {
                    let index = receipts.len();
                    let segment = if let Some(cached) = cache.segments.get(index) {
                        cached.clone()
                    } else {
                        self.create_segment(&text, &operation)?
                    };
                    let (contract, pcm) = decode_pcm_s16le_wav(
                        &segment.bytes,
                        AudioOriginDisclosure::SyntheticSpeech,
                    )?;
                    if contract.sample_rate_hz != SCRIPT_SAMPLE_RATE || contract.channels != 1 {
                        return Err(failure("speech_format_changed", false));
                    }
                    if index == cache.segments.len() {
                        cache.segments.push(segment.clone());
                    }
                    let input_end = input_start
                        + u32::try_from(text.len()).map_err(|_| speech_script::invalid_script())?;
                    receipts.push(SpeechSegmentProvenance {
                        job_id: segment.job_id.clone(),
                        input_start,
                        input_end,
                        input_digest: ContentDigest::from_bytes(text.as_bytes()),
                        output_digest: ContentDigest::from_bytes(&segment.bytes),
                        frames: contract.frame_count,
                        runtime: Box::new(segment.provenance.clone()),
                    });
                    input_start = input_end;
                    self.control.progress(cache.segments.len(), total);
                    (
                        pcm.to_vec(),
                        Some(u32::try_from(index).map_err(|_| speech_script::invalid_script())?),
                    )
                }
            };
            let start = wave.pcm_len();
            wave.push_pcm(SCRIPT_SAMPLE_RATE, 1, &pcm)?;
            ranges.push(start..wave.pcm_len());
            assembly.pieces.push(SpeechScriptPiece {
                event: action.event,
                frames: (pcm.len() / 2) as u64,
                pcm_digest: ContentDigest::from_bytes(&pcm),
                speech_segment,
                replay_of: None,
            });
        }
        self.control.check_cancelled()?;
        let first = receipts.first().ok_or_else(speech_script::invalid_script)?;
        let job_id = first.job_id.clone();
        let mut provenance = *first.runtime.clone();
        provenance.speech_segments = receipts;
        provenance.speech_script = Some(assembly);
        let bytes = wave.finish()?;
        speech_script::validate_output(text, operation, inputs, &provenance, &bytes)?;
        let contract = parse_pcm_s16le_wav(&bytes, AudioOriginDisclosure::SyntheticSpeech)?;
        *cache_guard = None;
        Ok(ExecutionOutput {
            bytes,
            media_type: AUDIO_MEDIA_TYPE.into(),
            executor_job_id: Some(job_id),
            external_provenance: Some(provenance),
            content_contract: Some(ArtifactContentContract::AudioClip(contract)),
        })
    }
}
