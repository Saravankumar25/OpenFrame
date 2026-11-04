//! Topic words. They route nothing: the person picks where to look (ADR-0041). The pointer asks them
//! which section a message that went unanswered may belong to, and code inside a picked section asks
//! them whether a message names another topic besides what it asked. The table holds fewer words than
//! the routing table it replaced on purpose: "planned", "when", "timeline" and a bare scene number
//! pointed at the wrong place, so they are gone or narrowed.

use std::sync::LazyLock;

use regex::Regex;

/// A topic word of a message and the sections it points at, the first one being where the pointer
/// sends the person. Any of them being picked means the pick covers that topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafHint {
    pub word: String,
    pub leaves: &'static [&'static str],
}

/// The topic words, most specific first. A longer phrase takes its words with it: "crew call" is
/// about call sheets, and its "crew" is not read again.
static HINTS: LazyLock<Vec<(Regex, &'static [&'static str])>> = LazyLock::new(|| {
    let t = |p: &str, leaves: &'static [&'static str]| {
        (
            Regex::new(&format!("(?i)\\b(?:{p})\\b")).expect("valid pattern"),
            leaves,
        )
    };
    vec![
        t(r"prop report|reports?", &["production/reports"]),
        t(
            r"call ?sheets?|crew calls?|cast calls?|call times?",
            &["callsheets/callsheets"],
        ),
        t(
            r"day out of days|dood|work days?",
            &["production/day-out-of-days"],
        ),
        t(r"daily view", &["production/daily-view"]),
        t(r"mood ?boards?", &["production/moodboards"]),
        t(r"storyboards?|panels?", &["production/storyboards"]),
        t(r"shot ?lists?|shots|lens(?:es)?", &["production/shots"]),
        t(r"story page|free page", &["story/page"]),
        t(r"story days?|story timeline|timeline", &["story/timeline"]),
        t(r"episodes?|seasons?", &["story/episodes"]),
        t(
            r"beat ?board|story board|beats?|cards?|acts?|outline",
            &["story/beat-board", "story/outline"],
        ),
        t(
            r"breakdown",
            &["breakdown/breakdown", "production/overview"],
        ),
        t(
            r"props?|wardrobe|costumes?|wears?|wearing|makeup|stunts?|vfx|special effects|set dressing|extras",
            &[
                "breakdown/breakdown",
                "production/catalog",
                "production/reports",
            ],
        ),
        t(r"catalog(?:ue)?", &["production/catalog"]),
        t(r"sides", &["production/sides"]),
        t(
            r"schedule[sd]?|unscheduled|shoot(?:ing)? days?|shoot dates?|which day|what day|day \d+|the shoot|wrap",
            &[
                "production/schedule",
                "production/daily-view",
                "production/day-out-of-days",
                "callsheets/callsheets",
            ],
        ),
        t(
            r"budgets?|how much|costs?|spend|spent|contingency|expenses?|money|rupees|paid|price|planned total",
            &["production/budget"],
        ),
        t(r"fees?", &["production/budget", "production/locations"]),
        t(r"locations?|address(?:es)?", &["production/locations"]),
        t(
            r"phone|e-?mail|mobile|whatsapp|contact",
            &[
                "production/cast-crew",
                "production/locations",
                "callsheets/callsheets",
            ],
        ),
        // A call sheet lists who is called, not when anyone is free.
        t(
            r"available|unavailable|availability",
            &["production/cast-crew", "production/locations"],
        ),
        t(
            r"who plays|plays|played by|playing|actors?|actress|cast|crew|director|producer|gaffer|cinematographer",
            &["production/cast-crew"],
        ),
        t(
            r"relationships?|protagonist|antagonist|villain|hero",
            &["story/characters"],
        ),
        t(r"characters?", &["story/characters", "screenplay/scenes"]),
        t(
            r"what happens in (?:scene \d+|this scene|that scene)|which scenes|how many scenes|scene text|dialogue|screenplays?|scripts?|drafts?",
            &["screenplay/scenes"],
        ),
        t(r"idea vault|vault", &["vault/project", "vault/global"]),
        t(
            r"files?|attachments?|pdfs?|folders?|documents?",
            &["files/files"],
        ),
        t(r"tasks?|to ?dos?", &["notes/tasks"]),
        t(r"notes?", &["notes/notes"]),
        t(r"members|project settings|preferences", &["help/settings"]),
        t(
            r"story|plot",
            &[
                "story/beat-board",
                "story/outline",
                "story/page",
                "story/characters",
                "screenplay/scenes",
            ],
        ),
    ]
});

/// The sections the words of a text point at, in the order of the table.
pub fn leaf_hints(text: &str) -> Vec<LeafHint> {
    let mut rest = text.to_string();
    let mut out = Vec::new();
    for (re, leaves) in HINTS.iter() {
        let mut word: Option<String> = None;
        let taken = re
            .replace_all(&rest, |c: &regex::Captures<'_>| {
                word.get_or_insert_with(|| c[0].to_lowercase());
                " \u{2022} ".to_string()
            })
            .into_owned();
        if let Some(word) = word {
            out.push(LeafHint { word, leaves });
            rest = taken;
        }
    }
    out
}
