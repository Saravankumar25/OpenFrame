//! Storyboards and panels (FSD §32, §102; FSD-STB-001, FSD-PROD-014..017).
//!
//! A storyboard may be associated with a screenplay scene (by identity) or be
//! standalone. Panels are ordered; the panel number and the linked shot's
//! label are derived, never stored. Reordering the panels of a scene storyboard
//! re-sequences the linked shots of that scene so shot numbers follow the
//! panel order (FSD §32.5 "Shot numbers are recalculated from order").

use std::collections::HashSet;
use std::path::Path;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{
    int, next_position, opt_int, place_at, renumber, sibling_ids, text, update_fields,
};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::common::{
    NAME_MAX, SHORT_MAX, TEXT_MAX, ingest_image_data, ingest_image_file, patch_text,
};
use super::scenes::SceneCtx;
use super::shots::{ShotDto, load_shot, shot_label};
use crate::core::AppCore;
use crate::modules::files::purge_asset_if_unreferenced;
use crate::registry::SearchDoc;
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, load_asset_opt, required_text};

const PANEL_SCOPE: &str = "storyboard_id = ?1 AND deleted_at IS NULL";
const PANEL_FIELDS: &[&str] = &[
    "description",
    "framing",
    "movement",
    "angle",
    "sound_note",
    "duration_ms",
    "note",
];
/// Longest planned panel duration (one hour).
const MAX_DURATION_MS: i64 = 3_600_000;

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardSummary {
    pub id: String,
    pub name: String,
    /// Current scene row (resolved by identity in the planning source).
    pub scene_id: Option<String>,
    pub scene_lineage_id: Option<String>,
    pub scene_number: Option<String>,
    /// Current heading, or the planned heading when the scene was removed.
    pub scene_heading: Option<String>,
    /// The associated scene is no longer in the script (planning kept for reference).
    pub scene_removed: bool,
    /// "Scene changed since planning".
    pub needs_review: bool,
    #[ts(type = "number")]
    pub panel_count: i64,
    pub cover: Option<AssetInfo>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PanelDto {
    pub id: String,
    pub storyboard_id: String,
    /// Panel number from order (1-based; derived).
    #[ts(type = "number")]
    pub number: i64,
    /// "placeholder", "image" or "sketch".
    pub visual_kind: String,
    pub asset: Option<AssetInfo>,
    pub description: String,
    pub framing: Option<String>,
    pub movement: Option<String>,
    pub angle: Option<String>,
    pub sound_note: Option<String>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    pub note: Option<String>,
    /// Linked shot (only while that shot exists).
    pub shot_id: Option<String>,
    /// Derived label of the linked shot, e.g. "12A".
    pub shot_label: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardDto {
    pub board: StoryboardSummary,
    pub panels: Vec<PanelDto>,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryboardListArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryboardIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateStoryboardArgs {
    /// Required for a standalone storyboard; defaults to the scene heading.
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameStoryboardArgs {
    pub id: String,
    pub name: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetStoryboardSceneArgs {
    pub id: String,
    /// None = make the storyboard standalone.
    pub scene_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddPanelArgs {
    pub storyboard_id: String,
    /// "placeholder" (Empty Panel), "image" (Import Image, needs `path`) or
    /// "sketch" (Draw / Sketch, needs `dataBase64` PNG).
    pub visual: String,
    #[serde(default)]
    #[ts(optional)]
    pub path: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub data_base64: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    /// Insert position (0-based); default = end.
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePanelArgs {
    pub id: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub expected_rev: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub framing: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub movement: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub angle: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub sound_note: Option<String>,
    /// Milliseconds; 0 clears the duration.
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPanelVisualArgs {
    pub id: String,
    /// "placeholder" clears the visual; "image" needs `path`; "sketch" needs `dataBase64`.
    pub visual: String,
    #[serde(default)]
    #[ts(optional)]
    pub path: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub data_base64: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderPanelArgs {
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MovePanelArgs {
    pub id: String,
    pub storyboard_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletePanelsArgs {
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkShotArgs {
    pub panel_id: String,
    /// None removes the link (neither object is deleted).
    pub shot_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShotFromPanelArgs {
    pub panel_id: String,
    /// Needed only when the storyboard is not associated with a scene.
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
}

// ------------------------------------------------------------------ loading

fn root_of(core: &AppCore) -> AppResult<std::path::PathBuf> {
    Ok(core.project()?.layout.root().to_path_buf())
}

type BoardRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    i64,
);

fn board_row(c: &Connection, id: &str) -> AppResult<BoardRow> {
    c.query_row(
        "SELECT id, name, scene_id, scene_lineage_id, scene_heading, scene_hash, needs_review, rev
         FROM storyboard WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("storyboard"))
}

pub(super) fn load_summary(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    id: &str,
) -> AppResult<StoryboardSummary> {
    let (id, name, scene_id, lineage, heading_snap, hash, flagged, rev) = board_row(c, id)?;
    let (mut scene_removed, mut needs_review, mut scene_number, mut scene_heading, mut cur_scene) =
        (false, false, None, heading_snap.clone(), scene_id.clone());
    if let Some(l) = &lineage {
        let (removed, review) = ctx.review_state(c, l, hash.as_deref(), flagged)?;
        scene_removed = removed;
        needs_review = review;
        if let Some(sc) = ctx.by_lineage(l) {
            scene_number = Some(ctx.number_label(sc));
            scene_heading = Some(sc.heading.clone());
            cur_scene = Some(sc.id.clone());
        }
    }
    let panel_count: i64 = c.query_row(
        "SELECT count(*) FROM storyboard_panel WHERE storyboard_id=?1 AND deleted_at IS NULL",
        [&id],
        |r| r.get(0),
    )?;
    let cover_id: Option<String> = c
        .query_row(
            "SELECT asset_id FROM storyboard_panel WHERE storyboard_id=?1 AND deleted_at IS NULL AND asset_id IS NOT NULL
             ORDER BY position, id LIMIT 1",
            [&id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(StoryboardSummary {
        id,
        name,
        scene_id: cur_scene,
        scene_lineage_id: lineage,
        scene_number,
        scene_heading,
        scene_removed,
        needs_review,
        panel_count,
        cover: load_asset_opt(c, root, cover_id.as_deref())?,
        rev,
    })
}

pub(super) fn load_panels(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    board_id: &str,
) -> AppResult<Vec<PanelDto>> {
    let ids = sibling_ids(c, "storyboard_panel", PANEL_SCOPE, &[text(board_id)])?;
    ids.iter()
        .enumerate()
        .map(|(i, id)| load_panel_at(c, root, ctx, id, i as i64 + 1))
        .collect()
}

fn load_panel_at(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    id: &str,
    number: i64,
) -> AppResult<PanelDto> {
    #[allow(clippy::type_complexity)]
    let row: (
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
        i64,
    ) = c
        .query_row(
            "SELECT id, storyboard_id, visual_kind, asset_id, description, framing, movement, angle, sound_note,
                    duration_ms, note, shot_id, rev
             FROM storyboard_panel WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
                    r.get(12)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("panel"))?;
    let (
        id,
        storyboard_id,
        visual_kind,
        asset_id,
        description,
        framing,
        movement,
        angle,
        sound_note,
        duration_ms,
        note,
        shot_id,
        rev,
    ) = row;
    let (shot_id, shot_label) = match shot_id {
        Some(s) => match shot_label(c, ctx, &s)? {
            Some(l) => (Some(s), Some(l.label)),
            None => (None, None),
        },
        None => (None, None),
    };
    Ok(PanelDto {
        id,
        storyboard_id,
        number,
        visual_kind,
        asset: load_asset_opt(c, root, asset_id.as_deref())?,
        description,
        framing,
        movement,
        angle,
        sound_note,
        duration_ms,
        note,
        shot_id,
        shot_label,
        rev,
    })
}

/// Load one panel with its derived number.
pub(super) fn load_panel(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    id: &str,
) -> AppResult<PanelDto> {
    let board: String = c
        .query_row(
            "SELECT storyboard_id FROM storyboard_panel WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("panel"))?;
    let ids = sibling_ids(c, "storyboard_panel", PANEL_SCOPE, &[text(&board)])?;
    let n = ids
        .iter()
        .position(|x| x == id)
        .map(|i| i as i64 + 1)
        .unwrap_or(0);
    load_panel_at(c, root, ctx, id, n)
}

pub fn list(
    core: &AppCore,
    actor: &Actor,
    _: StoryboardListArgs,
) -> AppResult<Vec<StoryboardSummary>> {
    actor.require(Capability::View, "view storyboards")?;
    let root = root_of(core)?;
    core.project()?.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let mut stmt = c.prepare("SELECT id, scene_lineage_id FROM storyboard WHERE deleted_at IS NULL ORDER BY position, id")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        if rows.len() > super::scenes::PRIME_THRESHOLD {
            ctx.prime_hashes(c)?;
        }
        let mut out: Vec<(usize, usize, StoryboardSummary)> = Vec::with_capacity(rows.len());
        for (i, (id, lineage)) in rows.into_iter().enumerate() {
            let s = load_summary(c, &root, &ctx, &id)?;
            // Scene storyboards in script order, then removed-scene boards, then standalone boards.
            let group = match &lineage {
                Some(l) if ctx.by_lineage(l).is_some() => ctx.order_of(l),
                Some(_) => usize::MAX - 1,
                None => usize::MAX,
            };
            out.push((group, i, s));
        }
        out.sort_by_key(|(g, i, _)| (*g, *i));
        Ok(out.into_iter().map(|(_, _, s)| s).collect())
    })
}

pub fn get(core: &AppCore, actor: &Actor, args: StoryboardIdArgs) -> AppResult<StoryboardDto> {
    actor.require(Capability::View, "view storyboards")?;
    let root = root_of(core)?;
    core.project()?.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        Ok(StoryboardDto {
            board: load_summary(c, &root, &ctx, &args.id)?,
            panels: load_panels(c, &root, &ctx, &args.id)?,
        })
    })
}

fn read_summary(core: &AppCore, id: &str) -> AppResult<StoryboardSummary> {
    let root = root_of(core)?;
    core.project()?
        .store
        .read(|c| load_summary(c, &root, &SceneCtx::load(c)?, id))
}

fn read_panel(core: &AppCore, id: &str) -> AppResult<PanelDto> {
    let root = root_of(core)?;
    core.project()?
        .store
        .read(|c| load_panel(c, &root, &SceneCtx::load(c)?, id))
}

// ------------------------------------------------------------------ storyboards

/// Insert a storyboard; scene storyboards snapshot the scene they were planned against.
pub(super) fn insert_board(
    tx: &Tx<'_>,
    ctx: &SceneCtx,
    name: Option<String>,
    scene_id: Option<&str>,
) -> AppResult<String> {
    let c = tx.conn();
    let scene = scene_id.map(|s| ctx.resolve(c, s)).transpose()?;
    let name = match (
        name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
        &scene,
    ) {
        (Some(n), _) => required_text(&n, "Storyboard name", NAME_MAX)?,
        (None, Some(sc)) => {
            let h = sc.heading.trim();
            let n = if h.is_empty() {
                format!("Scene {}", ctx.number_label(sc))
            } else {
                h.to_string()
            };
            n.chars().take(NAME_MAX).collect()
        }
        (None, None) => return Err(AppError::required("Storyboard name")),
    };
    let id = new_id();
    let now = now_ms();
    let pos = next_position(c, "storyboard", "deleted_at IS NULL", &[])?;
    let (sid, lineage, heading, hash) = match &scene {
        Some(sc) => {
            let (h, _) = ctx.hash(c, &sc.id)?;
            (
                Some(sc.id.clone()),
                Some(sc.lineage_id.clone()),
                Some(sc.heading.clone()),
                Some(h),
            )
        }
        None => (None, None, None, None),
    };
    c.execute(
        "INSERT INTO storyboard(id, name, scene_id, scene_lineage_id, scene_heading, scene_hash, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![id, name, sid, lineage, heading, hash, pos, now],
    )?;
    Ok(id)
}

pub fn create(
    core: &AppCore,
    actor: &Actor,
    args: CreateStoryboardArgs,
) -> AppResult<StoryboardSummary> {
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new("storyboard.create", "Created storyboard", Capability::Edit),
        |tx| {
            let ctx = SceneCtx::load(tx.conn())?;
            insert_board(tx, &ctx, args.name.clone(), args.scene_id.as_deref())
        },
    )?;
    read_summary(core, &id)
}

pub fn rename(
    core: &AppCore,
    actor: &Actor,
    args: RenameStoryboardArgs,
) -> AppResult<StoryboardSummary> {
    let name = required_text(&args.name, "Storyboard name", NAME_MAX)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.rename",
            format!("Renamed storyboard to “{name}”"),
            Capability::Edit,
        )
        .target("storyboard", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "storyboard",
                &args.id,
                &[("name", text(name.clone()))],
                &["name"],
                args.expected_rev,
                "storyboard",
            )?;
            Ok(())
        },
    )?;
    read_summary(core, &args.id)
}

pub fn set_scene(
    core: &AppCore,
    actor: &Actor,
    args: SetStoryboardSceneArgs,
) -> AppResult<StoryboardSummary> {
    let s = core.project()?;
    let summary = if args.scene_id.is_some() {
        "Associated storyboard with a scene"
    } else {
        "Made storyboard standalone"
    };
    s.store.mutate(
        actor,
        MutationMeta::new("storyboard.set_scene", summary, Capability::Edit)
            .target("storyboard", &args.id),
        |tx| {
            let c = tx.conn();
            board_row(c, &args.id)?;
            let fields: Vec<(&str, SqlValue)> = match &args.scene_id {
                Some(sid) => {
                    let ctx = SceneCtx::load(c)?;
                    let sc = ctx.resolve(c, sid)?;
                    let (h, _) = ctx.hash(c, &sc.id)?;
                    vec![
                        ("scene_id", text(sc.id.clone())),
                        ("scene_lineage_id", text(sc.lineage_id.clone())),
                        ("scene_heading", text(sc.heading.clone())),
                        ("scene_hash", text(h)),
                        ("needs_review", int(0)),
                    ]
                }
                None => vec![
                    ("scene_id", SqlValue::Null),
                    ("scene_lineage_id", SqlValue::Null),
                    ("scene_heading", SqlValue::Null),
                    ("scene_hash", SqlValue::Null),
                    ("needs_review", int(0)),
                ],
            };
            update_fields(
                c,
                "storyboard",
                &args.id,
                &fields,
                &[
                    "scene_id",
                    "scene_lineage_id",
                    "scene_heading",
                    "scene_hash",
                    "needs_review",
                ],
                None,
                "storyboard",
            )?;
            Ok(())
        },
    )?;
    read_summary(core, &args.id)
}

pub fn delete(core: &AppCore, actor: &Actor, args: StoryboardIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (name, pos): (String, i64) = s.store.read(|c| {
        c.query_row(
            "SELECT name, position FROM storyboard WHERE id=?1 AND deleted_at IS NULL",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("storyboard"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.delete",
            format!("Deleted storyboard “{name}”"),
            Capability::SoftDelete,
        )
        .target("storyboard", &args.id),
        |tx| {
            // Panels stay with the (recoverable) storyboard; linked shots are never touched.
            // Touching the panels keeps their search entries in step with the board.
            touch_panels(tx, &args.id)?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "storyboard",
                    table: "storyboard",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: Some(pos),
                },
            )
        },
    )
}

fn touch_panels(tx: &Tx<'_>, board: &str) -> AppResult<()> {
    tx.conn().execute(
        "UPDATE storyboard_panel SET rev=rev+1, updated_at=?1 WHERE storyboard_id=?2 AND deleted_at IS NULL",
        params![now_ms(), board],
    )?;
    Ok(())
}

/// Restore a storyboard with its panels (they never left it).
pub fn restore_storyboard(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "UPDATE storyboard SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
        params![now_ms(), row.object_id],
    )?;
    touch_panels(tx, &row.object_id)
}

// ------------------------------------------------------------------ panels

enum Visual {
    Placeholder,
    /// An image file chosen by the user.
    ImageFile(String),
    /// A pasted image (base64).
    ImageData(String),
    /// A drawing made in the sketch dialog (base64 PNG).
    Sketch(String),
}

fn parse_visual(kind: &str, path: Option<String>, data: Option<String>) -> AppResult<Visual> {
    let path = path.filter(|p| !p.trim().is_empty());
    let data = data.filter(|d| !d.trim().is_empty());
    match kind {
        "placeholder" => Ok(Visual::Placeholder),
        "image" => match (path, data) {
            (Some(p), _) => Ok(Visual::ImageFile(p)),
            (None, Some(d)) => Ok(Visual::ImageData(d)),
            _ => Err(AppError::required("Image file")),
        },
        "sketch" => data
            .map(Visual::Sketch)
            .ok_or_else(|| AppError::required("Sketch")),
        _ => Err(AppError::invalid_input("Unknown panel visual.")),
    }
}

/// Store the visual and return (visual_kind, asset_id).
fn store_visual(tx: &Tx<'_>, v: Visual) -> AppResult<(&'static str, Option<String>)> {
    Ok(match v {
        Visual::Placeholder => ("placeholder", None),
        Visual::ImageFile(p) => ("image", Some(ingest_image_file(tx, &p)?.id)),
        Visual::ImageData(d) => ("image", Some(ingest_image_data(tx, &d, "pasted-image")?.id)),
        Visual::Sketch(d) => ("sketch", Some(ingest_image_data(tx, &d, "sketch.png")?.id)),
    })
}

/// Insert a panel at `index` (default end). Returns the new id.
#[allow(clippy::too_many_arguments)]
pub(super) fn insert_panel(
    tx: &Tx<'_>,
    board_id: &str,
    visual_kind: &str,
    asset_id: Option<&str>,
    description: &str,
    framing: Option<&str>,
    movement: Option<&str>,
    angle: Option<&str>,
    shot_id: Option<&str>,
    index: Option<usize>,
) -> AppResult<String> {
    let c = tx.conn();
    let id = new_id();
    let now = now_ms();
    let pos = next_position(c, "storyboard_panel", PANEL_SCOPE, &[text(board_id)])?;
    c.execute(
        "INSERT INTO storyboard_panel(id, storyboard_id, visual_kind, asset_id, description, framing, movement, angle, shot_id,
                                      position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![id, board_id, visual_kind, asset_id, description, framing, movement, angle, shot_id, pos, now],
    )?;
    if let Some(i) = index {
        let sibs = sibling_ids(c, "storyboard_panel", PANEL_SCOPE, &[text(board_id)])?;
        place_at(c, "storyboard_panel", sibs, &id, Some(i))?;
    }
    Ok(id)
}

pub fn add_panel(core: &AppCore, actor: &Actor, args: AddPanelArgs) -> AppResult<PanelDto> {
    let description = args
        .description
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    if description.chars().count() > TEXT_MAX {
        return Err(AppError::invalid_input(format!(
            "Description is too long (maximum {TEXT_MAX} characters)."
        )));
    }
    let visual = parse_visual(&args.visual, args.path.clone(), args.data_base64.clone())?;
    let label = match visual {
        Visual::Placeholder => "Added empty panel",
        Visual::ImageFile(_) | Visual::ImageData(_) => "Added panel from image",
        Visual::Sketch(_) => "Added sketch panel",
    };
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new("storyboard.add_panel", label, Capability::Edit)
            .target("storyboard", &args.storyboard_id),
        |tx| {
            board_row(tx.conn(), &args.storyboard_id)?;
            let (kind, asset) = store_visual(tx, visual)?;
            insert_panel(
                tx,
                &args.storyboard_id,
                kind,
                asset.as_deref(),
                &description,
                None,
                None,
                None,
                None,
                args.index,
            )
        },
    )?;
    read_panel(core, &id)
}

pub fn update_panel(core: &AppCore, actor: &Actor, args: UpdatePanelArgs) -> AppResult<PanelDto> {
    let mut fields: Vec<(&'static str, SqlValue)> = Vec::new();
    if let Some(d) = args.description.clone() {
        let d = d.trim().to_string();
        if d.chars().count() > TEXT_MAX {
            return Err(AppError::invalid_input(format!(
                "Description is too long (maximum {TEXT_MAX} characters)."
            )));
        }
        fields.push(("description", text(d)));
    }
    patch_text(
        &mut fields,
        "framing",
        args.framing.clone(),
        "Framing",
        SHORT_MAX,
    )?;
    patch_text(
        &mut fields,
        "movement",
        args.movement.clone(),
        "Camera movement",
        SHORT_MAX,
    )?;
    patch_text(&mut fields, "angle", args.angle.clone(), "Angle", SHORT_MAX)?;
    patch_text(
        &mut fields,
        "sound_note",
        args.sound_note.clone(),
        "Dialogue / sound note",
        TEXT_MAX,
    )?;
    patch_text(
        &mut fields,
        "note",
        args.note.clone(),
        "Storyboard note",
        TEXT_MAX,
    )?;
    if let Some(ms) = args.duration_ms {
        if !(0..=MAX_DURATION_MS).contains(&ms) {
            return Err(AppError::invalid_input(
                "Duration must be between 0 and 60 minutes.",
            ));
        }
        fields.push(("duration_ms", opt_int((ms > 0).then_some(ms))));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.update_panel",
            "Edited storyboard panel",
            Capability::Edit,
        )
        .target("storyboard_panel", &args.id)
        .coalesce(format!("storyboard.update_panel:{}", args.id)),
        |tx| {
            ensure_panel(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "storyboard_panel",
                &args.id,
                &fields,
                PANEL_FIELDS,
                args.expected_rev,
                "panel",
            )?;
            Ok(())
        },
    )?;
    read_panel(core, &args.id)
}

fn ensure_panel(c: &Connection, id: &str) -> AppResult<(String, Option<String>, Option<String>)> {
    c.query_row(
        "SELECT p.storyboard_id, p.asset_id, p.shot_id FROM storyboard_panel p JOIN storyboard b ON b.id = p.storyboard_id
         WHERE p.id=?1 AND p.deleted_at IS NULL AND b.deleted_at IS NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("panel"))
}

pub fn set_panel_visual(
    core: &AppCore,
    actor: &Actor,
    args: SetPanelVisualArgs,
) -> AppResult<PanelDto> {
    let visual = parse_visual(&args.visual, args.path.clone(), args.data_base64.clone())?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.set_panel_visual",
            "Changed panel image",
            Capability::Edit,
        )
        .target("storyboard_panel", &args.id),
        |tx| {
            let (_, old_asset, _) = ensure_panel(tx.conn(), &args.id)?;
            let (kind, asset) = store_visual(tx, visual)?;
            update_fields(
                tx.conn(),
                "storyboard_panel",
                &args.id,
                &[
                    ("visual_kind", text(kind)),
                    (
                        "asset_id",
                        asset.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                    ),
                ],
                &["visual_kind", "asset_id"],
                None,
                "panel",
            )?;
            // The previous image stays on disk while undo can still bring it back;
            // it is purged with the panel/storyboard or when no longer referenced.
            let _ = old_asset;
            Ok(())
        },
    )?;
    read_panel(core, &args.id)
}

/// Re-sequence the linked shots of a scene so they follow the panel order of a
/// scene storyboard. Unlinked shots keep their slots.
fn sync_shots_to_panels(c: &Connection, board_id: &str) -> AppResult<()> {
    let lineage: Option<String> = c.query_row(
        "SELECT scene_lineage_id FROM storyboard WHERE id=?1",
        [board_id],
        |r| r.get(0),
    )?;
    let Some(lineage) = lineage else {
        return Ok(());
    };
    let shots = sibling_ids(
        c,
        "shot",
        "scene_lineage_id = ?1 AND deleted_at IS NULL",
        &[text(&lineage)],
    )?;
    let live: HashSet<&String> = shots.iter().collect();
    let mut linked: Vec<String> = Vec::new();
    let mut stmt = c.prepare(&format!(
        "SELECT shot_id FROM storyboard_panel WHERE {PANEL_SCOPE} AND shot_id IS NOT NULL ORDER BY position, id"
    ))?;
    for sid in stmt.query_map([board_id], |r| r.get::<_, String>(0))? {
        let sid = sid?;
        if live.contains(&sid) && !linked.contains(&sid) {
            linked.push(sid);
        }
    }
    if linked.len() < 2 {
        return Ok(());
    }
    let set: HashSet<&String> = linked.iter().collect();
    let mut next = linked.iter();
    let ordered: Vec<String> = shots
        .iter()
        .map(|s| {
            if set.contains(s) {
                next.next().cloned().unwrap_or_else(|| s.clone())
            } else {
                s.clone()
            }
        })
        .collect();
    renumber(c, "shot", &ordered)
}

pub fn reorder_panel(core: &AppCore, actor: &Actor, args: ReorderPanelArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.reorder_panel",
            "Moved storyboard panel",
            Capability::Edit,
        )
        .target("storyboard_panel", &args.id),
        |tx| {
            let (board, _, _) = ensure_panel(tx.conn(), &args.id)?;
            let sibs = sibling_ids(tx.conn(), "storyboard_panel", PANEL_SCOPE, &[text(&board)])?;
            place_at(
                tx.conn(),
                "storyboard_panel",
                sibs,
                &args.id,
                Some(args.index),
            )?;
            sync_shots_to_panels(tx.conn(), &board)?;
            tx.reindex("storyboard", &board);
            Ok(())
        },
    )
}

pub fn move_panel(core: &AppCore, actor: &Actor, args: MovePanelArgs) -> AppResult<PanelDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "storyboard.move_panel",
            "Moved panel to another storyboard",
            Capability::Edit,
        )
        .target("storyboard_panel", &args.id),
        |tx| {
            let c = tx.conn();
            let (from, _, _) = ensure_panel(c, &args.id)?;
            board_row(c, &args.storyboard_id)?;
            if from != args.storyboard_id {
                let pos = next_position(
                    c,
                    "storyboard_panel",
                    PANEL_SCOPE,
                    &[text(&args.storyboard_id)],
                )?;
                update_fields(
                    c,
                    "storyboard_panel",
                    &args.id,
                    &[
                        ("storyboard_id", text(args.storyboard_id.clone())),
                        ("position", int(pos)),
                    ],
                    &["storyboard_id", "position"],
                    None,
                    "panel",
                )?;
            }
            let sibs = sibling_ids(
                c,
                "storyboard_panel",
                PANEL_SCOPE,
                &[text(&args.storyboard_id)],
            )?;
            place_at(c, "storyboard_panel", sibs, &args.id, args.index)?;
            Ok(())
        },
    )?;
    read_panel(core, &args.id)
}

