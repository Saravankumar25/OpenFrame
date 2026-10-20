//! Story page: one plain writing page (Story > Story tab) for the writer's free text.
//!
//! The page is not linked to any card, scene, screenplay or other data. It is one row of
//! `story_page`, created by the first save. Saving is a normal project write (one transaction,
//! `rev` checked) but never enters the undo history (typing is undone in the text area itself),
//! never writes an activity entry, is not indexed for search, and is not offered to the assistant.
//! Backups and full project packages carry the table whole; exchange packages do not carry it.

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;

const ROW_ID: &str = "story";
/// The longest page that can be saved (5 MB of UTF-8).
pub(crate) const PAGE_MAX_BYTES: usize = 5 * 1024 * 1024;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("story.page.get", get)
        .meta(M::read("The text of the Story page.").hidden(h::FREE_WRITING));
    r.command("story.page.save", save)
        .meta(M::edit("Save the text of the Story page.").hidden(h::FREE_WRITING));
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryPageGetArgs {}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryPageDto {
    pub text: String,
    /// 0 until the first save.
    #[ts(type = "number")]
    pub rev: i64,
    /// Milliseconds since the epoch; 0 until the first save.
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryPageSaveArgs {
    pub text: String,
    /// The `rev` the writer's screen was based on (0 before the first save).
    #[ts(type = "number")]
    pub expected_rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryPageSaved {
    #[ts(type = "number")]
    pub rev: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// Line endings are stored as `\n` whatever the screen sent.
pub(crate) fn clean(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(crate) fn read(c: &Connection) -> AppResult<StoryPageDto> {
    let row = c
        .query_row(
            "SELECT body, rev, updated_at FROM story_page WHERE id = ?1",
            [ROW_ID],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((text, rev, updated_at)) => StoryPageDto {
            text,
            rev,
            updated_at,
        },
        None => StoryPageDto {
            text: String::new(),
            rev: 0,
            updated_at: 0,
        },
    })
}

fn get(core: &AppCore, actor: &Actor, _: StoryPageGetArgs) -> AppResult<StoryPageDto> {
    actor.require(Capability::View, "view this project")?;
    core.project()?.store.read(read)
}

fn save(core: &AppCore, actor: &Actor, a: StoryPageSaveArgs) -> AppResult<StoryPageSaved> {
    actor.require(Capability::Edit, "edit this project")?;
    let text = clean(&a.text);
    if text.len() > PAGE_MAX_BYTES {
        return Err(AppError::validation(
            "text",
            "The Story page is too long to save (over 5 MB). Move part of it to another document, then try again.",
        ));
    }
    let expected = a.expected_rev;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.page.save",
            "Saved the Story page",
            Capability::Edit,
        )
        .not_undoable()
        .quiet(),
        |tx| {
            let now = now_ms();
            let current = read(tx.conn())?;
            if current.rev != expected {
                return Err(AppError::stale("Story page"));
            }
            if current.rev > 0 && current.text == text {
                return Ok(StoryPageSaved {
                    rev: current.rev,
                    updated_at: current.updated_at,
                });
            }
            tx.conn().execute(
                "INSERT INTO story_page(id, body, created_at, updated_at, rev) VALUES (?1, ?2, ?3, ?3, 1)
                 ON CONFLICT(id) DO UPDATE SET body = excluded.body, updated_at = excluded.updated_at, rev = rev + 1",
                params![ROW_ID, text, now],
            )?;
            Ok(StoryPageSaved {
                rev: current.rev + 1,
                updated_at: now,
            })
        },
    )
}
