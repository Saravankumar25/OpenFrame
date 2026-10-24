//! Verifiers v2 (AI harness rebuild plan, section 5.9; ADR-0028 decisions 5 and 6).
//!
//! Only deterministic code judges an answer, never a model. After the model writes, these checks
//! run on the finished text: names, scene numbers, quotations, numbers, relationship words,
//! language, length, repetition, empty text and false change claims. An answer that passes every
//! check on the first pass is returned at once. Otherwise the model gets ONE short message that
//! names the exact offenders and writes again; at most three passes in all, inside the 60 second
//! budget the request already has. If every pass fails, the attempt with the fewest offenders is
//! shown with the offending sentences removed (when a sentence is left) and marked as not sure.

use std::collections::BTreeSet;
use std::time::Instant;

use openframe_ai::InferenceProvider;
use openframe_ai::client::{ChatMessage, ChatRequest, malformed};
use openframe_ai_checks as checks;
use openframe_domain::AppResult;

use super::orchestrator::{
    MAX_PASSES, Style, VERIFY_BUDGET, fit_words, keep_sentences, language_name, plain_text_with,
    without_false_change_claim,
};
use super::queries;

/// The checks, by the short code written to the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Check {
    Empty,
    Names,
    SceneNumbers,
    Quotes,
    Numbers,
    RelationshipWords,
    Language,
    Length,
    Repetition,
    ChangeClaim,
}

impl Check {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Check::Empty => "empty",
            Check::Names => "names",
            Check::SceneNumbers => "scene_numbers",
            Check::Quotes => "quotes",
            Check::Numbers => "numbers",
            Check::RelationshipWords => "relationship_words",
            Check::Language => "language",
            Check::Length => "length",
            Check::Repetition => "repetition",
            Check::ChangeClaim => "change_claim",
        }
    }
}

/// One failed check and the exact things that failed it.
#[derive(Debug, Clone)]
pub(crate) struct Offence {
    pub check: Check,
    pub items: Vec<String>,
}

/// How long an answer may be. A request that names a sentence count sets `max_sentences`.
#[derive(Debug, Clone)]
pub(crate) struct Limits {
    pub max_sentences: Option<usize>,
    pub max_paragraphs: Option<usize>,
    pub max_words: usize,
}

/// What the checks compare an answer with.
pub(crate) struct Facts {
    pub request: String,
    /// Everything the model saw as data: briefings, passages, notes.
    pub evidence: Vec<String>,
    /// The project's character and location names, as one text.
    pub names: String,
    /// Scene numbers the draft has.
    pub live_scenes: BTreeSet<u32>,
    /// The language code of a request that is not English, when it can be told.
    pub language: Option<&'static str>,
    pub limits: Limits,
    pub style: Style,
}

/// Words a written answer uses as labels and openings. They are not names of the project.
const LABEL_WORDS: &str = "Idea Ideas Suggestion Suggestions Note Notes Overall Summary Overview \
Conclusion Beginning Middle End Setup Payoff Theme Themes Characters Opening Ending Key Moments \
Act Acts Scene Scenes Draft Story Screenplay OpenFrame";

/// The language of the request, when it is one the check can tell and is not English.
pub(crate) fn expected_language(request: &str) -> Option<&'static str> {
    checks::detect_language(request).filter(|l| *l != "en" && language_name(l).is_some())
}

fn words(t: &str) -> usize {
    t.split_whitespace().count()
}

/// A short line with no end mark and no list marker: a heading of a deep answer.
fn is_heading(line: &str) -> bool {
    let l = line.trim();
    !l.is_empty()
        && words(l) <= 8
        && !l.ends_with(['.', '!', '?', ','])
        && !l.starts_with("- ")
        && !l.starts_with(|c: char| c.is_ascii_digit())
}

fn strip_headings(text: &str) -> String {
    text.lines()
        .filter(|l| !is_heading(l))
        .collect::<Vec<_>>()
        .join("\n")
}

