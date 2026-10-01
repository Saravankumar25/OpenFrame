//! Comments and Private Notes (FSD §23, §94; Security §8, §10.2).
//!
//! Comments attach to any supported object by `target_type` (the canonical table
//! name of the target, e.g. `story_scene_card`, `screenplay_scene`, `location`)
//! and `target_id`. Screenplay *text* comments target `screenplay_element` and
//! carry a text anchor (element id + UTF-16 range + surrounding context) so they
//! can be relocated after edits; when the anchored text no longer exists the
//! comment is flagged "context moved" and links to the nearest surviving scene —
//! it never disappears (FSD §23.4).
//!
//! Replies append to the same thread (`parent_id`). Resolve never deletes.
//!
//! Private notes are a separate authorization boundary: every read and write is
//! filtered by `owner_user_id = actor.user_id`; another user's note behaves as if
//! it did not exist (no existence leak), it is searchable only by its owner, and
//! it never records project activity.

use std::collections::{BTreeSet, HashMap};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{body_text, require_id};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Comments & Private Notes");
    r.command("comment.create", create).meta(M::command(
        Capability::Comment,
        "Add a comment to a project object (scene text, Scene Card, shot, …).",
    ));
    r.command("comment.reply", reply)
        .meta(M::command(Capability::Comment, "Reply to a comment."));
    r.query("comment.list", list).meta(M::read(
        "Comment threads on an object, scene, draft or review round.",
    ));
    r.command("comment.update", update).meta(M::command(
        Capability::Comment,
        "Edit a comment's text or discussion status.",
    ));
    r.command("comment.resolve", resolve).meta(M::command(
        Capability::ResolveComments,
        "Resolve a comment thread.",
    ));
    r.command("comment.reopen", reopen).meta(M::command(
        Capability::ResolveComments,
        "Reopen a resolved comment thread.",
    ));
    r.command("comment.delete", delete).meta(
        M::command(
            Capability::Comment,
            "Delete a comment (own comments; others need SoftDelete).",
        )
        .destructive(),
    );
    r.command("private_note.create", note_create)
        .meta(M::command(
            Capability::View,
            "Create one of your own Private Notes (visible only to you).",
        ));
    r.query("private_note.list", note_list)
        .meta(M::read("Your own Private Notes (never another user's)."));
    r.command("private_note.update", note_update)
        .meta(M::command(
            Capability::View,
            "Edit one of your own Private Notes.",
        ));
    r.command("private_note.delete", note_delete).meta(
        M::command(
            Capability::View,
            "Delete one of your own Private Notes (recoverable).",
        )
        .destructive(),
    );
    r.indexer("comment", index_comment);
    r.indexer("private_note", index_private_note);
    r.trash_handler(TrashHandler {
        object_type: "comment",
        table: "comment",
        label: "Comment",
        restore: None,
        purge: purge_comment,
    });
    r.trash_handler(TrashHandler {
        object_type: "private_note",
        table: "private_note",
        label: "Private note",
        restore: Some(restore_private_note),
        purge: purge_private_note,
    });
}

const MAX_BODY_BYTES: usize = 20_000;
/// Characters of surrounding text kept with a text anchor to help relocate it.
const CONTEXT_CHARS: usize = 32;

/// Comment statuses (FSD §23.3).
pub const STATUS_OPEN: &str = "Open";
pub const STATUS_DISCUSSION: &str = "In Discussion";
pub const STATUS_RESOLVED: &str = "Resolved";

/// Tables that can never be comment/private-note targets.
const NON_TARGETS: &[&str] = &[
    "comment",
    "private_note",
    "deleted_item",
    "project_member",
    "snapshot",
    "template",
    "asset",
];

// ------------------------------------------------------------------- DTOs

