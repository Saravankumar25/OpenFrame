//! Story workspace (FSD §7–14, §18, §25, §89–91; FSD-STORY-001..030).
//!
//! * `tree`: containers, shared sibling order, moves (identity-preserving)
//! * `board`: acts, sequences, beats, scene cards, parking lot, multi-select
//! * `display`: Card Display Settings (per-user view setting, never content)
//! * `links`: card to screenplay scene links that follow scene lineage
//! * `build`: Build Screenplay from the Beat Board (never overwrites)
//! * `characters`: character directory, relationships, manual card links
//! * `timeline`: Story Day assignment on screenplay scenes (never reorders)
//! * `episodes`: seasons and episodes for Episodic and Series projects
//! * `page`: the plain free-writing Story page (not linked to any other data)
//! * `index`: search indexers and Recently Deleted handlers
//!
//! Source-of-truth rules: the Beat Board is a reference layer. Nothing here
//! rewrites screenplay text; Build Screenplay only ever creates a NEW
//! screenplay or a NEW draft, and "Apply order" is an explicit, confirmed action.

use openframe_domain::{AppError, AppResult};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::registry::Registry;

pub mod board;
pub mod build;
pub mod characters;
pub mod display;
pub mod episodes;
pub mod index;
pub mod links;
pub mod page;
pub mod timeline;
pub mod tree;

pub fn register(r: &mut Registry) {
    r.module("Story");
    board::register(r);
    display::register(r);
    build::register(r);
    characters::register(r);
    timeline::register(r);
    episodes::register(r);
    page::register(r);
    index::register(r);
}

pub(crate) const TITLE_MAX: usize = 200;
pub(crate) const HEADING_MAX: usize = 200;
// Capacity of a Scene Card (Beat Board card redesign; limits are written in
// docs/engineering/33-production-depth-contract.md section 1 and tested in tests/story_cards.rs).
/// Short description: what the minimized card shows. Bytes, as before.
pub(crate) const DESC_MAX_BYTES: usize = 20_000;
/// Long description: scene-development material, counted in characters.
pub(crate) const LONG_DESC_MAX_CHARS: usize = 100_000;
/// Notes on a card, beat, act or sequence.
pub(crate) const NOTES_MAX_BYTES: usize = 500_000;
/// Reference attachments on one Scene Card, beat or sequence.
pub(crate) const ATTACHMENTS_MAX_PER_OWNER: i64 = 50;

/// Card/beat colour tokens offered by the UI (never meaningful on their own:
/// colour is decoration, not status).
pub(crate) const COLORS: &[&str] = &[
    "yellow", "orange", "red", "pink", "purple", "blue", "teal", "green", "gray",
];

pub(crate) fn clean_color(v: Option<String>) -> AppResult<Option<String>> {
    match v
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
    {
        None => Ok(None),
        Some(c) if COLORS.contains(&c.as_str()) => Ok(Some(c)),
        Some(_) => Err(AppError::invalid_input(
            "Choose one of the offered colours.",
        )),
    }
}

pub(crate) fn ensure_live(c: &Connection, table: &str, id: &str, what: &str) -> AppResult<()> {
    let ok: bool = c.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1 AND deleted_at IS NULL)"),
        [id],
        |r| r.get(0),
    )?;
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found(what))
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryIdArgs {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCreated {
    pub id: String,
}
