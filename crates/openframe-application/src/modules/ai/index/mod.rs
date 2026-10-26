//! The project index of the conversational assistant (spec `ai-conversational-harness`, sections 5.2
//! and 6): the one list of things a person can name in a message, and the code that turns a message
//! into those things.
//!
//! - `build`: reads the project database into an [`EntityIndex`] (screenplays, scenes, story
//!   characters with their aliases, cast and crew, locations, acts, cards, vault items, files,
//!   budget lines, shoot days, call sheets, and the relations between them).
//! - `resolve`: a pure function, `resolve(index, focus, message) -> Resolved`, no database, no model.
//! - `focus`: what the conversation is about now, kept with the conversation.
//!
//! The index lives in memory, one per open project. It is not a table: it is a few hundred small
//! strings that build in well under 200 ms for 500 scenes, so keeping it in the database would add a
//! migration and a second copy that can disagree with the real rows. A write invalidates it (the
//! store counts content commits) and the next message rebuilds it.

mod build;
mod focus;
mod resolve;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use openframe_domain::AppResult;
use parking_lot::Mutex;

pub use focus::{Focus, with_focus as focus_in};
pub use resolve::{Hit, How, Question, Resolved, SceneHit, normalize, resolve, scene_not_found};

use crate::core::ProjectSession;

/// What an entity is. Scenes and screenplays are found by number and title rules, the rest by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Scene,
    Character,
    Person,
    Location,
    Act,
    Card,
    VaultItem,
    File,
    BudgetLine,
    Day,
    CallSheet,
}

impl Kind {
    /// The word used in a question ("Which person do you mean ...").
    pub fn label(self) -> &'static str {
        match self {
            Kind::Scene => "scene",
            Kind::Character => "character",
            Kind::Person => "person",
            Kind::Location => "location",
            Kind::Act => "act",
            Kind::Card => "card",
            Kind::VaultItem => "Idea Vault item",
            Kind::File => "file",
            Kind::BudgetLine => "budget line",
            Kind::Day => "shoot day",
            Kind::CallSheet => "call sheet",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: Kind,
    pub id: String,
    /// The name as shown to the person.
    pub name: String,
    /// Other full names it answers to: a dialogue cue linked to a character, "Also called X" in the
    /// description, the name without "Dr.", "day 4" for Shoot Day 4, a production scene number.
    pub aliases: Vec<String>,
    /// Parts of a person's name (first, last). Weaker than an alias: two people can share one.
    pub tokens: Vec<String>,
    /// Scenes: the screenplay they belong to.
    pub screenplay_id: Option<String>,
    /// Scenes: 1-based place in the draft. Days: the shoot day number. Acts: the act number.
    pub ordinal: Option<u32>,
    /// A few words to tell it apart: a role, a heading, a status.
    pub detail: String,
}

impl Entity {
    pub fn new(kind: Kind, id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
            name: name.into(),
            aliases: Vec::new(),
            tokens: Vec::new(),
            screenplay_id: None,
            ordinal: None,
            detail: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScreenplayInfo {
    pub id: String,
    pub title: String,
    /// Feature, Short or Episodic.
    pub format: String,
    /// Scenes of the current draft (of the latest draft when none is marked current).
    pub scene_count: u32,
    /// The screenplay has a draft marked current.
    pub has_current_draft: bool,
    /// Other ways to name it: the words only its title has ("teaser"), its format ("the feature").
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    /// "plays" (cast to character), "appears_in" (character to screenplay), "in_act" (card to act),
    /// "for_day" (call sheet to day), or the Story relationship type ("brother of").
    pub kind: String,
}

/// How strongly a phrase names its entity. The score ranks candidates for one mention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rank {
    Name,
    Alias,
    Token,
}

impl Rank {
    pub(crate) fn score(self) -> f32 {
        match self {
            Rank::Name => 1.0,
            Rank::Alias => 0.97,
            Rank::Token => 0.9,
        }
    }
}

#[derive(Debug, Default)]
pub struct EntityIndex {
    pub screenplays: Vec<ScreenplayInfo>,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    /// Normalised phrase to entity position and rank. Scenes are left out: a scene is named by number.
    phrases: HashMap<String, Vec<(usize, Rank)>>,
    /// Phrases of five letters or more, with their word count, for a name typed with a typo.
    fuzzy: Vec<(String, usize, usize, Rank)>,
    by_id: HashMap<String, usize>,
}

impl EntityIndex {
    pub fn new(
        mut screenplays: Vec<ScreenplayInfo>,
        entities: Vec<Entity>,
        relations: Vec<Relation>,
    ) -> Self {
        add_screenplay_aliases(&mut screenplays);
        let mut phrases: HashMap<String, Vec<(usize, Rank)>> = HashMap::new();
        let mut fuzzy = Vec::new();
        let mut by_id = HashMap::new();
        for (i, e) in entities.iter().enumerate() {
            by_id.insert(e.id.clone(), i);
            if e.kind == Kind::Scene {
                continue;
            }
            let all = std::iter::once((&e.name, Rank::Name))
                .chain(e.aliases.iter().map(|a| (a, Rank::Alias)))
                .chain(e.tokens.iter().map(|t| (t, Rank::Token)));
            for (text, rank) in all {
                let p = normalize(text);
                if p.is_empty() {
                    continue;
                }
                let slot = phrases.entry(p.clone()).or_default();
                if !slot.iter().any(|(j, _)| *j == i) {
                    slot.push((i, rank));
                    if p.chars().count() >= 5 {
                        let words = p.split(' ').count();
                        fuzzy.push((p, words, i, rank));
                    }
                }
            }
        }
        Self {
            screenplays,
            entities,
            relations,
            phrases,
            fuzzy,
            by_id,
        }
    }

