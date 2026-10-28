//! What the assistant may remember (spec `ai-conversational-harness`, section 5.7). Code only, no
//! model: the person's own words are matched against fixed patterns, so what is saved can always be
//! explained, and a planted sentence in a note, a file or an earlier answer can never become a
//! memory because only the person's message and an answer the person just confirmed are read.
//!
//! A memory is saved when the message
//! - states a fact outright ("I'm the assistant director", "LOW TIDE RADIO is our main script"),
//! - states a preference outright ("always answer in short sentences", "call me Saravan"),
//! - asks to remember something ("remember that the shoot starts on 3 March"), or
//! - confirms the previous answer ("yes, that's right").
//!
//! It is never saved when it is a question, an insult, contact data (an email address, a phone
//! number, a web address), a private note, a line that tries to give the assistant orders, or
//! longer than a short sentence or two.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::modules::ai::index::normalize;

static CONTRACTION: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(\w)'(re|m|ve|ll)\b|(\w)n't\b"));

/// `normalize` after the common contractions are spelled out, so "We're" matches "we are" and
/// "isn't" matches "is not".
fn norm(raw: &str) -> String {
    let t = raw.replace('’', "'");
    let t = CONTRACTION.replace_all(&t, |c: &regex::Captures<'_>| match (c.get(1), c.get(2)) {
        (Some(w), Some(s)) => {
            let word = match s.as_str().to_lowercase().as_str() {
                "re" => "are",
                "m" => "am",
                "ve" => "have",
                _ => "will",
            };
            format!("{} {word}", w.as_str())
        }
        _ => format!("{} not", &c[3]),
    });
    normalize(&t)
}

/// The longest text one entry keeps.
pub const MAX_CHARS: usize = 200;
const MAX_WORDS: usize = 30;
/// A confirmed answer longer than this is an explanation, not a fact; it is not saved.
const CONFIRMABLE_CHARS: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiMemoryKind")]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Fact,
    Preference,
}

impl Kind {
    pub fn code(self) -> &'static str {
        match self {
            Kind::Fact => "fact",
            Kind::Preference => "preference",
        }
    }

    pub fn parse(s: &str) -> Kind {
        if s == "preference" {
            Kind::Preference
        } else {
            Kind::Fact
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub text: String,
    pub kind: Kind,
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("memory rule pattern")
}

/// "Remember that ...", "note that ...", "for the record ...": a request to keep the rest.
static EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^\s*(?:(?:by the way|btw|fyi|ok|okay|so|well|also|and|please)[,:]?\s+)*(?:(?:remember|note|keep in mind)(?:\s+that)?|for the record)[,:]?\s+",
    )
});
static FILLER: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)^\s*(?:(?:by the way|btw|fyi|ok|okay|so|well|also|and|please)[,:]?\s+)+")
});

/// Patterns on the normalised text (lower case words, no punctuation, "I'm" is "im").
static FACT: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^(?:i am|im) (?:the|an?|our) \w+(?: \w+){0,5}$",
        r"^(?:i am|im) (?:working|based|responsible|in charge)\b",
        r"^i (?:work|worked) (?:as|on|for|with|at|in)\b",
        r"^i (?:handle|manage|direct|produce|supervise|oversee|lead|run|edit|schedule)\b",
        r"^(?:my|our) (?:name|role|job|title|team|department|unit|crew|favourite|favorite|main|primary|lead|current|usual|preferred|default|language|deadline|budget|studio|company|production|film|script|screenplay|project|shoot|director|producer|writer|editor)(?: \w+){0,6} (?:is|are) \w+",
        r"^\w+(?: \w+){0,5} is (?:our|the|my) (?:main|primary|final|current|lead|only|first|latest) (?:script|screenplay|draft|project|film|feature|short)\b",
        r"^(?:we|our team) (?:are|is) (?:shooting|filming|prepping|writing|editing|working on|based in)\b",
        r"^(?:we|the team) (?:shoot|film) (?:in|at|on)\b",
    ]
    .iter()
    .map(|p| re(p))
    .collect()
});

