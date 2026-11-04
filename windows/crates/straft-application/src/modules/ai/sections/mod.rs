//! The section tree the person picks from before asking (spec `ai-section-scoped-assistant`,
//! ADR-0040). The person is the router: the assistant reads only the leaves that were picked, and
//! this module is the one table that says what a leaf is.
//!
//! - [`LEAVES`] has one entry per leaf (one screen of the app): its path, the fetch functions it
//!   runs, the search entity types, fact kinds, name kinds and agent tools that are homed in it,
//!   the instance picker it shows and its share of the token budget.
//! - Coverage tests (`tests.rs`) fail the build when a navigation screen, a `search_doc` entity
//!   type, a fact kind or an agent tool maps to zero leaves or to more than one. This is what keeps
//!   five overlapping taxonomies from coming back.
//! - [`gate`] is the one hard boundary: a fact whose kind is not homed in a picked leaf is dropped,
//!   whatever produced it. A fetcher that reads too wide wastes a read; it can never leak.
//!
//! Nothing here calls a model, and nothing here writes.

mod label;
pub mod pointer;
mod registry;
#[cfg(test)]
mod tests;
pub mod tools;
mod tree;

use serde::{Deserialize, Serialize};
use straft_domain::{Actor, AppError, AppResult, Capability};
use ts_rs::TS;

use super::dossier::{Fact, FetchCtx, Input};
use super::index::{Kind, Resolved};
use super::turn::TurnRead;

pub use label::using_label;
pub use registry::{BRANCHES, LEAVES};
pub use tree::{
    AiBranchDto, AiLeafDto, AiPickCallSheetDto, AiPickDayDto, AiPickDraftDto, AiPickSceneDto,
    AiPickScreenplayDto, AiSectionTreeDto, AiSectionsArgs, sections as sections_query,
};

/// One picked section with the instance choices made under it (the payload of `ai.ask`, spec
/// section 12.2). Only the choices the leaf's picker asks for are read; the rest are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export, rename = "AiSectionPick")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionPick {
    /// `production/cast-crew`
    pub leaf: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub screenplay_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub draft_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub scene_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shooting_day_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub call_sheet_id: Option<String>,
}

/// The most sections one question may read.
pub const MAX_PICKS: usize = 3;

/// A function that reads one part of the project for a picked leaf and returns facts.
pub type Source = fn(&FetchCtx<'_>, &TurnRead, &Resolved) -> Vec<Fact>;

/// The extra choice a leaf asks for under it. Names inside a question (a character, an actor, a
/// place) are not pickers: the name resolver finds them, restricted to the picked leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    None,
    /// Screenplay (required when the project has several), draft (default: current), scene (All or one).
    Screenplay,
    /// Scene (All or one).
    Scene,
    /// Shooting day (All or one).
    ShootingDay,
    /// Shooting day and, within it, a scene.
    DayAndScene,
    /// Call sheet or shooting day (All or one).
    CallSheet,
}

impl Picker {
    /// The word the panel and `ai.sections` use for it.
    pub fn code(self) -> &'static str {
        match self {
            Picker::None => "none",
            Picker::Screenplay => "screenplay",
            Picker::Scene => "scene",
            Picker::ShootingDay => "shootingDay",
            Picker::DayAndScene => "dayAndScene",
            Picker::CallSheet => "callSheet",
        }
    }
}

/// A top-level branch of the tree. Only leaves can be picked.
#[derive(Debug, Clone, Copy)]
pub struct Branch {
    pub id: &'static str,
    pub label: &'static str,
}

/// One leaf: one screen of the app.
pub struct Leaf {
    /// `production/cast-crew`: the branch, a slash, the leaf.
    pub id: &'static str,
    pub branch: &'static str,
    pub label: &'static str,
    /// Other names a person uses for it (the pointer and the tests match them).
    pub aliases: &'static [&'static str],
    pub picker: Picker,
    /// Its share of the dossier budget, relative to the other picked leaves (1 to 5).
    pub weight: u8,
    /// The least permission that may read it.
    pub min_capability: Capability,
    /// The fetch functions the leaf runs, in order. Each one treats the leaf as the topic of the
    /// question (see `FetchCtx::topic`), so a picked leaf is read without needing its keywords.
    pub sources: &'static [Source],
    /// `search_doc` entity types homed here.
    pub entity_types: &'static [&'static str],
    /// `Fact.kind` values homed here.
    pub fact_kinds: &'static [&'static str],
    /// Index entity kinds (names the resolver finds) this leaf serves: a name of that kind is
    /// read when the leaf is picked. The first leaf in the table that lists a kind is its home, the
    /// leaf the pointer names when no leaf that serves it was picked.
    pub name_kinds: &'static [Kind],
    /// Agent tools homed here.
    pub tools: &'static [&'static str],
    /// Shown by the panel when the leaf cannot answer, in one line.
    pub note: Option<&'static str>,
    /// A question a person would ask on this screen. Tests only.
    pub example: &'static str,
}

impl Leaf {
    /// The short name in the "Using:" line: a branch with one leaf of its own is just the branch
    /// ("Screenplay", "Call Sheets"); the Idea Vault keeps its path so This Project and Global differ;
    /// every other leaf is its own name ("Cast & Crew").
    pub fn short_label(&self) -> String {
        let branch = branch(self.branch).map_or(self.branch, |b| b.label);
        if self.branch == "idea-vault" {
            format!("{branch} > {}", self.label)
        } else if LEAVES.iter().filter(|l| l.branch == self.branch).count() == 1 {
            branch.to_string()
        } else {
            self.label.to_string()
        }
    }

