//! Templates (FSD §59, §113; Domain §20; UX §3.37, mock 153).
//!
//! Three scopes:
//! * `builtin`: read-only starting points shipped with OpenFrame;
//! * `global`: the user's own templates, stored in the application database
//!   (`sys_global_template`) and usable from any project;
//! * `project`: templates inside the open project (`template` table: undoable,
//!   recoverable delete).
//!
//! "Use" copies a template: the copy gets a new identity, so later edits to the
//! template never change anything already created from it (FSD §113).
//! `templates.apply` creates the document a template describes in the open project
//! (a screenplay starter, Story Board acts and beats, a call sheet, a shot list, a
//! moodboard, a budget). The template content is read once and copied into new
//! records with the owning module's own operations; nothing refers back to the
//! template afterwards.
//!
//! Built-in set (owner decision 2026-10-07, production depth spec §34): Feature Film
//! Screenplay, Short Film Screenplay, Episodic / Series, Simple Story Board, Indie
//! Call Sheet, Shot List, Moodboard and Lightweight Budget. The daily production
//! report is a report type, not a template.

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
    use crate::registry::OperationMetadata as M;
    r.query("templates.list", list)
        .meta(M::read("Built-in, personal and project templates."));
    r.query("templates.get", get)
        .meta(M::read("One template with its content."));
    r.command("templates.create", create)
        .meta(M::edit("Create a project or personal template."));
    r.command("templates.update", update)
        .meta(M::edit("Rename a template or change its content."));
    r.command("templates.delete", delete)
        .meta(M::soft_delete("Delete a project or personal template.").confirm());
    r.command("templates.copy", copy).meta(
        M::edit("Copy a template into this project or into My Templates.")
            .hidden(crate::registry::hidden::CROSS_PROJECT),
    );
    r.command("templates.apply", apply).meta(
        M::edit(
            "Create the document a template describes (screenplay starter, Story Board, call sheet, shot list, moodboard or budget) in this project.",
        )
        .hidden(crate::registry::hidden::DUPLICATE),
    );
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
    ("screenplay", "Screenplay starter"),
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

#[derive(Debug, Clone, Deserialize, TS)]
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
            "feature_screenplay",
            "screenplay",
            "Feature Film Screenplay",
            json!({ "description": "Feature film · three acts",
                    "scenes": [
                        { "heading": "EXT. OPENING LOCATION - DAY",
                          "elements": [{ "type": "action", "text": "The first image of the film." }] },
                        { "heading": "INT. MIDPOINT LOCATION - NIGHT",
                          "elements": [{ "type": "action", "text": "The story turns." }] },
                        { "heading": "EXT. FINAL LOCATION - DAY",
                          "elements": [{ "type": "action", "text": "The last image of the film." },
                                       { "type": "transition", "text": "FADE OUT." }] }
                    ] }),
        ),
        b(
            "short_screenplay",
            "screenplay",
            "Short Film Screenplay",
            json!({ "description": "Short film · one location",
                    "scenes": [
                        { "heading": "INT. MAIN LOCATION - DAY",
                          "elements": [{ "type": "action", "text": "Who we meet and what they want." },
                                       { "type": "transition", "text": "CUT TO BLACK." }] }
                    ] }),
        ),
        b(
            "episodic_screenplay",
            "screenplay",
            "Episodic / Series",
            json!({ "description": "Series episode · cold open and four acts",
                    "scenes": [
                        { "heading": "EXT. COLD OPEN LOCATION - NIGHT",
                          "elements": [{ "type": "note", "text": "COLD OPEN" },
                                       { "type": "action", "text": "The hook before the titles." }] },
                        { "heading": "INT. ACT ONE LOCATION - DAY",
                          "elements": [{ "type": "note", "text": "ACT ONE" }] },
                        { "heading": "INT. ACT TWO LOCATION - DAY",
                          "elements": [{ "type": "note", "text": "ACT TWO" }] },
                        { "heading": "INT. ACT THREE LOCATION - NIGHT",
                          "elements": [{ "type": "note", "text": "ACT THREE" }] },
                        { "heading": "EXT. ACT FOUR LOCATION - DAY",
                          "elements": [{ "type": "note", "text": "ACT FOUR" }] }
                    ] }),
        ),
        b(
            "story_board",
            "story_board",
            "Simple Story Board",
            json!({ "description": "Three acts with key beats", "acts": ["Act One", "Act Two", "Act Three"],
                    "beats": ["Opening image", "Inciting incident", "Midpoint", "Crisis", "Climax", "Resolution"] }),
        ),
        b(
            "call_sheet",
            "call_sheet",
            "Indie Call Sheet",
            json!({ "description": "Six default sections plus weather, crew and walkie channels",
                    "sections": ["weather", "crew", "communication"],
                    "practical": { "safety": "Report any accident or near miss to the 1st AD straight away.",
                                   "meals": "Lunch on set." } }),
        ),
        b(
            "shot_list",
            "shot_list",
            "Shot List",
            json!({ "description": "Standard coverage for one scene",
                    "shots": [
                        { "description": "Master shot", "size": "Wide" },
                        { "description": "Two-shot", "size": "Medium" },
                        { "description": "Single on the first character", "size": "Close-up" },
                        { "description": "Single on the second character", "size": "Close-up" },
                        { "description": "Insert of the key detail", "size": "Insert" }
                    ] }),
        ),
        b(
            "moodboard",
            "moodboard",
            "Moodboard",
            json!({ "description": "Look and feel board", "sections": ["Light", "Colour", "Locations", "Costume", "Texture"] }),
        ),
        b(
            "budget",
            "budget",
            "Lightweight Budget",
            json!({ "description": "Nine categories with Planned and Actual",
                    "lines": [
                        { "category": "Cast", "description": "Lead cast" },
                        { "category": "Crew", "description": "Key crew" },
                        { "category": "Locations", "description": "Location fees and permits" },
                        { "category": "Equipment", "description": "Camera, lights and sound" },
                        { "category": "Art/Props", "description": "Set dressing and props" },
                        { "category": "Travel/Transport", "description": "Fuel and vehicles" },
                        { "category": "Food", "description": "Meals on set" },
                        { "category": "Post/Other", "description": "Edit, sound mix and colour" },
                        { "category": "Contingency", "description": "Contingency" }
                    ] }),
        ),
    ]
}