/// Text anchor inside one screenplay element. Offsets are UTF-16 code units
/// (the unit the editor uses), `start < end`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CommentTextAnchor {
    pub element_id: String,
    pub start: u32,
    pub end: u32,
    #[serde(default)]
    pub before: String,
    #[serde(default)]
    pub after: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: String,
    pub parent_id: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub scene_id: Option<String>,
    /// Display number of the scene in its draft (derived from order), when known.
    pub scene_number: Option<u32>,
    pub scene_heading: Option<String>,
    /// For "context moved"/deleted targets: the nearest surviving scene to navigate to.
    pub nearest_scene_id: Option<String>,
    pub anchor: Option<CommentTextAnchor>,
    pub quoted_text: Option<String>,
    pub body: String,
    pub status: String,
    pub context_moved: bool,
    /// The commented object was deleted (the comment stays in history, FSD §94).
    pub target_deleted: bool,
    pub review_round_id: Option<String>,
    pub author_user_id: String,
    pub author_name: String,
    pub is_mine: bool,
    pub resolved_by: Option<String>,
    #[ts(type = "number | null")]
    pub resolved_at: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CommentThread {
    pub comment: CommentDto,
    pub replies: Vec<CommentDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PrivateNoteDto {
    pub id: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub body: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

// ------------------------------------------------------------------- args

/// Anchor as sent by the editor: the server derives quoted text and context.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentAnchorInput {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentCreateArgs {
    /// Canonical table name of the target (e.g. `screenplay_scene`, `story_scene_card`).
    pub target_type: String,
    pub target_id: String,
    pub body: String,
    /// Only for `screenplay_element` targets: the selected text range.
    #[serde(default)]
    pub anchor: Option<CommentAnchorInput>,
    /// Scene context for non-screenplay targets (optional).
    #[serde(default)]
    pub scene_id: Option<String>,
    /// Review round; defaults to the open round of the target's draft, if any.
    #[serde(default)]
    pub review_round_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentReplyArgs {
    pub parent_id: String,
    pub body: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentListArgs {
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    /// All comments on a screenplay draft (draft, its scenes and its text).
    #[serde(default)]
    pub draft_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub review_round_id: Option<String>,
    /// "open" (Open + In Discussion), "resolved", or "all" (default).
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub body: Option<String>,
    /// "Open" or "In Discussion" (use comment.resolve to resolve).
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommentIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePrivateNoteArgs {
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
    pub body: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListPrivateNotesArgs {
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePrivateNoteArgs {
    pub id: String,
    pub body: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrivateNoteIdArgs {
    pub id: String,
}

// ------------------------------------------------------------ text helpers

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Substring by UTF-16 offsets; None when out of range or splitting a surrogate pair.
pub(crate) fn utf16_slice(s: &str, start: usize, end: usize) -> Option<String> {
    let units: Vec<u16> = s.encode_utf16().collect();
    if start > end || end > units.len() {
        return None;
    }
    String::from_utf16(&units[start..end]).ok()
}

/// All occurrences of `needle` in `hay`, as UTF-16 start offsets.
fn find_all_utf16(hay: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return vec![];
    }
    hay.match_indices(needle)
        .map(|(b, _)| utf16_len(&hay[..b]))
        .collect()
}

fn tail_chars(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    chars[chars.len().saturating_sub(n)..].iter().collect()
}

fn head_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn excerpt(s: &str, n: usize) -> String {
    let one_line: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > n {
        format!("{}…", head_chars(&one_line, n))
    } else {
        one_line
    }
}

// -------------------------------------------------------- target validation

fn is_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn table_exists(c: &Connection, table: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |r| r.get(0),
    )?)
}

fn has_column(c: &Connection, table: &str, col: &str) -> AppResult<bool> {
    let mut stmt = c.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    Ok(cols.iter().any(|c| c == col))
}

/// Validate a comment/private-note target type (a canonical table name).
fn check_target_type(c: &Connection, target_type: &str) -> AppResult<()> {
    let ok = is_identifier(target_type)
        && !NON_TARGETS.contains(&target_type)
        && !["sys_", "search_", "sqlite_", "_"]
            .iter()
            .any(|p| target_type.starts_with(p))
        && table_exists(c, target_type)?
        && has_column(c, target_type, "id")?;
    if ok {
        Ok(())
    } else {
        Err(AppError::invalid_input(
            "Comments can't be attached to that kind of item.",
        ))
    }
}

/// Returns Some(deleted) when the target row exists, None when it doesn't.
fn target_state(c: &Connection, target_type: &str, target_id: &str) -> AppResult<Option<bool>> {
    // `target_type` comes from stored comment rows (possibly a received project): it is
    // interpolated below, so it must be a plain identifier.
    if !is_identifier(target_type) || !table_exists(c, target_type)? {
        return Ok(None);
    }
    if has_column(c, target_type, "deleted_at")? {
        Ok(c.query_row(
            &format!("SELECT deleted_at IS NOT NULL FROM \"{target_type}\" WHERE id=?1"),
            [target_id],
            |r| r.get(0),
        )
        .optional()?)
    } else {
        Ok(c.query_row(
            &format!("SELECT 0 FROM \"{target_type}\" WHERE id=?1"),
            [target_id],
            |r| r.get::<_, bool>(0),
        )
        .optional()?)
    }
}

fn check_target(c: &Connection, target_type: &str, target_id: &str) -> AppResult<()> {
    check_target_type(c, target_type)?;
    match target_state(c, target_type, target_id)? {
        Some(false) => Ok(()),
        _ => Err(AppError::not_found("item")),
    }
}

/// Scene that owns a screenplay element / is a screenplay scene.
fn scene_for_target(
    c: &Connection,
    target_type: &str,
    target_id: &str,
) -> AppResult<Option<String>> {
    Ok(match target_type {
        "screenplay_scene" => Some(target_id.to_string()),
        "screenplay_element" => c
            .query_row(
                "SELECT scene_id FROM screenplay_element WHERE id=?1",
                [target_id],
                |r| r.get(0),
            )
            .optional()?,
        _ => None,
    })
}

fn draft_for_scene(c: &Connection, scene_id: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT draft_id FROM screenplay_scene WHERE id=?1",
        [scene_id],
        |r| r.get(0),
    )
    .optional()?)
}

fn open_round_for_draft(c: &Connection, draft_id: &str) -> AppResult<Option<String>> {
    Ok(c
        .query_row(
            "SELECT id FROM review_round WHERE draft_id=?1 AND status='Open' AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1",
            [draft_id],
            |r| r.get(0),
        )
        .optional()?)
}

// ---------------------------------------------------------------- loading

struct Row {
    id: String,
    parent_id: Option<String>,
    target_type: String,
    target_id: String,
    scene_id: Option<String>,
    anchor_json: Option<String>,
    quoted_text: Option<String>,
    body: String,
    status: String,
    context_moved: bool,
    review_round_id: Option<String>,
    author_user_id: String,
    author_name: String,
    resolved_by: Option<String>,
    resolved_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
    rev: i64,
}

const COLS: &str = "id, parent_id, target_type, target_id, scene_id, anchor_json, quoted_text, body, status, context_moved,
                    review_round_id, author_user_id, author_name, resolved_by, resolved_at, created_at, updated_at, rev";

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        target_type: r.get(2)?,
        target_id: r.get(3)?,
        scene_id: r.get(4)?,
        anchor_json: r.get(5)?,
        quoted_text: r.get(6)?,
        body: r.get(7)?,
        status: r.get(8)?,
        context_moved: r.get(9)?,
        review_round_id: r.get(10)?,
        author_user_id: r.get(11)?,
        author_name: r.get(12)?,
        resolved_by: r.get(13)?,
        resolved_at: r.get(14)?,
        created_at: r.get(15)?,
        updated_at: r.get(16)?,
        rev: r.get(17)?,
    })
}

fn load_row(c: &Connection, id: &str) -> AppResult<Row> {
    c.query_row(
        &format!("SELECT {COLS} FROM comment WHERE id=?1 AND deleted_at IS NULL"),
        [id],
        map_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("comment"))
}

/// Scene facts used when presenting comments (number, heading, liveness).
struct SceneInfo {
    draft_id: String,
    position: i64,
    heading: String,
    deleted: bool,
}

struct Presenter<'c> {
    c: &'c Connection,
    scenes: HashMap<String, Option<SceneInfo>>,
    numbers: HashMap<String, HashMap<String, u32>>,
    targets: HashMap<(String, String), Option<bool>>,
}

