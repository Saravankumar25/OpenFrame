//! Domains: where the facts of a message live. Found by code from the resolved entities and topic
//! words; one message can touch several.
//!
//! With sections picked (ADR-0041) the person is the router and topic words route nothing. They are
//! only a hint for the pointer: [`leaf_hints`] says which sections a message's words point at. Its
//! table holds fewer words than the domain table on purpose: "planned", "when", "timeline" and a bare
//! scene number pointed at the wrong place, so they are gone or narrowed.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::AnswerType;
use crate::modules::ai::index::{How, Kind, Resolved};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Story,
    Screenplay,
    Breakdown,
    People,
    Production,
    Schedule,
    Budget,
    Visual,
    Files,
    Settings,
    Product,
}

impl Domain {
    pub fn code(self) -> &'static str {
        match self {
            Domain::Story => "story",
            Domain::Screenplay => "screenplay",
            Domain::Breakdown => "breakdown",
            Domain::People => "people",
            Domain::Production => "production",
            Domain::Schedule => "schedule",
            Domain::Budget => "budget",
            Domain::Visual => "visual",
            Domain::Files => "files",
            Domain::Settings => "settings",
            Domain::Product => "product",
        }
    }
}

/// Topic words per domain, in the order the domains are listed.
static TOPICS: LazyLock<Vec<(Domain, Regex)>> = LazyLock::new(|| {
    let t = |d, p: &str| {
        (
            d,
            Regex::new(&format!("(?i)\\b(?:{p})\\b")).expect("valid pattern"),
        )
    };
    vec![
        t(
            Domain::Story,
            r"story|plot|characters?|relationships?|related|bond|ending|theme|arc|acts?|beats?|twist|villain|hero|rivalry|protagonist|antagonist",
        ),
        t(
            Domain::Screenplay,
            r"scenes?|dialogue|dialog|draft|drafts|screenplay|screenplays|script|pages?|heading",
        ),
        t(
            Domain::Breakdown,
            r"props?|wardrobe|costumes?|vehicles?|vfx|makeup|sound|animals?|breakdown|stunts?|extras",
        ),
        t(
            Domain::People,
            r"cast|crew|actors?|actress|phone|email|e-mail|contact|who plays|plays|playing|director|producer|availability|available",
        ),
        t(
            Domain::Production,
            r"production|producer|permits?|equipment|logistics|tasks?|call ?sheets?|catering|transport|locations?|unit",
        ),
        t(
            Domain::Schedule,
            r"schedule|scheduled|which day|what day|shoot days?|shooting days?|shoot|call time|day \d+|calendar|timeline|dates?|when",
        ),
        t(
            Domain::Budget,
            r"budget|budgets|costs?|spend|spent|paid|pay|price|expenses?|investment|allocated|planned|actual|money|rupees|fees?|estimate",
        ),
        t(
            Domain::Visual,
            r"story ?boards?|shots|shot list|visuals?|images?|photos?|poster|palette|mood ?boards?|references?|look ?book|idea vault|vault|png|jpe?g",
        ),
        t(
            Domain::Files,
            r"files?|attachments?|pdf|documents?|folders?|exports?",
        ),
        t(
            Domain::Settings,
            r"settings?|preferences?|theme|language|password|backups?|appearance|dark mode|font|shortcuts?|notifications?",
        ),
        t(
            Domain::Product,
            r"how do i|how to|feature|app|openframe|trial|licen[cs]e|update|install|offline|version|button|menu",
        ),
    ]
});

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
            r"phone|e-?mail|mobile|whatsapp|contact|available|unavailable|availability",
            &[
                "production/cast-crew",
                "production/locations",
                "callsheets/callsheets",
            ],
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

/// The domains of a message, in the order of [`Domain`].
pub(super) fn find(message: &str, r: &Resolved, answer_type: AnswerType) -> Vec<Domain> {
    let mut out: Vec<Domain> = TOPICS
        .iter()
        .filter(|(_, re)| re.is_match(message))
        .map(|(d, _)| *d)
        .collect();
    let mut add = |d: Domain| {
        if !out.contains(&d) {
            out.push(d);
        }
    };
    if !r.characters.is_empty() || !r.acts.is_empty() {
        add(Domain::Story);
    }
    let named_screenplay = r
        .screenplays
        .iter()
        .any(|h| matches!(h.how, How::Exact | How::Alias | How::Fuzzy(_)));
    if named_screenplay || !r.scenes.is_empty() {
        add(Domain::Screenplay);
    }
    if !r.persons.is_empty() {
        add(Domain::People);
    }
    if !r.locations.is_empty() {
        add(Domain::Production);
    }
    if !r.days.is_empty() {
        add(Domain::Schedule);
    }
    if !r.budget_lines.is_empty() {
        add(Domain::Budget);
    }
    if !r.files.is_empty() {
        add(Domain::Files);
    }
    for (kind, _) in &r.others {
        match kind {
            Kind::Card => add(Domain::Story),
            Kind::VaultItem => add(Domain::Visual),
            Kind::CallSheet => add(Domain::Production),
            _ => {}
        }
    }
    if answer_type == AnswerType::Help {
        add(Domain::Product);
    }
    out.sort();
    out
}
