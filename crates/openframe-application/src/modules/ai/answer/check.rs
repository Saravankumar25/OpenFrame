//! The checker of the answer stage (spec `ai-conversational-harness`, section 5.6; ADR-0028).
//!
//! Ordinary code, never a model. It runs on every written answer and reports the exact offenders,
//! so a rewrite can name them. The rules, with the false alarms the S1b spike found built in:
//!
//! - **names**: a name in the answer exists in the facts, the request, the memory or the focus. A
//!   curly apostrophe and a possessive do not make a real name look invented, and a capital letter
//!   after any clause break (a full stop, a colon, a semicolon, a dash) is a sentence start.
//! - **numbers**: a digit number exists in the facts or the request. Both sides are written the same
//!   way first: Indian and western grouping, ordinals, month names. A number *word* in the answer
//!   ("one option", "two") is never checked: only a digit is a claim about the project.
//! - **required**: every value the dossier says a correct answer must state is there, normalised the
//!   same way, so "three" meets 3 and "November 5th" meets "Nov 5".
//! - **length**: inside the word range of the length class. Too short is soft (see `Rule::soft`).
//! - **repetition**: no run of six words twice. Digits count as words, so "scene 3 ... scene 4" is
//!   not a repeat.
//! - **language**: the language of the request.
//! - **ids**: no record id, no web address, no file path and no email address that the facts or the
//!   request did not hold.
//! - **internal**: none of the app's own words (facts, dossier, tool, record id, JSON).
//! - **abstain**: when the project holds nothing for the question, the answer says it could not find
//!   it or that none is recorded.
//! - **echo**: the lines of the data (`Kind | field: value`) are not copied.

use std::collections::HashSet;
use std::sync::LazyLock;

use openframe_ai_checks as checks;
use regex::Regex;

use super::super::text;
use super::super::turn::LengthClass;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid pattern")
}

/// What a check can find wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rule {
    Empty,
    Names,
    Numbers,
    Required,
    Length,
    TooShort,
    Repetition,
    Language,
    Ids,
    Internal,
    Abstain,
    Echo,
    ChangeClaim,
}

impl Rule {
    /// The short code written to the trace.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Rule::Empty => "empty",
            Rule::Names => "names",
            Rule::Numbers => "numbers",
            Rule::Required => "required",
            Rule::Length => "length",
            Rule::TooShort => "too_short",
            Rule::Repetition => "repetition",
            Rule::Language => "language",
            Rule::Ids => "ids",
            Rule::Internal => "internal",
            Rule::Abstain => "abstain",
            Rule::Echo => "echo",
            Rule::ChangeClaim => "change_claim",
        }
    }

    /// A soft offence says nothing wrong: the answer is too long or too thin. After the last pass such
    /// an answer is still shown (cut to the limit when it is too long), because the alternative is a
    /// fallback sentence that tells the person less.
    pub(crate) fn soft(self) -> bool {
        matches!(self, Rule::Length | Rule::TooShort)
    }
}

/// One failed rule and the exact things that failed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Offence {
    pub rule: Rule,
    pub items: Vec<String>,
}

/// The texts an answer may rest on. Record ids are not in any of them: the writer never sees them.
pub(crate) struct Material<'a> {
    /// The facts as the writer read them, and the statements about what the project does not hold.
    pub facts: &'a str,
    /// The person's message.
    pub request: &'a str,
    /// Memory entries, the summary, the recent turns, the focus and the project's screenplay titles.
    pub context: &'a str,
}

/// What a correct answer looks like.
pub(crate) struct Expect<'a> {
    pub class: LengthClass,
    /// Values the answer must state.
    pub required: &'a [String],
    /// The statements about what the project does not hold. The answer must say so when it has no
    /// required value to state.
    pub missing: &'a [String],
    /// The language of the request when it is not English.
    pub language: Option<&'static str>,
}

// ------------------------------------------------------------------ normal form