// ------------------------------------------------------------------ use a template

/// What a call sheet takes from a template: the optional sections to show and default
/// practical notes. Applied to a fresh copy of the prefilled document.
#[derive(Debug, Clone, Default)]
pub struct CallSheetLayout {
    pub sections: Vec<String>,
    pub practical: Vec<(String, String)>,
}

impl CallSheetLayout {
    pub fn apply(&self, doc: &mut crate::modules::schedule::callsheet::CallSheetDocument) {
        let o = &mut doc.optional;
        for s in &self.sections {
            match s.as_str() {
                "weather" => {
                    o.weather.get_or_insert_with(String::new);
                }
                "specialNotes" => {
                    o.special_notes.get_or_insert_with(String::new);
                }
                "attachments" => {
                    o.attachments.get_or_insert_with(Vec::new);
                }
                "referenceImages" => {
                    o.reference_images.get_or_insert_with(Vec::new);
                }
                "extras" => {
                    o.extras.get_or_insert_with(Vec::new);
                }
                "departments" => {
                    o.departments.get_or_insert_with(Vec::new);
                }
                "advance" => {
                    o.advance.get_or_insert_with(Default::default);
                }
                "crew" => {
                    o.crew.get_or_insert_with(Vec::new);
                }
                "communication" => {
                    o.communication.get_or_insert_with(Default::default);
                }
                _ => {}
            }
        }
        let p = &mut doc.practical;
        for (k, v) in &self.practical {
            let field = match k.as_str() {
                "parking" => &mut p.parking,
                "meetingPoint" => &mut p.meeting_point,
                "transport" => &mut p.travel_notes,
                "meals" => &mut p.meal_break,
                "toilets" => &mut p.toilets,
                "access" => &mut p.access,
                "safety" => &mut p.safety,
                "specialInstructions" => &mut p.special_instructions,
                _ => continue,
            };
            if field.trim().is_empty() {
                *field = v.clone();
            }
        }
    }
}

/// Read a call sheet template once (used by `callsheets.create`).
pub fn call_sheet_layout(
    core: &AppCore,
    actor: &Actor,
    t: &TemplateRefArgs,
) -> AppResult<CallSheetLayout> {
    let tpl = get(core, actor, t.clone())?;
    if tpl.template_type != "call_sheet" {
        return Err(AppError::invalid_input("Choose a call sheet template."));
    }
    Ok(call_sheet_layout_of(&tpl.content))
}

