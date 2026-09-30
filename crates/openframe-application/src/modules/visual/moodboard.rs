//! Moodboards (FSD §31, §101; FSD-PROD-011/012).
//!
//! A moodboard is a named, freeform canvas of image / note / link tiles. Tiles
//! can be moved (also as a multi-selection), resized and brought to the front or
//! sent to the back. Board names are free text; the common names are offered
//! as suggestions only. Notes marked internal/private are flagged so exports can
//! leave them out (FSD §31.5).

use std::path::Path;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{
    int, next_position, opt_text, place_at, sibling_ids, text, update_fields,
};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::common::{
    NAME_MAX, TEXT_MAX, ingest_image_data, ingest_image_file, normalize_url, url_host,
    vault_item_image,
};
use super::scenes::SceneCtx;
use crate::core::AppCore;
use crate::modules::files::purge_asset_if_unreferenced;
use crate::registry::SearchDoc;
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{
    AssetInfo, StoreSel, copy_asset_between, load_asset_opt, optional_text, required_text,
};

/// FSD §31.2 — offered as suggestions only; any name is allowed.
pub const SUGGESTED_NAMES: &[&str] = &[
    "Overall Look",
    "Cinematography",
    "Production Design",
    "Costume",
    "Lighting",
    "Character",
    "Location",
];

