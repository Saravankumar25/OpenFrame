//! The section registry (ADR-0041; `docs/specs/ai-conversational-harness.md`).
//!
//! The person is the router: before asking they pick one to three leaves, and the assistant reads
//! only those. A leaf is one screen of the app. This table is hand written and is the one place that
//! says, for a screen, which fetchers read it, which kinds of fact may pass, what the text search may
//! look at and which names belong to it. The coverage tests at the end fail the build when a
//! navigation screen or an indexed entity type maps to no leaf or to several.
//!
//! No model decides anything here: the leaves come from the person, everything after them is code.

use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::dossier::fetch::{
    breakdown, briefing, budget, files, notes, people, plans, product, production, schedule,
    screenplay, settings, story, story_tabs, visual,
};
use super::dossier::{Fact, FetchFn, P_LOW};
use super::index::Kind;
use super::queries::{self, DraftRef, SceneRow};
use super::retrieval::{self, RetrievalRoute, RetrieveRequest};
use crate::core::{AppCore, ProjectSession};
use crate::modules::production::privacy::may_see_contacts;
use crate::modules::schedule::board;

/// The most leaves one question may read.
pub const MAX_PICKS: usize = 3;
/// How many text passages one leaf may add; the budget cuts them first.
const PASSAGES_PER_LEAF: usize = 4;
/// The most characters of a draft or call sheet name in the "Using:" line.
const MAX_LABEL_NAME: usize = 60;
/// Longest text of one passage.
const PASSAGE_CHARS: usize = 900;
/// The fixed reply to a request with no section. No model runs.
/// How a request that reads every section is labelled. A person never sees it: only proposals checked
/// again and the evaluation switch read every section.
pub const EVERY_SECTION_LABEL: &str = "Using: every section";

pub const PICK_A_SECTION: &str =
    "Pick a section first, then ask again. The section list is above the question box.";

/// What a leaf asks for after it is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionPicker")]
pub enum SectionPicker {
    /// Which screenplay, which draft (default current), which scene (All, or one).
    Screenplay,
    /// Which scene (All, or one).
    Scene,
    /// Which shooting day (All, or one).
    Day,
    /// Which shooting day or call sheet (All, or one).
    CallSheet,
}

/// Who may read the contact details of a leaf. The dossier guard hides them for everyone else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinRole {
    /// The leaf holds no contact details.
    Anyone,
    /// Contact details: Owner and Editor only.
    ContactDetails,
}

impl MinRole {
    pub fn allows_contacts(self, actor: &Actor) -> bool {
        match self {
            MinRole::Anyone => true,
            MinRole::ContactDetails => may_see_contacts(actor),
        }
    }
}

/// The `sub` part of a navigation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sub {
    Any,
    Absent,
    Is(&'static str),
}

/// A key the navigation target must have or must lack ("taskId" tells Tasks from Notes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Any,
    Has(&'static str),
    Lacks(&'static str),
}

/// One rule of a leaf's search filter, read against `search_doc.nav_json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavRule {
    pub workspace: &'static str,
    pub sub: Sub,
    pub key: Key,
}

const fn nav(workspace: &'static str) -> NavRule {
    NavRule {
        workspace,
        sub: Sub::Any,
        key: Key::Any,
    }
}

const fn nav_bare(workspace: &'static str) -> NavRule {
    NavRule {
        workspace,
        sub: Sub::Absent,
        key: Key::Any,
    }
}

const fn nav_sub(workspace: &'static str, sub: &'static str) -> NavRule {
    NavRule {
        workspace,
        sub: Sub::Is(sub),
        key: Key::Any,
    }
}

const fn nav_key(workspace: &'static str, key: Key) -> NavRule {
    NavRule {
        workspace,
        sub: Sub::Any,
        key,
    }
}

impl NavRule {
    pub fn matches(&self, nav: &Value) -> bool {
        if nav.get("workspace").and_then(Value::as_str) != Some(self.workspace) {
            return false;
        }
        let sub = nav.get("sub").and_then(Value::as_str);
        let sub_ok = match self.sub {
            Sub::Any => true,
            Sub::Absent => sub.is_none(),
            Sub::Is(s) => sub == Some(s),
        };
        let key_ok = match self.key {
            Key::Any => true,
            Key::Has(k) => nav.get(k).is_some(),
            Key::Lacks(k) => nav.get(k).is_none(),
        };
        sub_ok && key_ok
    }

    /// The same test in SQL over `d.nav_json`. Every value is a constant of this file.
    pub fn sql(&self) -> String {
        let mut clause = format!(
            "json_extract(d.nav_json, '$.workspace') = '{}'",
            self.workspace
        );
        match self.sub {
            Sub::Any => {}
            Sub::Absent => clause.push_str(" AND json_extract(d.nav_json, '$.sub') IS NULL"),
            Sub::Is(s) => {
                clause.push_str(&format!(" AND json_extract(d.nav_json, '$.sub') = '{s}'"));
            }
        }
        match self.key {
            Key::Any => {}
            Key::Has(k) => {
                clause.push_str(&format!(" AND json_type(d.nav_json, '$.{k}') IS NOT NULL"));
            }
            Key::Lacks(k) => {
                clause.push_str(&format!(" AND json_type(d.nav_json, '$.{k}') IS NULL"));
            }
        }
        format!("({clause})")
    }
}

/// One way a leaf reads the project.
#[derive(Clone, Copy)]
pub struct LeafFetcher {
    pub name: &'static str,
    pub run: FetchFn,
}

const fn f(name: &'static str, run: FetchFn) -> LeafFetcher {
    LeafFetcher { name, run }
}

/// One screen of the app the person can pick.
pub struct Leaf {
    /// Path form, stable: the evaluation set and the trace use it.
    pub id: &'static str,
    pub branch: &'static str,
    pub label: &'static str,
    /// How the "Using:" line names it.
    pub title: &'static str,
    pub aliases: &'static [&'static str],
    pub fetchers: &'static [LeafFetcher],
    /// The kinds of fact that may pass. The domain fetchers mix kinds, so what they return is
    /// filtered here.
    pub kinds: &'static [&'static str],
    /// A `Count` or a `Missing` statement passes only when it speaks of one of these words: both
    /// kinds are shared by every area.
    pub about: &'static [&'static str],
    /// The words that open this leaf's fetchers whatever the question says.
    pub topic: &'static str,
    /// Where text search may look. Empty: this leaf has no indexed text.
    pub search: &'static [NavRule],
    /// The `search_doc` and index entity types whose home is this leaf.
    pub homes: &'static [&'static str],
    /// The navigation screens this leaf stands for.
    pub screens: &'static [&'static str],
    /// The kinds of name this leaf can answer about.
    pub names: &'static [Kind],
    pub picker: Option<SectionPicker>,
    /// The leaf's share of the dossier budget, against the other picked leaves.
    pub weight: u32,
    pub min_role: MinRole,
    /// Used by the tests only.
    pub examples: &'static [&'static str],
}

impl std::fmt::Debug for Leaf {
    fn fmt(&self, fm: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fm.debug_struct("Leaf").field("id", &self.id).finish()
    }
}

impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Leaf {
    /// How the pointer and the Add button name it: "Production > Budget", or "Files" when the branch
    /// holds one leaf of the same name.
    pub fn path(&self) -> String {
        if self.branch == self.label {
            self.label.to_string()
        } else {
            format!("{} > {}", self.branch, self.label)
        }
    }

    /// May this fact pass for this leaf? Statements of what is missing, and the note on which
    /// screenplay was assumed, are not facts of an area: `Missing` and `Count` are checked by their
    /// words, `Assumed` only where a leaf can read scenes.
    pub fn admits(&self, fact: &Fact) -> bool {
        if fact.is_missing() {
            let said = fact.sentence.as_deref().unwrap_or_default().to_lowercase();
            return self.about.iter().any(|w| said.contains(w));
        }
        match fact.kind {
            "Assumed" => self.names.contains(&Kind::Scene),
            "Count" if self.kinds.contains(&"Count") => {
                let what = fact.field("what").unwrap_or_default().to_lowercase();
                self.about.iter().any(|w| what.contains(w))
            }
            kind => self.kinds.contains(&kind),
        }
    }

    pub fn takes(&self, kind: Kind) -> bool {
        self.names.contains(&kind)
    }
}

use Kind as K;
use MinRole::{Anyone, ContactDetails};
use SectionPicker as P;

