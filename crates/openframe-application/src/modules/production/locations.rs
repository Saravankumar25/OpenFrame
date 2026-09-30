//! Locations (FSD §29, §99): a practical scouting notebook with photos,
//! practical notes, status and derived scene usage.
//!
//! - Only the name is required. Status: Idea → Shortlisted → Confirmed → Rejected
//!   (simple and reversible). A status change never alters the screenplay.
//! - Scene usage is derived from Location / Set breakdown elements whose
//!   catalog item is linked to the location.
//! - Replacement for selected scenes updates production associations only,
//!   never screenplay text (FSD §29.4).

use std::collections::HashMap;
use std::path::PathBuf;

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{
    int, next_position, opt_text, place_at, sibling_ids, text, update_fields,
};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::suggest::name_key;
use super::{
    PRODUCTION_STATES, ProductionSceneRef, SourceScenes, active_source, patch_text, scene_ref,
    sort_refs,
};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, ingest_file, load_asset, optional_text, required_text};

openframe_domain::string_enum!(
    /// Location status (FSD §29.3).
    ProductionLocationStatus {
        Idea = "Idea",
        Shortlisted = "Shortlisted",
        Confirmed = "Confirmed",
        Rejected = "Rejected",
    }
);

pub fn register(r: &mut Registry) {
    r.query("locations.list", list);
    r.query("locations.get", get);
    r.command("locations.create", create);
    r.command("locations.update", update);
    r.command("locations.set_status", set_status);
    r.command("locations.add_photos", add_photos);
    r.command("locations.remove_photo", remove_photo);
    r.command("locations.move_photo", move_photo);
    r.command("locations.set_archived", set_archived);
    r.command("locations.delete", delete);
    r.command("locations.replace", replace);
    r.indexer("location", index_location);
    r.trash_handler(TrashHandler {
        object_type: "location",
        table: "location",
        label: "Location",
        restore: None,
        purge: purge_location,
    });
    r.trash_handler(TrashHandler {
        object_type: "location_photo",
        table: "location_photo",
        label: "Location photo",
        restore: Some(restore_photo),
        purge: purge_photo,
    });
}

/// Practical notes (FSD §29.1; UX §3.24 labelled lines).
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct ProductionLocationNotes {
    pub access: Option<String>,
    pub parking: Option<String>,
    pub noise: Option<String>,
    pub power: Option<String>,
    pub permission: Option<String>,
    pub toilets: Option<String>,
    pub facilities: Option<String>,
    pub travel: Option<String>,
    pub general: Option<String>,
}

