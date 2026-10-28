//! Story page: one writing page (Story > Story tab) for the writer's free text.
//!
//! The page is not linked to any card, scene, screenplay or other data. It is one row of
//! `story_page`, created by the first save. Saving is a normal project write (one transaction,
//! `rev` checked) but never enters the undo history (typing is undone in the editor itself),
//! never writes an activity entry, is not indexed for search, and is not offered to the assistant.
//! Backups and full project packages carry the table whole; exchange packages do not carry it.
//!
//! The text may carry formatting (bold, italic, underline, highlight, a size per stretch of text;
//! owner request 2026-10-27). `body` keeps the plain text; `format` holds the runs of each line
//! ([`openframe_import_export::prose`]). A save whose formatting does not match its text is
//! refused; a stored formatting that does not match (an older build saved plain text over it) is
//! ignored on read, so the text is never lost or garbled.

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use openframe_import_export::prose::{self, ProseLine};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;

const ROW_ID: &str = "story";
/// The longest page that can be saved (5 MB of UTF-8).
pub(crate) const PAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
/// The longest formatting that can be saved (its JSON repeats the text and adds the styles).
const FORMAT_MAX_BYTES: usize = 4 * PAGE_MAX_BYTES;

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
    /// The formatting as JSON (one list of runs per line of `text`), or null for plain text.
    /// Always matches `text` when present.
    pub format: Option<String>,
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
    /// The formatting as JSON (see [`StoryPageDto::format`]); null or absent for plain text.
    #[serde(default)]
    #[ts(optional)]
    pub format: Option<String>,
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

/// The stored formatting when it parses and matches the text.
fn matching(text: &str, format: Option<&str>) -> Option<Vec<ProseLine>> {
    let lines: Vec<ProseLine> = serde_json::from_str(format?).ok()?;
    prose::check(text, &lines).ok()?;
    Some(lines)
}

/// The lines of the page with their formatting (plain lines when it has none).
pub(crate) fn lines(page: &StoryPageDto) -> Vec<ProseLine> {
    matching(&page.text, page.format.as_deref()).unwrap_or_else(|| prose::plain_lines(&page.text))
}

pub(crate) fn read(c: &Connection) -> AppResult<StoryPageDto> {
    let row = c
        .query_row(
            "SELECT body, format, rev, updated_at FROM story_page WHERE id = ?1",
            [ROW_ID],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get(2)?,
                    r.get(3)?,
                ))
            },
        )
        .optional()?;
    Ok(match row {
        Some((text, format, rev, updated_at)) => {
            let format = format.filter(|f| matching(&text, Some(f)).is_some());
            StoryPageDto {
                text,
                format,
                rev,
                updated_at,
            }
        }
        None => legacy_page(c)?.unwrap_or(StoryPageDto {
            text: String::new(),
            format: None,
            rev: 0,
            updated_at: 0,
        }),
    })
}

/// A Mac build of 2026-10-20 to 2026-10-27 saved the page under a random row id. Its text is read
/// as an unsaved page (rev 0), so the first save writes it to the fixed row and nothing is lost.
fn legacy_page(c: &Connection) -> AppResult<Option<StoryPageDto>> {
    Ok(c.query_row(
        "SELECT body, updated_at FROM story_page WHERE id <> ?1 ORDER BY updated_at DESC LIMIT 1",
        [ROW_ID],
        |r| {
            Ok(StoryPageDto {
                text: r.get(0)?,
                format: None,
                rev: 0,
                updated_at: r.get(1)?,
            })
        },
    )
    .optional()?)
}

/// Check the formatting sent with a save. Returns the compact JSON to store, or none when the
/// page carries no formatting at all.
fn checked_format(text: &str, format: Option<&str>) -> AppResult<Option<String>> {
    let Some(raw) = format else { return Ok(None) };
    if raw.len() > FORMAT_MAX_BYTES {
        return Err(AppError::validation(
            "format",
            "The formatting of the Story page is too large to save. Remove some formatting, then try again.",
        ));
    }
    let lines: Vec<ProseLine> = serde_json::from_str(raw).map_err(|e| {
        AppError::validation(
            "format",
            "The formatting of the Story page could not be read.",
        )
        .with_detail(e.to_string())
    })?;
    prose::check(text, &lines).map_err(|why| {
        AppError::validation(
            "format",
            "The formatting of the Story page does not match its text. The text was not saved; try again.",
        )
        .with_detail(why)
    })?;
    if !prose::any_styled(&lines) {
        return Ok(None);
    }
    serde_json::to_string(&lines)
        .map(Some)
        .map_err(|e| AppError::internal(format!("story page format: {e}")))
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
    let format = checked_format(&text, a.format.as_deref())?;
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
            if current.rev > 0 && current.text == text && current.format == format {
                return Ok(StoryPageSaved {
                    rev: current.rev,
                    updated_at: current.updated_at,
                });
            }
            tx.conn().execute(
                "INSERT INTO story_page(id, body, format, created_at, updated_at, rev) VALUES (?1, ?2, ?3, ?4, ?4, 1)
                 ON CONFLICT(id) DO UPDATE SET body = excluded.body, format = excluded.format,
                   updated_at = excluded.updated_at, rev = rev + 1",
                params![ROW_ID, text, format, now],
            )?;
            Ok(StoryPageSaved {
                rev: current.rev + 1,
                updated_at: now,
            })
        },
    )
}
