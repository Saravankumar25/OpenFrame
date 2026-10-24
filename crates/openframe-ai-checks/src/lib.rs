//! Deterministic checks of AI answers (ADR-0028 decisions 1, 5 and 9).
//!
//! Ordinary code, never a model: the offline evaluation harness (`openframe-ai-eval`) scores gold
//! cases with these functions, and the runtime verifier (Windows phase W6) uses the same ones, so the
//! development scores and the checks people meet in the app always agree.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

/// The language of a text, as a two-letter code (`en`, `hi`, `te`, `ta`, `kn`, `ml`, `es`, `fr`,
/// `de`, `zh`, `ja`, `ko`), or `None` when it has no letters. Non-Latin scripts win when they make
/// up at least 30 percent of the letters, because answers often carry Latin names (Lakshmi, Farooq
/// Senior) inside text of another language. Latin text is told apart by its common words.
pub fn detect_language(text: &str) -> Option<&'static str> {
    let mut n = Counts::default();
    for c in text.chars() {
        match c as u32 {
            0x0900..=0x097F => n.devanagari += 1,
            0x0C00..=0x0C7F => n.telugu += 1,
            0x0B80..=0x0BFF => n.tamil += 1,
            0x0C80..=0x0CFF => n.kannada += 1,
            0x0D00..=0x0D7F => n.malayalam += 1,
            0x3040..=0x30FF => n.kana += 1,
            0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF => n.hangul += 1,
            0x3400..=0x4DBF | 0x4E00..=0x9FFF => n.han += 1,
            _ if c.is_alphabetic() && (c.is_ascii() || ('\u{00C0}'..='\u{024F}').contains(&c)) => {
                n.latin += 1
            }
            _ => {}
        }
    }
    let total = n.total();
    if total == 0 {
        return None;
    }
    let share = |x: usize| x as f64 / total as f64;
    if n.kana > 0 && share(n.kana + n.han) >= 0.3 {
        return Some("ja");
    }
    let scripts = [
        (n.devanagari, "hi"),
        (n.telugu, "te"),
        (n.tamil, "ta"),
        (n.kannada, "kn"),
        (n.malayalam, "ml"),
        (n.hangul, "ko"),
        (n.han, "zh"),
    ];
    let (best, code) = scripts
        .iter()
        .copied()
        .max_by_key(|(k, _)| *k)
        .unwrap_or((0, "en"));
    if best > 0 && share(best) >= 0.3 {
        return Some(code);
    }
    Some(latin_language(text))
}

#[derive(Default)]
struct Counts {
    devanagari: usize,
    telugu: usize,
    tamil: usize,
    kannada: usize,
    malayalam: usize,
    kana: usize,
    hangul: usize,
    han: usize,
    latin: usize,
}

impl Counts {
    fn total(&self) -> usize {
        self.devanagari
            + self.telugu
            + self.tamil
            + self.kannada
            + self.malayalam
            + self.kana
            + self.hangul
            + self.han
            + self.latin
    }
}

const STOP: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "the", "and", "is", "of", "to", "in", "it", "with", "his", "her", "for", "on", "at",
            "was",
        ],
    ),
    (
        "es",
        &[
            "el", "los", "las", "y", "del", "una", "con", "por", "su", "es", "se", "para", "como",
            "pero",
        ],
    ),
    (
        "fr",
        &[
            "le", "les", "et", "des", "du", "une", "dans", "est", "pour", "il", "elle", "au",
            "sur", "pas",
        ],
    ),
    (
        "de",
        &[
            "der", "die", "das", "und", "ist", "ein", "eine", "mit", "den", "zu", "im", "auf",
            "sie", "er",
        ],
    ),
];

fn latin_language(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();
    STOP.iter()
        .map(|(code, list)| (words.iter().filter(|w| list.contains(w)).count(), *code))
        // A tie keeps the earlier language of the list.
        .fold(
            (0, "en"),
            |best, (hits, code)| if hits > best.0 { (hits, code) } else { best },
        )
        .1
}

/// Whether the answer is written in the expected language.
pub fn language_matches(text: &str, expected: &str) -> bool {
    detect_language(text).is_some_and(|l| l == expected)
}