impl ProductionLocationNotes {
    fn cleaned(self) -> AppResult<ProductionLocationNotes> {
        let f = |v: Option<String>, what: &str| optional_text(v, what, 4_000);
        Ok(ProductionLocationNotes {
            access: f(self.access, "Access notes")?,
            parking: f(self.parking, "Parking notes")?,
            noise: f(self.noise, "Noise notes")?,
            power: f(self.power, "Power notes")?,
            permission: f(self.permission, "Permission notes")?,
            toilets: f(self.toilets, "Toilet notes")?,
            facilities: f(self.facilities, "Facilities notes")?,
            travel: f(self.travel, "Travel notes")?,
            general: f(self.general, "Notes")?,
        })
    }
    fn all_text(&self) -> String {
        [
            &self.access,
            &self.parking,
            &self.noise,
            &self.power,
            &self.permission,
            &self.toilets,
            &self.facilities,
            &self.travel,
            &self.general,
        ]
        .iter()
        .filter_map(|v| v.as_deref())
        .collect::<Vec<_>>()
        .join(" ")
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionLocationPhotoDto {
    pub id: String,
    pub caption: Option<String>,
    pub asset: AssetInfo,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionLocationRefDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionLocationDto {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub contact: Option<String>,
    pub status: ProductionLocationStatus,
    pub notes: ProductionLocationNotes,
    /// Ordered; the first is the thumbnail.
    pub photos: Vec<ProductionLocationPhotoDto>,
    /// Scenes requiring this location (derived from the breakdown).
    pub scenes: Vec<ProductionSceneRef>,
    /// Number of Production Source scenes using it.
    #[ts(type = "number")]
    pub scene_count: i64,
    pub replacement: Option<ProductionLocationRefDto>,
    pub archived: bool,
    #[ts(type = "number")]
    pub rev: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationListArgs {
    #[serde(default)]
    pub status: Option<ProductionLocationStatus>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationCreateArgs {
    pub name: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub status: Option<ProductionLocationStatus>,
}

/// Field patch: omitted = unchanged; empty text = cleared; `notes` replaces all practical notes.
#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub notes: Option<ProductionLocationNotes>,
    #[serde(default)]
    #[ts(type = "number", optional = nullable)]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationStatusArgs {
    pub id: String,
    pub status: ProductionLocationStatus,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationPhotosArgs {
    pub id: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationPhotoMoveArgs {
    pub photo_id: String,
    pub index: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationArchiveArgs {
    pub id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionLocationReplaceArgs {
    /// The location being replaced (usually Rejected).
    pub location_id: String,
    /// The practical replacement.
    pub replacement_id: String,
    pub scene_ids: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionLocationReplaceResult {
    #[ts(type = "number")]
    pub scenes_updated: i64,
}

// ------------------------------------------------------------------ reads

struct LocRow {
    id: String,
    name: String,
    address: Option<String>,
    contact: Option<String>,
    status: String,
    notes_json: String,
    replacement_id: Option<String>,
    archived: bool,
    rev: i64,
    updated_at: i64,
}

const LOC_COLS: &str = "id, name, address, contact, status, notes_json, replacement_location_id, archived, rev, updated_at";

fn loc_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<LocRow> {
    Ok(LocRow {
        id: r.get(0)?,
        name: r.get(1)?,
        address: r.get(2)?,
        contact: r.get(3)?,
        status: r.get(4)?,
        notes_json: r.get(5)?,
        replacement_id: r.get(6)?,
        archived: r.get(7)?,
        rev: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

/// Scene refs per location, derived from Location / Set breakdown associations.
fn location_usage(
    c: &Connection,
    scenes: &SourceScenes,
) -> AppResult<HashMap<String, Vec<ProductionSceneRef>>> {
    let mut stmt = c.prepare(&format!(
        "SELECT DISTINCT ci.location_id, e.scene_id FROM breakdown_element e
         JOIN catalog_item ci ON ci.id = e.catalog_item_id
         WHERE ci.location_id IS NOT NULL AND ci.deleted_at IS NULL
           AND e.deleted_at IS NULL AND e.archived = 0 AND e.confirmation_state IN {PRODUCTION_STATES}"
    ))?;
    let pairs: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut out: HashMap<String, Vec<ProductionSceneRef>> = HashMap::new();
    for (loc, scene) in pairs {
        out.entry(loc)
            .or_default()
            .push(scene_ref(c, scenes, &scene)?);
    }
    for v in out.values_mut() {
        sort_refs(v);
        v.dedup();
    }
    Ok(out)
}

fn photos(
    c: &Connection,
    root: &std::path::Path,
    location_id: &str,
) -> AppResult<Vec<ProductionLocationPhotoDto>> {
    let mut stmt = c.prepare(
        "SELECT id, caption, asset_id FROM location_photo WHERE location_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows: Vec<(String, Option<String>, String)> = stmt
        .query_map([location_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    rows.into_iter()
        .map(|(id, caption, asset)| {
            Ok(ProductionLocationPhotoDto {
                id,
                caption,
                asset: load_asset(c, root, &asset)?,
            })
        })
        .collect()
}

fn to_dto(
    c: &Connection,
    root: &std::path::Path,
    row: LocRow,
    usage: &HashMap<String, Vec<ProductionSceneRef>>,
) -> AppResult<ProductionLocationDto> {
    let scenes = usage.get(&row.id).cloned().unwrap_or_default();
    let replacement = match &row.replacement_id {
        Some(rid) => c
            .query_row(
                "SELECT id, name FROM location WHERE id = ?1 AND deleted_at IS NULL",
                [rid],
                |r| {
                    Ok(ProductionLocationRefDto {
                        id: r.get(0)?,
                        name: r.get(1)?,
                    })
                },
            )
            .optional()?,
        None => None,
    };
    Ok(ProductionLocationDto {
        photos: photos(c, root, &row.id)?,
        scene_count: scenes.iter().filter(|s| s.in_source).count() as i64,
        scenes,
        replacement,
        status: ProductionLocationStatus::parse(&row.status)
            .unwrap_or(ProductionLocationStatus::Idea),
        notes: serde_json::from_str(&row.notes_json).unwrap_or_default(),
        id: row.id,
        name: row.name,
        address: row.address,
        contact: row.contact,
        archived: row.archived,
        rev: row.rev,
        updated_at: row.updated_at,
    })
}

fn usage_for(c: &Connection) -> AppResult<HashMap<String, Vec<ProductionSceneRef>>> {
    let source = active_source(c)?;
    let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
    location_usage(c, &scenes)
}

pub(crate) fn load_location(
    c: &Connection,
    root: &std::path::Path,
    id: &str,
) -> AppResult<ProductionLocationDto> {
    let row = c
        .query_row(
            &format!("SELECT {LOC_COLS} FROM location WHERE id = ?1 AND deleted_at IS NULL"),
            [id],
            loc_row,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("location"))?;
    to_dto(c, root, row, &usage_for(c)?)
}

fn list(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationListArgs,
) -> AppResult<Vec<ProductionLocationDto>> {
    actor.require(Capability::View, "view locations")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let needle = args
        .search
        .as_deref()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty());
    s.store.read(|c| {
        let mut stmt = c.prepare(&format!("SELECT {LOC_COLS} FROM location WHERE deleted_at IS NULL ORDER BY name COLLATE NOCASE, id"))?;
        let rows: Vec<LocRow> = stmt.query_map([], loc_row)?.collect::<Result<_, _>>()?;
        let usage = usage_for(c)?;
        let mut out = Vec::new();
        for row in rows {
            if row.archived && !args.include_archived {
                continue;
            }
            if args.status.is_some_and(|st| row.status != st.as_str()) {
                continue;
            }
            if let Some(n) = &needle {
                let hay = format!(
                    "{} {} {} {}",
                    row.name,
                    row.address.as_deref().unwrap_or(""),
                    row.contact.as_deref().unwrap_or(""),
                    row.notes_json
                )
                .to_lowercase();
                if !hay.contains(n) {
                    continue;
                }
            }
            out.push(to_dto(c, &root, row, &usage)?);
        }
        Ok(out)
    })
}

fn get(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationIdArgs,
) -> AppResult<ProductionLocationDto> {
    actor.require(Capability::View, "view locations")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &args.id))
}

// ------------------------------------------------------ shared (catalog)

fn insert_location(
    tx: &Tx<'_>,
    name: &str,
    address: Option<String>,
    contact: Option<String>,
    status: ProductionLocationStatus,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO location(id, name, address, contact, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, name, address, contact, status.as_str(), now],
    )?;
    Ok(id)
}

/// The location record for a Location / Set catalog item created during
/// breakdown: an existing location with the same name, or a new "Idea".
pub(crate) fn location_for_name(tx: &Tx<'_>, name: &str) -> AppResult<String> {
    let key = name_key(name);
    let mut stmt = tx.conn().prepare(
        "SELECT id, name FROM location WHERE deleted_at IS NULL ORDER BY archived, created_at",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    if let Some((id, _)) = rows.into_iter().find(|(_, n)| name_key(n) == key) {
        return Ok(id);
    }
    insert_location(tx, name.trim(), None, None, ProductionLocationStatus::Idea)
}

/// The Location / Set catalog item representing a location (created when missing).
pub(crate) fn catalog_item_for_location(tx: &Tx<'_>, location_id: &str) -> AppResult<String> {
    let existing: Option<String> = tx
        .conn()
        .query_row(
            "SELECT id FROM catalog_item WHERE location_id = ?1 AND deleted_at IS NULL AND archived = 0
             ORDER BY created_at LIMIT 1",
            [location_id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let name: String = tx
        .conn()
        .query_row(
            "SELECT name FROM location WHERE id = ?1 AND deleted_at IS NULL",
            [location_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("location"))?;
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO catalog_item(id, category, name, status, location_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'Required', ?4, ?5, ?5)",
        params![
            id,
            BreakdownCategory::Location.as_str(),
            name,
            location_id,
            now
        ],
    )?;
    Ok(id)
}

// ------------------------------------------------------------- mutations

fn name_of(c: &Connection, id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT name FROM location WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("location"))
}

fn create(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationCreateArgs,
) -> AppResult<ProductionLocationDto> {
    let name = required_text(&args.name, "Location name", 200)?;
    let address = optional_text(args.address, "Address / area", 1_000)?;
    let contact = optional_text(args.contact, "Contact", 500)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.create",
            format!("Added location “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            insert_location(
                tx,
                &name,
                address,
                contact,
                args.status.unwrap_or(ProductionLocationStatus::Idea),
            )
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &id))
}

fn update(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationUpdateArgs,
) -> AppResult<ProductionLocationDto> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut new_name = None;
    if let Some(n) = &args.name {
        let n = required_text(n, "Location name", 200)?;
        new_name = Some(n.clone());
        fields.push(("name", text(n)));
    }
    if let Some(v) = patch_text(args.address, "Address / area", 1_000)? {
        fields.push(("address", v));
    }
    if let Some(v) = patch_text(args.contact, "Contact", 500)? {
        fields.push(("contact", v));
    }
    if let Some(notes) = args.notes {
        let notes = notes.cleaned()?;
        fields.push((
            "notes_json",
            text(serde_json::to_string(&notes).map_err(|e| AppError::internal(e.to_string()))?),
        ));
    }
    let s = core.project()?;
    let label = s.store.read(|c| name_of(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new("locations.update", format!("Edited location “{label}”"), Capability::Edit)
            .target("location", &args.id)
            .coalesce(format!("locations.update:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "location",
                &args.id,
                &fields,
                &["name", "address", "contact", "notes_json"],
                args.expected_rev,
                "location",
            )?;
            if let Some(n) = &new_name {
                // One identity: linked Location / Set catalog items carry the same name.
                let mut stmt = tx.conn().prepare("SELECT id FROM catalog_item WHERE location_id = ?1 AND deleted_at IS NULL AND name <> ?2")?;
                let ids: Vec<String> = stmt.query_map(params![args.id, n], |r| r.get(0))?.collect::<Result<_, _>>()?;
                for id in ids {
                    update_fields(tx.conn(), "catalog_item", &id, &[("name", text(n.clone()))], &["name"], None, "catalog item")?;
                }
            }
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &args.id))
}

fn set_status(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationStatusArgs,
) -> AppResult<ProductionLocationDto> {
    let s = core.project()?;
    let label = s.store.read(|c| name_of(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.set_status",
            format!("Set “{label}” to {}", args.status.as_str()),
            Capability::Edit,
        )
        .target("location", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "location",
                &args.id,
                &[("status", text(args.status.as_str()))],
                &["status"],
                None,
                "location",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &args.id))
}

fn add_photos(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationPhotosArgs,
) -> AppResult<ProductionLocationDto> {
    if args.paths.is_empty() {
        return Err(AppError::required("A photo"));
    }
    if args.paths.len() > 200 {
        return Err(AppError::invalid_input("Add at most 200 photos at a time."));
    }
    let s = core.project()?;
    let label = s.store.read(|c| name_of(c, &args.id))?;
    let summary = if args.paths.len() == 1 {
        format!("Added a photo to “{label}”")
    } else {
        format!("Added {} photos to “{label}”", args.paths.len())
    };
    s.store.mutate(actor, MutationMeta::new("locations.add_photos", summary, Capability::Edit).target("location", &args.id), |tx| {
        for p in &args.paths {
            let asset = ingest_file(tx, &PathBuf::from(p))?;
            if !asset.media_type.starts_with("image/") {
                return Err(AppError::invalid_input(format!("“{}” is not an image. Choose JPG, PNG, GIF or WebP photos.", asset.original_name)));
            }
            let pos = next_position(tx.conn(), "location_photo", "location_id = ?1 AND deleted_at IS NULL", &[text(args.id.clone())])?;
            let now = now_ms();
            tx.conn().execute(
                "INSERT INTO location_photo(id, location_id, asset_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![new_id(), args.id, asset.id, pos, now],
            )?;
        }
        Ok(())
    })?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &args.id))
}

fn photo_parent(c: &Connection, photo_id: &str) -> AppResult<(String, i64)> {
    c.query_row(
        "SELECT location_id, position FROM location_photo WHERE id = ?1 AND deleted_at IS NULL",
        [photo_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("photo"))
}

fn remove_photo(core: &AppCore, actor: &Actor, args: ProductionLocationIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (loc, pos) = s.store.read(|c| photo_parent(c, &args.id))?;
    let label = s.store.read(|c| name_of(c, &loc))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.remove_photo",
            format!("Removed a photo from “{label}”"),
            Capability::SoftDelete,
        )
        .target("location", &loc),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "location_photo",
                    table: "location_photo",
                    id: &args.id,
                    title: Some(format!("Photo of {label}")),
                    parent_type: Some("location"),
                    parent_id: Some(loc.clone()),
                    position: Some(pos),
                },
            )
        },
    )
}

fn move_photo(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationPhotoMoveArgs,
) -> AppResult<()> {
    let s = core.project()?;
    let (loc, _) = s.store.read(|c| photo_parent(c, &args.photo_id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.move_photo",
            "Reordered location photos",
            Capability::Edit,
        )
        .target("location", &loc),
        |tx| {
            let sibs = sibling_ids(
                tx.conn(),
                "location_photo",
                "location_id = ?1 AND deleted_at IS NULL",
                &[text(loc.clone())],
            )?;
            place_at(
                tx.conn(),
                "location_photo",
                sibs,
                &args.photo_id,
                Some(args.index as usize),
            )
        },
    )
}

fn set_archived(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationArchiveArgs,
) -> AppResult<ProductionLocationDto> {
    let s = core.project()?;
    let label = s.store.read(|c| name_of(c, &args.id))?;
    let summary = if args.archived {
        format!("Archived location “{label}”")
    } else {
        format!("Restored location “{label}” from the archive")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("locations.set_archived", summary, Capability::Edit)
            .target("location", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "location",
                &args.id,
                &[("archived", int(args.archived as i64))],
                &["archived"],
                None,
                "location",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_location(c, &root, &args.id))
}

/// Recoverable delete; scene associations and schedule history stay readable.
fn delete(core: &AppCore, actor: &Actor, args: ProductionLocationIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let label = s.store.read(|c| name_of(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.delete",
            format!("Deleted location “{label}”"),
            Capability::SoftDelete,
        )
        .target("location", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "location",
                    table: "location",
                    id: &args.id,
                    title: Some(label.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: None,
                },
            )
        },
    )
}

/// Practical replacement for selected scenes (FSD §29.4): moves the scenes'
/// Location / Set associations to the replacement. Screenplay text is untouched.
fn replace(
    core: &AppCore,
    actor: &Actor,
    args: ProductionLocationReplaceArgs,
) -> AppResult<ProductionLocationReplaceResult> {
    if args.location_id == args.replacement_id {
        return Err(AppError::invalid_input(
            "Choose a different location as the replacement.",
        ));
    }
    if args.scene_ids.is_empty() {
        return Err(AppError::required("At least one scene"));
    }
    let s = core.project()?;
    let (from, to, to_archived): (String, String, bool) = s.store.read(|c| {
        let from = name_of(c, &args.location_id)?;
        let (to, archived): (String, bool) = c
            .query_row(
                "SELECT name, archived FROM location WHERE id = ?1 AND deleted_at IS NULL",
                [&args.replacement_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("location"))?;
        Ok((from, to, archived))
    })?;
    if to_archived {
        return Err(AppError::validation(
            "location",
            format!("“{to}” is archived. Restore it before using it as a replacement."),
        ));
    }
    let n = s.store.mutate(
        actor,
        MutationMeta::new(
            "locations.replace",
            format!(
                "Replaced “{from}” with “{to}” in {} scenes",
                args.scene_ids.len()
            ),
            Capability::Edit,
        )
        .target("location", &args.location_id),
        |tx| {
            let mut stmt = tx.conn().prepare(
                "SELECT id FROM catalog_item WHERE location_id = ?1 AND deleted_at IS NULL",
            )?;
            let from_items: Vec<String> = stmt
                .query_map([&args.location_id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            let target = catalog_item_for_location(tx, &args.replacement_id)?;
            let n =
                super::catalog::replace_item_in_scenes(tx, &from_items, &target, &args.scene_ids)?;
            update_fields(
                tx.conn(),
                "location",
                &args.location_id,
                &[(
                    "replacement_location_id",
                    opt_text(Some(args.replacement_id.clone())),
                )],
                &["replacement_location_id"],
                None,
                "location",
            )?;
            Ok(n)
        },
    )?;
    Ok(ProductionLocationReplaceResult { scenes_updated: n })
}

// ------------------------------------------------------- search / trash

#[allow(clippy::type_complexity)]
fn index_location(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, String, String, Option<i64>)> = c
        .query_row(
            "SELECT name, address, contact, status, notes_json, deleted_at FROM location WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let Some((name, address, contact, status, notes_json, None)) = row else {
        return Ok(None);
    };
    let notes: ProductionLocationNotes = serde_json::from_str(&notes_json).unwrap_or_default();
    Ok(Some(SearchDoc {
        entity_type: "location".into(),
        title: name,
        body: format!(
            "{} {} {} {}",
            address.unwrap_or_default(),
            contact.unwrap_or_default(),
            status,
            notes.all_text()
        ),
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "locations", "locationId": id }),
        owner_user_id: None,
    }))
}

fn purge_location(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let id = &row.object_id;
    let now = now_ms();
    tx.conn().execute(
        "UPDATE catalog_item SET location_id = NULL, updated_at = ?1, rev = rev + 1 WHERE location_id = ?2",
        params![now, id],
    )?;
    tx.conn().execute(
        "UPDATE location SET replacement_location_id = NULL, updated_at = ?1, rev = rev + 1 WHERE replacement_location_id = ?2",
        params![now, id],
    )?;
    let mut stmt = tx
        .conn()
        .prepare("SELECT id, asset_id FROM location_photo WHERE location_id = ?1")?;
    let photos: Vec<(String, String)> = stmt
        .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (pid, asset) in photos {
        tx.conn().execute(
            "DELETE FROM deleted_item WHERE table_name = 'location_photo' AND object_id = ?1",
            [&pid],
        )?;
        tx.conn()
            .execute("DELETE FROM location_photo WHERE id = ?1", [&pid])?;
        crate::modules::files::purge_asset_if_unreferenced(tx, &asset)?;
    }
    tx.conn()
        .execute("DELETE FROM location WHERE id = ?1", [id])?;
    Ok(())
}

/// Return a photo to its location (at its old position when possible). If the
/// location itself is deleted, the photo returns with it when it is restored.
fn restore_photo(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let loc: Option<String> = tx
        .conn()
        .query_row(
            "SELECT location_id FROM location_photo WHERE id = ?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(loc) = loc else {
        return Err(AppError::not_found("photo"));
    };
    tx.conn().execute(
        "UPDATE location_photo SET deleted_at = NULL, updated_at = ?1, rev = rev + 1 WHERE id = ?2",
        params![now_ms(), row.object_id],
    )?;
    let sibs = sibling_ids(
        tx.conn(),
        "location_photo",
        "location_id = ?1 AND deleted_at IS NULL",
        &[text(loc)],
    )?;
    let idx = row.position.map(|p| (p.max(1) - 1) as usize);
    place_at(tx.conn(), "location_photo", sibs, &row.object_id, idx)
}

fn purge_photo(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let asset: Option<String> = tx
        .conn()
        .query_row(
            "SELECT asset_id FROM location_photo WHERE id = ?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?;
    tx.conn()
        .execute("DELETE FROM location_photo WHERE id = ?1", [&row.object_id])?;
    if let Some(a) = asset {
        crate::modules::files::purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}
