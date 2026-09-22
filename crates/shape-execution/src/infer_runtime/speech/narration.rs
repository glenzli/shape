//! Sentence-aware batching and transient retry state for one narration request.

use crate::{ExecutionFailure, ExternalExecutionProvenance};
use shape_domain::ContentDigest;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub(super) const MAX_SEGMENT_CHARACTERS: usize = 240;

#[derive(Debug, Clone)]
pub(super) struct Segment {
    pub bytes: Arc<[u8]>,
    pub job_id: String,
    pub provenance: ExternalExecutionProvenance,
}

#[derive(Debug)]
pub(super) struct CachedNarration {
    pub key: ContentDigest,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Default)]
struct Progress {
    cancelled: AtomicBool,
    completed: AtomicUsize,
    total: AtomicUsize,
    cache: Mutex<Option<CachedNarration>>,
}

/// Shared transient progress and retry cache. No partial audio enters a project.
#[derive(Debug, Clone, Default)]
pub struct SpeechSynthesisControl {
    inner: Arc<Progress>,
}

impl SpeechSynthesisControl {
    /// Starts or resumes after the previous call has returned.
    pub fn resume(&self) {
        self.inner.cancelled.store(false, Ordering::Relaxed);
        self.progress(0, 0);
    }
    /// Stops at the next segment boundary, preserving validated segments for retry.
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Relaxed);
    }
    #[must_use]
    pub fn completed(&self) -> usize {
        self.inner.completed.load(Ordering::Relaxed)
    }
    #[must_use]
    pub fn total(&self) -> usize {
        self.inner.total.load(Ordering::Relaxed)
    }
    pub(super) fn check_cancelled(&self) -> Result<(), ExecutionFailure> {
        if self.inner.cancelled.load(Ordering::Relaxed) {
            Err(super::failure("speech_cancelled", false))
        } else {
            Ok(())
        }
    }
    pub(super) fn progress(&self, completed: usize, total: usize) {
        self.inner.total.store(total, Ordering::Relaxed);
        self.inner.completed.store(completed, Ordering::Relaxed);
    }
    pub(super) fn cache(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<CachedNarration>>, ExecutionFailure> {
        self.inner
            .cache
            .try_lock()
            .map_err(|_| super::failure("generation_busy", false))
    }
}

/// Preserves every UTF-8 byte, prefers punctuation/paragraph/word boundaries,
/// and bounds unpunctuated input without splitting a code point.
pub(crate) fn split_text(text: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let tail = &text[start..];
        let mut end = tail.len();
        let mut sentence_end = None;
        let mut word_end = None;
        // Leading blank lines belong to the following spoken segment. They
        // must not become a standalone request, even when longer than a batch.
        let leading = tail.len() - tail.trim_start().len();
        for (count, (relative, ch)) in tail[leading..].char_indices().enumerate() {
            let offset = leading + relative;
            if count == MAX_SEGMENT_CHARACTERS {
                end = sentence_end.or(word_end).unwrap_or(offset);
                break;
            }
            let after = offset + ch.len_utf8();
            if matches!(ch, '。' | '！' | '？' | '!' | '?' | ';' | '；' | '\n') || ch == '.' {
                sentence_end = Some(after);
            }
            if ch.is_whitespace() {
                word_end = Some(after);
            }
        }
        // Attach whitespace-only tails to the preceding request; never submit
        // a blank request or silently lose accepted source bytes.
        if tail[..end].trim().is_empty() && !segments.is_empty() {
            let previous = segments.pop().expect("previous segment");
            start -= str::len(previous);
            segments.push(&text[start..start + previous.len() + end]);
            start += previous.len() + end;
        } else {
            segments.push(&text[start..start + end]);
            start += end;
        }
    }
    segments
}

#[cfg(test)]
mod tests;