/// Every leaf, in the order the picker shows them.
pub const LEAVES: &[Leaf] = &[
    Leaf {
        id: "vault/project",
        branch: "Idea Vault",
        label: "This Project",
        title: "Idea Vault (This Project)",
        aliases: &["project vault", "idea vault", "vault"],
        fetchers: &[f("visual", visual::run)],
        kinds: &["Idea Vault item", "Count"],
        about: &["vault"],
        topic: "idea vault",
        search: &[nav("vault")],
        homes: &["vault_item"],
        screens: &["vault"],
        names: &[K::VaultItem],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What is in the vault for this project?"],
    },
    Leaf {
        id: "vault/global",
        branch: "Idea Vault",
        label: "Global",
        title: "Idea Vault (Global)",
        aliases: &["global vault", "global idea vault"],
        fetchers: &[f("global vault", visual::global_vault)],
        kinds: &["Global Idea Vault item"],
        about: &["vault"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["vault/global"],
        names: &[],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What ideas do I keep across projects?"],
    },
    Leaf {
        id: "story/page",
        branch: "Story",
        label: "Story Page",
        title: "Story Page",
        aliases: &["story page", "free page"],
        fetchers: &[f("story page", story_tabs::page)],
        kinds: &["Story page"],
        about: &["story page"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["story/page"],
        names: &[],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["What did I write on the Story page about the ending?"],
    },
    Leaf {
        id: "story/beat-board",
        branch: "Story",
        label: "Beat Board",
        title: "Beat Board",
        aliases: &["board", "story board", "cards", "acts", "beats"],
        fetchers: &[f("story", story::run)],
        kinds: &["Act", "Card", "Count", "Project"],
        about: &["act", "card", "beat", "sequence"],
        topic: "beat board acts cards",
        search: &[nav_bare("story")],
        homes: &[
            "story_act",
            "story_sequence",
            "story_beat",
            "story_scene_card",
        ],
        screens: &["story", "story/board"],
        names: &[K::Act, K::Card],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["How many cards are in Act 2?"],
    },
    Leaf {
        id: "story/outline",
        branch: "Story",
        label: "Outline",
        title: "Outline",
        aliases: &["story outline"],
        fetchers: &[f("story", story::run)],
        kinds: &["Act", "Card", "Count", "Project"],
        about: &["act", "card", "beat", "sequence"],
        topic: "outline acts cards",
        search: &[nav_bare("story")],
        homes: &[],
        screens: &["story/outline"],
        names: &[K::Act, K::Card],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["Give me the outline of the first act."],
    },
    Leaf {
        id: "story/characters",
        branch: "Story",
        label: "Characters",
        title: "Characters",
        aliases: &["character", "relationships"],
        fetchers: &[f("story", story::run)],
        kinds: &["Character", "Relationship", "Count"],
        about: &["character"],
        topic: "characters",
        search: &[nav_sub("story", "characters")],
        homes: &["story_character"],
        screens: &["story/characters"],
        names: &[K::Character],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["Who is the antagonist?"],
    },
    Leaf {
        id: "story/timeline",
        branch: "Story",
        label: "Timeline",
        title: "Story Timeline",
        aliases: &["story days", "story timeline"],
        fetchers: &[f("story timeline", story_tabs::timeline)],
        kinds: &["Story day", "Count"],
        about: &["story day", "timeline"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["story/timeline"],
        names: &[K::Scene],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["Which scenes happen on the second story day?"],
    },
    Leaf {
        id: "story/episodes",
        branch: "Story",
        label: "Episodes",
        title: "Episodes",
        aliases: &["season", "seasons", "episode"],
        fetchers: &[f("episodes", story_tabs::episodes)],
        kinds: &["Episode", "Count"],
        about: &["episode"],
        topic: "",
        search: &[nav_sub("story", "episodes")],
        homes: &["episode"],
        screens: &["story/episodes"],
        names: &[],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["How many episodes are in season one?"],
    },
    Leaf {
        id: "screenplay/scenes",
        branch: "Screenplay",
        label: "Scenes",
        title: "Screenplay",
        aliases: &["screenplay", "script", "draft", "scene", "dialogue"],
        fetchers: &[
            f("screenplay", screenplay::run),
            f("briefing", briefing::scenes),
        ],
        kinds: &[
            "Scene",
            "Scene text",
            "Scenes",
            "Screenplay",
            "Speaking characters",
            "Appearances",
            "Together",
            "Comment",
            "Neighbour scene",
            "Scene locations",
            "Your private note",
            "Count",
        ],
        about: &[
            "scene",
            "screenplay",
            "script",
            "draft",
            "page",
            "dialogue",
            "line",
            "word",
        ],
        topic: "",
        search: &[nav("screenplay")],
        homes: &["screenplay_scene"],
        screens: &["screenplay"],
        names: &[K::Scene, K::Character, K::Location],
        picker: Some(P::Screenplay),
        weight: 5,
        min_role: Anyone,
        examples: &["How many scenes does Ravi speak in?"],
    },
    Leaf {
        id: "breakdown/breakdown",
        branch: "Breakdown",
        label: "Breakdown",
        title: "Breakdown",
        aliases: &["props", "wardrobe", "elements"],
        fetchers: &[
            f("breakdown", breakdown::run),
            f("breakdown summary", breakdown::summary),
        ],
        kinds: &["Breakdown", "Count"],
        about: &["breakdown", "element", "project"],
        topic: "",
        search: &[nav("breakdown")],
        homes: &["scene_breakdown"],
        screens: &["breakdown"],
        names: &[K::Scene, K::Character],
        picker: Some(P::Scene),
        weight: 3,
        min_role: Anyone,
        examples: &["What props does scene 4 need?"],
    },
    Leaf {
        id: "production/overview",
        branch: "Production",
        label: "Overview",
        title: "Production Overview",
        aliases: &["production", "progress"],
        fetchers: &[f("production overview", plans::overview)],
        kinds: &["Production overview"],
        about: &["production"],
        topic: "",
        search: &[nav_bare("production"), nav("home")],
        homes: &["project", "exchange_review_record", "review_queue_item"],
        screens: &["production", "production/overview", "home"],
        names: &[],
        picker: None,
        weight: 1,
        min_role: Anyone,
        examples: &["How far along is the breakdown?"],
    },
    Leaf {
        id: "production/catalog",
        branch: "Production",
        label: "Catalog",
        title: "Catalog",
        aliases: &["catalogue", "items"],
        fetchers: &[f("catalog", plans::catalog)],
        kinds: &["Catalog item", "Count"],
        about: &["catalog"],
        topic: "",
        search: &[nav_sub("production", "catalog")],
        homes: &["catalog_item"],
        screens: &["production/catalog"],
        names: &[],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What vehicles are in the catalog?"],
    },
    Leaf {
        id: "production/locations",
        branch: "Production",
        label: "Locations",
        title: "Locations",
        aliases: &["location", "places", "permits"],
        fetchers: &[f("production", production::run)],
        kinds: &["Location", "Scene location", "Count"],
        about: &["location", "contact details"],
        topic: "locations",
        search: &[nav_sub("production", "locations")],
        homes: &["location"],
        screens: &["production/locations"],
        names: &[K::Location, K::Scene],
        picker: None,
        weight: 2,
        min_role: ContactDetails,
        examples: &["Which locations are confirmed?"],
    },
    Leaf {
        id: "production/cast-crew",
        branch: "Production",
        label: "Cast & Crew",
        title: "Cast & Crew",
        aliases: &["cast", "crew", "actors"],
        fetchers: &[f("people", people::run)],
        kinds: &["Cast", "Crew", "Count"],
        about: &["cast", "crew", "actor", "contact details"],
        topic: "cast crew",
        search: &[nav_sub("production", "cast-crew")],
        homes: &["cast_member", "crew_member"],
        screens: &["production/cast-crew"],
        names: &[K::Person, K::Character],
        picker: None,
        weight: 2,
        min_role: ContactDetails,
        examples: &["Who plays Anjali?"],
    },
    Leaf {
        id: "production/moodboards",
        branch: "Production",
        label: "Moodboards",
        title: "Moodboards",
        aliases: &["mood board", "mood boards", "moodboard"],
        fetchers: &[f("visual", visual::run)],
        kinds: &["Mood board", "Count"],
        about: &["moodboard", "mood board"],
        topic: "moodboards",
        search: &[nav_sub("production", "moodboards")],
        homes: &["moodboard", "moodboard_item"],
        screens: &["production/moodboards"],
        names: &[K::Scene],
        picker: Some(P::Scene),
        weight: 1,
        min_role: Anyone,
        examples: &["What is on the Monsoon moodboard?"],
    },
    Leaf {
        id: "production/storyboards",
        branch: "Production",
        label: "Storyboards",
        title: "Storyboards",
        aliases: &["storyboard", "panels"],
        fetchers: &[f("visual", visual::run)],
        kinds: &["Storyboard", "Count"],
        about: &["storyboard"],
        topic: "storyboards",
        search: &[nav_sub("production", "storyboards")],
        homes: &["storyboard", "storyboard_panel"],
        screens: &["production/storyboards"],
        names: &[K::Scene],
        picker: Some(P::Scene),
        weight: 1,
        min_role: Anyone,
        examples: &["Does scene 3 have a storyboard?"],
    },
    Leaf {
        id: "production/shots",
        branch: "Production",
        label: "Shot Lists",
        title: "Shot Lists",
        aliases: &["shots", "shot list", "setups"],
        fetchers: &[f("visual", visual::run)],
        kinds: &["Shot", "Count"],
        about: &["shot"],
        topic: "shots shot list",
        search: &[nav_sub("production", "shots")],
        homes: &["shot"],
        screens: &["production/shots"],
        names: &[K::Scene],
        picker: Some(P::Scene),
        weight: 2,
        min_role: Anyone,
        examples: &["How many shots are planned for scene 9?"],
    },
    Leaf {
        id: "production/schedule",
        branch: "Production",
        label: "Schedule",
        title: "Schedule",
        aliases: &["shooting schedule", "shoot days", "stripboard"],
        fetchers: &[f("schedule", schedule::run)],
        kinds: &[
            "Day",
            "Scene day",
            "Character days",
            "Shoot period",
            "Schedule warning",
            "Count",
        ],
        about: &["day", "schedule", "scene", "shoot"],
        topic: "schedule shooting days",
        search: &[nav_sub("production", "schedule")],
        homes: &["shooting_day"],
        screens: &["production/schedule"],
        names: &[K::Scene, K::Day, K::Character],
        picker: Some(P::Day),
        weight: 3,
        min_role: Anyone,
        examples: &["Which day is scene 12 shot?"],
    },
    Leaf {
        id: "production/day-out-of-days",
        branch: "Production",
        label: "Day Out of Days",
        title: "Day Out of Days",
        aliases: &["dood", "day out of days", "work days"],
        fetchers: &[f("day out of days", plans::day_out_of_days)],
        kinds: &["Day out of days", "Count"],
        about: &["day out of days", "cast", "schedule"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["production/dood"],
        names: &[K::Day, K::Character, K::Person],
        picker: Some(P::Day),
        weight: 2,
        min_role: Anyone,
        examples: &["How many work days does Ravi have?"],
    },
    Leaf {
        id: "production/daily-view",
        branch: "Production",
        label: "Daily View",
        title: "Daily View",
        aliases: &["daily", "today"],
        fetchers: &[f("daily view", plans::daily)],
        kinds: &["Daily view"],
        about: &["daily view", "schedule", "shoot day"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["production/daily"],
        names: &[K::Day, K::Scene],
        picker: Some(P::Day),
        weight: 2,
        min_role: Anyone,
        examples: &["What are we shooting on day 3?"],
    },
    Leaf {
        id: "production/sides",
        branch: "Production",
        label: "Sides",
        title: "Sides",
        aliases: &["side"],
        fetchers: &[f("sides", plans::sides)],
        kinds: &["Sides"],
        about: &["sides"],
        topic: "",
        search: &[nav_sub("production", "sides")],
        homes: &["side"],
        screens: &["production/sides"],
        names: &[K::Scene],
        picker: Some(P::Scene),
        weight: 1,
        min_role: Anyone,
        examples: &["Which sides hold scene 7?"],
    },
    Leaf {
        id: "production/reports",
        branch: "Production",
        label: "Reports",
        title: "Reports",
        aliases: &["report", "production report"],
        fetchers: &[f("reports", plans::reports)],
        kinds: &["Report", "Saved report"],
        about: &["report"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["production/reports"],
        names: &[K::Scene, K::Character, K::Location],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What does the prop report show?"],
    },
    Leaf {
        id: "production/budget",
        branch: "Production",
        label: "Budget",
        title: "Budget",
        aliases: &["cost", "costs", "money"],
        fetchers: &[f("budget", budget::run)],
        kinds: &[
            "Budget line",
            "Budget total",
            "Contingency setting",
            "No actual yet",
            "Over plan",
            "Count",
        ],
        about: &["budget", "line", "cost"],
        topic: "budget",
        search: &[nav_sub("production", "budget")],
        homes: &[],
        screens: &["production/budget"],
        names: &[K::BudgetLine],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["What is the planned total?"],
    },
    Leaf {
        id: "callsheets/callsheets",
        branch: "Call Sheets",
        label: "Call Sheets",
        title: "Call Sheets",
        aliases: &["call sheet", "callsheet", "call time"],
        fetchers: &[f("call sheets", schedule::call_sheets_section)],
        kinds: &[
            "Call sheet",
            "Call sheet departments",
            "Call sheet extras",
            "Call sheet place",
            "Call sheet practical",
            "Call sheet times",
            "Cast calls",
            "Crew calls",
            "Count",
        ],
        about: &["call sheet", "contact details"],
        topic: "call sheets",
        search: &[nav("callsheets")],
        homes: &["call_sheet"],
        screens: &["callsheets"],
        names: &[K::CallSheet, K::Day],
        picker: Some(P::CallSheet),
        weight: 3,
        min_role: ContactDetails,
        examples: &["Is the call sheet for day 2 final?"],
    },
    Leaf {
        id: "files/files",
        branch: "Files",
        label: "Files",
        title: "Files",
        aliases: &["file", "attachments", "documents", "folders"],
        fetchers: &[f("files", files::run)],
        kinds: &["File", "Count"],
        about: &["file"],
        topic: "files",
        search: &[nav("files")],
        homes: &["project_file"],
        screens: &["files"],
        names: &[K::File],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["Which files are in the Contracts folder?"],
    },
    Leaf {
        id: "notes/notes",
        branch: "Notes & Tasks",
        label: "Notes",
        title: "Notes",
        aliases: &["note", "project notes", "private notes"],
        fetchers: &[
            f("project notes", files::run),
            f("latest notes", notes::latest_notes),
            f("private notes", notes::private_notes),
        ],
        kinds: &["Project note", "Private note", "Count"],
        about: &["note"],
        topic: "notes",
        search: &[nav_key("notes", Key::Lacks("taskId"))],
        homes: &["project_note", "private_note", "comment"],
        screens: &["notes"],
        names: &[],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What do my notes say about permits?"],
    },
    Leaf {
        id: "notes/tasks",
        branch: "Notes & Tasks",
        label: "Tasks",
        title: "Tasks",
        aliases: &["task", "todo", "to do"],
        fetchers: &[f("tasks", production::run)],
        kinds: &["Task", "Count"],
        about: &["task"],
        topic: "tasks",
        search: &[nav_key("notes", Key::Has("taskId"))],
        homes: &["task"],
        screens: &["notes/tasks"],
        names: &[],
        picker: None,
        weight: 2,
        min_role: Anyone,
        examples: &["What open tasks do I have?"],
    },
    Leaf {
        id: "help/guide",
        branch: "App Help & Settings",
        label: "Product Guide",
        title: "Product Guide",
        aliases: &["help", "guide", "how do i"],
        fetchers: &[f("product guide", product::run)],
        kinds: &["Guide"],
        about: &["guide"],
        topic: "",
        search: &[],
        homes: &[],
        screens: &["help"],
        names: &[],
        picker: None,
        weight: 3,
        min_role: Anyone,
        examples: &["How do I export a screenplay?"],
    },
    Leaf {
        id: "help/settings",
        branch: "App Help & Settings",
        label: "Settings",
        title: "Settings",
        aliases: &["preferences", "project settings", "members"],
        fetchers: &[f("settings", settings::run), f("product guide", product::run)],
        kinds: &[
            "Project",
            "Project member",
            "Appearance",
            "Spell checking",
            "Guide",
        ],
        about: &["setting", "project"],
        topic: "project settings members",
        search: &[nav("settings")],
        homes: &["template"],
        screens: &[
            "settings",
            "settings/templates",
            "activity",
            "recentlyDeleted",
        ],
        names: &[],
        picker: None,
        weight: 1,
        min_role: Anyone,
        examples: &["Who is on this project?"],
    },
];

pub fn leaf(id: &str) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.id == id)
}

/// The leaf where a kind of name lives, for the pointer.
pub fn home_of(kind: Kind) -> &'static str {
    match kind {
        K::Scene => "screenplay/scenes",
        K::Character => "story/characters",
        K::Person => "production/cast-crew",
        K::Location => "production/locations",
        K::Act | K::Card => "story/beat-board",
        K::VaultItem => "vault/project",
        K::File => "files/files",
        K::BudgetLine => "production/budget",
        K::Day => "production/schedule",
        K::CallSheet => "callsheets/callsheets",
    }
}

/// The leaf an indexed entity type lives in.
pub fn home_of_entity(entity_type: &str) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.homes.contains(&entity_type))
}

/// May a document of this index kind hold text under these search rules? A comment or a private note
/// sits under the object it is attached to, so its navigation target decides, later.
pub fn kind_may_match(kind: &str, rules: &[NavRule]) -> bool {
    if matches!(kind, "comment" | "private_note") {
        return true;
    }
    home_of_entity(kind).is_some_and(|l| l.search.iter().any(|r| rules.contains(r)))
}

// ------------------------------------------------------------------ the picks

/// One section of a request, as the panel sends it. Ids are references the core checks.
#[derive(Debug, Clone, Default, Deserialize, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionPick")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionPick {
    pub leaf: String,
    #[serde(default)]
    #[ts(optional)]
    pub screenplay_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub draft_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub shooting_day_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub call_sheet_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickedDay {
    pub id: String,
    pub number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickedSheet {
    pub id: String,
    pub title: String,
}

/// A pick the core has checked.
#[derive(Debug, Clone)]
pub struct PickedLeaf {
    pub leaf: &'static Leaf,
    pub screenplay_id: Option<String>,
    /// The draft every read of this leaf uses (Scenes only).
    pub draft: Option<DraftRef>,
    pub scene: Option<(DraftRef, SceneRow)>,
    pub day: Option<PickedDay>,
    pub call_sheet: Option<PickedSheet>,
    /// How the "Using:" line names this pick.
    pub label: String,
}

impl PickedLeaf {
    /// The person narrowed this leaf to one scene, day or call sheet.
    pub fn narrowed(&self) -> bool {
        self.scene.is_some() || self.day.is_some() || self.call_sheet.is_some()
    }
}

/// The checked picks of one request.
#[derive(Debug, Clone)]
pub struct Selection {
    pub picks: Vec<PickedLeaf>,
    /// "Using: Screenplay (Draft 2, all scenes) + Cast & Crew"
    pub label: String,
}

impl Selection {
    /// Every section a project of this kind has, nothing narrowed and over the cap of a person's picks:
    /// what a request with no picks reads under `AppConfig::ai_eval_all_sections`, so the older test
    /// sets run the code that ships.
    pub fn every(episodic: bool) -> Self {
        let picks = LEAVES
            .iter()
            .filter(|l| available(l, episodic))
            .map(|leaf| PickedLeaf {
                leaf,
                screenplay_id: None,
                draft: None,
                scene: None,
                day: None,
                call_sheet: None,
                label: String::new(),
            })
            .collect();
        Self {
            picks,
            label: EVERY_SECTION_LABEL.to_string(),
        }
    }

    pub fn takes(&self, kind: Kind) -> bool {
        self.picks.iter().any(|p| p.leaf.takes(kind))
    }

    /// Is this leaf one of the picks?
    pub fn has(&self, leaf: &str) -> bool {
        self.picks.iter().any(|p| p.leaf.id == leaf)
    }

    /// The picked sections as a place to have looked in: "Budget and Cast & Crew".
    pub fn looked_in(&self) -> String {
        if self.picks.len() > MAX_PICKS {
            return queries::EVERYWHERE.to_string();
        }
        let names: Vec<String> = self
            .picks
            .iter()
            .map(|p| p.leaf.title.to_string())
            .collect();
        queries::join_and(&names)
    }

    /// The scene or the draft the request narrows to, for tools that default to one.
    pub fn draft(&self) -> Option<&DraftRef> {
        self.picks.iter().find_map(|p| p.draft.as_ref())
    }

    pub fn scene(&self) -> Option<&SceneRow> {
        self.picks
            .iter()
            .find(|p| p.leaf.picker == Some(SectionPicker::Screenplay))
            .and_then(|p| p.scene.as_ref())
            .map(|(_, s)| s)
    }

    /// Stored with the request: leaf ids, never a label or a title.
    pub fn codes(&self) -> Vec<&'static str> {
        self.picks.iter().map(|p| p.leaf.id).collect()
    }
}

fn invalid<T>(message: &str) -> AppResult<T> {
    Err(AppError::invalid_input(message))
}

pub fn is_episodic(c: &Connection) -> AppResult<bool> {
    let project_type: String =
        c.query_row("SELECT project_type FROM project LIMIT 1", [], |r| r.get(0))?;
    Ok(matches!(project_type.as_str(), "Episodic" | "Series"))
}

/// A leaf this project has. Only Episodes depends on the project.
fn available(l: &Leaf, episodic: bool) -> bool {
    l.id != "story/episodes" || episodic
}

/// What a request with no picks reads under `AppConfig::ai_eval_all_sections`.
pub fn every_section(c: &Connection) -> AppResult<Selection> {
    Ok(Selection::every(is_episodic(c)?))
}

/// Check the picks of a request. An unknown or repeated leaf, more than three, or an id that does not
/// belong to this project is invalid input: nothing runs.
pub fn resolve_picks(c: &Connection, picks: &[SectionPick]) -> AppResult<Selection> {
    if picks.is_empty() {
        return invalid("Pick a section first.");
    }
    if picks.len() > MAX_PICKS {
        return invalid("Pick at most 3 sections.");
    }
    let episodic = is_episodic(c)?;
    let mut out: Vec<PickedLeaf> = Vec::with_capacity(picks.len());
    for p in picks {
        let Some(l) = leaf(&p.leaf) else {
            return invalid("That section does not exist.");
        };
        if out.iter().any(|o| o.leaf.id == l.id) {
            return invalid("Each section can be picked once.");
        }
        if !available(l, episodic) {
            return invalid("This project has no episodes.");
        }
        out.push(resolve_pick(c, l, p)?);
    }
    let label = format!(
        "Using: {}",
        out.iter()
            .map(|p| p.label.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    );
    Ok(Selection { picks: out, label })
}

fn resolve_pick(c: &Connection, l: &'static Leaf, p: &SectionPick) -> AppResult<PickedLeaf> {
    let takes_screenplay = l.picker == Some(SectionPicker::Screenplay);
    let takes_scene = matches!(
        l.picker,
        Some(SectionPicker::Screenplay | SectionPicker::Scene)
    );
    let takes_day = matches!(
        l.picker,
        Some(SectionPicker::Day | SectionPicker::CallSheet)
    );
    let takes_sheet = l.picker == Some(SectionPicker::CallSheet);
    if (!takes_screenplay && (p.screenplay_id.is_some() || p.draft_id.is_some()))
        || (!takes_scene && p.scene_id.is_some())
        || (!takes_day && p.shooting_day_id.is_some())
        || (!takes_sheet && p.call_sheet_id.is_some())
    {
        return invalid("That choice does not apply to this section.");
    }
    let mut draft = match &p.draft_id {
        Some(id) => match queries::draft_by_id(c, id)? {
            Some(d) => Some(d),
            None => return invalid("That draft is not in this project."),
        },
        None => None,
    };
    let mut screenplay_id = p.screenplay_id.clone();
    if let Some(id) = &screenplay_id {
        let known: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM screenplay WHERE id = ?1 AND deleted_at IS NULL)",
            [id],
            |r| r.get(0),
        )?;
        if !known {
            return invalid("That screenplay is not in this project.");
        }
        if draft.as_ref().is_some_and(|d| d.screenplay_id != *id) {
            return invalid("That draft belongs to another screenplay.");
        }
    }
    let scene = match &p.scene_id {
        Some(id) => match queries::scene_by_id(c, id)? {
            Some(found) => Some(found),
            None => return invalid("That scene is not in this project."),
        },
        None => None,
    };
    if let (Some((scene_draft, _)), Some(d)) = (&scene, &draft)
        && scene_draft.id != d.id
    {
        return invalid("That scene belongs to another draft.");
    }
    if let (Some((scene_draft, _)), Some(id)) = (&scene, &screenplay_id)
        && scene_draft.screenplay_id != *id
    {
        return invalid("That scene belongs to another screenplay.");
    }
    // The Scenes section reads one draft: the chosen one, the draft of the chosen scene, else the
    // current draft of the chosen screenplay or of the only screenplay.
    if takes_screenplay {
        if draft.is_none() {
            draft = match (&scene, &screenplay_id) {
                (Some((d, _)), _) => Some(d.clone()),
                (None, Some(id)) => super::index::draft_for_screenplay(c, id)?,
                (None, None) => queries::current_draft(c)?,
            };
        }
        if let Some(d) = &draft {
            screenplay_id = Some(d.screenplay_id.clone());
        }
    }
    let day = match &p.shooting_day_id {
        Some(id) => Some(resolve_day(c, id)?),
        None => None,
    };
    let call_sheet = match &p.call_sheet_id {
        Some(id) => {
            let row: Option<(String, String)> = c
                .query_row(
                    "SELECT id, title FROM call_sheet
                     WHERE id = ?1 AND deleted_at IS NULL AND superseded_by IS NULL",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            match row {
                Some((id, title)) => Some(PickedSheet { id, title }),
                None => return invalid("That call sheet is not in this project."),
            }
        }
        None => None,
    };
    let label = pick_label(
        l,
        draft.as_ref(),
        scene.as_ref().map(|(_, s)| s),
        day.as_ref(),
        call_sheet.as_ref(),
    );
    Ok(PickedLeaf {
        leaf: l,
        screenplay_id,
        draft,
        scene,
        day,
        call_sheet,
        label,
    })
}