/// "1. " and "2) " list numbers are not numbers the answer claims.
fn strip_list_numbers(text: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            let digits = t.chars().take_while(char::is_ascii_digit).count();
            match t[digits..].chars().next() {
                Some('.' | ')') if (1..=2).contains(&digits) => &t[digits + 1..],
                _ => l,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn check(answer: &str, f: &Facts) -> Vec<Offence> {
    if answer.trim().is_empty() {
        return vec![Offence {
            check: Check::Empty,
            items: Vec::new(),
        }];
    }
    let ev: Vec<&str> = f.evidence.iter().map(String::as_str).collect();
    let mut out: Vec<Offence> = Vec::new();
    let mut add = |check: Check, items: Vec<String>| {
        if !items.is_empty() {
            out.push(Offence { check, items });
        }
    };

    let mut name_ev = ev.clone();
    name_ev.extend([f.request.as_str(), f.names.as_str(), LABEL_WORDS]);
    let for_names = if f.style == Style::Rich {
        strip_headings(answer)
    } else {
        answer.to_string()
    };
    add(Check::Names, checks::ungrounded_names(&for_names, &name_ev));

    let in_evidence: BTreeSet<u32> = ev.iter().flat_map(|e| checks::scene_refs(e)).collect();
    let cited = checks::scene_refs(answer);
    add(
        Check::SceneNumbers,
        cited
            .iter()
            .filter(|n| !f.live_scenes.contains(n) || !in_evidence.contains(n))
            .map(|n| format!("Scene {n}"))
            .collect(),
    );

    add(Check::Quotes, checks::missing_quotes(answer, &ev));

    let mut num_ev = ev.clone();
    num_ev.push(f.request.as_str());
    add(
        Check::Numbers,
        checks::unsupported_numbers(&strip_list_numbers(answer), &num_ev)
            .into_iter()
            .filter(|n| n.parse::<u32>().map_or(true, |n| !cited.contains(&n)))
            .collect(),
    );

    add(
        Check::RelationshipWords,
        checks::unsupported_relationship_words(answer, &ev),
    );

    if let Some(want) = f.language
        && !checks::language_matches(answer, want)
    {
        add(Check::Language, vec![want.to_string()]);
    }

    if words(answer) > f.limits.max_words * 3 / 2 {
        add(Check::Length, vec![format!("{} words", words(answer))]);
    }
    add(
        Check::Repetition,
        checks::repeated_ngram(answer, 4).into_iter().collect(),
    );
    if without_false_change_claim(answer) != answer {
        add(Check::ChangeClaim, vec!["a change was claimed".into()]);
    }
    out
}

fn total(offences: &[Offence]) -> usize {
    offences.iter().map(|o| o.items.len().max(1)).sum()
}

/// The one short message the model gets after a failed pass. Machine-written: the exact offenders
/// and what to do about them.
pub(crate) fn feedback(offences: &[Offence], f: &Facts) -> String {
    let mut lines: Vec<String> = Vec::new();
    for o in offences {
        let list = o.items.join(", ");
        lines.push(match o.check {
            Check::Empty => "The answer was empty. Answer the request from the data.".into(),
            Check::Names => format!(
                "These names are not in the project: {list}. Remove them or say you could not find them."
            ),
            Check::SceneNumbers => format!(
                "These scenes are not in the data: {list}. Cite only scenes that appear in the data."
            ),
            Check::Quotes => format!(
                "These quotations are not in the data: {list}. Remove them or paraphrase."
            ),
            Check::Numbers => {
                format!("These numbers are not in the data: {list}. Remove them.")
            }
            Check::RelationshipWords => format!(
                "These relationships are not recorded in the data: {list}. Remove them; say no relationship is recorded."
            ),
            Check::Language => format!(
                "Write the whole answer in {}.",
                f.language.and_then(language_name).unwrap_or("the language of the request")
            ),
            Check::Length => "The answer is too long. Make it shorter.".into(),
            Check::Repetition => {
                format!("The answer repeats itself (\"{list}\"). Say each thing once.")
            }
            Check::ChangeClaim => {
                "Do not say anything was changed. Nothing has been changed.".into()
            }
        });
    }
    lines.push("Write the answer again.".into());
    lines.join("\n")
}

// ------------------------------------------------------------------ fitting

/// Cut the answer to the limits: paragraphs, then sentences, then words, at sentence ends.
pub(crate) fn fit(text: String, l: &Limits) -> String {
    let mut t = text;
    if let Some(p) = l.max_paragraphs {
        t = t.split("\n\n").take(p).collect::<Vec<_>>().join("\n\n");
    }
    if let Some(n) = l.max_sentences {
        t = keep_sentences(t, n);
    }
    if words(&t) > l.max_words {
        t = fit_words(t, l.max_words);
    }
    t
}

/// A reply cut off by the token limit ends mid-sentence. Cut the last line back to its last full
/// sentence, or drop it when it has none and an earlier line remains. A list item or heading stays.
pub(crate) fn drop_unfinished(text: String) -> String {
    let t = text.trim_end();
    let last_start = t.rfind('\n').map_or(0, |i| i + 1);
    let last = &t[last_start..];
    let ends = |s: &str| {
        s.ends_with([
            '.', '!', '?', ')', '"', '”', '’', ':', '\u{3002}', '\u{0964}',
        ])
    };
    let listy = last.trim_start().starts_with("- ")
        || last.trim_start().starts_with(|c: char| c.is_ascii_digit());
    if ends(last) || listy || words(last) < 6 {
        return t.to_string();
    }
    match last.rfind(['.', '!', '?']) {
        Some(i) => format!("{}{}", &t[..last_start], &last[..=i]),
        None if last_start > 0 => t[..last_start].trim_end().to_string(),
        None => t.to_string(),
    }
}

// ------------------------------------------------------------------ removing offenders

/// Sentences and list lines of a text; `true` marks the start of a new line. A blank line is an
/// empty unit with `true`.
fn units(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push((String::new(), true));
            continue;
        }
        let mut first = true;
        let mut cur = String::new();
        let chars: Vec<char> = line.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            cur.push(ch);
            let ends =
                matches!(ch, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|c| c.is_whitespace());
            let list_number = cur.trim().chars().all(|c| c.is_ascii_digit() || c == '.');
            if ends && !list_number {
                out.push((cur.trim().to_string(), first));
                first = false;
                cur.clear();
            }
        }
        if !cur.trim().is_empty() {
            out.push((cur.trim().to_string(), first));
        }
    }
    out
}

