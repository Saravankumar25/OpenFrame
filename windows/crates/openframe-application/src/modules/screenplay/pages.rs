//! Page facts, the editor's page view and the locked pagination baseline
//! (Screenplay and Production Depth Specification §9.4, §10, §11; ADR-0015
//! decision 4; contract doc 32 §2.3).
//!
//! Everything here comes from the one layout engine
//! (`openframe_import_export::layout`) with the screenplay's page setup, so the
//! page view in the editor, the print preview and the PDF show the same pages.
//!
//! Lock Shooting Draft records a baseline (`screenplay_lock_baseline`): the
//! page setup used and a fingerprint of every page and every scene. A revision
//! started from the locked draft is laid out with the same settings and
//! compared with it: pages whose text differs, pages beyond the baseline, and
//! scenes changed, added, omitted or removed. The fingerprints ignore page
//! numbers, headers, footers and revision asterisks, so only content counts.

use std::collections::HashMap;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_import_export::layout::{self, LaidOutPage, LineStyle};
use openframe_import_export::{PageSetup, ScreenplayDoc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{ScreenplayDraftIdArgs, ScreenplayTitlePage, load_draft, load_screenplay};
use crate::core::AppCore;
use crate::modules::interchange::draft::{self as idraft, DraftInfo, LoadedScene};
use crate::modules::interchange::export::{InterchangePrintPreview, print_preview};
use crate::registry::Registry;
use crate::store::Tx;
use crate::util::require_id;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("screenplay.pagination", pagination)
        .meta(M::compute(
            "Page count of a draft and the pages each scene is on, with its page setup.",
        ));
    r.query("screenplay.page_view", page_view).meta(
        M::compute("The pages of a draft as they print, for the editor's page view.")
            .hidden(h::UI_FLOW),
    );
    r.query("screenplay.title_page_preview", title_page_preview)
        .meta(M::compute("A preview of a title page while it is being edited.").hidden(h::UI_FLOW));
    r.query("screenplay.revision_pages", revision_pages)
        .meta(M::compute(
            "Pages and scenes a revision changed compared with the locked shooting draft.",
        ));
}

// ------------------------------------------------------------------ helpers

