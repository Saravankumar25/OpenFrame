//! Small text helpers for the fetchers: the message as whole words, clipping, money, dates.

use crate::modules::ai::index::normalize;

/// The message as lower case words. `has` matches whole words and phrases, so "act" does not match
/// "actor".
pub struct Msg {
    pub norm: String,
    padded: String,
}

impl Msg {
    pub fn new(s: &str) -> Self {
        let norm = normalize(s);
        let padded = format!(" {norm} ");
        Self { norm, padded }
    }

    pub fn has(&self, phrase: &str) -> bool {
        self.padded.contains(&format!(" {phrase} "))
    }

    pub fn any(&self, phrases: &[&str]) -> bool {
        phrases.iter().any(|p| self.has(p))
    }

    /// A word that starts with `stem` ("character" matches "characters").
    pub fn stem(&self, stem: &str) -> bool {
        self.norm.split(' ').any(|w| w.starts_with(stem))
    }

    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.norm.split(' ').filter(|w| !w.is_empty())
    }

    /// Words that carry a topic: four letters or more and not a common word.
    pub fn keywords(&self) -> Vec<String> {
        self.words()
            .filter(|w| {
                w.chars().count() >= 3 && !COMMON.contains(w) && !w.chars().all(char::is_numeric)
            })
            .map(str::to_string)
            .collect()
    }
}

/// Words that name no topic. Not a language model: only the words that would match everything.
const COMMON: &[&str] = &[
    "what", "which", "when", "where", "who", "whom", "whose", "that", "this", "these", "those",
    "there", "with", "from", "have", "does", "about", "tell", "give", "show", "list", "many",
    "much", "please", "into", "than", "then", "them", "they", "their", "will", "would", "could",
    "should", "your", "mine", "also", "just", "like", "need", "needs", "make", "made", "some",
    "any", "all", "are", "was", "were", "been", "being", "for", "and", "the", "how", "can",
    "scene", "scenes", "film", "movie", "project", "explain", "describe", "know", "want", "said",
    "asked", "ask", "again", "only", "other", "each", "every", "more", "most", "less", "here",
    "over", "same", "very", "still", "know", "does", "doing", "day", "days", "who", "why", "you",
    "did", "has", "had", "not", "its", "our", "out", "one", "two", "get", "got", "let", "see",
    "say", "she", "her", "him", "his", "may", "now", "off", "per", "too", "way", "yes", "new",
    "old", "big", "bro", "use", "put", "run", "set", "ask", "act", "own", "try",
];

/// At most `n` characters, cut at a word end where possible, with an ellipsis when cut.
pub fn clip(s: &str, n: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= n {
        return s;
    }
    let cut: String = s.chars().take(n).collect();
    let cut = match cut.rfind(' ') {
        Some(i) if i > n / 2 => cut[..i].to_string(),
        _ => cut,
    };
    format!("{}\u{2026}", cut.trim_end_matches([',', ';', ':', ' ']))
}

/// Minutes after midnight as `HH:MM`.
pub fn clock(minutes: i64) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// A relationship type read with its person: "brother of" to "brother", "keeper of the station key
/// for" to "keeper of the station key".
pub fn relation_word(kind: &str) -> String {
    let k = kind.trim();
    for tail in [" of", " to", " for"] {
        if let Some(head) = k.strip_suffix(tail) {
            return head.trim().to_string();
        }
    }
    k.to_string()
}

/// "Varghese is brother of Mariamma." A type that starts with a verb ("works for") reads as one.
pub fn relation_sentence(from: &str, kind: &str, to: &str) -> String {
    let first = kind.split_whitespace().next().unwrap_or("");
    let verb = first.ends_with('s') && !first.ends_with("ss") && first.len() > 3;
    if verb {
        format!("{from} {kind} {to}.")
    } else {
        format!("{from} is {kind} {to}.")
    }
}

/// A cue or name written in capitals, as a name: "HARBOUR CLERK" to "Harbour Clerk".
pub fn title_case(s: &str) -> String {
    if s.chars().any(char::is_lowercase) {
        return s.to_string();
    }
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whole numbers in a list, as `3, 7, 12`.
pub fn numbers(v: impl IntoIterator<Item = u32>) -> String {
    v.into_iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// `2027-03-12` from a unix time in milliseconds.
pub fn date_of(ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .map(|d| format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day()))
        .unwrap_or_default()
}
