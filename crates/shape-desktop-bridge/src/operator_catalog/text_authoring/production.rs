//! Production presets compile into actual document controls; AI output is checked against them.
use super::{TextAuthoring, WritingProfile};
use shape_domain::speech_script::{
    SpeechDelivery, SpeechProduction, SpeechScriptEventKind as Kind, SpeechScriptIssue,
    SpeechScriptPlan,
};
use std::fmt::Write;
impl TextAuthoring {
    fn role_names(&self) -> Result<Vec<&str>, String> {
        let names: Vec<_> = self
            .cast
            .split([',', '，'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let unique: std::collections::BTreeSet<_> = names.iter().copied().collect();
        if names.len() > 32
            || unique.len() != names.len()
            || names
                .iter()
                .any(|s| s.len() > 128 || s.contains(['[', ']', ';', '；', ':', '：', '\n']))
        {
            return Err("invalid_script_cast".into());
        }
        Ok(names)
    }
    pub(super) fn validate_production(&self) -> Result<(), String> {
        self.role_names()?;
        if self.gap_seconds > 120
            || (self.profile == WritingProfile::Listening && self.delivery != SpeechDelivery::Clear)
        {
            return Err("invalid_production_settings".into());
        }
        Ok(())
    }
    pub(super) fn production_instruction(&self) -> Result<String, String> {
        let profile = match self.profile {
            WritingProfile::Listening => "listening",
            WritingProfile::Dialogue => "dialogue",
            _ => "narration",
        };
        let delivery = serde_json::to_value(self.delivery)
            .map_err(|e| e.to_string())?
            .as_str()
            .unwrap()
            .to_owned();
        let roles = self.role_names()?;
        let declarations = roles
            .iter()
            .map(|name| format!("[role: {name}; language: auto]"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut prompt = format!(
            "Required production header (before all spoken lines and playback controls):\n[production: {profile}]\n[delivery: {delivery}]\n{declarations}\nUse exactly these declared roles when supplied. Assign by actual speaker, not by paragraph or language. Do not invent extra roles. Each selected role must speak at least once. Language changes do not create a new person. Narration defaults to one person; dialogue uses stable distinct people. Overall delivery describes speech performance, separate from writing tone.\n"
        );
        if self.profile == WritingProfile::Listening {
            write!(prompt, "Each question MUST be a [scene: question-id] containing exactly one [repeat: {}; gap: {}s] ... [end-repeat] block. The repeat count means total plays; Shape renders once and replays identical audio. Never duplicate the spoken question in source. Put the spoken question number before the repeat block, then place every question body or complete dialogue inside it. After the block, {}insert [pause: {}s] for answering, including after the final question. Introductions may be outside question scenes. Example question sentence appears ONCE:\n[scene: question-1]\n[speaker: {}]\nNumber one.\n[repeat: {}; gap: {}s]\n[speaker: {}]\nExample question sentence.\n[end-repeat]\n{}[pause: {}s]\n",
                self.repeat_count, self.gap_seconds,
                if self.answer_beep { "play [audio: answer], then " } else { "" }, self.pause_seconds,
                roles.first().copied().unwrap_or("Narrator"), self.repeat_count, self.gap_seconds,
                roles.last().copied().unwrap_or("Reader"), if self.answer_beep { "[audio: answer]\n" } else { "" }, self.pause_seconds).expect("writing to string");
            if self.answer_beep {
                prompt.push_str("Also add [cue: answer; sound: beep; duration: 0.2s; level: soft] in the header. Use it after each repeat block and nowhere else.\n");
            } else {
                prompt.push_str("No answer beep is requested; do not add sound cues.\n");
            }
            prompt.push_str("Keep delivery clear and even for all roles. Do not emphasize answers or add local emotion changes. Do not place extra pauses at repeat boundaries; gap and answering time are separate controls.\n");
        }
        Ok(prompt)
    }
    // Check one complete production contract, including relationships between adjacent controls.
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_production(&self, plan: &mut SpeechScriptPlan) {
        if !plan.issues.is_empty() {
            return;
        }
        let mut issues = Vec::new();
        let mut issue = |line, code: &str| {
            issues.push(SpeechScriptIssue {
                line,
                code: code.into(),
                text: String::new(),
            });
        };
        let production = match self.profile {
            WritingProfile::Listening => SpeechProduction::Listening,
            WritingProfile::Dialogue => SpeechProduction::Dialogue,
            _ => SpeechProduction::Narration,
        };
        if plan.production != production || plan.delivery != self.delivery {
            issue(1, "production_mismatch");
        }
        if let Ok(roles) = self.role_names() {
            if !roles.is_empty()
                && (roles.len() != plan.roles.len()
                    || roles.iter().any(|r| !plan.roles.contains_key(*r)))
            {
                issue(1, "cast_mismatch");
            }
            for role in roles {
                if !plan
                    .events
                    .iter()
                    .any(|e| matches!(&e.kind, Kind::Speech { role: spoken, .. } if spoken == role))
                {
                    issue(1, "unused_role");
                }
            }
        }
        if production == SpeechProduction::Listening {
            let mut repeats = 0;
            let mut scene_repeats: Option<(u32, usize)> = None;
            for (index, event) in plan.events.iter().enumerate() {
                match event.kind {
                    Kind::Scene { .. } => {
                        if let Some((line, n)) = scene_repeats
                            && n != 1
                        {
                            issue(line, "question_repeat_count");
                        }
                        scene_repeats = Some((event.line, 0));
                    }
                    Kind::RepeatStart { count, gap_ms } => {
                        repeats += 1;
                        if let Some((_, n)) = &mut scene_repeats {
                            *n += 1;
                        } else {
                            issue(event.line, "question_scene_required");
                        }
                        if count != self.repeat_count
                            || gap_ms != u32::from(self.gap_seconds) * 1000
                        {
                            issue(event.line, "repeat_settings_mismatch");
                        }
                    }
                    Kind::RepeatEnd => {
                        let mut following = plan.events[index + 1..].iter().filter(|e| {
                            !matches!(e.kind, Kind::Note { .. } | Kind::Heading { .. })
                        });
                        if self.answer_beep
                            && !following.next().is_some_and(
                                |e| matches!(&e.kind,Kind::Cue { label } if label == "answer"),
                            )
                        {
                            issue(event.line, "answer_cue_missing");
                        }
                        if !following.next().is_some_and(|e| matches!(e.kind,Kind::Pause { milliseconds } if milliseconds == u32::from(self.pause_seconds)*1000)) { issue(event.line,"answer_pause_mismatch"); }
                    }
                    _ => {}
                }
            }
            if repeats == 0 {
                issue(1, "question_repeat_count");
            }
            if let Some((line, n)) = scene_repeats
                && n != 1
            {
                issue(line, "question_repeat_count");
            }
            if self.answer_beep {
                use shape_domain::speech_script::{CueLevel, SpeechCueAction};
                let definition = Some(SpeechCueAction::Beep {
                    milliseconds: 200,
                    level: CueLevel::Soft,
                });
                let plays = plan
                    .events
                    .iter()
                    .filter(|e| matches!(&e.kind, Kind::Cue { label } if label == "answer"))
                    .count();
                if plan.cues.len() != 1
                    || plan.cues.get("answer") != Some(&definition)
                    || plays != repeats
                {
                    issue(1, "answer_cue_definition_mismatch");
                }
            }
            if !self.answer_beep && !plan.cues.is_empty() {
                issue(1, "unexpected_cue");
            }
        }
        plan.issues.extend(issues.into_iter().take(128));
    }
}
