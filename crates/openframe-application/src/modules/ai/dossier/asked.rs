//! What the person asked, read from the words they typed. A value is demanded of an answer only when
//! the question asks for it, and a list row is demanded when it holds the words of the question. No
//! model takes part: this is plain word matching on the message and the row.

use super::Fact;
use crate::modules::ai::index::normalize;
use crate::modules::ai::pointer;

/// Fields that describe a row. They are demanded only when the question says so: "what time is the
/// call on Day 4" does not ask whether the sheet is a draft.
const ATTRIBUTES: &[&str] = &["status", "revision", "type", "category", "department"];

/// Words that ask for a status without saying "status".
const STATUS_WORDS: &[&str] = &[
    "status",
    "state",
    "draft",
    "final",
    "stale",
    "confirmed",
    "confirm",
    "shortlisted",
    "idea",
    "rejected",
    "backup",
    "open",
    "done",
    "finished",
    "completed",
    "pending",
    "approved",
];

/// Words that say nothing about which row is meant.
const ROW_FILLER: &[&str] = &[
    "anything",
    "something",
    "saved",
    "save",
    "yet",
    "still",
    "got",
    "kept",
    "mention",
    "mentions",
    "exist",
    "exists",
    "any",
    "one",
    "ones",
    "thing",
    "things",
    "kind",
    "kinds",
    "type",
    "types",
    "whats",
    "whos",
    "not",
    "no",
    "next",
    "now",
    "there",
];

/// The words of a question that can pick a row: its content words without the filler and without the
/// words that only name the list (`plain`, "task" for the tasks).
pub(crate) fn row_words(message: &str, plain: &[&str]) -> Vec<String> {
    let plain: Vec<String> = plain.iter().flat_map(|w| pointer::words(w)).collect();
    pointer::content_words(message)
        .into_iter()
        .filter(|w| !ROW_FILLER.contains(&w.as_str()) && !plain.contains(w))
        .collect()
}

/// The rows that hold the most words of the question, at most `max` of them. None when no row holds a
/// word, or when so many tie that the words do not pick anything.
pub(crate) fn best_rows(words: &[String], rows: &[String], max: usize) -> Vec<usize> {
    if words.is_empty() {
        return Vec::new();
    }
    let held: Vec<usize> = rows
        .iter()
        .map(|row| {
            let row = pointer::words(row);
            words.iter().filter(|w| row.contains(w)).count()
        })
        .collect();
    let best = held.iter().copied().max().unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    let hits: Vec<usize> = (0..rows.len()).filter(|i| held[*i] == best).collect();
    if hits.len() > max { Vec::new() } else { hits }
}

/// The person named this row: at least half of the words of its title are in the message, so saying
/// the title back adds nothing.
pub(crate) fn names_row(message: &str, title: &str) -> bool {
    let typed = pointer::words(message);
    let title = pointer::content_words(title);
    !title.is_empty() && title.iter().filter(|w| typed.contains(w)).count() * 2 >= title.len()
}

/// The question asks for more than `plain` and the names it holds: "the scene where Jose confesses"
/// is not asking for every scene of Jose.
pub(crate) fn asks_more_than(message: &str, names: &[String], plain: &[&str]) -> bool {
    let names: Vec<String> = names.iter().flat_map(|n| pointer::words(n)).collect();
    row_words(message, plain)
        .iter()
        .any(|w| !names.contains(w))
}

/// The question asks for a status, in any of the words for one.
pub(crate) fn status_asked(message: &str) -> bool {
    let typed = pointer::words(message);
    STATUS_WORDS
        .iter()
        .flat_map(|w| pointer::words(w))
        .any(|w| typed.contains(&w))
}

/// The name of the field that holds a value: the field that is the value, else the first that holds it.
fn field_of<'a>(f: &'a Fact, value: &str) -> Option<&'a str> {
    let v = value.to_lowercase();
    f.fields
        .iter()
        .find(|(_, x)| x.to_lowercase() == v)
        .or_else(|| f.fields.iter().find(|(_, x)| x.to_lowercase().contains(&v)))
        .map(|(n, _)| n.as_str())
}

