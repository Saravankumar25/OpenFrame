//! Washing the text a model wrote (ADR-0028: the person never sees markdown or our own markup).
//!
//! Plain sentences: markdown marks go, an opening that only announces the answer goes, a reply cut
//! by the token limit ends at its last full sentence, and a claim that a change was made is replaced
//! by the truth (only an applied Change Set changes the project). [`tidy`] does the washing in order.

use std::sync::LazyLock;

use regex::Regex;

use super::sanitize;

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid pattern")
}

pub(crate) fn word_count(t: &str) -> usize {
    t.split_whitespace().count()
}

// ------------------------------------------------------------------ markdown

static RULE: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^[ \t]*(?:-{3,}|\*{3,}|_{3,})[ \t]*$"));
static MARKER: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?m)^[ \t]{0,3}(?:#{1,6}[ \t]+|>[ \t]?|[-*\x{2022}+][ \t]+|\d{1,2}[.)][ \t]+)")
});
static LINK: LazyLock<Regex> = LazyLock::new(|| re(r"\[([^\]\n]+)\]\([^)\n]*\)"));
static BOLD: LazyLock<Regex> = LazyLock::new(|| re(r"\*\*([^\n]+?)\*\*|__([^\n]+?)__"));
static ITALIC: LazyLock<Regex> =
    LazyLock::new(|| re(r"\*([^\s*][^*\n]*?)\*|\b_([^\s_][^_\n]*?)_\b"));
static CODE: LazyLock<Regex> = LazyLock::new(|| re(r"`+([^`\n]+)`+"));

/// Model-written text as plain sentences: no markdown emphasis, heading hashes, bullet or number
/// markers, quote marks, rules, code ticks or link syntax. Line breaks stay. A small model slips
/// into markdown even when told not to, and the chat shows plain text only.
pub fn plain_text(text: &str) -> String {
    sanitize::clean(&unwashed_text(text))
}

/// `plain_text` before the wash of our own markup: only the markdown is gone.
pub fn unwashed_text(text: &str) -> String {
    let t = RULE.replace_all(text, "");
    let t = MARKER.replace_all(&t, "");
    let t = LINK.replace_all(&t, "$1");
    let t = BOLD.replace_all(&t, "$1$2");
    let t = ITALIC.replace_all(&t, "$1$2");
    let t = CODE.replace_all(&t, "$1");
    t.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

// ------------------------------------------------------------------ sentences

/// Keeps the first `n` sentences of a written answer. The person asked for that length (or the
/// prompt did), and a small model often writes on; the cut is at a sentence end, never mid-word.
pub(crate) fn keep_sentences(text: String, n: usize) -> String {
    let mut count = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (i, &(at, ch)) in chars.iter().enumerate() {
        if matches!(ch, '.' | '!' | '?' | '\u{0964}' | '\u{3002}') {
            let next = chars.get(i + 1).map(|&(_, c)| c);
            if next.is_none_or(|c| c.is_whitespace()) {
                count += 1;
                if count == n {
                    return text[..at + ch.len_utf8()].trim_end().to_string();
                }
            }
        }
    }
    text
}

/// Drops whole sentences from the end until the text fits `max` words, keeping at least one.
pub(crate) fn fit_words(text: String, max: usize) -> String {
    let mut fit = keep_sentences(text.clone(), 1);
    for n in 2.. {
        let t = keep_sentences(text.clone(), n);
        if word_count(&t) > max {
            break;
        }
        if t == text {
            return text;
        }
        fit = t;
    }
    fit
}

/// A reply cut off by the token limit ends mid-sentence. Cut the last line back to its last full
/// sentence, or drop it when it has none and an earlier line remains.
pub(crate) fn drop_unfinished(text: String) -> String {
    let t = text.trim_end();
    let last_start = t.rfind('\n').map_or(0, |i| i + 1);
    let last = &t[last_start..];
    let ends = |s: &str| {
        s.ends_with([
            '.', '!', '?', ')', '"', '”', '’', ':', '\u{3002}', '\u{0964}',
        ])
    };
    if ends(last) || word_count(last) < 6 {
        return t.to_string();
    }
    match last.rfind(['.', '!', '?']) {
        Some(i) => format!("{}{}", &t[..last_start], &last[..=i]),
        None if last_start > 0 => t[..last_start].trim_end().to_string(),
        None => t.to_string(),
    }
}

// ------------------------------------------------------------------ announcing openings

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
    let sentences = answer
        .split(['.', '!', '?'])
        .filter(|s| !s.trim().is_empty())
        .count();
    answer.trim_end().ends_with('?') && sentences == 1 && hits * 2 > r.len()
}

/// A model reply as the answer text: washed, without an announcing opening, ended at a full
/// sentence. Empty when it only handed the request back.
pub(crate) fn tidy(raw: &str, request: &str) -> String {
    let plain = without_preamble(&plain_text(raw), request);
    let text = drop_unfinished(plain);
    if echoes_request(&text, request) {
        String::new()
    } else {
        text
    }
}

// ------------------------------------------------------------------ change claims

