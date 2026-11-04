//! The one table of domain fetchers (spec section 14, "everything registers into one table").
//! Adding an area is one entry here plus its test.

use super::fetch;
use super::words::Msg;
use super::{Fact, FetchCtx};
use crate::modules::ai::index::{Kind, Resolved};
use crate::modules::ai::turn::{AnswerType, Domain, Group, TurnRead};

pub type FetchFn = fn(&FetchCtx<'_>, &TurnRead, &Resolved) -> Vec<Fact>;

/// One domain's way of reading the project.
pub struct Fetcher {
    pub domain: Domain,
    /// The answer groups it serves.
    pub groups: &'static [Group],
    /// The most tokens its facts may take before the lowest priority ones are cut.
    pub cap_tokens: usize,
    /// What it reads, for the documentation and the coverage test.
    pub source: &'static str,
    pub run: FetchFn,
}

use Group::{Brainstorm, Facts, Help, Narrative, Navigate};

/// Story, screenplay, breakdown, people, production (locations), schedule, budget, visual, files,
/// settings and product help.
pub const FETCHERS: &[Fetcher] = &[
    Fetcher {
        domain: Domain::Story,
        groups: &[Facts, Narrative, Brainstorm, Navigate],
        cap_tokens: 700,
        source: "story_character, story_character_relationship, story_act, story_scene_card, project",
        run: fetch::story::run,
    },
    Fetcher {
        domain: Domain::Screenplay,
        groups: &[Facts, Narrative, Brainstorm, Navigate],
        cap_tokens: 1_100,
        source: "screenplay, screenplay_scene, screenplay_element (queries::scenes, character_cues, scene_text)",
        run: fetch::screenplay::run,
    },
    Fetcher {
        domain: Domain::Breakdown,
        groups: &[Facts, Narrative, Brainstorm],
        cap_tokens: 600,
        source: "breakdown_element, production_source, catalog_item",
        run: fetch::breakdown::run,
    },
    Fetcher {
        domain: Domain::People,
        groups: &[Facts, Narrative, Brainstorm],
        cap_tokens: 600,
        source: "cast_member, crew_member, availability_window (production::availability)",
        run: fetch::people::run,
    },
    Fetcher {
        domain: Domain::Production,
        groups: &[Facts, Narrative, Brainstorm],
        cap_tokens: 700,
        source: "location, catalog_item, task, project_note, call_sheet",
        run: fetch::production::run,
    },
    Fetcher {
        domain: Domain::Schedule,
        groups: &[Facts, Narrative, Navigate],
        cap_tokens: 900,
        source: "shooting_schedule, shooting_day, schedule_strip, call_sheet (schedule::board)",
        run: fetch::schedule::run,
    },
    Fetcher {
        domain: Domain::Budget,
        groups: &[Facts, Narrative],
        cap_tokens: 700,
        source: "budget_snapshot, budget_line (exports::money)",
        run: fetch::budget::run,
    },
    Fetcher {
        domain: Domain::Visual,
        groups: &[Facts, Narrative, Brainstorm, Navigate],
        cap_tokens: 600,
        source: "shot, storyboard, storyboard_panel, moodboard, moodboard_item, vault_item",
        run: fetch::visual::run,
    },
    Fetcher {
        domain: Domain::Files,
        groups: &[Facts, Narrative],
        cap_tokens: 500,
        source: "project_file, project_file_folder, project_note",
        run: fetch::files::run,
    },
    Fetcher {
        domain: Domain::Settings,
        groups: &[Facts, Help],
        cap_tokens: 400,
        source: "project, project_member, appearance state, spelling settings",
        run: fetch::settings::run,
    },
    Fetcher {
        domain: Domain::Product,
        groups: &[Help, Facts],
        cap_tokens: 900,
        source: "modules/ai/knowledge.md (knowledge::relevant)",
        run: fetch::product::run,
    },
];

/// Chat and change requests need no facts; the rest read the project.
pub fn wants_facts(group: Group) -> bool {
    !matches!(group, Group::Chat | Group::Change)
}

/// The fetchers to run, in table order. Besides the domains the message touched, the group adds
/// the ones its plan needs (spec 5.4 table): a scene to explain brings its breakdown and its
/// shooting day; a relation brings the story and the scenes; a brainstorm brings the story around
/// the scene.
pub fn plan(read: &TurnRead, r: &Resolved, message: &str) -> Vec<&'static Fetcher> {
    let mut domains: Vec<Domain> = read.domains.clone();
    let mut add = |d: Domain| {
        if !domains.contains(&d) {
            domains.push(d);
        }
    };
    let scene = !r.scenes.is_empty();
    let cards = r.others.iter().any(|(k, _)| *k == Kind::Card);
    match read.group {
        Group::Narrative => {
            if scene {
                add(Domain::Screenplay);
                add(Domain::Breakdown);
                add(Domain::Schedule);
            }
            if !r.characters.is_empty() || !r.acts.is_empty() || cards || !scene {
                add(Domain::Story);
            }
            if !r.characters.is_empty() {
                add(Domain::Screenplay);
                add(Domain::People);
            }
        }
        Group::Brainstorm => {
            add(Domain::Screenplay);
            add(Domain::Story);
        }
        Group::Help => add(Domain::Product),
        _ => {}
    }
    // Words of the message that name a source the topic words of the read do not.
    let m = Msg::new(message);
    if fetch::people::asks_crew_job(&m) {
        add(Domain::People);
    }
    if fetch::screenplay::asks_speakers(&m) {
        add(Domain::Screenplay);
    }
    // A name the project does not hold: the Story says so.
    if fetch::story::unknown_name(message, r).is_some() {
        add(Domain::Story);
    }
    if !fetch::breakdown::categories_in(&m).is_empty()
        && (scene || read.answer_type == AnswerType::Count)
    {
        add(Domain::Breakdown);
    }
    if (scene || cards)
        && m.any(&[
            "where",
            "location",
            "locations",
            "filmed",
            "safety",
            "hospital",
            "permit",
            "permits",
        ])
    {
        add(Domain::Production);
    }
    if m.stem("permit") {
        add(Domain::Production);
    }
    if m.any(&[
        "rules",
        "rule",
        "safety",
        "policy",
        "restrictions",
        "requirements",
    ]) || m.stem("note")
    {
        add(Domain::Files);
    }
    if m.any(&[
        "problem",
        "problems",
        "clash",
        "clashes",
        "conflict",
        "conflicts",
        "warning",
        "warnings",
    ]) {
        add(Domain::Schedule);
    }
    // A name asked about in words the topic list does not know (another language): who plays it.
    if read.answer_type == AnswerType::Lookup
        && !r.characters.is_empty()
        && read
            .domains
            .iter()
            .all(|d| matches!(d, Domain::Story | Domain::Screenplay))
    {
        add(Domain::People);
    }
    FETCHERS
        .iter()
        .filter(|f| domains.contains(&f.domain) && f.groups.contains(&read.group))
        .collect()
}
