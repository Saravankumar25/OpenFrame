//! Shot lists (FSD §33, §103; FSD-SHOT-001, FSD-PROD-018..020).
//!
//! A shot belongs to a screenplay scene by identity (lineage). Its label
//! (e.g. "12B") is derived from the scene's displayed number plus the shot's
//! order within that scene — never stored, never renumbered by hand. A shot is
//! valid with only a description; every camera field is optional.

use std::path::Path;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, next_position, place_at, sibling_ids, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::common::{
    SHORT_MAX, TEXT_MAX, ingest_image_data, ingest_image_file, patch_text, shot_letters,
};
use super::scenes::{SceneCtx, SceneRow};
use super::storyboard::{PanelDto, insert_board, insert_panel, link, load_panel};
use crate::core::AppCore;
use crate::modules::files::purge_asset_if_unreferenced;
use crate::registry::SearchDoc;
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, load_asset_opt, required_text};

const SHOT_SCOPE: &str = "scene_lineage_id = ?1 AND deleted_at IS NULL";
const SHOT_FIELDS: &[&str] = &[
    "description",
    "size",
    "movement",
    "angle",
    "lens",
    "camera_notes",
    "characters_json",
    "sound_note",
    "reference_asset_id",
];
const DESCRIPTION_MAX: usize = 2_000;

// ------------------------------------------------------------------ DTOs