static MONTH: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(january|february|march|april|june|july|august|september|october|november|december)\b")
});
static ORDINAL: LazyLock<Regex> = LazyLock::new(|| re(r"\b(\d+)(?:st|nd|rd|th)\b"));
static NUMBER_WORD: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"\b(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty)\b",
    )
});
static DIGITS: LazyLock<Regex> = LazyLock::new(|| re(r"\d+"));
static WORDS: LazyLock<Regex> = LazyLock::new(|| re(r"[a-z0-9]+"));

fn month_short(full: &str) -> &'static str {
    match full {
        "january" => "jan",
        "february" => "feb",
        "march" => "mar",
        "april" => "apr",
        "june" => "jun",
        "july" => "jul",
        "august" => "aug",
        "september" => "sep",
        "october" => "oct",
        "november" => "nov",
        _ => "dec",
    }
}

fn number_of(word: &str) -> u32 {
    match word {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        _ => 20,
    }
}

/// A comma between two digits is a grouping mark: `40,00,000` and `4,000,000` are both 4000000.
fn without_group_commas(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        let between = c == ','
            && i > 0
            && chars[i - 1].is_ascii_digit()
            && chars.get(i + 1).is_some_and(char::is_ascii_digit);
        if !between {
            out.push(c);
        }
    }
    out
}

/// Lower case, straight apostrophes, short month names, ordinals as plain numbers, grouping commas
/// gone. With `words`, the number words one to twenty become digits too.
fn normal(s: &str, words: bool) -> String {
    let t = s.to_lowercase().replace(['\u{2019}', '\u{2018}'], "'");
    let t = MONTH.replace_all(&t, |c: &regex::Captures| month_short(&c[1]).to_string());
    let t = ORDINAL.replace_all(&t, "$1");
    let t = without_group_commas(&t);
    if words {
        NUMBER_WORD
            .replace_all(&t, |c: &regex::Captures| number_of(&c[1]).to_string())
            .into_owned()
    } else {
        t
    }
}

/// The normal form used to look for a value in an answer or in the facts.
pub(crate) fn norm(s: &str) -> String {
    normal(s, true)
}

fn has(haystack_norm: &str, needle: &str) -> bool {
    let n = norm(needle);
    let n = n.trim();
    !n.is_empty() && haystack_norm.contains(n)
}

// ------------------------------------------------------------------ the rules

/// Digit numbers of the answer that no text holds. Number words are not numbers here.
pub(crate) fn stray_numbers(answer: &str, pool: &[&str]) -> Vec<String> {
    let trim = |n: &str| {
        let t = n.trim_start_matches('0');
        if t.is_empty() { "0" } else { t }.to_string()
    };
    let known: HashSet<String> = pool
        .iter()
        .flat_map(|p| {
            DIGITS
                .find_iter(&norm(p))
                .map(|m| trim(m.as_str()))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut out: Vec<String> = Vec::new();
    for m in DIGITS.find_iter(&normal(answer, false)) {
        let n = trim(m.as_str());
        if !known.contains(&n) && !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

/// A dash or a semicolon ends a clause like a full stop does, so a capital letter after one is a
/// sentence start, not a name.
fn with_clause_breaks(s: &str) -> String {
    s.replace(['\u{2014}', '\u{2013}', ';'], ". ")
        .replace(" - ", ". ")
}

/// Names in the answer that no evidence text holds.
pub(crate) fn invented_names(answer: &str, evidence: &[&str]) -> Vec<String> {
    checks::ungrounded_names(&with_clause_breaks(answer), evidence)
}

/// The first run of six words that comes twice. Digits are words.
pub(crate) fn repeated_run(answer: &str) -> Option<String> {
    let t = norm(answer);
    let w: Vec<&str> = WORDS.find_iter(&t).map(|m| m.as_str()).collect();
    let mut seen: HashSet<&[&str]> = HashSet::new();
    w.windows(6)
        .find(|run| !seen.insert(run))
        .map(|r| r.join(" "))
}

static UUID: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b"));
static LINK: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\b(?:https?://|www\.)[^\s)>\]]+"));
static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b"));
static PATH: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)(?:\b[a-z]:\\[^\s]+|(?:^|[\s(])/(?:[\w.-]+/)+[\w.-]*|\\\\[\w.$-]+\\[^\s]+)")
});

/// Ids, links, addresses and paths in the answer. A record id is never allowed; the rest only when
/// the facts or the request held the same text.
pub(crate) fn stray_references(answer: &str, pool: &[&str]) -> Vec<String> {
    let pool: String = pool.join("\n").to_lowercase();
    let mut out: Vec<String> = Vec::new();
    if UUID.is_match(answer) {
        out.push("a record id".to_string());
    }
    for r in [&*LINK, &*EMAIL, &*PATH] {
        for m in r.find_iter(answer) {
            let t = m.as_str().trim().trim_end_matches(['.', ',', ';']);
            if !t.is_empty() && !pool.contains(&t.to_lowercase()) {
                out.push(t.to_string());
            }
        }
    }
    out.dedup();
    out
}

static INTERNAL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:\bdossier|\btool|\bjson\b|\brecord ids?\b|\bfacts\b|</?(?:facts|memory|focus|conversation)>)",
    )
});