/// FNV-1a 64-bit: a stable, dependency-free fingerprint (not a security hash).
fn fingerprint<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in p.as_bytes().iter().chain(std::iter::once(&0x1f)) {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

fn page_fingerprint(page: &LaidOutPage) -> String {
    let lines: Vec<String> = page
        .lines
        .iter()
        .filter(|l| {
            !matches!(
                l.style,
                LineStyle::PageNumber
                    | LineStyle::Header
                    | LineStyle::Footer
                    | LineStyle::RevisionMark
                    | LineStyle::SceneNumber
            )
        })
        .map(|l| format!("{}|{}|{}", l.row, l.style.as_str(), l.text))
        .collect();
    fingerprint(lines.iter().map(String::as_str))
}

fn scene_fingerprint(s: &LoadedScene) -> String {
    let mut parts: Vec<String> = vec![s.heading.trim().to_string(), s.omitted.to_string()];
    for e in &s.elements {
        parts.push(format!(
            "{}|{}|{}",
            e.kind.as_str(),
            e.custom
                .as_ref()
                .map(|c| c.export_name.as_str())
                .unwrap_or(""),
            e.text
        ));
    }
    fingerprint(parts.iter().map(String::as_str))
}

/// A draft laid out with a page setup (no title page; numbers shown as labels).
fn draft_layout(
    c: &Connection,
    draft_id: &str,
    page: Option<PageSetup>,
) -> AppResult<(DraftInfo, Vec<LoadedScene>, ScreenplayDoc, layout::Layout)> {
    let info = idraft::load_draft_info(c, draft_id)?;
    let scenes = idraft::load_scenes(c, draft_id)?;
    let doc = idraft::build_doc(&info, &scenes, None, true);
    let mut opts = idraft::screen_layout(&info);
    opts.title_page = false;
    if let Some(p) = page {
        opts.page = p;
    }
    let lay = layout::layout(&doc, &opts);
    Ok((info, scenes, doc, lay))
}

// ------------------------------------------------------------------ pagination

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayScenePages {
    pub scene_id: String,
    pub label: String,
    pub heading: String,
    pub first_page: u32,
    pub last_page: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayPagination {
    pub draft_id: String,
    /// Script pages (title page not counted).
    pub page_count: u32,
    pub first_page_number: u32,
    /// "US Letter" or "A4".
    pub paper: String,
    pub lines_per_page: u32,
    pub scenes: Vec<ScreenplayScenePages>,
}

pub(crate) fn pagination_for(c: &Connection, draft_id: &str) -> AppResult<ScreenplayPagination> {
    let (info, scenes, _, lay) = draft_layout(c, draft_id, None)?;
    Ok(ScreenplayPagination {
        draft_id: draft_id.to_string(),
        page_count: lay.script_page_count() as u32,
        first_page_number: info.page_setup.first_page_number.max(1),
        paper: info.page_setup.engine().paper.label().into(),
        lines_per_page: lay.geometry.lines_per_page as u32,
        scenes: lay
            .scenes
            .iter()
            .filter_map(|sp| {
                scenes.get(sp.scene).map(|s| ScreenplayScenePages {
                    scene_id: s.id.clone(),
                    label: s.label.clone(),
                    heading: if s.omitted {
                        "OMITTED".into()
                    } else {
                        s.heading.clone()
                    },
                    first_page: sp.first_page,
                    last_page: sp.last_page,
                })
            })
            .collect(),
    })
}

fn pagination(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftIdArgs,
) -> AppResult<ScreenplayPagination> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.draft_id, "draft")?;
    core.project()?.store.read(|c| {
        load_draft(c, &a.draft_id)?;
        pagination_for(c, &a.draft_id)
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayPageViewArgs {
    pub draft_id: String,
    /// Include the title page (defaults to the page setup's title page switch).
    #[serde(default)]
    #[ts(optional)]
    pub title_page: Option<bool>,
    /// Revision asterisks (defaults to the page setup).
    #[serde(default)]
    #[ts(optional)]
    pub revision_marks: Option<bool>,
}

fn page_view(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayPageViewArgs,
) -> AppResult<InterchangePrintPreview> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.draft_id, "draft")?;
    core.project()?.store.read(|c| {
        load_draft(c, &a.draft_id)?;
        let info = idraft::load_draft_info(c, &a.draft_id)?;
        let scenes = idraft::load_scenes(c, &a.draft_id)?;
        let opts = idraft::export_options(&info, a.title_page, None, false, a.revision_marks, true);
        let doc = idraft::build_doc(&info, &scenes, None, opts.scene_numbers);
        Ok(print_preview(&info, &doc, &opts.layout()))
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayTitlePagePreviewArgs {
    pub screenplay_id: String,
    pub title_page: ScreenplayTitlePage,
    /// The draft whose name and revision fill empty draft and revision lines.
    #[serde(default)]
    #[ts(optional)]
    pub draft_id: Option<String>,
}

fn title_page_preview(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayTitlePagePreviewArgs,
) -> AppResult<InterchangePrintPreview> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.screenplay_id, "screenplay")?;
    for (v, what, max) in a.title_page.limits() {
        if v.chars().count() > max {
            return Err(AppError::invalid_input(format!("{what} is too long.")));
        }
    }
    core.project()?.store.read(|c| {
        let sp = load_screenplay(c, &a.screenplay_id)?;
        let draft_id = match &a.draft_id {
            Some(d) => d.clone(),
            None => sp
                .current_draft_id
                .clone()
                .ok_or_else(|| AppError::not_found("draft"))?,
        };
        let mut info = idraft::load_draft_info(c, &draft_id)?;
        if info.screenplay_id != sp.id {
            return Err(AppError::not_found("draft"));
        }
        info.title_page_json =
            serde_json::to_string(&a.title_page).map_err(|e| AppError::internal(e.to_string()))?;
        let doc = ScreenplayDoc {
            title_page: idraft::title_page_for_export(&info),
            ..Default::default()
        };
        let mut opts = idraft::screen_layout(&info);
        opts.title_page = true;
        let mut preview = print_preview(&info, &doc, &opts);
        preview.pages.retain(|p| p.title_page);
        Ok(preview)
    })
}

// ------------------------------------------------------------------ baseline

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselinePage {
    page: u32,
    hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineScene {
    lineage: String,
    label: String,
    heading: String,
    hash: String,
}

/// Record the locked pagination of a draft (replaces an earlier baseline of it).
pub(crate) fn record_baseline_tx(tx: &Tx<'_>, draft_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let (info, scenes, _, lay) = draft_layout(c, draft_id, None)?;
    let page = info.page_setup.engine();
    let pages: Vec<BaselinePage> = lay
        .pages
        .iter()
        .map(|p| BaselinePage {
            page: p.number.unwrap_or(0),
            hash: page_fingerprint(p),
        })
        .collect();
    let scene_rows: Vec<BaselineScene> = scenes
        .iter()
        .map(|s| BaselineScene {
            lineage: s.lineage_id.clone(),
            label: s.label.clone(),
            heading: s.heading.clone(),
            hash: scene_fingerprint(s),
        })
        .collect();
    c.execute(
        "DELETE FROM screenplay_lock_baseline WHERE draft_id=?1",
        [draft_id],
    )?;
    let now = now_ms();
    c.execute(
        "INSERT INTO screenplay_lock_baseline(id, draft_id, settings_json, pages_json, scenes_json, page_count,
                                              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            new_id(),
            draft_id,
            json(&page)?,
            json(&pages)?,
            json(&scene_rows)?,
            lay.script_page_count() as i64,
            now
        ],
    )?;
    Ok(())
}

fn json<T: Serialize>(v: &T) -> AppResult<String> {
    serde_json::to_string(v).map_err(|e| AppError::internal(e.to_string()))
}

/// Remove the baseline of a draft (unlock).
pub(crate) fn clear_baseline_tx(tx: &Tx<'_>, draft_id: &str) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM screenplay_lock_baseline WHERE draft_id=?1",
        [draft_id],
    )?;
    Ok(())
}

// ------------------------------------------------------------ revision pages

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayRevisionScene {
    /// The scene in the revision (None when it was removed).
    pub scene_id: Option<String>,
    pub label: String,
    pub heading: String,
    /// "changed" | "added" | "omitted" | "removed"
    pub change: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayRevisionPages {
    pub draft_id: String,
    /// The locked draft the revision is compared with (None when there is no baseline).
    pub baseline_draft_id: Option<String>,
    pub baseline_draft_name: Option<String>,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    pub revision_reason: Option<String>,
    /// Date of the revision ("2026-10-04").
    pub revision_date: Option<String>,
    pub baseline_page_count: u32,
    pub page_count: u32,
    /// Printed page numbers whose content differs from the locked pages, or that are new.
    pub changed_pages: Vec<u32>,
    pub scenes: Vec<ScreenplayRevisionScene>,
}

/// The locked baseline a draft is compared with: its revision record, else its source draft.
fn baseline_of(c: &Connection, draft_id: &str) -> AppResult<Option<String>> {
    let from_record: Option<String> = c
        .query_row(
            "SELECT baseline_draft_id FROM screenplay_revision WHERE draft_id=?1",
            [draft_id],
            |r| r.get(0),
        )
        .optional()?;
    let candidate = match from_record {
        Some(b) => Some(b),
        None => c
            .query_row(
                "SELECT created_from_draft_id FROM screenplay_draft WHERE id=?1",
                [draft_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten(),
    };
    let Some(b) = candidate else { return Ok(None) };
    let has: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_lock_baseline WHERE draft_id=?1)",
        [&b],
        |r| r.get(0),
    )?;
    Ok(has.then_some(b))
}

pub(crate) fn revision_pages_for(
    c: &Connection,
    draft_id: &str,
) -> AppResult<ScreenplayRevisionPages> {
    let d = load_draft(c, draft_id)?;
    let revision_date: Option<String> = c
        .query_row(
            "SELECT revision_date FROM screenplay_revision WHERE draft_id=?1",
            [draft_id],
            |r| r.get(0),
        )
        .optional()?;
    let mut out = ScreenplayRevisionPages {
        draft_id: draft_id.to_string(),
        baseline_draft_id: None,
        baseline_draft_name: None,
        revision_label: d.revision_label.clone(),
        revision_color: d.revision_color.clone(),
        revision_reason: d.revision_reason.clone(),
        revision_date,
        baseline_page_count: 0,
        page_count: 0,
        changed_pages: Vec::new(),
        scenes: Vec::new(),
    };
    let Some(base_id) = baseline_of(c, draft_id)? else {
        out.page_count = draft_layout(c, draft_id, None)?.3.script_page_count() as u32;
        return Ok(out);
    };
    let (settings, pages_json, scenes_json, base_count): (String, String, String, i64) = c.query_row(
        "SELECT settings_json, pages_json, scenes_json, page_count FROM screenplay_lock_baseline WHERE draft_id=?1",
        [&base_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let damaged = |e: serde_json::Error| {
        AppError::new(
            "project_format.invalid",
            "The locked page record of this draft can't be read. Lock the shooting draft again to record it.",
        )
        .with_detail(e.to_string())
    };
    let page: PageSetup = serde_json::from_str(&settings).map_err(damaged)?;
    let base_pages: Vec<BaselinePage> = serde_json::from_str(&pages_json).map_err(damaged)?;
    let base_scenes: Vec<BaselineScene> = serde_json::from_str(&scenes_json).map_err(damaged)?;
    let (_, scenes, _, lay) = draft_layout(c, draft_id, Some(page))?;
    let base_by_page: HashMap<u32, &str> = base_pages
        .iter()
        .map(|p| (p.page, p.hash.as_str()))
        .collect();
    out.baseline_draft_name = Some(load_draft(c, &base_id)?.name);
    out.baseline_draft_id = Some(base_id);
    out.baseline_page_count = base_count as u32;
    out.page_count = lay.script_page_count() as u32;
    out.changed_pages = lay
        .pages
        .iter()
        .filter_map(|p| {
            let n = p.number?;
            (base_by_page.get(&n) != Some(&page_fingerprint(p).as_str())).then_some(n)
        })
        .collect();
    let base_by_lineage: HashMap<&str, &BaselineScene> = base_scenes
        .iter()
        .map(|s| (s.lineage.as_str(), s))
        .collect();
    for s in &scenes {
        let change = match base_by_lineage.get(s.lineage_id.as_str()) {
            None => Some("added"),
            Some(_) if s.omitted => Some("omitted"),
            Some(b) if b.hash != scene_fingerprint(s) => Some("changed"),
            Some(_) => None,
        };
        if let Some(change) = change {
            out.scenes.push(ScreenplayRevisionScene {
                scene_id: Some(s.id.clone()),
                label: s.label.clone(),
                heading: s.heading.clone(),
                change: change.into(),
            });
        }
    }
    for b in &base_scenes {
        if !scenes.iter().any(|s| s.lineage_id == b.lineage) {
            out.scenes.push(ScreenplayRevisionScene {
                scene_id: None,
                label: b.label.clone(),
                heading: b.heading.clone(),
                change: "removed".into(),
            });
        }
    }
    Ok(out)
}

fn revision_pages(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftIdArgs,
) -> AppResult<ScreenplayRevisionPages> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.draft_id, "draft")?;
    core.project()?
        .store
        .read(|c| revision_pages_for(c, &a.draft_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_are_stable_and_separate_parts() {
        assert_eq!(fingerprint(["a", "b"]), fingerprint(["a", "b"]));
        assert_ne!(fingerprint(["ab", ""]), fingerprint(["a", "b"]));
        assert_eq!(fingerprint(["x"]).len(), 16);
    }
}
