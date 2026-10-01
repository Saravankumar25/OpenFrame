//! Mutation proposal catalogue (AI spec §10.5–§10.6, §12, §21; FSD §42.7–§42.13).
//!
//! Each proposal tool turns validated arguments into a Change Set whose
//! operations are ordinary registry operations (`{op, args}`) — the same
//! operations the UI invokes. Nothing here writes to the project: proposals are
//! applied only by `change_set::accept` after explicit user approval.
//!
//! A proposal kind is offered only when every operation it needs is registered
//! in this build and explicitly exposed by its metadata, so the assistant never
//! suggests a change it cannot perform (and never one that is not allowed).
//!
//! The catalogue covers every meaningful user mutation (agentic spec §28); the
//! domain builders live in `catalog/<domain>.rs`. Arguments are validated against
//! each tool's strict, hand-written schema before a builder runs.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use openframe_domain::enums::ProjectStatus;
use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};

use super::queries::{self, join_or};
use super::scope::ResolvedScope;
use super::toolbox::schema as sc;
use super::tools::parse_args;
use super::types::*;
use crate::registry::Registry;

pub mod kit;
mod production;
mod schedule;
mod screenplay;
mod story;
mod visual;
mod workspace;

/// Builds a Change Set draft from schema-validated arguments. Never writes.
pub type Builder = fn(&PropCtx<'_>, &ProposalSpec, &Value) -> AppResult<ChangeSetDraft>;

pub struct ProposalSpec {
    pub tool: &'static str,
    pub description: &'static str,
    /// Registry operations this proposal may contain (all must be registered for it to be offered).
    pub required_ops: &'static [&'static str],
    /// Optional operations that widen the proposal when available.
    pub optional_ops: &'static [&'static str],
    pub cap: Capability,
    /// "create Scene Cards" → "…so creating Scene Cards is not available to you."
    pub action_phrase: &'static str,
    pub module: &'static str,
    pub schema: fn() -> Value,
    pub build: Builder,
}

/// What a proposal builder may read: canonical data through the caller's read
/// connection, the actor, the registry and (when known) the request's scope.
pub struct PropCtx<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub registry: &'a Registry,
    pub scope: Option<&'a ResolvedScope>,
}

fn scene_card_schema() -> Value {
    sc::obj(
        &[
            ("description", sc::s(600)),
            ("heading", sc::s(200)),
            ("act", sc::reference()),
            ("sequence", sc::reference()),
        ],
        &["description"],
    )
}
fn task_schema() -> Value {
    sc::obj(
        &[
            ("title", sc::s(200)),
            ("notes", sc::s(2000)),
            ("dueDate", sc::sd(10, "YYYY-MM-DD")),
        ],
        &["title"],
    )
}
fn note_schema() -> Value {
    sc::obj(&[("title", sc::s(200)), ("body", sc::s(20_000))], &["body"])
}
fn folder_schema() -> Value {
    sc::obj(&[("name", sc::s(120))], &["name"])
}
fn rename_file_schema() -> Value {
    sc::obj(
        &[("currentName", sc::reference()), ("newName", sc::s(200))],
        &["currentName", "newName"],
    )
}
fn status_schema() -> Value {
    let names: Vec<&str> = ProjectStatus::ALL.iter().map(|s| s.as_str()).collect();
    sc::obj(&[("status", sc::en(&names))], &["status"])
}
fn rename_character_schema() -> Value {
    sc::obj(
        &[
            ("from", sc::s(120)),
            ("to", sc::s(120)),
            ("includeRawText", sc::boolean()),
        ],
        &["from", "to"],
    )
}

// Operation names of the owning modules (registry contract).
pub const OP_CREATE_SCENE_CARD: &str = "story.create_card";
pub const OP_UPDATE_CHARACTER: &str = "story.update_character";
pub const OP_UPDATE_ELEMENT: &str = "screenplay.update_element";
pub const OP_UPDATE_CATALOG_ITEM: &str = "catalog.update";
pub const OP_CREATE_TASK: &str = "tasks.create";
pub const OP_CREATE_NOTE: &str = "notes.create";
pub const OP_CREATE_FOLDER: &str = "files.create_folder";
pub const OP_RENAME_FILE: &str = "files.rename";
pub const OP_SET_STATUS: &str = "project.set_status";