/// A storyboard panel attached to a shot (a shot may have several options).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShotPanelRef {
    pub panel_id: String,
    pub storyboard_id: String,
    pub storyboard_name: String,
    /// Panel number within its storyboard (derived).
    #[ts(type = "number")]
    pub number: i64,
    pub visual_kind: String,
    pub asset: Option<AssetInfo>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShotDto {
    pub id: String,
    /// Current scene row (resolved by identity), or the planned scene when removed.
    pub scene_id: String,
    pub scene_lineage_id: String,
    pub scene_number: Option<String>,
    pub scene_heading: String,
    /// The scene is no longer in the script; the shot is kept for reference (FSD §55).
    pub scene_removed: bool,
    /// Derived label, e.g. "12B" (letter only for a removed scene).
    pub label: String,
    /// 1-based order within the scene's coverage.
    #[ts(type = "number")]
    pub order: i64,
    pub description: String,
    pub size: Option<String>,
    pub movement: Option<String>,
    pub angle: Option<String>,
    pub lens: Option<String>,
    pub camera_notes: Option<String>,
    pub characters: Vec<String>,
    pub sound_note: Option<String>,
    pub reference_asset: Option<AssetInfo>,
    /// Primary storyboard panel (first attached).
    pub storyboard_panel_id: Option<String>,
    pub panels: Vec<ShotPanelRef>,
    /// "Scene changed since planning".
    pub needs_review: bool,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

pub struct ShotLabel {
    pub label: String,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShotListArgs {
    /// A scene row id (from any draft; resolved by identity).
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub scene_lineage_id: Option<String>,
    /// Also return shots of scenes removed from the script (hidden by default).
    #[serde(default)]
    #[ts(optional)]
    pub include_removed: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShotIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateShotArgs {
    pub scene_id: String,
    pub description: String,
    /// Insert position (0-based) in the scene's coverage; default = end ("Add Shot").
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<usize>,
    #[serde(default)]
    #[ts(optional)]
    pub size: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub movement: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub angle: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub lens: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub camera_notes: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub characters: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub sound_note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateShotArgs {
    pub id: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub expected_rev: Option<i64>,
    #[serde(default)]
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub size: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub movement: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub angle: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub lens: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub camera_notes: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub characters: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub sound_note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderShotArgs {
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveShotArgs {
    pub id: String,
    pub scene_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteShotsArgs {
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShotReferenceArgs {
    pub id: String,
    /// Image file to copy into the project; omit both to remove the reference image.
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
pub struct ShotPanelArgs {
    pub shot_id: String,
    pub panel_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyPlanningArgs {
    pub from_scene_lineage_id: String,
    pub to_scene_id: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CopyPlanningResult {
    #[ts(type = "number")]
    pub shots: i64,
    #[ts(type = "number")]
    pub storyboards: i64,
}

// ------------------------------------------------------------------ loading

fn root_of(core: &AppCore) -> AppResult<std::path::PathBuf> {
    Ok(core.project()?.layout.root().to_path_buf())
}

/// Derived label of a live shot, or None if it is deleted/missing.
pub fn shot_label(c: &Connection, ctx: &SceneCtx, shot_id: &str) -> AppResult<Option<ShotLabel>> {
    let lineage: Option<String> = c
        .query_row(
            "SELECT scene_lineage_id FROM shot WHERE id=?1 AND deleted_at IS NULL",
            [shot_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(lineage) = lineage else {
        return Ok(None);
    };
    let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
    let idx = sibs.iter().position(|s| s == shot_id).unwrap_or(0);
    Ok(Some(ShotLabel {
        label: make_label(ctx.by_lineage(&lineage).map(|s| ctx.number_label(s)), idx),
    }))
}

fn make_label(scene_number: Option<String>, idx: usize) -> String {
    format!("{}{}", scene_number.unwrap_or_default(), shot_letters(idx))
}

fn panels_for_shot(
    c: &Connection,
    root: &Path,
    shot_id: &str,
    primary: Option<&str>,
) -> AppResult<Vec<ShotPanelRef>> {
    let mut stmt = c.prepare(
        "SELECT p.id, p.storyboard_id, b.name, p.visual_kind, p.asset_id,
                (SELECT count(*) FROM storyboard_panel q WHERE q.storyboard_id = p.storyboard_id AND q.deleted_at IS NULL
                   AND (q.position < p.position OR (q.position = p.position AND q.id <= p.id)))
         FROM storyboard_panel p JOIN storyboard b ON b.id = p.storyboard_id
         WHERE p.shot_id = ?1 AND p.deleted_at IS NULL AND b.deleted_at IS NULL
         ORDER BY (p.id = ?2) DESC, b.position, p.position, p.id",
    )?;
    let rows = stmt
        .query_map(params![shot_id, primary.unwrap_or("")], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(panel_id, storyboard_id, storyboard_name, visual_kind, asset_id, number)| {
                Ok(ShotPanelRef {
                    panel_id,
                    storyboard_id,
                    storyboard_name,
                    number,
                    visual_kind,
                    asset: load_asset_opt(c, root, asset_id.as_deref())?,
                })
            },
        )
        .collect()
}

const SHOT_COLS: &str = "id, scene_id, scene_lineage_id, scene_heading, scene_hash, needs_review, description, size, movement,
     angle, lens, camera_notes, characters_json, sound_note, reference_asset_id, storyboard_panel_id, created_at, updated_at, rev";

fn shot_from_row(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    r: &rusqlite::Row<'_>,
    idx: usize,
) -> AppResult<ShotDto> {
    let id: String = r.get(0)?;
    let stored_scene: String = r.get(1)?;
    let lineage: String = r.get(2)?;
    let heading_snap: Option<String> = r.get(3)?;
    let hash: Option<String> = r.get(4)?;
    let flagged: bool = r.get(5)?;
    let chars_json: String = r.get(12)?;
    let reference: Option<String> = r.get(14)?;
    let primary: Option<String> = r.get(15)?;
    let (scene_removed, needs_review) = ctx.review_state(c, &lineage, hash.as_deref(), flagged)?;
    let current = ctx.by_lineage(&lineage);
    let scene_number = current.map(|s| ctx.number_label(s));
    let panels = panels_for_shot(c, root, &id, primary.as_deref())?;
    Ok(ShotDto {
        label: make_label(scene_number.clone(), idx),
        scene_id: current.map(|s| s.id.clone()).unwrap_or(stored_scene),
        scene_heading: current
            .map(|s| s.heading.clone())
            .or(heading_snap)
            .unwrap_or_default(),
        scene_number,
        scene_lineage_id: lineage,
        scene_removed,
        order: idx as i64 + 1,
        description: r.get(6)?,
        size: r.get(7)?,
        movement: r.get(8)?,
        angle: r.get(9)?,
        lens: r.get(10)?,
        camera_notes: r.get(11)?,
        characters: serde_json::from_str(&chars_json).unwrap_or_default(),
        sound_note: r.get(13)?,
        reference_asset: load_asset_opt(c, root, reference.as_deref())?,
        storyboard_panel_id: panels.first().map(|p| p.panel_id.clone()),
        panels,
        needs_review,
        created_at: r.get(16)?,
        updated_at: r.get(17)?,
        rev: r.get(18)?,
        id,
    })
}

/// All live shots of a scene identity, in coverage order.
pub fn load_lineage_shots(
    c: &Connection,
    root: &Path,
    ctx: &SceneCtx,
    lineage: &str,
) -> AppResult<Vec<ShotDto>> {
    let mut stmt = c.prepare(&format!(
        "SELECT {SHOT_COLS} FROM shot WHERE {SHOT_SCOPE} ORDER BY position, id"
    ))?;
    let mut rows = stmt.query([lineage])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let idx = out.len();
        out.push(shot_from_row(c, root, ctx, r, idx)?);
    }
    Ok(out)
}

pub fn load_shot(c: &Connection, root: &Path, ctx: &SceneCtx, id: &str) -> AppResult<ShotDto> {
    let lineage: String = c
        .query_row(
            "SELECT scene_lineage_id FROM shot WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("shot"))?;
    load_lineage_shots(c, root, ctx, &lineage)?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::not_found("shot"))
}

fn read_shot(core: &AppCore, id: &str) -> AppResult<ShotDto> {
    let root = root_of(core)?;
    core.project()?
        .store
        .read(|c| load_shot(c, &root, &SceneCtx::load(c)?, id))
}

pub fn list(core: &AppCore, actor: &Actor, args: ShotListArgs) -> AppResult<Vec<ShotDto>> {
    actor.require(Capability::View, "view shot lists")?;
    let root = root_of(core)?;
    core.project()?.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        if let Some(l) = &args.scene_lineage_id {
            return load_lineage_shots(c, &root, &ctx, l);
        }
        if let Some(sid) = &args.scene_id {
            let lineage = match ctx.by_id(sid) {
                Some(s) => s.lineage_id.clone(),
                None => c
                    .query_row("SELECT lineage_id FROM screenplay_scene WHERE id=?1", [sid], |r| r.get(0))
                    .optional()?
                    .ok_or_else(|| AppError::not_found("scene"))?,
            };
            return load_lineage_shots(c, &root, &ctx, &lineage);
        }
        // Whole project: active scenes in script order, then (optionally) removed scenes.
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for sc in &ctx.scenes {
            if seen.insert(sc.lineage_id.clone()) {
                out.extend(load_lineage_shots(c, &root, &ctx, &sc.lineage_id)?);
            }
        }
        if args.include_removed.unwrap_or(false) {
            let mut stmt = c.prepare("SELECT DISTINCT scene_lineage_id FROM shot WHERE deleted_at IS NULL ORDER BY scene_lineage_id")?;
            let lineages = stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
            for l in lineages.into_iter().filter(|l| ctx.by_lineage(l).is_none()) {
                out.extend(load_lineage_shots(c, &root, &ctx, &l)?);
            }
        }
        Ok(out)
    })
}

pub fn get(core: &AppCore, actor: &Actor, args: ShotIdArgs) -> AppResult<ShotDto> {
    actor.require(Capability::View, "view shot lists")?;
    read_shot(core, &args.id)
}

// ------------------------------------------------------------------ writes

#[derive(Default)]
pub struct NewShot {
    pub description: String,
    pub size: Option<String>,
    pub movement: Option<String>,
    pub angle: Option<String>,
    pub lens: Option<String>,
    pub camera_notes: Option<String>,
    pub characters: Vec<String>,
    pub sound_note: Option<String>,
    pub reference_asset_id: Option<String>,
}

fn clean_opt(v: Option<String>, what: &str, max: usize) -> AppResult<Option<String>> {
    crate::util::optional_text(v, what, max)
}

fn clean_characters(v: Vec<String>) -> AppResult<Vec<String>> {
    if v.len() > 60 {
        return Err(AppError::invalid_input(
            "A shot can list at most 60 characters.",
        ));
    }
    let mut out: Vec<String> = Vec::new();
    for c in v {
        let c = c.trim().to_string();
        if c.is_empty() {
            continue;
        }
        if c.chars().count() > 80 {
            return Err(AppError::invalid_input(
                "A character name is too long (maximum 80 characters).",
            ));
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(&c)) {
            out.push(c);
        }
    }
    Ok(out)
}

impl NewShot {
    fn validated(self) -> AppResult<NewShot> {
        Ok(NewShot {
            description: required_text(&self.description, "Description", DESCRIPTION_MAX)?,
            size: clean_opt(self.size, "Shot size", SHORT_MAX)?,
            movement: clean_opt(self.movement, "Movement", SHORT_MAX)?,
            angle: clean_opt(self.angle, "Angle", SHORT_MAX)?,
            lens: clean_opt(self.lens, "Lens", SHORT_MAX)?,
            camera_notes: clean_opt(self.camera_notes, "Camera notes", TEXT_MAX)?,
            characters: clean_characters(self.characters)?,
            sound_note: clean_opt(self.sound_note, "Sound note", TEXT_MAX)?,
            reference_asset_id: self.reference_asset_id,
        })
    }
}

/// Insert a shot into a scene's coverage (default: appended at the end).
pub fn insert_shot(
    tx: &Tx<'_>,
    ctx: &SceneCtx,
    scene: &SceneRow,
    shot: NewShot,
    index: Option<usize>,
) -> AppResult<String> {
    let shot = shot.validated()?;
    let c = tx.conn();
    let (hash, _) = ctx.hash(c, &scene.id)?;
    let id = new_id();
    let now = now_ms();
    let pos = next_position(c, "shot", SHOT_SCOPE, &[text(&scene.lineage_id)])?;
    let chars =
        serde_json::to_string(&shot.characters).map_err(|e| AppError::internal(e.to_string()))?;
    c.execute(
        "INSERT INTO shot(id, scene_id, scene_lineage_id, scene_heading, scene_hash, position, description, size, movement, angle,
                          lens, camera_notes, characters_json, sound_note, reference_asset_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
        params![
            id,
            scene.id,
            scene.lineage_id,
            scene.heading,
            hash,
            pos,
            shot.description,
            shot.size,
            shot.movement,
            shot.angle,
            shot.lens,
            shot.camera_notes,
            chars,
            shot.sound_note,
            shot.reference_asset_id,
            now
        ],
    )?;
    if let Some(i) = index {
        let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&scene.lineage_id)])?;
        place_at(c, "shot", sibs, &id, Some(i))?;
    }
    Ok(id)
}

fn short(s: &str) -> String {
    let t: String = s.trim().chars().take(48).collect();
    if s.trim().chars().count() > 48 {
        format!("{t}…")
    } else {
        t
    }
}

pub fn create(core: &AppCore, actor: &Actor, args: CreateShotArgs) -> AppResult<ShotDto> {
    let s = core.project()?;
    let summary = format!("Added shot “{}”", short(&args.description));
    let id = s.store.mutate(
        actor,
        MutationMeta::new("shot.create", summary, Capability::Edit),
        |tx| {
            let ctx = SceneCtx::load(tx.conn())?;
            let scene = ctx.resolve(tx.conn(), &args.scene_id)?;
            insert_shot(
                tx,
                &ctx,
                &scene,
                NewShot {
                    description: args.description.clone(),
                    size: args.size.clone(),
                    movement: args.movement.clone(),
                    angle: args.angle.clone(),
                    lens: args.lens.clone(),
                    camera_notes: args.camera_notes.clone(),
                    characters: args.characters.clone().unwrap_or_default(),
                    sound_note: args.sound_note.clone(),
                    reference_asset_id: None,
                },
                args.index,
            )
        },
    )?;
    read_shot(core, &id)
}

fn ensure_shot(c: &Connection, id: &str) -> AppResult<(String, Option<String>)> {
    c.query_row(
        "SELECT scene_lineage_id, reference_asset_id FROM shot WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("shot"))
}

pub fn update(core: &AppCore, actor: &Actor, args: UpdateShotArgs) -> AppResult<ShotDto> {
    let mut fields: Vec<(&'static str, SqlValue)> = Vec::new();
    if let Some(d) = &args.description {
        fields.push((
            "description",
            text(required_text(d, "Description", DESCRIPTION_MAX)?),
        ));
    }
    patch_text(
        &mut fields,
        "size",
        args.size.clone(),
        "Shot size",
        SHORT_MAX,
    )?;
    patch_text(
        &mut fields,
        "movement",
        args.movement.clone(),
        "Movement",
        SHORT_MAX,
    )?;
    patch_text(&mut fields, "angle", args.angle.clone(), "Angle", SHORT_MAX)?;
    patch_text(&mut fields, "lens", args.lens.clone(), "Lens", SHORT_MAX)?;
    patch_text(
        &mut fields,
        "camera_notes",
        args.camera_notes.clone(),
        "Camera notes",
        TEXT_MAX,
    )?;
    patch_text(
        &mut fields,
        "sound_note",
        args.sound_note.clone(),
        "Sound note",
        TEXT_MAX,
    )?;
    if let Some(chars) = args.characters.clone() {
        let chars = clean_characters(chars)?;
        fields.push((
            "characters_json",
            text(serde_json::to_string(&chars).map_err(|e| AppError::internal(e.to_string()))?),
        ));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("shot.update", "Edited shot", Capability::Edit)
            .target("shot", &args.id)
            .coalesce(format!("shot.update:{}", args.id)),
        |tx| {
            ensure_shot(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "shot",
                &args.id,
                &fields,
                SHOT_FIELDS,
                args.expected_rev,
                "shot",
            )?;
            Ok(())
        },
    )?;
    read_shot(core, &args.id)
}

pub fn reorder(core: &AppCore, actor: &Actor, args: ReorderShotArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("shot.reorder", "Reordered shots", Capability::Edit)
            .target("shot", &args.id),
        |tx| {
            let (lineage, _) = ensure_shot(tx.conn(), &args.id)?;
            let sibs = sibling_ids(tx.conn(), "shot", SHOT_SCOPE, &[text(&lineage)])?;
            place_at(tx.conn(), "shot", sibs, &args.id, Some(args.index))?;
            Ok(())
        },
    )
}

pub fn move_to_scene(core: &AppCore, actor: &Actor, args: MoveShotArgs) -> AppResult<ShotDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("shot.move", "Moved shot to another scene", Capability::Edit)
            .target("shot", &args.id),
        |tx| {
            let c = tx.conn();
            let (from, _) = ensure_shot(c, &args.id)?;
            let ctx = SceneCtx::load(c)?;
            let scene = ctx.resolve(c, &args.scene_id)?;
            if scene.lineage_id != from {
                let (hash, _) = ctx.hash(c, &scene.id)?;
                let pos = next_position(c, "shot", SHOT_SCOPE, &[text(&scene.lineage_id)])?;
                update_fields(
                    c,
                    "shot",
                    &args.id,
                    &[
                        ("scene_id", text(scene.id.clone())),
                        ("scene_lineage_id", text(scene.lineage_id.clone())),
                        ("scene_heading", text(scene.heading.clone())),
                        ("scene_hash", text(hash)),
                        ("needs_review", int(0)),
                        ("position", int(pos)),
                    ],
                    &[
                        "scene_id",
                        "scene_lineage_id",
                        "scene_heading",
                        "scene_hash",
                        "needs_review",
                        "position",
                    ],
                    None,
                    "shot",
                )?;
            }
            let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&scene.lineage_id)])?;
            place_at(c, "shot", sibs, &args.id, args.index)?;
            Ok(())
        },
    )?;
    read_shot(core, &args.id)
}

pub fn duplicate(core: &AppCore, actor: &Actor, args: ShotIdArgs) -> AppResult<ShotDto> {
    let s = core.project()?;
    let new_id = s.store.mutate(actor, MutationMeta::new("shot.duplicate", "Duplicated shot", Capability::Edit).target("shot", &args.id), |tx| {
        let c = tx.conn();
        let (lineage, _) = ensure_shot(c, &args.id)?;
        let id = new_id();
        let now = now_ms();
        let pos = next_position(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
        c.execute(
            "INSERT INTO shot(id, scene_id, scene_lineage_id, scene_heading, scene_hash, needs_review, position, description, size,
                              movement, angle, lens, camera_notes, characters_json, sound_note, reference_asset_id,
                              created_at, updated_at)
             SELECT ?1, scene_id, scene_lineage_id, scene_heading, scene_hash, needs_review, ?2, description, size,
                    movement, angle, lens, camera_notes, characters_json, sound_note, reference_asset_id, ?3, ?3
             FROM shot WHERE id=?4",
            params![id, pos, now, args.id],
        )?;
        let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
        let at = sibs.iter().position(|x| x == &args.id).map(|i| i + 1);
        place_at(c, "shot", sibs, &id, at)?;
        Ok(id)
    })?;
    read_shot(core, &new_id)
}

pub fn delete(core: &AppCore, actor: &Actor, args: DeleteShotsArgs) -> AppResult<()> {
    if args.ids.is_empty() {
        return Ok(());
    }
    if args.ids.len() > 500 {
        return Err(AppError::invalid_input(
            "Delete at most 500 shots at a time.",
        ));
    }
    let s = core.project()?;
    // Titles use the labels shown before the delete.
    let titles: Vec<(String, String, i64)> = s.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        args.ids
            .iter()
            .map(|id| {
                let (lineage, _) = ensure_shot(c, id)?;
                let desc: String =
                    c.query_row("SELECT description FROM shot WHERE id=?1", [id], |r| {
                        r.get(0)
                    })?;
                let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
                let idx = sibs.iter().position(|x| x == id).unwrap_or(0);
                let label =
                    make_label(ctx.by_lineage(&lineage).map(|sc| ctx.number_label(sc)), idx);
                Ok((
                    format!("Shot {label} — {}", short(&desc)),
                    lineage,
                    idx as i64,
                ))
            })
            .collect()
    })?;
    let summary = if args.ids.len() == 1 {
        format!("Deleted {}", titles[0].0)
    } else {
        format!("Deleted {} shots", args.ids.len())
    };
    s.store.mutate(
        actor,
        MutationMeta::new("shot.delete", summary, Capability::SoftDelete),
        |tx| {
            for (id, (title, lineage, idx)) in args.ids.iter().zip(titles.iter()) {
                // Linked panels keep their link on the row; it is hidden while the shot is deleted.
                soft_delete(
                    tx,
                    DeleteSpec {
                        object_type: "shot",
                        table: "shot",
                        id,
                        title: Some(title.clone()),
                        parent_type: Some("scene"),
                        parent_id: Some(lineage.clone()),
                        position: Some(*idx),
                    },
                )?;
            }
            Ok(())
        },
    )
}

pub fn set_reference_image(
    core: &AppCore,
    actor: &Actor,
    args: ShotReferenceArgs,
) -> AppResult<ShotDto> {
    let path = args.path.clone().filter(|p| !p.trim().is_empty());
    let data = args.data_base64.clone().filter(|d| !d.trim().is_empty());
    let label = if path.is_some() || data.is_some() {
        "Set shot reference image"
    } else {
        "Removed shot reference image"
    };
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("shot.set_reference_image", label, Capability::Edit)
            .target("shot", &args.id),
        |tx| {
            ensure_shot(tx.conn(), &args.id)?;
            let asset = match (path, data) {
                (Some(p), _) => Some(ingest_image_file(tx, &p)?.id),
                (None, Some(d)) => Some(ingest_image_data(tx, &d, "reference")?.id),
                (None, None) => None,
            };
            update_fields(
                tx.conn(),
                "shot",
                &args.id,
                &[(
                    "reference_asset_id",
                    asset.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                )],
                SHOT_FIELDS,
                None,
                "shot",
            )?;
            Ok(())
        },
    )?;
    read_shot(core, &args.id)
}