    /// `Production > Cast & Crew`; a branch with one leaf of its own name is just `Breakdown`.
    pub fn path_label(&self) -> String {
        let branch = branch(self.branch).map_or(self.branch, |b| b.label);
        if branch == self.label {
            branch.to_string()
        } else {
            format!("{branch} > {}", self.label)
        }
    }
}

/// Check the picks of one question: one to three, each a known leaf the person may read, none
/// twice. The words are the ones the panel shows.
pub fn validate(actor: &Actor, picks: &[SectionPick]) -> AppResult<Vec<&'static Leaf>> {
    if picks.is_empty() {
        return Err(AppError::invalid_input("Pick a section to ask about."));
    }
    if picks.len() > MAX_PICKS {
        return Err(AppError::invalid_input(
            "Three sections at most. Remove one to add another.",
        ));
    }
    let mut out: Vec<&'static Leaf> = Vec::new();
    for p in picks {
        let l = leaf(&p.leaf).ok_or_else(|| label::unknown(&p.leaf))?;
        if out.iter().any(|o| o.id == l.id) {
            return Err(AppError::invalid_input(format!(
                "{} was picked twice.",
                l.path_label()
            )));
        }
        if !actor.can(l.min_capability) {
            return Err(AppError::permission_denied(&format!(
                "read {}",
                l.path_label()
            )));
        }
        out.push(l);
    }
    Ok(out)
}

pub fn branch(id: &str) -> Option<&'static Branch> {
    BRANCHES.iter().find(|b| b.id == id)
}

pub fn leaf(id: &str) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.id == id)
}

/// Fact kinds that several leaves produce (a count is a count of something). They are not homed by
/// kind: [`run_leaf`] stamps them with the leaf that read them, and [`gate`] checks the stamp.
/// `Assumed` and a statement of something missing carry no project data of their own.
pub const SHARED_KINDS: &[&str] = &["Count", "Found", "Assumed", "Missing"];

/// Search entity types the assistant never retrieves, with the reason.
pub const NOT_RETRIEVED: &[(&str, &str)] = &[
    (
        "exchange_review_record",
        "an exchange package review record, a human workflow (ADR-0013 decision 7)",
    ),
    (
        "review_queue_item",
        "a review queue entry of an import, a human workflow (ADR-0013 decision 7)",
    ),
];

/// Tools that work across the picked leaves (search, navigation, retrieval, private notes). Round
/// R6 scopes them to the picks; until then they are listed so the coverage test can name them.
pub const CROSS_TOOLS: &[&str] = &[
    "search_project",
    "retrieve_context",
    "open_workspace",
    "open_object",
];

/// Tools that read pages which are no section of the project (Activity, Recently Deleted, the
/// Exchange package history). A section-scoped turn does not offer them (R6).
pub const UNSECTIONED_TOOLS: &[&str] = &[
    "recent_activity",
    "recently_deleted",
    "package_history",
    "propose_restore_deleted",
];

/// Tools that end the agent loop; they read and write nothing.
pub const LOOP_TOOLS: &[&str] = &["final_answer"];

pub fn home_of_kind(kind: &str) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.fact_kinds.contains(&kind))
}

pub fn home_of_entity_type(entity_type: &str) -> Option<&'static Leaf> {
    LEAVES
        .iter()
        .find(|l| l.entity_types.contains(&entity_type))
}

pub fn home_of_name_kind(kind: Kind) -> Option<&'static Leaf> {
    LEAVES.iter().find(|l| l.name_kinds.contains(&kind))
}

/// Whether a name of this kind is read by any picked leaf. A kind no leaf serves (a day is an
/// address, not content) is always served.
pub fn name_served(picked: &[&Leaf], kind: Kind) -> bool {
    !LEAVES.iter().any(|l| l.name_kinds.contains(&kind))
        || picked.iter().any(|l| l.name_kinds.contains(&kind))
}

/// Run the sources of one leaf. Every fact is stamped with the leaf that read it.
pub fn run_leaf(
    leaf: &'static Leaf,
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    r: &Resolved,
) -> Vec<Fact> {
    let mut out = Vec::new();
    for source in leaf.sources {
        for mut f in source(ctx, read, r) {
            f.home = Some(leaf.id);
            out.push(f);
        }
    }
    out
}

/// Read one leaf for a message: the dossier input is the same one `dossier::build` is given.
pub fn read_leaf(leaf: &'static Leaf, input: &Input<'_>) -> Vec<Fact> {
    let ctx = FetchCtx::new(input);
    run_leaf(leaf, &ctx, input.read, input.resolved)
}

/// Whether a fact may reach the dossier given the leaves that were picked.
pub fn passes(picked: &[&Leaf], f: &Fact) -> bool {
    if SHARED_KINDS.contains(&f.kind) {
        return match f.home {
            Some(h) => picked.iter().any(|l| l.id == h),
            // Orientation and statements code made itself, before any leaf ran.
            None => matches!(f.kind, "Assumed" | "Missing"),
        };
    }
    home_of_kind(f.kind).is_some_and(|home| picked.iter().any(|l| l.id == home.id))
}

/// The one gate at the end of the dossier stage: drop every fact that is not homed in a picked leaf.
pub fn gate(picked: &[&Leaf], facts: &mut Vec<Fact>) {
    facts.retain(|f| passes(picked, f));
}