const CAPTION_MAX: usize = 300;
const CANVAS_MAX: i64 = 20_000;
const TILE_MIN: i64 = 40;
const TILE_MAX: i64 = 4_000;
const ITEM_FIELDS: &[&str] = &["caption", "body", "url", "link_title", "is_private"];

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MoodboardSummary {
    pub id: String,
    pub name: String,
    #[ts(type = "number")]
    pub item_count: i64,
    /// Scene this board is a reference for (FSD §34.1 "Moodboard reference").
    pub scene_lineage_id: Option<String>,
    pub scene_number: Option<String>,
    pub scene_heading: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MoodboardItemDto {
    pub id: String,
    pub moodboard_id: String,
    /// "image", "note" or "link".
    pub kind: String,
    #[ts(type = "number")]
    pub x: i64,
    #[ts(type = "number")]
    pub y: i64,
    #[ts(type = "number")]
    pub w: i64,
    #[ts(type = "number")]
    pub h: i64,
    #[ts(type = "number")]
    pub z: i64,
    pub caption: Option<String>,
    pub body: Option<String>,
    pub url: Option<String>,
    /// Host shown under a link tile, e.g. "imdb.com".
    pub url_host: Option<String>,
    pub link_title: Option<String>,
    pub asset: Option<AssetInfo>,
    pub source_vault_item_id: Option<String>,
    pub is_private: bool,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct MoodboardDto {
    pub board: MoodboardSummary,
    pub notes: Option<String>,
    pub items: Vec<MoodboardItemDto>,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoodboardListArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoodboardIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateMoodboardArgs {
    pub name: String,
    /// Optional scene this board is a reference for.
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameMoodboardArgs {
    pub id: String,
    pub name: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoodboardNotesArgs {
    pub id: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderMoodboardArgs {
    pub id: String,
    pub index: usize,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMoodboardImagesArgs {
    pub moodboard_id: String,
    pub paths: Vec<String>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub x: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub y: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMoodboardImageDataArgs {
    pub moodboard_id: String,
    /// Base64 image data (a pasted image).
    pub data_base64: String,
    #[serde(default)]
    #[ts(optional)]
    pub file_name: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub x: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub y: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMoodboardVaultImageArgs {
    pub moodboard_id: String,
    pub vault_item_id: String,
    /// Project Idea Vault (default) or the Global Idea Vault.
    #[serde(default)]
    #[ts(optional)]
    pub store: Option<StoreSel>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub x: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub y: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMoodboardNoteArgs {
    pub moodboard_id: String,
    pub text: String,
    /// Internal/private note (left out of standard exports).
    #[serde(default)]
    #[ts(optional)]
    pub is_private: Option<bool>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub x: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub y: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddMoodboardLinkArgs {
    pub moodboard_id: String,
    pub url: String,
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub x: Option<i64>,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub y: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateMoodboardItemArgs {
    pub id: String,
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub expected_rev: Option<i64>,
    /// Empty string clears the caption.
    #[serde(default)]
    #[ts(optional)]
    pub caption: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub body: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub url: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub link_title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub is_private: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoodboardItemMove {
    pub id: String,
    #[ts(type = "number")]
    pub x: i64,
    #[ts(type = "number")]
    pub y: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveMoodboardItemsArgs {
    pub moves: Vec<MoodboardItemMove>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeMoodboardItemArgs {
    pub id: String,
    #[ts(type = "number")]
    pub w: i64,
    #[ts(type = "number")]
    pub h: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArrangeMoodboardItemArgs {
    pub id: String,
    /// "front" or "back".
    pub placement: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteMoodboardItemsArgs {
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultImagesArgs {
    #[serde(default)]
    #[ts(optional)]
    pub store: Option<StoreSel>,
}

// ------------------------------------------------------------------ loading

fn root_of(core: &AppCore) -> AppResult<std::path::PathBuf> {
    Ok(core.project()?.layout.root().to_path_buf())
}

fn load_summary(c: &Connection, ctx: &SceneCtx, id: &str) -> AppResult<MoodboardSummary> {
    let (id, name, lineage, rev): (String, String, Option<String>, i64) = c
        .query_row("SELECT id, name, scene_lineage_id, rev FROM moodboard WHERE id=?1 AND deleted_at IS NULL", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .optional()?
        .ok_or_else(|| AppError::not_found("moodboard"))?;
    let item_count: i64 = c.query_row(
        "SELECT count(*) FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL",
        [&id],
        |r| r.get(0),
    )?;
    let scene = lineage.as_deref().and_then(|l| ctx.by_lineage(l));
    Ok(MoodboardSummary {
        scene_number: scene.map(|s| ctx.number_label(s)),
        scene_heading: scene.map(|s| s.heading.clone()),
        id,
        name,
        item_count,
        scene_lineage_id: lineage,
        rev,
    })
}

const ITEM_COLS: &str = "id, moodboard_id, kind, x, y, w, h, z, caption, body, url, link_title, asset_id, source_vault_item_id, is_private, rev";

fn item_from_row(
    c: &Connection,
    root: &Path,
    r: &rusqlite::Row<'_>,
) -> AppResult<MoodboardItemDto> {
    let url: Option<String> = r.get(10)?;
    let asset_id: Option<String> = r.get(12)?;
    Ok(MoodboardItemDto {
        id: r.get(0)?,
        moodboard_id: r.get(1)?,
        kind: r.get(2)?,
        x: r.get(3)?,
        y: r.get(4)?,
        w: r.get(5)?,
        h: r.get(6)?,
        z: r.get(7)?,
        caption: r.get(8)?,
        body: r.get(9)?,
        url_host: url.as_deref().map(url_host),
        url,
        link_title: r.get(11)?,
        asset: load_asset_opt(c, root, asset_id.as_deref())?,
        source_vault_item_id: r.get(13)?,
        is_private: r.get(14)?,
        rev: r.get(15)?,
    })
}

fn load_item(c: &Connection, root: &Path, id: &str) -> AppResult<MoodboardItemDto> {
    let mut stmt = c.prepare(&format!(
        "SELECT {ITEM_COLS} FROM moodboard_item WHERE id=?1 AND deleted_at IS NULL"
    ))?;
    let mut rows = stmt.query([id])?;
    match rows.next()? {
        Some(r) => item_from_row(c, root, r),
        None => Err(AppError::not_found("moodboard item")),
    }
}

fn load_items(c: &Connection, root: &Path, board: &str) -> AppResult<Vec<MoodboardItemDto>> {
    let mut stmt =
        c.prepare(&format!("SELECT {ITEM_COLS} FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL ORDER BY z, id"))?;
    let mut rows = stmt.query([board])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        out.push(item_from_row(c, root, r)?);
    }
    Ok(out)
}

fn read_summary(core: &AppCore, id: &str) -> AppResult<MoodboardSummary> {
    core.project()?
        .store
        .read(|c| load_summary(c, &SceneCtx::load(c)?, id))
}

fn read_items(core: &AppCore, ids: &[String]) -> AppResult<Vec<MoodboardItemDto>> {
    let root = root_of(core)?;
    core.project()?
        .store
        .read(|c| ids.iter().map(|id| load_item(c, &root, id)).collect())
}

pub fn list(
    core: &AppCore,
    actor: &Actor,
    _: MoodboardListArgs,
) -> AppResult<Vec<MoodboardSummary>> {
    actor.require(Capability::View, "view moodboards")?;
    core.project()?.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let ids = sibling_ids(c, "moodboard", "deleted_at IS NULL", &[])?;
        ids.iter().map(|id| load_summary(c, &ctx, id)).collect()
    })
}

pub fn suggestions(core: &AppCore, actor: &Actor, _: MoodboardListArgs) -> AppResult<Vec<String>> {
    actor.require(Capability::View, "view moodboards")?;
    core.project()?.store.read(|c| {
        let mut stmt = c.prepare("SELECT name FROM moodboard WHERE deleted_at IS NULL")?;
        let used: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(SUGGESTED_NAMES
            .iter()
            .filter(|n| !used.iter().any(|u| u.trim().eq_ignore_ascii_case(n)))
            .map(|n| n.to_string())
            .collect())
    })
}

pub fn get(core: &AppCore, actor: &Actor, args: MoodboardIdArgs) -> AppResult<MoodboardDto> {
    actor.require(Capability::View, "view moodboards")?;
    let root = root_of(core)?;
    core.project()?.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let board = load_summary(c, &ctx, &args.id)?;
        let notes: Option<String> =
            c.query_row("SELECT notes FROM moodboard WHERE id=?1", [&args.id], |r| {
                r.get(0)
            })?;
        Ok(MoodboardDto {
            board,
            notes,
            items: load_items(c, &root, &args.id)?,
        })
    })
}

pub fn vault_images(
    core: &AppCore,
    actor: &Actor,
    args: VaultImagesArgs,
) -> AppResult<Vec<super::common::VaultImageRef>> {
    actor.require(Capability::View, "view the Idea Vault")?;
    crate::util::with_store(core, args.store.unwrap_or_default(), |s| {
        let root = s.root().to_path_buf();
        s.read(|c| super::common::vault_images(c, &root, 500))
    })
}

// ------------------------------------------------------------------ boards

pub fn create(
    core: &AppCore,
    actor: &Actor,
    args: CreateMoodboardArgs,
) -> AppResult<MoodboardSummary> {
    let name = required_text(&args.name, "Board name", NAME_MAX)?;
    let s = core.project()?;
    let id = new_id();
    s.store.mutate(
        actor,
        MutationMeta::new("moodboard.create", format!("Created moodboard “{name}”"), Capability::Edit).target("moodboard", &id),
        |tx| {
            let c = tx.conn();
            let (scene_id, lineage) = match &args.scene_id {
                Some(sid) => {
                    let ctx = SceneCtx::load(c)?;
                    let sc = ctx.resolve(c, sid)?;
                    (Some(sc.id), Some(sc.lineage_id))
                }
                None => (None, None),
            };
            let now = now_ms();
            let pos = next_position(c, "moodboard", "deleted_at IS NULL", &[])?;
            c.execute(
                "INSERT INTO moodboard(id, name, scene_id, scene_lineage_id, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, name, scene_id, lineage, pos, now],
            )?;
            Ok(())
        },
    )?;
    read_summary(core, &id)
}

pub fn rename(
    core: &AppCore,
    actor: &Actor,
    args: RenameMoodboardArgs,
) -> AppResult<MoodboardSummary> {
    let name = required_text(&args.name, "Board name", NAME_MAX)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.rename",
            format!("Renamed moodboard to “{name}”"),
            Capability::Edit,
        )
        .target("moodboard", &args.id),
        |tx| {
            ensure_board(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "moodboard",
                &args.id,
                &[("name", text(name.clone()))],
                &["name"],
                args.expected_rev,
                "moodboard",
            )?;
            Ok(())
        },
    )?;
    read_summary(core, &args.id)
}

pub fn update_notes(core: &AppCore, actor: &Actor, args: MoodboardNotesArgs) -> AppResult<()> {
    let notes = optional_text(args.notes, "Notes", 20_000)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.update_notes",
            "Edited moodboard notes",
            Capability::Edit,
        )
        .target("moodboard", &args.id)
        .coalesce(format!("moodboard.notes:{}", args.id)),
        |tx| {
            ensure_board(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "moodboard",
                &args.id,
                &[("notes", opt_text(notes.clone()))],
                &["notes"],
                None,
                "moodboard",
            )?;
            Ok(())
        },
    )
}

pub fn reorder(core: &AppCore, actor: &Actor, args: ReorderMoodboardArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.reorder",
            "Reordered moodboards",
            Capability::Edit,
        )
        .target("moodboard", &args.id),
        |tx| {
            ensure_board(tx.conn(), &args.id)?;
            let sibs = sibling_ids(tx.conn(), "moodboard", "deleted_at IS NULL", &[])?;
            place_at(tx.conn(), "moodboard", sibs, &args.id, Some(args.index))?;
            Ok(())
        },
    )
}

pub fn delete(core: &AppCore, actor: &Actor, args: MoodboardIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (name, idx) = s.store.read(|c| {
        let name: String = c
            .query_row(
                "SELECT name FROM moodboard WHERE id=?1 AND deleted_at IS NULL",
                [&args.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("moodboard"))?;
        let sibs = sibling_ids(c, "moodboard", "deleted_at IS NULL", &[])?;
        Ok((
            name,
            sibs.iter().position(|x| x == &args.id).unwrap_or(0) as i64,
        ))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.delete",
            format!("Deleted moodboard “{name}”"),
            Capability::SoftDelete,
        )
        .target("moodboard", &args.id),
        |tx| {
            // Items stay on the (recoverable) board. Touching them keeps their search
            // entries in step with the board through delete, undo and restore.
            touch_items(tx, &args.id)?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "moodboard",
                    table: "moodboard",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: Some(idx),
                },
            )
        },
    )
}

fn touch_items(tx: &Tx<'_>, board: &str) -> AppResult<()> {
    tx.conn().execute(
        "UPDATE moodboard_item SET rev=rev+1, updated_at=?1 WHERE moodboard_id=?2 AND deleted_at IS NULL",
        params![now_ms(), board],
    )?;
    Ok(())
}

/// Restore a moodboard with its items (they never left it).
pub fn restore_moodboard(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "UPDATE moodboard SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
        params![now_ms(), row.object_id],
    )?;
    touch_items(tx, &row.object_id)
}

fn ensure_board(c: &Connection, id: &str) -> AppResult<()> {
    let ok: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM moodboard WHERE id=?1 AND deleted_at IS NULL)",
        [id],
        |r| r.get(0),
    )?;
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found("moodboard"))
    }
}

// ------------------------------------------------------------------ items

fn clamp_pos(v: i64) -> i64 {
    v.clamp(0, CANVAS_MAX)
}

fn clamp_size(v: i64) -> i64 {
    v.clamp(TILE_MIN, TILE_MAX)
}

/// Default placement: a gentle cascade so new tiles never stack exactly.
fn default_xy(c: &Connection, board: &str) -> AppResult<(i64, i64)> {
    let n: i64 = c.query_row(
        "SELECT count(*) FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL",
        [board],
        |r| r.get(0),
    )?;
    Ok((24 + (n % 6) * 34 + (n / 6 % 4) * 220, 24 + (n % 6) * 30))
}

/// Tile size for an image: 240 px wide, height from the image's aspect ratio.
fn image_size(asset: &AssetInfo) -> (i64, i64) {
    match (asset.width, asset.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            let tw = 240.min(w.max(TILE_MIN));
            (tw, clamp_size(tw * h / w))
        }
        _ => (240, 160),
    }
}

struct NewItem<'a> {
    board: &'a str,
    kind: &'a str,
    x: Option<i64>,
    y: Option<i64>,
    w: i64,
    h: i64,
    body: Option<String>,
    url: Option<String>,
    link_title: Option<String>,
    asset_id: Option<String>,
    source_vault_item_id: Option<String>,
    is_private: bool,
}

