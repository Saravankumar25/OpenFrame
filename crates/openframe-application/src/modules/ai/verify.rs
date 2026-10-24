//! Verifiers v2 (AI harness rebuild plan, section 5.9; ADR-0028 decisions 5 and 6).
//!
//! Only deterministic code judges an answer, never a model. After the model writes, these checks
//! run on the finished text (after a first sentence that only announces the answer is dropped):
//! names, scene numbers, quotations, numbers, counts of scenes, relationship words, mood the script
//! does not state, language, length, repetition, empty text and false change claims. An answer that passes every
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
    MAX_PASSES, Style, VERIFY_BUDGET, fit_words, keep_sentences, language_name, unwashed_text,
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
    SceneCounts,
    RelationshipWords,
    Atmosphere,
    Language,
    Length,
    TooShort,
    Repetition,
    ChangeClaim,
    /// The answer copies the notes the model was given instead of answering in its own words.
    EvidenceEcho,
}

impl Check {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Check::Empty => "empty",
            Check::Names => "names",
            Check::SceneNumbers => "scene_numbers",
            Check::Quotes => "quotes",
            Check::Numbers => "numbers",
            Check::SceneCounts => "scene_counts",
            Check::RelationshipWords => "relationship_words",
            Check::Atmosphere => "atmosphere_unsupported",
            Check::Language => "language",
            Check::Length => "length",
            Check::TooShort => "too_short",
            Check::Repetition => "repetition",
            Check::ChangeClaim => "change_claim",
            Check::EvidenceEcho => "evidence_echo",
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
    /// An answer with fewer words than this is asked for once more, longer (an explanation of one
    /// scene that stops after a sentence).
    pub min_words: Option<usize>,
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
    let numbers: Vec<String> = checks::unsupported_numbers(&strip_list_numbers(answer), &num_ev)
        .into_iter()
        .filter(|n| n.parse::<u32>().map_or(true, |n| !cited.contains(&n)))
        .collect();
    // A count of scenes is a different claim from a scene number: "Scene 16" exists, "16 scenes" is
    // true only when the draft says so. A number already named above is not named twice.
    let counts: Vec<String> =
        checks::unsupported_scene_counts(answer, &num_ev, f.live_scenes.len())
            .into_iter()
            .filter(|c| !numbers.iter().any(|n| c.starts_with(n.as_str())))
            .collect();
    add(Check::Numbers, numbers);
    add(Check::SceneCounts, counts);

    add(
        Check::RelationshipWords,
        checks::unsupported_relationship_words(answer, &ev),
    );
    add(
        Check::Atmosphere,
        checks::unsupported_atmosphere(answer, &num_ev),
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
            Check::SceneCounts => format!(
                "These scene counts are not in the data: {list}. The draft has {} scenes; use only counts the data states.",
                f.live_scenes.len()
            ),
            Check::RelationshipWords => format!(
                "These relationships are not recorded in the data: {list}. Remove them; say no relationship is recorded."
            ),
            Check::Atmosphere => format!(
                "The script does not state: {list}. Describe only what the script shows: who does what and says what, in order. Do not add light, mood or feelings."
            ),
            Check::Language => format!(
                "Write the whole answer in {}.",
                f.language.and_then(language_name).unwrap_or("the language of the request")
            ),
            Check::Length => "The answer is too long. Make it shorter.".into(),
            Check::TooShort => {
                "The answer is too short. Write two short paragraphs covering the scene in order."
                    .into()
            }
            Check::Repetition => {
                format!("The answer repeats itself (\"{list}\"). Say each thing once.")
            }
            Check::ChangeClaim => {
                "Do not say anything was changed. Nothing has been changed.".into()
            }
            Check::EvidenceEcho => "Answer in your own sentences; do not copy the notes.".into(),
        });
    }
    // The last line asks for the answer alone, so the model does not talk about the rewrite.
    lines.push("Reply with the corrected answer only.".into());
    lines.join("\n")
}

// ------------------------------------------------------------------ copying the notes

/// An answer this short, after the markup of the notes was washed out of it, only echoed labels.
const ECHO_MIN_WORDS: usize = 25;

/// The check `evidence_echo` (section 5.9): it fails only when the wash removed note markup or
/// labels from this very answer and under 25 words of prose are left. Restating the script's own
/// lines is not an offence: the scene evidence holds those lines, so a correct explanation repeats
/// them, and cutting such sentences left real answers a fraction of their length. The offence has
/// a word count and no sentences, so the last pass never drops sentences for it.
pub(crate) fn evidence_echo(answer: &str, washed: bool) -> Option<Offence> {
    (washed && words(answer) < ECHO_MIN_WORDS).then(|| Offence {
        check: Check::EvidenceEcho,
        items: vec![format!("{} words", words(answer))],
    })
}

