//! The answer stage: WRITE (model) and CHECK (code), at most three passes (spec
//! `ai-conversational-harness`, sections 5.5 and 5.6; ADR-0028 decisions 5 and 6; slice S7).
//!
//! One stage with typed input ([`Input`]) and typed output ([`Answered`]), the same for every
//! question that is not a refusal, a navigation or a change:
//!
//! 1. The writer prompt is built from the dossier, the memory, the focus and the message
//!    (`prompt.rs`), stable parts first.
//! 2. The model writes. The reply is washed to plain sentences (`text::tidy`).
//! 3. Code checks it (`check.rs`). If pass 1 passes every rule it is shown at once. Otherwise the model
//!    gets one short message that names the exact offenders and writes again, three passes in all,
//!    inside the 60 second budget the request already has.
//! 4. After three failed passes the person still gets a true answer, written by code
//!    (`fallback.rs`): a lookup, a count or a comparison shows the first answering fact as one plain
//!    sentence; any other type shows the abstention sentence and the facts that were fetched. Never
//!    a canned overview. A pass that fails only on length (too long or too thin) is not a failure of
//!    the answer: it is shown, cut to the limit when it is too long.
//!
//! The model never decides whether data exists (code does, in the dossier) and no record id, path or
//! contact detail can reach the person unless the facts held it.

mod check;
mod fallback;
mod prompt;
#[cfg(test)]
mod tests;

use std::time::Instant;

use straft_ai::InferenceProvider;
use straft_ai::client::{ChatMessage, ChatRequest, SLOT_ANSWER};
use straft_ai_checks as checks;
use straft_domain::AppResult;

use super::dossier::{Dossier, Fact};
use super::index::{EntityIndex, Focus, Resolved};
use super::memory::TurnMemory;
use super::turn::{AnswerType, Group, TurnRead};
use super::{queries, text};
use check::{Expect, Material, Offence};

/// The time a request may spend in rewrite passes (ADR-0028 decision 6).
pub(super) const VERIFY_BUDGET: std::time::Duration = std::time::Duration::from_secs(60);
/// Passes in all, the first included (ADR-0028 decision 5).
pub(super) const MAX_PASSES: usize = 3;
/// Sampling of the writer: moderate (spike S1b).
const TEMPERATURE: f32 = 0.3;
/// The most sources one answer lists behind the sources button.
const MAX_SOURCES: usize = 8;

/// The language of a reply the check can tell and the writer is asked for.
pub(crate) fn language_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "hi" => "Hindi",
        "te" => "Telugu",
        "ta" => "Tamil",
        "kn" => "Kannada",
        "ml" => "Malayalam",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        _ => return None,
    })
}

/// The language of a message when it is one the check knows and is not English.
pub(super) fn language_of(message: &str) -> Option<&'static str> {
    checks::detect_language(message).filter(|l| *l != "en" && language_name(l).is_some())
}

/// The input of the stage.
pub(super) struct Input<'a> {
    pub model: &'a dyn InferenceProvider,
    pub message: &'a str,
    pub read: &'a TurnRead,
    /// The facts. `None` for chat, which is written with no facts at all.
    pub dossier: Option<&'a Dossier>,
    pub resolved: Option<&'a Resolved>,
    pub memory: &'a TurnMemory,
    /// The index, for the focus line and the project map.
    pub index: Option<&'a EntityIndex>,
    pub focus: &'a Focus,
    pub started: Instant,
    /// True once the person pressed Stop.
    pub stopped: &'a dyn Fn() -> bool,
    /// `progress(step, total, label)`.
    pub progress: &'a dyn Fn(u32, u32, &str),
}

/// What decided the text that is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shown {
    /// The model's answer passed every rule (on pass `n`).
    Written,
    /// The model's answer failed only on length; it is shown, cut when too long.
    WrittenSoft,
    /// Code wrote one sentence from the first answering fact.
    LastResort,
    /// Code wrote the abstention sentence (and the facts fetched).
    Abstained,
    /// The person pressed Stop.
    Stopped,
}