/// Words in a text. Han, kana and Hangul characters count as half a word each, so a limit in words
/// means about the same length in every language.
pub fn word_count(text: &str) -> usize {
    let cjk = text.chars().filter(|c| matches!(*c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF)).count();
    let spaced = text.split_whitespace().filter(|w| w.chars().any(|c| c.is_alphanumeric() && !matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF))).count();
    spaced + cjk.div_ceil(2)
}

/// Whether `haystack` contains `needle`, ignoring case.
pub fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Whether the actual tool arguments hold every expected argument. Objects match key by key,
/// strings by case-insensitive containment (so `"Kavitha"` matches `"Dr. Kavitha Nair"`), numbers
/// and strings by their text (`21` matches `"21"`), and every expected array item must match some
/// actual item.
pub fn args_match(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => e
            .iter()
            .all(|(k, v)| a.get(k).is_some_and(|av| args_match(v, av))),
        (Value::Array(e), Value::Array(a)) => {
            e.iter().all(|ev| a.iter().any(|av| args_match(ev, av)))
        }
        (Value::String(e), _) => scalar_text(actual).is_some_and(|a| contains_ci(&a, e.trim())),
        (Value::Number(_) | Value::Bool(_), _) => scalar_text(actual) == scalar_text(expected),
        (Value::Null, _) => actual.is_null(),
        _ => false,
    }
}

fn scalar_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

const NOT_NAMES: &[&str] = &[
    "the",
    "this",
    "that",
    "these",
    "those",
    "there",
    "then",
    "they",
    "she",
    "her",
    "his",
    "him",
    "and",
    "but",
    "when",
    "where",
    "what",
    "which",
    "who",
    "while",
    "with",
    "after",
    "before",
    "scene",
    "scenes",
    "act",
    "draft",
    "here",
    "his",
    "its",
    "one",
    "two",
    "three",
    "yes",
    "not",
    "for",
    "from",
    "into",
    "int",
    "ext",
    "day",
    "night",
    "dawn",
    "dusk",
    "continuous",
    "openframe",
    "inferred",
    "exact",
];

/// How much of an answer is grounded in the project: the share of its names (capitalised words of
/// three letters or more) and numbers that also appear in `source`. Returns (found, total); an
/// answer with nothing to check is (0, 0).
pub fn grounding(answer: &str, source: &str) -> (usize, usize) {
    let source = source.to_lowercase();
    let mut found = 0;
    let mut total = 0;
    for token in answer.split(|c: char| !(c.is_alphanumeric() || c == '\'')) {
        let t = token.trim_matches('\'');
        let first = t.chars().next();
        let is_number = !t.is_empty() && t.chars().all(|c| c.is_ascii_digit());
        let is_name = t.chars().count() >= 3
            && first.is_some_and(|c| c.is_uppercase())
            && !NOT_NAMES.contains(&t.to_lowercase().as_str());
        if is_number || is_name {
            total += 1;
            if source.contains(&t.to_lowercase()) {
                found += 1;
            }
        }
    }
    (found, total)
}

/// An answer that says nothing: empty, or one sentence repeated three times or more.
pub fn empty_or_repeated(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    let sentences: Vec<String> = t
        .split(['.', '!', '?', '\n', '。'])
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.len() > 8)
        .collect();
    sentences
        .iter()
        .any(|s| sentences.iter().filter(|o| *o == s).count() >= 3)
}

// ------------------------------------------------------------------ answer-versus-evidence checks
//
// Ordinary code that compares what an answer claims with the texts it may rest on (the project,
// the request, tool results). The evaluation harness and the runtime verifier call the same
// functions (harness plan 5.9). They report offenders; deciding what to do is the caller's job.

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid pattern")
}

/// A word of a text: letters, with an apostrophe allowed between letters. Digits and punctuation
/// end a word, so "V.O." gives "V" and "O".
struct Tok<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

fn word_tokens(text: &str) -> Vec<Tok<'_>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &(b, c)) in chars.iter().enumerate() {
        let inner_apostrophe = matches!(c, '\'' | '’')
            && start.is_some()
            && chars.get(i + 1).is_some_and(|(_, n)| n.is_alphabetic());
        if c.is_alphabetic() || inner_apostrophe {
            start.get_or_insert(b);
        } else if let Some(s) = start.take() {
            out.push(Tok {
                start: s,
                end: b,
                text: &text[s..b],
            });
        }
    }
    if let Some(s) = start {
        out.push(Tok {
            start: s,
            end: text.len(),
            text: &text[s..],
        });
    }
    out
}

