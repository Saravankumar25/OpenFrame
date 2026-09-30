//! Screenplay content editing (FSD §15.5–15.8, §16, §92).
//!
//! All content changes are element-level operations applied in order inside one
//! pipeline mutation. The editor batches keystrokes (~400 ms) into
//! `screenplay.apply_edits`; typing in one element coalesces into one undo step.
//! Server-side validation enforces element types and text shape; locked drafts
//! reject every change with `permission.locked_draft`.

use std::collections::BTreeSet;

use openframe_domain::enums::ElementType;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{place_at, sibling_ids, text, update_fields};
use regex::RegexBuilder;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{bump_seq, drafts, editable_draft, live_scene_ids, scene_number};
use crate::core::AppCore;
use crate::modules::comments;
use crate::registry::Registry;
use crate::store::{DeleteSpec, MutationMeta, Tx, soft_delete};
use crate::util::require_id;

pub fn register(r: &mut Registry) {
    r.command("screenplay.apply_edits", apply_edits);
    r.command("screenplay.insert_element", insert_element_cmd);
    r.command("screenplay.update_element", update_element_cmd);
    r.command("screenplay.delete_element", delete_element_cmd);
    r.command("screenplay.move_element", move_element_cmd);
    r.command("screenplay.create_scene", create_scene_cmd);
    r.command("screenplay.update_scene", update_scene_cmd);
    r.command("screenplay.move_scene", move_scene_cmd);
    r.command("screenplay.delete_scene", delete_scene_cmd);
    r.command("screenplay.split_scene", split_scene_cmd);
    r.command("screenplay.merge_scene", merge_scene_cmd);
    r.command("screenplay.replace_all", replace_all);
}

/// Largest single element (a very long action paragraph is still far below this).
pub const MAX_ELEMENT_BYTES: usize = 20_000;
pub const MAX_HEADING_CHARS: usize = 300;
const MAX_OPS: usize = 5_000;
const MAX_NOTE_BYTES: usize = 20_000;

// --------------------------------------------------------------------- ops