const CORE: &[ProposalSpec] = &[
    ProposalSpec {
        tool: "propose_scene_card",
        description: "Prepare a new Scene Card (in a named act or sequence; otherwise in the Parking Lot).",
        required_ops: &[OP_CREATE_SCENE_CARD],
        optional_ops: &[],
        cap: Capability::Edit,
        action_phrase: "creating Scene Cards",
        module: "Story",
        schema: scene_card_schema,
        build: scene_card,
    },
    ProposalSpec {
        tool: "propose_task",
        description: "Prepare a new project task (optionally with notes and a due date).",
        required_ops: &[OP_CREATE_TASK],
        optional_ops: &[],
        cap: Capability::Edit,
        action_phrase: "creating tasks",
        module: "Notes & Tasks",
        schema: task_schema,
        build: task,
    },
    ProposalSpec {
        tool: "propose_project_note",
        description: "Prepare a new Project Note (e.g. to save an answer).",
        required_ops: &[OP_CREATE_NOTE],
        optional_ops: &[],
        cap: Capability::Edit,
        action_phrase: "creating Project Notes",
        module: "Notes & Tasks",
        schema: note_schema,
        build: note,
    },
    ProposalSpec {
        tool: "propose_folder",
        description: "Prepare a new folder in Project Files.",
        required_ops: &[OP_CREATE_FOLDER],
        optional_ops: &[],
        cap: Capability::Edit,
        action_phrase: "changing Project Files",
        module: "Files",
        schema: folder_schema,
        build: folder,
    },
    ProposalSpec {
        tool: "propose_rename_file",
        description: "Prepare renaming a file in Project Files.",
        required_ops: &[OP_RENAME_FILE],
        optional_ops: &[],
        cap: Capability::Edit,
        action_phrase: "renaming files",
        module: "Files",
        schema: rename_file_schema,
        build: rename_file,
    },
    ProposalSpec {
        tool: "propose_project_status",
        description: "Prepare changing the project status.",
        required_ops: &[OP_SET_STATUS],
        optional_ops: &[],
        cap: Capability::ManageProject,
        action_phrase: "changing the project status",
        module: "Project",
        schema: status_schema,
        build: project_status,
    },
    ProposalSpec {
        tool: "propose_rename_character",
        description: "Prepare a structured rename of a character across the project (Character record, screenplay Character cues, Cast catalog entries). Raw dialogue/action text is excluded unless includeRawText is true.",
        required_ops: &[OP_UPDATE_CHARACTER],
        optional_ops: &[OP_UPDATE_ELEMENT, OP_UPDATE_CATALOG_ITEM],
        cap: Capability::Edit,
        action_phrase: "renaming characters",
        module: "Story",
        schema: rename_character_schema,
        build: rename_character,
    },
];

/// Every proposal tool, in catalogue order.
pub static PROPOSALS: LazyLock<Vec<&'static ProposalSpec>> = LazyLock::new(|| {
    CORE.iter()
        .chain(workspace::SPECS)
        .chain(story::SPECS)
        .chain(screenplay::SPECS)
        .chain(production::SPECS)
        .chain(schedule::SPECS)
        .chain(visual::SPECS)
        .collect()
});

pub fn spec(tool: &str) -> Option<&'static ProposalSpec> {
    PROPOSALS.iter().copied().find(|p| p.tool == tool)
}

fn op_available(reg: &Registry, op: &str) -> bool {
    super::toolbox::op_exposed(reg, op)
}

/// Proposal kinds this build can actually perform.
pub fn available(reg: &Registry) -> Vec<&'static ProposalSpec> {
    PROPOSALS
        .iter()
        .copied()
        .filter(|p| p.required_ops.iter().all(|op| op_available(reg, op)))
        .collect()
}