fn insert_item(tx: &Tx<'_>, it: NewItem<'_>) -> AppResult<String> {
    let c = tx.conn();
    let (dx, dy) = default_xy(c, it.board)?;
    let z: i64 = c.query_row(
        "SELECT COALESCE(MAX(z), 0) + 1 FROM moodboard_item WHERE moodboard_id=?1",
        [it.board],
        |r| r.get(0),
    )?;
    let id = new_id();
    let now = now_ms();
    c.execute(
        "INSERT INTO moodboard_item(id, moodboard_id, kind, x, y, w, h, z, body, url, link_title, asset_id, source_vault_item_id,
                                    is_private, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            id,
            it.board,
            it.kind,
            clamp_pos(it.x.unwrap_or(dx)),
            clamp_pos(it.y.unwrap_or(dy)),
            clamp_size(it.w),
            clamp_size(it.h),
            z,
            it.body,
            it.url,
            it.link_title,
            it.asset_id,
            it.source_vault_item_id,
            it.is_private,
            now
        ],
    )?;
    Ok(id)
}

pub fn add_images(
    core: &AppCore,
    actor: &Actor,
    args: AddMoodboardImagesArgs,
) -> AppResult<Vec<MoodboardItemDto>> {
    if args.paths.is_empty() {
        return Ok(vec![]);
    }
    if args.paths.len() > 200 {
        return Err(AppError::invalid_input("Add at most 200 images at a time."));
    }
    let s = core.project()?;
    let label = if args.paths.len() == 1 {
        "Added image to moodboard".to_string()
    } else {
        format!("Added {} images to moodboard", args.paths.len())
    };
    let ids = s.store.mutate(
        actor,
        MutationMeta::new("moodboard.add_images", label, Capability::Edit)
            .target("moodboard", &args.moodboard_id),
        |tx| {
            ensure_board(tx.conn(), &args.moodboard_id)?;
            let mut ids = Vec::new();
            for (i, p) in args.paths.iter().enumerate() {
                let asset = ingest_image_file(tx, p)?;
                let (w, h) = image_size(&asset);
                let off = i as i64 * 28;
                ids.push(insert_item(
                    tx,
                    NewItem {
                        board: &args.moodboard_id,
                        kind: "image",
                        x: args.x.map(|x| x + off),
                        y: args.y.map(|y| y + off),
                        w,
                        h,
                        body: None,
                        url: None,
                        link_title: None,
                        asset_id: Some(asset.id),
                        source_vault_item_id: None,
                        is_private: false,
                    },
                )?);
            }
            Ok(ids)
        },
    )?;
    read_items(core, &ids)
}

