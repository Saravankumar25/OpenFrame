//! Templates (FSD §59, §113; Domain §20; UX §3.37, mock 153).
//!
//! Three scopes:
//! * `builtin` — read-only starting points shipped with OpenFrame;
//! * `global`  — the user's own templates, stored in the application database
//!   (`sys_global_template`) and usable from any project;
//! * `project` — templates inside the open project (`template` table: undoable,
//!   recoverable delete).
//!
//! "Use" copies a template: the copy gets a new identity, so later edits to the
//! template never change anything already created from it (FSD §113). Workspaces
//! that create documents (call sheets, shot lists, moodboards, reports) read
//! templates with `templates.list { templateType }` / `templates.get`.

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::required_text;

pub fn register(r: &mut Registry) {
    r.query("templates.list", list);
    r.query("templates.get", get);
    r.command("templates.create", create);
    r.command("templates.update", update);
    r.command("templates.delete", delete);
    r.command("templates.copy", copy);
    r.indexer("template", index_template);
    r.trash_handler(TrashHandler {
        object_type: "template",
        table: "template",
        label: "Template",
        restore: None,
        purge,
    });
}

/// Supported template types (FSD §59) and their labels.
pub const TEMPLATE_TYPES: &[(&str, &str)] = &[
    ("call_sheet", "Call sheet"),
    ("shot_list", "Shot list"),
    ("moodboard", "Moodboard"),
    ("report", "Report"),
    ("story_board", "Story Board starter"),
    ("breakdown", "Breakdown document"),
    ("budget", "Simple budget"),
    ("project_starter", "Project starter"),
];

const MAX_CONTENT_BYTES: usize = 256 * 1024;