/// Every operation a Change Set may contain (defence in depth at apply time):
/// listed by a proposal tool AND explicitly exposed by its registry metadata.
/// `ai.*` operations (Change Set review, the assistant itself) never qualify.
pub fn allowed_op(op: &str) -> bool {
    PROPOSALS
        .iter()
        .any(|p| p.required_ops.contains(&op) || p.optional_ops.contains(&op))
        && super::toolbox::op_exposed(crate::registry::catalog(), op)
}

/// The capability the operation's command checks (from its explicit metadata),
/// used to pre-check permission at preview and apply time.
pub fn op_capability(op: &str) -> Capability {
    crate::registry::catalog()
        .metadata(op)
        .and_then(|m| m.required_capability)
        .unwrap_or(Capability::ManageProject)
}

/// "You are a Viewer on this project, so renaming characters is not available to you."
pub fn denial_message(actor: &Actor, spec: &ProposalSpec) -> String {
    format!(
        "You are {} {} on this project, so {} is not available to you.",
        article(actor.role.label()),
        actor.role.label(),
        spec.action_phrase
    )
}

fn article(w: &str) -> &'static str {
    if w.starts_with(['A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    }
}

/// Compatibility context for callers without a scope (the Change Set re-check path).
pub struct BuildCtx<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub registry: &'a Registry,
}

fn clean(s: &str, what: &str, max: usize) -> AppResult<String> {
    let t = s.trim();
    if t.is_empty() {
        return Err(queries::ambiguous(format!("What should the {what} be?")));
    }
    Ok(queries::truncate_chars(t, max))
}

fn draft(tool: &str, title: String, summary: String, module: &str, args: Value) -> ChangeSetDraft {
    ChangeSetDraft {
        title,
        summary,
        operations: Vec::new(),
        preview: Vec::new(),
        exclusions: Vec::new(),
        targets: Vec::new(),
        modules: vec![module.to_string()],
        base_rows: Vec::new(),
        source_tool: tool.to_string(),
        source_args: args,
        sources: Vec::new(),
    }
}

/// Build a Change Set proposal from tool arguments (validated against the tool's
/// strict schema here). Does not write anything.
pub fn build(ctx: &BuildCtx<'_>, tool: &str, args: &Value) -> AppResult<ChangeSetDraft> {
    let spec = spec(tool).ok_or_else(|| {
        AppError::ai(
            "unknown_tool",
            "That isn't something the assistant can do yet.",
        )
    })?;
    if !spec
        .required_ops
        .iter()
        .all(|op| op_available(ctx.registry, op))
    {
        return Err(AppError::ai(
            "unsupported",
            "I can't prepare that change in this version of OpenFrame. Nothing was changed.",
        ));
    }
    let args = if args.is_null() {
        json!({})
    } else {
        args.clone()
    };
    sc::validate(spec.tool, &(spec.schema)(), &args)?;
    let pctx = PropCtx {
        conn: ctx.conn,
        actor: ctx.actor,
        registry: ctx.registry,
        scope: None,
    };
    (spec.build)(&pctx, spec, &args)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SceneCardArgs {
    description: String,
    #[serde(default)]
    heading: Option<String>,
    #[serde(default)]
    act: Option<String>,
    #[serde(default)]
    sequence: Option<String>,
}

fn scene_card(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a: SceneCardArgs = parse_args(spec.tool, args)?;
    if let Some(seq) = a
        .sequence
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return story::card_in_sequence(ctx, spec, args, seq, &a.description, a.heading.as_deref());
    }
    let description = clean(&a.description, "Scene Card description", 600)?;
    let heading = a
        .heading
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(|h| queries::truncate_chars(h, 160));
    let mut d = draft(
        spec.tool,
        "Proposed Scene Card".into(),
        "I prepared a suggestion. Nothing has been added yet.".into(),
        spec.module,
        args.clone(),
    );
    let (parent_type, parent_id, place) =
        match a.act.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(act) => {
                let (id, title) = find_act(ctx.conn, act)?;
                d.base_rows.push(("story_act".into(), id.clone()));
                d.targets.push(ObjRef {
                    table: "story_act".into(),
                    id: id.clone(),
                    label: title.clone(),
                });
                ("act", Some(id), title)
            }
            None => ("parking", None, "Parking Lot".to_string()),
        };
    let mut parent = json!({"parentType": parent_type});
    if let Some(id) = &parent_id {
        parent["parentId"] = json!(id);
    }
    let mut op_args = json!({"parent": parent, "shortDescription": description});
    if let Some(h) = &heading {
        op_args["sceneHeading"] = json!(h);
    }
    d.operations.push(OpCall {
        op: OP_CREATE_SCENE_CARD.into(),
        args: op_args,
        label: format!(
            "Create Scene Card “{}”",
            queries::truncate_chars(&description, 60)
        ),
    });
    d.preview
        .push(PreviewRow::normal("Description", description));
    if let Some(h) = heading {
        d.preview.push(PreviewRow::normal("Scene heading", h));
    }
    d.preview.push(PreviewRow::normal("Place in", place));
    d.preview.push(PreviewRow::normal("Impact", "1 new object"));
    Ok(d)
}