fn call_sheet_layout_of(content: &Value) -> CallSheetLayout {
    // Sections are named by key ("weather") or by the label the app shows ("Weather").
    let sections = content
        .get("sections")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter_map(|s| {
                    crate::modules::schedule::callsheet::OPTIONAL_SECTIONS
                        .iter()
                        .find(|(k, l)| {
                            k.eq_ignore_ascii_case(s.trim()) || l.eq_ignore_ascii_case(s.trim())
                        })
                        .map(|(k, _)| k.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    let practical = content
        .get("practical")
        .and_then(Value::as_object)
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                .collect()
        })
        .unwrap_or_default();
    CallSheetLayout {
        sections,
        practical,
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyTemplateArgs {
    pub scope: TemplateScope,
    pub id: String,
    /// Shooting day for a call sheet template.
    #[serde(default)]
    #[ts(optional)]
    pub day_id: Option<String>,
    /// Screenplay scene for a shot list template.
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
    /// Currency for a budget template when the project has no budget yet (default USD).
    #[serde(default)]
    #[ts(optional)]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTemplate {
    pub template_type: String,
    /// Where the new document opens.
    #[ts(type = "Record<string, unknown>")]
    pub nav: Value,
    /// "Created a call sheet for Shoot Day 4".
    pub summary: String,
    /// Records created.
    pub created: usize,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(200)
                .collect()
        })
        .unwrap_or_default()
}

/// A list entry that is either plain text or an object with the text in `field` (templates made
/// in the app hold plain lines; the built-in ones hold objects).
fn text_or<'a>(v: &'a Value, field: &str) -> Option<&'a str> {
    v.as_str()
        .or_else(|| v.get(field).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// A budget template line: `{ "category", "description" }`, or the text "Category: description"
/// (a line without a known category goes to Post/Other).
fn budget_line_of(v: &Value) -> Option<(String, String)> {
    use crate::modules::schedule::budget::BUDGET_CATEGORIES;
    if let (Some(cat), Some(desc)) = (
        v.get("category").and_then(Value::as_str),
        v.get("description").and_then(Value::as_str),
    ) {
        return Some((cat.to_string(), desc.to_string()));
    }
    let text = v.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    if let Some((cat, desc)) = text.split_once(':')
        && let Some(known) = BUDGET_CATEGORIES
            .iter()
            .find(|c| c.eq_ignore_ascii_case(cat.trim()))
        && !desc.trim().is_empty()
    {
        return Some((known.to_string(), desc.trim().to_string()));
    }
    Some(("Post/Other".to_string(), text.to_string()))
}

/// The id an operation returned: a plain id, `{ "id" }`, or an edit result's `createdIds[0]`.
fn id_of(v: &Value) -> Option<String> {
    v.as_str()
        .or_else(|| v.get("id").and_then(Value::as_str))
        .or_else(|| v.pointer("/createdIds/0").and_then(Value::as_str))
        .map(str::to_string)
}

/// "Use Template": create the document a template describes. Each record is made by the
/// owning module's operation (with its own validation, permissions and search), and the
/// template content is copied, so later template edits never change it.
///
/// All or nothing: when any record fails, the ones already made are undone and the project is
/// as it was; on success the steps are grouped into one "Used template" undo step.
fn apply(core: &AppCore, actor: &Actor, a: ApplyTemplateArgs) -> AppResult<AppliedTemplate> {
    use crate::modules::ai::change_set::{group_undo_steps, steps_since};
    actor.require(Capability::Edit, "use a template in this project")?;
    let s = core.project()?;
    let before: i64 = s.store.read(|c| {
        Ok(c.query_row(
            "SELECT COALESCE(MAX(seq), 0) FROM sys_undo WHERE actor_id IS ?1",
            [&actor.user_id],
            |r| r.get(0),
        )?)
    })?;
    match apply_steps(core, actor, a) {
        Ok(done) => {
            let label = format!("Used template: {}", done.summary);
            if let Err(e) = group_undo_steps(&s.store, actor, before, &label, None) {
                // Grouping is a convenience; every step stays undoable on its own.
                tracing::warn!(code = e.code_str(), "could not group template undo steps");
            }
            Ok(done)
        }
        Err(e) => {
            for _ in 0..steps_since(&s.store, actor, before)? {
                if let Err(undo_err) = s.store.undo(actor) {
                    tracing::error!(
                        code = undo_err.code_str(),
                        "could not roll back a partly used template"
                    );
                    return Err(AppError::new(
                        e.code_str(),
                        format!(
                            "{} Some records were already made and could not be removed automatically; use Undo to remove them.",
                            e.message
                        ),
                    ));
                }
            }
            Err(e)
        }
    }
}

fn apply_steps(core: &AppCore, actor: &Actor, a: ApplyTemplateArgs) -> AppResult<AppliedTemplate> {
    let s = core.project()?;
    let tpl = get(
        core,
        actor,
        TemplateRefArgs {
            scope: a.scope,
            id: a.id.clone(),
        },
    )?;
    let c = &tpl.content;
    let run = |op: &str, args: Value| core.dispatch(actor, op, args);
    let done = |nav: Value, summary: String, created: usize| AppliedTemplate {
        template_type: tpl.template_type.clone(),
        nav,
        summary,
        created,
    };
    match tpl.template_type.as_str() {
        "call_sheet" => {
            let day = a.day_id.ok_or_else(|| {
                AppError::invalid_input("Choose the shooting day for the call sheet.")
            })?;
            let id = run(
                "callsheets.create",
                json!({ "dayId": day, "template": { "scope": a.scope, "id": a.id } }),
            )?;
            let id = id_of(&id).unwrap_or_default();
            Ok(done(
                json!({ "workspace": "callsheets", "callSheetId": id }),
                format!("Created a call sheet from “{}”", tpl.name),
                1,
            ))
        }
        "budget" => {
            let existing: Option<String> = s.store.read(|c| {
                Ok(c.query_row(
                    "SELECT id FROM budget_snapshot WHERE is_current = 1 AND deleted_at IS NULL LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .optional()?)
            })?;
            let budget = match existing {
                Some(id) => id,
                None => {
                    let currency = a.currency.unwrap_or_else(|| "USD".into());
                    id_of(&run("budget.create", json!({ "currency": currency }))?)
                        .unwrap_or_default()
                }
            };
            let mut n = 0;
            for line in c
                .get("lines")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(200)
            {
                let Some((cat, desc)) = budget_line_of(line) else {
                    continue;
                };
                run(
                    "budget.add_line",
                    json!({ "budgetId": budget, "category": cat, "description": desc,
                            "planned": line.get("planned").and_then(Value::as_i64).unwrap_or(0) }),
                )?;
                n += 1;
            }
            Ok(done(
                json!({ "workspace": "production", "sub": "budget" }),
                format!("Added {n} budget lines from “{}”", tpl.name),
                n,
            ))
        }
        "story_board" => {
            let mut n = 0;
            let mut acts = Vec::new();
            for act in strings(c.get("acts")) {
                acts.extend(id_of(&run("story.create_act", json!({ "title": act }))?));
                n += 1;
            }
            // Beats go into the first act the template created (the Parking Lot when it has none).
            let parent = acts
                .first()
                .map(|act| json!({ "parentType": "act", "parentId": act }));
            for beat in strings(c.get("beats")) {
                run(
                    "story.create_beat",
                    json!({ "parent": parent, "text": beat }),
                )?;
                n += 1;
            }
            Ok(done(
                json!({ "workspace": "story" }),
                format!("Added {n} Story Board items from “{}”", tpl.name),
                n,
            ))
        }
        "shot_list" => {
            let scene = a
                .scene_id
                .ok_or_else(|| AppError::invalid_input("Choose the scene for the shot list."))?;
            let mut n = 0;
            for shot in c
                .get("shots")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(200)
            {
                let Some(desc) = text_or(shot, "description") else {
                    continue;
                };
                let mut args = json!({ "sceneId": scene, "description": desc });
                if let Some(size) = shot.get("size").and_then(Value::as_str) {
                    args["size"] = json!(size);
                }
                run("shot.create", args)?;
                n += 1;
            }
            Ok(done(
                json!({ "workspace": "production", "sub": "shots", "sceneId": scene }),
                format!("Added {n} shots from “{}”", tpl.name),
                n,
            ))
        }
        "moodboard" => {
            let board =
                id_of(&run("moodboard.create", json!({ "name": tpl.name }))?).unwrap_or_default();
            let mut n = 1;
            for section in strings(c.get("sections")) {
                run(
                    "moodboard.add_note",
                    json!({ "moodboardId": board, "text": section }),
                )?;
                n += 1;
            }
            Ok(done(
                json!({ "workspace": "production", "sub": "moodboards", "moodboardId": board }),
                format!("Created a moodboard from “{}”", tpl.name),
                n,
            ))
        }
        "screenplay" => {
            let existing: i64 = s.store.read(|c| {
                Ok(c.query_row(
                    "SELECT count(*) FROM screenplay WHERE deleted_at IS NULL AND episode_id IS NULL",
                    [],
                    |r| r.get(0),
                )?)
            })?;
            if existing > 0 {
                return Err(AppError::conflict(
                    "This project already has a screenplay. Start a new project to use a screenplay template.",
                ));
            }
            let created = run("screenplay.create", json!({}))?;
            let draft = created["draft"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mut n = 1;
            for scene in c
                .get("scenes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(200)
            {
                let Some(heading) = text_or(scene, "heading") else {
                    continue;
                };
                let sid = id_of(&run(
                    "screenplay.create_scene",
                    json!({ "draftId": draft, "heading": heading }),
                )?)
                .unwrap_or_default();
                n += 1;
                for el in scene
                    .get("elements")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .take(200)
                {
                    if let (Some(t), Some(text)) = (
                        el.get("type").and_then(Value::as_str),
                        el.get("text").and_then(Value::as_str),
                    ) {
                        run(
                            "screenplay.insert_element",
                            json!({ "sceneId": sid, "elementType": t, "text": text }),
                        )?;
                    }
                }
            }
            Ok(done(
                json!({ "workspace": "screenplay" }),
                format!("Started the screenplay from “{}”", tpl.name),
                n,
            ))
        }
        _ => Err(AppError::invalid_input(
            "This template can't create a document. Copy it into the project to keep it.",
        )),
    }
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
