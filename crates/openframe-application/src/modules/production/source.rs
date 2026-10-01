//! Production Source selection, "Update Production Source" reconciliation and
//! the derived Production Overview (FSD §53–54, §108, §160–161; Domain §10).
//!
//! - The user chooses a named draft; the choice is recorded with who/when/why.
//! - A newer draft never silently replaces the source: it is only reported
//!   ("Newer revision available — Review Production Update").
//! - Updating compares old and new drafts by stable scene identity
//!   (`lineage_id`). Unambiguous matches carry their breakdown to the new scene;
//!   removed or ambiguous scenes keep their breakdown as historical. Nothing is
//!   ever deleted.

use std::collections::HashMap;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::breakdown::{insert_state_if_missing, scene_statuses};
use super::script::{self, ProductionDraftInfo, SceneRow};
use super::{PRODUCTION_STATES, ProductionSceneRef, SourceScenes, active_source, require_source};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{MutationMeta, Tx};
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Production");
    r.query("production.drafts", drafts)
        .meta(M::read("Drafts that can become the Production Source."));
    r.query("production.source", source)
        .meta(M::read("The active Production Source draft."));
    r.command("production.set_source", set_source).meta(
        M::edit("Choose the Production Source draft (production follows this draft).").confirm(),
    );
    r.query("production.preview_update", preview_update)
        .meta(M::compute(
            "Preview of updating Production to a newer draft (scenes added, changed, removed).",
        ));
    r.command("production.apply_update", apply_update).meta(
        M::edit("Update Production to a newer draft (breakdown carried by scene identity).")
            .confirm(),
    );
    r.query("production.overview", overview).meta(M::compute("Production overview: source draft, breakdown progress, catalog, locations, cast and crew counts."));
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSourceInfo {
    pub source_id: String,
    pub draft: ProductionDraftInfo,
    #[ts(type = "number")]
    pub selected_at: i64,
    pub selected_by: Option<String>,
    pub reason: Option<String>,
    /// A newer draft of the same screenplay (informational until the user acts).
    pub newer: Option<ProductionDraftInfo>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSourceOptions {
    pub drafts: Vec<ProductionDraftInfo>,
    pub active_draft_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSceneChange {
    pub lineage_id: String,
    pub old_scene_id: Option<String>,
    pub new_scene_id: Option<String>,
    pub old_number: Option<String>,
    pub new_number: Option<String>,
    pub heading: String,
    pub old_heading: Option<String>,
    pub heading_changed: bool,
    pub text_changed: bool,
    pub moved: bool,
    /// "added" | "removed" | "changed" | "moved" | "ambiguous"
    pub status: String,
    /// Production breakdown elements attached to this scene identity.
    #[ts(type = "number")]
    pub element_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionUpdatePreview {
    pub from: ProductionDraftInfo,
    pub to: ProductionDraftInfo,
    #[ts(type = "number")]
    pub added: i64,
    #[ts(type = "number")]
    pub removed: i64,
    #[ts(type = "number")]
    pub text_changed: i64,
    #[ts(type = "number")]
    pub heading_changed: i64,
    #[ts(type = "number")]
    pub moved: i64,
    #[ts(type = "number")]
    pub unchanged: i64,
    #[ts(type = "number")]
    pub ambiguous: i64,
    /// Every scene that is not unchanged, in new-script order then removed scenes.
    pub changes: Vec<ProductionSceneChange>,
    /// Production breakdown elements that stay (nothing is deleted).
    #[ts(type = "number")]
    pub elements_kept: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionUpdateResult {
    pub source: ProductionSourceInfo,
    #[ts(type = "number")]
    pub mapped_scenes: i64,
    #[ts(type = "number")]
    pub added_scenes: i64,
    #[ts(type = "number")]
    pub historical_scenes: i64,
    #[ts(type = "number")]
    pub review_scenes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionOverview {
    pub source: Option<ProductionSourceInfo>,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub complete_count: i64,
    /// Scenes with at least one confirmed element.
    #[ts(type = "number")]
    pub planned_count: i64,
    #[ts(type = "number")]
    pub pending_suggestions: i64,
    pub needs_review: Vec<ProductionSceneRef>,
    pub needs_breakdown: Vec<ProductionSceneRef>,
    #[ts(type = "number")]
    pub historical_elements: i64,
    #[ts(type = "number")]
    pub location_count: i64,
    /// Still Idea or Shortlisted.
    #[ts(type = "number")]
    pub unresolved_locations: i64,
    #[ts(type = "number")]
    pub character_count: i64,
    /// Characters without a primary actor.
    pub unresolved_characters: Vec<String>,
    #[ts(type = "number")]
    pub catalog_count: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionEmptyArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionSetSourceArgs {
    pub draft_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionDraftArgs {
    pub draft_id: String,
}

// ------------------------------------------------------------------ reads

#[allow(clippy::type_complexity)]
pub(crate) fn source_info(c: &Connection) -> AppResult<Option<ProductionSourceInfo>> {
    let row: Option<(String, String, i64, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT id, draft_id, selected_at, selected_by, selection_reason FROM production_source
             WHERE active = 1 ORDER BY selected_at DESC, id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((id, draft_id, selected_at, selected_by, reason)) = row else {
        return Ok(None);
    };
    let Some(draft) = script::draft_info(c, &draft_id)? else {
        return Ok(None);
    };
    let newer = script::newer_draft(c, &draft)?;
    Ok(Some(ProductionSourceInfo {
        source_id: id,
        draft,
        selected_at,
        selected_by,
        reason,
        newer,
    }))
}

fn drafts(
    core: &AppCore,
    actor: &Actor,
    _: ProductionEmptyArgs,
) -> AppResult<ProductionSourceOptions> {
    actor.require(Capability::View, "view production")?;
    let s = core.project()?;
    s.store.read(|c| {
        Ok(ProductionSourceOptions {
            drafts: script::list_drafts(c)?,
            active_draft_id: active_source(c)?.map(|a| a.draft_id),
        })
    })
}

fn source(
    core: &AppCore,
    actor: &Actor,
    _: ProductionEmptyArgs,
) -> AppResult<Option<ProductionSourceInfo>> {
    actor.require(Capability::View, "view production")?;
    let s = core.project()?;
    s.store.read(source_info)
}

fn live_draft(c: &Connection, draft_id: &str) -> AppResult<ProductionDraftInfo> {
    match script::draft_info(c, draft_id)? {
        Some(d) if !d.deleted => Ok(d),
        _ => Err(AppError::not_found("draft")),
    }
}

// ------------------------------------------------------------ set source

fn insert_source(
    tx: &Tx<'_>,
    draft_id: &str,
    reason: Option<String>,
    previous: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO production_source(id, draft_id, active, selection_reason, selected_by, selected_at, previous_source_id, created_at, updated_at)
         VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?5, ?5)",
        params![id, draft_id, reason, tx.actor().display_name, now, previous],
    )?;
    Ok(id)
}

/// Baselines for every scene of a draft that has no production state yet.
fn init_states(tx: &Tx<'_>, draft_id: &str, needs_breakdown: bool) -> AppResult<()> {
    let scenes = script::draft_scenes(tx.conn(), draft_id)?;
    let els = script::draft_elements(tx.conn(), draft_id)?;
    for s in scenes {
        let e = els.get(&s.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let heading = script::effective_heading(&s.heading, e);
        let enc = script::encode_scene(&heading, e);
        insert_state_if_missing(tx, &s.lineage_id, draft_id, &heading, &enc, needs_breakdown)?;
    }
    Ok(())
}

/// First selection of a Production Source (FSD §53; mock 103). The screenplay is not changed.
fn set_source(
    core: &AppCore,
    actor: &Actor,
    args: ProductionSetSourceArgs,
) -> AppResult<ProductionSourceInfo> {
    let reason = optional_text(args.reason, "Reason", 1_000)?;
    let s = core.project()?;
    let draft = s.store.read(|c| live_draft(c, &args.draft_id))?;
    s.store.mutate(
        actor,
        MutationMeta::new("production.set_source", format!("Set “{}” as the Production Source", draft.name), Capability::Edit)
            .target("screenplay_draft", &draft.id),
        |tx| {
            if let Some(active) = active_source(tx.conn())? {
                let name = script::draft_info(tx.conn(), &active.draft_id)?.map(|d| d.name).unwrap_or_default();
                if active.draft_id == args.draft_id {
                    return Err(AppError::conflict(format!("“{name}” is already the Production Source.")));
                }
                return Err(AppError::new(
                    "conflict.production_source_set",
                    format!("Production already uses “{name}”. Review the production update first — nothing changes until you confirm."),
                ));
            }
            insert_source(tx, &args.draft_id, reason.clone(), None)?;
            init_states(tx, &args.draft_id, false)?;
            Ok(())
        },
    )?;
    s.store.read(|c| {
        source_info(c)?.ok_or_else(|| AppError::internal("source missing after selection"))
    })
}

// ------------------------------------------------------- reconciliation

struct DiffScene {
    row: SceneRow,
    heading: String,
    body: String,
}

fn diff_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<DiffScene>> {
    let els = script::draft_elements(c, draft_id)?;
    Ok(script::draft_scenes(c, draft_id)?
        .into_iter()
        .map(|row| {
            let e = els.get(&row.id).map(|v| v.as_slice()).unwrap_or(&[]);
            let heading = script::effective_heading(&row.heading, e);
            let enc = script::encode_scene(&heading, e);
            let body = enc
                .split_once('\n')
                .map(|(_, b)| b.to_string())
                .unwrap_or_default();
            DiffScene { row, heading, body }
        })
        .collect())
}

fn norm_heading(h: &str) -> String {
    h.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
        .replace(['—', '–'], "-")
}

/// Longest common subsequence of two lineage sequences (for "moved" scenes).
fn lcs(a: &[&str], b: &[&str]) -> std::collections::HashSet<String> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = std::collections::HashSet::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.insert(a[i].to_string());
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

struct Reconciliation {
    changes: Vec<ProductionSceneChange>,
    unchanged: i64,
    /// lineage → new scene (unambiguous matches, including unchanged).
    mapping: Vec<(String, String)>,
    /// lineages absent from the old draft (new scene id).
    added: Vec<String>,
    removed: i64,
}

fn reconcile(c: &Connection, old_draft: &str, new_draft: &str) -> AppResult<Reconciliation> {
    let old = diff_scenes(c, old_draft)?;
    let new = diff_scenes(c, new_draft)?;
    let mut old_by: HashMap<&str, Vec<&DiffScene>> = HashMap::new();
    for s in &old {
        old_by.entry(s.row.lineage_id.as_str()).or_default().push(s);
    }
    let mut new_by: HashMap<&str, Vec<&DiffScene>> = HashMap::new();
    for s in &new {
        new_by.entry(s.row.lineage_id.as_str()).or_default().push(s);
    }
    let mut counts: HashMap<String, i64> = HashMap::new();
    let mut stmt = c.prepare(&format!(
        "SELECT scene_lineage_id, count(*) FROM breakdown_element WHERE deleted_at IS NULL AND confirmation_state IN {PRODUCTION_STATES}
         GROUP BY scene_lineage_id"
    ))?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (k, v) = row?;
        counts.insert(k, v);
    }
    let unique = |m: &HashMap<&str, Vec<&DiffScene>>, l: &str| {
        m.get(l).map(|v| v.len() == 1).unwrap_or(false)
    };
    let old_seq: Vec<&str> = old
        .iter()
        .map(|s| s.row.lineage_id.as_str())
        .filter(|l| unique(&old_by, l) && unique(&new_by, l))
        .collect();
    let new_seq: Vec<&str> = new
        .iter()
        .map(|s| s.row.lineage_id.as_str())
        .filter(|l| unique(&old_by, l) && unique(&new_by, l))
        .collect();
    let stable = lcs(&old_seq, &new_seq);

    let mut rec = Reconciliation {
        changes: vec![],
        unchanged: 0,
        mapping: vec![],
        added: vec![],
        removed: 0,
    };
    let mut seen_ambiguous: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for s in &new {
        let l = s.row.lineage_id.as_str();
        let olds = old_by.get(l).cloned().unwrap_or_default();
        let element_count = counts.get(l).copied().unwrap_or(0);
        if olds.is_empty() {
            rec.added.push(s.row.id.clone());
            rec.changes.push(ProductionSceneChange {
                lineage_id: l.to_string(),
                old_scene_id: None,
                new_scene_id: Some(s.row.id.clone()),
                old_number: None,
                new_number: Some(s.row.number.clone()),
                heading: s.heading.clone(),
                old_heading: None,
                heading_changed: false,
                text_changed: false,
                moved: false,
                status: "added".into(),
                element_count,
            });
            continue;
        }
        if olds.len() > 1 || !unique(&new_by, l) {
            if seen_ambiguous.insert(l) {
                rec.changes.push(ProductionSceneChange {
                    lineage_id: l.to_string(),
                    old_scene_id: olds.first().map(|o| o.row.id.clone()),
                    new_scene_id: Some(s.row.id.clone()),
                    old_number: olds.first().map(|o| o.row.number.clone()),
                    new_number: Some(s.row.number.clone()),
                    heading: s.heading.clone(),
                    old_heading: olds.first().map(|o| o.heading.clone()),
                    heading_changed: false,
                    text_changed: false,
                    moved: false,
                    status: "ambiguous".into(),
                    element_count,
                });
            }
            continue;
        }
        let o = olds[0];
        let heading_changed = norm_heading(&o.heading) != norm_heading(&s.heading);
        let text_changed = o.body != s.body;
        let moved = !stable.contains(l);
        rec.mapping.push((l.to_string(), s.row.id.clone()));
        if !heading_changed && !text_changed && !moved {
            rec.unchanged += 1;
            continue;
        }
        rec.changes.push(ProductionSceneChange {
            lineage_id: l.to_string(),
            old_scene_id: Some(o.row.id.clone()),
            new_scene_id: Some(s.row.id.clone()),
            old_number: Some(o.row.number.clone()),
            new_number: Some(s.row.number.clone()),
            heading: s.heading.clone(),
            old_heading: if heading_changed {
                Some(o.heading.clone())
            } else {
                None
            },
            heading_changed,
            text_changed,
            moved,
            status: if heading_changed || text_changed {
                "changed".into()
            } else {
                "moved".into()
            },
            element_count,
        });
    }
    for o in &old {
        let l = o.row.lineage_id.as_str();
        if new_by.contains_key(l) {
            continue;
        }
        rec.removed += 1;
        rec.changes.push(ProductionSceneChange {
            lineage_id: l.to_string(),
            old_scene_id: Some(o.row.id.clone()),
            new_scene_id: None,
            old_number: Some(o.row.number.clone()),
            new_number: None,
            heading: o.heading.clone(),
            old_heading: None,
            heading_changed: false,
            text_changed: false,
            moved: false,
            status: "removed".into(),
            element_count: counts.get(l).copied().unwrap_or(0),
        });
    }
    Ok(rec)
}

fn preview(c: &Connection, draft_id: &str) -> AppResult<ProductionUpdatePreview> {
    let active = require_source(c)?;
    let from =
        script::draft_info(c, &active.draft_id)?.ok_or_else(|| AppError::not_found("draft"))?;
    let to = live_draft(c, draft_id)?;
    if from.id == to.id {
        return Err(AppError::conflict(format!(
            "“{}” is already the Production Source.",
            to.name
        )));
    }
    let rec = reconcile(c, &from.id, &to.id)?;
    let count = |f: &dyn Fn(&ProductionSceneChange) -> bool| {
        rec.changes.iter().filter(|c| f(c)).count() as i64
    };
    let elements_kept: i64 = c.query_row(
        &format!("SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL AND confirmation_state IN {PRODUCTION_STATES}"),
        [],
        |r| r.get(0),
    )?;
    Ok(ProductionUpdatePreview {
        added: count(&|c| c.status == "added"),
        removed: count(&|c| c.status == "removed"),
        text_changed: count(&|c| c.text_changed),
        heading_changed: count(&|c| c.heading_changed),
        moved: count(&|c| c.moved),
        ambiguous: count(&|c| c.status == "ambiguous"),
        unchanged: rec.unchanged,
        changes: rec.changes,
        elements_kept,
        from,
        to,
    })
}

fn preview_update(
    core: &AppCore,
    actor: &Actor,
    args: ProductionDraftArgs,
) -> AppResult<ProductionUpdatePreview> {
    actor.require(Capability::View, "view production")?;
    let s = core.project()?;
    s.store.read(|c| preview(c, &args.draft_id))
}

/// "Update Production Baseline" (mock 111): the new draft becomes the source;
/// unambiguous scene identities carry their breakdown; everything else is
/// kept as history and flagged. Never deletes production data.
fn apply_update(
    core: &AppCore,
    actor: &Actor,
    args: ProductionSetSourceArgs,
) -> AppResult<ProductionUpdateResult> {
    let reason = optional_text(args.reason, "Reason", 1_000)?;
    let s = core.project()?;
    let (from, to) = s.store.read(|c| {
        let p = preview(c, &args.draft_id)?;
        Ok((p.from, p.to))
    })?;
    let (mapped, added, historical, review) = s.store.mutate(
        actor,
        MutationMeta::new("production.apply_update", format!("Updated Production Source from “{}” to “{}”", from.name, to.name), Capability::Edit)
            .target("screenplay_draft", &to.id),
        |tx| {
            let c = tx.conn();
            let active = require_source(c)?;
            let rec = reconcile(c, &active.draft_id, &args.draft_id)?;
            let now = now_ms();
            c.execute("UPDATE production_source SET active = 0, updated_at = ?1, rev = rev + 1 WHERE active = 1", [now])?;
            let new_source = insert_source(tx, &args.draft_id, reason.clone(), Some(&active.id))?;
            // Baselines for scenes that have none: the OLD text, so real changes show as "Needs Review".
            let old_els = script::draft_elements(c, &active.draft_id)?;
            for o in script::draft_scenes(c, &active.draft_id)? {
                let e = old_els.get(&o.id).map(|v| v.as_slice()).unwrap_or(&[]);
                let heading = script::effective_heading(&o.heading, e);
                insert_state_if_missing(tx, &o.lineage_id, &active.draft_id, &heading, &script::encode_scene(&heading, e), false)?;
            }
            // Carry breakdown rows (any source) to the matching scene of the new draft.
            let mut mapped = 0i64;
            for (lineage, scene) in &rec.mapping {
                let n = c.execute(
                    "UPDATE breakdown_element SET source_id = ?1, scene_id = ?2, updated_at = ?3, rev = rev + 1
                     WHERE scene_lineage_id = ?4 AND deleted_at IS NULL AND (source_id <> ?1 OR scene_id <> ?2)",
                    params![new_source, scene, now, lineage],
                )?;
                if n > 0 {
                    mapped += 1;
                }
            }
            // New scenes: "Needs Breakdown".
            init_states(tx, &args.draft_id, true)?;
            let review = rec.changes.iter().filter(|c| c.status == "changed" && c.element_count > 0).count() as i64;
            Ok((mapped, rec.added.len() as i64, rec.removed, review))
        },
    )?;
    let source = s.store.read(|c| {
        source_info(c)?.ok_or_else(|| AppError::internal("source missing after update"))
    })?;
    Ok(ProductionUpdateResult {
        source,
        mapped_scenes: mapped,
        added_scenes: added,
        historical_scenes: historical,
        review_scenes: review,
    })
}

// ------------------------------------------------------------- overview

fn overview(
    core: &AppCore,
    actor: &Actor,
    _: ProductionEmptyArgs,
) -> AppResult<ProductionOverview> {
    actor.require(Capability::View, "view production")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let info = source_info(c)?;
        let active = active_source(c)?;
        let scenes = SourceScenes::load(c, active.as_ref().map(|a| a.draft_id.as_str()))?;
        let statuses = match &active {
            Some(a) => scene_statuses(c, a, &scenes)?,
            None => vec![],
        };
        let r = |st: &super::breakdown::SceneStatus| ProductionSceneRef {
            scene_id: st.row.id.clone(),
            number: Some(st.row.number.clone()),
            heading: st.heading.clone(),
            in_source: true,
        };
        let historical_elements: i64 = {
            let mut stmt = c.prepare(&format!(
                "SELECT scene_id FROM breakdown_element WHERE deleted_at IS NULL AND archived = 0 AND confirmation_state IN {PRODUCTION_STATES}"
            ))?;
            let ids: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
            ids.iter().filter(|id| scenes.get(id).is_none()).count() as i64
        };
        let (location_count, unresolved_locations): (i64, i64) = c.query_row(
            "SELECT count(*), COALESCE(SUM(status IN ('Idea','Shortlisted')), 0) FROM location WHERE deleted_at IS NULL AND archived = 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let catalog_count: i64 =
            c.query_row("SELECT count(*) FROM catalog_item WHERE deleted_at IS NULL AND archived = 0", [], |r| r.get(0))?;
        let dir = super::people::directory(c, &root, false)?;
        Ok(ProductionOverview {
            source: info,
            scene_count: statuses.len() as i64,
            complete_count: statuses.iter().filter(|s| s.complete).count() as i64,
            planned_count: statuses.iter().filter(|s| s.confirmed > 0).count() as i64,
            pending_suggestions: statuses.iter().map(|s| s.suggested).sum(),
            needs_review: statuses.iter().filter(|s| s.needs_review).map(r).collect(),
            needs_breakdown: statuses.iter().filter(|s| s.needs_breakdown).map(r).collect(),
            historical_elements,
            location_count,
            unresolved_locations,
            character_count: dir.characters.len() as i64,
            unresolved_characters: dir.characters.iter().filter(|ch| ch.primary_cast_id.is_none()).map(|ch| ch.name.clone()).collect(),
            catalog_count,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::lcs;

    #[test]
    fn lcs_finds_stable_order() {
        let s = lcs(&["a", "b", "c", "d"], &["a", "c", "b", "d"]);
        assert!(s.contains("a") && s.contains("d"));
        assert_eq!(s.len(), 3);
    }
}