fn straight(s: &str) -> String {
    s.replace(['’', '‘'], "'")
}

/// The word without a trailing possessive ("Lakshmi's" gives "Lakshmi").
fn without_possessive(w: &str) -> &str {
    ["'s", "’s", "'", "’"]
        .iter()
        .find_map(|suf| w.strip_suffix(suf).filter(|s| !s.is_empty()))
        .unwrap_or(w)
}

/// Words that are never names wherever they stand: screenplay labels, months, weekdays and the
/// spelled numbers of "Act Two". Compared in lower case.
const ALLOWED: &[&str] = &[
    "i",
    "ok",
    "int",
    "ext",
    "day",
    "night",
    "continuous",
    "scene",
    "scenes",
    "act",
    "acts",
    "openframe",
    "vo",
    "os",
    "cont'd",
    "contd",
    "more",
    "ai",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "jan",
    "feb",
    "mar",
    "apr",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// Common English words that start a sentence and are not names. Compared in lower case, and only
/// for the first word of a sentence.
const SENTENCE_STARTERS: &[&str] = &[
    "the",
    "a",
    "an",
    "she",
    "he",
    "they",
    "it",
    "we",
    "you",
    "in",
    "on",
    "at",
    "when",
    "then",
    "this",
    "that",
    "these",
    "those",
    "what",
    "who",
    "why",
    "how",
    "where",
    "which",
    "after",
    "before",
    "but",
    "and",
    "or",
    "so",
    "yes",
    "no",
    "here",
    "there",
    "later",
    "meanwhile",
    "scene",
    "act",
    "as",
    "if",
    "while",
    "during",
    "once",
    "soon",
    "now",
    "next",
    "first",
    "second",
    "third",
    "finally",
    "also",
    "however",
    "still",
    "instead",
    "because",
    "although",
    "though",
    "since",
    "until",
    "with",
    "without",
    "from",
    "to",
    "of",
    "for",
    "by",
    "into",
    "onto",
    "over",
    "under",
    "through",
    "between",
    "among",
    "about",
    "around",
    "across",
    "behind",
    "beyond",
    "inside",
    "outside",
    "near",
    "despite",
    "both",
    "each",
    "every",
    "some",
    "any",
    "all",
    "many",
    "most",
    "more",
    "several",
    "other",
    "another",
    "such",
    "only",
    "just",
    "even",
    "perhaps",
    "maybe",
    "probably",
    "possibly",
    "likely",
    "overall",
    "ultimately",
    "essentially",
    "basically",
    "actually",
    "honestly",
    "sure",
    "certainly",
    "please",
    "thanks",
    "hello",
    "hi",
    "hey",
    "okay",
    "ok",
    "well",
    "oh",
    "ah",
    "note",
    "notes",
    "let",
    "her",
    "his",
    "their",
    "its",
    "our",
    "my",
    "your",
    "one",
    "two",
    "three",
    "everything",
    "everyone",
    "someone",
    "nothing",
    "nobody",
    "something",
    "anyone",
    "few",
    "little",
    "much",
    "whether",
    "whereas",
    "unlike",
    "like",
    "given",
    "based",
    "according",
    "looking",
    "considering",
    "throughout",
    "early",
    "late",
    "eventually",
    "initially",
    "originally",
    "immediately",
    "suddenly",
    "slowly",
    "quickly",
    "clearly",
    "unfortunately",
    "fortunately",
    "sadly",
    "interestingly",
    "importantly",
    "notably",
    "specifically",
    "generally",
    "typically",
    "usually",
    "often",
    "sometimes",
    "always",
    "never",
    "again",
    "yet",
    "thus",
    "therefore",
    "hence",
    "otherwise",
    "afterward",
    "afterwards",
    "together",
    "alone",
    "opening",
    "ending",
    "closing",
    "strengths",
    "weaknesses",
    "theme",
    "themes",
    "summary",
    "answer",
    "idea",
    "ideas",
    "suggestion",
    "suggestions",
    "option",
    "options",
    "critique",
    "overview",
    "character",
    "characters",
    "plot",
    "story",
    "script",
    "screenplay",
    "draft",
    "dialogue",
    "action",
    "tone",
    "mood",
    "conflict",
    "stakes",
    "strong",
    "consider",
    "try",
    "add",
    "make",
    "use",
    "keep",
    "show",
    "give",
    "cut",
    "trim",
    "move",
    "replace",
    "start",
    "end",
    "open",
    "close",
    "think",
    "look",
    "ask",
    "tell",
    "remember",
    "imagine",
    "don't",
    "can't",
    "couldn't",
    "didn't",
    "doesn't",
    "isn't",
    "wasn't",
    "i'm",
    "i've",
    "i'd",
    "i'll",
];

fn is_name_word(w: &str) -> bool {
    let mut letters = w.chars().filter(|c| c.is_alphabetic());
    letters.next().is_some_and(char::is_uppercase) && letters.next().is_some()
}

/// Whether the text before byte `at` ends a sentence: the start of the text, a new line, or one of
/// `. ! ? :`, ignoring spaces and opening quotes, brackets and list marks.
fn at_sentence_start(text: &str, at: usize) -> bool {
    let mut newline = false;
    for c in text[..at].chars().rev() {
        match c {
            '\n' | '\r' => newline = true,
            c if c.is_whitespace() => {}
            '"' | '\'' | '“' | '‘' | '(' | '[' | '*' | '_' | '#' | '>' | '-' | '•' | '–'
            | '\u{2014}' => {}
            '.' | '!' | '?' | ':' | '。' | '！' | '？' => return true,
            _ => return newline,
        }
    }
    true
}

/// Proper-name candidates in an answer: runs of capitalised words ("Lakshmi", "Venkat Rao",
/// "Farooq Senior") and all-caps words of two or more letters ("DEVI"), in order, without repeats.
/// A common English word that opens a sentence is not a name ("She", "Meanwhile", the "The" of
/// "The Tower"). Screenplay labels, months and weekdays are never names. Text in scripts without
/// capitals (Indic, Han, kana, Hangul) gives none; Latin names inside it are still found. German
/// nouns are capitalised, so this check is meant for English answers.
pub fn candidate_names(answer: &str) -> Vec<String> {
    fn flush(
        answer: &str,
        run: &mut Option<(usize, Vec<String>)>,
        seen: &mut std::collections::HashSet<String>,
        out: &mut Vec<String>,
    ) {
        let Some((start, mut words)) = run.take() else {
            return;
        };
        if at_sentence_start(answer, start)
            && words
                .first()
                .is_some_and(|w| SENTENCE_STARTERS.contains(&straight(&w.to_lowercase()).as_str()))
        {
            words.remove(0);
        }
        if words.is_empty() {
            return;
        }
        let name = words.join(" ");
        if seen.insert(name.to_lowercase()) {
            out.push(name);
        }
    }
    let toks = word_tokens(answer);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut run: Option<(usize, Vec<String>)> = None;
    let mut prev_end = 0;
    for t in &toks {
        let w = without_possessive(t.text);
        let allowed = ALLOWED.contains(&straight(&w.to_lowercase()).as_str());
        if !is_name_word(w) || allowed {
            flush(answer, &mut run, &mut seen, &mut out);
            continue;
        }
        let gap = &answer[prev_end..t.start];
        let joins = !gap.is_empty() && gap.chars().all(|c| matches!(c, ' ' | '\t' | '\u{a0}'));
        if !joins {
            flush(answer, &mut run, &mut seen, &mut out);
        }
        run.get_or_insert_with(|| (t.start, Vec::new()))
            .1
            .push(w.to_string());
        prev_end = t.end;
        if w.len() != t.text.len() {
            // A possessive ends a name: "Lakshmi's Tower" is two things.
            flush(answer, &mut run, &mut seen, &mut out);
        }
    }
    flush(answer, &mut run, &mut seen, &mut out);
    out
}

/// The lower-case words of a text, each also without a trailing possessive.
fn word_set(text: &str) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    for t in word_tokens(text) {
        set.insert(straight(&t.text.to_lowercase()));
        set.insert(straight(&without_possessive(t.text).to_lowercase()));
    }
    set
}