fn mentions(unit: &str, o: &Offence) -> bool {
    let lower = unit.to_lowercase();
    match o.check {
        Check::Names | Check::Quotes => o.items.iter().any(|i| lower.contains(&i.to_lowercase())),
        Check::SceneNumbers => {
            let refs = checks::scene_refs(unit);
            o.items.iter().any(|i| {
                i.trim_start_matches("Scene ")
                    .parse::<u32>()
                    .is_ok_and(|n| refs.contains(&n))
            })
        }
        Check::Numbers => o.items.iter().any(|n| {
            unit.split(|c: char| !c.is_ascii_digit())
                .any(|d| d == n.as_str())
        }),
        Check::RelationshipWords => o.items.iter().any(|w| {
            lower
                .split(|c: char| !c.is_alphabetic())
                .any(|x| x == w || x.strip_suffix('s') == Some(w.as_str()))
        }),
        _ => false,
    }
}

/// The text without the sentences that hold an offender, or None when nothing would be left.
pub(crate) fn without_offenders(text: &str, offences: &[Offence]) -> Option<String> {
    let mut out = String::new();
    let mut kept = 0;
    for (unit, newline) in units(text) {
        if unit.is_empty() {
            out.push('\n');
            continue;
        }
        if offences.iter().any(|o| mentions(&unit, o)) {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push(if newline { '\n' } else { ' ' });
        }
        out.push_str(&unit);
        kept += 1;
    }
    (kept > 0).then(|| out.trim().to_string())
}