impl Shown {
    /// The trace value for the fallback used (empty when the model's answer was shown).
    pub(super) fn fallback_code(self) -> &'static str {
        match self {
            Shown::LastResort => "last_resort",
            Shown::Abstained => "abstained",
            _ => "",
        }
    }
}

/// What one pass found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PassLog {
    /// The rules that failed, by code.
    pub rules: Vec<&'static str>,
    /// The exact offenders, `rule: item, item` (kept in the request record, never in the trace).
    pub offenders: Vec<String>,
    pub words: usize,
}

impl PassLog {
    fn of(offences: &[Offence], words: usize) -> Self {
        Self {
            rules: offences.iter().map(|o| o.rule.code()).collect(),
            offenders: offences
                .iter()
                .map(|o| {
                    if o.items.is_empty() {
                        o.rule.code().to_string()
                    } else {
                        format!("{}: {}", o.rule.code(), o.items.join(", "))
                    }
                })
                .collect(),
            words,
        }
    }
}

/// The output of the stage.
#[derive(Debug, Clone)]
pub(super) struct Answered {
    pub text: String,
    pub shown: Shown,
    pub passes: Vec<PassLog>,
    /// The facts the shown answer rests on, for the sources button.
    pub sources: Vec<Fact>,
}

impl Answered {
    /// Passes the model made.
    pub(super) fn pass_count(&self) -> u32 {
        self.passes.len().max(1) as u32
    }

    /// The rules that failed on any pass, in order, without repeats.
    pub(super) fn failed_rules(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for p in &self.passes {
            for r in &p.rules {
                if !out.iter().any(|x| x == r) {
                    out.push((*r).to_string());
                }
            }
        }
        out
    }
}

fn stopped(passes: Vec<PassLog>) -> Answered {
    Answered {
        text: super::orchestrator::STOPPED.to_string(),
        shown: Shown::Stopped,
        passes,
        sources: Vec::new(),
    }
}

/// Whether a picked section holds the cast: only then may the focus line say who plays a character.
fn cast_picked(i: &Input<'_>) -> bool {
    i.dossier.is_some_and(|d| {
        d.leaves.iter().any(|u| {
            super::sections::leaf(u.leaf)
                .is_some_and(|l| l.name_kinds.contains(&super::index::Kind::Person))
        })
    })
}

/// A scene-by-scene briefing that did not reach every scene says so in a line code writes: the
/// model stopped at its token limit, the text was cut to the word limit, or the facts were cut
/// before it wrote. The writer never states how far it got.
fn coverage(text: String, d: &Dossier, briefing: bool) -> String {
    if !briefing {
        return text;
    }
    let covered = text
        .lines()
        .filter(|l| {
            let l = l.trim_start().to_lowercase();
            l.strip_prefix("scene ")
                .is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        })
        .count();
    let total = d.facts.iter().filter(|f| f.kind == "Scene").count();
    if covered == 0 || (covered >= total && !d.trimmed) {
        return text;
    }
    let what = if covered == 1 {
        "scene".to_string()
    } else {
        format!("{covered} scenes")
    };
    format!("{}\nThis covers the first {what}.", text.trim_end())
}

/// The memory the writer and the checker may use under the sections picked now. An earlier answer
/// stays only when every section it was asked under is still picked (spec 3, 1): otherwise its text
/// could carry facts of a section the person has since left out. The question is kept; the answer is
/// dropped, and so is the right to confirm it. With no picks (an internal call) nothing is dropped.
fn restricted(m: &TurnMemory, picked: &[super::dossier::LeafUse]) -> TurnMemory {
    let mut out = m.clone();
    if picked.is_empty() {
        return out;
    }
    for t in &mut out.conversation.recent {
        if !t
            .leaves
            .iter()
            .all(|l| picked.iter().any(|p| p.leaf == l.as_str()))
        {
            t.answered.clear();
            t.confirmable = false;
        }
    }
    out
}