static PREFERENCE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"^(?:always|never|from now on|going forward|in future|in the future)\b",
        r"^i (?:prefer|always|never)\b",
        r"^(?:keep|make)(?: your| the| all)? (?:answers|replies|responses|messages)\b",
        r"^(?:answer|reply|respond|write|speak|talk)(?: to me)? (?:in|using|with)\b",
        r"^use (?:short|plain|simple|brief|formal|casual|british|american)\b",
        r"^(?:you can |please )?call me \w+(?: \w+)?$",
        r"^(?:address me as|refer to me as|i go by|im called|i am called)\b",
    ]
    .iter()
    .map(|p| re(p))
    .collect()
});

/// A question or a request is not a statement.
static ASKING: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^(?:who|whom|whose|what|when|where|why|how|which|can|could|would|should|will|do|does|did|is|are|was|were|am|has|have|may|shall|tell me|show me|list|open|find|give me|go to|take me|never mind)\b",
    )
});

/// "I'm a bit lost" is a mood, not a role.
static MOOD: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(?:i am|im) (?:a|an) (?:bit|little|lot|few|touch|tad|big|huge|quick|long|tiny)\b")
});

/// "My script is broken" is a complaint, not a fact.
static COMPLAINT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\b(?:broken|wrong|missing|not|bad|terrible|awful|confusing|stuck|failing|crashed|lost|gone|empty|late|slow|bug|error|problem|issue|hate)\b",
    )
});

static EMAIL: LazyLock<Regex> = LazyLock::new(|| re(r"[\w.+-]+@[\w-]+\.[\w.-]+"));
static PHONE: LazyLock<Regex> = LazyLock::new(|| re(r"\+?\d[\d\s().-]{6,}\d"));
static ADDRESS_WEB: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)https?://|www\.|\.com\b"));
static CONTACT_WORDS: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\bmy (?:phone|mobile|cell|number|email|e-mail|whatsapp|address)\b"));
static SENTENCE_BREAK: LazyLock<Regex> = LazyLock::new(|| re(r"[.!;]\s+\S"));

const INSULTS: &[&str] = &[
    "stupid",
    "idiot",
    "idiotic",
    "useless",
    "dumb",
    "moron",
    "shut up",
    "crap",
    "crappy",
    "garbage",
    "hate you",
    "hate this",
    "sucks",
    "worthless",
    "rubbish",
    "pathetic",
    "damn",
    "fuck",
    "fucking",
    "shit",
    "shitty",
    "bitch",
    "bastard",
    "asshole",
    "bullshit",
    "wtf",
    "screw you",
    "piece of junk",
    "you suck",
    "loser",
];

/// Phrases that try to give the assistant orders. Memory is for facts about the person's work.
const ORDERS: &[&str] = &[
    "ignore previous",
    "ignore all",
    "ignore the above",
    "ignore your",
    "ignore these",
    "disregard",
    "forget previous",
    "forget your",
    "system prompt",
    "developer mode",
    "jailbreak",
    "you are now",
    "pretend to be",
    "pretend you",
    "act as a",
    "act as an",
    "act as if",
    "new instructions",
    "override",
    "reveal your",
    "bypass",
    "do anything now",
];

fn has_phrase(n: &str, phrases: &[&str]) -> bool {
    let padded = format!(" {n} ");
    phrases.iter().any(|p| padded.contains(&format!(" {p} ")))
}

/// True when `raw` holds anything that must never be remembered, whoever wrote it.
fn unsafe_text(raw: &str) -> bool {
    let n = norm(raw);
    raw.contains('?')
        || raw.contains('<')
        || raw.contains("```")
        || EMAIL.is_match(raw)
        || PHONE.is_match(raw)
        || ADDRESS_WEB.is_match(raw)
        || CONTACT_WORDS.is_match(raw)
        || has_phrase(&n, INSULTS)
        || has_phrase(&n, ORDERS)
        || n.contains("private note")
        || n.contains("project data")
        || n.contains("conversation history")
        || n.contains("user request")
}

/// One short statement: a few words, at most two sentences.
fn short_enough(raw: &str) -> bool {
    let words = raw.split_whitespace().count();
    raw.chars().count() <= MAX_CHARS
        && (1..=MAX_WORDS).contains(&words)
        && SENTENCE_BREAK.find_iter(raw).count() <= 1
}

fn tidy(raw: &str) -> String {
    let t = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '\u{201c}' || c == '\u{201d}')
        .trim()
        .to_string();
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => t,
    }
}