// ------------------------------------------------------------------ the passes

/// Sampling of one generation.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Sampling {
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
}

impl Sampling {
    pub(crate) fn request(&self, messages: Vec<ChatMessage>) -> ChatRequest {
        let mut r = ChatRequest::new(messages)
            .max_tokens(self.max_tokens)
            .temperature(self.temperature);
        r.top_p = self.top_p;
        r.top_k = self.top_k;
        r
    }
}

/// What the passes are doing, for the progress line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Checking,
    Rewriting,
}

/// An answer that went through the passes.
#[derive(Debug)]
pub(crate) struct Checked {
    pub text: String,
    pub passes: u32,
    /// Checks that failed on any pass, by code, in order, without repeats.
    pub failed: Vec<String>,
    /// Every check passed on the last pass.
    pub clean: bool,
}

/// Write the answer and check it, up to three passes inside the verifier budget (measured from
/// `started`). A failure of the first model call is the caller's to report; a failure of a later
/// call ends the passes and the best attempt so far is used.
#[cfg(test)]
pub(crate) fn generate_checked(
    model: &dyn InferenceProvider,
    base: &[ChatMessage],
    sampling: &Sampling,
    f: &Facts,
    started: Instant,
    notify: &dyn Fn(Phase),
) -> AppResult<Checked> {
    generate_checked_until(model, base, sampling, f, started, notify, &|| false)
}

/// Like [`generate_checked`], and the passes end early when `stopped` says the person pressed Stop.
pub(crate) fn generate_checked_until(
    model: &dyn InferenceProvider,
    base: &[ChatMessage],
    sampling: &Sampling,
    f: &Facts,
    started: Instant,
    notify: &dyn Fn(Phase),
    stopped: &dyn Fn() -> bool,
) -> AppResult<Checked> {
    let mut attempts: Vec<(String, Vec<Offence>)> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut messages = base.to_vec();
    let mut passes = 0;
    loop {
        passes += 1;
        if passes > 1 {
            notify(Phase::Rewriting);
        }
        let raw = match model.chat(&sampling.request(messages.clone())) {
            Ok(r) => r,
            Err(e) if passes == 1 => return Err(e),
            Err(_) => {
                passes -= 1;
                break;
            }
        };
        notify(Phase::Checking);
        let text = fit(drop_unfinished(plain_text_with(&raw, f.style)), &f.limits);
        let offences = check(&text, f);
        if offences.is_empty() {
            return Ok(Checked {
                text,
                passes,
                failed,
                clean: true,
            });
        }
        for o in &offences {
            if !failed.iter().any(|c| c == o.check.code()) {
                failed.push(o.check.code().to_string());
            }
        }
        messages = base.to_vec();
        messages.push(ChatMessage::assistant(queries::truncate_chars(
            &text, 1_200,
        )));
        messages.push(ChatMessage::user(feedback(&offences, f)));
        attempts.push((text, offences));
        if passes >= MAX_PASSES || started.elapsed() >= VERIFY_BUDGET || stopped() {
            break;
        }
    }
    let (text, offences) = attempts
        .into_iter()
        .reduce(|best, next| {
            if total(&next.1) < total(&best.1) {
                next
            } else {
                best
            }
        })
        .expect("the first pass made an attempt");
    if text.trim().is_empty() {
        return Err(malformed("empty answer"));
    }
    let text = if offences.iter().any(|o| o.check == Check::ChangeClaim) {
        without_false_change_claim(&text)
    } else {
        without_offenders(&text, &offences).unwrap_or(text)
    };
    Ok(Checked {
        text,
        passes,
        failed,
        clean: false,
    })
}

#[cfg(test)]
mod tests;