/// The app's own words an answer must not use.
pub(crate) fn internal_words(answer: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in INTERNAL.find_iter(answer) {
        let w = m.as_str().to_lowercase();
        if !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

static ABSTAIN: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:couldn't find|could not find|can't find|cannot find|unable to find|did not find|didn't find|not in the project|no information|none recorded|none entered|none listed|not recorded|not entered|not listed|not shown|no record|nothing recorded|nothing entered|isn't recorded|is not recorded|aren't recorded|are not recorded|hasn't been (?:recorded|entered)|has no .{0,60}(?:recorded|entered|listed)|no .{0,40} (?:recorded|entered|listed)|nothing in the project)",
    )
});

/// Words of a statement that say what it is about, not that something is missing.
const FILLER: &[&str] = &[
    "project", "this", "that", "with", "from", "have", "your", "role", "entered", "recorded",
    "found", "nothing", "there", "their", "which", "matches", "shown", "listed", "does", "into",
];

fn content_words(s: &str) -> HashSet<String> {
    WORDS
        .find_iter(&s.to_lowercase())
        .map(|m| m.as_str().to_string())
        .filter(|w| w.len() >= 4 && !FILLER.contains(&w.as_str()))
        .collect()
}

/// A word or ending that denies: an answer that only names the missing thing ("the wardrobe is a
/// blue raincoat") has not said it is missing.
static NEGATION: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\b(?:no|none|not|nothing|nobody|empty|without|missing|never|cannot|unable)\b|n't\b")
});

/// The answer says the project holds nothing for the question: a phrase for it, or a denial that
/// names the thing one of the statements about what is missing is about.
pub(crate) fn states_absence(answer: &str, missing: &[String]) -> bool {
    let a = norm(answer);
    if ABSTAIN.is_match(&a) {
        return true;
    }
    if !NEGATION.is_match(&a) {
        return false;
    }
    let said = content_words(&a);
    missing.iter().any(|m| !content_words(m).is_disjoint(&said))
}

static ECHO: LazyLock<Regex> = LazyLock::new(|| re(r"\s\|\s|^\|\s|\s\|$"));

// ------------------------------------------------------------------ the whole check