impl<'c> Presenter<'c> {
    fn new(c: &'c Connection) -> Self {
        Self {
            c,
            scenes: HashMap::new(),
            numbers: HashMap::new(),
            targets: HashMap::new(),
        }
    }

    fn scene(&mut self, id: &str) -> AppResult<Option<&SceneInfo>> {
        if !self.scenes.contains_key(id) {
            let info = self
                .c
                .query_row(
                    "SELECT draft_id, position, heading, deleted_at IS NOT NULL FROM screenplay_scene WHERE id=?1",
                    [id],
                    |r| Ok(SceneInfo { draft_id: r.get(0)?, position: r.get(1)?, heading: r.get(2)?, deleted: r.get(3)? }),
                )
                .optional()?;
            self.scenes.insert(id.to_string(), info);
        }
        Ok(self.scenes.get(id).and_then(|s| s.as_ref()))
    }

    fn numbers(&mut self, draft_id: &str) -> AppResult<&HashMap<String, u32>> {
        if !self.numbers.contains_key(draft_id) {
            let mut stmt = self
                .c
                .prepare("SELECT id FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id")?;
            let ids: Vec<String> = stmt
                .query_map([draft_id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            let map = ids
                .into_iter()
                .enumerate()
                .map(|(i, id)| (id, (i + 1) as u32))
                .collect();
            self.numbers.insert(draft_id.to_string(), map);
        }
        Ok(&self.numbers[draft_id])
    }

    fn nearest_live_scene(&self, draft_id: &str, position: i64) -> AppResult<Option<String>> {
        Ok(self
            .c
            .query_row(
                "SELECT id FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY abs(position - ?2), position LIMIT 1",
                params![draft_id, position],
                |r| r.get(0),
            )
            .optional()?)
    }

    fn target_deleted(&mut self, tt: &str, tid: &str) -> AppResult<bool> {
        let key = (tt.to_string(), tid.to_string());
        if !self.targets.contains_key(&key) {
            let st = target_state(self.c, tt, tid)?;
            self.targets.insert(key.clone(), st);
        }
        Ok(!matches!(self.targets[&key], Some(false)))
    }

    fn present(&mut self, row: Row, me: &str) -> AppResult<CommentDto> {
        let anchor: Option<CommentTextAnchor> = row
            .anchor_json
            .as_deref()
            .and_then(|j| serde_json::from_str(j).ok());
        let mut target_deleted = self.target_deleted(&row.target_type, &row.target_id)?;
        let mut scene_number = None;
        let mut scene_heading = None;
        let mut nearest = None;
        if let Some(sid) = row.scene_id.clone() {
            let facts = self
                .scene(&sid)?
                .map(|s| (s.draft_id.clone(), s.position, s.heading.clone(), s.deleted));
            match facts {
                Some((draft_id, position, heading, deleted)) => {
                    if deleted {
                        target_deleted = target_deleted || row.target_type == "screenplay_scene";
                        nearest = self.nearest_live_scene(&draft_id, position)?;
                    } else {
                        scene_number = self.numbers(&draft_id)?.get(&sid).copied();
                        scene_heading = Some(heading);
                        nearest = Some(sid.clone());
                    }
                }
                None => target_deleted = true,
            }
        }
        // A text comment whose element vanished is "context moved" even if the
        // flag was not persisted (e.g. the element was purged elsewhere).
        let context_moved =
            row.context_moved || (row.target_type == "screenplay_element" && target_deleted);
        Ok(CommentDto {
            is_mine: row.author_user_id == me,
            id: row.id,
            parent_id: row.parent_id,
            target_type: row.target_type,
            target_id: row.target_id,
            scene_id: row.scene_id,
            scene_number,
            scene_heading,
            nearest_scene_id: nearest,
            anchor,
            quoted_text: row.quoted_text,
            body: row.body,
            status: row.status,
            context_moved,
            target_deleted,
            review_round_id: row.review_round_id,
            author_user_id: row.author_user_id,
            author_name: row.author_name,
            resolved_by: row.resolved_by,
            resolved_at: row.resolved_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            rev: row.rev,
        })
    }
}

/// Threads (root comment + replies) matching a WHERE clause on root comments.
fn threads(
    c: &Connection,
    me: &str,
    where_sql: &str,
    params: Vec<SqlValue>,
    status: Option<&str>,
) -> AppResult<Vec<CommentThread>> {
    let status_sql = match status {
        Some("open") => " AND status IN ('Open','In Discussion')",
        Some("resolved") => " AND status = 'Resolved'",
        Some("all") | None => "",
        Some(_) => return Err(AppError::invalid_input("Unknown comment filter.")),
    };
    let sql = format!(
        "SELECT {COLS} FROM comment WHERE parent_id IS NULL AND deleted_at IS NULL AND ({where_sql}){status_sql} ORDER BY created_at, id"
    );
    let mut stmt = c.prepare(&sql)?;
    let roots: Vec<Row> = stmt
        .query_map(params_from_iter(params.iter()), map_row)?
        .collect::<Result<_, _>>()?;
    let mut p = Presenter::new(c);
    let mut out = Vec::with_capacity(roots.len());
    let mut reply_stmt =
        c.prepare(&format!("SELECT {COLS} FROM comment WHERE parent_id=?1 AND deleted_at IS NULL ORDER BY created_at, id"))?;
    for root in roots {
        let replies: Vec<Row> = reply_stmt
            .query_map([&root.id], map_row)?
            .collect::<Result<_, _>>()?;
        let comment = p.present(root, me)?;
        let replies = replies
            .into_iter()
            .map(|r| p.present(r, me))
            .collect::<AppResult<Vec<_>>>()?;
        out.push(CommentThread { comment, replies });
    }
    Ok(out)
}

pub(crate) fn thread_by_id(c: &Connection, me: &str, id: &str) -> AppResult<CommentThread> {
    threads(c, me, "id = ?1", vec![text(id)], None)?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("comment"))
}

/// Open (unresolved) root comments on a screenplay draft, its scenes and its text.
pub(crate) fn open_count_for_draft(c: &Connection, draft_id: &str) -> AppResult<i64> {
    Ok(c.query_row(
        "SELECT count(*) FROM comment WHERE parent_id IS NULL AND deleted_at IS NULL AND status IN ('Open','In Discussion')
           AND ((target_type='screenplay_draft' AND target_id=?1)
                OR scene_id IN (SELECT id FROM screenplay_scene WHERE draft_id=?1))",
        [draft_id],
        |r| r.get(0),
    )?)
}

// ---------------------------------------------------------------- anchoring

fn build_anchor(
    element_id: &str,
    element_text: &str,
    start: usize,
    end: usize,
) -> AppResult<(CommentTextAnchor, String)> {
    let len = utf16_len(element_text);
    if start >= end || end > len {
        return Err(AppError::invalid_input(
            "Select some text in the screenplay to comment on it.",
        ));
    }
    let quoted = utf16_slice(element_text, start, end).ok_or_else(|| {
        AppError::invalid_input("Select some text in the screenplay to comment on it.")
    })?;
    let before = utf16_slice(element_text, 0, start)
        .map(|s| tail_chars(&s, CONTEXT_CHARS))
        .unwrap_or_default();
    let after = utf16_slice(element_text, end, len)
        .map(|s| head_chars(&s, CONTEXT_CHARS))
        .unwrap_or_default();
    Ok((
        CommentTextAnchor {
            element_id: element_id.to_string(),
            start: start as u32,
            end: end as u32,
            before,
            after,
        },
        quoted,
    ))
}

enum Located {
    At {
        element_id: String,
        start: usize,
        scene_id: String,
        text: String,
    },
    Moved,
}

/// Find where a quoted text now lives: same element (nearest occurrence to the
/// old offset, preferring matching context), otherwise another element of the
/// same scene; otherwise the context moved.
fn locate(
    c: &Connection,
    anchor: &CommentTextAnchor,
    quoted: &str,
    fallback_scene: Option<&str>,
) -> AppResult<Located> {
    if quoted.is_empty() {
        return Ok(Located::Moved);
    }
    let element: Option<(String, String)> = c
        .query_row(
            "SELECT scene_id, text FROM screenplay_element WHERE id=?1",
            [&anchor.element_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let score = |text: &str, pos: usize| -> (u8, usize) {
        // Prefer occurrences whose surrounding context still matches; then proximity.
        let end = pos + utf16_len(quoted);
        let before_ok = anchor.before.is_empty()
            || utf16_slice(text, 0, pos)
                .map(|b| b.ends_with(&anchor.before))
                .unwrap_or(false);
        let after_ok = anchor.after.is_empty()
            || utf16_slice(text, end, utf16_len(text))
                .map(|a| a.starts_with(&anchor.after))
                .unwrap_or(false);
        let ctx = 2 - (before_ok as u8) - (after_ok as u8);
        (ctx, pos.abs_diff(anchor.start as usize))
    };
    if let Some((scene_id, text)) = &element {
        let hits = find_all_utf16(text, quoted);
        if let Some(best) = hits.into_iter().min_by_key(|p| score(text, *p)) {
            return Ok(Located::At {
                element_id: anchor.element_id.clone(),
                start: best,
                scene_id: scene_id.clone(),
                text: text.clone(),
            });
        }
    }
    let scene = element.as_ref().map(|(s, _)| s.as_str()).or(fallback_scene);
    if let Some(scene_id) = scene {
        let mut stmt = c.prepare(
            "SELECT id, text FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
        )?;
        let rows: Vec<(String, String)> = stmt
            .query_map([scene_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut best: Option<((u8, usize), String, usize, String)> = None;
        for (id, text) in rows {
            for pos in find_all_utf16(&text, quoted) {
                let s = score(&text, pos);
                if best.as_ref().map(|b| s < b.0).unwrap_or(true) {
                    best = Some((s, id.clone(), pos, text.clone()));
                }
            }
        }
        if let Some((_, id, pos, text)) = best {
            return Ok(Located::At {
                element_id: id,
                start: pos,
                scene_id: scene_id.to_string(),
                text,
            });
        }
    }
    Ok(Located::Moved)
}

/// Re-evaluate text anchors after screenplay edits (called inside the same
/// mutation, so undo restores anchors together with the text). Considers
/// comments anchored to `elements`, plus "context moved" comments in `scenes`
/// whose text may have reappeared.
pub(crate) fn reanchor(
    tx: &Tx<'_>,
    elements: &BTreeSet<String>,
    scenes: &BTreeSet<String>,
) -> AppResult<()> {
    if elements.is_empty() && scenes.is_empty() {
        return Ok(());
    }
    let c = tx.conn();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    {
        let mut by_target = c.prepare(
            "SELECT id FROM comment WHERE target_type='screenplay_element' AND target_id=?1 AND parent_id IS NULL AND deleted_at IS NULL",
        )?;
        for e in elements {
            for id in by_target.query_map([e], |r| r.get::<_, String>(0))? {
                ids.insert(id?);
            }
        }
        let mut moved = c.prepare(
            "SELECT id FROM comment WHERE target_type='screenplay_element' AND context_moved=1 AND scene_id=?1
               AND parent_id IS NULL AND deleted_at IS NULL",
        )?;
        for s in scenes {
            for id in moved.query_map([s], |r| r.get::<_, String>(0))? {
                ids.insert(id?);
            }
        }
    }
    for id in ids {
        let row = load_row(c, &id)?;
        let Some(anchor) = row
            .anchor_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<CommentTextAnchor>(j).ok())
        else {
            continue;
        };
        let quoted = row.quoted_text.clone().unwrap_or_default();
        match locate(c, &anchor, &quoted, row.scene_id.as_deref())? {
            Located::At {
                element_id,
                start,
                scene_id,
                text: el_text,
            } => {
                let (new_anchor, _) =
                    build_anchor(&element_id, &el_text, start, start + utf16_len(&quoted))?;
                let unchanged = new_anchor == anchor
                    && !row.context_moved
                    && row.target_id == element_id
                    && row.scene_id.as_deref() == Some(scene_id.as_str());
                if !unchanged {
                    let json = serde_json::to_string(&new_anchor)
                        .map_err(|e| AppError::internal(e.to_string()))?;
                    update_fields(
                        c,
                        "comment",
                        &id,
                        &[
                            ("target_id", text(element_id.clone())),
                            ("scene_id", text(scene_id.clone())),
                            ("anchor_json", text(json)),
                            ("context_moved", SqlValue::Integer(0)),
                        ],
                        &["target_id", "scene_id", "anchor_json", "context_moved"],
                        None,
                        "comment",
                    )?;
                    // Replies follow their thread's target.
                    c.execute(
                        "UPDATE comment SET target_id=?1, scene_id=?2, updated_at=?3, rev=rev+1
                         WHERE parent_id=?4 AND (target_id IS NOT ?1 OR scene_id IS NOT ?2)",
                        params![element_id, scene_id, now_ms(), id],
                    )?;
                }
            }
            Located::Moved => {
                if !row.context_moved {
                    update_fields(
                        c,
                        "comment",
                        &id,
                        &[("context_moved", SqlValue::Integer(1))],
                        &["context_moved"],
                        None,
                        "comment",
                    )?;
                }
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ comments

fn clean_body(body: String) -> AppResult<String> {
    let body = body_text(body, "Comment", MAX_BODY_BYTES)?;
    if body.trim().is_empty() {
        return Err(AppError::required("Comment text"));
    }
    Ok(body.trim_end().to_string())
}

fn create(core: &AppCore, actor: &Actor, args: CommentCreateArgs) -> AppResult<CommentThread> {
    let body = clean_body(args.body)?;
    require_id(&args.target_id, "item")?;
    let s = core.project()?;
    let summary = format!("Commented: “{}”", excerpt(&body, 48));
    let id = s.store.mutate(
        actor,
        MutationMeta::new("comment.create", summary, Capability::Comment).target(&args.target_type, &args.target_id),
        |tx| {
            let c = tx.conn();
            check_target(c, &args.target_type, &args.target_id)?;
            let mut scene_id = scene_for_target(c, &args.target_type, &args.target_id)?;
            if scene_id.is_none()
                && let Some(sid) = &args.scene_id
            {
                require_id(sid, "scene")?;
                scene_id = Some(sid.clone());
            }
            let (anchor_json, quoted) = match (&args.anchor, args.target_type.as_str()) {
                (Some(a), "screenplay_element") => {
                    let el_text: String =
                        c.query_row("SELECT text FROM screenplay_element WHERE id=?1", [&args.target_id], |r| r.get(0))?;
                    let (anchor, quoted) = build_anchor(&args.target_id, &el_text, a.start as usize, a.end as usize)?;
                    (Some(serde_json::to_string(&anchor).map_err(|e| AppError::internal(e.to_string()))?), Some(quoted))
                }
                (Some(_), _) => return Err(AppError::invalid_input("Only screenplay text can have a text selection.")),
                (None, _) => (None, None),
            };
            let draft_id = match (&scene_id, args.target_type.as_str()) {
                (_, "screenplay_draft") => Some(args.target_id.clone()),
                (Some(sid), t) if t.starts_with("screenplay_") => draft_for_scene(c, sid)?,
                _ => None,
            };
            let round = match args.review_round_id {
                Some(r) => {
                    let ok: bool = c.query_row(
                        "SELECT EXISTS(SELECT 1 FROM review_round WHERE id=?1 AND deleted_at IS NULL)",
                        [&r],
                        |x| x.get(0),
                    )?;
                    if !ok {
                        return Err(AppError::not_found("review round"));
                    }
                    Some(r)
                }
                None => match &draft_id {
                    Some(d) => open_round_for_draft(c, d)?,
                    None => None,
                },
            };
            let id = new_id();
            let now = now_ms();
            c.execute(
                "INSERT INTO comment(id, review_round_id, parent_id, target_type, target_id, scene_id, anchor_json, quoted_text,
                                     body, status, author_user_id, author_name, created_at, updated_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, 'Open', ?9, ?10, ?11, ?11)",
                params![
                    id,
                    round,
                    args.target_type,
                    args.target_id,
                    scene_id,
                    anchor_json,
                    quoted,
                    body,
                    actor.user_id,
                    actor.display_name,
                    now
                ],
            )?;
            Ok(id)
        },
    )?;
    s.store.read(|c| thread_by_id(c, &actor.user_id, &id))
}

fn reply(core: &AppCore, actor: &Actor, args: CommentReplyArgs) -> AppResult<CommentThread> {
    let body = clean_body(args.body)?;
    let s = core.project()?;
    let root_id = s.store.mutate(
        actor,
        MutationMeta::new("comment.reply", format!("Replied: “{}”", excerpt(&body, 48)), Capability::Comment)
            .target("comment", &args.parent_id),
        |tx| {
            let c = tx.conn();
            let parent = load_row(c, &args.parent_id)?;
            if parent.parent_id.is_some() {
                return Err(AppError::invalid_input("Reply to the thread's first comment."));
            }
            let now = now_ms();
            c.execute(
                "INSERT INTO comment(id, review_round_id, parent_id, target_type, target_id, scene_id, body, status,
                                     author_user_id, author_name, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Open', ?8, ?9, ?10, ?10)",
                params![
                    new_id(),
                    parent.review_round_id,
                    parent.id,
                    parent.target_type,
                    parent.target_id,
                    parent.scene_id,
                    body,
                    actor.user_id,
                    actor.display_name,
                    now
                ],
            )?;
            tx.reindex("comment", &parent.id);
            Ok(parent.id)
        },
    )?;
    s.store.read(|c| thread_by_id(c, &actor.user_id, &root_id))
}

fn list(core: &AppCore, actor: &Actor, args: CommentListArgs) -> AppResult<Vec<CommentThread>> {
    actor.require(Capability::View, "view comments")?;
    let s = core.project()?;
    let status = args.status.as_deref();
    s.store.read(|c| {
        let mut clauses: Vec<String> = Vec::new();
        let mut params: Vec<SqlValue> = Vec::new();
        let mut push = |clause: &str, values: Vec<SqlValue>| {
            let mut clause = clause.to_string();
            for (i, v) in values.into_iter().enumerate() {
                params.push(v);
                clause = clause.replace(&format!("?{}", i + 1), &format!("?{}", params.len()));
            }
            clauses.push(clause);
        };
        if let Some(d) = &args.draft_id {
            push(
                "((target_type='screenplay_draft' AND target_id=?1) OR scene_id IN (SELECT id FROM screenplay_scene WHERE draft_id=?1))",
                vec![text(d.clone())],
            );
        }
        if let Some(tt) = &args.target_type {
            push("target_type = ?1", vec![text(tt.clone())]);
        }
        if let Some(tid) = &args.target_id {
            push("target_id = ?1", vec![text(tid.clone())]);
        }
        if let Some(sid) = &args.scene_id {
            push("scene_id = ?1", vec![text(sid.clone())]);
        }
        if let Some(r) = &args.review_round_id {
            push("review_round_id = ?1", vec![text(r.clone())]);
        }
        if clauses.is_empty() {
            return Err(AppError::invalid_input("Choose what to list comments for."));
        }
        threads(c, &actor.user_id, &clauses.join(" AND "), params, status)
    })
}

fn update(core: &AppCore, actor: &Actor, args: CommentUpdateArgs) -> AppResult<CommentThread> {
    let s = core.project()?;
    let body = args.body.map(clean_body).transpose()?;
    if let Some(st) = &args.status
        && st != STATUS_OPEN
        && st != STATUS_DISCUSSION
    {
        return Err(AppError::invalid_input("Use Resolve to resolve a comment."));
    }
    let summary = match (&body, &args.status) {
        (Some(_), _) => "Edited a comment".to_string(),
        (None, Some(st)) => format!("Marked a comment {st}"),
        (None, None) => return s.store.read(|c| thread_by_id(c, &actor.user_id, &args.id)),
    };
    let root = s.store.mutate(
        actor,
        MutationMeta::new("comment.update", summary, Capability::Comment)
            .target("comment", &args.id),
        |tx| {
            let c = tx.conn();
            let row = load_row(c, &args.id)?;
            let mut fields: Vec<(&str, SqlValue)> = Vec::new();
            if let Some(b) = &body {
                if row.author_user_id != actor.user_id {
                    return Err(AppError::permission_denied("edit other people's comments"));
                }
                fields.push(("body", text(b.clone())));
            }
            if let Some(st) = &args.status {
                if row.parent_id.is_some() {
                    return Err(AppError::invalid_input(
                        "Replies share their thread's status.",
                    ));
                }
                fields.push(("status", text(st.clone())));
                fields.push(("resolved_by", SqlValue::Null));
                fields.push(("resolved_at", SqlValue::Null));
            }
            update_fields(
                c,
                "comment",
                &args.id,
                &fields,
                &["body", "status", "resolved_by", "resolved_at"],
                args.expected_rev,
                "comment",
            )?;
            let root = row.parent_id.unwrap_or(row.id);
            tx.reindex("comment", &root);
            Ok(root)
        },
    )?;
    s.store.read(|c| thread_by_id(c, &actor.user_id, &root))
}

fn set_resolved(
    core: &AppCore,
    actor: &Actor,
    id: &str,
    resolved: bool,
) -> AppResult<CommentThread> {
    let s = core.project()?;
    let (action, summary) = if resolved {
        ("comment.resolve", "Resolved a comment")
    } else {
        ("comment.reopen", "Reopened a comment")
    };
    s.store.mutate(
        actor,
        MutationMeta::new(action, summary, Capability::ResolveComments).target("comment", id),
        |tx| {
            let c = tx.conn();
            let row = load_row(c, id)?;
            if row.parent_id.is_some() {
                return Err(AppError::invalid_input(
                    "Resolve the thread from its first comment.",
                ));
            }
            let fields: Vec<(&str, SqlValue)> = if resolved {
                if row.status == STATUS_RESOLVED {
                    return Ok(());
                }
                vec![
                    ("status", text(STATUS_RESOLVED)),
                    ("resolved_by", text(actor.display_name.clone())),
                    ("resolved_at", SqlValue::Integer(now_ms())),
                ]
            } else {
                if row.status != STATUS_RESOLVED {
                    return Ok(());
                }
                vec![
                    ("status", text(STATUS_OPEN)),
                    ("resolved_by", SqlValue::Null),
                    ("resolved_at", SqlValue::Null),
                ]
            };
            update_fields(
                c,
                "comment",
                id,
                &fields,
                &["status", "resolved_by", "resolved_at"],
                None,
                "comment",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| thread_by_id(c, &actor.user_id, id))
}

fn resolve(core: &AppCore, actor: &Actor, args: CommentIdArgs) -> AppResult<CommentThread> {
    set_resolved(core, actor, &args.id, true)
}

fn reopen(core: &AppCore, actor: &Actor, args: CommentIdArgs) -> AppResult<CommentThread> {
    set_resolved(core, actor, &args.id, false)
}

fn delete(core: &AppCore, actor: &Actor, args: CommentIdArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("comment.delete", "Deleted a comment", Capability::Comment)
            .target("comment", &args.id),
        |tx| {
            let c = tx.conn();
            let row = load_row(c, &args.id)?;
            if row.author_user_id != actor.user_id && !actor.role.allows(Capability::SoftDelete) {
                return Err(AppError::permission_denied(
                    "delete other people's comments",
                ));
            }
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "comment",
                    table: "comment",
                    id: &row.id,
                    title: Some(format!("Comment: {}", excerpt(&row.body, 60))),
                    parent_type: Some(&row.target_type),
                    parent_id: Some(row.target_id.clone()),
                    position: None,
                },
            )?;
            if let Some(p) = &row.parent_id {
                tx.reindex("comment", p);
            }
            Ok(())
        },
    )
}

fn purge_comment(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let parent: Option<Option<String>> = c
        .query_row(
            "SELECT parent_id FROM comment WHERE id=?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?;
    c.execute("DELETE FROM comment WHERE parent_id=?1", [&row.object_id])?;
    c.execute("DELETE FROM comment WHERE id=?1", [&row.object_id])?;
    if let Some(Some(p)) = parent {
        tx.reindex("comment", &p);
    }
    Ok(())
}

/// Permanently remove all comments (and replies) on the given targets. Used when
/// the owning objects are purged.
pub(crate) fn purge_for_targets(tx: &Tx<'_>, target_type: &str, ids: &[String]) -> AppResult<()> {
    let c = tx.conn();
    for id in ids {
        let mut stmt = c.prepare("SELECT id FROM comment WHERE target_type=?1 AND target_id=?2")?;
        let cids: Vec<String> = stmt
            .query_map(params![target_type, id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for cid in cids {
            c.execute("DELETE FROM comment WHERE parent_id=?1", [&cid])?;
            c.execute(
                "DELETE FROM deleted_item WHERE table_name='comment' AND object_id=?1",
                [&cid],
            )?;
            c.execute("DELETE FROM comment WHERE id=?1", [&cid])?;
        }
    }
    Ok(())
}

/// Where a comment (or private note) on a target opens. Uses the target's own
/// search navigation when it has one (it carries the right sub-page, episode
/// and focus parameter for that workspace).
fn nav_for_target(
    c: &Connection,
    target_type: &str,
    target_id: &str,
    scene_id: Option<&str>,
) -> AppResult<Value> {
    use crate::modules::screenplay::{draft_nav, scene_nav};
    match (target_type, scene_id) {
        ("screenplay_element" | "screenplay_scene" | "screenplay_draft", Some(s)) => {
            return scene_nav(c, s);
        }
        ("screenplay_draft", None) => return draft_nav(c, target_id),
        ("screenplay_element" | "screenplay_scene", None) => {
            return Ok(json!({ "workspace": "screenplay" }));
        }
        ("breakdown_element", _) => {
            let scene: Option<(Option<String>, Option<String>)> = c
                .query_row(
                    "SELECT scene_id, scene_lineage_id FROM breakdown_element WHERE id=?1",
                    [target_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let mut v = json!({ "workspace": "breakdown", "elementId": target_id });
            if let Some((s, l)) = scene {
                if let Some(s) = s {
                    v["sceneId"] = json!(s);
                }
                if let Some(l) = l {
                    v["sceneLineageId"] = json!(l);
                }
            }
            return Ok(v);
        }
        _ => {}
    }
    let indexed: Option<String> = c
        .query_row(
            "SELECT nav_json FROM search_doc WHERE source_table=?1 AND entity_id=?2",
            params![target_type, target_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(v) = indexed
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .filter(|v| v.get("workspace").is_some())
    {
        return Ok(v);
    }
    Ok(match target_type {
        "story_scene_card" => json!({ "workspace": "story", "cardId": target_id }),
        "story_beat" => json!({ "workspace": "story", "beatId": target_id }),
        "story_act" => json!({ "workspace": "story", "actId": target_id }),
        "story_sequence" => json!({ "workspace": "story", "sequenceId": target_id }),
        "story_character" => {
            json!({ "workspace": "story", "sub": "characters", "characterId": target_id })
        }
        "location" => {
            json!({ "workspace": "production", "sub": "locations", "locationId": target_id })
        }
        "vault_item" => json!({ "workspace": "vault", "itemId": target_id }),
        _ => json!({ "workspace": "production", "targetType": target_type, "targetId": target_id }),
    })
}

fn index_comment(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<Row> = c
        .query_row(
            &format!("SELECT {COLS} FROM comment WHERE id=?1 AND deleted_at IS NULL"),
            [id],
            map_row,
        )
        .optional()?;
    let Some(row) = row else { return Ok(None) };
    if row.parent_id.is_some() {
        return Ok(None);
    }
    let mut stmt = c.prepare(
        "SELECT body FROM comment WHERE parent_id=?1 AND deleted_at IS NULL ORDER BY created_at",
    )?;
    let replies: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut body = row.body.clone();
    if let Some(q) = &row.quoted_text {
        body.push_str("\n“");
        body.push_str(q);
        body.push('”');
    }
    for r in replies {
        body.push('\n');
        body.push_str(&r);
    }
    Ok(Some(SearchDoc {
        entity_type: "comment".into(),
        title: format!("Comment by {}: {}", row.author_name, excerpt(&row.body, 60)),
        body,
        context: "Comments".into(),
        nav: {
            let mut v =
                nav_for_target(c, &row.target_type, &row.target_id, row.scene_id.as_deref())?;
            v["commentId"] = json!(row.id);
            v
        },
        owner_user_id: None,
    }))
}

// ------------------------------------------------------------- private notes

fn note_row(c: &Connection, owner: &str, id: &str) -> AppResult<PrivateNoteDto> {
    c.query_row(
        "SELECT id, target_type, target_id, body, created_at, updated_at, rev FROM private_note
         WHERE id=?1 AND owner_user_id=?2 AND deleted_at IS NULL",
        params![id, owner],
        |r| {
            Ok(PrivateNoteDto {
                id: r.get(0)?,
                target_type: r.get(1)?,
                target_id: r.get(2)?,
                body: r.get(3)?,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
                rev: r.get(6)?,
            })
        },
    )
    .optional()?
    // Another user's note is indistinguishable from a missing one (Security §7.4).
    .ok_or_else(|| AppError::not_found("private note"))
}

fn clean_note(body: String) -> AppResult<String> {
    let body = body_text(body, "Private note", MAX_BODY_BYTES)?;
    if body.trim().is_empty() {
        return Err(AppError::required("Note text"));
    }
    Ok(body.trim_end().to_string())
}

fn note_create(
    core: &AppCore,
    actor: &Actor,
    args: CreatePrivateNoteArgs,
) -> AppResult<PrivateNoteDto> {
    let body = clean_note(args.body)?;
    if args.target_type.is_some() != args.target_id.is_some() {
        return Err(AppError::invalid_input(
            "A note target needs both a type and an id.",
        ));
    }
    let s = core.project()?;
    // Private notes never appear in project activity (existence is private).
    let id = s.store.mutate(actor, MutationMeta::new("private_note.create", "Added a private note", Capability::View).quiet(), |tx| {
        let c = tx.conn();
        if let (Some(tt), Some(tid)) = (&args.target_type, &args.target_id) {
            require_id(tid, "item")?;
            check_target(c, tt, tid)?;
        }
        let id = new_id();
        let now = now_ms();
        c.execute(
            "INSERT INTO private_note(id, owner_user_id, target_type, target_id, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![id, actor.user_id, args.target_type, args.target_id, body, now],
        )?;
        Ok(id)
    })?;
    s.store.read(|c| note_row(c, &actor.user_id, &id))
}

fn note_list(
    core: &AppCore,
    actor: &Actor,
    args: ListPrivateNotesArgs,
) -> AppResult<Vec<PrivateNoteDto>> {
    actor.require(Capability::View, "view your private notes")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id FROM private_note WHERE owner_user_id=?1 AND deleted_at IS NULL
               AND (?2 IS NULL OR target_type=?2) AND (?3 IS NULL OR target_id=?3)
             ORDER BY created_at, id",
        )?;
        let ids: Vec<String> = stmt
            .query_map(
                params![actor.user_id, args.target_type, args.target_id],
                |r| r.get(0),
            )?
            .collect::<Result<_, _>>()?;
        ids.iter()
            .map(|id| note_row(c, &actor.user_id, id))
            .collect()
    })
}

fn note_update(
    core: &AppCore,
    actor: &Actor,
    args: UpdatePrivateNoteArgs,
) -> AppResult<PrivateNoteDto> {
    let body = clean_note(args.body)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "private_note.update",
            "Edited a private note",
            Capability::View,
        )
        .quiet(),
        |tx| {
            note_row(tx.conn(), &actor.user_id, &args.id)?;
            update_fields(
                tx.conn(),
                "private_note",
                &args.id,
                &[("body", text(body.clone()))],
                &["body"],
                None,
                "private note",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| note_row(c, &actor.user_id, &args.id))
}

fn note_delete(core: &AppCore, actor: &Actor, args: PrivateNoteIdArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "private_note.delete",
            "Deleted a private note",
            Capability::View,
        )
        .quiet(),
        |tx| {
            let note = note_row(tx.conn(), &actor.user_id, &args.id)?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "private_note",
                    table: "private_note",
                    id: &note.id,
                    // No content in the title: trash titles appear in activity summaries.
                    title: Some("Private note".to_string()),
                    parent_type: note.target_type.as_deref(),
                    parent_id: note.target_id.clone(),
                    position: None,
                },
            )
        },
    )
}

fn owner_of(c: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(c.query_row(
        "SELECT owner_user_id FROM private_note WHERE id=?1",
        [id],
        |r| r.get(0),
    )
    .optional()?)
}

fn restore_private_note(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    if owner_of(tx.conn(), &row.object_id)?.as_deref() != Some(tx.actor().user_id.as_str()) {
        return Err(AppError::not_found("deleted item"));
    }
    tx.conn().execute(
        "UPDATE private_note SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
        params![now_ms(), row.object_id],
    )?;
    Ok(())
}

fn purge_private_note(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    if owner_of(tx.conn(), &row.object_id)?.as_deref() != Some(tx.actor().user_id.as_str()) {
        return Err(AppError::not_found("deleted item"));
    }
    tx.conn()
        .execute("DELETE FROM private_note WHERE id=?1", [&row.object_id])?;
    scrub_undo_history(tx, &row.object_id)?;
    Ok(())
}

/// Permanent deletion must really remove the note text: the undo log keeps full row
/// images, and it travels in backup/project packages (Security review PN-02).
fn scrub_undo_history(tx: &Tx<'_>, note_id: &str) -> AppResult<()> {
    if openframe_domain::ids::is_valid_id(note_id) {
        tx.conn().execute(
            "DELETE FROM sys_undo WHERE instr(changes_json, ?1) > 0",
            [note_id],
        )?;
    }
    Ok(())
}

fn index_private_note(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, String)> = c
        .query_row(
            "SELECT owner_user_id, target_type, target_id, body FROM private_note WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((owner, tt, tid, body)) = row else {
        return Ok(None);
    };
    let nav = match (tt.as_deref(), tid.as_deref()) {
        (Some(tt), Some(tid)) => {
            let scene = scene_for_target(c, tt, tid)?;
            let mut v = nav_for_target(c, tt, tid, scene.as_deref())?;
            v["privateNoteId"] = json!(id);
            v
        }
        _ => json!({ "workspace": "notes", "privateNoteId": id }),
    };
    Ok(Some(SearchDoc {
        entity_type: "private_note".into(),
        title: format!("Private note: {}", excerpt(&body, 60)),
        body,
        context: "Private note — visible only to you".into(),
        nav,
        // Only the owner's searches can return this document (Security §10.2).
        owner_user_id: Some(owner),
    }))
}

/// Remove every private note attached to the given targets (all owners) — used
/// only when the targets themselves are permanently purged.
pub(crate) fn purge_private_notes_for_targets(
    tx: &Tx<'_>,
    target_type: &str,
    ids: &[String],
) -> AppResult<()> {
    for id in ids {
        let mut stmt = tx
            .conn()
            .prepare("SELECT id FROM private_note WHERE target_type=?1 AND target_id=?2")?;
        let nids: Vec<String> = stmt
            .query_map(params![target_type, id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for nid in nids {
            tx.conn().execute(
                "DELETE FROM deleted_item WHERE table_name='private_note' AND object_id=?1",
                [&nid],
            )?;
            tx.conn()
                .execute("DELETE FROM private_note WHERE id=?1", [&nid])?;
            scrub_undo_history(tx, &nid)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_offsets_follow_the_editor() {
        let s = "Ça va 🎬 RED FOLDER";
        let pos = find_all_utf16(s, "RED");
        assert_eq!(pos, vec![9]); // 🎬 is two UTF-16 units
        assert_eq!(utf16_slice(s, 9, 12).unwrap(), "RED");
        let (a, q) = build_anchor("e", s, 9, 19).unwrap();
        assert_eq!(q, "RED FOLDER");
        assert!(a.before.ends_with("🎬 "));
        assert!(build_anchor("e", s, 5, 5).is_err());
    }

    #[test]
    fn identifiers_are_strict() {
        assert!(is_identifier("story_scene_card"));
        assert!(!is_identifier("x; DROP TABLE"));
        assert!(!is_identifier("Screenplay"));
    }
}