pub fn delete_panels(core: &AppCore, actor: &Actor, args: DeletePanelsArgs) -> AppResult<()> {
    if args.ids.is_empty() {
        return Ok(());
    }
    if args.ids.len() > 500 {
        return Err(AppError::invalid_input(
            "Delete at most 500 panels at a time.",
        ));
    }
    let s = core.project()?;
    let label = if args.ids.len() == 1 {
        "Deleted storyboard panel".to_string()
    } else {
        format!("Deleted {} storyboard panels", args.ids.len())
    };
    s.store.mutate(
        actor,
        MutationMeta::new("storyboard.delete_panels", label, Capability::SoftDelete),
        |tx| {
            for id in &args.ids {
                let c = tx.conn();
                let (board, _, _) = ensure_panel(c, id)?;
                let sibs = sibling_ids(c, "storyboard_panel", PANEL_SCOPE, &[text(&board)])?;
                let idx = sibs.iter().position(|x| x == id).unwrap_or(0) as i64;
                let desc: String = c.query_row(
                    "SELECT description FROM storyboard_panel WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )?;
                let title = if desc.trim().is_empty() {
                    format!("Panel {}", idx + 1)
                } else {
                    format!(
                        "Panel {} — {}",
                        idx + 1,
                        desc.chars().take(60).collect::<String>()
                    )
                };
                // Links to shots are kept on the row so restore brings them back; shots are untouched.
                soft_delete(
                    tx,
                    DeleteSpec {
                        object_type: "storyboard_panel",
                        table: "storyboard_panel",
                        id,
                        title: Some(title),
                        parent_type: Some("storyboard"),
                        parent_id: Some(board),
                        position: Some(idx),
                    },
                )?;
            }
            Ok(())
        },
    )
}

/// Link (or unlink) a panel and a shot. Several panels may be options for one
/// shot; a new link never replaces an existing one (FSD §102).
pub(super) fn link(tx: &Tx<'_>, panel_id: &str, shot_id: Option<&str>) -> AppResult<()> {
    let c = tx.conn();
    let (_, _, old_shot) = ensure_panel(c, panel_id)?;
    if let Some(sid) = shot_id {
        let ok: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM shot WHERE id=?1 AND deleted_at IS NULL)",
            [sid],
            |r| r.get(0),
        )?;
        if !ok {
            return Err(AppError::not_found("shot"));
        }
    }
    if old_shot.as_deref() == shot_id {
        return Ok(());
    }
    update_fields(
        c,
        "storyboard_panel",
        panel_id,
        &[("shot_id", shot_id.map(text).unwrap_or(SqlValue::Null))],
        &["shot_id"],
        None,
        "panel",
    )?;
    // Keep the shot's primary panel pointer consistent.
    if let Some(old) = old_shot {
        let primary: Option<String> = c
            .query_row(
                "SELECT storyboard_panel_id FROM shot WHERE id=?1",
                [&old],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        if primary.as_deref() == Some(panel_id) {
            let next: Option<String> = c
                .query_row(
                    "SELECT p.id FROM storyboard_panel p JOIN storyboard b ON b.id=p.storyboard_id
                     WHERE p.shot_id=?1 AND p.deleted_at IS NULL AND b.deleted_at IS NULL ORDER BY b.position, p.position LIMIT 1",
                    [&old],
                    |r| r.get(0),
                )
                .optional()?;
            update_fields(
                c,
                "shot",
                &old,
                &[(
                    "storyboard_panel_id",
                    next.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                )],
                &["storyboard_panel_id"],
                None,
                "shot",
            )?;
        }
    }
    if let Some(sid) = shot_id {
        let primary_live: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM shot s JOIN storyboard_panel p ON p.id = s.storyboard_panel_id
                           JOIN storyboard b ON b.id = p.storyboard_id
             WHERE s.id=?1 AND p.deleted_at IS NULL AND b.deleted_at IS NULL AND p.shot_id = s.id)",
            [sid],
            |r| r.get(0),
        )?;
        if !primary_live {
            update_fields(
                c,
                "shot",
                sid,
                &[("storyboard_panel_id", text(panel_id))],
                &["storyboard_panel_id"],
                None,
                "shot",
            )?;
        }
    }
    Ok(())
}