/// One element-level edit. Ops are applied in order; indexes refer to the
/// state after the previous ops (the editor computes them that way).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ScreenplayEditOp {
    #[serde(rename_all = "camelCase")]
    InsertElement {
        /// Client-generated identity (UUID) so later ops in the same batch can refer to it.
        #[serde(default)]
        #[ts(optional)]
        id: Option<String>,
        scene_id: String,
        #[serde(default)]
        #[ts(optional)]
        index: Option<u32>,
        element_type: ElementType,
        #[serde(default)]
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    UpdateElement {
        id: String,
        #[serde(default)]
        #[ts(optional)]
        element_type: Option<ElementType>,
        #[serde(default)]
        #[ts(optional)]
        text: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    DeleteElement { id: String },
    #[serde(rename_all = "camelCase")]
    MoveElement {
        id: String,
        scene_id: String,
        #[serde(default)]
        #[ts(optional)]
        index: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    InsertScene {
        #[serde(default)]
        #[ts(optional)]
        id: Option<String>,
        #[serde(default)]
        #[ts(optional)]
        index: Option<u32>,
        #[serde(default)]
        heading: String,
    },
    #[serde(rename_all = "camelCase")]
    UpdateScene { id: String, heading: String },
    #[serde(rename_all = "camelCase")]
    MoveScene { id: String, index: u32 },
    /// Recoverable delete; the scene keeps any elements still in it.
    #[serde(rename_all = "camelCase")]
    DeleteScene { id: String },
    /// The element becomes the heading of a new scene; following elements move into it.
    #[serde(rename_all = "camelCase")]
    SplitScene {
        element_id: String,
        #[serde(default)]
        #[ts(optional)]
        new_scene_id: Option<String>,
    },
    /// Merge a scene into the previous one (its heading becomes an action line).
    #[serde(rename_all = "camelCase")]
    MergeScene { scene_id: String },
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayApplyEditsArgs {
    pub draft_id: String,
    pub ops: Vec<ScreenplayEditOp>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayEditResult {
    #[ts(type = "number")]
    pub seq: i64,
    /// Ids created by the batch (scenes/elements), in op order.
    pub created_ids: Vec<String>,
}

/// What a batch touched: drives comment re-anchoring and search re-indexing.
#[derive(Default)]
pub(crate) struct Touched {
    pub scenes: BTreeSet<String>,
    pub elements: BTreeSet<String>,
    pub structure: bool,
    pub created: Vec<String>,
}

// -------------------------------------------------------------- validation

pub(crate) fn validate_element(t: ElementType, value: &str) -> AppResult<()> {
    if t == ElementType::SceneHeading {
        return Err(AppError::validation(
            "element_type",
            "A scene heading starts a new scene. Change the element to Scene Heading in the editor or add a new scene.",
        ));
    }
    if value.len() > MAX_ELEMENT_BYTES {
        return Err(AppError::invalid_input(
            "This paragraph is too long. Split it into several paragraphs.",
        ));
    }
    if value.contains('\0') {
        return Err(AppError::invalid_input(
            "The text contains characters that can't be stored.",
        ));
    }
    let single_line = matches!(
        t,
        ElementType::Character
            | ElementType::Parenthetical
            | ElementType::Transition
            | ElementType::Shot
    );
    if single_line && (value.contains('\n') || value.contains('\r')) {
        let what = match t {
            ElementType::Character => "A character name",
            ElementType::Parenthetical => "A parenthetical",
            ElementType::Transition => "A transition",
            _ => "A shot",
        };
        return Err(AppError::validation(
            "element_text",
            format!("{what} must be a single line."),
        ));
    }
    Ok(())
}

pub(crate) fn validate_heading(h: &str) -> AppResult<()> {
    if h.contains('\n') || h.contains('\r') || h.contains('\0') {
        return Err(AppError::validation(
            "heading",
            "A scene heading must be a single line.",
        ));
    }
    if h.chars().count() > MAX_HEADING_CHARS {
        return Err(AppError::invalid_input(format!(
            "A scene heading can be at most {MAX_HEADING_CHARS} characters."
        )));
    }
    Ok(())
}

fn fresh_id(c: &Connection, table: &str, wanted: Option<String>, what: &str) -> AppResult<String> {
    match wanted {
        Some(id) => {
            require_id(&id, what)?;
            let exists: bool = c.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
                [&id],
                |r| r.get(0),
            )?;
            if exists {
                return Err(AppError::conflict(
                    "That content already exists. Reload the screenplay and try again.",
                ));
            }
            Ok(id)
        }
        None => Ok(new_id()),
    }
}

// ---------------------------------------------------------------- lookups

struct ElementInfo {
    scene_id: String,
    element_type: ElementType,
    text: String,
}

fn element_in_draft(c: &Connection, id: &str, draft_id: &str) -> AppResult<ElementInfo> {
    let row: Option<(String, String, String, String, bool)> = c
        .query_row(
            "SELECT e.scene_id, e.element_type, e.text, s.draft_id, s.deleted_at IS NOT NULL
             FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id WHERE e.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    match row {
        Some((scene_id, t, text, d, false)) if d == draft_id => Ok(ElementInfo {
            scene_id,
            element_type: ElementType::parse_field(&t, "element type")?,
            text,
        }),
        _ => Err(AppError::not_found("screenplay element")),
    }
}

fn scene_in_draft(c: &Connection, id: &str, draft_id: &str) -> AppResult<String> {
    let row: Option<(String, String, bool)> = c
        .query_row(
            "SELECT draft_id, heading, deleted_at IS NOT NULL FROM screenplay_scene WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        Some((d, heading, false)) if d == draft_id => Ok(heading),
        _ => Err(AppError::not_found("scene")),
    }
}

fn element_ids(c: &Connection, scene_id: &str) -> AppResult<Vec<String>> {
    sibling_ids(c, "screenplay_element", "scene_id = ?1", &[text(scene_id)])
}

fn draft_of_scene(c: &Connection, scene_id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT draft_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
        [scene_id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("scene"))
}

fn draft_of_element(c: &Connection, id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT s.draft_id FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id WHERE e.id=?1 AND s.deleted_at IS NULL",
        [id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("screenplay element"))
}

// --------------------------------------------------------------- primitives

fn insert_element_row(
    c: &Connection,
    id: &str,
    scene_id: &str,
    index: Option<usize>,
    t: ElementType,
    value: &str,
) -> AppResult<()> {
    let siblings = element_ids(c, scene_id)?;
    let now = now_ms();
    c.execute(
        "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, scene_id, siblings.len() as i64 + 1, t.as_str(), value, now],
    )?;
    if index.map(|i| i < siblings.len()).unwrap_or(false) {
        place_at(c, "screenplay_element", siblings, id, index)?;
    }
    Ok(())
}

fn move_element_row(
    c: &Connection,
    id: &str,
    from_scene: &str,
    to_scene: &str,
    index: Option<usize>,
) -> AppResult<()> {
    if from_scene != to_scene {
        update_fields(
            c,
            "screenplay_element",
            id,
            &[("scene_id", text(to_scene))],
            &["scene_id"],
            None,
            "screenplay element",
        )?;
    }
    let siblings = element_ids(c, to_scene)?;
    place_at(c, "screenplay_element", siblings, id, index)
}

fn insert_scene_row(
    c: &Connection,
    id: &str,
    draft_id: &str,
    index: Option<usize>,
    heading: &str,
) -> AppResult<()> {
    let siblings = live_scene_ids(c, draft_id)?;
    let now = now_ms();
    c.execute(
        "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, draft_id, new_id(), siblings.len() as i64 + 1, heading, now],
    )?;
    if index.map(|i| i < siblings.len()).unwrap_or(false) {
        place_at(c, "screenplay_scene", siblings, id, index)?;
    }
    Ok(())
}

pub(crate) fn soft_delete_scene(tx: &Tx<'_>, draft_id: &str, scene_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let live = live_scene_ids(c, draft_id)?;
    if live.len() <= 1 {
        return Err(AppError::validation(
            "scene",
            "A screenplay needs at least one scene.",
        ));
    }
    let index = live
        .iter()
        .position(|s| s == scene_id)
        .ok_or_else(|| AppError::not_found("scene"))?;
    let heading: String = c.query_row(
        "SELECT heading FROM screenplay_scene WHERE id=?1",
        [scene_id],
        |r| r.get(0),
    )?;
    let label = if heading.trim().is_empty() {
        "Untitled scene".to_string()
    } else {
        heading.trim().to_string()
    };
    soft_delete(
        tx,
        DeleteSpec {
            object_type: "screenplay_scene",
            table: "screenplay_scene",
            id: scene_id,
            title: Some(format!("Scene {} — {label}", index + 1)),
            parent_type: Some("screenplay_draft"),
            parent_id: Some(draft_id.to_string()),
            position: Some(index as i64),
        },
    )
}

// ---------------------------------------------------------------- applying

/// Apply one op. `tx` is inside a mutation on an editable draft.
pub(crate) fn apply_op(
    tx: &Tx<'_>,
    draft_id: &str,
    op: ScreenplayEditOp,
    t: &mut Touched,
) -> AppResult<()> {
    let c = tx.conn();
    match op {
        ScreenplayEditOp::InsertElement {
            id,
            scene_id,
            index,
            element_type,
            text: value,
        } => {
            validate_element(element_type, &value)?;
            scene_in_draft(c, &scene_id, draft_id)?;
            let id = fresh_id(c, "screenplay_element", id, "screenplay element")?;
            insert_element_row(
                c,
                &id,
                &scene_id,
                index.map(|i| i as usize),
                element_type,
                &value,
            )?;
            t.scenes.insert(scene_id);
            t.elements.insert(id.clone());
            t.created.push(id);
        }
        ScreenplayEditOp::UpdateElement {
            id,
            element_type,
            text: value,
        } => {
            let info = element_in_draft(c, &id, draft_id)?;
            let new_type = element_type.unwrap_or(info.element_type);
            let new_text = value.unwrap_or_else(|| info.text.clone());
            validate_element(new_type, &new_text)?;
            let mut fields = Vec::new();
            if new_type != info.element_type {
                fields.push(("element_type", text(new_type.as_str())));
            }
            if new_text != info.text {
                fields.push(("text", text(new_text)));
            }
            if !fields.is_empty() {
                update_fields(
                    c,
                    "screenplay_element",
                    &id,
                    &fields,
                    &["element_type", "text"],
                    None,
                    "screenplay element",
                )?;
                t.scenes.insert(info.scene_id);
                t.elements.insert(id);
            }
        }
        ScreenplayEditOp::DeleteElement { id } => {
            let info = element_in_draft(c, &id, draft_id)?;
            c.execute("DELETE FROM screenplay_element WHERE id=?1", [&id])?;
            t.scenes.insert(info.scene_id);
            t.elements.insert(id);
        }
        ScreenplayEditOp::MoveElement {
            id,
            scene_id,
            index,
        } => {
            let info = element_in_draft(c, &id, draft_id)?;
            scene_in_draft(c, &scene_id, draft_id)?;
            move_element_row(c, &id, &info.scene_id, &scene_id, index.map(|i| i as usize))?;
            t.scenes.insert(info.scene_id);
            t.scenes.insert(scene_id);
            t.elements.insert(id);
        }
        ScreenplayEditOp::InsertScene { id, index, heading } => {
            validate_heading(&heading)?;
            let id = fresh_id(c, "screenplay_scene", id, "scene")?;
            insert_scene_row(c, &id, draft_id, index.map(|i| i as usize), &heading)?;
            t.structure = true;
            t.scenes.insert(id.clone());
            t.created.push(id);
        }
        ScreenplayEditOp::UpdateScene { id, heading } => {
            validate_heading(&heading)?;
            let old = scene_in_draft(c, &id, draft_id)?;
            if old != heading {
                update_fields(
                    c,
                    "screenplay_scene",
                    &id,
                    &[("heading", text(heading))],
                    &["heading"],
                    None,
                    "scene",
                )?;
                t.scenes.insert(id);
            }
        }
        ScreenplayEditOp::MoveScene { id, index } => {
            scene_in_draft(c, &id, draft_id)?;
            let siblings = live_scene_ids(c, draft_id)?;
            place_at(c, "screenplay_scene", siblings, &id, Some(index as usize))?;
            t.structure = true;
            t.scenes.insert(id);
        }
        ScreenplayEditOp::DeleteScene { id } => {
            scene_in_draft(c, &id, draft_id)?;
            soft_delete_scene(tx, draft_id, &id)?;
            t.structure = true;
            t.elements.extend(element_ids(c, &id)?);
            t.scenes.insert(id);
        }
        ScreenplayEditOp::SplitScene {
            element_id,
            new_scene_id,
        } => {
            let info = element_in_draft(c, &element_id, draft_id)?;
            let heading: String = info.text.replace(['\r', '\n'], " ");
            validate_heading(&heading)?;
            let scenes = live_scene_ids(c, draft_id)?;
            let at = scenes
                .iter()
                .position(|s| *s == info.scene_id)
                .ok_or_else(|| AppError::not_found("scene"))?;
            let new_id = fresh_id(c, "screenplay_scene", new_scene_id, "scene")?;
            insert_scene_row(c, &new_id, draft_id, Some(at + 1), &heading)?;
            let els = element_ids(c, &info.scene_id)?;
            let idx = els
                .iter()
                .position(|e| *e == element_id)
                .unwrap_or(els.len());
            for e in &els[idx + 1..] {
                move_element_row(c, e, &info.scene_id, &new_id, None)?;
                t.elements.insert(e.clone());
            }
            c.execute("DELETE FROM screenplay_element WHERE id=?1", [&element_id])?;
            t.elements.insert(element_id);
            t.scenes.insert(info.scene_id);
            t.scenes.insert(new_id.clone());
            t.structure = true;
            t.created.push(new_id);
        }
        ScreenplayEditOp::MergeScene { scene_id } => {
            let heading = scene_in_draft(c, &scene_id, draft_id)?;
            let scenes = live_scene_ids(c, draft_id)?;
            let at = scenes
                .iter()
                .position(|s| *s == scene_id)
                .ok_or_else(|| AppError::not_found("scene"))?;
            if at == 0 {
                return Err(AppError::validation(
                    "scene",
                    "The first scene has no previous scene to merge into.",
                ));
            }
            let prev = scenes[at - 1].clone();
            if !heading.trim().is_empty() {
                let id = new_id();
                insert_element_row(c, &id, &prev, None, ElementType::Action, &heading)?;
                t.elements.insert(id.clone());
                t.created.push(id);
            }
            for e in element_ids(c, &scene_id)? {
                move_element_row(c, &e, &scene_id, &prev, None)?;
                t.elements.insert(e);
            }
            soft_delete_scene(tx, draft_id, &scene_id)?;
            t.scenes.insert(prev);
            t.scenes.insert(scene_id);
            t.structure = true;
        }
    }
    Ok(())
}

/// Bookkeeping after content ops: edit sequence, comment anchors, search.
pub(crate) fn finish(tx: &Tx<'_>, draft_id: &str, t: &Touched) -> AppResult<i64> {
    let c = tx.conn();
    let seq = bump_seq(c, draft_id)?;
    comments::reanchor(tx, &t.elements, &t.scenes)?;
    if t.structure {
        // Display numbers shift: every scene's search title changes.
        for s in live_scene_ids(c, draft_id)? {
            tx.reindex("screenplay_scene", &s);
        }
    }
    for s in &t.scenes {
        tx.reindex("screenplay_scene", s);
    }
    Ok(seq)
}

/// Group consecutive typing into one undo step: batches that only create or
/// change a single element (or a single heading) share a coalesce key.
fn coalesce_key(ops: &[ScreenplayEditOp]) -> Option<String> {
    let mut element: Option<&str> = None;
    let mut heading: Option<&str> = None;
    for op in ops {
        match op {
            ScreenplayEditOp::InsertElement { id: Some(id), .. }
            | ScreenplayEditOp::UpdateElement { id, .. } => {
                if heading.is_some() || element.map(|e| e != id.as_str()).unwrap_or(false) {
                    return None;
                }
                element = Some(id.as_str());
            }
            ScreenplayEditOp::UpdateScene { id, .. } => {
                if element.is_some() || heading.map(|h| h != id.as_str()).unwrap_or(false) {
                    return None;
                }
                heading = Some(id.as_str());
            }
            _ => return None,
        }
    }
    element
        .map(|e| format!("screenplay.type:{e}"))
        .or_else(|| heading.map(|h| format!("screenplay.heading:{h}")))
}

fn summary_for(c: &Connection, ops: &[ScreenplayEditOp]) -> String {
    let scene = match ops.first() {
        Some(ScreenplayEditOp::UpdateElement { id, .. })
        | Some(ScreenplayEditOp::InsertElement { id: Some(id), .. }) => c
            .query_row(
                "SELECT scene_id FROM screenplay_element WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten(),
        Some(ScreenplayEditOp::InsertElement { scene_id, .. })
        | Some(ScreenplayEditOp::UpdateScene { id: scene_id, .. }) => Some(scene_id.clone()),
        _ => None,
    };
    let n = scene.and_then(|s| scene_number(c, &s).ok().flatten());
    match (ops.len(), n) {
        (_, Some(n)) if coalesce_key(ops).is_some() => format!("Typing in Scene {n}"),
        _ => "Edited the screenplay".to_string(),
    }
}

/// Run content ops on a draft as one mutation, then maybe record an automatic history point.
pub(crate) fn run_ops(
    core: &AppCore,
    actor: &Actor,
    draft_id: &str,
    action: &'static str,
    summary: Option<String>,
    ops: Vec<ScreenplayEditOp>,
    coalesce: Option<String>,
) -> AppResult<ScreenplayEditResult> {
    if ops.len() > MAX_OPS {
        return Err(AppError::invalid_input(
            "Too many changes at once. Try a smaller edit.",
        ));
    }
    let s = core.project()?;
    let summary = match summary {
        Some(s) => s,
        None => s.store.read(|c| Ok(summary_for(c, &ops)))?,
    };
    let mut meta =
        MutationMeta::new(action, summary, Capability::Edit).target("screenplay_draft", draft_id);
    if let Some(k) = coalesce {
        meta = meta.coalesce(k);
    }
    let result = s.store.mutate(actor, meta, |tx| {
        editable_draft(tx.conn(), draft_id)?;
        let mut touched = Touched::default();
        for op in ops {
            apply_op(tx, draft_id, op, &mut touched)?;
        }
        let seq = finish(tx, draft_id, &touched)?;
        Ok(ScreenplayEditResult {
            seq,
            created_ids: touched.created,
        })
    })?;
    drafts::maybe_record_history(core, actor, draft_id);
    Ok(result)
}

fn apply_edits(
    core: &AppCore,
    actor: &Actor,
    args: ScreenplayApplyEditsArgs,
) -> AppResult<ScreenplayEditResult> {
    if args.ops.is_empty() {
        let s = core.project()?;
        let seq = s.store.read(|c| {
            super::load_draft(c, &args.draft_id)?;
            super::current_seq(c, &args.draft_id)
        })?;
        return Ok(ScreenplayEditResult {
            seq,
            created_ids: vec![],
        });
    }
    let key = coalesce_key(&args.ops);
    run_ops(
        core,
        actor,
        &args.draft_id,
        "screenplay.edit",
        None,
        args.ops,
        key,
    )
}

// ------------------------------------------------------- single-op commands

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayInsertElementArgs {
    pub scene_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<u32>,
    pub element_type: ElementType,
    #[serde(default)]
    pub text: String,
}

fn insert_element_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayInsertElementArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core
        .project()?
        .store
        .read(|c| draft_of_scene(c, &a.scene_id))?;
    let op = ScreenplayEditOp::InsertElement {
        id: None,
        scene_id: a.scene_id,
        index: a.index,
        element_type: a.element_type,
        text: a.text,
    };
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.edit",
        Some("Added a screenplay element".into()),
        vec![op],
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayUpdateElementArgs {
    pub id: String,
    #[serde(default)]
    #[ts(optional)]
    pub element_type: Option<ElementType>,
    #[serde(default)]
    #[ts(optional)]
    pub text: Option<String>,
}

fn update_element_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayUpdateElementArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core.project()?.store.read(|c| draft_of_element(c, &a.id))?;
    let ops = vec![ScreenplayEditOp::UpdateElement {
        id: a.id,
        element_type: a.element_type,
        text: a.text,
    }];
    let key = coalesce_key(&ops);
    run_ops(core, actor, &draft, "screenplay.edit", None, ops, key)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayElementIdArgs {
    pub id: String,
}

fn delete_element_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayElementIdArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core.project()?.store.read(|c| draft_of_element(c, &a.id))?;
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.edit",
        Some("Deleted a screenplay element".into()),
        vec![ScreenplayEditOp::DeleteElement { id: a.id }],
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayMoveElementArgs {
    pub id: String,
    pub scene_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<u32>,
}

fn move_element_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayMoveElementArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core.project()?.store.read(|c| draft_of_element(c, &a.id))?;
    let op = ScreenplayEditOp::MoveElement {
        id: a.id,
        scene_id: a.scene_id,
        index: a.index,
    };
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.edit",
        Some("Moved a screenplay element".into()),
        vec![op],
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayCreateSceneArgs {
    pub draft_id: String,
    /// Position among scenes (0-based); appended when omitted.
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<u32>,
    #[serde(default)]
    pub heading: String,
}

/// New scene from the navigator (FSD §16.3): heading typed by the writer, an
/// empty action line ready for writing, number derived from position.
fn create_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayCreateSceneArgs,
) -> AppResult<ScreenplayEditResult> {
    let scene_id = new_id();
    let ops = vec![
        ScreenplayEditOp::InsertScene {
            id: Some(scene_id.clone()),
            index: a.index,
            heading: a.heading,
        },
        ScreenplayEditOp::InsertElement {
            id: None,
            scene_id,
            index: None,
            element_type: ElementType::Action,
            text: String::new(),
        },
    ];
    run_ops(
        core,
        actor,
        &a.draft_id,
        "screenplay.create_scene",
        Some("Added a scene".into()),
        ops,
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayUpdateSceneArgs {
    pub scene_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub heading: Option<String>,
    /// Non-printing scene note (FSD §16.6). Empty clears it.
    #[serde(default)]
    #[ts(optional)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub synopsis: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub story_day: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub time_note: Option<String>,
}

fn update_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayUpdateSceneArgs,
) -> AppResult<ScreenplayEditResult> {
    let s = core.project()?;
    let draft = s.store.read(|c| draft_of_scene(c, &a.scene_id))?;
    if let Some(h) = &a.heading {
        validate_heading(h)?;
    }
    let clean = |v: Option<String>, max: usize, what: &str| -> AppResult<Option<Option<String>>> {
        match v {
            None => Ok(None),
            Some(v) if v.len() > max => {
                Err(AppError::invalid_input(format!("{what} is too long.")))
            }
            Some(v) => Ok(Some(Some(v).filter(|x| !x.trim().is_empty()))),
        }
    };
    let notes = clean(a.notes, MAX_NOTE_BYTES, "The scene note")?;
    let synopsis = clean(a.synopsis, MAX_NOTE_BYTES, "The synopsis")?;
    let story_day = clean(a.story_day, 200, "The story day")?;
    let time_note = clean(a.time_note, 200, "The time note")?;
    let only_notes = a.heading.is_none();
    let summary = if only_notes {
        "Edited scene notes"
    } else {
        "Edited a scene heading"
    };
    let key = format!("screenplay.scene_meta:{}", a.scene_id);
    let result = s.store.mutate(
        actor,
        MutationMeta::new("screenplay.update_scene", summary, Capability::Edit)
            .target("screenplay_scene", &a.scene_id)
            .coalesce(key),
        |tx| {
            let c = tx.conn();
            editable_draft(c, &draft)?;
            let mut fields: Vec<(&str, rusqlite::types::Value)> = Vec::new();
            if let Some(h) = a.heading.clone() {
                fields.push(("heading", text(h)));
            }
            for (col, v) in [
                ("notes", &notes),
                ("synopsis", &synopsis),
                ("story_day", &story_day),
                ("time_note", &time_note),
            ] {
                if let Some(v) = v {
                    fields.push((col, openframe_persistence::rows::opt_text(v.clone())));
                }
            }
            update_fields(
                c,
                "screenplay_scene",
                &a.scene_id,
                &fields,
                &["heading", "notes", "synopsis", "story_day", "time_note"],
                None,
                "scene",
            )?;
            let mut t = Touched::default();
            t.scenes.insert(a.scene_id.clone());
            let seq = finish(tx, &draft, &t)?;
            Ok(ScreenplayEditResult {
                seq,
                created_ids: vec![],
            })
        },
    )?;
    Ok(result)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayMoveSceneArgs {
    pub scene_id: String,
    pub index: u32,
}

fn move_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayMoveSceneArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core
        .project()?
        .store
        .read(|c| draft_of_scene(c, &a.scene_id))?;
    let op = ScreenplayEditOp::MoveScene {
        id: a.scene_id,
        index: a.index,
    };
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.move_scene",
        Some("Moved a scene".into()),
        vec![op],
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySceneIdArgs {
    pub scene_id: String,
}

fn delete_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySceneIdArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core
        .project()?
        .store
        .read(|c| draft_of_scene(c, &a.scene_id))?;
    let label = core.project()?.store.read(|c| {
        Ok(c.query_row(
            "SELECT heading FROM screenplay_scene WHERE id=?1",
            [&a.scene_id],
            |r| r.get::<_, String>(0),
        )?)
    })?;
    let summary = format!(
        "Deleted scene “{}”",
        if label.trim().is_empty() {
            "Untitled scene"
        } else {
            label.trim()
        }
    );
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.delete_scene",
        Some(summary),
        vec![ScreenplayEditOp::DeleteScene { id: a.scene_id }],
        None,
    )
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySplitSceneArgs {
    pub element_id: String,
}

fn split_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySplitSceneArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core
        .project()?
        .store
        .read(|c| draft_of_element(c, &a.element_id))?;
    let op = ScreenplayEditOp::SplitScene {
        element_id: a.element_id,
        new_scene_id: None,
    };
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.split_scene",
        Some("Split a scene".into()),
        vec![op],
        None,
    )
}

fn merge_scene_cmd(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySceneIdArgs,
) -> AppResult<ScreenplayEditResult> {
    let draft = core
        .project()?
        .store
        .read(|c| draft_of_scene(c, &a.scene_id))?;
    let op = ScreenplayEditOp::MergeScene {
        scene_id: a.scene_id,
    };
    run_ops(
        core,
        actor,
        &draft,
        "screenplay.merge_scene",
        Some("Merged scenes".into()),
        vec![op],
        None,
    )
}

// ----------------------------------------------------------- find/replace

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayReplaceAllArgs {
    pub draft_id: String,
    pub query: String,
    pub replacement: String,
    #[serde(default)]
    pub match_case: bool,
    #[serde(default)]
    pub whole_word: bool,
    /// Include non-printing note elements (when notes are shown in the editor).
    #[serde(default)]
    pub include_notes: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayReplaceResult {
    pub count: u32,
    #[ts(type = "number")]
    pub seq: i64,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte ranges of matches (non-overlapping, left to right) with the same
/// semantics as the editor's find: literal text, optional case-insensitivity,
/// optional whole-word boundaries (letters, digits and `_` are word characters).
pub fn find_matches(
    hay: &str,
    query: &str,
    match_case: bool,
    whole_word: bool,
) -> AppResult<Vec<(usize, usize)>> {
    if query.is_empty() {
        return Ok(vec![]);
    }
    let re = RegexBuilder::new(&regex::escape(query))
        .case_insensitive(!match_case)
        .build()
        .map_err(|e| {
            AppError::invalid_input("That search can't be used.").with_detail(e.to_string())
        })?;
    Ok(re
        .find_iter(hay)
        .filter(|m| {
            !whole_word
                || (!hay[..m.start()]
                    .chars()
                    .next_back()
                    .map(is_word_char)
                    .unwrap_or(false)
                    && !hay[m.end()..]
                        .chars()
                        .next()
                        .map(is_word_char)
                        .unwrap_or(false))
        })
        .map(|m| (m.start(), m.end()))
        .collect())
}

pub fn replace_matches(hay: &str, ranges: &[(usize, usize)], replacement: &str) -> String {
    let mut out = String::with_capacity(hay.len());
    let mut last = 0;
    for (s, e) in ranges {
        out.push_str(&hay[last..*s]);
        out.push_str(replacement);
        last = *e;
    }
    out.push_str(&hay[last..]);
    out
}

/// Replace every match in the draft as ONE undoable change (UX §3.13: atomic).
fn replace_all(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayReplaceAllArgs,
) -> AppResult<ScreenplayReplaceResult> {
    if a.query.is_empty() {
        return Err(AppError::required("Find text"));
    }
    if a.query.chars().count() > 500 || a.replacement.chars().count() > 2_000 {
        return Err(AppError::invalid_input("That text is too long."));
    }
    let s = core.project()?;
    let summary = format!("Replaced “{}” with “{}”", a.query, a.replacement);
    let result = s.store.mutate(
        actor,
        MutationMeta::new("screenplay.replace_all", summary, Capability::Edit)
            .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            editable_draft(c, &a.draft_id)?;
            let mut t = Touched::default();
            let mut count = 0u32;
            for scene in super::load_scenes(c, &a.draft_id)? {
                let hits = find_matches(&scene.heading, &a.query, a.match_case, a.whole_word)?;
                if !hits.is_empty() {
                    let new_heading = replace_matches(&scene.heading, &hits, &a.replacement);
                    validate_heading(&new_heading)?;
                    count += hits.len() as u32;
                    update_fields(
                        c,
                        "screenplay_scene",
                        &scene.id,
                        &[("heading", text(new_heading))],
                        &["heading"],
                        None,
                        "scene",
                    )?;
                    t.scenes.insert(scene.id.clone());
                }
                for el in &scene.elements {
                    if el.element_type == ElementType::Note && !a.include_notes {
                        continue;
                    }
                    let hits = find_matches(&el.text, &a.query, a.match_case, a.whole_word)?;
                    if hits.is_empty() {
                        continue;
                    }
                    let new_text = replace_matches(&el.text, &hits, &a.replacement);
                    validate_element(el.element_type, &new_text)?;
                    count += hits.len() as u32;
                    update_fields(
                        c,
                        "screenplay_element",
                        &el.id,
                        &[("text", text(new_text))],
                        &["text"],
                        None,
                        "screenplay element",
                    )?;
                    t.scenes.insert(scene.id.clone());
                    t.elements.insert(el.id.clone());
                }
            }
            let seq = if count > 0 {
                finish(tx, &a.draft_id, &t)?
            } else {
                super::current_seq(c, &a.draft_id)?
            };
            Ok(ScreenplayReplaceResult { count, seq })
        },
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_respects_case_and_whole_words() {
        let t = "The folder. FOLDERS of folder_x and Folder!";
        assert_eq!(find_matches(t, "folder", true, false).unwrap().len(), 2);
        assert_eq!(find_matches(t, "folder", false, false).unwrap().len(), 4);
        let whole = find_matches(t, "folder", false, true).unwrap();
        assert_eq!(whole.len(), 2, "FOLDERS and folder_x are not whole words");
        assert_eq!(
            replace_matches(t, &whole, "file"),
            "The file. FOLDERS of folder_x and file!"
        );
        assert!(
            find_matches(t, "(", false, false).unwrap().is_empty(),
            "regex syntax is literal"
        );
    }

    #[test]
    fn element_validation() {
        assert!(validate_element(ElementType::SceneHeading, "INT. X").is_err());
        assert!(validate_element(ElementType::Character, "A\nB").is_err());
        assert!(validate_element(ElementType::Dialogue, "Line one\nLine two").is_ok());
        assert!(validate_heading("INT. HOUSE — DAY").is_ok());
        assert!(validate_heading("INT.\nHOUSE").is_err());
    }

    #[test]
    fn typing_batches_coalesce_per_element() {
        let up = |id: &str| ScreenplayEditOp::UpdateElement {
            id: id.into(),
            element_type: None,
            text: Some("x".into()),
        };
        assert_eq!(
            coalesce_key(&[up("a")]).as_deref(),
            Some("screenplay.type:a")
        );
        assert_eq!(
            coalesce_key(&[up("a"), up("a")]).as_deref(),
            Some("screenplay.type:a")
        );
        assert_eq!(coalesce_key(&[up("a"), up("b")]), None);
        assert_eq!(
            coalesce_key(&[ScreenplayEditOp::DeleteElement { id: "a".into() }]),
            None
        );
    }
}