fn find_act(c: &Connection, act: &str) -> AppResult<(String, String)> {
    let mut stmt =
        c.prepare("SELECT id, title FROM story_act WHERE deleted_at IS NULL ORDER BY position")?;
    let acts: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let a = act.to_lowercase();
    if let Some((id, t)) = acts.iter().find(|(_, t)| t.to_lowercase() == a) {
        return Ok((id.clone(), t.clone()));
    }
    let digits: String = a
        .trim_start_matches("act")
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if let Ok(n) = digits.parse::<usize>()
        && let Some((id, t)) = acts.get(n.wrapping_sub(1))
    {
        return Ok((id.clone(), t.clone()));
    }
    let partial: Vec<&(String, String)> = acts
        .iter()
        .filter(|(_, t)| t.to_lowercase().contains(&a))
        .collect();
    match partial.len() {
        1 => Ok(partial[0].clone()),
        0 => Err(AppError::ai(
            "not_found",
            format!("I couldn't find an act called “{act}”."),
        )),
        _ => Err(queries::ambiguous(format!(
            "Which act do you mean: {}?",
            join_or(&partial.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>())
        ))),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskArgs {
    title: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
}

fn task(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a: TaskArgs = parse_args(spec.tool, args)?;
    let due = match a
        .due_date
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
    {
        Some(d) => Some((d.to_string(), kit::date_ms(d)?)),
        None => None,
    };
    let title = clean(&a.title, "task title", 200)?;
    let mut d = draft(
        spec.tool,
        "Proposed task".into(),
        "I prepared a task. Nothing has been added yet.".into(),
        spec.module,
        args.clone(),
    );
    let mut op_args = json!({"title": title});
    if let Some(n) = a.notes.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        op_args["notes"] = json!(queries::truncate_chars(n, 2000));
        d.preview
            .push(PreviewRow::normal("Notes", queries::truncate_chars(n, 200)));
    }
    if let Some((text, ms)) = &due {
        op_args["dueAt"] = json!(ms);
        d.preview.push(PreviewRow::normal("Due", text.clone()));
    }
    d.preview
        .insert(0, PreviewRow::normal("Task", title.clone()));
    d.preview.push(PreviewRow::normal("Impact", "1 new object"));
    d.operations.push(OpCall {
        op: OP_CREATE_TASK.into(),
        args: op_args,
        label: format!("Create task “{title}”"),
    });
    Ok(d)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NoteArgs {
    #[serde(default)]
    title: Option<String>,
    body: String,
}

fn note(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a: NoteArgs = parse_args(spec.tool, args)?;
    let body = clean(&a.body, "note", 20_000)?;
    let title = a
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| queries::truncate_chars(t, 200));
    let mut d = draft(
        spec.tool,
        "Proposed Project Note".into(),
        "I prepared a Project Note. Nothing has been added yet.".into(),
        spec.module,
        args.clone(),
    );
    let mut op_args = json!({"body": body});
    if let Some(t) = &title {
        op_args["title"] = json!(t);
        d.preview.push(PreviewRow::normal("Title", t.clone()));
    }
    d.preview.push(PreviewRow::normal(
        "Note",
        queries::truncate_chars(&body, 240),
    ));
    d.preview.push(PreviewRow::normal("Impact", "1 new object"));
    d.operations.push(OpCall {
        op: OP_CREATE_NOTE.into(),
        args: op_args,
        label: "Create Project Note".into(),
    });
    Ok(d)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FolderArgs {
    name: String,
}

fn folder(_ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a: FolderArgs = parse_args(spec.tool, args)?;
    let name = clean(&a.name, "folder name", 120)?;
    let mut d = draft(
        spec.tool,
        "Proposed folder".into(),
        "I prepared a new folder. Nothing has been added yet.".into(),
        spec.module,
        args.clone(),
    );
    d.preview.push(PreviewRow::normal("Folder", name.clone()));
    d.preview
        .push(PreviewRow::normal("Place in", "Project Files"));
    d.preview.push(PreviewRow::normal("Impact", "1 new object"));
    d.operations.push(OpCall {
        op: OP_CREATE_FOLDER.into(),
        args: json!({"name": name}),
        label: format!("Create folder “{name}”"),
    });
    Ok(d)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenameFileArgs {
    current_name: String,
    new_name: String,
}

fn rename_file(ctx: &PropCtx<'_>, spec: &ProposalSpec, args: &Value) -> AppResult<ChangeSetDraft> {
    let a: RenameFileArgs = parse_args(spec.tool, args)?;
    let new_name = clean(&a.new_name, "new file name", 200)?;
    let cur = a.current_name.trim().to_lowercase();
    let mut stmt = ctx.conn.prepare("SELECT id, display_name, rev FROM project_file WHERE deleted_at IS NULL ORDER BY display_name")?;
    let files: Vec<(String, String, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let exact: Vec<&(String, String, i64)> = files
        .iter()
        .filter(|(_, n, _)| n.to_lowercase() == cur)
        .collect();
    let matches: Vec<&(String, String, i64)> = if exact.is_empty() {
        files
            .iter()
            .filter(|(_, n, _)| n.to_lowercase().contains(&cur))
            .collect()
    } else {
        exact
    };
    let (id, name, rev) = match matches.len() {
        1 => matches[0].clone(),
        0 => {
            return Err(AppError::ai(
                "not_found",
                format!(
                    "I couldn't find a file called “{}” in Project Files.",
                    a.current_name.trim()
                ),
            ));
        }
        _ => {
            return Err(queries::ambiguous(format!(
                "Which file do you mean: {}?",
                join_or(
                    &matches
                        .iter()
                        .take(5)
                        .map(|(_, n, _)| n.clone())
                        .collect::<Vec<_>>()
                )
            )));
        }
    };
    let mut d = draft(
        spec.tool,
        "Proposed rename".into(),
        "I prepared a rename. Nothing has been changed yet.".into(),
        spec.module,
        args.clone(),
    );
    d.base_rows.push(("project_file".into(), id.clone()));
    d.targets.push(ObjRef {
        table: "project_file".into(),
        id: id.clone(),
        label: name.clone(),
    });
    d.preview
        .push(PreviewRow::normal("File", format!("{name} → {new_name}")));
    d.preview
        .push(PreviewRow::normal("Impact", "1 object renamed"));
    d.operations.push(OpCall {
        op: OP_RENAME_FILE.into(),
        args: json!({"id": id, "name": new_name, "expectedRev": rev}),
        label: format!("Rename file “{name}” to “{new_name}”"),
    });
    Ok(d)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StatusArgs {
    status: String,
}

fn project_status(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a: StatusArgs = parse_args(spec.tool, args)?;
    let status = ProjectStatus::parse(a.status.trim()).ok_or_else(|| {
        queries::ambiguous(format!(
            "Which status should the project have: {}?",
            join_or(
                &ProjectStatus::ALL
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect::<Vec<_>>()
            )
        ))
    })?;
    let (id, current): (String, String) =
        ctx.conn
            .query_row("SELECT id, status FROM project LIMIT 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
    let mut d = draft(
        spec.tool,
        "Proposed status change".into(),
        "I prepared a status change. Nothing has been changed yet.".into(),
        spec.module,
        args.clone(),
    );
    d.base_rows.push(("project".into(), id.clone()));
    d.targets.push(ObjRef {
        table: "project".into(),
        id,
        label: "Project".into(),
    });
    d.preview.push(PreviewRow::normal(
        "Status",
        format!("{current} → {}", status.as_str()),
    ));
    d.operations.push(OpCall {
        op: OP_SET_STATUS.into(),
        args: json!({"status": status.as_str()}),
        label: format!("Set project status to {}", status.as_str()),
    });
    Ok(d)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenameCharacterArgs {
    from: String,
    to: String,
    #[serde(default)]
    include_raw_text: bool,
}

/// Replace the name part of a cue, keeping extensions: "RAVI (V.O.)" → "RAGHAV (V.O.)".
fn renamed_cue(text: &str, to: &str) -> String {
    let t = text.trim();
    match t.find('(') {
        Some(i) => format!("{} {}", to.to_uppercase(), t[i..].trim()),
        None => to.to_uppercase(),
    }
}

fn word_count(haystack: &str, needle: &str) -> usize {
    let h = haystack.to_lowercase();
    let n = needle.to_lowercase();
    if n.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = h[start..].find(&n) {
        let abs = start + pos;
        let before = h[..abs].chars().next_back();
        let after = h[abs + n.len()..].chars().next();
        if !before.is_some_and(|c| c.is_alphanumeric())
            && !after.is_some_and(|c| c.is_alphanumeric())
        {
            count += 1;
        }
        start = abs + n.len();
    }
    count
}

/// Structured rename (AI spec §12; FSD §42.12): structured references and raw
/// text are distinct categories; raw dialogue/action text is NOT included by default;
/// locked drafts are excluded and reported.
fn rename_character(
    ctx: &PropCtx<'_>,
    spec: &ProposalSpec,
    args: &Value,
) -> AppResult<ChangeSetDraft> {
    let a: RenameCharacterArgs = parse_args(spec.tool, args)?;
    let from = clean(&a.from, "current name", 120)?;
    let to = clean(&a.to, "new name", 120)?;
    if from.eq_ignore_ascii_case(&to) {
        return Err(queries::ambiguous(
            "The new name is the same as the current one. What should the character be called?",
        ));
    }
    let c = ctx.conn;
    let records = queries::character_records(c)?;
    let exact: Vec<&(String, String, i64)> = records
        .iter()
        .filter(|(_, n, _)| n.eq_ignore_ascii_case(&from))
        .collect();
    let (char_id, char_name, _) = match exact.len() {
        1 => exact[0].clone(),
        0 => {
            let similar: Vec<String> = records
                .iter()
                .filter(|(_, n, _)| n.to_lowercase().contains(&from.to_lowercase()))
                .map(|(_, n, _)| n.clone())
                .collect();
            return Err(if similar.len() > 1 {
                queries::ambiguous(format!(
                    "There are {} characters with similar names: {}. Which one should I use?",
                    similar.len(),
                    queries::join_and(&similar)
                ))
            } else {
                AppError::ai(
                    "not_found",
                    format!("I couldn't find a Character record named “{from}”."),
                )
            });
        }
        _ => {
            return Err(queries::ambiguous(format!(
                "There are several Character records named “{from}”. Open the one you mean in the Character Room and ask again."
            )));
        }
    };
    let mut d = draft(
        spec.tool,
        String::new(),
        "I found the Character record and its structured references. Raw dialogue/action mentions are listed separately and not changed by default.".into(),
        spec.module,
        args.clone(),
    );
    let mut modules: BTreeMap<&str, ()> = BTreeMap::new();
    modules.insert("Story", ());
    d.base_rows
        .push(("story_character".into(), char_id.clone()));
    d.targets.push(ObjRef {
        table: "story_character".into(),
        id: char_id.clone(),
        label: char_name.clone(),
    });
    d.operations.push(OpCall {
        op: OP_UPDATE_CHARACTER.into(),
        args: json!({"id": char_id, "name": to}),
        label: format!("Rename Character {char_name} → {to}"),
    });
    d.preview.push(PreviewRow::normal(
        "Canonical Character",
        format!("{char_name} → {to} · 1 object"),
    ));

    // Screenplay Character cues (structured) in the current draft. Earlier drafts
    // are kept as written (historical versions are never silently rewritten) and a
    // locked draft is excluded — the lock can only be changed through a revision.
    let from_cue = queries::normalize_cue(&from);
    let current = queries::current_draft(c)?;
    let mut stmt = c.prepare(
        "SELECT e.id, e.text, d.id, d.name, d.status FROM screenplay_element e
         JOIN screenplay_scene s ON s.id=e.scene_id JOIN screenplay_draft d ON d.id=s.draft_id
         WHERE e.element_type='character' AND s.deleted_at IS NULL AND d.deleted_at IS NULL",
    )?;
    let cues: Vec<(String, String, String, String, String)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let matching: Vec<&(String, String, String, String, String)> = cues
        .iter()
        .filter(|(_, t, _, _, _)| queries::normalize_cue(t) == from_cue)
        .collect();
    let is_current = |draft_id: &String| current.as_ref().is_some_and(|d| &d.id == draft_id);
    let (in_current, earlier): (Vec<_>, Vec<_>) = matching
        .into_iter()
        .partition(|(_, _, did, _, _)| is_current(did));
    let (locked, open): (Vec<_>, Vec<_>) = in_current
        .into_iter()
        .partition(|(_, _, _, _, status)| status == "Locked");
    if !open.is_empty() {
        if op_available(ctx.registry, OP_UPDATE_ELEMENT) {
            modules.insert("Screenplay", ());
            for (id, text, _, _, _) in &open {
                d.base_rows.push(("screenplay_element".into(), id.clone()));
                d.operations.push(OpCall {
                    op: OP_UPDATE_ELEMENT.into(),
                    args: json!({"id": id, "text": renamed_cue(text, &to)}),
                    label: "Update screenplay Character cue".into(),
                });
            }
            let draft_name = current.as_ref().map(|d| d.label()).unwrap_or_default();
            d.preview.push(PreviewRow::normal(
                format!("Screenplay Character cues ({draft_name})"),
                open.len().to_string(),
            ));
        } else {
            d.exclusions.push(PreviewRow::excluded(
                "Screenplay Character cues (not available in this version)",
                open.len().to_string(),
            ));
        }
    }
    if !locked.is_empty() {
        let name = locked[0].3.clone();
        d.exclusions.push(PreviewRow::locked(
            "Locked / not allowed",
            format!("{} ({name} locked)", locked.len()),
        ));
    }
    if !earlier.is_empty() {
        d.exclusions.push(PreviewRow::excluded(
            "Earlier drafts (kept as written)",
            format!("{} cues", earlier.len()),
        ));
    }

    // Cast catalog entries for this character (the Production identity of the
    // character). Breakdown elements reference the catalog entry, so they follow it;
    // the previous name stays a catalog alias (Production module rule).
    let mut stmt = c.prepare(
        "SELECT id, name, rev FROM catalog_item WHERE deleted_at IS NULL AND category='Cast'
           AND (character_id=?1 OR (character_id IS NULL AND lower(name)=lower(?2)))",
    )?;
    let items: Vec<(String, String, i64)> = stmt
        .query_map(params![char_id, char_name], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<Result<_, _>>()?;
    let items: Vec<(String, String, i64)> = items
        .into_iter()
        .filter(|(_, n, _)| !n.eq_ignore_ascii_case(&to))
        .collect();
    if !items.is_empty() {
        if op_available(ctx.registry, OP_UPDATE_CATALOG_ITEM) {
            modules.insert("Production", ());
            for (id, name, rev) in &items {
                d.base_rows.push(("catalog_item".into(), id.clone()));
                d.operations.push(OpCall {
                    op: OP_UPDATE_CATALOG_ITEM.into(),
                    args: json!({"id": id, "name": to, "expectedRev": rev}),
                    label: format!("Rename Cast catalog entry {name} → {to}"),
                });
            }
            let linked = queries::count_in(
                c,
                "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL AND catalog_item_id IN",
                &items
                    .iter()
                    .map(|(id, _, _)| id.clone())
                    .collect::<Vec<_>>(),
            )?;
            d.preview.push(PreviewRow::normal(
                "Cast catalog entries",
                items.len().to_string(),
            ));
            if linked > 0 {
                d.preview.push(PreviewRow::normal(
                    "Breakdown references (linked, follow automatically)",
                    linked.to_string(),
                ));
            }
        } else {
            d.exclusions.push(PreviewRow::excluded(
                "Cast catalog entries (not available in this version)",
                items.len().to_string(),
            ));
        }
    }
    // Cast members link to the character by identity — they follow automatically.
    let cast = queries::count(
        c,
        "SELECT count(*) FROM cast_member WHERE deleted_at IS NULL AND character_id=?1",
        params![char_id],
    )?;
    if cast > 0 {
        d.preview.push(PreviewRow::normal(
            "Cast references (linked, follow automatically)",
            cast.to_string(),
        ));
    }

    // Raw dialogue/action text: counted, never changed by default.
    let mut stmt = c.prepare(
        "SELECT e.text FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id
         WHERE e.element_type IN ('dialogue','action','parenthetical') AND s.deleted_at IS NULL AND e.text LIKE ?1 ESCAPE '\\'",
    )?;
    // User text: `%` and `_` must match literally (LIKE wildcards).
    let pattern = format!(
        "%{}%",
        from.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let raw: usize = stmt
        .query_map([pattern], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .map(|t| word_count(&t, &from))
        .sum();
    if raw > 0 {
        let label = if a.include_raw_text {
            "Raw dialogue/action text (explicit text edits are made in the Screenplay)"
        } else {
            "Excluded: raw dialogue/action text"
        };
        d.exclusions
            .push(PreviewRow::excluded(label, format!("{raw} mentions")));
    }
    let objects = d.operations.len();
    d.modules = modules.keys().map(|m| m.to_string()).collect();
    d.title = format!(
        "Proposed changes — {} · {}",
        if objects == 1 {
            "1 object".to_string()
        } else {
            format!("{objects} objects")
        },
        if d.modules.len() == 1 {
            "1 module".to_string()
        } else {
            format!("{} modules", d.modules.len())
        }
    );
    Ok(d)
}

/// Current revision snapshot for base-version protection.
pub fn snapshot(c: &Connection, rows: &[(String, String)]) -> AppResult<Vec<Value>> {
    let mut out = Vec::new();
    for (table, id) in rows {
        let rev = queries::row_rev(c, table, id)?;
        out.push(json!({"table": table, "id": id, "rev": rev}));
    }
    Ok(out)
}

/// Current project id (base-version guard against a replaced/restored project).
pub fn project_id(c: &Connection) -> AppResult<Option<String>> {
    Ok(
        c.query_row("SELECT id FROM project LIMIT 1", [], |r| r.get(0))
            .optional()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_rename_keeps_extensions() {
        assert_eq!(renamed_cue("RAVI (V.O.)", "Raghav"), "RAGHAV (V.O.)");
        assert_eq!(renamed_cue("Ravi", "Raghav"), "RAGHAV");
    }

    #[test]
    fn raw_mentions_count_whole_words_only() {
        assert_eq!(word_count("Ravi sees Ravindra. RAVI!", "Ravi"), 2);
        assert_eq!(word_count("nothing here", "Ravi"), 0);
    }
}
