//! Exchange packages — export side (FSD §47.2–47.4, §50; Import/Export §9–§11;
//! Security §22 "redaction" rules).
//!
//! An exchange package is an explicit subset: the selected workspace content,
//! its source version identity, the context needed to understand it, comments
//! and attachments only when selected, and the package metadata. Private notes
//! are never read here, so they can't leak into a package by accident.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use openframe_domain::{
    Actor, AppError, AppResult, Capability, PACKAGE_FORMAT_VERSION, new_id, now_ms,
};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::format::{
    self, EntryData, PackageManifest, PackageScope, PackageSourceDraft, PackageSourceVersion,
    PackageType, PackageUser,
};
use super::{PackageCount, log_package};
use crate::core::AppCore;

// ============================================================ content model
// Shared by export, import (sessions) and the reviewer workspace. All fields are
// tolerant on read (`default`) so older packages of the same format version load.

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExComment {
    pub id: String,
    pub parent_id: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub scene_id: Option<String>,
    pub scene_lineage_id: Option<String>,
    pub quoted_text: Option<String>,
    pub body: String,
    pub status: String,
    pub author_user_id: String,
    pub author_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExDraft {
    pub id: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExElement {
    pub id: String,
    pub element_type: String,
    pub text: String,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExScene {
    pub id: String,
    pub lineage_id: String,
    pub number: u32,
    pub heading: String,
    pub synopsis: Option<String>,
    pub rev: i64,
    pub elements: Vec<ExElement>,
}

impl ExScene {
    pub fn full_text(&self) -> String {
        let mut t = self.heading.clone();
        for e in &self.elements {
            t.push('\n');
            t.push_str(&e.text);
        }
        t
    }
}

/// Screenplay review package and response package content.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScriptContent {
    pub draft: ExDraft,
    pub scenes: Vec<ExScene>,
    pub comments: Vec<ExComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExAct {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    pub position: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExSequence {
    pub id: String,
    pub act_id: Option<String>,
    pub title: String,
    pub note: Option<String>,
    pub position: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExBeat {
    pub id: String,
    pub parent_type: String,
    pub parent_id: Option<String>,
    pub text: String,
    pub note: Option<String>,
    pub color: Option<String>,
    pub position: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExCard {
    pub id: String,
    pub parent_type: String,
    pub parent_id: Option<String>,
    pub short_description: String,
    pub scene_heading: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    pub position: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExAttachment {
    pub id: String,
    pub owner_type: String,
    pub owner_id: String,
    pub file_name: String,
    /// Entry inside the package.
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StoryContent {
    pub acts: Vec<ExAct>,
    pub sequences: Vec<ExSequence>,
    pub beats: Vec<ExBeat>,
    pub cards: Vec<ExCard>,
    pub attachments: Vec<ExAttachment>,
    pub comments: Vec<ExComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExSceneRef {
    pub id: String,
    pub lineage_id: String,
    pub number: u32,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExBreakdownElement {
    pub id: String,
    pub scene_id: String,
    pub scene_lineage_id: String,
    pub category: String,
    pub display_name: String,
    pub notes: Option<String>,
    pub catalog_item_id: Option<String>,
    pub catalog_name: Option<String>,
    pub confirmation_state: String,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BreakdownContent {
    pub source_draft_id: String,
    pub source_draft_name: String,
    pub scenes: Vec<ExSceneRef>,
    pub elements: Vec<ExBreakdownElement>,
    pub comments: Vec<ExComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExShot {
    pub id: String,
    pub scene_id: String,
    pub scene_lineage_id: String,
    pub position: i64,
    pub description: String,
    pub size: Option<String>,
    pub movement: Option<String>,
    pub angle: Option<String>,
    pub lens: Option<String>,
    pub camera_notes: Option<String>,
    pub characters: Vec<String>,
    pub sound_note: Option<String>,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ShotsContent {
    pub scenes: Vec<ExSceneRef>,
    pub shots: Vec<ExShot>,
    pub comments: Vec<ExComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExStrip {
    pub id: String,
    pub scene_id: String,
    pub scene_lineage_id: String,
    pub heading: String,
    pub position: i64,
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExMarker {
    pub id: String,
    pub marker_type: String,
    pub label: String,
    pub at_time: Option<String>,
    pub duration_minutes: Option<i64>,
    pub notes: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExDay {
    pub id: String,
    pub position: i64,
    pub date: Option<String>,
    pub notes: Option<String>,
    pub is_off_day: bool,
    pub planned_minutes: Option<i64>,
    pub rev: i64,
    pub strips: Vec<ExStrip>,
    pub markers: Vec<ExMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScheduleContent {
    pub schedule_id: String,
    pub schedule_name: String,
    pub days: Vec<ExDay>,
    pub comments: Vec<ExComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CallReviewContent {
    pub call_sheet_id: String,
    pub title: String,
    pub revision: i64,
    pub status: String,
    pub shoot_day_id: String,
    pub schedule_id: String,
    pub document: Value,
    pub rev: i64,
    pub comments: Vec<ExComment>,
}

pub fn parse_content<T: for<'de> Deserialize<'de>>(v: &Value) -> AppResult<T> {
    serde_json::from_value(v.clone()).map_err(|e| format::incomplete(format!("content: {e}")))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn count(label: &str, n: usize) -> PackageCount {
    PackageCount {
        label: label.to_string(),
        count: n as u32,
    }
}

/// Human counts of what an exchange package contains (inspect / preview).
pub fn content_summary(ty: PackageType, v: &Value) -> Vec<PackageCount> {
    match ty {
        PackageType::ScriptReview | PackageType::Response => {
            let c: ScriptContent = parse_content(v).unwrap_or_default();
            vec![
                count("Scenes", c.scenes.len()),
                count("Comments", c.comments.len()),
            ]
        }
        PackageType::Story => {
            let c: StoryContent = parse_content(v).unwrap_or_default();
            vec![
                count("Acts", c.acts.len()),
                count("Sequences", c.sequences.len()),
                count("Beats", c.beats.len()),
                count("Scene Cards", c.cards.len()),
                count("Comments", c.comments.len()),
                count("Attachments", c.attachments.len()),
            ]
        }
        PackageType::Breakdown => {
            let c: BreakdownContent = parse_content(v).unwrap_or_default();
            vec![
                count("Scenes", c.scenes.len()),
                count("Breakdown elements", c.elements.len()),
                count("Comments", c.comments.len()),
            ]
        }
        PackageType::Shots => {
            let c: ShotsContent = parse_content(v).unwrap_or_default();
            vec![
                count("Scenes", c.scenes.len()),
                count("Shots", c.shots.len()),
                count("Comments", c.comments.len()),
            ]
        }
        PackageType::Schedule => {
            let c: ScheduleContent = parse_content(v).unwrap_or_default();
            vec![
                count("Shooting days", c.days.len()),
                count(
                    "Scheduled scenes",
                    c.days.iter().map(|d| d.strips.len()).sum(),
                ),
                count(
                    "Day notes",
                    c.days.iter().filter(|d| d.notes.is_some()).count(),
                ),
                count("Comments", c.comments.len()),
            ]
        }
        PackageType::CallReview => {
            let c: CallReviewContent = parse_content(v).unwrap_or_default();
            vec![count("Call sheet", 1), count("Comments", c.comments.len())]
        }
        PackageType::Backup | PackageType::Project => vec![],
    }
}

// ================================================================ args / DTOs

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageExchangeArgs {
    pub package_type: PackageType,
    /// Screenplay review: the source draft (default: current draft).
    #[serde(default)]
    pub draft_id: Option<String>,
    /// Script review: "full" | "comments" (Script + comments) | "scenes";
    /// Story Board: "full" | "act"; others: "full" | "scenes".
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub scene_ids: Vec<String>,
    #[serde(default)]
    pub act_id: Option<String>,
    #[serde(default)]
    pub call_sheet_id: Option<String>,
    #[serde(default)]
    pub include_comments: bool,
    #[serde(default)]
    pub include_attachments: bool,
    /// Always refused: private notes are not allowed in exchange packages.
    #[serde(default)]
    pub include_private_notes: bool,
    /// Destination (export only).
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageDraftOption {
    pub id: String,
    pub name: String,
    pub screenplay_title: String,
    pub status: String,
    pub is_current: bool,
    pub scene_count: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSceneOption {
    pub id: String,
    pub number: u32,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageExchangeSources {
    pub drafts: Vec<PackageDraftOption>,
    pub acts: Vec<PackageOption>,
    pub production_source: Option<PackageOption>,
    pub schedule: Option<PackageOption>,
    pub call_sheets: Vec<PackageOption>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageExportSummary {
    pub package_type: PackageType,
    pub type_label: String,
    pub project_title: String,
    /// What the package is built from, e.g. "Draft 7 — Director Rewrite".
    pub target: String,
    pub scope_label: String,
    pub exported_by: String,
    #[ts(type = "number")]
    pub date: i64,
    pub counts: Vec<PackageCount>,
    pub comments_included: bool,
    pub attachments_included: bool,
    /// Attachments that could be included for this scope.
    pub attachments_available: u32,
    /// Always true: "Private notes: excluded".
    pub private_notes_excluded: bool,
    pub suggested_file_name: String,
    /// What the receiver can do with it (FSD §47.4: explain before export).
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageExportResult {
    pub path: String,
    pub file_name: String,
    pub package_id: String,
    pub summary: PackageExportSummary,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageScenesArgs {
    #[serde(default)]
    pub draft_id: Option<String>,
}

// ================================================================= loading

pub(crate) struct DraftInfo {
    pub id: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub name: String,
    pub status: String,
    pub rev: i64,
}

pub(crate) fn load_draft_info(c: &Connection, id: &str) -> AppResult<Option<DraftInfo>> {
    Ok(c.query_row(
        "SELECT d.id, d.screenplay_id, s.title, d.name, d.status, d.rev
         FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id
         WHERE d.id = ?1 AND d.deleted_at IS NULL",
        [id],
        |r| {
            Ok(DraftInfo {
                id: r.get(0)?,
                screenplay_id: r.get(1)?,
                screenplay_title: r.get(2)?,
                name: r.get(3)?,
                status: r.get(4)?,
                rev: r.get(5)?,
            })
        },
    )
    .optional()?)
}

/// Current draft of a screenplay (or of the first screenplay when None).
pub(crate) fn current_draft(
    c: &Connection,
    screenplay_id: Option<&str>,
) -> AppResult<Option<DraftInfo>> {
    let id: Option<String> = match screenplay_id {
        Some(sid) => c
            .query_row(
                "SELECT current_draft_id FROM screenplay WHERE id=?1 AND deleted_at IS NULL",
                [sid],
                |r| r.get(0),
            )
            .optional()?
            .flatten(),
        None => c
            .query_row(
                "SELECT current_draft_id FROM screenplay WHERE deleted_at IS NULL AND current_draft_id IS NOT NULL
                 ORDER BY created_at, id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?,
    };
    match id {
        Some(id) => load_draft_info(c, &id),
        None => Ok(None),
    }
}

/// Live scenes of a draft in order, with elements, numbered by order.
pub(crate) fn load_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<ExScene>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, synopsis, rev FROM screenplay_scene
         WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let mut scenes: Vec<ExScene> = stmt
        .query_map([draft_id], |r| {
            Ok(ExScene {
                id: r.get(0)?,
                lineage_id: r.get(1)?,
                number: 0,
                heading: r.get(2)?,
                synopsis: r.get(3)?,
                rev: r.get(4)?,
                elements: vec![],
            })
        })?
        .collect::<Result<_, _>>()?;
    let index: HashMap<String, usize> = scenes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    let mut stmt = c.prepare(
        "SELECT e.id, e.scene_id, e.element_type, e.text, e.rev FROM screenplay_element e
         JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL ORDER BY e.scene_id, e.position, e.id",
    )?;
    let rows = stmt
        .query_map([draft_id], |r| {
            Ok((
                r.get::<_, String>(1)?,
                ExElement {
                    id: r.get(0)?,
                    element_type: r.get(2)?,
                    text: r.get(3)?,
                    rev: r.get(4)?,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (scene_id, el) in rows {
        if let Some(i) = index.get(&scene_id) {
            scenes[*i].elements.push(el);
        }
    }
    for (i, s) in scenes.iter_mut().enumerate() {
        s.number = i as u32 + 1;
    }
    Ok(scenes)
}

struct CommentRow {
    c: ExComment,
}

/// Comments (and their replies) on the given targets. Never private notes.
fn load_comments(
    c: &Connection,
    targets: &HashSet<(String, String)>,
    scene_of: &HashMap<String, String>,
    lineage_of: &HashMap<String, String>,
) -> AppResult<Vec<ExComment>> {
    let mut stmt = c.prepare(
        "SELECT id, parent_id, target_type, target_id, scene_id, quoted_text, body, status, author_user_id, author_name, created_at
         FROM comment WHERE deleted_at IS NULL ORDER BY created_at, id",
    )?;
    let rows: Vec<CommentRow> = stmt
        .query_map([], |r| {
            Ok(CommentRow {
                c: ExComment {
                    id: r.get(0)?,
                    parent_id: r.get(1)?,
                    target_type: r.get(2)?,
                    target_id: r.get(3)?,
                    scene_id: r.get(4)?,
                    scene_lineage_id: None,
                    quoted_text: r.get(5)?,
                    body: r.get(6)?,
                    status: r.get(7)?,
                    author_user_id: r.get(8)?,
                    author_name: r.get(9)?,
                    created_at: r.get(10)?,
                },
            })
        })?
        .collect::<Result<_, _>>()?;
    let roots: HashSet<String> = rows
        .iter()
        .filter(|r| {
            r.c.parent_id.is_none()
                && targets.contains(&(r.c.target_type.clone(), r.c.target_id.clone()))
        })
        .map(|r| r.c.id.clone())
        .collect();
    let mut out = vec![];
    for r in rows {
        let keep = roots.contains(&r.c.id)
            || r.c
                .parent_id
                .as_ref()
                .map(|p| roots.contains(p))
                .unwrap_or(false);
        if !keep {
            continue;
        }
        let mut ex = r.c;
        if let Some(sid) = scene_of.get(&ex.target_id) {
            ex.scene_id = Some(sid.clone());
        } else if ex.target_type == "screenplay_scene" {
            ex.scene_id = Some(ex.target_id.clone());
        }
        ex.scene_lineage_id = ex
            .scene_id
            .as_ref()
            .and_then(|s| lineage_of.get(s).cloned());
        out.push(ex);
    }
    Ok(out)
}

fn scene_refs(c: &Connection, draft_id: &str) -> AppResult<Vec<ExSceneRef>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([draft_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, (id, lineage_id, heading))| ExSceneRef {
            id,
            lineage_id,
            number: i as u32 + 1,
            heading,
        })
        .collect())
}

pub(crate) fn active_source(c: &Connection) -> AppResult<Option<(String, String, String)>> {
    Ok(c.query_row(
        "SELECT p.id, p.draft_id, d.name FROM production_source p JOIN screenplay_draft d ON d.id = p.draft_id
         WHERE p.active = 1 ORDER BY p.selected_at DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?)
}

pub(crate) fn current_schedule(c: &Connection) -> AppResult<Option<(String, String)>> {
    Ok(c.query_row(
        "SELECT id, name FROM shooting_schedule WHERE deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?)
}

// ================================================================= building

pub(crate) struct Built {
    pub content: Value,
    pub attachments: Vec<(String, PathBuf)>,
    pub attachments_available: u32,
    pub counts: Vec<PackageCount>,
    pub target: String,
    pub scope: PackageScope,
    pub source_draft: Option<PackageSourceDraft>,
    pub source_versions: Vec<PackageSourceVersion>,
    pub included_ids: Vec<String>,
    pub base: BTreeMap<String, i64>,
    pub comments_included: bool,
    pub attachments_included: bool,
    pub explanation: String,
}

fn key(table: &str, id: &str) -> String {
    format!("{table}:{id}")
}

fn to_value<T: Serialize>(v: &T) -> AppResult<Value> {
    serde_json::to_value(v).map_err(|e| AppError::internal(e.to_string()))
}

fn scope_scene_filter(all: &[String], wanted: &[String]) -> AppResult<HashSet<String>> {
    if wanted.is_empty() {
        return Err(AppError::validation(
            "sceneIds",
            "Choose at least one scene.",
        ));
    }
    let set: HashSet<String> = all.iter().cloned().collect();
    if wanted.iter().any(|w| !set.contains(w)) {
        return Err(AppError::validation(
            "sceneIds",
            "One of the chosen scenes isn't part of this draft.",
        ));
    }
    Ok(wanted.iter().cloned().collect())
}

fn scene_numbers_label(numbers: &[u32]) -> String {
    let list: Vec<String> = numbers.iter().map(|n| n.to_string()).collect();
    if list.len() == 1 {
        format!("Scene {}", list[0])
    } else {
        format!("Scenes {}", list.join(", "))
    }
}

fn build_script(c: &Connection, a: &PackageExchangeArgs) -> AppResult<Built> {
    let draft = match &a.draft_id {
        Some(id) => load_draft_info(c, id)?.ok_or_else(|| AppError::not_found("draft"))?,
        None => current_draft(c, None)?.ok_or_else(|| {
            AppError::validation(
                "draftId",
                "This project has no screenplay draft to send for review yet.",
            )
        })?,
    };
    let mut scenes = load_scenes(c, &draft.id)?;
    let scope_kind = a.scope.clone().unwrap_or_else(|| "full".into());
    let include_comments = a.include_comments || scope_kind == "comments";
    let scope_label = match scope_kind.as_str() {
        "full" => "Full script".to_string(),
        "comments" => "Script + comments".to_string(),
        "scenes" => {
            let all: Vec<String> = scenes.iter().map(|s| s.id.clone()).collect();
            let keep = scope_scene_filter(&all, &a.scene_ids)?;
            scenes.retain(|s| keep.contains(&s.id));
            scene_numbers_label(&scenes.iter().map(|s| s.number).collect::<Vec<_>>())
        }
        _ => {
            return Err(AppError::validation(
                "scope",
                "Choose Full script, Script + comments or Specific scenes only.",
            ));
        }
    };
    let mut base = BTreeMap::new();
    base.insert(key("screenplay_draft", &draft.id), draft.rev);
    let mut targets = HashSet::new();
    let mut scene_of = HashMap::new();
    let mut lineage_of = HashMap::new();
    targets.insert(("screenplay_draft".to_string(), draft.id.clone()));
    for s in &scenes {
        base.insert(key("screenplay_scene", &s.id), s.rev);
        targets.insert(("screenplay_scene".to_string(), s.id.clone()));
        lineage_of.insert(s.id.clone(), s.lineage_id.clone());
        for e in &s.elements {
            base.insert(key("screenplay_element", &e.id), e.rev);
            targets.insert(("screenplay_element".to_string(), e.id.clone()));
            scene_of.insert(e.id.clone(), s.id.clone());
        }
    }
    let comments = if include_comments {
        load_comments(c, &targets, &scene_of, &lineage_of)?
    } else {
        vec![]
    };
    let counts = vec![
        count("Scenes", scenes.len()),
        count("Comments", comments.len()),
    ];
    let included_ids = std::iter::once(draft.id.clone())
        .chain(scenes.iter().map(|s| s.id.clone()))
        .collect();
    let content = ScriptContent {
        draft: ExDraft {
            id: draft.id.clone(),
            screenplay_id: draft.screenplay_id.clone(),
            screenplay_title: draft.screenplay_title.clone(),
            name: draft.name.clone(),
            status: draft.status.clone(),
        },
        scenes,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: vec![],
        attachments_available: 0,
        counts,
        target: draft.name.clone(),
        scope: PackageScope {
            kind: scope_kind,
            label: scope_label,
            ids: a.scene_ids.clone(),
        },
        source_draft: Some(PackageSourceDraft {
            id: draft.id,
            screenplay_id: draft.screenplay_id,
            name: draft.name,
            status: draft.status,
        }),
        source_versions: vec![],
        included_ids,
        base,
        comments_included: include_comments,
        attachments_included: false,
        explanation: "The reviewer opens a read-only copy of this draft in OpenFrame and adds comments. They send back a Response package; nothing in your project changes until you preview it and choose what to import.".into(),
    })
}

fn build_story(c: &Connection, root: &Path, a: &PackageExchangeArgs) -> AppResult<Built> {
    let mut acts: Vec<ExAct> = {
        let mut stmt = c.prepare(
            "SELECT id, title, note, position, rev FROM story_act WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        stmt.query_map([], |r| {
            Ok(ExAct {
                id: r.get(0)?,
                title: r.get(1)?,
                note: r.get(2)?,
                position: r.get(3)?,
                rev: r.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?
    };
    let scope_kind = a.scope.clone().unwrap_or_else(|| "full".into());
    let scope_label = match scope_kind.as_str() {
        "full" => "Whole Story Board".to_string(),
        "act" => {
            let id = a
                .act_id
                .as_deref()
                .ok_or_else(|| AppError::required("Act"))?;
            acts.retain(|x| x.id == id);
            let act = acts.first().ok_or_else(|| AppError::not_found("act"))?;
            format!("Act “{}”", act.title)
        }
        _ => {
            return Err(AppError::validation(
                "scope",
                "Choose the whole Story Board or one Act.",
            ));
        }
    };
    let act_ids: HashSet<String> = acts.iter().map(|x| x.id.clone()).collect();
    let whole = scope_kind == "full";
    let sequences: Vec<ExSequence> = {
        let mut stmt = c.prepare(
            "SELECT id, act_id, title, note, position, rev FROM story_sequence WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let all: Vec<ExSequence> = stmt
            .query_map([], |r| {
                Ok(ExSequence {
                    id: r.get(0)?,
                    act_id: r.get(1)?,
                    title: r.get(2)?,
                    note: r.get(3)?,
                    position: r.get(4)?,
                    rev: r.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        all.into_iter()
            .filter(|s| match &s.act_id {
                Some(a) => act_ids.contains(a),
                None => whole,
            })
            .collect()
    };
    let seq_ids: HashSet<String> = sequences.iter().map(|x| x.id.clone()).collect();
    let in_scope = |pt: &str, pid: &Option<String>| -> bool {
        match (pt, pid) {
            ("act", Some(id)) => act_ids.contains(id),
            ("sequence", Some(id)) => seq_ids.contains(id),
            ("parking", _) | ("unassigned", _) => whole,
            _ => false,
        }
    };
    let beats: Vec<ExBeat> = {
        let mut stmt = c.prepare(
            "SELECT id, parent_type, parent_id, text, note, color, position, rev FROM story_beat
             WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let all: Vec<ExBeat> = stmt
            .query_map([], |r| {
                Ok(ExBeat {
                    id: r.get(0)?,
                    parent_type: r.get(1)?,
                    parent_id: r.get(2)?,
                    text: r.get(3)?,
                    note: r.get(4)?,
                    color: r.get(5)?,
                    position: r.get(6)?,
                    rev: r.get(7)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        all.into_iter()
            .filter(|b| in_scope(&b.parent_type, &b.parent_id))
            .collect()
    };
    let cards: Vec<ExCard> = {
        let mut stmt = c.prepare(
            "SELECT id, parent_type, parent_id, short_description, scene_heading, notes, color, position, rev
             FROM story_scene_card WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let all: Vec<ExCard> = stmt
            .query_map([], |r| {
                Ok(ExCard {
                    id: r.get(0)?,
                    parent_type: r.get(1)?,
                    parent_id: r.get(2)?,
                    short_description: r.get(3)?,
                    scene_heading: r.get(4)?,
                    notes: r.get(5)?,
                    color: r.get(6)?,
                    position: r.get(7)?,
                    rev: r.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        all.into_iter()
            .filter(|x| in_scope(&x.parent_type, &x.parent_id))
            .collect()
    };
    let mut base = BTreeMap::new();
    let mut targets = HashSet::new();
    let mut included = vec![];
    for x in &acts {
        base.insert(key("story_act", &x.id), x.rev);
        targets.insert(("story_act".to_string(), x.id.clone()));
        included.push(x.id.clone());
    }
    for x in &sequences {
        base.insert(key("story_sequence", &x.id), x.rev);
        targets.insert(("story_sequence".to_string(), x.id.clone()));
        targets.insert(("sequence".to_string(), x.id.clone()));
        included.push(x.id.clone());
    }
    for x in &beats {
        base.insert(key("story_beat", &x.id), x.rev);
        targets.insert(("story_beat".to_string(), x.id.clone()));
        targets.insert(("beat".to_string(), x.id.clone()));
        included.push(x.id.clone());
    }
    for x in &cards {
        base.insert(key("story_scene_card", &x.id), x.rev);
        targets.insert(("story_scene_card".to_string(), x.id.clone()));
        targets.insert(("scene_card".to_string(), x.id.clone()));
        included.push(x.id.clone());
    }
    let comments = if a.include_comments {
        load_comments(c, &targets, &HashMap::new(), &HashMap::new())?
    } else {
        vec![]
    };
    // Attachments on included Scene Cards / Beats / Sequences (only when selected).
    let owners: HashSet<(String, String)> = cards
        .iter()
        .map(|x| ("scene_card".to_string(), x.id.clone()))
        .chain(beats.iter().map(|x| ("beat".to_string(), x.id.clone())))
        .chain(
            sequences
                .iter()
                .map(|x| ("sequence".to_string(), x.id.clone())),
        )
        .collect();
    let mut available = vec![];
    {
        let mut stmt = c.prepare(
            "SELECT a.id, a.owner_type, a.owner_id, a.asset_id, s.original_name FROM story_attachment a
             JOIN asset s ON s.id = a.asset_id ORDER BY a.position, a.id",
        )?;
        let rows: Vec<(String, String, String, String, String)> = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;
        for (id, ot, oid, asset_id, name) in rows {
            if !owners.contains(&(ot.clone(), oid.clone()))
                || !openframe_domain::ids::is_valid_id(&id)
            {
                continue;
            }
            if let Ok(p) = crate::util::asset_file_path(c, root, &asset_id)
                && p.is_file()
            {
                let entry = format!(
                    "attachments/{id}/{}",
                    openframe_security::sanitize_file_name(&name)
                );
                available.push((
                    ExAttachment {
                        id,
                        owner_type: ot,
                        owner_id: oid,
                        file_name: name,
                        entry,
                    },
                    p,
                ));
            }
        }
    }
    let attachments_available = available.len() as u32;
    let (attachments, files): (Vec<ExAttachment>, Vec<(String, PathBuf)>) = if a.include_attachments
    {
        available
            .into_iter()
            .map(|(x, p)| (x.clone(), (x.entry, p)))
            .unzip()
    } else {
        (vec![], vec![])
    };
    let counts = vec![
        count("Acts", acts.len()),
        count("Sequences", sequences.len()),
        count("Beats", beats.len()),
        count("Scene Cards", cards.len()),
        count("Comments", comments.len()),
        count("Attachments", attachments.len()),
    ];
    let content = StoryContent {
        acts,
        sequences,
        beats,
        cards,
        attachments,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: files,
        attachments_available,
        counts,
        target: "Story Board".into(),
        scope: PackageScope {
            kind: scope_kind,
            label: scope_label,
            ids: a.act_id.iter().cloned().collect(),
        },
        source_draft: None,
        source_versions: vec![],
        included_ids: included,
        base,
        comments_included: a.include_comments,
        attachments_included: a.include_attachments && attachments_available > 0,
        explanation: "The receiver can import the outline as a new copy, or — for the same project — propose changes you review before anything is applied.".into(),
    })
}

fn build_breakdown(c: &Connection, a: &PackageExchangeArgs) -> AppResult<Built> {
    let (source_id, draft_id, draft_name) = active_source(c)?.ok_or_else(|| {
        AppError::validation(
            "source",
            "Choose a Production Source before sending a Breakdown package.",
        )
    })?;
    let mut scenes = scene_refs(c, &draft_id)?;
    let scope_kind = a.scope.clone().unwrap_or_else(|| "full".into());
    let scope_label = if scope_kind == "scenes" {
        let all: Vec<String> = scenes.iter().map(|s| s.id.clone()).collect();
        let keep = scope_scene_filter(&all, &a.scene_ids)?;
        scenes.retain(|s| keep.contains(&s.id));
        scene_numbers_label(&scenes.iter().map(|s| s.number).collect::<Vec<_>>())
    } else {
        "All scenes".to_string()
    };
    let scene_ids: HashSet<String> = scenes.iter().map(|s| s.id.clone()).collect();
    let mut stmt = c.prepare(
        "SELECT b.id, b.scene_id, b.scene_lineage_id, b.category, b.display_name, b.notes, b.catalog_item_id,
                ci.name, b.confirmation_state, b.rev
         FROM breakdown_element b LEFT JOIN catalog_item ci ON ci.id = b.catalog_item_id
         WHERE b.source_id=?1 AND b.deleted_at IS NULL AND b.confirmation_state IN ('Confirmed','Manual')
         ORDER BY b.scene_id, b.category, b.display_name, b.id",
    )?;
    let elements: Vec<ExBreakdownElement> = stmt
        .query_map([&source_id], |r| {
            Ok(ExBreakdownElement {
                id: r.get(0)?,
                scene_id: r.get(1)?,
                scene_lineage_id: r.get(2)?,
                category: r.get(3)?,
                display_name: r.get(4)?,
                notes: r.get(5)?,
                catalog_item_id: r.get(6)?,
                catalog_name: r.get(7)?,
                confirmation_state: r.get(8)?,
                rev: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| scene_ids.contains(&e.scene_id))
        .collect();
    let mut base = BTreeMap::new();
    let mut targets = HashSet::new();
    for e in &elements {
        base.insert(key("breakdown_element", &e.id), e.rev);
        targets.insert(("breakdown_element".to_string(), e.id.clone()));
    }
    let comments = if a.include_comments {
        load_comments(c, &targets, &HashMap::new(), &HashMap::new())?
    } else {
        vec![]
    };
    let counts = vec![
        count("Scenes", scenes.len()),
        count("Breakdown elements", elements.len()),
        count("Comments", comments.len()),
    ];
    let included_ids = elements.iter().map(|e| e.id.clone()).collect();
    let content = BreakdownContent {
        source_draft_id: draft_id.clone(),
        source_draft_name: draft_name.clone(),
        scenes,
        elements,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: vec![],
        attachments_available: 0,
        counts,
        target: format!("Production Source: {draft_name}"),
        scope: PackageScope {
            kind: scope_kind,
            label: scope_label,
            ids: a.scene_ids.clone(),
        },
        source_draft: None,
        source_versions: vec![PackageSourceVersion {
            kind: "productionSource".into(),
            id: draft_id,
            label: draft_name,
        }],
        included_ids,
        base,
        comments_included: a.include_comments,
        attachments_included: false,
        explanation: "Confirmed breakdown elements for the chosen scenes. When they come back, you preview additions and changes before anything is applied.".into(),
    })
}

fn build_shots(c: &Connection, a: &PackageExchangeArgs) -> AppResult<Built> {
    let draft = match active_source(c)? {
        Some((_, d, _)) => Some(d),
        None => current_draft(c, None)?.map(|d| d.id),
    }
    .ok_or_else(|| {
        AppError::validation(
            "source",
            "This project has no screenplay scenes to plan shots for yet.",
        )
    })?;
    let mut scenes = scene_refs(c, &draft)?;
    let scope_kind = a.scope.clone().unwrap_or_else(|| "full".into());
    let scope_label = if scope_kind == "scenes" {
        let all: Vec<String> = scenes.iter().map(|s| s.id.clone()).collect();
        let keep = scope_scene_filter(&all, &a.scene_ids)?;
        scenes.retain(|s| keep.contains(&s.id));
        scene_numbers_label(&scenes.iter().map(|s| s.number).collect::<Vec<_>>())
    } else {
        "All scenes".to_string()
    };
    let lineages: HashSet<String> = scenes.iter().map(|s| s.lineage_id.clone()).collect();
    let mut stmt = c.prepare(
        "SELECT id, scene_id, scene_lineage_id, position, description, size, movement, angle, lens, camera_notes,
                characters_json, sound_note, rev
         FROM shot WHERE deleted_at IS NULL ORDER BY scene_lineage_id, position, id",
    )?;
    let shots: Vec<ExShot> = stmt
        .query_map([], |r| {
            let chars: String = r.get(10)?;
            Ok(ExShot {
                id: r.get(0)?,
                scene_id: r.get(1)?,
                scene_lineage_id: r.get(2)?,
                position: r.get(3)?,
                description: r.get(4)?,
                size: r.get(5)?,
                movement: r.get(6)?,
                angle: r.get(7)?,
                lens: r.get(8)?,
                camera_notes: r.get(9)?,
                characters: serde_json::from_str(&chars).unwrap_or_default(),
                sound_note: r.get(11)?,
                rev: r.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|s| lineages.contains(&s.scene_lineage_id))
        .collect();
    let mut base = BTreeMap::new();
    let mut targets = HashSet::new();
    for s in &shots {
        base.insert(key("shot", &s.id), s.rev);
        targets.insert(("shot".to_string(), s.id.clone()));
    }
    let comments = if a.include_comments {
        load_comments(c, &targets, &HashMap::new(), &HashMap::new())?
    } else {
        vec![]
    };
    let counts = vec![
        count("Scenes", scenes.len()),
        count("Shots", shots.len()),
        count("Comments", comments.len()),
    ];
    let included_ids = shots.iter().map(|s| s.id.clone()).collect();
    let content = ShotsContent {
        scenes,
        shots,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: vec![],
        attachments_available: 0,
        counts,
        target: "Shot List".into(),
        scope: PackageScope {
            kind: scope_kind,
            label: scope_label,
            ids: a.scene_ids.clone(),
        },
        source_draft: None,
        source_versions: vec![],
        included_ids,
        base,
        comments_included: a.include_comments,
        attachments_included: false,
        explanation: "Shots and their planning fields for the chosen scenes. Returned changes are added or updated only after you confirm them.".into(),
    })
}

fn build_schedule(c: &Connection, a: &PackageExchangeArgs) -> AppResult<Built> {
    let (schedule_id, schedule_name) = current_schedule(c)?.ok_or_else(|| {
        AppError::validation("schedule", "This project has no shooting schedule yet.")
    })?;
    let mut stmt = c.prepare(
        "SELECT id, position, shoot_date, notes, is_off_day, planned_minutes, rev FROM shooting_day
         WHERE schedule_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let mut days: Vec<ExDay> = stmt
        .query_map([&schedule_id], |r| {
            Ok(ExDay {
                id: r.get(0)?,
                position: r.get(1)?,
                date: r.get(2)?,
                notes: r.get(3)?,
                is_off_day: r.get(4)?,
                planned_minutes: r.get(5)?,
                rev: r.get(6)?,
                strips: vec![],
                markers: vec![],
            })
        })?
        .collect::<Result<_, _>>()?;
    let index: HashMap<String, usize> = days
        .iter()
        .enumerate()
        .map(|(i, d)| (d.id.clone(), i))
        .collect();
    let mut stmt = c.prepare(
        "SELECT st.id, st.day_id, st.scene_id, st.scene_lineage_id, COALESCE(sc.heading, st.source_heading), st.position, st.rev
         FROM schedule_strip st LEFT JOIN screenplay_scene sc ON sc.id = st.scene_id
         WHERE st.schedule_id=?1 AND st.deleted_at IS NULL AND st.archived = 0 AND st.day_id IS NOT NULL
         ORDER BY st.position, st.id",
    )?;
    let strips: Vec<(String, ExStrip)> = stmt
        .query_map([&schedule_id], |r| {
            Ok((
                r.get::<_, String>(1)?,
                ExStrip {
                    id: r.get(0)?,
                    scene_id: r.get(2)?,
                    scene_lineage_id: r.get(3)?,
                    heading: r.get(4)?,
                    position: r.get(5)?,
                    rev: r.get(6)?,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut base = BTreeMap::new();
    for (day, s) in strips {
        if let Some(i) = index.get(&day) {
            base.insert(key("schedule_strip", &s.id), s.rev);
            days[*i].strips.push(s);
        }
    }
    let mut stmt = c.prepare(
        "SELECT m.id, m.day_id, m.marker_type, m.label, m.at_time, m.duration_minutes, m.notes, m.position
         FROM schedule_marker m JOIN shooting_day d ON d.id = m.day_id
         WHERE d.schedule_id=?1 AND m.deleted_at IS NULL ORDER BY m.position, m.id",
    )?;
    let markers: Vec<(String, ExMarker)> = stmt
        .query_map([&schedule_id], |r| {
            Ok((
                r.get::<_, String>(1)?,
                ExMarker {
                    id: r.get(0)?,
                    marker_type: r.get(2)?,
                    label: r.get(3)?,
                    at_time: r.get(4)?,
                    duration_minutes: r.get(5)?,
                    notes: r.get(6)?,
                    position: r.get(7)?,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    for (day, m) in markers {
        if let Some(i) = index.get(&day) {
            days[*i].markers.push(m);
        }
    }
    let mut targets = HashSet::new();
    for d in &days {
        base.insert(key("shooting_day", &d.id), d.rev);
        targets.insert(("shooting_day".to_string(), d.id.clone()));
    }
    let comments = if a.include_comments {
        load_comments(c, &targets, &HashMap::new(), &HashMap::new())?
    } else {
        vec![]
    };
    let counts = vec![
        count("Shooting days", days.len()),
        count(
            "Scheduled scenes",
            days.iter().map(|d| d.strips.len()).sum(),
        ),
        count(
            "Breaks and moves",
            days.iter().map(|d| d.markers.len()).sum(),
        ),
        count("Comments", comments.len()),
    ];
    let included_ids = days.iter().map(|d| d.id.clone()).collect();
    let content = ScheduleContent {
        schedule_id: schedule_id.clone(),
        schedule_name: schedule_name.clone(),
        days,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: vec![],
        attachments_available: 0,
        counts,
        target: schedule_name.clone(),
        scope: PackageScope {
            kind: "schedule".into(),
            label: "Entire schedule".into(),
            ids: vec![],
        },
        source_draft: None,
        source_versions: vec![PackageSourceVersion {
            kind: "schedule".into(),
            id: schedule_id,
            label: schedule_name,
        }],
        included_ids,
        base,
        comments_included: a.include_comments,
        attachments_included: false,
        explanation: "Shooting days, scene assignments, breaks and day notes. Importing a returned schedule never deletes or silently reorders your days.".into(),
    })
}

fn build_call_review(c: &Connection, a: &PackageExchangeArgs) -> AppResult<Built> {
    let id = match &a.call_sheet_id {
        Some(id) => id.clone(),
        None => c
            .query_row(
                "SELECT id FROM call_sheet WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::validation("callSheetId", "This project has no call sheet yet."))?,
    };
    let row: (String, i64, String, String, String, String, i64) = c
        .query_row(
            "SELECT title, revision, status, shoot_day_id, schedule_id, document_json, rev FROM call_sheet
             WHERE id=?1 AND deleted_at IS NULL",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("call sheet"))?;
    let (title, revision, status, shoot_day_id, schedule_id, document_json, rev) = row;
    let mut targets = HashSet::new();
    targets.insert(("call_sheet".to_string(), id.clone()));
    let comments = if a.include_comments {
        load_comments(c, &targets, &HashMap::new(), &HashMap::new())?
    } else {
        vec![]
    };
    let mut base = BTreeMap::new();
    base.insert(key("call_sheet", &id), rev);
    let counts = vec![count("Call sheet", 1), count("Comments", comments.len())];
    let content = CallReviewContent {
        call_sheet_id: id.clone(),
        title: title.clone(),
        revision,
        status,
        shoot_day_id,
        schedule_id,
        document: serde_json::from_str(&document_json).unwrap_or(Value::Null),
        rev,
        comments,
    };
    Ok(Built {
        content: to_value(&content)?,
        attachments: vec![],
        attachments_available: 0,
        counts,
        target: format!("{title} (revision {revision})"),
        scope: PackageScope {
            kind: "callSheet".into(),
            label: title,
            ids: vec![id.clone()],
        },
        source_draft: None,
        source_versions: vec![],
        included_ids: vec![id],
        base,
        comments_included: a.include_comments,
        attachments_included: false,
        explanation: "A snapshot of this call sheet for review. Returned feedback is added to the call sheet as comments and never changes the shooting schedule.".into(),
    })
}

pub(crate) fn build(c: &Connection, root: &Path, a: &PackageExchangeArgs) -> AppResult<Built> {
    if a.include_private_notes {
        return Err(AppError::validation(
            "includePrivateNotes",
            "Private notes can't be included in review packages. They stay visible only to you.",
        ));
    }
    match a.package_type {
        PackageType::ScriptReview => build_script(c, a),
        PackageType::Story => build_story(c, root, a),
        PackageType::Breakdown => build_breakdown(c, a),
        PackageType::Shots => build_shots(c, a),
        PackageType::Schedule => build_schedule(c, a),
        PackageType::CallReview => build_call_review(c, a),
        PackageType::Response => Err(AppError::invalid_input(
            "Response packages are exported from the review package viewer.",
        )),
        PackageType::Backup | PackageType::Project => Err(AppError::invalid_input(
            "Use Create Backup or Export Full Project for whole-project packages.",
        )),
    }
}

// ================================================================== ops

pub(crate) fn sources(core: &AppCore, actor: &Actor) -> AppResult<PackageExchangeSources> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT d.id, d.name, s.title, d.status, s.current_draft_id = d.id,
                    (SELECT count(*) FROM screenplay_scene x WHERE x.draft_id = d.id AND x.deleted_at IS NULL)
             FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id
             WHERE d.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY s.created_at, d.created_at DESC, d.id",
        )?;
        let drafts = stmt
            .query_map([], |r| {
                Ok(PackageDraftOption {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    screenplay_title: r.get(2)?,
                    status: r.get(3)?,
                    is_current: r.get::<_, Option<bool>>(4)?.unwrap_or(false),
                    scene_count: r.get::<_, i64>(5)? as u32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = c.prepare("SELECT id, title FROM story_act WHERE deleted_at IS NULL ORDER BY position, id")?;
        let acts = stmt
            .query_map([], |r| Ok(PackageOption { id: r.get(0)?, label: r.get(1)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut stmt = c.prepare(
            "SELECT id, title || ' (revision ' || revision || ', ' || status || ')' FROM call_sheet
             WHERE deleted_at IS NULL ORDER BY updated_at DESC, id",
        )?;
        let call_sheets = stmt
            .query_map([], |r| Ok(PackageOption { id: r.get(0)?, label: r.get(1)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PackageExchangeSources {
            drafts,
            acts,
            production_source: active_source(c)?.map(|(_, id, name)| PackageOption { id, label: name }),
            schedule: current_schedule(c)?.map(|(id, name)| PackageOption { id, label: name }),
            call_sheets,
        })
    })
}

pub(crate) fn scenes(
    core: &AppCore,
    actor: &Actor,
    a: PackageScenesArgs,
) -> AppResult<Vec<PackageSceneOption>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let draft = match a.draft_id {
            Some(id) => Some(id),
            None => current_draft(c, None)?.map(|d| d.id),
        };
        let Some(draft) = draft else {
            return Ok(vec![]);
        };
        Ok(scene_refs(c, &draft)?
            .into_iter()
            .map(|s| PackageSceneOption {
                id: s.id,
                number: s.number,
                heading: if s.heading.trim().is_empty() {
                    "(untitled scene)".into()
                } else {
                    s.heading
                },
            })
            .collect())
    })
}

fn summary_of(
    core: &AppCore,
    actor: &Actor,
    ty: PackageType,
    b: &Built,
) -> AppResult<PackageExportSummary> {
    let s = core.project()?;
    let project_title = s.manifest.lock().title.clone();
    let suggested = openframe_security::sanitize_file_name(&format!(
        "{project_title} - {} - {}",
        b.target,
        ty.label()
    ));
    Ok(PackageExportSummary {
        package_type: ty,
        type_label: ty.label().to_string(),
        project_title,
        target: b.target.clone(),
        scope_label: b.scope.label.clone(),
        exported_by: actor.display_name.clone(),
        date: now_ms(),
        counts: b.counts.clone(),
        comments_included: b.comments_included,
        attachments_included: b.attachments_included,
        attachments_available: b.attachments_available,
        private_notes_excluded: true,
        suggested_file_name: format!("{suggested}.{}", ty.extension()),
        explanation: b.explanation.clone(),
    })
}

pub(crate) fn preview(
    core: &AppCore,
    actor: &Actor,
    a: PackageExchangeArgs,
) -> AppResult<PackageExportSummary> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let built = s.store.read(|c| build(c, &root, &a))?;
    summary_of(core, actor, a.package_type, &built)
}

pub(crate) fn export(
    core: &AppCore,
    actor: &Actor,
    a: PackageExchangeArgs,
) -> AppResult<PackageExportResult> {
    actor.require(
        Capability::CreatePackage,
        "create packages from this project",
    )?;
    let dest = format::package_destination(a.path.as_deref().unwrap_or(""), a.package_type)?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let built = s.store.read(|c| build(c, &root, &a))?;
    let summary = summary_of(core, actor, a.package_type, &built)?;
    let (project_id, project_title) = {
        let m = s.manifest.lock();
        (m.project_id.clone(), m.title.clone())
    };
    let manifest = PackageManifest {
        format: format::FORMAT_NAME.into(),
        package_type: a.package_type,
        format_version: PACKAGE_FORMAT_VERSION,
        package_id: new_id(),
        source_project_id: project_id,
        source_project_title: project_title,
        exported_at: now_ms(),
        app_version: core.config.app_version.clone(),
        schema_version: None,
        source_draft: built.source_draft.clone(),
        source_versions: built.source_versions.clone(),
        included_object_ids: built.included_ids.clone(),
        scope: built.scope.clone(),
        comments_included: built.comments_included,
        attachments_included: built.attachments_included,
        private_notes_included: false,
        base_snapshot: built.base.clone(),
        originating_user: PackageUser {
            user_id: actor.user_id.clone(),
            display_name: actor.display_name.clone(),
        },
        label: None,
        responds_to: None,
    };
    let mut entries = vec![(
        format::CONTENT_ENTRY.to_string(),
        EntryData::Bytes(format::to_json_bytes(&built.content)?),
    )];
    for (name, path) in &built.attachments {
        entries.push((name.clone(), EntryData::File(path.clone())));
    }
    core.save.external_started();
    let written = format::write_package(&dest, &manifest, entries, &|_, _| {}, &|| false);
    core.save.external_finished();
    written?;
    let file_name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let counts: Vec<String> = built
        .counts
        .iter()
        .filter(|c| c.count > 0)
        .map(|c| format!("{} {}", c.count, c.label.to_lowercase()))
        .collect();
    log_package(
        &s.store,
        actor,
        "export",
        a.package_type,
        &manifest.package_id,
        &file_name,
        &format!(
            "Exported {} “{file_name}” ({}; private notes excluded)",
            a.package_type.label(),
            if counts.is_empty() {
                plural(0, "item", "items")
            } else {
                counts.join(", ")
            }
        ),
        None,
    )?;
    Ok(PackageExportResult {
        path: dest.to_string_lossy().into_owned(),
        file_name,
        package_id: manifest.package_id,
        summary,
    })
}