pub fn add_image_data(
    core: &AppCore,
    actor: &Actor,
    args: AddMoodboardImageDataArgs,
) -> AppResult<MoodboardItemDto> {
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.add_image_data",
            "Pasted image into moodboard",
            Capability::Edit,
        )
        .target("moodboard", &args.moodboard_id),
        |tx| {
            ensure_board(tx.conn(), &args.moodboard_id)?;
            let asset = ingest_image_data(
                tx,
                &args.data_base64,
                args.file_name.as_deref().unwrap_or("pasted-image"),
            )?;
            let (w, h) = image_size(&asset);
            insert_item(
                tx,
                NewItem {
                    board: &args.moodboard_id,
                    kind: "image",
                    x: args.x,
                    y: args.y,
                    w,
                    h,
                    body: None,
                    url: None,
                    link_title: None,
                    asset_id: Some(asset.id),
                    source_vault_item_id: None,
                    is_private: false,
                },
            )
        },
    )?;
    Ok(read_items(core, &[id])?.remove(0))
}

/// Place an Idea Vault image on the board. The moodboard item is a new object
/// (no live link back to the vault). A project-vault image shares the project's
/// asset file; a Global Idea Vault image is copied into the project.
pub fn add_vault_image(
    core: &AppCore,
    actor: &Actor,
    args: AddMoodboardVaultImageArgs,
) -> AppResult<MoodboardItemDto> {
    let s = core.project()?;
    let meta = MutationMeta::new(
        "moodboard.add_vault_image",
        "Added Idea Vault image to moodboard",
        Capability::Edit,
    )
    .target("moodboard", &args.moodboard_id);
    let not_found = || {
        AppError::new(
            "not_found.vault_item",
            "That Idea Vault image could not be found. It may have been deleted.",
        )
    };
    let insert = |tx: &Tx<'_>, asset: AssetInfo, title: Option<String>| -> AppResult<String> {
        let (w, h) = image_size(&asset);
        let id = insert_item(
            tx,
            NewItem {
                board: &args.moodboard_id,
                kind: "image",
                x: args.x,
                y: args.y,
                w,
                h,
                body: None,
                url: None,
                link_title: None,
                asset_id: Some(asset.id),
                source_vault_item_id: Some(args.vault_item_id.clone()),
                is_private: false,
            },
        )?;
        if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
            let cap: String = t.trim().chars().take(CAPTION_MAX).collect();
            tx.conn().execute(
                "UPDATE moodboard_item SET caption=?1 WHERE id=?2",
                params![cap, id],
            )?;
        }
        Ok(id)
    };
    let id = match args.store.unwrap_or_default() {
        StoreSel::Project => s.store.mutate(actor, meta, |tx| {
            ensure_board(tx.conn(), &args.moodboard_id)?;
            let (asset_id, title) =
                vault_item_image(tx.conn(), &args.vault_item_id)?.ok_or_else(not_found)?;
            let asset = crate::util::load_asset(tx.conn(), tx.root(), &asset_id)?;
            insert(tx, asset, title)
        })?,
        StoreSel::Global => {
            actor.require(Capability::View, "view the Idea Vault")?;
            let g = core.global_store()?;
            let groot = g.root().to_path_buf();
            g.read(|gc| {
                let (asset_id, title) =
                    vault_item_image(gc, &args.vault_item_id)?.ok_or_else(not_found)?;
                s.store.mutate(actor, meta, |tx| {
                    ensure_board(tx.conn(), &args.moodboard_id)?;
                    let asset = copy_asset_between(gc, &groot, &asset_id, tx)?;
                    insert(tx, asset, title)
                })
            })?
        }
    };
    Ok(read_items(core, &[id])?.remove(0))
}

