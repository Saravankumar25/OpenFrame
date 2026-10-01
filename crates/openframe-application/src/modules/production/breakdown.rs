//! Breakdown workspace (FSD §26–27, §54, §96–97, §125, §143).
//!
//! "What does this scene require to shoot?" Scenes come from the Production
//! Source draft; elements are scene-level associations to reusable catalog
//! items. Suggestions (deterministic engine, `suggest.rs`) are stored as
//! `Suggested` rows that never count as production data until accepted;
//! rejected suggestions are remembered per scene + source and not re-suggested.

use std::collections::{HashMap, HashSet};

use openframe_domain::enums::{BreakdownCategory, ConfirmationState};
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::catalog::{
    BreakdownCatalogChoice, CatalogMatchDto, CatalogStatus, find_matches, match_dtos_with,
    match_kind, resolve_choice,
};
use super::script::{self, ElementRow, SceneRow};
use super::suggest::{self, BreakdownSceneFacts, Candidate, ScriptElement, name_key};
use super::{
    ActiveSource, PRODUCTION_STATES, SourceScenes, active_source, catalog_usage, parse_category,
    patch_text, require_source,
};
use crate::core::AppCore;
use crate::registry::{Registry, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::required_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Breakdown");
    r.query("breakdown.scenes", scenes).meta(M::compute(
        "Breakdown scene list with element counts, completion and review flags.",
    ));
    r.query("breakdown.scene", scene).meta(M::read(
        "Breakdown of one scene: elements by category with confirmation state.",
    ));
    r.query("breakdown.scene_changes", scene_changes)
        .meta(M::compute(
            "What changed in a scene since its breakdown was reviewed.",
        ));
    r.command("breakdown.suggest", suggest_op).meta(M::edit(
        "Run the breakdown suggestion engine on a scene (adds Suggested elements for review).",
    ));
    r.command("breakdown.accept", accept).meta(
        M::edit("Confirm a suggested breakdown element (optionally correcting category/name).")
            .hidden(crate::registry::hidden::DUPLICATE),
    );
    r.command("breakdown.accept_many", accept_many)
        .meta(M::edit("Confirm several suggested breakdown elements."));
    r.command("breakdown.reject", reject)
        .meta(M::edit("Reject suggested breakdown elements.").destructive());
    r.command("breakdown.add_element", add_element)
        .meta(M::edit("Add a breakdown element to a scene manually."));
    r.command("breakdown.update_element", update_element)
        .meta(M::edit("Edit a breakdown element's notes."));
    r.command("breakdown.remove_element", remove_element)
        .meta(M::soft_delete("Remove a breakdown element (recoverable)."));
    r.command("breakdown.set_archived", set_archived)
        .meta(M::edit("Archive or unarchive a breakdown element."));
    r.command("breakdown.set_complete", set_complete)
        .meta(M::edit("Mark a scene's breakdown complete or incomplete."));
    r.command("breakdown.mark_reviewed", mark_reviewed)
        .meta(M::edit("Mark a changed scene's breakdown as reviewed."));
    r.command("breakdown.apply_suggested_update", apply_suggested_update)
        .meta(M::edit(
            "Re-run suggestions for a changed scene and keep confirmed elements.",
        ));
    r.trash_handler(TrashHandler {
        object_type: "breakdown_element",
        table: "breakdown_element",
        label: "Breakdown element",
        restore: None,
        purge: purge_element,
    });
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownElementDto {
    pub id: String,
    pub scene_id: String,
    pub category: BreakdownCategory,
    /// Current name: the catalog item's name when linked (renames show everywhere).
    pub name: String,
    /// The name as tagged / suggested in this scene.
    pub display_name: String,
    pub catalog_item_id: Option<String>,
    pub catalog_status: Option<CatalogStatus>,
    pub catalog_archived: bool,
    /// The linked catalog item was deleted (recoverable) or permanently removed.
    pub catalog_removed: bool,
    pub state: ConfirmationState,
    /// "manual", "tagged" or "suggested".
    pub origin: String,
    /// Matched phrase / tagged text.
    pub evidence: Option<String>,
    /// "Likely" / "Possible" for suggestions.
    pub confidence: Option<String>,
    pub notes: Option<String>,
    pub span_element_id: Option<String>,
    #[ts(type = "number | null")]
    pub span_start: Option<i64>,
    #[ts(type = "number | null")]
    pub span_end: Option<i64>,
    pub archived: bool,
    /// Suggestions only: "exact", "ambiguous" or "none".
    pub match_kind: Option<String>,
    /// Suggestions only: existing catalog items that look the same.
    pub matches: Vec<CatalogMatchDto>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownSceneRow {
    pub scene_id: String,
    pub lineage_id: String,
    pub number: String,
    pub heading: String,
    pub omitted: bool,
    #[ts(type = "number")]
    pub confirmed_count: i64,
    #[ts(type = "number")]
    pub suggested_count: i64,
    pub complete: bool,
    /// The script changed after production planning began (FSD §54, §125).
    pub needs_review: bool,
    pub heading_changed: bool,
    pub text_changed: bool,
    /// New since the source was chosen and not broken down yet.
    pub needs_breakdown: bool,
    /// e.g. "Scene 24 changed in Shooting Draft B. Review Breakdown."
    pub change_message: Option<String>,
    #[ts(type = "number | null")]
    pub changed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownHistoricalScene {
    pub scene_id: String,
    pub heading: String,
    pub draft_name: Option<String>,
    pub elements: Vec<BreakdownElementDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownScenesView {
    pub source: Option<super::source::ProductionSourceInfo>,
    pub scenes: Vec<BreakdownSceneRow>,
    /// Breakdown kept for scenes no longer in the source ("Scene removed from current source.").
    pub historical: Vec<BreakdownHistoricalScene>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownScriptLine {
    pub element_id: String,
    pub element_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownCategoryGroup {
    pub category: BreakdownCategory,
    pub confirmed: Vec<BreakdownElementDto>,
    pub suggestions: Vec<BreakdownElementDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownSceneDetail {
    pub scene_id: String,
    pub lineage_id: String,
    pub number: Option<String>,
    pub heading: String,
    pub in_source: bool,
    pub draft_name: Option<String>,
    #[ts(type = "number")]
    pub scene_count: i64,
    pub facts: BreakdownSceneFacts,
    pub script: Vec<BreakdownScriptLine>,
    /// Only categories with content; categories with confirmed content first (FSD §26.6, §96).
    pub groups: Vec<BreakdownCategoryGroup>,
    #[ts(type = "number")]
    pub confirmed_count: i64,
    #[ts(type = "number")]
    pub suggestion_count: i64,
    pub complete: bool,
    pub needs_review: bool,
    pub heading_changed: bool,
    pub text_changed: bool,
    pub needs_breakdown: bool,
    pub change_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownCandidateDto {
    pub category: BreakdownCategory,
    pub name: String,
    pub evidence: String,
}

/// "Scene 24 changed. Review breakdown differences." (Mock 110)
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownSceneChanges {
    pub scene_id: String,
    pub number: Option<String>,
    pub heading: String,
    pub baseline_heading: Option<String>,
    pub heading_changed: bool,
    pub text_changed: bool,
    pub removed_candidates: Vec<BreakdownCandidateDto>,
    pub added_candidates: Vec<BreakdownCandidateDto>,
    /// "New elements suggested: 3"
    #[ts(type = "number")]
    pub new_suggestions: i64,
    /// "Existing elements requiring review: 2"
    pub elements_to_review: Vec<BreakdownElementDto>,
    /// e.g. "Shot list exists: Yes — review recommended"
    pub planning_notes: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownSuggestResult {
    #[ts(type = "number")]
    pub added: i64,
    #[ts(type = "number")]
    pub pending: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownAcceptManyResult {
    #[ts(type = "number")]
    pub accepted: i64,
    /// Suggestions with ambiguous catalog matches: the user must choose (never attached silently).
    pub needs_choice: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownCountResult {
    #[ts(type = "number")]
    pub count: i64,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownScenesArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownSceneArgs {
    pub scene_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownIdsArgs {
    pub ids: Vec<String>,
}

/// Accept one suggestion, optionally edited first (FSD §27.6).
#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownAcceptArgs {
    pub id: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub catalog: BreakdownCatalogChoice,
}

/// Tagged text: a screenplay element and UTF-16 offsets within its text.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownSpan {
    pub element_id: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownAddArgs {
    pub scene_id: String,
    pub category: String,
    pub name: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub span: Option<BreakdownSpan>,
    #[serde(default)]
    pub catalog: BreakdownCatalogChoice,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownArchiveArgs {
    pub id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownCompleteArgs {
    pub scene_id: String,
    pub complete: bool,
}

// ------------------------------------------------------------ elements

const EL_SQL: &str = "SELECT e.id, e.scene_id, e.category, e.display_name, e.catalog_item_id, ci.name, ci.status,
        COALESCE(ci.archived, 0), ci.deleted_at, e.catalog_removed_at, e.confirmation_state, e.origin,
        e.source_evidence, e.confidence, e.notes, e.span_element_id, e.span_start, e.span_end, e.archived, e.rev
     FROM breakdown_element e LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id
     WHERE e.deleted_at IS NULL";

fn el_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<BreakdownElementDto> {
    let display_name: String = r.get(3)?;
    let catalog_name: Option<String> = r.get(5)?;
    let status: Option<String> = r.get(6)?;
    let cat_deleted: Option<i64> = r.get(8)?;
    let removed_at: Option<i64> = r.get(9)?;
    let state: String = r.get(10)?;
    let category: String = r.get(2)?;
    Ok(BreakdownElementDto {
        id: r.get(0)?,
        scene_id: r.get(1)?,
        category: BreakdownCategory::parse(&category).unwrap_or(BreakdownCategory::Props),
        name: catalog_name.clone().unwrap_or_else(|| display_name.clone()),
        display_name,
        catalog_item_id: r.get(4)?,
        catalog_status: status.as_deref().and_then(CatalogStatus::parse),
        catalog_archived: r.get(7)?,
        catalog_removed: cat_deleted.is_some() || removed_at.is_some(),
        state: ConfirmationState::parse(&state).unwrap_or(ConfirmationState::Suggested),
        origin: r.get(11)?,
        evidence: r.get(12)?,
        confidence: r.get(13)?,
        notes: r.get(14)?,
        span_element_id: r.get(15)?,
        span_start: r.get(16)?,
        span_end: r.get(17)?,
        archived: r.get(18)?,
        match_kind: None,
        matches: vec![],
        rev: r.get(19)?,
    })
}

fn scene_elements_dto(c: &Connection, scene_id: &str) -> AppResult<Vec<BreakdownElementDto>> {
    let mut stmt = c.prepare(&format!(
        "{EL_SQL} AND e.scene_id = ?1 ORDER BY e.created_at, e.id"
    ))?;
    let rows = stmt
        .query_map([scene_id], el_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn load_element(c: &Connection, id: &str) -> AppResult<BreakdownElementDto> {
    c.query_row(&format!("{EL_SQL} AND e.id = ?1"), [id], el_row)
        .optional()?
        .ok_or_else(|| AppError::not_found("breakdown element"))
}

// ------------------------------------------------------ scene state

struct StateRow {
    complete: bool,
    needs_breakdown: bool,
    baseline_heading: Option<String>,
    baseline_text: Option<String>,
}

fn states(c: &Connection) -> AppResult<HashMap<String, StateRow>> {
    let mut stmt =
        c.prepare("SELECT scene_lineage_id, complete, needs_breakdown, baseline_heading, baseline_text FROM production_scene_state")?;
    let mut out = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            StateRow {
                complete: r.get(1)?,
                needs_breakdown: r.get(2)?,
                baseline_heading: r.get(3)?,
                baseline_text: r.get(4)?,
            },
        ))
    })? {
        let (k, v) = row?;
        out.insert(k, v);
    }
    Ok(out)
}

fn body_of(encoded: &str) -> &str {
    encoded.split_once('\n').map(|(_, b)| b).unwrap_or("")
}

fn norm_heading(h: &str) -> String {
    h.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
        .replace(['—', '–'], "-")
}

/// Current encoded text of a scene.
fn current_encoded(
    c: &Connection,
    scene_id: &str,
    raw_heading: &str,
) -> AppResult<(String, String)> {
    let els = script::scene_elements(c, scene_id)?;
    let heading = script::effective_heading(raw_heading, &els);
    let enc = script::encode_scene(&heading, &els);
    Ok((heading, enc))
}

/// Insert a production scene state (baseline = the given text) when none exists.
pub(crate) fn insert_state_if_missing(
    tx: &Tx<'_>,
    lineage: &str,
    draft_id: &str,
    heading: &str,
    encoded: &str,
    needs_breakdown: bool,
) -> AppResult<()> {
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO production_scene_state(id, scene_lineage_id, complete, needs_breakdown, baseline_draft_id, baseline_heading,
                                            baseline_text, baseline_fingerprint, created_at, updated_at)
         VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(scene_lineage_id) DO NOTHING",
        params![new_id(), lineage, needs_breakdown, draft_id, heading, encoded, script::fingerprint(encoded), now],
    )?;
    Ok(())
}

/// Make sure a scene the user is working on has a state row (baseline = current text).
fn ensure_state(tx: &Tx<'_>, scene_id: &str) -> AppResult<String> {
    let sc =
        script::scene_lookup(tx.conn(), scene_id)?.ok_or_else(|| AppError::not_found("scene"))?;
    let (heading, enc) = current_encoded(tx.conn(), scene_id, &sc.heading)?;
    insert_state_if_missing(tx, &sc.lineage_id, &sc.draft_id, &heading, &enc, false)?;
    Ok(sc.lineage_id)
}

fn state_id(c: &Connection, lineage: &str) -> AppResult<String> {
    Ok(c.query_row(
        "SELECT id FROM production_scene_state WHERE scene_lineage_id = ?1",
        [lineage],
        |r| r.get(0),
    )?)
}

// ------------------------------------------------------ scene statuses

#[derive(Debug, Clone)]
pub(crate) struct SceneStatus {
    pub row: SceneRow,
    pub heading: String,
    pub confirmed: i64,
    pub suggested: i64,
    pub complete: bool,
    pub needs_review: bool,
    pub heading_changed: bool,
    pub text_changed: bool,
    pub needs_breakdown: bool,
    pub changed_at: Option<i64>,
}

fn element_counts(c: &Connection) -> AppResult<HashMap<String, (i64, i64)>> {
    let mut stmt = c.prepare(&format!(
        "SELECT scene_id, SUM(confirmation_state IN {PRODUCTION_STATES}), SUM(confirmation_state = 'Suggested')
         FROM breakdown_element WHERE deleted_at IS NULL AND archived = 0 GROUP BY scene_id"
    ))?;
    let mut out = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })? {
        let (s, a, b) = row?;
        out.insert(s, (a, b));
    }
    Ok(out)
}

/// Derived per-scene breakdown status for the active source (progress is
/// derived from confirmation state; "complete" is the manual indicator).
pub(crate) fn scene_statuses(
    c: &Connection,
    source: &ActiveSource,
    scenes: &SourceScenes,
) -> AppResult<Vec<SceneStatus>> {
    let els = script::draft_elements(c, &source.draft_id)?;
    let counts = element_counts(c)?;
    let states = states(c)?;
    let empty: Vec<ElementRow> = vec![];
    let mut out = Vec::with_capacity(scenes.rows.len());
    for row in &scenes.rows {
        let e = els.get(&row.id).unwrap_or(&empty);
        let heading = script::effective_heading(&row.heading, e);
        let enc = script::encode_scene(&heading, e);
        let (confirmed, suggested) = counts.get(&row.id).copied().unwrap_or((0, 0));
        let st = states.get(&row.lineage_id);
        let complete = st.map(|s| s.complete).unwrap_or(false);
        let (heading_changed, text_changed) = match st
            .and_then(|s| s.baseline_text.as_deref().map(|t| (s, t)))
        {
            Some((s, base)) => (
                norm_heading(s.baseline_heading.as_deref().unwrap_or("")) != norm_heading(&heading),
                body_of(base) != body_of(&enc),
            ),
            None => (false, false),
        };
        let planned = confirmed > 0 || complete;
        let needs_review = planned && (heading_changed || text_changed);
        let needs_breakdown =
            st.map(|s| s.needs_breakdown).unwrap_or(true) && confirmed == 0 && !complete;
        out.push(SceneStatus {
            changed_at: if needs_review {
                Some(script::scene_changed_at(row.updated_at, e))
            } else {
                None
            },
            row: row.clone(),
            heading,
            confirmed,
            suggested,
            complete,
            needs_review,
            heading_changed,
            text_changed,
            needs_breakdown,
        });
    }
    Ok(out)
}

fn change_message(st: &SceneStatus, draft_name: &str) -> Option<String> {
    if !st.needs_review {
        return None;
    }
    Some(if st.heading_changed && !st.text_changed {
        format!(
            "Scene {} heading changed in {draft_name}. The location/time description changed — review the breakdown.",
            st.row.number
        )
    } else {
        format!(
            "Scene {} changed in {draft_name}. Review Breakdown.",
            st.row.number
        )
    })
}

fn draft_name(c: &Connection, draft_id: &str) -> AppResult<String> {
    Ok(script::draft_info(c, draft_id)?
        .map(|d| d.name)
        .unwrap_or_else(|| "the source draft".into()))
}

// ------------------------------------------------------------------ reads

fn scenes(core: &AppCore, actor: &Actor, _: BreakdownScenesArgs) -> AppResult<BreakdownScenesView> {
    actor.require(Capability::View, "view the breakdown")?;
    let s = core.project()?;
    s.store.read(|c| {
        let info = super::source::source_info(c)?;
        let Some(source) = active_source(c)? else {
            return Ok(BreakdownScenesView {
                source: info,
                scenes: vec![],
                historical: vec![],
            });
        };
        let src_scenes = SourceScenes::load(c, Some(&source.draft_id))?;
        let dname = info
            .as_ref()
            .map(|i| i.draft.name.clone())
            .unwrap_or_default();
        let rows = scene_statuses(c, &source, &src_scenes)?
            .into_iter()
            .map(|st| BreakdownSceneRow {
                change_message: change_message(&st, &dname),
                scene_id: st.row.id.clone(),
                lineage_id: st.row.lineage_id.clone(),
                number: st.row.number.clone(),
                heading: st.heading.clone(),
                omitted: st.row.omitted,
                confirmed_count: st.confirmed,
                suggested_count: st.suggested,
                complete: st.complete,
                needs_review: st.needs_review,
                heading_changed: st.heading_changed,
                text_changed: st.text_changed,
                needs_breakdown: st.needs_breakdown,
                changed_at: st.changed_at,
            })
            .collect();
        // Historical breakdown: production rows attached to scenes outside the source.
        let mut stmt = c.prepare(&format!(
            "{EL_SQL} AND e.confirmation_state IN {PRODUCTION_STATES} ORDER BY e.created_at, e.id"
        ))?;
        let all: Vec<BreakdownElementDto> =
            stmt.query_map([], el_row)?.collect::<Result<_, _>>()?;
        let mut hist: Vec<BreakdownHistoricalScene> = Vec::new();
        for el in all
            .into_iter()
            .filter(|e| src_scenes.get(&e.scene_id).is_none())
        {
            if let Some(h) = hist.iter_mut().find(|h| h.scene_id == el.scene_id) {
                h.elements.push(el);
                continue;
            }
            let lk = script::scene_lookup(c, &el.scene_id)?;
            let draft_name = match &lk {
                Some(l) => script::draft_info(c, &l.draft_id)?.map(|d| d.name),
                None => None,
            };
            hist.push(BreakdownHistoricalScene {
                scene_id: el.scene_id.clone(),
                heading: lk.map(|l| l.heading).unwrap_or_default(),
                draft_name,
                elements: vec![el],
            });
        }
        Ok(BreakdownScenesView {
            source: info,
            scenes: rows,
            historical: hist,
        })
    })
}

fn build_detail(c: &Connection, scene_id: &str) -> AppResult<BreakdownSceneDetail> {
    let lk = script::scene_lookup(c, scene_id)?.ok_or_else(|| AppError::not_found("scene"))?;
    let source = active_source(c)?;
    let src_scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
    let els = script::scene_elements(c, scene_id)?;
    let heading = script::effective_heading(&lk.heading, &els);
    let in_source = src_scenes.get(scene_id).is_some();
    let status = match (&source, in_source) {
        (Some(src), true) => scene_statuses(c, src, &src_scenes)?
            .into_iter()
            .find(|s| s.row.id == scene_id),
        _ => None,
    };
    let dname = draft_name(c, &lk.draft_id)?;
    let usage = catalog_usage(c, &src_scenes)?;
    let mut elements = scene_elements_dto(c, scene_id)?;
    for e in elements
        .iter_mut()
        .filter(|e| e.state == ConfirmationState::Suggested)
    {
        let m = find_matches(c, e.category, &e.display_name, None)?;
        e.match_kind = Some(match_kind(&m).to_string());
        e.matches = match_dtos_with(&usage, m);
    }
    let mut groups: Vec<BreakdownCategoryGroup> = BreakdownCategory::ALL
        .iter()
        .map(|cat| BreakdownCategoryGroup {
            category: *cat,
            confirmed: elements
                .iter()
                .filter(|e| {
                    e.category == *cat
                        && e.state != ConfirmationState::Suggested
                        && e.state != ConfirmationState::Rejected
                })
                .cloned()
                .collect(),
            suggestions: elements
                .iter()
                .filter(|e| e.category == *cat && e.state == ConfirmationState::Suggested)
                .cloned()
                .collect(),
        })
        .filter(|g| !g.confirmed.is_empty() || !g.suggestions.is_empty())
        .collect();
    groups.sort_by_key(|g| g.confirmed.is_empty());
    let confirmed_count = elements
        .iter()
        .filter(|e| {
            matches!(
                e.state,
                ConfirmationState::Confirmed | ConfirmationState::Manual
            ) && !e.archived
        })
        .count() as i64;
    let suggestion_count = elements
        .iter()
        .filter(|e| e.state == ConfirmationState::Suggested)
        .count() as i64;
    Ok(BreakdownSceneDetail {
        scene_id: scene_id.to_string(),
        lineage_id: lk.lineage_id.clone(),
        number: src_scenes.get(scene_id).map(|r| r.number.clone()),
        facts: suggest::parse_heading(&heading),
        heading,
        in_source,
        draft_name: Some(dname.clone()),
        scene_count: src_scenes.rows.len() as i64,
        script: els
            .iter()
            .map(|e| BreakdownScriptLine {
                element_id: e.id.clone(),
                element_type: e.element_type.clone(),
                text: e.text.clone(),
            })
            .collect(),
        groups,
        confirmed_count,
        suggestion_count,
        complete: status.as_ref().map(|s| s.complete).unwrap_or(false),
        needs_review: status.as_ref().map(|s| s.needs_review).unwrap_or(false),
        heading_changed: status.as_ref().map(|s| s.heading_changed).unwrap_or(false),
        text_changed: status.as_ref().map(|s| s.text_changed).unwrap_or(false),
        needs_breakdown: status.as_ref().map(|s| s.needs_breakdown).unwrap_or(false),
        change_message: status.as_ref().and_then(|s| change_message(s, &dname)),
    })
}

fn scene(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownSceneArgs,
) -> AppResult<BreakdownSceneDetail> {
    actor.require(Capability::View, "view the breakdown")?;
    let s = core.project()?;
    s.store.read(|c| build_detail(c, &args.scene_id))
}

// ------------------------------------------------------------ suggestions

/// Upper-case names of characters known in the source (for action mentions).
fn known_names(c: &Connection, scenes: &SourceScenes, draft_id: &str) -> AppResult<Vec<String>> {
    Ok(super::people::known_characters(c, scenes, Some(draft_id))?
        .into_iter()
        .map(|e| e.key)
        .collect())
}

fn candidates_for(
    heading: &str,
    els: &[(String, String, String)],
    known: &[String],
) -> Vec<Candidate> {
    let script_els: Vec<ScriptElement<'_>> = els
        .iter()
        .map(|(id, t, x)| ScriptElement {
            id: id.as_str(),
            element_type: t.as_str(),
            text: x.as_str(),
        })
        .collect();
    suggest::suggest(heading, &script_els, known)
}

/// Keys (category, name key) already represented in a scene, including linked
/// catalog names and aliases, so nothing present is suggested again.
fn present_keys(c: &Connection, scene_id: &str) -> AppResult<HashSet<(String, String)>> {
    let mut out = HashSet::new();
    let mut stmt = c.prepare(
        "SELECT e.category, e.display_name, ci.name, e.catalog_item_id FROM breakdown_element e
         LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id
         WHERE e.scene_id = ?1 AND e.deleted_at IS NULL",
    )?;
    let rows: Vec<(String, String, Option<String>, Option<String>)> = stmt
        .query_map([scene_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    for (cat, display, cname, item) in rows {
        out.insert((cat.clone(), name_key(&display)));
        if let Some(n) = cname {
            out.insert((cat.clone(), name_key(&n)));
        }
        if let Some(item) = item {
            let mut a =
                c.prepare("SELECT alias_key FROM catalog_alias WHERE catalog_item_id = ?1")?;
            for k in a.query_map([&item], |r| r.get::<_, String>(0))? {
                out.insert((cat.clone(), k?));
            }
        }
    }
    Ok(out)
}

fn dismissed_keys(
    c: &Connection,
    source_id: &str,
    lineage: &str,
) -> AppResult<HashSet<(String, String)>> {
    let mut stmt = c.prepare("SELECT category, name_key FROM breakdown_dismissed WHERE source_id = ?1 AND scene_lineage_id = ?2")?;
    let rows = stmt
        .query_map(params![source_id, lineage], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<HashSet<_>, _>>()?;
    Ok(rows)
}

/// Store new candidates as pending suggestions (never production data).
fn insert_candidates(
    tx: &Tx<'_>,
    source: &ActiveSource,
    scene: &SceneRow,
    cands: &[Candidate],
) -> AppResult<i64> {
    let present = present_keys(tx.conn(), &scene.id)?;
    let dismissed = dismissed_keys(tx.conn(), &source.id, &scene.lineage_id)?;
    let mut added = 0;
    for cand in cands {
        let k = (cand.category.as_str().to_string(), cand.key.clone());
        if present.contains(&k) || dismissed.contains(&k) {
            continue;
        }
        let now = now_ms();
        tx.conn().execute(
            "INSERT INTO breakdown_element(id, source_id, scene_id, scene_lineage_id, category, display_name, source_evidence,
                                           confirmation_state, origin, span_element_id, span_start, span_end, confidence,
                                           created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Suggested', 'suggested', ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                new_id(),
                source.id,
                scene.id,
                scene.lineage_id,
                cand.category.as_str(),
                cand.name,
                cand.evidence,
                cand.element_id,
                cand.span.map(|s| s.0),
                cand.span.map(|s| s.1),
                cand.confidence,
                now
            ],
        )?;
        added += 1;
    }
    Ok(added)
}

fn source_scene(
    c: &Connection,
    scene_id: &str,
) -> AppResult<(ActiveSource, SourceScenes, SceneRow)> {
    let source = require_source(c)?;
    let scenes = SourceScenes::load(c, Some(&source.draft_id))?;
    let row = scenes.get(scene_id).cloned().ok_or_else(|| {
        AppError::new(
            "validation.scene_not_in_source",
            "This scene isn't part of the current Production Source. Its breakdown is kept for reference.",
        )
    })?;
    Ok((source, scenes, row))
}

fn scene_label(core: &AppCore, scene_id: &str) -> AppResult<String> {
    let s = core.project()?;
    s.store.read(|c| {
        let source = active_source(c)?;
        let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
        Ok(match scenes.get(scene_id) {
            Some(r) => format!("Scene {}", r.number),
            None => "a removed scene".to_string(),
        })
    })
}

fn suggest_op(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownSceneArgs,
) -> AppResult<BreakdownSuggestResult> {
    let label = scene_label(core, &args.scene_id)?;
    let s = core.project()?;
    let added = s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.suggest",
            format!("Suggested elements for {label}"),
            Capability::Edit,
        )
        .target("screenplay_scene", &args.scene_id),
        |tx| {
            let c = tx.conn();
            let (source, scenes, row) = source_scene(c, &args.scene_id)?;
            let els = script::scene_elements(c, &row.id)?;
            let heading = script::effective_heading(&row.heading, &els);
            let known = known_names(c, &scenes, &source.draft_id)?;
            let tuples: Vec<(String, String, String)> = els
                .into_iter()
                .map(|e| (e.id, e.element_type, e.text))
                .collect();
            let cands = candidates_for(&heading, &tuples, &known);
            insert_candidates(tx, &source, &row, &cands)
        },
    )?;
    let pending: i64 = s.store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM breakdown_element WHERE scene_id = ?1 AND deleted_at IS NULL AND confirmation_state = 'Suggested'",
            [&args.scene_id],
            |r| r.get(0),
        )?)
    })?;
    Ok(BreakdownSuggestResult { added, pending })
}

/// Accept one suggestion; returns the id of the production element.
fn accept_one(
    tx: &Tx<'_>,
    id: &str,
    category: Option<&str>,
    name: Option<&str>,
    choice: &BreakdownCatalogChoice,
) -> AppResult<String> {
    let row: Option<(String, String, String, String, String, String)> = tx
        .conn()
        .query_row(
            "SELECT scene_id, category, display_name, confirmation_state, source_id, scene_lineage_id
             FROM breakdown_element WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let (scene_id, orig_cat, display, state, source_id, lineage) =
        row.ok_or_else(|| AppError::not_found("suggestion"))?;
    if state != ConfirmationState::Suggested.as_str() {
        return Err(AppError::conflict("This suggestion was already handled."));
    }
    let cat = match category {
        Some(c) => parse_category(c)?,
        None => parse_category(&orig_cat)?,
    };
    let name = match name {
        Some(n) => required_text(n, "Name", 200)?,
        None => display.clone(),
    };
    let item = resolve_choice(tx, cat, &name, choice)?;
    if cat.as_str() != orig_cat || name_key(&name) != name_key(&display) {
        // Edited before accepting: the original suggestion is handled for this scene + source.
        remember_dismissed(tx, &source_id, &lineage, &orig_cat, &display)?;
    }
    let dup: Option<String> = tx
        .conn()
        .query_row(
            &format!(
                "SELECT id FROM breakdown_element WHERE scene_id = ?1 AND catalog_item_id = ?2 AND id <> ?3
                   AND deleted_at IS NULL AND confirmation_state IN {PRODUCTION_STATES} LIMIT 1"
            ),
            params![scene_id, item, id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(existing) = dup {
        // Already required by this scene: the suggestion is simply resolved.
        tx.conn()
            .execute("DELETE FROM breakdown_element WHERE id = ?1", [id])?;
        return Ok(existing);
    }
    update_fields(
        tx.conn(),
        "breakdown_element",
        id,
        &[
            (
                "confirmation_state",
                text(ConfirmationState::Confirmed.as_str()),
            ),
            ("category", text(cat.as_str())),
            ("display_name", text(name)),
            ("catalog_item_id", text(item)),
        ],
        &[
            "confirmation_state",
            "category",
            "display_name",
            "catalog_item_id",
        ],
        None,
        "suggestion",
    )?;
    ensure_state(tx, &scene_id)?;
    Ok(id.to_string())
}

fn suggestion_name(core: &AppCore, id: &str) -> AppResult<String> {
    let s = core.project()?;
    s.store.read(|c| {
        c.query_row(
            "SELECT display_name FROM breakdown_element WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("suggestion"))
    })
}

fn accept(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownAcceptArgs,
) -> AppResult<BreakdownElementDto> {
    let name = args
        .name
        .clone()
        .unwrap_or(suggestion_name(core, &args.id)?);
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.accept",
            format!("Accepted “{}”", name.trim()),
            Capability::Edit,
        )
        .target("breakdown_element", &args.id),
        |tx| {
            accept_one(
                tx,
                &args.id,
                args.category.as_deref(),
                args.name.as_deref(),
                &args.catalog,
            )
        },
    )?;
    s.store.read(|c| load_element(c, &id))
}

/// Batch accept (FSD §97): each suggestion uses its obvious catalog match or a
/// new item; ambiguous ones are returned for an explicit choice.
fn accept_many(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownIdsArgs,
) -> AppResult<BreakdownAcceptManyResult> {
    if args.ids.is_empty() {
        return Ok(BreakdownAcceptManyResult {
            accepted: 0,
            needs_choice: vec![],
        });
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.accept_many",
            format!("Accepted {} suggestions", args.ids.len()),
            Capability::Edit,
        ),
        |tx| {
            let mut accepted = 0;
            let mut needs_choice = Vec::new();
            for id in &args.ids {
                match accept_one(tx, id, None, None, &BreakdownCatalogChoice::default()) {
                    Ok(_) => accepted += 1,
                    Err(e) if e.code_str() == "validation.ambiguous_match" => {
                        needs_choice.push(id.clone())
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(BreakdownAcceptManyResult {
                accepted,
                needs_choice,
            })
        },
    )
}

fn remember_dismissed(
    tx: &Tx<'_>,
    source: &str,
    lineage: &str,
    category: &str,
    name: &str,
) -> AppResult<()> {
    let key = name_key(name);
    let exists: bool = tx.conn().query_row(
        "SELECT EXISTS(SELECT 1 FROM breakdown_dismissed WHERE source_id = ?1 AND scene_lineage_id = ?2 AND category = ?3 AND name_key = ?4)",
        params![source, lineage, category, key],
        |r| r.get(0),
    )?;
    if !exists {
        let now = now_ms();
        tx.conn().execute(
            "INSERT INTO breakdown_dismissed(id, source_id, scene_lineage_id, category, name_key, display_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![new_id(), source, lineage, category, key, name, now],
        )?;
    }
    Ok(())
}

/// Reject suggestions: they disappear from the scene and are not suggested
/// again for this scene + source. No production data is created (FSD §27.5).
fn reject(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownIdsArgs,
) -> AppResult<BreakdownCountResult> {
    if args.ids.is_empty() {
        return Ok(BreakdownCountResult { count: 0 });
    }
    let summary = if args.ids.len() == 1 {
        format!(
            "Rejected suggestion “{}”",
            suggestion_name(core, &args.ids[0])?
        )
    } else {
        format!("Dismissed {} suggestions", args.ids.len())
    };
    let s = core.project()?;
    s.store.mutate(actor, MutationMeta::new("breakdown.reject", summary, Capability::Edit), |tx| {
        let mut n = 0;
        for id in &args.ids {
            let row: Option<(String, String, String, String, String)> = tx
                .conn()
                .query_row(
                    "SELECT source_id, scene_lineage_id, category, display_name, confirmation_state FROM breakdown_element
                     WHERE id = ?1 AND deleted_at IS NULL",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .optional()?;
            let (source, lineage, cat, name, state) = row.ok_or_else(|| AppError::not_found("suggestion"))?;
            if state != ConfirmationState::Suggested.as_str() {
                return Err(AppError::conflict("Only suggestions can be rejected. Use Remove for confirmed elements."));
            }
            remember_dismissed(tx, &source, &lineage, &cat, &name)?;
            tx.conn().execute("DELETE FROM breakdown_element WHERE id = ?1", [id])?;
            n += 1;
        }
        Ok(BreakdownCountResult { count: n })
    })
}

// ------------------------------------------------------------- manual

/// UTF-16 slice of `text` (offsets from the UI selection).
fn utf16_slice(text: &str, start: u32, end: u32) -> Option<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (s, e) = (start as usize, end as usize);
    if s >= e || e > units.len() {
        return None;
    }
    String::from_utf16(&units[s..e]).ok()
}

fn add_element(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownAddArgs,
) -> AppResult<BreakdownElementDto> {
    let cat = parse_category(&args.category)?;
    let name = required_text(&args.name, "Name", 200)?;
    let notes = crate::util::optional_text(args.notes.clone(), "Notes", 4_000)?;
    let label = scene_label(core, &args.scene_id)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new("breakdown.add_element", format!("Added “{name}” to {label}"), Capability::Edit).target("screenplay_scene", &args.scene_id),
        |tx| {
            let c = tx.conn();
            let (source, _, row) = source_scene(c, &args.scene_id)?;
            let (evidence, span) = match &args.span {
                Some(sp) => {
                    let el: Option<String> = c
                        .query_row("SELECT text FROM screenplay_element WHERE id = ?1 AND scene_id = ?2", params![sp.element_id, row.id], |r| r.get(0))
                        .optional()?;
                    let el = el.ok_or_else(|| AppError::invalid_input("The selected text is not part of this scene."))?;
                    let ev = utf16_slice(&el, sp.start, sp.end).ok_or_else(|| AppError::invalid_input("The selected text changed. Select it again."))?;
                    (Some(ev.trim().to_string()), Some(sp.clone()))
                }
                None => (None, None),
            };
            let item = resolve_choice(tx, cat, &name, &args.catalog)?;
            let dup: bool = c.query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM breakdown_element WHERE scene_id = ?1 AND catalog_item_id = ?2
                       AND deleted_at IS NULL AND confirmation_state IN {PRODUCTION_STATES})"
                ),
                params![row.id, item],
                |r| r.get(0),
            )?;
            if dup {
                return Err(AppError::conflict(format!("“{name}” is already in this scene's breakdown.")));
            }
            // A pending suggestion for the same thing is resolved by this manual entry.
            let key = name_key(&name);
            let mut stmt = c.prepare(
                "SELECT id, display_name FROM breakdown_element WHERE scene_id = ?1 AND category = ?2
                   AND confirmation_state = 'Suggested' AND deleted_at IS NULL",
            )?;
            let pend: Vec<(String, String)> =
                stmt.query_map(params![row.id, cat.as_str()], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
            for (pid, pname) in pend {
                if name_key(&pname) == key {
                    c.execute("DELETE FROM breakdown_element WHERE id = ?1", [&pid])?;
                }
            }
            let id = new_id();
            let now = now_ms();
            c.execute(
                "INSERT INTO breakdown_element(id, source_id, scene_id, scene_lineage_id, category, catalog_item_id, display_name, notes,
                                               source_evidence, confirmation_state, origin, span_element_id, span_start, span_end,
                                               created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Manual', ?10, ?11, ?12, ?13, ?14, ?14)",
                params![
                    id,
                    source.id,
                    row.id,
                    row.lineage_id,
                    cat.as_str(),
                    item,
                    name,
                    notes,
                    evidence,
                    if span.is_some() { "tagged" } else { "manual" },
                    span.as_ref().map(|s| s.element_id.clone()),
                    span.as_ref().map(|s| s.start as i64),
                    span.as_ref().map(|s| s.end as i64),
                    now
                ],
            )?;
            ensure_state(tx, &row.id)?;
            Ok(id)
        },
    )?;
    s.store.read(|c| load_element(c, &id))
}

fn element_name(core: &AppCore, id: &str) -> AppResult<(String, String, String)> {
    let s = core.project()?;
    s.store.read(|c| {
        c.query_row(
            "SELECT display_name, scene_id, confirmation_state FROM breakdown_element WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("breakdown element"))
    })
}

fn update_element(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownUpdateArgs,
) -> AppResult<BreakdownElementDto> {
    let (name, _, _) = element_name(core, &args.id)?;
    let mut fields = Vec::new();
    if let Some(v) = patch_text(args.notes, "Notes", 4_000)? {
        fields.push(("notes", v));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.update_element",
            format!("Edited notes for “{name}”"),
            Capability::Edit,
        )
        .target("breakdown_element", &args.id)
        .coalesce(format!("breakdown.notes:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "breakdown_element",
                &args.id,
                &fields,
                &["notes"],
                None,
                "breakdown element",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load_element(c, &args.id))
}

/// Remove a scene association. The catalog item is never deleted (FSD §26.7).
fn remove_element(core: &AppCore, actor: &Actor, args: BreakdownIdArgs) -> AppResult<()> {
    let (name, scene_id, state) = element_name(core, &args.id)?;
    if state == ConfirmationState::Suggested.as_str() {
        return Err(AppError::invalid_input("Use Reject for suggestions."));
    }
    let label = scene_label(core, &scene_id)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.remove_element",
            format!("Removed “{name}” from {label}"),
            Capability::SoftDelete,
        )
        .target("breakdown_element", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "breakdown_element",
                    table: "breakdown_element",
                    id: &args.id,
                    title: Some(format!("{name} ({label})")),
                    parent_type: Some("screenplay_scene"),
                    parent_id: Some(scene_id.clone()),
                    position: None,
                },
            )
        },
    )
}

/// Archive (or restore) a historical element whose scene left the source (FSD §54).
fn set_archived(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownArchiveArgs,
) -> AppResult<BreakdownElementDto> {
    let (name, _, _) = element_name(core, &args.id)?;
    let s = core.project()?;
    let summary = if args.archived {
        format!("Archived “{name}” from a removed scene")
    } else {
        format!("Unarchived “{name}”")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("breakdown.set_archived", summary, Capability::Edit)
            .target("breakdown_element", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "breakdown_element",
                &args.id,
                &[("archived", int(args.archived as i64))],
                &["archived"],
                None,
                "breakdown element",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load_element(c, &args.id))
}

fn set_complete(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownCompleteArgs,
) -> AppResult<BreakdownSceneDetail> {
    let label = scene_label(core, &args.scene_id)?;
    let summary = if args.complete {
        format!("Marked {label} breakdown complete")
    } else {
        format!("Reopened {label} breakdown")
    };
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("breakdown.set_complete", summary, Capability::Edit)
            .target("screenplay_scene", &args.scene_id),
        |tx| {
            source_scene(tx.conn(), &args.scene_id)?;
            let lineage = ensure_state(tx, &args.scene_id)?;
            let id = state_id(tx.conn(), &lineage)?;
            update_fields(
                tx.conn(),
                "production_scene_state",
                &id,
                &[("complete", int(args.complete as i64))],
                &["complete"],
                None,
                "scene",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| build_detail(c, &args.scene_id))
}

/// "Keep Production Data": the current script text becomes the baseline.
fn mark_reviewed_tx(tx: &Tx<'_>, scene_id: &str) -> AppResult<()> {
    let (_, _, row) = source_scene(tx.conn(), scene_id)?;
    let lineage = ensure_state(tx, scene_id)?;
    let id = state_id(tx.conn(), &lineage)?;
    let (heading, enc) = current_encoded(tx.conn(), scene_id, &row.heading)?;
    let draft: String = tx.conn().query_row(
        "SELECT draft_id FROM screenplay_scene WHERE id = ?1",
        [scene_id],
        |r| r.get(0),
    )?;
    update_fields(
        tx.conn(),
        "production_scene_state",
        &id,
        &[
            ("baseline_heading", text(heading)),
            ("baseline_fingerprint", text(script::fingerprint(&enc))),
            ("baseline_text", text(enc)),
            ("baseline_draft_id", text(draft)),
            ("reviewed_at", int(now_ms())),
        ],
        &[
            "baseline_heading",
            "baseline_fingerprint",
            "baseline_text",
            "baseline_draft_id",
            "reviewed_at",
        ],
        None,
        "scene",
    )?;
    Ok(())
}

fn mark_reviewed(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownSceneArgs,
) -> AppResult<BreakdownSceneDetail> {
    let label = scene_label(core, &args.scene_id)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.mark_reviewed",
            format!("Reviewed script changes in {label}"),
            Capability::Edit,
        )
        .target("screenplay_scene", &args.scene_id),
        |tx| mark_reviewed_tx(tx, &args.scene_id),
    )?;
    s.store.read(|c| build_detail(c, &args.scene_id))
}

// ------------------------------------------------------ script changes

struct ChangeAnalysis {
    removed: Vec<Candidate>,
    added: Vec<Candidate>,
}

fn analyse_changes(
    c: &Connection,
    scene_id: &str,
) -> AppResult<(SceneRow, Option<String>, String, ChangeAnalysis)> {
    let (source, scenes, row) = source_scene(c, scene_id)?;
    let known = known_names(c, &scenes, &source.draft_id)?;
    let els = script::scene_elements(c, scene_id)?;
    let heading = script::effective_heading(&row.heading, &els);
    let cur: Vec<(String, String, String)> = els
        .into_iter()
        .map(|e| (e.id, e.element_type, e.text))
        .collect();
    let new_c = candidates_for(&heading, &cur, &known);
    let base: Option<(Option<String>, Option<String>)> = c
        .query_row(
            "SELECT baseline_heading, baseline_text FROM production_scene_state WHERE scene_lineage_id = ?1",
            [&row.lineage_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (base_heading, base_text) = base.unwrap_or((None, None));
    let old_c = match &base_text {
        Some(t) => {
            let (h, els) = script::decode_scene(t);
            let tuples: Vec<(String, String, String)> = els
                .into_iter()
                .enumerate()
                .map(|(i, (ty, tx))| (format!("baseline-{i}"), ty, tx))
                .collect();
            candidates_for(&h, &tuples, &known)
        }
        None => new_c.clone(),
    };
    let key = |c: &Candidate| (c.category, c.key.clone());
    let old_keys: HashSet<_> = old_c.iter().map(key).collect();
    let new_keys: HashSet<_> = new_c.iter().map(key).collect();
    let removed = old_c
        .iter()
        .filter(|c| !new_keys.contains(&key(c)))
        .cloned()
        .collect();
    let added = new_c
        .iter()
        .filter(|c| !old_keys.contains(&key(c)))
        .cloned()
        .collect();
    Ok((
        row,
        base_heading,
        heading,
        ChangeAnalysis { removed, added },
    ))
}

/// Visual planning that exists for a scene, probed generically so this module
/// does not depend on the visual module's schema ("Shot list exists: Yes").
fn planning_notes(c: &Connection, scene_id: &str, lineage: &str) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND (name LIKE 'shot%' OR name LIKE 'storyboard%') ORDER BY name",
    )?;
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut notes: Vec<String> = Vec::new();
    for raw in tables {
        // Names come from sqlite_master (a received project can carry any schema): quote them.
        let t = raw.replace('"', "\"\"");
        let mut cs = c.prepare(&format!(
            "SELECT name FROM pragma_table_info('{}')",
            raw.replace('\'', "''")
        ))?;
        let cols: Vec<String> = cs.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        let live = if cols.iter().any(|c| c == "deleted_at") {
            " AND deleted_at IS NULL"
        } else {
            ""
        };
        let found = if cols.iter().any(|c| c == "scene_lineage_id") {
            c.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM \"{t}\" WHERE scene_lineage_id = ?1{live})"),
                [lineage],
                |r| r.get::<_, bool>(0),
            )?
        } else if cols.iter().any(|c| c == "scene_id") {
            c.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM \"{t}\" WHERE scene_id = ?1{live})"),
                [scene_id],
                |r| r.get::<_, bool>(0),
            )?
        } else {
            false
        };
        if found {
            let label = if t.starts_with("shot") {
                "Shot list exists: Yes — review recommended"
            } else {
                "Storyboard exists: Yes — review recommended"
            };
            if !notes.iter().any(|n| n == label) {
                notes.push(label.to_string());
            }
        }
    }
    Ok(notes)
}