/// Check one answer. Empty when it passes every rule.
pub(crate) fn check(answer: &str, m: &Material<'_>, e: &Expect<'_>) -> Vec<Offence> {
    let mut out: Vec<Offence> = Vec::new();
    let mut add = |rule: Rule, items: Vec<String>| out.push(Offence { rule, items });
    if answer.trim().is_empty() {
        add(Rule::Empty, Vec::new());
        return out;
    }
    let evidence: [&str; 3] = [m.facts, m.request, m.context];
    let n = checks::word_count(answer);
    let (lo, hi) = e.class.words();
    if n > hi {
        add(Rule::Length, vec![format!("{n} words, at most {hi}")]);
    } else if n < lo {
        add(Rule::TooShort, vec![format!("{n} words, at least {lo}")]);
    }
    let seen = norm(answer);
    let absent: Vec<String> = e
        .required
        .iter()
        .filter(|r| !has(&seen, r))
        .cloned()
        .collect();
    if !absent.is_empty() {
        add(Rule::Required, absent);
    }
    if e.required.is_empty()
        && !e.missing.is_empty()
        && e.language.is_none()
        && !states_absence(answer, e.missing)
    {
        add(Rule::Abstain, Vec::new());
    }
    let names = invented_names(answer, &[evidence[0], evidence[1], evidence[2], LABELS]);
    if !names.is_empty() {
        add(Rule::Names, names);
    }
    let numbers = stray_numbers(answer, &evidence);
    if !numbers.is_empty() {
        add(Rule::Numbers, numbers);
    }
    if let Some(want) = e.language
        && !checks::language_matches(answer, want)
    {
        add(Rule::Language, vec![want.to_string()]);
    }
    if let Some(run) = repeated_run(answer) {
        add(Rule::Repetition, vec![run]);
    }
    let refs = stray_references(answer, &evidence);
    if !refs.is_empty() {
        add(Rule::Ids, refs);
    }
    let internal = internal_words(answer);
    if !internal.is_empty() {
        add(Rule::Internal, internal);
    }
    if ECHO.is_match(answer) {
        add(Rule::Echo, Vec::new());
    }
    if text::claims_a_change(answer) {
        add(Rule::ChangeClaim, Vec::new());
    }
    out
}

/// Words a written answer uses as labels. They are not names of the project.
const LABELS: &str = "Idea Ideas Suggestion Suggestions Note Notes Overall Summary Overview \
Conclusion Beginning Middle End Setup Payoff Theme Themes Characters Opening Ending Key Moments \
Act Acts Scene Scenes Draft Story Screenplay OpenFrame Studio \nSorry Apologies Thanks Thank Welcome Glad Happy Understood Noted Hi Hello Hey Cheers Goodbye Bye \nGood Great Nice Fine Alright Hmm";

/// The one short message that follows a failed pass. Machine-written: the exact offenders. The
/// wording is the one the spike measured ("Rewrite your answer. Fix exactly these problems: ...").
pub(crate) fn feedback(offences: &[Offence], say: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for o in offences {
        let list = o.items.join(", ");
        lines.push(match o.rule {
            Rule::Empty => "the answer was empty; answer the request".to_string(),
            Rule::Names => format!("names not in the project: {list}; remove them"),
            Rule::Numbers => format!("numbers not in the project: {list}; remove them"),
            Rule::Required => format!("missing: {list}; state each of them"),
            Rule::Length => format!("length {list}; make it shorter"),
            Rule::TooShort => format!("length {list}; add what the project records"),
            Rule::Repetition => format!("repeated words: \"{list}\"; say each thing once"),
            Rule::Language => format!(
                "write the whole answer in {}",
                super::language_name(&list).unwrap_or("the language of the request")
            ),
            Rule::Ids => format!(
                "do not write ids, links, addresses or file paths that the request did not have: {list}"
            ),
            Rule::Internal => format!(
                "do not use the words {list}; say \"the project\" instead"
            ),
            Rule::Abstain => {
                "say plainly that the project has none recorded, or that you could not find it".to_string()
            }
            Rule::Echo => "write sentences; do not copy the lines of the data".to_string(),
            Rule::ChangeClaim => "nothing was changed; do not say that anything was renamed, added, removed or updated".to_string(),
        });
    }
    format!(
        "Rewrite your answer. Fix exactly these problems: {}. {say}",
        lines.join("; ")
    )
}

/// Cut an answer that is too long to the limit at a sentence end.
pub(crate) fn cut_to(answer: &str, class: LengthClass) -> String {
    let (_, hi) = class.words();
    text::fit_words(answer.to_string(), hi)
}