/// The reply after a request that proposed nothing must not say a change was made: only an
/// applied Change Set changes the project, and applying is always the person's own click.
/// A first-person claim of a change ("I renamed", "has been updated") is replaced by the truth.
pub fn without_false_change_claim(text: &str) -> String {
    if claims_a_change(text) {
        "I haven't changed anything. Ask me to prepare the change and you can review it before anything is applied.".to_string()
    } else {
        text.to_string()
    }
}

/// The text says a change was made.
pub(crate) fn claims_a_change(text: &str) -> bool {
    static CLAIM: LazyLock<Regex> = LazyLock::new(|| {
        re(
            r"(?i)\b(?:i(?:'ve|\s+have)?\s+(?:now\s+|successfully\s+)?(?:renamed|changed|updated|deleted|removed|added|created|applied|moved|edited|replaced|saved)|(?:has|have)\s+been\s+(?:successfully\s+)?(?:renamed|changed|updated|deleted|removed|added|created|applied|moved|edited|replaced|saved)|(?:changes?|updates?)\s+(?:were|was|is|are)\s+(?:successfully\s+)?applied|successfully\s+(?:renamed|changed|updated|deleted|removed|added|created|applied))\b",
        )
    });
    CLAIM.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_text_loses_markdown_but_keeps_its_words_and_line_breaks() {
        assert_eq!(
            plain_text(
                "## Summary\n**Devi** finds the *red* kite.\n- She mends it.\n* Kiran flies it.\n1. At dawn."
            ),
            "Summary\nDevi finds the red kite.\nShe mends it.\nKiran flies it.\nAt dawn."
        );
        assert_eq!(
            plain_text("__Bold__ and `code` and [a link](https://example.com).\n---\n> quoted"),
            "Bold and code and a link.\n\nquoted"
        );
        // Plain prose, snake_case, arithmetic and a year are left alone.
        let plain = "Scene 3 has 2*3 beats in act_one. 1984 was long ago.";
        assert_eq!(plain_text(plain), plain);
        assert_eq!(plain_text("  \n"), "");
    }

    #[test]
    fn written_answers_keep_the_asked_number_of_sentences() {
        let t = "Ravi waits. Arjun comes late! Does Meera know? She does.".to_string();
        assert_eq!(keep_sentences(t.clone(), 1), "Ravi waits.");
        assert_eq!(
            keep_sentences(t.clone(), 3),
            "Ravi waits. Arjun comes late! Does Meera know?"
        );
        assert_eq!(keep_sentences(t.clone(), 9), t);
    }

    #[test]
    fn default_length_drops_whole_sentences_to_fit() {
        let t = "One two three. Four five six. Seven eight nine.".to_string();
        assert_eq!(fit_words(t.clone(), 6), "One two three. Four five six.");
        assert_eq!(fit_words(t.clone(), 1), "One two three.");
        assert_eq!(fit_words(t.clone(), 99), t);
    }

    #[test]
    fn a_reply_cut_by_the_token_limit_ends_at_the_last_full_sentence() {
        assert_eq!(
            drop_unfinished("Ravi waits at the station. Then Arjun comes in and the".to_string()),
            "Ravi waits at the station."
        );
        // Nothing to cut back to on a one line reply: it stays.
        let one = "Ravi waits at the station for the late train".to_string();
        assert_eq!(drop_unfinished(one.clone()), one);
        // A short last line is left alone.
        assert_eq!(
            drop_unfinished("Ravi waits.\nYes".to_string()),
            "Ravi waits.\nYes"
        );
    }

    #[test]
    fn a_first_sentence_that_only_announces_the_answer_is_dropped() {
        assert_eq!(
            without_preamble(
                "Okay, let's dive into the scene. Ravi waits.",
                "explain the scene"
            ),
            "Ravi waits."
        );
        assert_eq!(
            without_preamble("Sure! Here you go. Ravi waits.", "x"),
            "Here you go. Ravi waits.",
            "a pure preamble goes, the rest stays"
        );
        // A sentence that carries a fact stays.
        let t = "Sure, Ravi waits at the station in scene 3.";
        assert_eq!(without_preamble(t, "where is Ravi"), t);
        // Never empties the answer.
        assert_eq!(without_preamble("Okay.", "x"), "Okay.");
    }

    #[test]
    fn an_answer_that_hands_the_request_back_is_empty() {
        assert_eq!(tidy("Explain scene 4.", "explain scene 4"), "");
        assert_eq!(
            tidy("What do you mean by explain scene 4?", "explain scene 4"),
            ""
        );
        assert_eq!(tidy("Ravi waits.", "explain scene 4"), "Ravi waits.");
    }

    #[test]
    fn a_reply_never_claims_a_change_that_was_not_applied() {
        assert!(
            without_false_change_claim("I've renamed the scene.").starts_with("I haven't changed")
        );
        assert!(
            without_false_change_claim("The change has been applied.")
                .starts_with("I haven't changed")
        );
        assert_eq!(without_false_change_claim("Ravi waits."), "Ravi waits.");
    }
}