pub fn add_note(
    core: &AppCore,
    actor: &Actor,
    args: AddMoodboardNoteArgs,
) -> AppResult<MoodboardItemDto> {
    let body = required_text(&args.text, "Note", TEXT_MAX)?;
    let private = args.is_private.unwrap_or(false);
    let s = core.project()?;
    let label = if private {
        "Added internal note to moodboard"
    } else {
        "Added note to moodboard"
    };
    let id = s.store.mutate(
        actor,
        MutationMeta::new("moodboard.add_note", label, Capability::Edit)
            .target("moodboard", &args.moodboard_id),
        |tx| {
            ensure_board(tx.conn(), &args.moodboard_id)?;
            insert_item(
                tx,
                NewItem {
                    board: &args.moodboard_id,
                    kind: "note",
                    x: args.x,
                    y: args.y,
                    w: 180,
                    h: 110,
                    body: Some(body.clone()),
                    url: None,
                    link_title: None,
                    asset_id: None,
                    source_vault_item_id: None,
                    is_private: private,
                },
            )
        },
    )?;
    Ok(read_items(core, &[id])?.remove(0))
}

pub fn add_link(
    core: &AppCore,
    actor: &Actor,
    args: AddMoodboardLinkArgs,
) -> AppResult<MoodboardItemDto> {
    let url = normalize_url(&args.url)?;
    let title = optional_text(args.title.clone(), "Link title", 200)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.add_link",
            "Added link to moodboard",
            Capability::Edit,
        )
        .target("moodboard", &args.moodboard_id),
        |tx| {
            ensure_board(tx.conn(), &args.moodboard_id)?;
            insert_item(
                tx,
                NewItem {
                    board: &args.moodboard_id,
                    kind: "link",
                    x: args.x,
                    y: args.y,
                    w: 200,
                    h: 64,
                    body: None,
                    url: Some(url.clone()),
                    link_title: title.clone(),
                    asset_id: None,
                    source_vault_item_id: None,
                    is_private: false,
                },
            )
        },
    )?;
    Ok(read_items(core, &[id])?.remove(0))
}

