//! Tasks (FSD §106, §158; UX §3.35, mock 151).
//!
//! Title required; optional due date, owner and related object. Open ⇄ Done.
//! Marking a task done never changes the related object. Deleting is recoverable.
//! No dependencies, Gantt, time tracking or sprints.

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, next_position, opt_int, opt_text, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::core::AppCore;
use crate::modules::project::describe_row;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{optional_text, required_text};

pub fn register(r: &mut Registry) {
    r.query("tasks.list", list);
    r.query("tasks.relatable_types", relatable_types);
    r.command("tasks.create", create);
    r.command("tasks.update", update);
    r.command("tasks.set_done", set_done);
    r.command("tasks.delete", delete);
    r.indexer("task", index_task);
    r.trash_handler(TrashHandler {
        object_type: "task",
        table: "task",
        label: "Task",
        restore: None,
        purge: purge_task,
    });
}

/// Objects a task may relate to (FSD §106: Scene, Location, Character, Breakdown
/// Item, Shooting Day, Call Sheet — plus the Story Board card and people).
/// Keys are table names (= search entity types).
pub const RELATABLE: &[(&str, &str)] = &[
    ("screenplay_scene", "Scene"),
    ("story_scene_card", "Scene Card"),
    ("story_character", "Character"),
    ("location", "Location"),
    ("breakdown_element", "Breakdown Item"),
    ("catalog_item", "Catalog Item"),
    ("cast_member", "Cast"),
    ("crew_member", "Crew"),
    ("shooting_day", "Shooting Day"),
    ("call_sheet", "Call Sheet"),
];