fn resolve_day(c: &Connection, id: &str) -> AppResult<PickedDay> {
    let Some(schedule) = board::current_schedule(c)? else {
        return invalid("That shooting day is not in this project.");
    };
    let days = board::load_days(c, &schedule.id)?;
    match days.iter().find(|d| d.id == id) {
        Some(d) => Ok(PickedDay {
            id: d.id.clone(),
            number: d.number.map(|n| n as u32),
        }),
        None => invalid("That shooting day is not in this project."),
    }
}

/// A name the person typed, inside the "Using:" line: on one line and short.
fn in_label(name: &str) -> String {
    let one_line = name.split_whitespace().collect::<Vec<_>>().join(" ");
    queries::truncate_chars(&one_line, MAX_LABEL_NAME)
}

/// "Screenplay (Draft 2, all scenes)", "Breakdown (Scene 4)", "Schedule (Day 3)", "Budget".
fn pick_label(
    l: &Leaf,
    draft: Option<&DraftRef>,
    scene: Option<&SceneRow>,
    day: Option<&PickedDay>,
    sheet: Option<&PickedSheet>,
) -> String {
    if l.picker == Some(SectionPicker::Screenplay) {
        let which = match scene {
            Some(s) => format!("Scene {}", s.number),
            None => "all scenes".to_string(),
        };
        return match draft {
            Some(d) => format!("{} ({}, {which})", l.title, in_label(&d.label())),
            None => format!("{} ({which})", l.title),
        };
    }
    if let Some(s) = sheet {
        return format!("{} ({})", l.title, in_label(&s.title));
    }
    if let Some(d) = day {
        return match d.number {
            Some(n) => format!("{} (Day {n})", l.title),
            None => format!("{} (Off day)", l.title),
        };
    }
    match scene {
        Some(s) => format!("{} (Scene {})", l.title, s.number),
        None => l.title.to_string(),
    }
}