/// The answer is the request handed back: the same words, or one question that repeats most of
/// them. It says nothing, so it counts as an empty answer.
fn echoes_request(answer: &str, request: &str) -> bool {
    let (a, r) = (plain_words(answer), plain_words(request));
    if a.is_empty() || r.is_empty() {
        return false;
    }
    if a == r {
        return true;
    }
    let hits = r.iter().filter(|w| a.contains(w)).count();
    answer.trim_end().ends_with('?') && units(answer).len() == 1 && hits * 2 > r.len()
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

/// Words of an opening that says nothing: the filler of "Okay, let's dive into this screenplay."
const FILLER: &[&str] = &[
    "okay",
    "ok",
    "sure",
    "alright",
    "all",
    "right",
    "certainly",
    "absolutely",
    "of",
    "course",
    "great",
    "question",
    "lets",
    "let",
    "us",
    "delve",
    "dive",
    "deep",
    "deeper",
    "break",
    "down",
    "look",
    "looking",
    "at",
    "into",
    "in",
    "on",
    "explore",
    "take",
    "a",
    "walk",
    "through",
    "the",
    "this",
    "that",
    "these",
    "those",
    "project",
    "screenplay",
    "script",
    "scene",
    "scenes",
    "draft",
    "story",
    "film",
    "filmmaker",
    "writer",
    "friend",
    "and",
    "to",
    "what",
    "whats",
    "happening",
    "happens",
    "happen",
    "going",
    "here",
    "heres",
    "is",
    "it",
    "its",
    "now",
    "first",
    "start",
    "begin",
    "by",
    "with",
    "some",
    "detail",
    "details",
    "closer",
    "closely",
    "together",
    "we",
    "you",
    "your",
    "my",
    "our",
    "i",
    "ill",
    "will",
    "can",
    "then",
    "so",
    "well",
    "for",
    "about",
];

/// Openings that announce an answer instead of giving it.
const OPENINGS: &[&str] = &[
    "okay",
    "ok",
    "sure",
    "alright",
    "all right",
    "certainly",
    "absolutely",
    "of course",
    "great question",
    "lets delve",
    "lets dive",
    "lets break down",
    "lets look",
    "lets explore",
    "lets take a look",
    "lets walk through",
    "let us delve",
    "let us dive",
    "let us break down",
    "let us look",
    "let us explore",
    "let us take a look",
    "let us walk through",
];

/// Lower-case words of a text, apostrophes dropped: "Let's" gives "lets".
fn plain_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace(['\'', '\u{2019}'], "")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// A first sentence that only announces the answer: it opens with okay, sure, alright, certainly,
/// of course or great question, or with "let's" and delve, dive, break down, look, explore, take a
/// look or walk through; and every word is filler or a word of the request. A sentence that holds
/// any other word (a name, an event, a number the request did not have) carries a fact and stays.
fn is_preamble(sentence: &str, request: &str) -> bool {
    let words = plain_words(sentence);
    let joined = words.join(" ");
    let opens = OPENINGS
        .iter()
        .any(|s| joined == *s || joined.starts_with(&format!("{s} ")));
    let asked = plain_words(request);
    opens
        && words
            .iter()
            .all(|w| FILLER.contains(&w.as_str()) || asked.contains(w))
}

/// The answer without a first sentence that is pure preamble. Never empties an answer.
pub(crate) fn without_preamble(text: &str, request: &str) -> String {
    let t = text.trim_start();
    let end = t.char_indices().find_map(|(i, c)| match c {
        '\n' => Some(i),
        '.' | '!' | '?' if t[i + 1..].chars().next().is_none_or(char::is_whitespace) => Some(i + 1),
        _ => None,
    });
    let Some(end) = end else {
        return text.to_string();
    };
    let rest = t[end..].trim_start();
    if rest.is_empty() || !is_preamble(&t[..end], request) {
        return text.to_string();
    }
    rest.to_string()
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
        Check::Names | Check::Quotes | Check::SceneCounts | Check::Atmosphere => {
            o.items.iter().any(|i| lower.contains(&i.to_lowercase()))
        }
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
    let mut asked_longer = false;
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
        let unwashed = unwashed_text(&raw, f.style);
        let washed = super::sanitize::clean(&unwashed);
        let was_washed = super::sanitize::copies_markup(&raw, &unwashed);
        let plain = without_preamble(&washed, &f.request);
        let text = fit(drop_unfinished(plain), &f.limits);
        let text = if echoes_request(&text, &f.request) {
            String::new()
        } else {
            text
        };
        let mut offences = check(&text, f);
        if let Some(echo) = evidence_echo(&text, was_washed) {
            // An answer that was only labels is an echo, not an empty answer.
            offences.retain(|o| o.check != Check::Empty);
            offences.push(echo);
        }
        // A thin answer is asked to grow once; the second answer is taken as it comes.
        if offences.is_empty() && !asked_longer && f.min_words.is_some_and(|m| words(&text) < m) {
            asked_longer = true;
            offences.push(Offence {
                check: Check::TooShort,
                items: vec![format!("{} words", words(&text))],
            });
        }
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
    // A short answer that failed nothing else is a good answer.
    if offences.iter().all(|o| o.check == Check::TooShort) {
        return Ok(Checked {
            text,
            passes,
            failed,
            clean: true,
        });
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