fn scene_changes(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownSceneArgs,
) -> AppResult<BreakdownSceneChanges> {
    actor.require(Capability::View, "view the breakdown")?;
    let s = core.project()?;
    s.store.read(|c| {
        let (row, base_heading, heading, a) = analyse_changes(c, &args.scene_id)?;
        let detail = build_detail(c, &args.scene_id)?;
        let source = require_source(c)?;
        let present = present_keys(c, &row.id)?;
        let dismissed = dismissed_keys(c, &source.id, &row.lineage_id)?;
        let new_suggestions = a
            .added
            .iter()
            .filter(|cand| {
                let k = (cand.category.as_str().to_string(), cand.key.clone());
                !present.contains(&k) && !dismissed.contains(&k)
            })
            .count() as i64;
        let removed_keys: HashSet<(String, String)> = a
            .removed
            .iter()
            .map(|c| (c.category.as_str().to_string(), c.key.clone()))
            .collect();
        let elements_to_review: Vec<BreakdownElementDto> = detail
            .groups
            .iter()
            .flat_map(|g| g.confirmed.iter())
            .filter(|e| {
                removed_keys.contains(&(e.category.as_str().to_string(), name_key(&e.display_name)))
                    || removed_keys.contains(&(e.category.as_str().to_string(), name_key(&e.name)))
            })
            .cloned()
            .collect();
        let dto = |c: &Candidate| BreakdownCandidateDto {
            category: c.category,
            name: c.name.clone(),
            evidence: c.evidence.clone(),
        };
        Ok(BreakdownSceneChanges {
            scene_id: row.id.clone(),
            number: Some(row.number.clone()),
            heading,
            baseline_heading: base_heading,
            heading_changed: detail.heading_changed,
            text_changed: detail.text_changed,
            removed_candidates: a.removed.iter().map(dto).collect(),
            added_candidates: a.added.iter().map(dto).collect(),
            new_suggestions,
            elements_to_review,
            planning_notes: planning_notes(c, &row.id, &row.lineage_id)?,
        })
    })
}

/// "Apply Suggested Update": new candidates become pending suggestions and the
/// current text becomes the baseline. Existing elements are never removed.
fn apply_suggested_update(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownSceneArgs,
) -> AppResult<BreakdownSuggestResult> {
    let label = scene_label(core, &args.scene_id)?;
    let s = core.project()?;
    let added = s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.apply_suggested_update",
            format!("Reviewed script changes in {label}"),
            Capability::Edit,
        )
        .target("screenplay_scene", &args.scene_id),
        |tx| {
            let (row, _, _, a) = analyse_changes(tx.conn(), &args.scene_id)?;
            let source = require_source(tx.conn())?;
            let n = insert_candidates(tx, &source, &row, &a.added)?;
            mark_reviewed_tx(tx, &args.scene_id)?;
            Ok(n)
        },
    )?;
    let pending: i64 = s.store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM breakdown_element WHERE scene_id = ?1 AND deleted_at IS NULL AND confirmation_state = 'Suggested'",
            [&args.scene_id],
            |r| r.get(0),
        )?)
    })?;
    Ok(BreakdownSuggestResult { added, pending })
}

fn purge_element(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM breakdown_element WHERE id = ?1",
        [&row.object_id],
    )?;
    Ok(())
}