fn item_board(c: &Connection, id: &str) -> AppResult<(String, String)> {
    c.query_row(
        "SELECT i.moodboard_id, i.kind FROM moodboard_item i JOIN moodboard m ON m.id = i.moodboard_id
         WHERE i.id=?1 AND i.deleted_at IS NULL AND m.deleted_at IS NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("moodboard item"))
}

pub fn update_item(
    core: &AppCore,
    actor: &Actor,
    args: UpdateMoodboardItemArgs,
) -> AppResult<MoodboardItemDto> {
    let mut fields: Vec<(&'static str, SqlValue)> = Vec::new();
    if let Some(c) = args.caption.clone() {
        fields.push((
            "caption",
            opt_text(optional_text(Some(c), "Caption", CAPTION_MAX)?),
        ));
    }
    if let Some(t) = args.link_title.clone() {
        fields.push((
            "link_title",
            opt_text(optional_text(Some(t), "Link title", 200)?),
        ));
    }
    if let Some(p) = args.is_private {
        fields.push(("is_private", int(p as i64)));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.update_item",
            "Edited moodboard item",
            Capability::Edit,
        )
        .target("moodboard_item", &args.id)
        .coalesce(format!("moodboard.update_item:{}", args.id)),
        |tx| {
            let (_, kind) = item_board(tx.conn(), &args.id)?;
            let mut fields = fields.clone();
            if let Some(b) = &args.body {
                if kind != "note" {
                    return Err(AppError::invalid_input("Only notes have text."));
                }
                fields.push(("body", text(required_text(b, "Note", TEXT_MAX)?)));
            }
            if let Some(u) = &args.url {
                if kind != "link" {
                    return Err(AppError::invalid_input("Only links have a web address."));
                }
                fields.push(("url", text(normalize_url(u)?)));
            }
            update_fields(
                tx.conn(),
                "moodboard_item",
                &args.id,
                &fields,
                ITEM_FIELDS,
                args.expected_rev,
                "moodboard item",
            )?;
            Ok(())
        },
    )?;
    Ok(read_items(core, std::slice::from_ref(&args.id))?.remove(0))
}