/// The name candidates of the answer that the evidence does not support. A candidate is grounded
/// when each of its words occurs as a whole word, ignoring case, in at least one evidence text.
pub fn ungrounded_names(answer: &str, evidence: &[&str]) -> Vec<String> {
    let sets: Vec<_> = evidence.iter().map(|e| word_set(e)).collect();
    candidate_names(answer)
        .into_iter()
        .filter(|n| {
            !n.split_whitespace().all(|w| {
                let w = straight(&w.to_lowercase());
                sets.iter().any(|s| s.contains(&w))
            })
        })
        .collect()
}

static SCENE_REF: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\b(scenes?|sc\.?)\s*(\d+(?:\s*(?:,|and|&|-|–|\x{2014}|to|through)\s*\d+)*)")
});
static SCENE_PART: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(\d+)|(,|and|&|-|–|\x{2014}|to|through)"));

/// Scene numbers an answer cites, sorted without repeats: "Scene 12", "scene 5", "Sc. 7",
/// "Scenes 5 and 6", "scenes 3-4", "scenes 3 to 4" (a range is expanded when it is at most 50
/// long). A comma list counts only after the plural "scenes".
// shortcut: "Scene 5 and 2 men" reads 2 as a scene; upgrade if the bench shows it.
pub fn scene_refs(answer: &str) -> Vec<u32> {
    let mut out = std::collections::BTreeSet::new();
    for cap in SCENE_REF.captures_iter(answer) {
        let plural = cap[1].to_lowercase() == "scenes";
        let mut last: Option<u32> = None;
        let mut range = false;
        for p in SCENE_PART.captures_iter(&cap[2]) {
            if let Some(n) = p.get(1) {
                let Ok(n) = n.as_str().parse::<u32>() else {
                    break;
                };
                if range && let Some(a) = last.filter(|a| *a <= n && n - *a <= 50) {
                    out.extend(a..=n);
                }
                out.insert(n);
                last = Some(n);
                range = false;
            } else {
                let sep = p[2].to_lowercase();
                if sep == "," && !plural {
                    break;
                }
                range = matches!(sep.as_str(), "-" | "–" | "\u{2014}" | "to" | "through");
            }
        }
    }
    out.into_iter().collect()
}