/// The text that tells the checker what the answer may rest on, besides the facts.
fn context_text(i: &Input<'_>, memory: &TurnMemory) -> String {
    let mut c = String::new();
    for e in &memory.entries {
        c.push_str(&e.text);
        c.push('\n');
    }
    c.push_str(&memory.conversation.summary_text());
    c.push('\n');
    for t in &memory.conversation.recent {
        c.push_str(&t.asked);
        c.push('\n');
        c.push_str(&t.answered);
        c.push('\n');
    }
    c.push_str(&prompt::focus_text(i.focus, i.index, cast_picked(i)));
    c.push('\n');
    if let Some(ix) = i.index {
        c.push_str(&prompt::map_titles(ix));
    }
    c
}

/// Write the answer and check it.
///
/// A failure of the first model call is the caller's to report (the model is not ready, or it
/// failed); a failure of a later call ends the passes and the best attempt so far is used.
pub(super) fn run(i: &Input<'_>) -> AppResult<Answered> {
    let group = i.read.group;
    let class = i.read.length_class;
    let chat = group == Group::Chat;
    let no_dossier = Dossier::default();
    let dossier = if chat {
        &no_dossier
    } else {
        i.dossier.unwrap_or(&no_dossier)
    };

    let mut facts_text = dossier.render_plain();
    for m in &dossier.missing {
        facts_text.push_str(m);
        facts_text.push('\n');
    }
    let memory = restricted(i.memory, &dossier.leaves);
    let context = context_text(i, &memory);
    let material = Material {
        facts: &facts_text,
        request: i.message,
        context: &context,
    };
    // The narrative types are plain paragraphs; a briefing that goes scene by scene is the one list.
    let narrative = matches!(
        i.read.answer_type,
        AnswerType::Describe
            | AnswerType::Brief
            | AnswerType::StoryExplanation
            | AnswerType::Relation
    );
    let briefing =
        i.read.answer_type == AnswerType::Brief && super::turn::scene_by_scene(i.message);
    let expect = Expect {
        class,
        required: &dossier.required,
        missing: &dossier.missing,
        language: language_of(i.message),
        no_list: narrative && !briefing,
        narrative,
    };

    let map = i
        .dossier
        .map(|d| prompt::project_map(&d.leaves))
        .unwrap_or_default();
    let focus = prompt::focus_text(i.focus, i.index, cast_picked(i));
    let base = prompt::build(&prompt::Parts {
        group,
        class,
        map: &map,
        memory: &memory,
        focus: &focus,
        dossier: (!chat).then_some(dossier),
        resolved: i.resolved,
        message: i.message,
        language: expect.language,
        briefing,
    });

    let say = if briefing {
        format!("{} {}", prompt::BRIEFING_SAY, prompt::length_say(class))
    } else {
        prompt::length_say(class).to_string()
    };
    let mut messages: Vec<ChatMessage> = base;
    let mut logs: Vec<PassLog> = Vec::new();
    let mut attempts: Vec<(String, Vec<Offence>)> = Vec::new();
    for pass in 1..=MAX_PASSES {
        if (i.stopped)() {
            return Ok(stopped(logs));
        }
        (i.progress)(
            2,
            3,
            if pass == 1 {
                "Writing the answer"
            } else {
                "Writing the answer again"
            },
        );
        let mut req = ChatRequest::new(messages.clone())
            .max_tokens(prompt::max_tokens(class, expect.language))
            .temperature(TEMPERATURE)
            .slot(SLOT_ANSWER);
        req.top_p = None;
        let raw = match i.model.chat(&req) {
            Ok(r) => r,
            Err(e) if pass == 1 => return Err(e),
            Err(_) => break,
        };
        // Stop was pressed while the model wrote: the finished text is not shown.
        if (i.stopped)() {
            return Ok(stopped(logs));
        }
        (i.progress)(3, 3, "Checking the answer");
        let written = text::tidy(&raw, i.message);
        let offences = check::check(&written, &material, &expect);
        logs.push(PassLog::of(&offences, text::word_count(&written)));
        if offences.is_empty() {
            return Ok(Answered {
                sources: sources_of(dossier, &written),
                text: coverage(written, dossier, briefing),
                shown: Shown::Written,
                passes: logs,
            });
        }
        let ask = check::feedback(&offences, &say);
        attempts.push((written.clone(), offences));
        if pass == MAX_PASSES || i.started.elapsed() >= VERIFY_BUDGET {
            break;
        }
        messages.push(ChatMessage::assistant(queries::truncate_chars(
            &written, 1_200,
        )));
        messages.push(ChatMessage::user(ask));
    }
    if (i.stopped)() {
        return Ok(stopped(logs));
    }

    // An answer that failed only on length says nothing wrong: show it (cut when it is too long).
    if let Some((best, offences)) = attempts
        .iter()
        .rev()
        .find(|(t, o)| !t.trim().is_empty() && o.iter().all(|x| x.rule.soft()))
    {
        let shown = if offences.iter().any(|o| o.rule == check::Rule::Length) {
            check::cut_to(best, class)
        } else {
            best.clone()
        };
        return Ok(Answered {
            sources: sources_of(dossier, &shown),
            text: coverage(shown, dossier, briefing),
            shown: Shown::WrittenSoft,
            passes: logs,
        });
    }

    // Three failed passes: code writes a true answer.
    let (text, shown, used) = match group {
        Group::Chat => (fallback::chat(), Shown::Abstained, Vec::new()),
        _ if matches!(
            i.read.answer_type,
            AnswerType::Lookup | AnswerType::Count | AnswerType::Compare
        ) =>
        {
            match fallback::last_resort(dossier) {
                Some((t, fact)) => (t, Shown::LastResort, vec![fact]),
                None => {
                    let (t, used) = fallback::abstention(dossier, true);
                    (t, Shown::Abstained, used)
                }
            }
        }
        _ => {
            let (t, used) = fallback::abstention(dossier, false);
            (t, Shown::Abstained, used)
        }
    };
    Ok(Answered {
        text,
        shown,
        passes: logs,
        sources: used,
    })
}