// ------------------------------------------------------------------ text inside the picks

/// Text search inside the picked leaves, one list per pick.
#[derive(Debug, Clone, Default)]
pub struct Retrieved {
    pub passages: Vec<Vec<Fact>>,
    /// True when a path the search wanted was missing, so it ran as full text.
    pub degraded: bool,
    /// False when no picked leaf had anything to search.
    pub searched: bool,
}

impl Retrieved {
    pub fn path_code(&self) -> &'static str {
        match (self.searched, self.degraded) {
            (false, _) => "none",
            (true, true) => "full_text",
            (true, false) => "hybrid",
        }
    }
}

/// Run the hybrid search for each picked leaf, prefiltered by its navigation rules and the owner
/// rule. Without vectors the same call is full text inside the leaf, and `degraded` says so. A leaf
/// narrowed to one instance reads that instance through its fetchers instead: search would bring
/// back its neighbours.
pub fn retrieve(
    core: &AppCore,
    session: &ProjectSession,
    actor: &Actor,
    selection: &Selection,
    query: &str,
) -> Retrieved {
    let mut out = Retrieved::default();
    for p in &selection.picks {
        let mut facts = Vec::new();
        if !p.leaf.search.is_empty() && !p.narrowed() {
            out.searched = true;
            let req = RetrieveRequest {
                query: query.to_string(),
                draft: p.draft.as_ref().map(|d| d.id.clone()),
                limit: PASSAGES_PER_LEAF,
                route: RetrievalRoute::HybridGraph,
                nav: Some(p.leaf.search),
            };
            match retrieval::retrieve(core, session, actor, &req) {
                Ok(packet) => {
                    out.degraded |= packet.degraded;
                    for (i, item) in packet.items.iter().enumerate() {
                        let Some(src) = packet.sources.get(i) else {
                            continue;
                        };
                        let title = packet.provenance.get(i).map_or("", |v| v.label.as_str());
                        let mut fact = Fact::new("Found", &src.entity_id)
                            .f("type", &src.entity_type)
                            .f("title", title)
                            .f("where", &item.source)
                            .f("text", queries::truncate_chars(&item.text, PASSAGE_CHARS))
                            .prio(P_LOW);
                        if src.entity_type == "private_note" {
                            fact = fact.private_to(&actor.user_id);
                        }
                        facts.push(fact);
                    }
                }
                Err(e) => {
                    tracing::warn!(code = e.code_str(), "text search inside a section failed");
                    out.degraded = true;
                }
            }
        }
        out.passages.push(facts);
    }
    out
}