    pub fn screenplay(&self, id: &str) -> Option<&ScreenplayInfo> {
        self.screenplays.iter().find(|s| s.id == id)
    }

    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.by_id.get(id).map(|i| &self.entities[*i])
    }

    /// Scene `ordinal` (1-based) of a screenplay.
    pub fn scene(&self, screenplay_id: &str, ordinal: u32) -> Option<&Entity> {
        self.entities.iter().find(|e| {
            e.kind == Kind::Scene
                && e.ordinal == Some(ordinal)
                && e.screenplay_id.as_deref() == Some(screenplay_id)
        })
    }

    /// The cast and crew entries that play a character ("who plays Young Anjali").
    pub fn played_by(&self, character_id: &str) -> Vec<&Entity> {
        self.relations
            .iter()
            .filter(|r| r.kind == "plays" && r.to == character_id)
            .filter_map(|r| self.entity(&r.from))
            .collect()
    }

    pub(crate) fn phrase(&self, p: &str) -> Option<&[(usize, Rank)]> {
        self.phrases.get(p).map(Vec::as_slice)
    }

    pub(crate) fn fuzzy_pool(&self) -> &[(String, usize, usize, Rank)] {
        &self.fuzzy
    }

    /// Build the index from the project database.
    pub fn from_database(c: &rusqlite::Connection) -> AppResult<Self> {
        build::build(c)
    }
}

/// The words only one title has ("teaser"), and the format words ("feature", "short"), name a
/// screenplay when the person does not type the whole title.
fn add_screenplay_aliases(sps: &mut [ScreenplayInfo]) {
    const SKIP: [&str; 6] = ["the", "and", "film", "movie", "script", "screenplay"];
    let words: Vec<Vec<String>> = sps
        .iter()
        .map(|s| normalize(&s.title).split(' ').map(str::to_string).collect())
        .collect();
    let largest = sps
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.scene_count)
        .filter(|(_, m)| {
            sps.iter()
                .filter(|s| s.scene_count == m.scene_count)
                .count()
                == 1
        })
        .map(|(i, _)| i);
    for i in 0..sps.len() {
        let mut found: Vec<String> = Vec::new();
        for w in &words[i] {
            let unique = words
                .iter()
                .enumerate()
                .all(|(j, other)| j == i || !other.contains(w));
            if unique && w.chars().count() >= 4 && !SKIP.contains(&w.as_str()) {
                found.push(w.clone());
            }
        }
        let format = sps[i].format.to_lowercase();
        if !format.is_empty()
            && sps
                .iter()
                .filter(|s| s.format.to_lowercase() == format)
                .count()
                == 1
        {
            found.push(format);
        } else if largest == Some(i) {
            // "The feature" when every screenplay is formatted as one: the longest is meant.
            found.push("feature".to_string());
        }
        for a in found {
            if !sps[i].aliases.contains(&a) {
                sps[i].aliases.push(a);
            }
        }
    }
}

/// The index of one open project, kept until a write makes it stale.
#[derive(Default)]
pub struct IndexCache {
    slot: Mutex<Option<(u64, Arc<EntityIndex>)>>,
}

/// The index of the open project: the kept one when nothing was written since it was built,
/// otherwise a fresh build.
pub fn project_index(s: &ProjectSession) -> AppResult<Arc<EntityIndex>> {
    let mut slot = s.ai_index.slot.lock();
    // Read before the build: a write that lands during it leaves the stored number behind, so the
    // next call builds again instead of trusting a half-old index.
    let generation = s.store.generation();
    if let Some((g, ix)) = slot.as_ref()
        && *g == generation
    {
        return Ok(ix.clone());
    }
    let ix = Arc::new(s.store.read(EntityIndex::from_database)?);
    *slot = Some((generation, ix.clone()));
    Ok(ix)
}

/// What the assistant knows about a message before it runs.
pub struct Reading {
    pub resolved: Resolved,
    /// The conversation focus once this message is part of it. Its `answer_type` is still the one
    /// read for the previous message.
    pub focus_after: Focus,
    /// The person's previous message in this conversation.
    pub previous_message: Option<String>,
}

/// Resolve `message` against the project index and the focus of `conversation`. `None` when the
/// index cannot be built: the assistant then runs as it did before, never blocked by this step.
pub fn read_message(
    s: &ProjectSession,
    user_id: &str,
    conversation: &str,
    message: &str,
) -> Option<Reading> {
    let built = project_index(s).and_then(|ix| {
        let focus = s.store.read(|c| focus::load(c, user_id, conversation))?;
        let previous = s
            .store
            .read(|c| focus::last_message(c, user_id, conversation))?;
        Ok((ix, focus, previous))
    });
    match built {
        Ok((ix, focus, previous_message)) => {
            let resolved = resolve(&ix, &focus, message);
            let focus_after = focus.after(&resolved);
            Some(Reading {
                resolved,
                focus_after,
                previous_message,
            })
        }
        Err(e) => {
            tracing::warn!(
                code = e.code_str(),
                "the assistant's project index could not be read"
            );
            None
        }
    }
}

/// The draft the assistant should read for a screenplay: its current draft, else its latest.
pub fn draft_for_screenplay(
    c: &rusqlite::Connection,
    screenplay_id: &str,
) -> AppResult<Option<super::queries::DraftRef>> {
    let drafts: Vec<_> = super::queries::list_drafts(c)?
        .into_iter()
        .filter(|d| d.screenplay_id == screenplay_id)
        .collect();
    Ok(drafts
        .iter()
        .find(|d| d.is_current)
        .or_else(|| drafts.last())
        .cloned())
}