/// Move one or several tiles (multi-select group move) as one undoable step.
pub fn move_items(core: &AppCore, actor: &Actor, args: MoveMoodboardItemsArgs) -> AppResult<()> {
    if args.moves.is_empty() {
        return Ok(());
    }
    if args.moves.len() > 500 {
        return Err(AppError::invalid_input("Move at most 500 items at a time."));
    }
    let s = core.project()?;
    let label = if args.moves.len() == 1 {
        "Moved moodboard item".to_string()
    } else {
        format!("Moved {} moodboard items", args.moves.len())
    };
    s.store.mutate(
        actor,
        MutationMeta::new("moodboard.move_items", label, Capability::Edit),
        |tx| {
            for m in &args.moves {
                item_board(tx.conn(), &m.id)?;
                update_fields(
                    tx.conn(),
                    "moodboard_item",
                    &m.id,
                    &[("x", int(clamp_pos(m.x))), ("y", int(clamp_pos(m.y)))],
                    &["x", "y"],
                    None,
                    "moodboard item",
                )?;
            }
            Ok(())
        },
    )
}

pub fn resize_item(core: &AppCore, actor: &Actor, args: ResizeMoodboardItemArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "moodboard.resize_item",
            "Resized moodboard item",
            Capability::Edit,
        )
        .target("moodboard_item", &args.id),
        |tx| {
            item_board(tx.conn(), &args.id)?;
            update_fields(
                tx.conn(),
                "moodboard_item",
                &args.id,
                &[
                    ("w", int(clamp_size(args.w))),
                    ("h", int(clamp_size(args.h))),
                ],
                &["w", "h"],
                None,
                "moodboard item",
            )?;
            Ok(())
        },
    )
}

/// Reorder a tile in the stack: bring to front / send to back.
pub fn arrange_item(
    core: &AppCore,
    actor: &Actor,
    args: ArrangeMoodboardItemArgs,
) -> AppResult<()> {
    let front = match args.placement.as_str() {
        "front" => true,
        "back" => false,
        _ => return Err(AppError::invalid_input("Choose front or back.")),
    };
    let s = core.project()?;
    let label = if front {
        "Brought moodboard item to front"
    } else {
        "Sent moodboard item to back"
    };
    s.store.mutate(actor, MutationMeta::new("moodboard.arrange_item", label, Capability::Edit).target("moodboard_item", &args.id), |tx| {
        let (board, _) = item_board(tx.conn(), &args.id)?;
        let z: i64 = tx.conn().query_row(
            if front {
                "SELECT COALESCE(MAX(z), 0) + 1 FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL"
            } else {
                "SELECT COALESCE(MIN(z), 0) - 1 FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL"
            },
            [&board],
            |r| r.get(0),
        )?;
        update_fields(tx.conn(), "moodboard_item", &args.id, &[("z", int(z))], &["z"], None, "moodboard item")?;
        Ok(())
    })
}

fn item_title(
    kind: &str,
    caption: Option<String>,
    body: Option<String>,
    link: Option<String>,
) -> String {
    let t = caption
        .or(match kind {
            "note" => body,
            "link" => link,
            _ => None,
        })
        .map(|t| t.trim().chars().take(60).collect::<String>())
        .filter(|t| !t.is_empty());
    match (kind, t) {
        ("image", Some(t)) => format!("Image — {t}"),
        ("note", Some(t)) => format!("Note — {t}"),
        ("link", Some(t)) => format!("Link — {t}"),
        ("note", None) => "Note".into(),
        ("link", None) => "Link".into(),
        _ => "Image".into(),
    }
}

pub fn delete_items(
    core: &AppCore,
    actor: &Actor,
    args: DeleteMoodboardItemsArgs,
) -> AppResult<()> {
    if args.ids.is_empty() {
        return Ok(());
    }
    if args.ids.len() > 500 {
        return Err(AppError::invalid_input(
            "Delete at most 500 items at a time.",
        ));
    }
    let s = core.project()?;
    let label = if args.ids.len() == 1 {
        "Deleted moodboard item".to_string()
    } else {
        format!("Deleted {} moodboard items", args.ids.len())
    };
    s.store.mutate(actor, MutationMeta::new("moodboard.delete_items", label, Capability::SoftDelete), |tx| {
        for id in &args.ids {
            let (board, kind) = item_board(tx.conn(), id)?;
            let (caption, body, link, z): (Option<String>, Option<String>, Option<String>, i64) = tx.conn().query_row(
                "SELECT caption, body, COALESCE(link_title, url), z FROM moodboard_item WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "moodboard_item",
                    table: "moodboard_item",
                    id,
                    title: Some(item_title(&kind, caption, body, link)),
                    parent_type: Some("moodboard"),
                    parent_id: Some(board),
                    position: Some(z),
                },
            )?;
        }
        Ok(())
    })
}

