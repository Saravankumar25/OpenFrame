//! The long-term memory table (`ai_memory`, migration 0018): what the person stated outright or
//! confirmed, per user, in this project, on this computer.
//!
//! Retrieval is keyword and entity matching in code (no model): an entry is offered when the message
//! names a thing the entry is about, or shares a content word with it. A preference ("always answer
//! in short sentences") has no subject to match, so it is always offered. At most five entries and
//! about 300 tokens go to a turn.

use std::collections::HashSet;

use openframe_domain::{AppResult, new_id, now_ms};
use rusqlite::{Connection, Row, params};
use serde::Serialize;
use ts_rs::TS;

use super::approx_tokens;
use super::rules::{Candidate, Kind};
use crate::modules::ai::index::normalize;

/// Entries kept per person and project. A full memory saves nothing new until the person deletes some.
pub const MAX_ENTRIES: usize = 200;
/// Entries offered to one turn.
pub const RETRIEVE_COUNT: usize = 5;
/// Tokens (about four characters each) offered to one turn.
pub const RETRIEVE_TOKENS: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiMemoryEntry")]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub id: String,
    pub text: String,
    pub kind: Kind,
    /// Ids of the screenplay, characters and people the entry is about.
    pub entity_refs: Vec<String>,
    pub source_request_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

fn entry(r: &Row<'_>) -> rusqlite::Result<MemoryEntry> {
    let kind: String = r.get(2)?;
    let refs: String = r.get(3)?;
    Ok(MemoryEntry {
        id: r.get(0)?,
        text: r.get(1)?,
        kind: Kind::parse(&kind),
        entity_refs: serde_json::from_str(&refs).unwrap_or_default(),
        source_request_id: r.get(4)?,
        created_at: r.get(5)?,
    })
}

/// This user's entries, newest first.
pub fn list(c: &Connection, user_id: &str) -> AppResult<Vec<MemoryEntry>> {
    let mut st = c.prepare(
        "SELECT id, text, kind, entity_refs_json, source_request_id, created_at
         FROM ai_memory WHERE user_id = ?1 AND deleted_at IS NULL
         ORDER BY created_at DESC, id DESC",
    )?;
    Ok(st
        .query_map([user_id], entry)?
        .collect::<Result<Vec<_>, _>>()?)
}

const STOP: &[&str] = &[
    "the",
    "and",
    "for",
    "are",
    "was",
    "with",
    "that",
    "this",
    "what",
    "who",
    "how",
    "when",
    "where",
    "which",
    "you",
    "your",
    "our",
    "can",
    "does",
    "did",
    "has",
    "have",
    "from",
    "about",
    "scene",
    "script",
    "screenplay",
    "answer",
    "answers",
    "tell",
    "show",
    "please",
];

/// The content words of a text: three letters or more, not a filler word.
fn content_words(text: &str) -> HashSet<String> {
    normalize(text)
        .split(' ')
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(w))
        .map(str::to_string)
        .collect()
}

/// The entries to offer this turn, best match first. `ids` are the things the message and the
/// conversation focus name.
pub fn retrieve(
    c: &Connection,
    user_id: &str,
    message: &str,
    ids: &[String],
) -> AppResult<Vec<MemoryEntry>> {
    let words = content_words(message);
    let mut scored: Vec<(usize, MemoryEntry)> = list(c, user_id)?
        .into_iter()
        .filter_map(|e| {
            let about = e.entity_refs.iter().filter(|r| ids.contains(r)).count();
            let shared = content_words(&e.text).intersection(&words).count();
            let score = 3 * about + shared;
            (score > 0 || e.kind == Kind::Preference).then_some((score, e))
        })
        .collect();
    // `list` is newest first and the sort is stable, so equal scores keep the newest first.
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    let mut out = Vec::new();
    let mut tokens = 0;
    for (_, e) in scored {
        let t = approx_tokens(&e.text);
        if out.len() >= RETRIEVE_COUNT || tokens + t > RETRIEVE_TOKENS {
            break;
        }
        tokens += t;
        out.push(e);
    }
    Ok(out)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Saved {
    New(Box<MemoryEntry>),
    /// The same words are already remembered.
    Duplicate,
    /// The person has [`MAX_ENTRIES`] entries; nothing is dropped behind their back.
    Full,
}

/// Save a candidate for `user`.
pub fn save(
    c: &Connection,
    user: &str,
    cand: &Candidate,
    entity_refs: &[String],
    source_request_id: &str,
) -> AppResult<Saved> {
    let have = list(c, user)?;
    let key = normalize(&cand.text);
    if have.iter().any(|e| normalize(&e.text) == key) {
        return Ok(Saved::Duplicate);
    }
    if have.len() >= MAX_ENTRIES {
        return Ok(Saved::Full);
    }
    let e = MemoryEntry {
        id: new_id(),
        text: cand.text.clone(),
        kind: cand.kind,
        entity_refs: entity_refs.to_vec(),
        source_request_id: Some(source_request_id.to_string()),
        created_at: now_ms(),
    };
    c.execute(
        "INSERT INTO ai_memory(id, user_id, text, kind, entity_refs_json, source_request_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            e.id,
            user,
            e.text,
            e.kind.code(),
            serde_json::to_string(&e.entity_refs).unwrap_or_else(|_| "[]".into()),
            e.source_request_id,
            e.created_at
        ],
    )?;
    Ok(Saved::New(Box::new(e)))
}

/// Forget one entry of `user`. The words are wiped at once and the row only keeps its id and
/// times; it does not go to Recently Deleted, because the person asked to be forgotten. `false`
/// when there is no such entry (or it belongs to someone else).
pub fn delete(c: &Connection, user: &str, id: &str) -> AppResult<bool> {
    let changed = c.execute(
        "UPDATE ai_memory SET deleted_at = ?1, text = '', entity_refs_json = '[]'
         WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL",
        params![now_ms(), id, user],
    )?;
    Ok(changed > 0)
}
