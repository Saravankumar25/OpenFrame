//! Domains: where the facts of a message live. Found by code from the resolved entities and topic
//! words; one message can touch several.

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