/// The facts the answer used: a fact is used when the answer holds a value it states (a required
/// value, or a short field value such as a name, an amount or a date). At most [`MAX_SOURCES`].
pub(super) fn sources_of(d: &Dossier, answer: &str) -> Vec<Fact> {
    let seen = check::norm(answer);
    let mut out: Vec<Fact> = Vec::new();
    for f in &d.facts {
        if f.record_id.is_empty() || f.is_missing() {
            continue;
        }
        let hit = f.requires.iter().any(|r| mentions(&seen, r))
            || f.fields.iter().any(|(_, v)| {
                if v.chars().count() <= 60 {
                    mentions(&seen, v)
                } else {
                    retells(&seen, v)
                }
            })
            || f.sentence.as_deref().is_some_and(|s| mentions(&seen, s));
        if hit && !out.iter().any(|x| x.record_id == f.record_id) {
            out.push(f.clone());
        }
        if out.len() >= MAX_SOURCES {
            break;
        }
    }
    out
}

/// A long text (a passage, a scene's text) is used when the answer retells it: at least three of
/// its words of five letters or more are in the answer. A passage is never quoted whole.
fn retells(seen_norm: &str, text: &str) -> bool {
    let words = |s: &str| -> Vec<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() >= 5)
            .map(str::to_string)
            .collect()
    };
    let seen: std::collections::HashSet<String> = words(seen_norm).into_iter().collect();
    let mut shared: Vec<String> = words(&check::norm(text))
        .into_iter()
        .filter(|w| seen.contains(w))
        .collect();
    shared.sort_unstable();
    shared.dedup();
    shared.len() >= 3
}

/// The value is in the answer. A value shorter than three characters that holds no digit ("Sc",
/// "of") is too common to count.
fn mentions(seen_norm: &str, value: &str) -> bool {
    let v = check::norm(value);
    let v = v.trim();
    if v.chars().count() < 3 && !v.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    !v.is_empty() && seen_norm.contains(v)
}

/// For the evaluation: whether the checks for an unsupported relationship word and an unstated
/// feeling (off in the app, `check::STRICT_NARRATIVE_CHECKS`) would flag `answer`, given the texts
/// the answer may rest on. The harness runs it over the expected answers of the section-scoped
/// cases to measure the false positives before the checks are turned on (spec 3.7).
pub fn narrative_flags(answer: &str, evidence: &[&str]) -> (bool, bool) {
    (
        !check::unsupported_relations(answer, evidence).is_empty(),
        !check::unstated_moods(answer, evidence).is_empty(),
    )
}
