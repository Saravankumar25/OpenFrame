//! Small text helpers for the fetchers: the message as whole words, clipping, money, dates.

use crate::modules::ai::index::normalize;

/// The message as lower case words. `has` matches whole words and phrases, so "act" does not match
/// "actor".
pub struct Msg {
    pub norm: String,
    padded: String,
    /// The words the person typed, without a section's topic words.
    typed: String,
}

impl Msg {
    pub fn new(s: &str) -> Self {
        let norm = normalize(s);
        let padded = format!(" {norm} ");
        Self {
            typed: norm.clone(),
            norm,
            padded,
        }
    }

    /// The message with the words of a section the person picked, so a fetcher that opens on a
    /// topic word reads its area whatever the question says. [`Msg::keywords`] stays with the words
    /// the person typed: a topic word never becomes a search term.
    pub fn with_topic(s: &str, topic: &str) -> Self {
        let typed = normalize(s);
        let norm = normalize(&format!("{s} {topic}"));
        let padded = format!(" {norm} ");
        Self {
            typed,
            norm,
            padded,
        }
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

    /// The same message with some phrases taken out: "call sheet" must not read as "call".
    pub fn without(&self, phrases: &[&str]) -> Msg {
        let mut t = format!(" {} ", self.norm);
        for p in phrases {
            t = t.replace(&format!(" {p} "), " ");
        }
        Msg::new(t.trim())
    }

    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.norm.split(' ').filter(|w| !w.is_empty())
    }

    /// Words that carry a topic: four letters or more and not a common word.
    pub fn keywords(&self) -> Vec<String> {
        self.typed
            .split(' ')
            .filter(|w| !w.is_empty())
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

const MONTHS: [&str; 12] = [
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
];

/// The dates a message names, as (month, day): "5 March", "March 5th", "2027-03-05". No year is read:
/// a schedule covers one shoot.
pub fn dates_in(message: &str) -> Vec<(u32, u32)> {
    let month_of = |w: &str| -> Option<u32> {
        let w = w.to_lowercase();
        MONTHS
            .iter()
            .position(|m| *m == w || m[..3] == w)
            .map(|i| i as u32 + 1)
    };
    let day_of = |w: &str| -> Option<u32> {
        let t = w.trim_end_matches(|c: char| c.is_alphabetic());
        t.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
    };
    let mut out = Vec::new();
    let tokens: Vec<&str> = message
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | '?' | '.' | '!' | ';' | ':' | '(' | ')')
        })
        .filter(|t| !t.is_empty())
        .collect();
    for t in &tokens {
        let parts: Vec<&str> = t.split('-').collect();
        if let [y, m, d] = parts[..]
            && y.len() == 4
            && let (Ok(m), Ok(d)) = (m.parse::<u32>(), d.parse::<u32>())
        {
            out.push((m, d));
        }
    }
    for pair in tokens.windows(2) {
        if let (Some(d), Some(m)) = (day_of(pair[0]), month_of(pair[1])) {
            out.push((m, d));
        } else if let (Some(m), Some(d)) = (month_of(pair[0]), day_of(pair[1])) {
            out.push((m, d));
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// A `2027-03-05` date falls on a (month, day) the message named.
pub fn date_is(iso: &str, (month, day): (u32, u32)) -> bool {
    iso.ends_with(&format!("-{month:02}-{day:02}"))
}

/// A (month, day) falls inside a window of two `2027-03-05` dates.
pub fn date_within(start: &str, end: &str, (month, day): (u32, u32)) -> bool {
    let at = |year: &str| format!("{year}-{month:02}-{day:02}");
    let year = start.split('-').next().unwrap_or_default();
    let d = at(year);
    start <= d.as_str() && d.as_str() <= end
}

/// `2027-03-12` from a unix time in milliseconds.
pub fn date_of(ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .map(|d| format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_read_in_the_ways_a_person_writes_them() {
        assert_eq!(dates_in("what is due on 5 March"), vec![(3, 5)]);
        assert_eq!(dates_in("who is free on March 8th?"), vec![(3, 8)]);
        assert_eq!(dates_in("2027-03-06 please"), vec![(3, 6)]);
        assert!(dates_in("may I see day 4").is_empty());
        assert!(dates_in("the 5 marching bands").is_empty());
    }

    #[test]
    fn a_date_falls_on_a_day_or_inside_a_window() {
        assert!(date_is("2027-03-05", (3, 5)));
        assert!(!date_is("2027-03-15", (3, 5)));
        assert!(date_within("2027-03-04", "2027-03-08", (3, 5)));
        assert!(!date_within("2027-03-06", "2027-03-08", (3, 5)));
    }

    #[test]
    fn a_phrase_taken_out_no_longer_counts() {
        let m = Msg::new("the call sheet for day 4");
        assert!(m.has("call"));
        assert!(!m.without(&["call sheet"]).has("call"));
    }
}