// ------------------------------------------------------------------ search / trash

pub fn index_moodboard(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT name, notes, deleted_at FROM moodboard WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((name, notes, None)) => Some(SearchDoc {
            entity_type: "moodboard".into(),
            title: name,
            body: notes.unwrap_or_default(),
            context: "Production · Moodboards".into(),
            nav: json!({ "workspace": "production", "sub": "moodboards", "moodboardId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

pub fn index_item(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    #[allow(clippy::type_complexity)]
    let row: Option<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>, Option<i64>)> = c
        .query_row(
            "SELECT i.moodboard_id, m.name, i.kind, i.caption, i.body, i.url, i.link_title, i.deleted_at, m.deleted_at
             FROM moodboard_item i JOIN moodboard m ON m.id = i.moodboard_id WHERE i.id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .optional()?;
    let Some((board, board_name, kind, caption, body, url, link_title, None, None)) = row else {
        return Ok(None);
    };
    let text_parts: Vec<String> = [
        caption.clone(),
        body.clone(),
        link_title.clone(),
        url.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();
    if text_parts.is_empty() {
        return Ok(None);
    }
    Ok(Some(SearchDoc {
        entity_type: "moodboard_item".into(),
        title: item_title(&kind, caption, body, link_title.or(url)),
        body: format!("{} {}", text_parts.join(" "), board_name),
        context: "Production · Moodboards".into(),
        nav: json!({ "workspace": "production", "sub": "moodboards", "moodboardId": board, "itemId": id }),
        owner_user_id: None,
    }))
}

pub const UNASSIGNED_BOARD: &str = "Unassigned";

/// Restore an item to its board (same place and stacking). If the board is gone,
/// it goes to an "Unassigned" moodboard (FSD §52.5).
pub fn restore_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let board_ok = match &row.parent_id {
        Some(b) => c.query_row(
            "SELECT EXISTS(SELECT 1 FROM moodboard WHERE id=?1 AND deleted_at IS NULL)",
            [b],
            |r| r.get::<_, bool>(0),
        )?,
        None => false,
    };
    let board = if board_ok {
        row.parent_id.clone().unwrap_or_default()
    } else {
        let existing: Option<String> = c
            .query_row(
                "SELECT id FROM moodboard WHERE name=?1 AND deleted_at IS NULL ORDER BY position LIMIT 1",
                [UNASSIGNED_BOARD],
                |r| r.get(0),
            )
            .optional()?;
        match existing {
            Some(id) => id,
            None => {
                let id = new_id();
                let now = now_ms();
                let pos = next_position(c, "moodboard", "deleted_at IS NULL", &[])?;
                c.execute(
                    "INSERT INTO moodboard(id, name, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![id, UNASSIGNED_BOARD, pos, now],
                )?;
                id
            }
        }
    };
    c.execute(
        "UPDATE moodboard_item SET deleted_at=NULL, moodboard_id=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
        params![board, now_ms(), row.object_id],
    )?;
    tx.reindex("moodboard_item", &row.object_id);
    Ok(())
}

fn purge_item_row(tx: &Tx<'_>, id: &str) -> AppResult<()> {
    let c = tx.conn();
    let asset: Option<String> = c
        .query_row(
            "SELECT asset_id FROM moodboard_item WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    c.execute(
        "DELETE FROM deleted_item WHERE table_name='moodboard_item' AND object_id=?1",
        [id],
    )?;
    c.execute("DELETE FROM moodboard_item WHERE id=?1", [id])?;
    if let Some(a) = asset {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

pub fn purge_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    purge_item_row(tx, &row.object_id)
}

pub fn purge_moodboard(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let ids: Vec<String> = {
        let mut stmt = tx
            .conn()
            .prepare("SELECT id FROM moodboard_item WHERE moodboard_id=?1")?;
        stmt.query_map([&row.object_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for id in ids {
        purge_item_row(tx, &id)?;
    }
    tx.conn()
        .execute("DELETE FROM moodboard WHERE id=?1", [&row.object_id])?;
    Ok(())
}