static STRAIGHT_QUOTED: LazyLock<Regex> = LazyLock::new(|| re(r#""([^"]+)""#));
static CURLY_QUOTED: LazyLock<Regex> = LazyLock::new(|| re(r"“([^”]+)”"));

/// Text inside double quotes (straight or curly) of at least three words, in order.
pub fn quoted_spans(answer: &str) -> Vec<String> {
    let mut found: Vec<(usize, String)> = [&*STRAIGHT_QUOTED, &*CURLY_QUOTED]
        .iter()
        .flat_map(|r| r.captures_iter(answer))
        .filter_map(|c| {
            let m = c.get(1)?;
            let t = m.as_str().trim();
            (t.split_whitespace().count() >= 3).then(|| (m.start(), t.to_string()))
        })
        .collect();
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, t)| t).collect()
}

/// Lower case, curly apostrophes straight, everything but letters, digits and apostrophes turned
/// into single spaces, so a quote matches whatever its punctuation and spacing.
fn plain(text: &str) -> String {
    straight(&text.to_lowercase())
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quoted spans (see [`quoted_spans`]) that no evidence text contains, ignoring case, spacing,
/// punctuation and curly apostrophes. An ellipsis inside a quote splits it into parts, and each
/// part must be found.
pub fn missing_quotes(answer: &str, evidence: &[&str]) -> Vec<String> {
    let hay: Vec<String> = evidence.iter().map(|e| format!(" {} ", plain(e))).collect();
    quoted_spans(answer)
        .into_iter()
        .filter(|span| {
            span.split("...")
                .flat_map(|p| p.split('…'))
                .map(plain)
                .filter(|p| !p.is_empty())
                .any(|part| {
                    let needle = format!(" {part} ");
                    !hay.iter().any(|h| h.contains(&needle))
                })
        })
        .collect()
}

static DIGITS: LazyLock<Regex> = LazyLock::new(|| re(r"\d+"));

/// Digit runs in the answer that appear in no evidence text as a digit run. The number 1 is never
/// reported (it is "a" in many sentences). Without repeats, in order.
pub fn unsupported_numbers(answer: &str, evidence: &[&str]) -> Vec<String> {
    let known: std::collections::HashSet<&str> = evidence
        .iter()
        .flat_map(|e| DIGITS.find_iter(e).map(|m| m.as_str()))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for m in DIGITS.find_iter(answer) {
        let n = m.as_str();
        if n != "1" && !known.contains(n) && !out.iter().any(|o| o == n) {
            out.push(n.to_string());
        }
    }
    out
}

const RELATIONSHIPS: &[&str] = &[
    "mother",
    "father",
    "sister",
    "brother",
    "wife",
    "husband",
    "son",
    "daughter",
    "uncle",
    "aunt",
    "cousin",
    "grandmother",
    "grandfather",
    "niece",
    "nephew",
    "girlfriend",
    "boyfriend",
    "fiance",
    "fiancee",
    "lover",
    "rival",
    "enemy",
    "boss",
    "employee",
    "partner",
    "best friend",
    "friend",
    "colleague",
    "neighbour",
    "neighbor",
    "landlord",
    "tenant",
    "teacher",
    "student",
];

static RELATIONSHIP: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)\b({})s?\b", RELATIONSHIPS.join("|"))));

