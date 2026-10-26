//! What is shown when the model's answer failed the checks three times (spec 4.1 and 5.6).
//!
//! Written by code from the facts, so it is true by construction:
//!
//! - a lookup, a count or a comparison gets the **last-resort fact sentence**, built from the first
//!   answering fact: "Lead cast, planned: 40,00,000.";
//! - any other type gets the **abstention sentence** and the first facts that were fetched;
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
    if f.kind == "Count"
        && let (Some(what), Some(value)) = (f.field("what"), f.field("value"))
    {
        return end_with_stop(format!("{value} {what}"));
    }
    let wanted: Vec<String> = f.requires.iter().map(|r| norm(r)).collect();
    let holds = |v: &str| {
        let n = norm(v);
        wanted
            .iter()
            .any(|r| !r.is_empty() && n.contains(r.as_str()))
    };
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
    let list: Vec<String> = asked.iter().map(|(n, v)| format!("{n}: {v}")).collect();
    end_with_stop(format!("{subject}, {}", list.join(", ")))
}

/// A statement about what could not be found, as a sentence ("I could not find a makeup artist for
/// this project. I looked in Cast and Crew."), capitalised and ended in case a caller gave less.
fn missing_sentence(s: &str) -> String {
    let mut c = s.trim().chars();
    let first = c.next().map(|f| f.to_uppercase().collect::<String>());
    end_with_stop(first.unwrap_or_default() + c.as_str())
}

/// The last-resort fact sentence and the fact it was built from: the first fact that carries a value
/// the answer had to state, followed by what the project does not hold ("Anna, actor: Maya Rao.
/// Contact details are not shown for your role."). `None` when no fact carries a value (the project
/// holds nothing for the question, or the question has no single value to state): the abstention is
/// shown instead.
pub(super) fn last_resort(d: &Dossier) -> Option<(String, Fact)> {
    let fact = real(d).find(|f| !f.requires.is_empty())?;
    let mut text = fact_sentence(fact);
    for m in &d.missing {
        text.push(' ');
        text.push_str(&missing_sentence(m));
    }
    Some((text, fact.clone()))
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