fn type_label(t: &str) -> Option<&'static str> {
    TEMPLATE_TYPES
        .iter()
        .find(|(k, _)| *k == t)
        .map(|(_, l)| *l)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum TemplateScope {
    Builtin,
    Global,
    Project,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub scope: TemplateScope,
    pub template_type: String,
    pub type_label: String,
    pub name: String,
    /// Short description shown on the template card (content `description`).
    pub description: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub content: Value,
    #[ts(type = "number | null")]
    pub updated_at: Option<i64>,
    #[ts(type = "number | null")]
    pub rev: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TemplatesListing {
    pub builtin: Vec<TemplateDto>,
    pub global: Vec<TemplateDto>,
    pub project: Vec<TemplateDto>,
    /// False when no project is open (project templates unavailable).
    pub project_open: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateListArgs {
    #[serde(default)]
    pub template_type: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateRefArgs {
    pub scope: TemplateScope,
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateTemplateArgs {
    pub scope: TemplateScope,
    pub template_type: String,
    pub name: String,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub content: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateTemplateArgs {
    pub scope: TemplateScope,
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub content: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyTemplateArgs {
    pub scope: TemplateScope,
    pub id: String,
    /// Destination: `project` ("Use in This Project") or `global` ("Save to My Templates").
    pub to_scope: TemplateScope,
    #[serde(default)]
    pub name: Option<String>,
}

// ------------------------------------------------------------------ built-ins

fn builtins() -> Vec<TemplateDto> {
    let b = |id: &str, t: &str, name: &str, content: Value| TemplateDto {
        id: format!("builtin:{id}"),
        scope: TemplateScope::Builtin,
        template_type: t.to_string(),
        type_label: type_label(t).unwrap_or(t).to_string(),
        name: name.to_string(),
        description: content
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        content,
        updated_at: None,
        rev: None,
    };
    vec![
        b(
            "project_starter",
            "project_starter",
            "Project starter",
            json!({ "description": "Feature · 3 acts", "acts": ["Act One", "Act Two", "Act Three"],
                    "notes": ["Logline", "Themes", "Research to do"] }),
        ),
        b(
            "story_board",
            "story_board",
            "Story Board starter",
            json!({ "description": "Three-act beats", "acts": ["Act One", "Act Two", "Act Three"],
                    "beats": ["Opening image", "Inciting incident", "Midpoint", "Crisis", "Climax", "Resolution"] }),
        ),
        b(
            "call_sheet",
            "call_sheet",
            "Call sheet layout",
            json!({ "description": "Standard one-page", "sections": ["General call", "Scenes", "Cast", "Crew",
                    "Locations & parking", "Weather", "Nearest hospital", "Notes"] }),
        ),
        b(
            "shot_list",
            "shot_list",
            "Shot list",
            json!({ "description": "Standard columns", "columns": ["Shot", "Size", "Angle", "Movement", "Lens",
                    "Description", "Notes"] }),
        ),
        b(
            "moodboard",
            "moodboard",
            "Location scouting board",
            json!({ "description": "Moodboard layout", "sections": ["Exteriors", "Interiors", "Light", "Access & parking"] }),
        ),
        b(
            "report",
            "report",
            "Daily production report",
            json!({ "description": "One page per day", "sections": ["Scenes shot", "Pages", "Setups", "Call & wrap times",
                    "Notes"] }),
        ),
        b(
            "budget",
            "budget",
            "Simple budget",
            json!({ "description": "9 categories", "categories": ["Story & rights", "Cast", "Crew", "Locations",
                    "Equipment", "Art & costume", "Travel & food", "Post-production", "Contingency"] }),
        ),
    ]
}

// ------------------------------------------------------------------ helpers

fn check_type(t: &str) -> AppResult<()> {
    if type_label(t).is_some() {
        Ok(())
    } else {
        Err(AppError::validation("type", "Choose a template type."))
    }
}

fn check_content(v: Option<Value>) -> AppResult<Value> {
    let v = v.unwrap_or_else(|| json!({}));
    if !v.is_object() {
        return Err(AppError::invalid_input(
            "Template content must be an object.",
        ));
    }
    if v.to_string().len() > MAX_CONTENT_BYTES {
        return Err(AppError::invalid_input("This template is too large."));
    }
    Ok(v)
}

fn require_local(actor: &Actor) -> AppResult<()> {
    // Global templates belong to the person at this computer, not to changes that arrive in a package or from AI.
    if matches!(actor.origin, ActorOrigin::Local) {
        Ok(())
    } else {
        Err(AppError::new(
            "permission.denied",
            "Only the person using this computer can change its templates.",
        ))
    }
}

fn dto(
    scope: TemplateScope,
    id: String,
    t: String,
    name: String,
    content: String,
    updated_at: i64,
    rev: Option<i64>,
) -> TemplateDto {
    let content: Value = serde_json::from_str(&content).unwrap_or_else(|_| json!({}));
    TemplateDto {
        id,
        scope,
        type_label: type_label(&t).unwrap_or("Template").to_string(),
        template_type: t,
        name,
        description: content
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        content,
        updated_at: Some(updated_at),
        rev,
    }
}

fn project_rows(c: &Connection, id: Option<&str>, t: Option<&str>) -> AppResult<Vec<TemplateDto>> {
    let mut stmt = c.prepare(
        "SELECT id, template_type, name, content_json, updated_at, rev FROM template
         WHERE deleted_at IS NULL AND (?1 IS NULL OR id=?1) AND (?2 IS NULL OR template_type=?2)
         ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = stmt
        .query_map(params![id, t], |r| {
            Ok(dto(
                TemplateScope::Project,
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                Some(r.get(5)?),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn global_rows(c: &Connection, id: Option<&str>, t: Option<&str>) -> AppResult<Vec<TemplateDto>> {
    let mut stmt = c.prepare(
        "SELECT id, template_type, name, content_json, updated_at FROM sys_global_template
         WHERE (?1 IS NULL OR id=?1) AND (?2 IS NULL OR template_type=?2) ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = stmt
        .query_map(params![id, t], |r| {
            Ok(dto(
                TemplateScope::Global,
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                None,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn find(core: &AppCore, scope: TemplateScope, id: &str) -> AppResult<TemplateDto> {
    let found = match scope {
        TemplateScope::Builtin => builtins().into_iter().find(|t| t.id == id),
        TemplateScope::Global => core
            .with_app_db(|c| global_rows(c, Some(id), None))?
            .into_iter()
            .next(),
        TemplateScope::Project => core
            .project()?
            .store
            .read(|c| project_rows(c, Some(id), None))?
            .into_iter()
            .next(),
    };
    found.ok_or_else(|| AppError::not_found("template"))
}

/// Insert a template in the given (writable) scope and return it.
fn insert(
    core: &AppCore,
    actor: &Actor,
    scope: TemplateScope,
    t: &str,
    name: &str,
    content: &Value,
) -> AppResult<TemplateDto> {
    let id = new_id();
    let json = content.to_string();
    match scope {
        TemplateScope::Builtin => {
            return Err(AppError::invalid_input(
                "Built-in templates can't be changed.",
            ));
        }
        TemplateScope::Global => {
            require_local(actor)?;
            core.with_app_db(|c| {
                let now = now_ms();
                c.execute(
                    "INSERT INTO sys_global_template(id, template_type, name, content_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![id, t, name, json, now],
                )?;
                Ok(())
            })?;
        }
        TemplateScope::Project => {
            let s = core.project()?;
            s.store.mutate(
                actor,
                MutationMeta::new("templates.create", format!("Added template “{name}”"), Capability::Edit).target("template", &id),
                |tx| {
                    let now = now_ms();
                    tx.conn().execute(
                        "INSERT INTO template(id, template_type, name, content_json, created_by, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![id, t, name, json, tx.actor().user_id, now],
                    )?;
                    Ok(())
                },
            )?;
        }
    }
    find(core, scope, &id)
}

// ---------------------------------------------------------------- operations

fn list(core: &AppCore, actor: &Actor, args: TemplateListArgs) -> AppResult<TemplatesListing> {
    let t = args.template_type.as_deref();
    let builtin = builtins()
        .into_iter()
        .filter(|b| t.map(|t| b.template_type == t).unwrap_or(true))
        .collect();
    let global = core.with_app_db(|c| global_rows(c, None, t))?;
    let (project, project_open) = match core.project_opt() {
        Some(s) => {
            actor.require(Capability::View, "view templates")?;
            (s.store.read(|c| project_rows(c, None, t))?, true)
        }
        None => (vec![], false),
    };
    Ok(TemplatesListing {
        builtin,
        global,
        project,
        project_open,
    })
}

fn get(core: &AppCore, actor: &Actor, args: TemplateRefArgs) -> AppResult<TemplateDto> {
    if args.scope == TemplateScope::Project {
        actor.require(Capability::View, "view templates")?;
    }
    find(core, args.scope, &args.id)
}

fn create(core: &AppCore, actor: &Actor, args: CreateTemplateArgs) -> AppResult<TemplateDto> {
    check_type(&args.template_type)?;
    let name = required_text(&args.name, "Template name", 120)?;
    let content = check_content(args.content)?;
    insert(
        core,
        actor,
        args.scope,
        &args.template_type,
        &name,
        &content,
    )
}

fn update(core: &AppCore, actor: &Actor, args: UpdateTemplateArgs) -> AppResult<TemplateDto> {
    let name = args
        .name
        .as_deref()
        .map(|n| required_text(n, "Template name", 120))
        .transpose()?;
    let content = args.content.map(|c| check_content(Some(c))).transpose()?;
    match args.scope {
        TemplateScope::Builtin => {
            return Err(AppError::invalid_input(
                "Built-in templates can't be changed. Copy one to edit it.",
            ));
        }
        TemplateScope::Global => {
            require_local(actor)?;
            let n = core.with_app_db(|c| {
                let now = now_ms();
                let mut changed = 0;
                if let Some(n) = &name {
                    changed = c.execute(
                        "UPDATE sys_global_template SET name=?1, updated_at=?2 WHERE id=?3",
                        params![n, now, args.id],
                    )?;
                }
                if let Some(v) = &content {
                    changed = c.execute(
                        "UPDATE sys_global_template SET content_json=?1, updated_at=?2 WHERE id=?3",
                        params![v.to_string(), now, args.id],
                    )?;
                }
                if name.is_none() && content.is_none() {
                    changed = c.query_row(
                        "SELECT count(*) FROM sys_global_template WHERE id=?1",
                        [&args.id],
                        |r| r.get(0),
                    )?;
                }
                Ok(changed)
            })?;
            if n == 0 {
                return Err(AppError::not_found("template"));
            }
        }
        TemplateScope::Project => {
            let s = core.project()?;
            s.store.mutate(
                actor,
                MutationMeta::new("templates.update", "Edited template", Capability::Edit)
                    .target("template", &args.id)
                    .coalesce(format!("templates.update:{}", args.id)),
                |tx| {
                    let live: bool = tx.conn().query_row(
                        "SELECT EXISTS(SELECT 1 FROM template WHERE id=?1 AND deleted_at IS NULL)",
                        [&args.id],
                        |r| r.get(0),
                    )?;
                    if !live {
                        return Err(AppError::not_found("template"));
                    }
                    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
                    if let Some(n) = &name {
                        fields.push(("name", text(n.clone())));
                    }
                    if let Some(v) = &content {
                        fields.push(("content_json", text(v.to_string())));
                    }
                    update_fields(
                        tx.conn(),
                        "template",
                        &args.id,
                        &fields,
                        &["name", "content_json"],
                        None,
                        "template",
                    )?;
                    Ok(())
                },
            )?;
        }
    }
    find(core, args.scope, &args.id)
}

/// Project templates go to Recently Deleted; the user's global templates are
/// removed from this computer (the UI asks for confirmation first).
fn delete(core: &AppCore, actor: &Actor, args: TemplateRefArgs) -> AppResult<()> {
    match args.scope {
        TemplateScope::Builtin => Err(AppError::invalid_input(
            "Built-in templates can't be deleted.",
        )),
        TemplateScope::Global => {
            require_local(actor)?;
            let n = core.with_app_db(|c| {
                Ok(c.execute("DELETE FROM sys_global_template WHERE id=?1", [&args.id])?)
            })?;
            if n == 0 {
                Err(AppError::not_found("template"))
            } else {
                Ok(())
            }
        }
        TemplateScope::Project => {
            let s = core.project()?;
            let name = find(core, TemplateScope::Project, &args.id)?.name;
            s.store.mutate(
                actor,
                MutationMeta::new(
                    "templates.delete",
                    format!("Deleted template “{name}”"),
                    Capability::SoftDelete,
                )
                .target("template", &args.id),
                |tx| {
                    soft_delete(
                        tx,
                        DeleteSpec {
                            object_type: "template",
                            table: "template",
                            id: &args.id,
                            title: Some(name.clone()),
                            parent_type: None,
                            parent_id: None,
                            position: None,
                        },
                    )
                },
            )
        }
    }
}

/// Use a template (copy into the project) or keep a project template for every
/// project (copy to global). The copy is independent of its source.
fn copy(core: &AppCore, actor: &Actor, args: CopyTemplateArgs) -> AppResult<TemplateDto> {
    if args.to_scope == TemplateScope::Builtin {
        return Err(AppError::invalid_input(
            "Templates can't be copied into the built-in set.",
        ));
    }
    let src = get(
        core,
        actor,
        TemplateRefArgs {
            scope: args.scope,
            id: args.id.clone(),
        },
    )?;
    let name = match args.name.as_deref() {
        Some(n) => required_text(n, "Template name", 120)?,
        None => src.name.clone(),
    };
    insert(
        core,
        actor,
        args.to_scope,
        &src.template_type,
        &name,
        &src.content,
    )
}

fn index_template(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, Option<i64>)> = c
        .query_row(
            "SELECT name, template_type, deleted_at FROM template WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((name, t, None)) => Some(SearchDoc {
            entity_type: "template".into(),
            title: name,
            body: type_label(&t).unwrap_or("Template").to_string(),
            context: "Templates".into(),
            nav: json!({ "workspace": "settings", "sub": "templates", "templateId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn purge(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM template WHERE id=?1", [&row.object_id])?;
    Ok(())
}