pub fn attach_panel(core: &AppCore, actor: &Actor, args: ShotPanelArgs) -> AppResult<ShotDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "shot.attach_panel",
            "Attached storyboard panel to shot",
            Capability::Edit,
        )
        .target("shot", &args.shot_id),
        |tx| {
            ensure_shot(tx.conn(), &args.shot_id)?;
            link(tx, &args.panel_id, Some(&args.shot_id))
        },
    )?;
    read_shot(core, &args.shot_id)
}

pub fn detach_panel(core: &AppCore, actor: &Actor, args: ShotPanelArgs) -> AppResult<ShotDto> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "shot.detach_panel",
            "Detached storyboard panel from shot",
            Capability::Edit,
        )
        .target("shot", &args.shot_id),
        |tx| {
            ensure_shot(tx.conn(), &args.shot_id)?;
            let linked: Option<String> = tx
                .conn()
                .query_row(
                    "SELECT shot_id FROM storyboard_panel WHERE id=?1",
                    [&args.panel_id],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            if linked.as_deref() != Some(args.shot_id.as_str()) {
                return Err(AppError::invalid_input(
                    "That panel isn't attached to this shot.",
                ));
            }
            link(tx, &args.panel_id, None)
        },
    )?;
    read_shot(core, &args.shot_id)
}

