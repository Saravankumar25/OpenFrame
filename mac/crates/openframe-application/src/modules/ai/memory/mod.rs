//! The assistant's memory (spec `ai-conversational-harness`, sections 5.1 and 5.7; slice S6).
//!
//! Two kinds, both loaded at the start of a turn by [`prepare`] and both written by code:
//!
//! - Short-term (`short`): the last eight turns of the conversation word for word within about
//!   1,200 tokens, and a one-line-per-turn summary of the older ones built from the focus each turn
//!   stored. Typed output [`ConversationMemory`].
//! - Long-term (`long`, `rules`): what the person stated outright or confirmed, saved per user in the
//!   open project, kept on this computer, listed in Settings, each entry deletable. Typed output
//!   `Vec<MemoryEntry>`. The saving rules are fixed patterns, never a model.
//!
//! Owner decision 2026-10-26: long-term memory is on by default.

mod long;
mod rules;
mod short;
#[cfg(test)]
mod tests;

use openframe_domain::{Actor, AppResult, Capability};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::core::{AppCore, ProjectSession};
use crate::modules::ai::index::{EntityIndex, Focus, Resolved};
use crate::modules::ai::records::ai_meta;
use crate::registry::Registry;

pub use long::{MAX_ENTRIES, MemoryEntry, RETRIEVE_COUNT, RETRIEVE_TOKENS, Saved};
pub use rules::{Candidate, Kind, candidate};
pub use short::{ConversationMemory, RECENT_TOKENS, RECENT_TURNS, RecentTurn};

/// About four characters to a token: the same rough measure the prompt budgets use.
pub(crate) fn approx_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// What one turn knows before it starts.
#[derive(Debug, Clone, Default)]
pub struct TurnMemory {
    pub conversation: ConversationMemory,
    /// Long-term entries that match the message.
    pub entries: Vec<MemoryEntry>,
}

impl TurnMemory {
    /// The recent turns as (message, answer) pairs, oldest first: the history the prompt carries.
    pub fn history(&self) -> Vec<(String, String)> {
        self.conversation
            .recent
            .iter()
            .map(|t| (t.asked.clone(), t.answered.clone()))
            .collect()
    }
}

/// Input of the prepare stage.
pub struct Input<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub conversation: &'a str,
    pub message: &'a str,
    /// The conversation focus once this message is part of it.
    pub focus: &'a Focus,
    /// Ids of the things the message and the focus name (see [`ids_for`]).
    pub entity_ids: &'a [String],
    /// Names for the summary; without it older turns are left out of the summary.
    pub names: Option<&'a EntityIndex>,
}

/// Stage "prepare" (section 5.1, steps 2 and 3). Memory that cannot be read leaves the turn without
/// it: the long-term part is a help, never a reason to fail a request.
pub fn prepare(i: &Input<'_>) -> AppResult<TurnMemory> {
    let conversation = short::load(i.conn, i.actor, i.conversation, i.focus, i.names)?;
    let entries =
        long::retrieve(i.conn, &i.actor.user_id, i.message, i.entity_ids).unwrap_or_else(|e| {
            tracing::warn!(
                code = e.code_str(),
                "the assistant's memory could not be read"
            );
            Vec::new()
        });
    Ok(TurnMemory {
        conversation,
        entries,
    })
}

/// The ids a message names, for matching memory entries.
pub fn mentioned_ids(r: Option<&Resolved>) -> Vec<String> {
    let Some(r) = r else { return Vec::new() };
    let mut ids: Vec<String> = Vec::new();
    for list in [
        &r.screenplays,
        &r.persons,
        &r.characters,
        &r.locations,
        &r.acts,
        &r.days,
        &r.files,
        &r.budget_lines,
    ] {
        ids.extend(list.iter().map(|h| h.id.clone()));
    }
    ids.extend(r.others.iter().map(|(_, h)| h.id.clone()));
    ids.extend(r.scenes.iter().filter_map(|s| s.scene_id.clone()));
    ids
}

/// The mentioned ids plus the screenplay, characters and people the conversation is about.
pub fn ids_for(mentioned: &[String], focus: &Focus) -> Vec<String> {
    let mut ids = mentioned.to_vec();
    ids.extend(focus.screenplay.clone());
    ids.extend(focus.characters.iter().cloned());
    ids.extend(focus.persons.iter().cloned());
    ids.sort();
    ids.dedup();
    ids
}

/// `scope_json` with the memory counts added (no migration: the request already has the column).
pub fn counts_in(mut scope_json: Value, m: &TurnMemory) -> Value {
    if let Value::Object(o) = &mut scope_json {
        o.insert(
            "memory".into(),
            json!({
                "recent": m.conversation.recent.len(),
                "earlier": m.conversation.summary_lines.len(),
                "remembered": m.entries.len(),
            }),
        );
    }
    scope_json
}

/// Stage "store" (section 5.7): keep what the person's message states or confirms. A failure is
/// logged and ignored; the answer has already been stored.
pub fn remember(
    s: &ProjectSession,
    actor: &Actor,
    request_id: &str,
    message: &str,
    m: &TurnMemory,
    entity_ids: &[String],
) {
    let last = m
        .conversation
        .recent
        .last()
        .filter(|t| t.confirmable)
        .map(|t| t.answered.as_str());
    let Some(cand) = candidate(message, last) else {
        return;
    };
    if let Err(e) = s.store.mutate(actor, ai_meta("ai.memory_save"), |tx| {
        long::save(
            tx.conn(),
            &tx.actor().user_id,
            &cand,
            entity_ids,
            request_id,
        )
    }) {
        tracing::warn!(code = e.code_str(), "the assistant could not save a memory");
    }
}

// ------------------------------------------------------------------ operations

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiMemoryListDto {
    pub entries: Vec<MemoryEntry>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiMemoryDeleteArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiMemoryDeleteDto {
    /// False when the entry was already gone.
    pub deleted: bool,
}

fn memory_list(core: &AppCore, actor: &Actor, _: super::AiEmptyArgs) -> AppResult<AiMemoryListDto> {
    actor.require(Capability::UseAi, "use the assistant")?;
    let s = core.project()?;
    Ok(AiMemoryListDto {
        entries: s.store.read(|c| long::list(c, &actor.user_id))?,
    })
}

fn memory_delete(
    core: &AppCore,
    actor: &Actor,
    args: AiMemoryDeleteArgs,
) -> AppResult<AiMemoryDeleteDto> {
    let s = core.project()?;
    let deleted = s.store.mutate(actor, ai_meta("ai.memory_delete"), |tx| {
        long::delete(tx.conn(), &tx.actor().user_id, &args.id)
    })?;
    Ok(AiMemoryDeleteDto { deleted })
}

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("ai.memory_list", memory_list).meta(
        M::query(
            Capability::UseAi,
            "What the assistant remembers about the user in this project (their own entries only).",
        )
        .hidden(h::AI_SELF),
    );
    r.command("ai.memory_delete", memory_delete).meta(
        M::command(
            Capability::UseAi,
            "Make the assistant forget one of the user's remembered entries; the words are erased.",
        )
        .destructive()
        .irreversible()
        .hidden(h::AI_SELF),
    );
}