pub fn link_shot(core: &AppCore, actor: &Actor, args: LinkShotArgs) -> AppResult<PanelDto> {
    let s = core.project()?;
    let label = if args.shot_id.is_some() {
        "Linked panel to shot"
    } else {
        "Unlinked panel from shot"
    };
    s.store.mutate(
        actor,
        MutationMeta::new("storyboard.link_shot", label, Capability::Edit)
            .target("storyboard_panel", &args.panel_id),
        |tx| link(tx, &args.panel_id, args.shot_id.as_deref()),
    )?;
    read_panel(core, &args.panel_id)
}

/// Path B (FSD §34): Screenplay scene → Storyboard → Shot List.
pub fn create_shot_from_panel(
    core: &AppCore,
    actor: &Actor,
    args: ShotFromPanelArgs,
) -> AppResult<ShotDto> {
    let s = core.project()?;
    let shot_id = s.store.mutate(actor, MutationMeta::new("storyboard.create_shot_from_panel", "Created shot from panel", Capability::Edit), |tx| {
        let c = tx.conn();
        let (board, _, _) = ensure_panel(c, &args.panel_id)?;
        let ctx = SceneCtx::load(c)?;
        let board_lineage: Option<String> = c.query_row("SELECT scene_lineage_id FROM storyboard WHERE id=?1", [&board], |r| r.get(0))?;
        let scene = match (&args.scene_id, board_lineage.as_deref().and_then(|l| ctx.by_lineage(l))) {
            (Some(sid), _) => ctx.resolve(c, sid)?,
            (None, Some(sc)) => sc.clone(),
            (None, None) => {
                return Err(AppError::new(
                    "validation.required",
                    "Choose the scene this shot belongs to. This storyboard isn't linked to a scene in the current script.",
                ));
            }
        };
        let (desc, framing, movement, angle, sound): (String, Option<String>, Option<String>, Option<String>, Option<String>) = c.query_row(
            "SELECT description, framing, movement, angle, sound_note FROM storyboard_panel WHERE id=?1",
            [&args.panel_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;
        let desc = if desc.trim().is_empty() { "Shot from storyboard panel".to_string() } else { desc };
        let shot_id = super::shots::insert_shot(
            tx,
            &ctx,
            &scene,
            super::shots::NewShot { description: desc, size: framing, movement, angle, sound_note: sound, ..Default::default() },
            None,
        )?;
        link(tx, &args.panel_id, Some(&shot_id))?;
        Ok(shot_id)
    })?;
    let root = root_of(core)?;
    s.store
        .read(|c| load_shot(c, &root, &SceneCtx::load(c)?, &shot_id))
}

// ------------------------------------------------------------------ search / trash

pub fn index_storyboard(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT name, scene_heading, deleted_at FROM storyboard WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((name, heading, None)) => Some(SearchDoc {
            entity_type: "storyboard".into(),
            title: name,
            body: heading.unwrap_or_default(),
            context: "Production · Storyboards".into(),
            nav: json!({ "workspace": "production", "sub": "storyboards", "storyboardId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

pub fn index_panel(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    #[allow(clippy::type_complexity)]
    let row: Option<(String, String, String, Option<String>, Option<String>, Option<String>, Option<i64>, Option<i64>)> = c
        .query_row(
            "SELECT p.storyboard_id, b.name, p.description, p.note, p.sound_note, p.framing, p.deleted_at, b.deleted_at
             FROM storyboard_panel p JOIN storyboard b ON b.id = p.storyboard_id WHERE p.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?;
    Ok(match row {
        Some((board, board_name, desc, note, sound, framing, None, None))
            if !desc.trim().is_empty() || note.is_some() =>
        {
            let title = if desc.trim().is_empty() {
                format!("Panel in {board_name}")
            } else {
                desc
            };
            Some(SearchDoc {
                entity_type: "storyboard_panel".into(),
                title,
                body: [Some(board_name), note, sound, framing]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                context: "Production · Storyboards".into(),
                nav: json!({ "workspace": "production", "sub": "storyboards", "storyboardId": board, "panelId": id }),
                owner_user_id: None,
            })
        }
        _ => None,
    })
}

/// Restore a panel to its storyboard and previous order. If the storyboard is
/// gone, the panel goes to an "Unassigned panels" storyboard (FSD §52.5).
pub fn restore_panel(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let board_ok = match &row.parent_id {
        Some(b) => c.query_row(
            "SELECT EXISTS(SELECT 1 FROM storyboard WHERE id=?1 AND deleted_at IS NULL)",
            [b],
            |r| r.get::<_, bool>(0),
        )?,
        None => false,
    };
    let board = if board_ok {
        row.parent_id.clone().unwrap_or_default()
    } else {
        unassigned_board(tx)?
    };
    let pos = next_position(c, "storyboard_panel", PANEL_SCOPE, &[text(&board)])?;
    c.execute(
        "UPDATE storyboard_panel SET deleted_at=NULL, storyboard_id=?1, position=?2, updated_at=?3, rev=rev+1 WHERE id=?4",
        params![board, pos, now_ms(), row.object_id],
    )?;
    let sibs = sibling_ids(c, "storyboard_panel", PANEL_SCOPE, &[text(&board)])?;
    let index = if board_ok {
        row.position.map(|p| p.max(0) as usize)
    } else {
        None
    };
    place_at(c, "storyboard_panel", sibs, &row.object_id, index)?;
    tx.reindex("storyboard_panel", &row.object_id);
    Ok(())
}

pub const UNASSIGNED_BOARD: &str = "Unassigned panels";

fn unassigned_board(tx: &Tx<'_>) -> AppResult<String> {
    let c = tx.conn();
    let existing: Option<String> = c
        .query_row(
            "SELECT id FROM storyboard WHERE name=?1 AND scene_lineage_id IS NULL AND deleted_at IS NULL ORDER BY position LIMIT 1",
            [UNASSIGNED_BOARD],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = new_id();
    let now = now_ms();
    let pos = next_position(c, "storyboard", "deleted_at IS NULL", &[])?;
    c.execute(
        "INSERT INTO storyboard(id, name, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, UNASSIGNED_BOARD, pos, now],
    )?;
    Ok(id)
}

fn purge_panel_row(tx: &Tx<'_>, panel_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let asset: Option<String> = c
        .query_row(
            "SELECT asset_id FROM storyboard_panel WHERE id=?1",
            [panel_id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    c.execute(
        "UPDATE shot SET storyboard_panel_id=NULL, updated_at=?1, rev=rev+1 WHERE storyboard_panel_id=?2",
        params![now_ms(), panel_id],
    )?;
    c.execute(
        "DELETE FROM deleted_item WHERE table_name='storyboard_panel' AND object_id=?1",
        [panel_id],
    )?;
    c.execute("DELETE FROM storyboard_panel WHERE id=?1", [panel_id])?;
    if let Some(a) = asset {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

pub fn purge_panel(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    purge_panel_row(tx, &row.object_id)
}

pub fn purge_storyboard(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut stmt = tx
            .conn()
            .prepare("SELECT id FROM storyboard_panel WHERE storyboard_id=?1")?;
        stmt.query_map([&row.object_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for id in ids {
        purge_panel_row(tx, &id)?;
    }
    tx.conn()
        .execute("DELETE FROM storyboard WHERE id=?1", [&row.object_id])?;
    Ok(())
}
