//! Production module: Production Source, Breakdown, Catalog, Locations and
//! Cast & Crew (FSD §26–30, §53–54, §96–100, §108, §124–125, §143–144, §160–163).
//!
//! Source-of-truth rules (FSD §63, §124):
//! - Production planning references screenplay scenes of the selected
//!   Production Source draft. It never writes screenplay tables.
//! - A newer draft never silently replaces the source; "Update Production
//!   Source" reconciles by stable scene identity (`lineage_id`) and never
//!   deletes breakdown data.
//! - Suggestions are never production data until the user accepts them.
//!
//! Layout: `script` (read-only screenplay access), `suggest` (deterministic
//! suggestion engine), `source` (source selection, reconciliation, overview),
//! `breakdown`, `catalog`, `locations`, `people` (cast & crew).

use std::collections::HashMap;

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{AppError, AppResult};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::registry::Registry;
use crate::util::optional_text;

pub mod breakdown;
pub mod catalog;
pub mod locations;
pub mod people;
pub mod script;
pub mod source;
pub mod suggest;

use script::SceneRow;

pub fn register(r: &mut Registry) {
    source::register(r);
    breakdown::register(r);
    catalog::register(r);
    locations::register(r);
    people::register(r);
}

/// Confirmed and Manual rows are production truth (Domain §4 Breakdown Element).
pub(crate) const PRODUCTION_STATES: &str = "('Confirmed','Manual')";

#[derive(Debug, Clone)]
pub(crate) struct ActiveSource {
    pub id: String,
    pub draft_id: String,
}

pub(crate) fn active_source(c: &Connection) -> AppResult<Option<ActiveSource>> {
    Ok(c.query_row(
        "SELECT id, draft_id FROM production_source WHERE active = 1 ORDER BY selected_at DESC, id DESC LIMIT 1",
        [],
        |r| Ok(ActiveSource { id: r.get(0)?, draft_id: r.get(1)? }),
    )
    .optional()?)
}

pub(crate) fn require_source(c: &Connection) -> AppResult<ActiveSource> {
    active_source(c)?.ok_or_else(|| {
        AppError::new(
            "validation.no_production_source",
            "Choose a screenplay draft as the Production Source first. Your screenplay is not changed.",
        )
    })
}

/// The scenes of the active source draft with lookup maps.
#[derive(Debug, Default)]
pub(crate) struct SourceScenes {
    pub rows: Vec<SceneRow>,
    pub by_id: HashMap<String, usize>,
}

impl SourceScenes {
    pub fn load(c: &Connection, draft_id: Option<&str>) -> AppResult<SourceScenes> {
        let rows = match draft_id {
            Some(d) => script::draft_scenes(c, d)?,
            None => Vec::new(),
        };
        let by_id = rows
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        Ok(SourceScenes { rows, by_id })
    }

    pub fn get(&self, scene_id: &str) -> Option<&SceneRow> {
        self.by_id.get(scene_id).map(|i| &self.rows[*i])
    }
}

/// A reference to a scene for "Used in Scenes" lists and navigation.
#[derive(Debug, Clone, Serialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionSceneRef {
    pub scene_id: String,
    /// Display number in the Production Source; None when the scene is no longer in it.
    pub number: Option<String>,
    pub heading: String,
    /// False = "Scene removed from current source" (historical, FSD §54).
    pub in_source: bool,
}

pub(crate) fn scene_ref(
    c: &Connection,
    scenes: &SourceScenes,
    scene_id: &str,
) -> AppResult<ProductionSceneRef> {
    if let Some(r) = scenes.get(scene_id) {
        return Ok(ProductionSceneRef {
            scene_id: scene_id.to_string(),
            number: Some(r.number.clone()),
            heading: r.heading.clone(),
            in_source: true,
        });
    }
    let heading = script::scene_lookup(c, scene_id)?
        .map(|s| s.heading)
        .unwrap_or_default();
    Ok(ProductionSceneRef {
        scene_id: scene_id.to_string(),
        number: None,
        heading,
        in_source: false,
    })
}

/// Sort scene refs: source scenes by number, then historical ones.
pub(crate) fn sort_refs(refs: &mut [ProductionSceneRef]) {
    refs.sort_by(|a, b| {
        let na = a
            .number
            .as_deref()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX);
        let nb = b
            .number
            .as_deref()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(u32::MAX);
        na.cmp(&nb).then_with(|| a.heading.cmp(&b.heading))
    });
}

/// Scene usage per catalog item, derived from production breakdown rows
/// (Domain §4: "scene usage is derived from references").
pub(crate) fn catalog_usage(
    c: &Connection,
    scenes: &SourceScenes,
) -> AppResult<HashMap<String, Vec<ProductionSceneRef>>> {
    let mut stmt = c.prepare(&format!(
        "SELECT DISTINCT catalog_item_id, scene_id FROM breakdown_element
         WHERE catalog_item_id IS NOT NULL AND deleted_at IS NULL AND archived = 0
           AND confirmation_state IN {PRODUCTION_STATES}"
    ))?;
    let pairs: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<String, Vec<ProductionSceneRef>> = HashMap::new();
    let mut cache: HashMap<String, ProductionSceneRef> = HashMap::new();
    for (item, scene) in pairs {
        let r = match cache.get(&scene) {
            Some(r) => r.clone(),
            None => {
                let r = scene_ref(c, scenes, &scene)?;
                cache.insert(scene.clone(), r.clone());
                r
            }
        };
        out.entry(item).or_default().push(r);
    }
    for v in out.values_mut() {
        sort_refs(v);
    }
    Ok(out)
}

pub(crate) fn parse_category(value: &str) -> AppResult<BreakdownCategory> {
    BreakdownCategory::parse(value.trim()).ok_or_else(|| {
        AppError::validation(
            "category",
            "Choose one of the breakdown categories (Cast, Props, Wardrobe…).",
        )
    })
}

/// Update-patch helper for optional text: None = unchanged, Some("") = clear.
pub(crate) fn patch_text(
    value: Option<String>,
    what: &str,
    max: usize,
) -> AppResult<Option<SqlValue>> {
    match value {
        None => Ok(None),
        Some(v) => Ok(Some(match optional_text(Some(v), what, max)? {
            Some(t) => SqlValue::Text(t),
            None => SqlValue::Null,
        })),
    }
}

/// "Scenes 12, 27, 38" / "Scene 9" / "18 scenes" (UX §3.23 table text).
pub(crate) fn used_in_label(refs: &[ProductionSceneRef]) -> String {
    let nums: Vec<&str> = refs.iter().filter_map(|r| r.number.as_deref()).collect();
    match nums.len() {
        0 if refs.is_empty() => "Not used yet".into(),
        0 => "Removed scenes only".into(),
        1 => format!("Scene {}", nums[0]),
        n if n <= 6 => format!("Scenes {}", nums.join(", ")),
        n => format!("{n} scenes"),
    }
}

/// Upper-case character key used to connect cues, Story characters and cast.
pub(crate) fn character_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}