/// Path A (FSD §34): Screenplay scene → Shot List → Storyboard. Creates a panel
/// for the shot in the scene's storyboard (creating that storyboard if needed).
pub fn create_panel(core: &AppCore, actor: &Actor, args: ShotIdArgs) -> AppResult<PanelDto> {
    let s = core.project()?;
    let panel_id = s.store.mutate(actor, MutationMeta::new("shot.create_panel", "Created storyboard panel from shot", Capability::Edit).target("shot", &args.id), |tx| {
        let c = tx.conn();
        let (lineage, reference) = ensure_shot(c, &args.id)?;
        let ctx = SceneCtx::load(c)?;
        let board: Option<String> = c
            .query_row(
                "SELECT id FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL ORDER BY position, id LIMIT 1",
                [&lineage],
                |r| r.get(0),
            )
            .optional()?;
        let board = match board {
            Some(b) => b,
            None => {
                let scene = ctx.by_lineage(&lineage).cloned().ok_or_else(|| {
                    AppError::new(
                        "conflict.state",
                        "This shot's scene is no longer in the script, so a new storyboard can't be created for it.",
                    )
                })?;
                insert_board(tx, &ctx, None, Some(&scene.id))?
            }
        };
        let (desc, size, movement, angle): (String, Option<String>, Option<String>, Option<String>) = c.query_row(
            "SELECT description, size, movement, angle FROM shot WHERE id=?1",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        // The shot's reference image becomes the panel visual when there is one.
        let kind = if reference.is_some() { "image" } else { "placeholder" };
        let pid = insert_panel(tx, &board, kind, reference.as_deref(), &desc, size.as_deref(), movement.as_deref(), angle.as_deref(), None, None)?;
        link(tx, &pid, Some(&args.id))?;
        Ok(pid)
    })?;
    let root = root_of(core)?;
    s.store
        .read(|c| load_panel(c, &root, &SceneCtx::load(c)?, &panel_id))
}

/// "Copy Planning" (FSD §55): copy a scene's shots and scene storyboards to another
/// scene (e.g. a duplicated scene). The copies are new, independent identities.
pub fn copy_planning(
    core: &AppCore,
    actor: &Actor,
    args: CopyPlanningArgs,
) -> AppResult<CopyPlanningResult> {
    let s = core.project()?;
    s.store.mutate(actor, MutationMeta::new("shot.copy_planning", "Copied scene planning", Capability::Edit), |tx| {
        let c = tx.conn();
        let ctx = SceneCtx::load(c)?;
        let target = ctx.resolve(c, &args.to_scene_id)?;
        if target.lineage_id == args.from_scene_lineage_id {
            return Err(AppError::invalid_input("Choose a different scene to copy the planning to."));
        }
        let (hash, _) = ctx.hash(c, &target.id)?;
        let now = now_ms();
        let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let src_shots = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&args.from_scene_lineage_id)])?;
        for old in &src_shots {
            let id = new_id();
            let pos = next_position(c, "shot", SHOT_SCOPE, &[text(&target.lineage_id)])?;
            c.execute(
                "INSERT INTO shot(id, scene_id, scene_lineage_id, scene_heading, scene_hash, position, description, size, movement,
                                  angle, lens, camera_notes, characters_json, sound_note, reference_asset_id, created_at, updated_at)
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6, description, size, movement, angle, lens, camera_notes, characters_json,
                        sound_note, reference_asset_id, ?7, ?7 FROM shot WHERE id=?8",
                params![id, target.id, target.lineage_id, target.heading, hash, pos, now, old],
            )?;
            map.insert(old.clone(), id);
        }
        let boards: Vec<(String, String)> = {
            let mut stmt = c.prepare(
                "SELECT id, name FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL ORDER BY position, id",
            )?;
            stmt.query_map([&args.from_scene_lineage_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?
        };
        for (old_board, name) in &boards {
            let board = insert_board(tx, &ctx, Some(name.clone()), Some(&target.id))?;
            let panels = sibling_ids(c, "storyboard_panel", "storyboard_id = ?1 AND deleted_at IS NULL", &[text(old_board)])?;
            for p in panels {
                #[allow(clippy::type_complexity)]
                let (kind, asset, desc, framing, movement, angle, sound, dur, note, shot): (
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
                ) = c.query_row(
                    "SELECT visual_kind, asset_id, description, framing, movement, angle, sound_note, duration_ms, note, shot_id
                     FROM storyboard_panel WHERE id=?1",
                    [&p],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
                )?;
                let new_shot = shot.and_then(|s| map.get(&s).cloned());
                let pid = insert_panel(tx, &board, &kind, asset.as_deref(), &desc, framing.as_deref(), movement.as_deref(), angle.as_deref(), None, None)?;
                c.execute(
                    "UPDATE storyboard_panel SET sound_note=?1, duration_ms=?2, note=?3 WHERE id=?4",
                    params![sound, dur, note, pid],
                )?;
                if let Some(sid) = new_shot {
                    link(tx, &pid, Some(&sid))?;
                }
            }
        }
        if src_shots.is_empty() && boards.is_empty() {
            return Err(AppError::invalid_input("There is no planning to copy for that scene."));
        }
        Ok(CopyPlanningResult { shots: src_shots.len() as i64, storyboards: boards.len() as i64 })
    })
}

