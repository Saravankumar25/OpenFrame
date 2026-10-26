//! A spelling-tolerant copy of the request, for routing only (AI harness rebuild plan, section 5.2).
//!
//! The direct routes and the lane rules are written against the right spelling ("How many
//! scenes ..."). A misspelt request ("How mny seens are there") missed them and went to the model.
//! `correct` returns a ROUTING copy: a word that is a near miss of exactly one word of the routing
//! vocabulary (the keywords the routes use plus the project's own names) becomes that word. The
//! original text is still what the model reads and what is stored. Code only; no model.
//!
//! Rules: a word of 4 to 7 letters within edit distance 1, or of 8 or more letters within
//! distance 2 (a swap of two neighbouring letters counts as 1), of exactly one vocabulary word
//! with the same first letter. Never changed: a vocabulary word, a common English word, a word
//! with a digit, a word in quotes, a word that is not plain Latin letters.

use std::collections::BTreeSet;

/// Words the routes (`commands`, `lanes`, `understand`) and the tool picker look for, four letters
/// or more. A test checks that each one still appears in the routing code, so the list cannot
/// drift away from the routes.
pub(super) const KEYWORDS: &[&str] = &[
    "scene",
    "scenes",
    "character",
    "characters",
    "speak",
    "speaks",
    "speaking",
    "dialogue",
    "dialogues",
    "lines",
    "location",
    "locations",
    "night",
    "count",
    "many",
    "which",
    "list",
    "shot",
    "shots",
    "schedule",
    "cast",
    "crew",
    "prop",
    "props",
    "task",
    "tasks",
    "note",
    "notes",
    "screenplay",
    "script",
    "draft",
    "drafts",
    "mention",
    "mentions",
    "total",
    "number",
    "describe",
    "summary",
    "summarize",
    "explain",
    "theme",
    "themes",
    "ideas",
    "relationship",
    "between",
    "wardrobe",
    "costume",
    "costumes",
    "budget",
    "vault",
    "status",
    "versions",
    "files",
    "contacts",
    "permits",
    "equipment",
    "storyboard",
    "moodboard",
    "breakdown",
    "compare",
    "critique",
    "opening",
    "ending",
    "whole",
    "entire",
    "overall",
    "rename",
    "search",
    "where",
    "interior",
    "exterior",
];

/// Words with a fixed misspelling that the edit-distance rule cannot reach (two edits in a short
/// word, or a letter missing in a word below four letters).
const MISSPELT: &[(&str, &str)] = &[
    ("seens", "scenes"),
    ("sceens", "scenes"),
    ("sceen", "scene"),
    ("scens", "scenes"),
    ("mny", "many"),
    ("manny", "many"),
    ("scne", "scene"),
    ("scnes", "scenes"),
];

/// Ordinary English words of 4 to 7 letters that sit one letter from a keyword. They are never
/// "corrected" ("witch" is not "which", "shoot" is not "shot").
pub(super) const COMMON: &[&str] = &[
    "about", "after", "again", "being", "below", "bring", "built", "called", "chart", "child",
    "clear", "close", "could", "crowd", "dance", "decide", "every", "first", "found", "from",
    "gist", "give", "given", "grant", "great", "group", "happen", "have", "heart", "horse",
    "house", "into", "join", "just", "keep", "kind", "know", "large", "later", "least", "leave",
    "less", "level", "light", "like", "lined", "link", "lost", "made", "make", "mean", "might",
    "mind", "mine", "miss", "more", "most", "move", "much", "must", "name", "near", "need", "next",
    "nice", "none", "noted", "only", "open", "other", "party", "place", "plain", "plant", "point",
    "price", "quiet", "quite", "right", "round", "same", "scale", "scare", "scent", "scone",
    "score", "scope", "scream", "sense", "shall", "shape", "share", "sharp", "shoot", "shop",
    "short", "shout", "show", "shows", "shown", "side", "since", "skill", "small", "some", "soon",
    "sort", "speed", "spoke", "spot", "start", "state", "still", "stone", "stood", "story", "such",
    "sure", "take", "talk", "team", "tell", "than", "that", "their", "them", "then", "there",
    "these", "they", "thing", "think", "this", "those", "three", "through", "tight", "time",
    "told", "took", "tone", "touch", "town", "treat", "tried", "true", "turn", "under", "until",
    "upon", "used", "very", "wait", "walk", "want", "watch", "water", "well", "went", "were",
    "what", "when", "whence", "while", "white", "whom", "whose", "wide", "will", "wish", "with",
    "witch", "woman", "women", "words", "work", "world", "would", "write", "wrong", "years",
    "young", "your",
];

/// Letters of the word, lower case; `None` unless it is plain ASCII letters.
fn plain_word(w: &str) -> Option<String> {
    w.chars()
        .all(|c| c.is_ascii_alphabetic())
        .then(|| w.to_ascii_lowercase())
}

