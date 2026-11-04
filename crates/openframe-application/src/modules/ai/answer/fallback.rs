//! What is shown when the model's answer failed the checks three times (spec 4.1 and 5.6).
//!
//! Written by code from the facts, so it is true by construction:
//!
//! - any answer with values it had to state gets the **last-resort fact sentences**, one per fact that
//!   holds such a value: "Lead cast, planned: 40,00,000.";
//! - any other answer gets the **abstention sentence** and the first facts that were fetched;
//! - when the fetch found nothing, the abstention says so and names what is missing.
//!
//! Never a canned overview of the project. The trace counts how often each is used, so the rate can
//! be driven down.

use std::sync::LazyLock;

use regex::Regex;

use super::super::dossier::{Dossier, Fact};
use super::super::queries::truncate_chars;
use super::check::norm;

/// Internal codes the person never sees, in the words a person uses.
const CODE_WORDS: &[(&str, &str)] = &[
    ("actor_conflict", "actor clash"),
    ("cast_unavailable", "actor unavailable"),
    ("location_conflict", "location clash"),
    ("location_unavailable", "location unavailable"),
    ("missing_location", "missing location"),
    ("duration_overflow", "long day"),
    ("travel_overflow", "long travel"),
    ("call_sheet_stale", "out of date call sheet"),
    ("script_removed", "removed scene"),
];

static CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[a-z]+(?:_[a-z0-9]+)+\b").expect("valid pattern"));

/// No snake_case code reaches the person: a known code is its plain words, any other is spaced.
fn plain(s: &str) -> String {
    CODE.replace_all(s, |c: &regex::Captures<'_>| {
        let code = &c[0];
        CODE_WORDS
            .iter()
            .find(|(k, _)| *k == code)
            .map_or_else(|| code.replace('_', " "), |(_, w)| (*w).to_string())
    })
    .into_owned()
}

/// The facts a fallback may quote: real records, not notes about what is missing or assumed.
fn real(d: &Dossier) -> impl Iterator<Item = &Fact> {
    d.facts
        .iter()
        .filter(|f| !f.is_missing() && f.kind != "Assumed")
}

fn end_with_stop(mut s: String) -> String {
    if !s.ends_with(['.', '!', '?']) {
        s.push('.');
    }
    s
}

/// One plain sentence from a fact. A relationship is its own sentence, and a count reads "10 scenes
/// in LOST PROPERTY." Otherwise the first field names the thing (a bare number is named by its
/// field: "scene 2") and the fields that hold a required value follow (the first three when none
/// does): "Lead cast, planned: 40,00,000."
pub(super) fn fact_sentence(f: &Fact) -> String {
    plain(&raw_sentence(f))
}

fn raw_sentence(f: &Fact) -> String {
    if let Some(s) = &f.sentence {
        return end_with_stop(s.clone());
    }
    let wanted: Vec<String> = f.requires.iter().map(|r| norm(r)).collect();
    let holds = |v: &str| wanted.iter().any(|r| holds_value(v, r));
    // A count says its value, unless what must be stated is another of its fields ("different items").
    if f.kind == "Count"
        && let (Some(what), Some(value)) = (f.field("what"), f.field("value"))
        && (f.requires.is_empty() || holds(value))
    {
        return end_with_stop(format!("{value} {what}"));
    }
    let Some(((first, subject), rest)) = f.fields.split_first() else {
        return end_with_stop(f.kind.to_string());
    };
    let subject = if subject.chars().all(|c| c.is_ascii_digit()) {
        format!("{first} {subject}")
    } else {
        subject.clone()
    };
    let mut asked: Vec<&(String, String)> = rest.iter().filter(|(_, v)| holds(v)).collect();
    if asked.is_empty() {
        asked = rest.iter().take(3).collect();
    }
    if asked.is_empty() {
        return end_with_stop(subject);
    }
    // A long text is told by the sentences that hold what must be stated, not read out whole.
    let list: Vec<String> = asked
        .iter()
        .map(|(n, v)| {
            let held = sentences_holding(v, &wanted);
            if v.chars().count() > LONG_VALUE && !held.is_empty() {
                format!("{n}: {held}")
            } else {
                format!("{n}: {v}")
            }
        })
        .collect();
    end_with_stop(format!("{subject}, {}", list.join(", ")))
}

