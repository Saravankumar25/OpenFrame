//! Deterministic checks of AI answers (ADR-0028 decisions 1, 5 and 9).
//!
//! Ordinary code, never a model: the offline evaluation harness (`openframe-ai-eval`) scores gold
//! cases with these functions, and the runtime verifier (Windows phase W6) uses the same ones, so the
//! development scores and the checks people meet in the app always agree.

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
}