/// Edit distance with a swap of two neighbours counted as one edit; stops past `max`.
pub(super) fn distance(a: &str, b: &str, max: usize) -> Option<usize> {
    let (a, b): (Vec<u8>, Vec<u8>) = (a.bytes().collect(), b.bytes().collect());
    if a.len().abs_diff(b.len()) > max {
        return None;
    }
    let mut prev2: Vec<usize> = vec![0; b.len() + 1];
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                cur[j] = cur[j].min(prev2[j - 2] + 1);
            }
        }
        prev2 = prev;
        prev = cur;
    }
    Some(prev[b.len()]).filter(|d| *d <= max)
}

/// The vocabulary word `word` is a near miss of: the closest one, when exactly one is closest.
fn nearest<'a>(word: &str, vocabulary: &'a BTreeSet<String>) -> Option<&'a str> {
    let n = word.len();
    let max = match n {
        4..=7 => 1,
        8.. => 2,
        _ => return None,
    };
    let first = word.as_bytes()[0];
    let mut found: Vec<(usize, &str)> = Vec::new();
    for v in vocabulary {
        if v.as_bytes()[0] != first {
            continue;
        }
        if let Some(d) = distance(word, v, max).filter(|d| *d > 0) {
            found.push((d, v));
        }
    }
    let closest = found.iter().map(|(d, _)| *d).min()?;
    match found
        .iter()
        .filter(|(d, _)| *d == closest)
        .collect::<Vec<_>>()[..]
    {
        [(_, w)] => Some(w),
        _ => None,
    }
}

/// A plain inflection of a vocabulary word ("scripts", "speaking") is a word, not a misspelling.
fn inflection(word: &str, vocabulary: &BTreeSet<String>) -> bool {
    const ENDINGS: [&str; 6] = ["s", "es", "ed", "ing", "er", "ers"];
    ENDINGS.iter().any(|e| {
        word.strip_suffix(e)
            .is_some_and(|stem| vocabulary.contains(stem))
    })
}

/// The replacement written the way the original was: UPPER, Capitalised or lower.
fn match_case(original: &str, word: &str) -> String {
    let mut chars = original.chars();
    let first_upper = chars.next().is_some_and(char::is_uppercase);
    let rest_upper = original.len() > 1 && original.chars().skip(1).all(char::is_uppercase);
    if first_upper && rest_upper {
        word.to_uppercase()
    } else if first_upper {
        let mut c = word.chars();
        c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default()
    } else {
        word.to_string()
    }
}

/// The routing copy of `text`. `names` are the project's own words (character and location names)
/// and join the keywords as vocabulary. Text with nothing to correct comes back unchanged.
pub(super) fn correct(text: &str, names: &[String]) -> String {
    let mut vocabulary: BTreeSet<String> = KEYWORDS.iter().map(|w| (*w).to_string()).collect();
    // A capitalised word is left alone when it is near a name: it may be a different person.
    let names: BTreeSet<String> = names
        .iter()
        .filter_map(|n| plain_word(n).filter(|w| w.len() >= 4))
        .filter(|w| !KEYWORDS.contains(&w.as_str()))
        .collect();
    vocabulary.extend(names.iter().cloned());
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(close) = quote {
            out.push(c);
            if c == close {
                quote = None;
            }
            i += 1;
            continue;
        }
        let opens = match c {
            '"' => Some('"'),
            '\u{201C}' => Some('\u{201D}'),
            '\u{2018}' => Some('\u{2019}'),
            // A single quote opens a quotation only at the start of a word and with a close ahead.
            '\'' if i == 0 || !chars[i - 1].is_alphanumeric() => Some('\''),
            _ => None,
        };
        if let Some(close) = opens
            && chars[i + 1..].iter().enumerate().any(|(k, x)| {
                *x == close && chars.get(i + 2 + k).is_none_or(|n| !n.is_alphanumeric())
            })
        {
            quote = Some(close);
            out.push(c);
            i += 1;
            continue;
        }
        if !c.is_alphanumeric() {
            out.push(c);
            i += 1;
            continue;
        }
        let start = i;
        // A contraction or possessive ("Devi's") is one token and is left alone.
        while i < chars.len()
            && (chars[i].is_alphanumeric()
                || (chars[i] == '\''
                    && chars[i - 1].is_alphabetic()
                    && chars.get(i + 1).is_some_and(|n| n.is_alphabetic())))
        {
            i += 1;
        }
        let token: String = chars[start..i].iter().collect();
        out.push_str(&fix(&token, &vocabulary, &names));
    }
    out
}

fn fix(token: &str, vocabulary: &BTreeSet<String>, names: &BTreeSet<String>) -> String {
    if token.contains('\'') {
        return token.to_string();
    }
    let Some(lower) = plain_word(token) else {
        return token.to_string();
    };
    if vocabulary.contains(&lower)
        || COMMON.contains(&lower.as_str())
        || inflection(&lower, vocabulary)
    {
        return token.to_string();
    }
    let target = MISSPELT
        .iter()
        .find(|(m, _)| *m == lower)
        .map(|(_, w)| *w)
        .or_else(|| nearest(&lower, vocabulary));
    match target {
        Some(w) if names.contains(w) && token.chars().next().is_some_and(char::is_uppercase) => {
            token.to_string()
        }
        Some(w) => match_case(token, w),
        None => token.to_string(),
    }
}

#[cfg(test)]
mod tests;
