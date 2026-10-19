//! Scene navigator columns the writer can show or hide (SCRIPT-017).
//!
//! One read returns, per live scene of a draft: the page the scene starts on (from the same
//! page layout the PDF export uses), how many notes it carries (script notes plus the scene's
//! notes field), its revision status against the draft it was revised from, and its production
//! scene number when the project has production numbers (ADR-0015). Columns whose data does not
//! exist are reported as absent so the screen can hide them instead of showing empty cells.

use std::collections::HashMap;

use openframe_domain::enums::{DraftStatus, ElementType};
use openframe_domain::{Actor, AppResult, Capability};
use openframe_import_export::layout::{self, LayoutOptions, LineStyle};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::modules::interchange::draft as ie_draft;
use crate::modules::screenplay::compare::{ScreenplaySceneChangeKind, compare_scenes};
use crate::modules::screenplay::{load_draft, load_scenes};
use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("writing.navigator_columns", navigator_columns).meta(
        M::read("Scene navigator columns: start page, notes, revision status and production number per scene.")
            .hidden(h::UI_FLOW),
    );
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingNavigatorArgs {
    pub draft_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum NavigatorRevisionKind {
    Added,
    Changed,
    Moved,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorRevision {
    pub kind: NavigatorRevisionKind,
    /// The revision's name, for example "Blue Revision".
    pub label: Option<String>,
    /// The revision colour key (see the Revisions drawer).
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct NavigatorSceneColumns {
    pub scene_id: String,
    /// Printed page the scene starts on; None for a scene without a heading.
    pub page: Option<u32>,
    pub notes: u32,
    pub revision: Option<NavigatorRevision>,
    pub production_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct WritingNavigatorColumns {
    pub rows: Vec<NavigatorSceneColumns>,
    /// Script pages (title page excluded).
    pub page_count: u32,
    /// The draft is a revision of another draft, so the Revision column has meaning.
    pub has_revisions: bool,
    /// The project stores production scene numbers (ADR-0015) and at least one scene has one.
    pub has_production_numbers: bool,
}

fn navigator_columns(
    core: &AppCore,
    actor: &Actor,
    a: WritingNavigatorArgs,
) -> AppResult<WritingNavigatorColumns> {
    actor.require(Capability::View, "view the screenplay")?;
    crate::util::require_id(&a.draft_id, "draft")?;
    core.project()?.store.read(|c| columns(c, &a.draft_id))
}

pub(crate) fn columns(c: &Connection, draft_id: &str) -> AppResult<WritingNavigatorColumns> {
    let draft = load_draft(c, draft_id)?;
    let scenes = load_scenes(c, draft_id)?;

    // Start pages from the export layout. Each scene gets its order as a printed number (so
    // this does not depend on how the export numbers scenes), and the first page that number
    // appears on is where the scene starts.
    let info = ie_draft::load_draft_info(c, draft_id)?;
    let loaded = ie_draft::load_scenes(c, draft_id)?;
    let mut doc = ie_draft::build_doc(&info, &loaded, None, false);
    let mut by_marker: HashMap<String, String> = HashMap::new();
    for (i, (scene, src)) in doc.scenes.iter_mut().zip(&loaded).enumerate() {
        if scene.heading.trim().is_empty() {
            scene.number = None;
            continue;
        }
        let marker = (i + 1).to_string();
        scene.number = Some(marker.clone());
        by_marker.insert(marker, src.id.clone());
    }
    let opts = LayoutOptions {
        title_page: false,
        scene_numbers: true,
        include_notes: false,
        revision_marks: false,
        revision_info: false,
        page_numbers: true,
        paginate: true,
        // The screenplay's own page setup, so page numbers agree with the page view and PDF.
        page: info.page_setup.engine(),
    };
    let lay = layout::layout(&doc, &opts);
    let mut start_page: HashMap<String, u32> = HashMap::new();
    for page in lay.pages.iter().filter(|p| !p.title_page) {
        let Some(n) = page.number else { continue };
        for line in &page.lines {
            if line.style == LineStyle::SceneNumber
                && let Some(id) = by_marker.get(line.text.trim())
            {
                start_page.entry(id.clone()).or_insert(n);
            }
        }
    }

    // Revision status against the draft this one was revised from.
    let mut revision: HashMap<String, NavigatorRevisionKind> = HashMap::new();
    let is_revision = draft.status == DraftStatus::Revision || draft.revision_label.is_some();
    if is_revision && let Some(src) = &draft.created_from {
        let source = load_scenes(c, src)?;
        let (_, changes) = compare_scenes(&source, &scenes);
        for ch in changes {
            let kind = match ch.kind {
                ScreenplaySceneChangeKind::Added => NavigatorRevisionKind::Added,
                ScreenplaySceneChangeKind::Changed => NavigatorRevisionKind::Changed,
                ScreenplaySceneChangeKind::Moved => NavigatorRevisionKind::Moved,
                _ => continue,
            };
            if let Some(b) = ch.b {
                revision.insert(b.scene_id, kind);
            }
        }
    }

    let production = production_numbers(c, draft_id)?;
    let has_production_numbers = production.values().any(|v| !v.trim().is_empty());

    let mut scene_notes: HashMap<String, bool> = HashMap::new();
    let mut st = c.prepare(
        "SELECT id, notes FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL",
    )?;
    for row in st.query_map([draft_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })? {
        let (id, n) = row?;
        scene_notes.insert(id, n.is_some_and(|t| !t.trim().is_empty()));
    }

    let rows = scenes
        .iter()
        .map(|s| {
            let script_notes = s
                .elements
                .iter()
                .filter(|e| e.element_type == ElementType::Note && !e.text.trim().is_empty())
                .count() as u32;
            NavigatorSceneColumns {
                scene_id: s.id.clone(),
                page: start_page.get(&s.id).copied(),
                notes: script_notes + u32::from(*scene_notes.get(&s.id).unwrap_or(&false)),
                revision: revision.get(&s.id).map(|k| NavigatorRevision {
                    kind: *k,
                    label: draft.revision_label.clone(),
                    color: draft.revision_color.clone(),
                }),
                production_number: production
                    .get(&s.id)
                    .filter(|v| !v.trim().is_empty())
                    .cloned(),
            }
        })
        .collect();
    Ok(WritingNavigatorColumns {
        rows,
        page_count: lay.script_page_count() as u32,
        has_revisions: is_revision && draft.created_from.is_some(),
        has_production_numbers,
    })
}

/// Stored production scene numbers (ADR-0015: frozen at lock, imported or set by hand). A
/// draft whose numbers still follow scene order has none, and the navigator hides the column.
fn production_numbers(c: &Connection, draft_id: &str) -> AppResult<HashMap<String, String>> {
    let mut out = HashMap::new();
    let mut st = c.prepare(
        "SELECT id, production_number FROM screenplay_scene
         WHERE draft_id=?1 AND deleted_at IS NULL AND production_number IS NOT NULL",
    )?;
    for row in st.query_map([draft_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (id, n) = row?;
        out.insert(id, n);
    }
    Ok(out)
}