/// A fact or preference the person stated in `message`, or `None`.
pub fn from_statement(message: &str) -> Option<Candidate> {
    let (rest, explicit) = match EXPLICIT.find(message) {
        Some(m) => (&message[m.end()..], true),
        None => match FILLER.find(message) {
            Some(m) => (&message[m.end()..], false),
            None => (message, false),
        },
    };
    let text = tidy(rest);
    let n = norm(&text);
    if n.split(' ').filter(|w| !w.is_empty()).count() < 3
        || !short_enough(&text)
        || unsafe_text(&text)
        || ASKING.is_match(&n)
    {
        return None;
    }
    if PREFERENCE.iter().any(|p| p.is_match(&n)) {
        return Some(Candidate {
            text,
            kind: Kind::Preference,
        });
    }
    if explicit
        || (FACT.iter().any(|p| p.is_match(&n)) && !COMPLAINT.is_match(&n) && !MOOD.is_match(&n))
    {
        return Some(Candidate {
            text,
            kind: Kind::Fact,
        });
    }
    None
}

/// Words that open a confirmation and the words that make it one.
const OPENERS: &[&str] = &[
    "yes", "yeah", "yep", "yup", "correct", "right", "exactly", "perfect", "that", "thats", "it",
    "this", "remember", "save",
];
const SIGNS: &[&str] = &[
    "right",
    "correct",
    "exactly",
    "remember",
    "save",
    "spot on",
    "that s it",
];
const DENIALS: &[&str] = &[
    "not",
    "wrong",
    "no",
    "nope",
    "never",
    "incorrect",
    "isn",
    "didn",
    "doesn",
];

/// "Yes, that's right", "correct, remember that": a short agreement with the last answer.
pub fn is_confirmation(message: &str) -> bool {
    if message.contains('?') {
        return false;
    }
    let n = norm(message);
    let words: Vec<&str> = n.split(' ').filter(|w| !w.is_empty()).collect();
    if words.is_empty() || words.len() > 8 || !OPENERS.contains(&words[0]) {
        return false;
    }
    if words.iter().any(|w| DENIALS.contains(w)) {
        return false;
    }
    let padded = format!(" {n} ");
    SIGNS.iter().any(|s| padded.contains(&format!(" {s} ")))
}

/// The answer text that may be kept when the person confirms it: a short, plain statement with no
/// question, no doubt, no contact data and nothing that reads like an instruction (an answer can
/// repeat text planted in a note, and a confirmation must not carry it into memory). `None` for
/// anything else.
pub fn usable_answer(answer: &str) -> Option<String> {
    let one_line = answer.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = one_line.to_lowercase();
    let doubtful = [
        "could not find",
        "couldn't find",
        "not sure",
        "don't know",
        "do not know",
        "no information",
        "i can't",
        "i cannot",
        "sorry",
        "i'm not",
        "unable to",
    ];
    if one_line.is_empty()
        || one_line.chars().count() > CONFIRMABLE_CHARS
        || unsafe_text(&one_line)
        || crate::modules::ai::dossier::guard::looks_like_instruction(&one_line)
        || doubtful.iter().any(|d| lower.contains(d))
    {
        return None;
    }
    let mut kept = String::new();
    for sentence in one_line.split_inclusive(". ") {
        if kept.chars().count() + sentence.chars().count() > MAX_CHARS {
            break;
        }
        kept.push_str(sentence);
    }
    let kept = kept.trim().to_string();
    (!kept.is_empty()).then_some(kept)
}

/// A confirmation of `previous_answer` (already vetted with [`usable_answer`] by the caller or not:
/// it is vetted here too).
pub fn from_confirmation(message: &str, previous_answer: &str) -> Option<Candidate> {
    if !is_confirmation(message) {
        return None;
    }
    usable_answer(previous_answer).map(|text| Candidate {
        text,
        kind: Kind::Fact,
    })
}

/// What, if anything, this message adds to the long-term memory. `previous_answer` is the answer
/// the person has just read, when it is one that may be confirmed.
pub fn candidate(message: &str, previous_answer: Option<&str>) -> Option<Candidate> {
    from_statement(message).or_else(|| previous_answer.and_then(|a| from_confirmation(message, a)))
}