/// A field holds a wanted value. A bare number has to be a whole number of the field: 4 is not in 14.
fn holds_value(field: &str, wanted: &str) -> bool {
    let n = norm(field);
    if wanted.is_empty() {
        false
    } else if wanted.chars().all(|c| c.is_ascii_digit()) {
        n.split(|c: char| !c.is_ascii_digit()).any(|t| t == wanted)
    } else {
        n.contains(wanted)
    }
}

/// A field longer than this is not quoted whole by a code sentence.
const LONG_VALUE: usize = 120;

/// The sentences of a text that hold a wanted value, in order.
fn sentences_holding(text: &str, wanted: &[String]) -> String {
    text.split_inclusive(['.', '!', '?'])
        .filter(|s| {
            let n = norm(s);
            wanted.iter().any(|w| holds_value(&n, w))
        })
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Code sentences for the required values an answer left out: one per fact that holds one, built only
/// from that fact, and only for the values still missing. Empty when no fact holds them.
pub(super) fn added_sentences(d: &Dossier, absent: &[String]) -> (String, Vec<Fact>) {
    let mut said = String::new();
    let mut used = Vec::new();
    for f in real(d) {
        let mut wanted: Vec<String> = f
            .requires
            .iter()
            .filter(|r| absent.contains(r))
            .cloned()
            .collect();
        wanted.retain(|w| !norm(&said).contains(norm(w).trim()));
        if wanted.is_empty() {
            continue;
        }
        let mut only = f.clone();
        only.requires = wanted;
        let sentence = fact_sentence(&only);
        if !said.is_empty() {
            said.push(' ');
        }
        said.push_str(&sentence);
        used.push(f.clone());
    }
    (said, used)
}

/// A statement about what could not be found, as a sentence ("I could not find a makeup artist for
/// this project. I looked in Cast and Crew."), capitalised and ended in case a caller gave less.
fn missing_sentence(s: &str) -> String {
    let mut c = s.trim().chars();
    let first = c.next().map(|f| f.to_uppercase().collect::<String>());
    end_with_stop(first.unwrap_or_default() + c.as_str())
}

/// The last-resort answer and the facts it was built from: one sentence for every fact that carries a
/// value the answer had to state, followed by what the project does not hold ("Anna, actor: Maya Rao.
/// Contact details are not shown for your role."). `None` when no fact carries a value (the project
/// holds nothing for the question, or the question has no single value to state): the abstention is
/// shown instead.
pub(super) fn last_resort(d: &Dossier) -> Option<(String, Vec<Fact>)> {
    let (mut text, used) = added_sentences(d, &d.required);
    if used.is_empty() {
        return None;
    }
    for m in &d.missing {
        text.push(' ');
        text.push_str(&missing_sentence(m));
    }
    Some((text, used))
}

/// The abstention sentence. For a question with a single value to state (`value_asked`) that the
/// project does not hold, it says what is missing and quotes nothing else; otherwise it also shows
/// the first facts that were fetched.
pub(super) fn abstention(d: &Dossier, value_asked: bool) -> (String, Vec<Fact>) {
    let used: Vec<Fact> = if value_asked && !d.missing.is_empty() {
        Vec::new()
    } else {
        real(d).take(2).cloned().collect()
    };
    if used.is_empty() {
        let text = if d.missing.is_empty() {
            super::super::queries::could_not_find("that", super::super::queries::EVERYWHERE)
        } else {
            d.missing
                .iter()
                .map(|m| missing_sentence(m))
                .collect::<Vec<_>>()
                .join(" ")
        };
        return (text, used);
    }
    let said: Vec<String> = used
        .iter()
        .map(|f| truncate_chars(&fact_sentence(f), 240))
        .collect();
    (
        format!(
            "I could not write a reliable answer to that. This is what the project records: {}",
            said.join(" ")
        ),
        used,
    )
}

/// A short, calm reply when even a chat reply failed the checks.
pub(super) fn chat() -> String {
    "Sorry about that. Tell me what you need and I will try again.".to_string()
}