/// One required value, tested by the field that holds it. An attribute needs its own word in the
/// question. A name the person typed themselves is not news, so it is not demanded back; a bare number
/// is, because "how many scenes on day 4" may have the answer 4.
pub(crate) fn value_is_asked(f: &Fact, value: &str, message: &str) -> bool {
    let attribute = field_of(f, value).is_some_and(|n| ATTRIBUTES.contains(&n));
    if attribute {
        let typed = pointer::words(message);
        let names_field = field_of(f, value)
            .is_some_and(|n| pointer::words(n).iter().any(|w| typed.contains(w)));
        let names_value = pointer::words(value).iter().any(|w| typed.contains(w));
        return names_field
            || names_value
            || (field_of(f, value) == Some("status") && status_asked(message));
    }
    let v = normalize(value);
    let digits_only = v.chars().all(|c| c.is_ascii_digit() || c == ' ');
    // "Is it confirmed" asks for the status, so saying the word back is the answer.
    let a_status = STATUS_WORDS.contains(&v.as_str());
    digits_only || a_status || !format!(" {} ", normalize(message)).contains(&format!(" {v} "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn the_row_that_holds_the_most_asked_words_is_picked() {
        let rows = s(&["Crane up the mast", "Crane down", "Static wide"]);
        let words = row_words("Which scene has a crane up shot?", &["shot", "scene"]);
        assert_eq!(best_rows(&words, &rows, 3), vec![0]);
    }

    #[test]
    fn no_row_is_picked_when_too_many_tie_or_none_holds_a_word() {
        let rows = s(&["boat one", "boat two", "boat three"]);
        assert!(best_rows(&s(&["boat"]), &rows, 2).is_empty());
        assert!(best_rows(&s(&["car"]), &rows, 3).is_empty());
        assert!(best_rows(&[], &rows, 3).is_empty());
    }

    #[test]
    fn the_words_that_name_the_list_pick_no_row() {
        assert!(row_words("What is in the vault?", &["vault"]).is_empty());
        assert_eq!(
            row_words("anything saved about shortwave?", &["vault"]),
            s(&["shortwave"])
        );
    }

    #[test]
    fn a_row_named_by_most_of_its_title_is_named() {
        assert!(names_row(
            "Is the Lighthouse Road recce done?",
            "Recce Lighthouse Road for scene 21"
        ));
        assert!(!names_row(
            "Is there an open task about permits?",
            "Confirm the offshore permit with the harbour master"
        ));
    }

    #[test]
    fn a_status_is_demanded_only_when_a_word_asks_for_it() {
        let sheet = Fact::new("Call sheet", "c").f("day", "Day 4").f("status", "Draft");
        assert!(!value_is_asked(&sheet, "Draft", "what time is the call on day 4"));
        assert!(value_is_asked(&sheet, "Draft", "what is the status of day 4"));
        assert!(value_is_asked(&sheet, "Draft", "is day 4 final"));
        assert!(value_is_asked(&sheet, "Draft", "is day 4 a draft"));
    }

    #[test]
    fn a_name_the_person_typed_is_not_demanded_back_and_a_number_always_is() {
        let f = Fact::new("Note", "n").f("title", "Permit status").f("count", "4");
        assert!(!value_is_asked(&f, "Permit status", "what does the permit status note say"));
        assert!(value_is_asked(&f, "Permit status", "which note is about the beach"));
        assert!(value_is_asked(&f, "4", "how many scenes are on day 4"));
        assert!(value_is_asked(&f, "Confirmed", "is that location confirmed"));
    }

    #[test]
    fn the_scene_where_someone_does_a_thing_asks_for_more_than_their_scenes() {
        let plain = &["scene", "appear", "speak", "day"];
        let names = s(&["Jose"]);
        assert!(asks_more_than("In which scene does Jose confess?", &names, plain));
        assert!(!asks_more_than("Which scenes does Jose appear in?", &names, plain));
    }
}