/// What the trace keeps of a request's sections: codes and numbers only (ADR-0028 decision 8).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct SectionsTrace {
    pub leaves: Vec<String>,
    pub tokens: Vec<usize>,
    pub instances: usize,
    pub retrieval: String,
    pub degraded: bool,
    pub outside: usize,
    /// The leaf the pointer named, when it fired.
    pub pointer: Option<String>,
    /// What fired it: `name`, `topic`, `exists`, `help`, `tool` or `open`.
    pub pointer_trigger: Option<String>,
}

impl SectionsTrace {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ------------------------------------------------------------------ the operation

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionsArgs")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionsArgs {
    /// The draft whose scenes the scene list shows. Default: the current draft.
    #[serde(default)]
    #[ts(optional)]
    pub draft_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionLeaf")]
#[serde(rename_all = "camelCase")]
pub struct SectionLeafDto {
    pub id: String,
    pub branch: String,
    pub label: String,
    /// How the "Using:" line names it.
    pub title: String,
    pub picker: Option<SectionPicker>,
    /// False for Episodes in a project that has none.
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionBranch")]
#[serde(rename_all = "camelCase")]
pub struct SectionBranchDto {
    pub label: String,
    pub leaves: Vec<SectionLeafDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionDraft")]
#[serde(rename_all = "camelCase")]
pub struct SectionDraftDto {
    pub id: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSectionInstance")]
#[serde(rename_all = "camelCase")]
pub struct SectionInstanceDto {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSections")]
#[serde(rename_all = "camelCase")]
pub struct SectionsDto {
    pub branches: Vec<SectionBranchDto>,
    pub drafts: Vec<SectionDraftDto>,
    /// The scenes of the asked draft, else of the current draft.
    pub scenes: Vec<SectionInstanceDto>,
    pub days: Vec<SectionInstanceDto>,
    pub call_sheets: Vec<SectionInstanceDto>,
    #[ts(type = "number")]
    pub max_picks: u32,
}

/// The tree for the picker, and the things an instance picker lists.
pub fn sections_op(core: &AppCore, actor: &Actor, args: SectionsArgs) -> AppResult<SectionsDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    s.store.read(|c| {
        let episodic = is_episodic(c)?;
        let mut branches: Vec<SectionBranchDto> = Vec::new();
        for l in LEAVES {
            let dto = SectionLeafDto {
                id: l.id.to_string(),
                branch: l.branch.to_string(),
                label: l.label.to_string(),
                title: l.title.to_string(),
                picker: l.picker,
                available: available(l, episodic),
            };
            match branches.iter_mut().find(|b| b.label == l.branch) {
                Some(b) => b.leaves.push(dto),
                None => branches.push(SectionBranchDto {
                    label: l.branch.to_string(),
                    leaves: vec![dto],
                }),
            }
        }
        let all_drafts = queries::list_drafts(c)?;
        let chosen = match &args.draft_id {
            Some(id) => match all_drafts.iter().find(|d| d.id == *id) {
                Some(d) => Some(d.clone()),
                None => return invalid("That draft is not in this project."),
            },
            None => queries::current_draft(c)?
                .or_else(|| all_drafts.iter().find(|d| d.is_current).cloned()),
        };
        let scenes = match &chosen {
            Some(d) => queries::scenes(c, &d.id)?
                .iter()
                .map(|s| SectionInstanceDto {
                    id: s.id.clone(),
                    label: s.label(),
                })
                .collect(),
            None => Vec::new(),
        };
        let days = match board::current_schedule(c)? {
            Some(sched) => board::load_days(c, &sched.id)?
                .into_iter()
                .map(|d| SectionInstanceDto {
                    label: match (d.number, d.date.as_deref()) {
                        (Some(n), Some(date)) => format!("Day {n} ({date})"),
                        (Some(n), None) => format!("Day {n}"),
                        (None, Some(date)) => format!("Off day ({date})"),
                        (None, None) => "Off day".to_string(),
                    },
                    id: d.id,
                })
                .collect(),
            None => Vec::new(),
        };
        let mut st = c.prepare(
            "SELECT id, title FROM call_sheet WHERE deleted_at IS NULL AND superseded_by IS NULL
             ORDER BY created_at, revision LIMIT 100",
        )?;
        let call_sheets = st
            .query_map(params![], |r| {
                Ok(SectionInstanceDto {
                    id: r.get(0)?,
                    label: r.get(1)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(SectionsDto {
            branches,
            drafts: all_drafts
                .into_iter()
                .map(|d| SectionDraftDto {
                    id: d.id,
                    screenplay_id: d.screenplay_id,
                    screenplay_title: d.screenplay_title,
                    name: d.name,
                    is_current: d.is_current,
                })
                .collect(),
            scenes,
            days,
            call_sheets,
            max_picks: MAX_PICKS as u32,
        })
    })
}

/// The picks as the stage log keeps them.
pub fn stage_json(trace: &SectionsTrace) -> Value {
    json!({
        "leaves": trace.leaves,
        "tokens": trace.tokens,
        "instances": trace.instances,
        "retrieval": trace.retrieval,
        "degraded": trace.degraded,
        "outside": trace.outside,
        "pointer": trace.pointer,
        "pointerTrigger": trace.pointer_trigger,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use regex::Regex;

    use super::*;
    use crate::modules::ai::intelligence::documents::KINDS;

    fn src() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).expect("source folder") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                rust_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    /// Every (workspace, sub) a `search_doc` navigation target is written with, from the source.
    fn nav_writers() -> BTreeSet<String> {
        let re =
            Regex::new(r#""workspace":\s*"([a-zA-Z]+)"(?:,\s*"sub":\s*"([a-z-]+)")?"#).unwrap();
        let mut files = Vec::new();
        rust_files(&src(), &mut files);
        let mut out = BTreeSet::new();
        for p in files {
            let text = std::fs::read_to_string(&p).expect("source file");
            for c in re.captures_iter(&text) {
                out.insert(match c.get(2) {
                    Some(sub) => format!("{}/{}", &c[1], sub.as_str()),
                    None => c[1].to_string(),
                });
            }
        }
        out
    }

    /// The names of `case` lines inside one Swift enum.
    fn swift_cases(file: &str, name: &str) -> Vec<String> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/macos/Sources/OpenFrameUI")
            .join(file);
        let text = std::fs::read_to_string(&path).expect("the Mac navigation source");
        let start = text.find(&format!("enum {name}:")).expect("the enum");
        let body = &text[start..];
        let body = &body[..body.find("var id:").expect("the end of the cases")];
        let mut out = Vec::new();
        for line in body.lines() {
            if let Some(rest) = line.trim().strip_prefix("case ") {
                out.extend(rest.split(',').map(|w| w.trim().to_string()));
            }
        }
        out
    }

    /// The screens of the Mac navigation, in the ids the core's links use.
    fn mac_screens() -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for w in swift_cases("Project/ProjectNavigation.swift", "Workspace") {
            out.insert(match w.as_str() {
                "ideaVault" => "vault".to_string(),
                "callSheets" => "callsheets".to_string(),
                _ => w,
            });
        }
        for t in swift_cases("Project/ProjectNavigation.swift", "ProductionTab") {
            out.insert(format!(
                "production/{}",
                match t.as_str() {
                    "castCrew" => "cast-crew",
                    "shotLists" => "shots",
                    "dayOutOfDays" => "dood",
                    other => other,
                }
            ));
        }
        for t in swift_cases("Story/StoryModel.swift", "StoryTab") {
            out.insert(format!("story/{t}"));
        }
        out
    }

    /// Every entity type `search_doc` or the derived index holds, from the source.
    fn entity_types() -> BTreeSet<String> {
        let re = Regex::new(r#"\.indexer\("([a-z_]+)""#).unwrap();
        let mut files = Vec::new();
        rust_files(&src(), &mut files);
        let mut out = BTreeSet::new();
        for p in files {
            let text = std::fs::read_to_string(&p).expect("source file");
            for c in re.captures_iter(&text) {
                out.insert(c[1].to_string());
            }
        }
        for k in KINDS {
            out.insert(k.kind.to_string());
        }
        out
    }

    #[test]
    fn the_leaf_ids_are_the_ones_the_evaluation_set_is_written_against() {
        let ids: Vec<&str> = LEAVES.iter().map(|l| l.id).collect();
        assert_eq!(
            ids,
            vec![
                "vault/project",
                "vault/global",
                "story/page",
                "story/beat-board",
                "story/outline",
                "story/characters",
                "story/timeline",
                "story/episodes",
                "screenplay/scenes",
                "breakdown/breakdown",
                "production/overview",
                "production/catalog",
                "production/locations",
                "production/cast-crew",
                "production/moodboards",
                "production/storyboards",
                "production/shots",
                "production/schedule",
                "production/day-out-of-days",
                "production/daily-view",
                "production/sides",
                "production/reports",
                "production/budget",
                "callsheets/callsheets",
                "files/files",
                "notes/notes",
                "notes/tasks",
                "help/guide",
                "help/settings",
            ]
        );
    }

    #[test]
    fn every_leaf_id_is_unique() {
        let ids: BTreeSet<&str> = LEAVES.iter().map(|l| l.id).collect();
        assert_eq!(ids.len(), LEAVES.len());
    }

    #[test]
    fn every_navigation_screen_maps_to_exactly_one_leaf() {
        let mut screens = nav_writers();
        screens.extend(mac_screens());
        // Screens the core links to by a built value, not a literal.
        screens.insert("help".to_string());
        assert!(screens.contains("production/cast-crew"), "{screens:?}");
        assert!(screens.contains("story/board"), "{screens:?}");
        let mut owners: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for l in LEAVES {
            for s in l.screens {
                owners.entry(s).or_default().push(l.id);
            }
        }
        for s in &screens {
            let found = owners.get(s.as_str()).cloned().unwrap_or_default();
            assert_eq!(found.len(), 1, "screen {s} maps to {found:?}");
        }
        for (s, found) in &owners {
            assert_eq!(found.len(), 1, "screen {s} maps to {found:?}");
        }
    }

    #[test]
    fn every_indexed_entity_type_maps_to_exactly_one_leaf() {
        let types = entity_types();
        assert!(types.contains("screenplay_scene"), "{types:?}");
        assert!(types.contains("scene_breakdown"), "{types:?}");
        for t in &types {
            let found: Vec<&str> = LEAVES
                .iter()
                .filter(|l| l.homes.contains(&t.as_str()))
                .map(|l| l.id)
                .collect();
            assert_eq!(found.len(), 1, "entity type {t} maps to {found:?}");
        }
        for l in LEAVES {
            for h in l.homes {
                assert!(types.contains(*h), "{} names unknown type {h}", l.id);
            }
        }
    }

    #[test]
    fn every_kind_a_fetcher_writes_passes_for_some_leaf_or_is_left_out_on_purpose() {
        // Added by the stage, not by a fetcher.
        const LEFT_OUT: &[&str] = &["Found", "Assumed"];
        let re = Regex::new(r#"Fact::new\(\s*"([^"]+)""#).unwrap();
        let mut files = Vec::new();
        rust_files(&src().join("modules/ai/dossier/fetch"), &mut files);
        let mut kinds = BTreeSet::new();
        for p in files {
            let text = std::fs::read_to_string(&p).expect("source file");
            for c in re.captures_iter(&text) {
                kinds.insert(c[1].to_string());
            }
        }
        assert!(kinds.contains("Cast"), "{kinds:?}");
        for k in &kinds {
            let admitted = LEAVES.iter().any(|l| l.kinds.contains(&k.as_str()));
            assert!(
                admitted || LEFT_OUT.contains(&k.as_str()),
                "no section admits the kind {k}"
            );
        }
    }

    #[test]
    fn a_visual_kind_passes_only_for_its_own_leaf() {
        for (kind, id) in [
            ("Mood board", "production/moodboards"),
            ("Storyboard", "production/storyboards"),
            ("Shot", "production/shots"),
            ("Idea Vault item", "vault/project"),
        ] {
            let fact = Fact::new(kind, "x");
            let admitted: Vec<&str> = LEAVES
                .iter()
                .filter(|l| l.admits(&fact))
                .map(|l| l.id)
                .collect();
            assert_eq!(admitted, vec![id], "{kind}");
        }
    }

    #[test]
    fn a_count_or_a_gap_passes_only_when_it_speaks_of_the_leaf() {
        let locations = leaf("production/locations").unwrap();
        let of_tasks = Fact::new("Count", "")
            .f("what", "open tasks")
            .f("value", "3");
        let of_places = Fact::new("Count", "")
            .f("what", "locations")
            .f("value", "3");
        assert!(!locations.admits(&of_tasks));
        assert!(locations.admits(&of_places));
        assert!(leaf("notes/tasks").unwrap().admits(&of_tasks));
        assert!(!locations.admits(&Fact::absent("any tasks", "Notes and Tasks")));
        assert!(locations.admits(&Fact::absent("any locations", "Production")));
    }

    #[test]
    fn a_navigation_rule_reads_workspace_sub_and_key() {
        let task = json!({ "workspace": "notes", "taskId": "t" });
        let note = json!({ "workspace": "notes", "noteId": "n" });
        assert!(leaf("notes/tasks").unwrap().search[0].matches(&task));
        assert!(!leaf("notes/tasks").unwrap().search[0].matches(&note));
        assert!(leaf("notes/notes").unwrap().search[0].matches(&note));
        assert!(!leaf("notes/notes").unwrap().search[0].matches(&task));
        let cast = json!({ "workspace": "production", "sub": "cast-crew", "castMemberId": "c" });
        assert!(leaf("production/cast-crew").unwrap().search[0].matches(&cast));
        assert!(!leaf("production/budget").unwrap().search[0].matches(&cast));
        let card = json!({ "workspace": "story", "cardId": "c" });
        let character = json!({ "workspace": "story", "sub": "characters", "characterId": "c" });
        assert!(leaf("story/beat-board").unwrap().search[0].matches(&card));
        assert!(!leaf("story/beat-board").unwrap().search[0].matches(&character));
    }

    #[test]
    fn a_navigation_rule_means_the_same_in_sql() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE d(nav_json TEXT)").unwrap();
        let docs = [
            json!({ "workspace": "notes", "taskId": "t" }),
            json!({ "workspace": "notes", "noteId": "n" }),
            json!({ "workspace": "story", "cardId": "c" }),
            json!({ "workspace": "story", "sub": "characters", "characterId": "c" }),
            json!({ "workspace": "production", "sub": "shots", "shotId": "s" }),
        ];
        for d in &docs {
            c.execute("INSERT INTO d VALUES (?1)", [d.to_string()])
                .unwrap();
        }
        for l in LEAVES {
            for rule in l.search {
                let sql = format!("SELECT nav_json FROM d WHERE {}", rule.sql());
                let mut st = c.prepare(&sql).unwrap();
                let got: BTreeSet<String> = st
                    .query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .map(Result::unwrap)
                    .collect();
                let want: BTreeSet<String> = docs
                    .iter()
                    .filter(|d| rule.matches(d))
                    .map(Value::to_string)
                    .collect();
                assert_eq!(got, want, "{}", l.id);
            }
        }
    }

    #[test]
    fn a_contact_leaf_keeps_contact_details_from_a_viewer() {
        assert_eq!(
            leaf("production/cast-crew").unwrap().min_role,
            MinRole::ContactDetails
        );
        assert_eq!(
            leaf("production/locations").unwrap().min_role,
            MinRole::ContactDetails
        );
        assert_eq!(
            leaf("callsheets/callsheets").unwrap().min_role,
            MinRole::ContactDetails
        );
    }

    #[test]
    fn a_leaf_is_named_by_its_branch_unless_they_are_the_same() {
        assert_eq!(
            leaf("production/budget").unwrap().path(),
            "Production > Budget"
        );
        assert_eq!(leaf("files/files").unwrap().path(), "Files");
        assert_eq!(
            leaf("screenplay/scenes").unwrap().path(),
            "Screenplay > Scenes"
        );
        assert_eq!(leaf("callsheets/callsheets").unwrap().path(), "Call Sheets");
    }

    #[test]
    fn every_kind_of_name_has_a_home_that_can_answer_about_it() {
        for k in [
            K::Scene,
            K::Character,
            K::Person,
            K::Location,
            K::Act,
            K::Card,
            K::VaultItem,
            K::File,
            K::BudgetLine,
            K::Day,
            K::CallSheet,
        ] {
            let home = leaf(home_of(k)).expect("a known leaf");
            assert!(home.takes(k), "{} does not take {k:?}", home.id);
        }
    }

    #[test]
    fn every_leaf_has_what_the_picker_and_the_tests_need() {
        for l in LEAVES {
            assert!(!l.fetchers.is_empty(), "{}", l.id);
            assert!(!l.kinds.is_empty(), "{}", l.id);
            assert!(!l.about.is_empty(), "{}", l.id);
            assert!(!l.examples.is_empty(), "{}", l.id);
            assert!(!l.aliases.is_empty(), "{}", l.id);
            assert!(l.weight > 0, "{}", l.id);
            for rule in l.search {
                assert!(
                    rule.workspace.chars().all(|c| c.is_ascii_alphabetic()),
                    "{}",
                    l.id
                );
            }
        }
    }

    #[test]
    fn the_label_names_what_was_really_chosen() {
        let draft = DraftRef {
            id: "d".into(),
            name: "Draft 2".into(),
            status: "Writing".into(),
            ordinal: 2,
            is_current: true,
            episode_id: None,
            screenplay_id: "s".into(),
            screenplay_title: "The Short".into(),
            several_screenplays: false,
        };
        let scenes = leaf("screenplay/scenes").unwrap();
        assert_eq!(
            pick_label(scenes, Some(&draft), None, None, None),
            "Screenplay (Draft 2, all scenes)"
        );
        let cast = leaf("production/cast-crew").unwrap();
        assert_eq!(pick_label(cast, None, None, None, None), "Cast & Crew");
        let day = PickedDay {
            id: "x".into(),
            number: Some(3),
        };
        let schedule = leaf("production/schedule").unwrap();
        assert_eq!(
            pick_label(schedule, None, None, Some(&day), None),
            "Schedule (Day 3)"
        );
    }

    #[test]
    fn a_trace_of_sections_holds_codes_and_numbers() {
        let t = SectionsTrace {
            leaves: vec!["production/budget".into()],
            tokens: vec![120],
            instances: 0,
            retrieval: "full_text".into(),
            degraded: true,
            outside: 1,
            pointer: Some("production/budget".into()),
            pointer_trigger: Some("name".into()),
        };
        let v: Value = serde_json::from_str(&t.to_json()).unwrap();
        assert_eq!(v["leaves"][0], "production/budget");
        assert_eq!(v["retrieval"], "full_text");
        assert_eq!(stage_json(&t)["tokens"][0], 120);
        assert_eq!(stage_json(&t)["pointer"], "production/budget");
        assert_eq!(v["pointer_trigger"], "name");
    }
}
