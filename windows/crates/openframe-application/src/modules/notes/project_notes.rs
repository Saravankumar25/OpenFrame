//! Project notes (FSD §106, §159; UX §3.35, mock 151).

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{body_text, optional_text};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.query("notes.list", list)
        .meta(M::search("Project Notes (optionally filtered by text)."));
    r.query("notes.get", get).meta(M::read("One Project Note."));
    r.command("notes.create", create)
        .meta(M::edit("Create a Project Note."));
    r.command("notes.update", update)
        .meta(M::edit("Edit a Project Note's title or text."));
    r.command("notes.set_pinned", set_pinned)
        .meta(M::edit("Pin or unpin a Project Note."));
    r.command("notes.delete", delete)
        .meta(M::soft_delete("Move a Project Note to Recently Deleted."));
    r.indexer("project_note", index_note);
    r.trash_handler(TrashHandler {
        object_type: "project_note",
        table: "project_note",
        label: "Note",
        restore: None,
        purge: purge_note,
    });
}

const MAX_BODY_BYTES: usize = 200_000;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NoteDto {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub pinned: bool,
    pub created_by_name: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteListArgs {
    /// Optional text filter over title and body (the list is small; plain matching).
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateNoteArgs {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateNoteArgs {
    pub id: String,
    /// Absent/null = unchanged; "" = no title.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetNotePinnedArgs {
    pub id: String,
    pub pinned: bool,
}

/// Display title for a note: its title, else the first line of the body.
pub fn note_label(title: Option<&str>, body: &str) -> String {
    if let Some(t) = title.map(str::trim).filter(|t| !t.is_empty()) {
        return t.to_string();
    }
    let first = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first.is_empty() {
        "Untitled note".to_string()
    } else {
        let mut s: String = first.chars().take(60).collect();
        if first.chars().count() > 60 {
            s.push('…');
        }
        s
    }
}

fn load(c: &Connection, id: &str) -> AppResult<NoteDto> {
    c.query_row(
        "SELECT n.id, n.title, n.body, n.pinned, m.display_name, n.created_at, n.updated_at, n.rev
         FROM project_note n LEFT JOIN project_member m ON m.user_id = n.created_by
         WHERE n.id=?1 AND n.deleted_at IS NULL",
        [id],
        row_to_dto,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("note"))
}

fn row_to_dto(r: &rusqlite::Row<'_>) -> rusqlite::Result<NoteDto> {
    Ok(NoteDto {
        id: r.get(0)?,
        title: r.get(1)?,
        body: r.get(2)?,
        pinned: r.get::<_, i64>(3)? != 0,
        created_by_name: r.get(4)?,
        created_at: r.get(5)?,
        updated_at: r.get(6)?,
        rev: r.get(7)?,
    })
}

fn list(core: &AppCore, actor: &Actor, args: NoteListArgs) -> AppResult<Vec<NoteDto>> {
    actor.require(Capability::View, "view project notes")?;
    let s = core.project()?;
    let needle = args
        .text
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty());
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT n.id, n.title, n.body, n.pinned, m.display_name, n.created_at, n.updated_at, n.rev
             FROM project_note n LEFT JOIN project_member m ON m.user_id = n.created_by
             WHERE n.deleted_at IS NULL ORDER BY n.pinned DESC, n.updated_at DESC, n.id",
        )?;
        let rows = stmt.query_map([], row_to_dto)?.collect::<Result<Vec<_>, _>>()?;
        Ok(match needle {
            Some(n) => rows
                .into_iter()
                .filter(|r| {
                    r.title.as_deref().unwrap_or("").to_lowercase().contains(&n) || r.body.to_lowercase().contains(&n)
                })
                .collect(),
            None => rows,
        })
    })
}

fn get(core: &AppCore, actor: &Actor, args: NoteIdArgs) -> AppResult<NoteDto> {
    actor.require(Capability::View, "view project notes")?;
    let s = core.project()?;
    s.store.read(|c| load(c, &args.id))
}

fn create(core: &AppCore, actor: &Actor, args: CreateNoteArgs) -> AppResult<NoteDto> {
    let title = optional_text(args.title, "Title", 200)?;
    let body = body_text(args.body.unwrap_or_default(), "Note", MAX_BODY_BYTES)?;
    let s = core.project()?;
    let id = new_id();
    let label = note_label(title.as_deref(), &body);
    s.store.mutate(
        actor,
        MutationMeta::new("notes.create", format!("Added note “{label}”"), Capability::Edit).target("project_note", &id),
        |tx| {
            let now = now_ms();
            tx.conn().execute(
                "INSERT INTO project_note(id, title, body, pinned, created_by, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, title, body, args.pinned as i64, tx.actor().user_id, now],
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load(c, &id))
}

fn update(core: &AppCore, actor: &Actor, args: UpdateNoteArgs) -> AppResult<NoteDto> {
    let title = args
        .title
        .map(|t| optional_text(Some(t), "Title", 200))
        .transpose()?;
    let body = args
        .body
        .map(|b| body_text(b, "Note", MAX_BODY_BYTES))
        .transpose()?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("notes.update", "Edited note", Capability::Edit)
            .target("project_note", &args.id)
            .coalesce(format!("notes.update:{}", args.id)),
        |tx| {
            let live: bool = tx.conn().query_row(
                "SELECT EXISTS(SELECT 1 FROM project_note WHERE id=?1 AND deleted_at IS NULL)",
                [&args.id],
                |r| r.get(0),
            )?;
            if !live {
                return Err(AppError::not_found("note"));
            }
            let mut fields = Vec::new();
            if let Some(t) = &title {
                fields.push(("title", opt_text(t.clone())));
            }
            if let Some(b) = &body {
                fields.push(("body", text(b.clone())));
            }
            update_fields(
                tx.conn(),
                "project_note",
                &args.id,
                &fields,
                &["title", "body"],
                args.expected_rev,
                "note",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load(c, &args.id))
}

fn set_pinned(core: &AppCore, actor: &Actor, args: SetNotePinnedArgs) -> AppResult<NoteDto> {
    let s = core.project()?;
    let summary = if args.pinned {
        "Pinned note"
    } else {
        "Unpinned note"
    };
    s.store.mutate(
        actor,
        MutationMeta::new("notes.set_pinned", summary, Capability::Edit)
            .target("project_note", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "project_note",
                &args.id,
                &[("pinned", int(args.pinned as i64))],
                &["pinned"],
                None,
                "note",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load(c, &args.id))
}

fn delete(core: &AppCore, actor: &Actor, args: NoteIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let note = s.store.read(|c| load(c, &args.id))?;
    let label = note_label(note.title.as_deref(), &note.body);
    s.store.mutate(
        actor,
        MutationMeta::new(
            "notes.delete",
            format!("Deleted note “{label}”"),
            Capability::SoftDelete,
        )
        .target("project_note", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "project_note",
                    table: "project_note",
                    id: &args.id,
                    title: Some(label.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: None,
                },
            )
        },
    )
}

fn index_note(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(Option<String>, String, Option<i64>)> = c
        .query_row(
            "SELECT title, body, deleted_at FROM project_note WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, body, None)) => Some(SearchDoc {
            entity_type: "project_note".into(),
            title: note_label(title.as_deref(), &body),
            body,
            context: "Notes & Tasks".into(),
            nav: json!({ "workspace": "notes", "noteId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn purge_note(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM project_note WHERE id=?1", [&row.object_id])?;
    Ok(())
}