// ------------------------------------------------------------------ search / trash

pub fn index_shot(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    #[allow(clippy::type_complexity)]
    let row: Option<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<i64>,
    )> = c
        .query_row(
            "SELECT description, scene_heading, size, movement, angle, lens, camera_notes, characters_json, sound_note, deleted_at
             FROM shot WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
        )
        .optional()?;
    Ok(match row {
        Some((desc, heading, size, movement, angle, lens, notes, chars, sound, None)) => {
            let chars: Vec<String> = serde_json::from_str(&chars).unwrap_or_default();
            let body = [
                heading,
                size,
                movement,
                angle,
                lens,
                notes,
                sound,
                Some(chars.join(" ")),
            ]
            .into_iter()
            .flatten()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
            Some(SearchDoc {
                entity_type: "shot".into(),
                title: desc,
                body,
                context: "Production · Shot List".into(),
                nav: json!({ "workspace": "production", "sub": "shots", "shotId": id }),
                owner_user_id: None,
            })
        }
        _ => None,
    })
}

/// Restore a shot to its scene and previous order. Shots belong to a scene identity,
/// so they always have a home; if the scene has since left the script the shot is
/// listed with that scene's kept (removed-scene) planning.
pub fn restore_shot(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let lineage: String = c.query_row(
        "SELECT scene_lineage_id FROM shot WHERE id=?1",
        [&row.object_id],
        |r| r.get(0),
    )?;
    let pos = next_position(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
    c.execute(
        "UPDATE shot SET deleted_at=NULL, position=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
        params![pos, now_ms(), row.object_id],
    )?;
    let sibs = sibling_ids(c, "shot", SHOT_SCOPE, &[text(&lineage)])?;
    place_at(
        c,
        "shot",
        sibs,
        &row.object_id,
        row.position.map(|p| p.max(0) as usize),
    )?;
    Ok(())
}

pub fn purge_shot(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let asset: Option<String> = c
        .query_row(
            "SELECT reference_asset_id FROM shot WHERE id=?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    // Panels stay; only their link to the purged shot is removed.
    c.execute(
        "UPDATE storyboard_panel SET shot_id=NULL, updated_at=?1, rev=rev+1 WHERE shot_id=?2",
        params![now_ms(), row.object_id],
    )?;
    c.execute("DELETE FROM shot WHERE id=?1", [&row.object_id])?;
    if let Some(a) = asset {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}