fn relatable_label(t: &str) -> Option<&'static str> {
    RELATABLE.iter().find(|(k, _)| *k == t).map(|(_, l)| *l)
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RelatableType {
    pub target_type: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelatedRef {
    pub target_type: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RelatedObject {
    pub target_type: String,
    pub target_id: String,
    /// e.g. "Location".
    pub type_label: String,
    pub title: Option<String>,
    /// False when the related object was deleted (the task keeps the reference).
    pub available: bool,
    #[ts(type = "unknown")]
    pub nav: Option<Value>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    /// "Open" | "Done"
    pub status: String,
    pub owner_user_id: Option<String>,
    pub owner_name: Option<String>,
    #[ts(type = "number | null")]
    pub due_at: Option<i64>,
    pub notes: Option<String>,
    pub related: Option<RelatedObject>,
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
pub struct TaskListArgs {
    /// "Open" | "Done"; absent = all.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub owner_user_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTaskArgs {
    pub title: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub due_at: Option<i64>,
    #[serde(default)]
    pub owner_user_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub related: Option<RelatedRef>,
}

/// Patch: absent fields are unchanged. Use the `clear*` flags (or "" for text) to remove a value.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateTaskArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub due_at: Option<i64>,
    #[serde(default)]
    pub clear_due: bool,
    /// "" removes the owner.
    #[serde(default)]
    pub owner_user_id: Option<String>,
    /// "" removes the notes.
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub related: Option<RelatedRef>,
    #[serde(default)]
    pub clear_related: bool,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTaskDoneArgs {
    pub id: String,
    pub done: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRefArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelatableTypesArgs {}

// ------------------------------------------------------------------ helpers

/// A related object's title/nav from its search projection, falling back to its row.
fn related_object(c: &Connection, target_type: &str, target_id: &str) -> AppResult<RelatedObject> {
    let doc: Option<(String, String)> = c
        .query_row(
            "SELECT title, nav_json FROM search_doc WHERE source_table=?1 AND entity_id=?2 AND owner_user_id IS NULL",
            params![target_type, target_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let type_label = relatable_label(target_type).unwrap_or("Item").to_string();
    Ok(match doc {
        Some((title, nav)) => RelatedObject {
            target_type: target_type.into(),
            target_id: target_id.into(),
            type_label,
            title: Some(title).filter(|t| !t.trim().is_empty()),
            available: true,
            nav: serde_json::from_str(&nav).ok(),
        },
        None => {
            let (title, available) =
                describe_row(c, target_type, target_id)?.unwrap_or((None, false));
            RelatedObject {
                target_type: target_type.into(),
                target_id: target_id.into(),
                type_label,
                title,
                available,
                nav: None,
            }
        }
    })
}

fn load(c: &Connection, id: &str) -> AppResult<TaskDto> {
    type Row = (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
        i64,
    );
    let row: Option<Row> = c
        .query_row(
            "SELECT t.id, t.title, t.status, t.owner_user_id, m.display_name, t.due_at, t.notes, t.target_type, t.target_id,
                    t.created_at, t.updated_at, t.rev
             FROM task t LEFT JOIN project_member m ON m.user_id = t.owner_user_id
             WHERE t.id=?1 AND t.deleted_at IS NULL",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                ))
            },
        )
        .optional()?;
    let (
        id,
        title,
        status,
        owner_user_id,
        owner_name,
        due_at,
        notes,
        tt,
        tid,
        created_at,
        updated_at,
        rev,
    ) = row.ok_or_else(|| AppError::not_found("task"))?;
    let related = match (tt, tid) {
        (Some(t), Some(i)) => Some(related_object(c, &t, &i)?),
        _ => None,
    };
    Ok(TaskDto {
        id,
        title,
        status,
        owner_user_id,
        owner_name,
        due_at,
        notes,
        related,
        created_at,
        updated_at,
        rev,
    })
}

fn check_owner(c: &Connection, owner: &str) -> AppResult<()> {
    let ok: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM project_member WHERE user_id=?1 AND deleted_at IS NULL)",
        [owner],
        |r| r.get(0),
    )?;
    if ok {
        Ok(())
    } else {
        Err(AppError::validation(
            "owner",
            "Choose someone who is part of this project.",
        ))
    }
}

fn check_related(c: &Connection, r: &RelatedRef) -> AppResult<()> {
    if relatable_label(&r.target_type).is_none() {
        return Err(AppError::invalid_input(
            "A task can't be related to that kind of item.",
        ));
    }
    match describe_row(c, &r.target_type, &r.target_id)? {
        Some((_, true)) => Ok(()),
        _ => Err(AppError::not_found("related item")),
    }
}

/// Due dates are calendar days stored as epoch ms; keep them in a sane range.
fn check_due(due: Option<i64>) -> AppResult<()> {
    const MIN: i64 = 0; // 1970
    const MAX: i64 = 32_503_680_000_000; // year 3000
    match due {
        Some(d) if !(MIN..=MAX).contains(&d) => {
            Err(AppError::validation("due", "That due date isn't valid."))
        }
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------- operations

fn relatable_types(
    _core: &AppCore,
    _actor: &Actor,
    _: RelatableTypesArgs,
) -> AppResult<Vec<RelatableType>> {
    Ok(RELATABLE
        .iter()
        .map(|(t, l)| RelatableType {
            target_type: (*t).into(),
            label: (*l).into(),
        })
        .collect())
}

fn list(core: &AppCore, actor: &Actor, args: TaskListArgs) -> AppResult<Vec<TaskDto>> {
    actor.require(Capability::View, "view tasks")?;
    if let Some(st) = &args.status
        && st != "Open"
        && st != "Done"
    {
        return Err(AppError::invalid_input("Unknown task status."));
    }
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id FROM task WHERE deleted_at IS NULL AND (?1 IS NULL OR status=?1) AND (?2 IS NULL OR owner_user_id=?2)
             ORDER BY CASE status WHEN 'Open' THEN 0 ELSE 1 END, position, created_at, id",
        )?;
        let ids: Vec<String> =
            stmt.query_map(params![args.status, args.owner_user_id], |r| r.get(0))?.collect::<Result<_, _>>()?;
        ids.iter().map(|id| load(c, id)).collect()
    })
}

fn create(core: &AppCore, actor: &Actor, args: CreateTaskArgs) -> AppResult<TaskDto> {
    let title = required_text(&args.title, "Task", 300)?;
    let notes = optional_text(args.notes, "Notes", 5_000)?;
    let owner = args.owner_user_id.filter(|o| !o.trim().is_empty());
    check_due(args.due_at)?;
    let s = core.project()?;
    let id = new_id();
    s.store.mutate(actor, MutationMeta::new("tasks.create", format!("Added task “{title}”"), Capability::Edit).target("task", &id), |tx| {
        let c = tx.conn();
        if let Some(o) = &owner {
            check_owner(c, o)?;
        }
        if let Some(r) = &args.related {
            check_related(c, r)?;
        }
        let pos = next_position(c, "task", "1=1", &[])?;
        let now = now_ms();
        c.execute(
            "INSERT INTO task(id, title, status, owner_user_id, due_at, notes, target_type, target_id, position, created_at, updated_at)
             VALUES (?1, ?2, 'Open', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                title,
                owner,
                args.due_at,
                notes,
                args.related.as_ref().map(|r| r.target_type.clone()),
                args.related.as_ref().map(|r| r.target_id.clone()),
                pos,
                now
            ],
        )?;
        Ok(())
    })?;
    s.store.read(|c| load(c, &id))
}

fn update(core: &AppCore, actor: &Actor, args: UpdateTaskArgs) -> AppResult<TaskDto> {
    let title = args
        .title
        .as_deref()
        .map(|t| required_text(t, "Task", 300))
        .transpose()?;
    let notes = args
        .notes
        .map(|n| optional_text(Some(n), "Notes", 5_000))
        .transpose()?;
    check_due(args.due_at)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("tasks.update", "Edited task", Capability::Edit)
            .target("task", &args.id)
            .coalesce(format!("tasks.update:{}", args.id)),
        |tx| {
            let c = tx.conn();
            let live: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM task WHERE id=?1 AND deleted_at IS NULL)",
                [&args.id],
                |r| r.get(0),
            )?;
            if !live {
                return Err(AppError::not_found("task"));
            }
            let mut fields: Vec<(&str, SqlValue)> = Vec::new();
            if let Some(t) = &title {
                fields.push(("title", text(t.clone())));
            }
            if args.clear_due {
                fields.push(("due_at", opt_int(None)));
            } else if let Some(d) = args.due_at {
                fields.push(("due_at", int(d)));
            }
            if let Some(o) = &args.owner_user_id {
                if o.trim().is_empty() {
                    fields.push(("owner_user_id", opt_text(None::<String>)));
                } else {
                    check_owner(c, o)?;
                    fields.push(("owner_user_id", text(o.clone())));
                }
            }
            if let Some(n) = &notes {
                fields.push(("notes", opt_text(n.clone())));
            }
            if args.clear_related {
                fields.push(("target_type", opt_text(None::<String>)));
                fields.push(("target_id", opt_text(None::<String>)));
            } else if let Some(r) = &args.related {
                check_related(c, r)?;
                fields.push(("target_type", text(r.target_type.clone())));
                fields.push(("target_id", text(r.target_id.clone())));
            }
            update_fields(
                c,
                "task",
                &args.id,
                &fields,
                &[
                    "title",
                    "due_at",
                    "owner_user_id",
                    "notes",
                    "target_type",
                    "target_id",
                ],
                args.expected_rev,
                "task",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load(c, &args.id))
}

/// Mark Done / Reopen (FSD §158). Never touches the related object.
fn set_done(core: &AppCore, actor: &Actor, args: SetTaskDoneArgs) -> AppResult<TaskDto> {
    let s = core.project()?;
    let title: String = s.store.read(|c| {
        c.query_row(
            "SELECT title FROM task WHERE id=?1 AND deleted_at IS NULL",
            [&args.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("task"))
    })?;
    let summary = if args.done {
        format!("Completed task “{title}”")
    } else {
        format!("Reopened task “{title}”")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("tasks.set_done", summary, Capability::Edit).target("task", &args.id),
        |tx| {
            let status = if args.done { "Done" } else { "Open" };
            update_fields(
                tx.conn(),
                "task",
                &args.id,
                &[("status", text(status))],
                &["status"],
                None,
                "task",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load(c, &args.id))
}

fn delete(core: &AppCore, actor: &Actor, args: TaskRefArgs) -> AppResult<()> {
    let s = core.project()?;
    let (title, pos): (String, i64) = s.store.read(|c| {
        c.query_row(
            "SELECT title, position FROM task WHERE id=?1 AND deleted_at IS NULL",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("task"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "tasks.delete",
            format!("Deleted task “{title}”"),
            Capability::SoftDelete,
        )
        .target("task", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "task",
                    table: "task",
                    id: &args.id,
                    title: Some(title.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: Some(pos),
                },
            )
        },
    )
}

fn index_task(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT title, notes, deleted_at FROM task WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, notes, None)) => Some(SearchDoc {
            entity_type: "task".into(),
            title,
            body: notes.unwrap_or_default(),
            context: "Notes & Tasks".into(),
            nav: json!({ "workspace": "notes", "taskId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn purge_task(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM task WHERE id=?1", [&row.object_id])?;
    Ok(())
}