/// The relationship words of a text, in lower case, singular. "best friend" also gives "friend".
fn relationship_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for m in RELATIONSHIP.captures_iter(text) {
        let w = m[1].to_lowercase();
        if w == "best friend" {
            out.push("friend".to_string());
        }
        out.push(w);
    }
    out
}

/// Relationship words ("sister", "husband", "rival", "boss" and others) that the answer uses and
/// no evidence text does (whole word, ignoring case, plural allowed). Without repeats, in order.
pub fn unsupported_relationship_words(answer: &str, evidence: &[&str]) -> Vec<String> {
    let known: std::collections::HashSet<String> = evidence
        .iter()
        .flat_map(|e| relationship_words(e))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for w in relationship_words(answer) {
        if !known.contains(&w) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// The first word n-gram (lower case, punctuation stripped) that occurs three or more times, or
/// None. `n` of 0 gives None.
pub fn repeated_ngram(answer: &str, n: usize) -> Option<String> {
    let words: Vec<String> = answer
        .split_whitespace()
        .map(|w| {
            straight(&w.to_lowercase())
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect();
    if n == 0 || words.len() < n {
        return None;
    }
    let mut counts: std::collections::HashMap<&[String], usize> = std::collections::HashMap::new();
    for w in words.windows(n) {
        *counts.entry(w).or_default() += 1;
    }
    words
        .windows(n)
        .find(|w| counts[w] >= 3)
        .map(|w| w.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn languages_are_told_apart_even_with_latin_names_inside() {
        assert_eq!(
            detect_language("Lakshmi climbs the tower in the storm and fixes the feeder."),
            Some("en")
        );
        assert_eq!(
            detect_language("लक्ष्मी टावर पर चढ़ती है और Farooq बस लाता है।"),
            Some("hi")
        );
        assert_eq!(
            detect_language("లక్ష్మి టవర్ ఎక్కుతుంది, Venkat Rao చూస్తాడు."),
            Some("te")
        );
        assert_eq!(detect_language("லக்ஷ்மி கோபுரத்தில் ஏறுகிறாள்."), Some("ta"));
        assert_eq!(detect_language("ಲಕ್ಷ್ಮಿ ಗೋಪುರವನ್ನು ಹತ್ತುತ್ತಾಳೆ."), Some("kn"));
        assert_eq!(detect_language("ലക്ഷ്മി ടവറിൽ കയറുന്നു."), Some("ml"));
        assert_eq!(
            detect_language("Lakshmi sube a la torre y la radio vuelve a sonar."),
            Some("es")
        );
        assert_eq!(
            detect_language("Lakshmi grimpe sur la tour et la radio est de retour."),
            Some("fr")
        );
        assert_eq!(
            detect_language("Lakshmi klettert auf den Turm und das Radio ist wieder da."),
            Some("de")
        );
        assert_eq!(
            detect_language("Lakshmi 爬上铁塔，修好了天线。"),
            Some("zh")
        );
        assert_eq!(
            detect_language("Lakshmi は塔に登り、アンテナを直します。"),
            Some("ja")
        );
        assert_eq!(
            detect_language("Lakshmi는 탑에 올라가 안테나를 고칩니다."),
            Some("ko")
        );
        assert_eq!(detect_language("  123 ... "), None);
        assert!(language_matches(
            "Resumen: Lakshmi sube a la torre y el transmisor vuelve a funcionar.",
            "es"
        ));
        assert!(!language_matches("Lakshmi climbs the tower.", "te"));
    }

    #[test]
    fn word_counts_are_comparable_across_scripts() {
        assert_eq!(word_count("Lakshmi climbs the tower."), 4);
        assert_eq!(word_count("用两句话概括"), 3);
        assert_eq!(word_count(""), 0);
    }

    #[test]
    fn arguments_match_by_subset_containment_and_text() {
        let actual = json!({ "name": "Dr. Kavitha Nair", "sceneNumber": 21, "characters": ["LAKSHMI", "SURESH"], "extra": true });
        assert!(args_match(&json!({ "name": "kavitha" }), &actual));
        assert!(args_match(&json!({ "sceneNumber": "21" }), &actual));
        assert!(args_match(&json!({ "characters": ["Suresh"] }), &actual));
        assert!(args_match(&json!({}), &actual));
        assert!(!args_match(&json!({ "name": "Padma" }), &actual));
        assert!(!args_match(&json!({ "missing": "x" }), &actual));
        assert!(!args_match(&json!({ "characters": ["Padma"] }), &actual));
    }

    #[test]
    fn grounding_counts_names_and_numbers_found_in_the_source() {
        let source = "Lakshmi climbs the tower. Venkat Rao built it in 1989.";
        assert_eq!(
            grounding("Lakshmi climbs. Venkat built it in 1989.", source),
            (3, 3)
        );
        assert_eq!(grounding("Priya built it in 2001.", source), (0, 2));
        assert_eq!(grounding("the end", source), (0, 0));
    }

    #[test]
    fn empty_and_looping_answers_are_caught() {
        assert!(empty_or_repeated("   "));
        assert!(empty_or_repeated(
            "The tower stands. The tower stands. The tower stands."
        ));
        assert!(!empty_or_repeated("The tower stands. Lakshmi climbs it."));
    }

    #[test]
    fn names_are_found_and_sentence_start_words_are_not_names() {
        assert_eq!(
            candidate_names("Lakshmi meets Venkat Rao and DEVI at the tower."),
            ["Lakshmi", "Venkat Rao", "DEVI"]
        );
        // Sentence-start words, the "The" of a title, and repeats.
        assert_eq!(
            candidate_names(
                "She climbs. Then Farooq Senior arrives. The Tower hums. Meanwhile Lakshmi waits. Lakshmi sings."
            ),
            ["Farooq Senior", "Tower", "Lakshmi"]
        );
        assert_eq!(
            candidate_names("Yes: Lakshmi.\nMeanwhile Ravi."),
            ["Lakshmi", "Ravi"]
        );
        // Mid-sentence capitals count, even common ones.
        assert_eq!(candidate_names("He asked When she left."), ["When"]);
        // Labels, months and weekdays never count; possessives end a name.
        assert_eq!(candidate_names("INT. TOWER - DAY"), ["TOWER"]);
        assert_eq!(
            candidate_names(
                "In March on Monday, Scene 4 and ACT TWO. I said OK to the AI (V.O., O.S., CONT'D, MORE)."
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            candidate_names("Lakshmi's Tower stands."),
            ["Lakshmi", "Tower"]
        );
        // Scripts without capitals give no names; Latin names inside them still do.
        assert!(candidate_names("लक्ष्मी टावर पर चढ़ती है। 塔に登る。 탑에 올라간다.").is_empty());
        assert_eq!(
            candidate_names("लक्ष्मी टावर पर चढ़ती है और Farooq बस लाता है।"),
            ["Farooq"]
        );
        assert!(candidate_names("").is_empty());
    }

    #[test]
    fn invented_names_are_the_ones_the_evidence_lacks() {
        let source = "Lakshmi climbs the tower. VENKAT RAO watches. Farooq's bus.";
        let request = "Who is Devi?";
        let ev = [source, request];
        assert!(ungrounded_names("Lakshmi and Venkat Rao see Farooq near Devi.", &ev).is_empty());
        assert_eq!(
            ungrounded_names("Lakshmi meets Priya Nair at the tower.", &ev),
            ["Priya Nair"]
        );
        // Every word of a name must be there: Venkat is, Reddy is not.
        assert_eq!(
            ungrounded_names("Venkat Reddy waits.", &ev),
            ["Venkat Reddy"]
        );
        assert_eq!(ungrounded_names("Lakshmi waits.", &[]), ["Lakshmi"]);
    }

    #[test]
    fn scene_numbers_are_read_in_every_common_form() {
        assert_eq!(scene_refs("Scene 12 and scene 5, then Sc. 7."), [5, 7, 12]);
        assert_eq!(scene_refs("Scenes 5 and 6 matter."), [5, 6]);
        assert_eq!(
            scene_refs("In scenes 3-4 and scenes 8 to 10."),
            [3, 4, 8, 9, 10]
        );
        assert_eq!(scene_refs("Scenes 2, 4 and 9."), [2, 4, 9]);
        // A comma after the singular is not a list.
        assert_eq!(scene_refs("In scene 5, 2 men talk."), [5]);
        // A range longer than 50 is only its two ends.
        assert_eq!(scene_refs("Scenes 1 to 90"), [1, 90]);
        assert!(scene_refs("The scenery is nice. Scene one.").is_empty());
        assert!(scene_refs("").is_empty());
    }

    #[test]
    fn quotes_must_be_in_the_evidence() {
        let ans = "She says \"I can't leave the tower\" and later “the radio is dead again”. He says \"No\" and \"two words\".";
        assert_eq!(
            quoted_spans(ans),
            ["I can't leave the tower", "the radio is dead again"]
        );
        let ev = ["LAKSHMI\nI can’t leave   the Tower, not tonight.\nThe radio is dead again!"];
        assert!(missing_quotes(ans, &ev).is_empty());
        assert_eq!(
            missing_quotes("He shouts \"we will never surrender\" loudly.", &ev),
            ["we will never surrender"]
        );
        // An ellipsis splits a quote; each part is checked.
        assert!(missing_quotes("\"I can't leave ... not tonight\"", &ev).is_empty());
        assert_eq!(
            missing_quotes("\"I can't leave ... not ever\"", &ev),
            ["I can't leave ... not ever"]
        );
        assert_eq!(missing_quotes("\"one two three\"", &[]), ["one two three"]);
    }

    #[test]
    fn numbers_without_support_are_reported_once() {
        let ev = ["Scene 4 has 12 people and 3 dogs."];
        assert!(unsupported_numbers("There are 12 people, 3 dogs and 1 cat.", &ev).is_empty());
        assert_eq!(
            unsupported_numbers("About 40 people, 40 dogs, 12 cats, 2012.", &ev),
            ["40", "2012"]
        );
        assert!(unsupported_numbers("No digits here.", &ev).is_empty());
    }

    #[test]
    fn relationship_words_need_a_source() {
        let ev = ["Suresh is Lakshmi's brother. Padma, the neighbours, wait."];
        assert!(
            unsupported_relationship_words("Her Brother Suresh and the neighbour Padma.", &ev)
                .is_empty()
        );
        assert_eq!(
            unsupported_relationship_words(
                "Her sister and her husband, and a rival. The sisters argue.",
                &ev
            ),
            ["sister", "husband", "rival"]
        );
        // "best friend" needs the phrase; "friend" alone in the evidence is not enough.
        assert_eq!(
            unsupported_relationship_words("her best friend", &["a friend of hers"]),
            ["best friend"]
        );
        assert!(unsupported_relationship_words("her best friend", &["her best friend"]).is_empty());
        // Whole words only.
        assert!(unsupported_relationship_words("The sonnet and the bossa nova.", &[]).is_empty());
    }

    #[test]
    fn repeated_ngrams_are_found() {
        let loop_text = "The tower hums. The tower hums! the Tower hums, and Lakshmi climbs.";
        assert_eq!(repeated_ngram(loop_text, 3), Some("the tower hums".into()));
        assert_eq!(
            repeated_ngram("one two one two one two", 2),
            Some("one two".into())
        );
        assert_eq!(
            repeated_ngram("The tower hums twice. The tower hums.", 3),
            None
        );
        assert_eq!(repeated_ngram("short", 4), None);
        assert_eq!(repeated_ngram("a a a a", 0), None);
    }
}
