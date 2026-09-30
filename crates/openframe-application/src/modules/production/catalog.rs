//! Production Catalog (FSD §28, §98, §143–144, §163): reusable production
//! identities referenced by breakdown elements across scenes.
//!
//! - One identity per item; scene usage is derived from breakdown references.
//! - Removing a scene association never deletes the item.
//! - Archive (default) removes the item from new selections while historical
//!   associations stay readable. Delete is recoverable; affected breakdown
//!   entries keep a visible "removed" status.

use std::collections::HashMap;
use std::path::PathBuf;

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, opt_text, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::suggest::name_key;
use super::{
    ProductionSceneRef, SourceScenes, active_source, catalog_usage, parse_category, patch_text,
    used_in_label,
};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, ingest_file, load_asset_opt, optional_text, required_text};

openframe_domain::string_enum!(
    /// Catalog item status (FSD §28.6).
    CatalogStatus {
        Required = "Required",
        Searching = "Searching",
        Shortlisted = "Shortlisted",
        Confirmed = "Confirmed",
        NotRequired = "Not Required",
    }
);

pub fn register(r: &mut Registry) {
    r.query("catalog.list", list);
    r.query("catalog.get", get);
    r.query("catalog.find_matches", find_matches_op);
    r.command("catalog.create", create);
    r.command("catalog.update", update);
    r.command("catalog.set_image", set_image);
    r.command("catalog.add_alias", add_alias);
    r.command("catalog.remove_alias", remove_alias);
    r.command("catalog.set_archived", set_archived);
    r.command("catalog.delete", delete);
    r.command("catalog.replace_in_scenes", replace_in_scenes);
    r.indexer("catalog_item", index_item);
    r.trash_handler(TrashHandler {
        object_type: "catalog_item",
        table: "catalog_item",
        label: "Catalog item",
        restore: None,
        purge: purge_item,
    });
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CatalogAliasDto {
    pub id: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDto {
    pub id: String,
    pub category: BreakdownCategory,
    pub name: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub contact: Option<String>,
    pub status: CatalogStatus,
    pub image: Option<AssetInfo>,
    pub archived: bool,
    pub character_id: Option<String>,
    pub location_id: Option<String>,
    pub aliases: Vec<CatalogAliasDto>,
    /// Derived from breakdown references ("Used in Scenes", FSD §28.4).
    pub used_in: Vec<ProductionSceneRef>,
    pub used_in_label: String,
    #[ts(type = "number")]
    pub rev: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// A possible existing catalog item for a new breakdown entry (FSD §28.5, §143).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMatchDto {
    pub id: String,
    pub name: String,
    pub category: BreakdownCategory,
    pub status: CatalogStatus,
    /// Same identity (name or alias) — shown first; otherwise merely similar.
    pub exact: bool,
    pub used_in_label: String,
}

/// How a breakdown entry chooses its catalog item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum BreakdownChoiceMode {
    /// Reuse the single exact match, create when nothing similar exists,
    /// otherwise ask the user (`validation.ambiguous_match`).
    #[default]
    Auto,
    /// Use `catalogItemId`.
    Existing,
    /// Always create a new catalog item (FSD §143 "Create New").
    New,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownCatalogChoice {
    #[serde(default)]
    pub mode: BreakdownChoiceMode,
    #[serde(default)]
    pub catalog_item_id: Option<String>,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogListArgs {
    #[serde(default)]
    pub category: Option<BreakdownCategory>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogMatchArgs {
    pub category: String,
    pub name: String,
    #[serde(default)]
    pub exclude_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogCreateArgs {
    pub category: String,
    pub name: String,
    #[serde(default)]
    pub status: Option<CatalogStatus>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
}

/// Field patch: omitted = unchanged; empty text = cleared.
#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<CatalogStatus>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    #[ts(type = "number", optional = nullable)]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogImageArgs {
    pub id: String,
    /// A user-chosen image file, or null to remove the image.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogAliasArgs {
    pub id: String,
    pub alias: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogArchiveArgs {
    pub id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogReplaceArgs {
    pub from_id: String,
    pub to_id: String,
    /// Scenes whose association moves to `toId`. The old item is kept.
    pub scene_ids: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CatalogReplaceResult {
    #[ts(type = "number")]
    pub scenes_updated: i64,
}

// ------------------------------------------------------------------ reads

const ITEM_COLS: &str =
    "id, category, name, description, notes, contact, status, image_asset_id, archived,
    character_id, location_id, rev, updated_at";

struct ItemRow {
    id: String,
    category: String,
    name: String,
    description: Option<String>,
    notes: Option<String>,
    contact: Option<String>,
    status: String,
    image_asset_id: Option<String>,
    archived: bool,
    character_id: Option<String>,
    location_id: Option<String>,
    rev: i64,
    updated_at: i64,
}

fn item_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ItemRow> {
    Ok(ItemRow {
        id: r.get(0)?,
        category: r.get(1)?,
        name: r.get(2)?,
        description: r.get(3)?,
        notes: r.get(4)?,
        contact: r.get(5)?,
        status: r.get(6)?,
        image_asset_id: r.get(7)?,
        archived: r.get(8)?,
        character_id: r.get(9)?,
        location_id: r.get(10)?,
        rev: r.get(11)?,
        updated_at: r.get(12)?,
    })
}

fn aliases_by_item(c: &Connection) -> AppResult<HashMap<String, Vec<CatalogAliasDto>>> {
    let mut stmt =
        c.prepare("SELECT id, catalog_item_id, alias FROM catalog_alias ORDER BY alias")?;
    let mut out: HashMap<String, Vec<CatalogAliasDto>> = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, item, alias) = row?;
        out.entry(item)
            .or_default()
            .push(CatalogAliasDto { id, alias });
    }
    Ok(out)
}

fn to_dto(
    c: &Connection,
    root: &std::path::Path,
    row: ItemRow,
    aliases: &mut HashMap<String, Vec<CatalogAliasDto>>,
    usage: &HashMap<String, Vec<ProductionSceneRef>>,
) -> AppResult<CatalogItemDto> {
    let used_in = usage.get(&row.id).cloned().unwrap_or_default();
    Ok(CatalogItemDto {
        category: BreakdownCategory::parse(&row.category).unwrap_or(BreakdownCategory::Props),
        status: CatalogStatus::parse(&row.status).unwrap_or(CatalogStatus::Required),
        image: load_asset_opt(c, root, row.image_asset_id.as_deref())?,
        aliases: aliases.remove(&row.id).unwrap_or_default(),
        used_in_label: used_in_label(&used_in),
        used_in,
        id: row.id,
        name: row.name,
        description: row.description,
        notes: row.notes,
        contact: row.contact,
        archived: row.archived,
        character_id: row.character_id,
        location_id: row.location_id,
        rev: row.rev,
        updated_at: row.updated_at,
    })
}

pub(crate) fn load_item(
    c: &Connection,
    root: &std::path::Path,
    id: &str,
) -> AppResult<CatalogItemDto> {
    let row = c
        .query_row(
            &format!("SELECT {ITEM_COLS} FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL"),
            [id],
            item_row,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("catalog item"))?;
    let source = active_source(c)?;
    let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
    let usage = catalog_usage(c, &scenes)?;
    let mut aliases = aliases_by_item(c)?;
    to_dto(c, root, row, &mut aliases, &usage)
}

fn category_rank(cat: &str) -> usize {
    BreakdownCategory::ALL
        .iter()
        .position(|c| c.as_str() == cat)
        .unwrap_or(usize::MAX)
}

fn list(core: &AppCore, actor: &Actor, args: CatalogListArgs) -> AppResult<Vec<CatalogItemDto>> {
    actor.require(Capability::View, "view the production catalog")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    let needle = args
        .search
        .as_deref()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty());
    s.store.read(|c| {
        let mut stmt = c.prepare(&format!(
            "SELECT {ITEM_COLS} FROM catalog_item WHERE deleted_at IS NULL"
        ))?;
        let mut rows: Vec<ItemRow> = stmt.query_map([], item_row)?.collect::<Result<_, _>>()?;
        rows.sort_by(|a, b| {
            category_rank(&a.category)
                .cmp(&category_rank(&b.category))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        let source = active_source(c)?;
        let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
        let usage = catalog_usage(c, &scenes)?;
        let mut aliases = aliases_by_item(c)?;
        let mut out = Vec::new();
        for row in rows {
            if row.archived && !args.include_archived {
                continue;
            }
            if args
                .category
                .is_some_and(|cat| row.category != cat.as_str())
            {
                continue;
            }
            if let Some(n) = &needle {
                let alias_hit = aliases
                    .get(&row.id)
                    .map(|v| v.iter().any(|a| a.alias.to_lowercase().contains(n)))
                    .unwrap_or(false);
                let hit = row.name.to_lowercase().contains(n)
                    || row
                        .description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(n)
                    || row
                        .notes
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(n)
                    || alias_hit;
                if !hit {
                    continue;
                }
            }
            out.push(to_dto(c, &root, row, &mut aliases, &usage)?);
        }
        Ok(out)
    })
}

fn get(core: &AppCore, actor: &Actor, args: CatalogIdArgs) -> AppResult<CatalogItemDto> {
    actor.require(Capability::View, "view the production catalog")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &args.id))
}

// -------------------------------------------------------------- matching

#[derive(Debug, Clone)]
pub(crate) struct MatchRow {
    pub id: String,
    pub name: String,
    pub category: String,
    pub status: String,
    pub exact: bool,
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn similar(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let (pa, pb) = (format!(" {a} "), format!(" {b} "));
    if pa.contains(&pb) || pb.contains(&pa) {
        return true;
    }
    let shortest = a.chars().count().min(b.chars().count());
    shortest >= 4 && levenshtein(a, b) <= (shortest / 5).max(1)
}

/// Likely existing items for a name in a category: exact identity (name or
/// alias) first, then similar names. Archived and deleted items are not offered.
pub(crate) fn find_matches(
    c: &Connection,
    category: BreakdownCategory,
    name: &str,
    exclude: Option<&str>,
) -> AppResult<Vec<MatchRow>> {
    let key = name_key(name);
    if key.is_empty() {
        return Ok(vec![]);
    }
    let mut stmt = c.prepare(
        "SELECT id, name, category, status FROM catalog_item
         WHERE category = ?1 AND deleted_at IS NULL AND archived = 0 ORDER BY name",
    )?;
    let items: Vec<(String, String, String, String)> = stmt
        .query_map([category.as_str()], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut alias_stmt = c.prepare("SELECT catalog_item_id, alias_key FROM catalog_alias")?;
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();
    for row in alias_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (item, k) = row?;
        aliases.entry(item).or_default().push(k);
    }
    let mut out = Vec::new();
    for (id, iname, cat, status) in items {
        if Some(id.as_str()) == exclude {
            continue;
        }
        let ikey = name_key(&iname);
        let akeys = aliases.get(&id).cloned().unwrap_or_default();
        let exact = ikey == key || akeys.contains(&key);
        let sim = !exact && (similar(&ikey, &key) || akeys.iter().any(|k| similar(k, &key)));
        if exact || sim {
            out.push(MatchRow {
                id,
                name: iname,
                category: cat,
                status,
                exact,
            });
        }
    }
    out.sort_by_key(|m| !m.exact);
    Ok(out)
}

/// "exact" (one identical item, nothing else similar), "ambiguous" or "none".
pub(crate) fn match_kind(matches: &[MatchRow]) -> &'static str {
    match matches {
        [] => "none",
        [one] if one.exact => "exact",
        _ => "ambiguous",
    }
}

pub(crate) fn match_dtos(
    c: &Connection,
    scenes: &SourceScenes,
    matches: Vec<MatchRow>,
) -> AppResult<Vec<CatalogMatchDto>> {
    let usage = catalog_usage(c, scenes)?;
    Ok(match_dtos_with(&usage, matches))
}

pub(crate) fn match_dtos_with(
    usage: &HashMap<String, Vec<ProductionSceneRef>>,
    matches: Vec<MatchRow>,
) -> Vec<CatalogMatchDto> {
    matches
        .into_iter()
        .map(|m| CatalogMatchDto {
            used_in_label: used_in_label(usage.get(&m.id).map(|v| v.as_slice()).unwrap_or(&[])),
            category: BreakdownCategory::parse(&m.category).unwrap_or(BreakdownCategory::Props),
            status: CatalogStatus::parse(&m.status).unwrap_or(CatalogStatus::Required),
            id: m.id,
            name: m.name,
            exact: m.exact,
        })
        .collect()
}

fn find_matches_op(
    core: &AppCore,
    actor: &Actor,
    args: CatalogMatchArgs,
) -> AppResult<Vec<CatalogMatchDto>> {
    actor.require(Capability::View, "view the production catalog")?;
    let cat = parse_category(&args.category)?;
    let s = core.project()?;
    s.store.read(|c| {
        let source = active_source(c)?;
        let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
        let m = find_matches(c, cat, &args.name, args.exclude_id.as_deref())?;
        match_dtos(c, &scenes, m)
    })
}

// ------------------------------------------------------------- mutations

pub(crate) struct NewItem<'a> {
    pub category: BreakdownCategory,
    pub name: &'a str,
    pub status: CatalogStatus,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub contact: Option<String>,
}

/// Insert a catalog item. A Location / Set item is linked to a Location record
/// (an existing one with the same name, or a new "Idea"); a Cast item is linked
/// to the Story character of the same name when one exists (read-only lookup).
pub(crate) fn create_item(tx: &Tx<'_>, item: NewItem<'_>) -> AppResult<String> {
    let name = required_text(item.name, "Name", 200)?;
    let id = new_id();
    let now = now_ms();
    let character_id: Option<String> = if item.category == BreakdownCategory::Cast {
        let key = super::character_key(&name);
        super::script::story_characters(tx.conn())?
            .into_iter()
            .find(|(_, n)| super::character_key(n) == key)
            .map(|(id, _)| id)
    } else {
        None
    };
    tx.conn().execute(
        "INSERT INTO catalog_item(id, category, name, description, notes, contact, status, character_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            item.category.as_str(),
            name,
            item.description,
            item.notes,
            item.contact,
            item.status.as_str(),
            character_id,
            now
        ],
    )?;
    if item.category == BreakdownCategory::Location {
        let loc = super::locations::location_for_name(tx, &name)?;
        tx.conn().execute(
            "UPDATE catalog_item SET location_id = ?1 WHERE id = ?2",
            params![loc, id],
        )?;
    }
    Ok(id)
}

fn add_alias_row(tx: &Tx<'_>, item_id: &str, alias: &str) -> AppResult<bool> {
    let alias = alias.trim();
    let key = name_key(alias);
    if key.is_empty() {
        return Ok(false);
    }
    let (name,): (String,) = tx.conn().query_row(
        "SELECT name FROM catalog_item WHERE id = ?1",
        [item_id],
        |r| Ok((r.get(0)?,)),
    )?;
    if name_key(&name) == key {
        return Ok(false);
    }
    let exists: bool = tx.conn().query_row(
        "SELECT EXISTS(SELECT 1 FROM catalog_alias WHERE catalog_item_id = ?1 AND alias_key = ?2)",
        params![item_id, key],
        |r| r.get(0),
    )?;
    if exists {
        return Ok(false);
    }
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO catalog_alias(id, catalog_item_id, alias, alias_key, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![new_id(), item_id, alias, key, now],
    )?;
    tx.reindex("catalog_item", item_id);
    Ok(true)
}

/// Resolve the catalog item a breakdown entry should use (FSD §26.5, §143).
/// Never attaches an ambiguous match silently.
pub(crate) fn resolve_choice(
    tx: &Tx<'_>,
    category: BreakdownCategory,
    name: &str,
    choice: &BreakdownCatalogChoice,
) -> AppResult<String> {
    match choice.mode {
        BreakdownChoiceMode::Existing => {
            let id = choice
                .catalog_item_id
                .as_deref()
                .ok_or_else(|| AppError::required("Catalog item"))?;
            let row: Option<(String, String, bool)> = tx
                .conn()
                .query_row(
                    "SELECT category, name, archived FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let (cat, iname, archived) = row.ok_or_else(|| AppError::not_found("catalog item"))?;
            if archived {
                return Err(AppError::validation(
                    "catalog_item",
                    format!(
                        "“{iname}” is archived. Restore it in the Catalog to use it again, or create a new item."
                    ),
                ));
            }
            if cat != category.as_str() {
                return Err(AppError::validation(
                    "category",
                    format!(
                        "“{iname}” is in {cat}. Choose an item from {}, or create a new one.",
                        category.as_str()
                    ),
                ));
            }
            // Remember the name this scene used, so later matching finds the same identity.
            add_alias_row(tx, id, name)?;
            Ok(id.to_string())
        }
        BreakdownChoiceMode::New => create_item(
            tx,
            NewItem {
                category,
                name,
                status: CatalogStatus::Required,
                description: None,
                notes: None,
                contact: None,
            },
        ),
        BreakdownChoiceMode::Auto => {
            let m = find_matches(tx.conn(), category, name, None)?;
            match match_kind(&m) {
                "none" => create_item(
                    tx,
                    NewItem {
                        category,
                        name,
                        status: CatalogStatus::Required,
                        description: None,
                        notes: None,
                        contact: None,
                    },
                ),
                "exact" => Ok(m[0].id.clone()),
                _ => Err(AppError::new(
                    "validation.ambiguous_match",
                    format!(
                        "The catalog already has something similar to “{}”. Choose the existing item or create a new one.",
                        name.trim()
                    ),
                )),
            }
        }
    }
}

fn create(core: &AppCore, actor: &Actor, args: CatalogCreateArgs) -> AppResult<CatalogItemDto> {
    let cat = parse_category(&args.category)?;
    let name = required_text(&args.name, "Name", 200)?;
    let description = optional_text(args.description, "Description", 4_000)?;
    let notes = optional_text(args.notes, "Notes", 20_000)?;
    let contact = optional_text(args.contact, "Contact", 500)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "catalog.create",
            format!("Added “{name}” to the catalog"),
            Capability::Edit,
        ),
        |tx| {
            create_item(
                tx,
                NewItem {
                    category: cat,
                    name: &name,
                    status: args.status.unwrap_or(CatalogStatus::Required),
                    description,
                    notes,
                    contact,
                },
            )
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &id))
}

fn update(core: &AppCore, actor: &Actor, args: CatalogUpdateArgs) -> AppResult<CatalogItemDto> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut new_name = None;
    if let Some(n) = &args.name {
        let n = required_text(n, "Name", 200)?;
        new_name = Some(n.clone());
        fields.push(("name", text(n)));
    }
    if let Some(st) = args.status {
        fields.push(("status", text(st.as_str())));
    }
    if let Some(v) = patch_text(args.description, "Description", 4_000)? {
        fields.push(("description", v));
    }
    if let Some(v) = patch_text(args.notes, "Notes", 20_000)? {
        fields.push(("notes", v));
    }
    if let Some(v) = patch_text(args.contact, "Contact", 500)? {
        fields.push(("contact", v));
    }
    let s = core.project()?;
    let label: String = s.store.read(|c| {
        c.query_row(
            "SELECT name FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
            [&args.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("catalog item"))
    })?;
    let summary = match (&new_name, args.status) {
        (Some(n), _) if *n != label => format!("Renamed “{label}” to “{n}”"),
        (_, Some(st)) if fields.len() == 1 => format!("Set “{label}” to {}", st.as_str()),
        _ => format!("Edited “{label}”"),
    };
    s.store.mutate(
        actor,
        MutationMeta::new("catalog.update", summary, Capability::Edit)
            .target("catalog_item", &args.id)
            .coalesce(format!("catalog.update:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "catalog_item",
                &args.id,
                &fields,
                &["name", "status", "description", "notes", "contact"],
                args.expected_rev,
                "catalog item",
            )?;
            if let Some(n) = &new_name {
                // The previous name keeps matching the same identity.
                if name_key(n) != name_key(&label) {
                    add_alias_row(tx, &args.id, &label)?;
                }
                // One identity: a Location / Set item and its Location record share the name.
                let loc: Option<String> = tx.conn().query_row(
                    "SELECT location_id FROM catalog_item WHERE id = ?1",
                    [&args.id],
                    |r| r.get(0),
                )?;
                if let Some(loc) = loc {
                    let cur: Option<String> = tx
                        .conn()
                        .query_row(
                            "SELECT name FROM location WHERE id = ?1 AND deleted_at IS NULL",
                            [&loc],
                            |r| r.get(0),
                        )
                        .optional()?;
                    if cur.as_deref().is_some_and(|c| c != n) {
                        update_fields(
                            tx.conn(),
                            "location",
                            &loc,
                            &[("name", text(n.clone()))],
                            &["name"],
                            None,
                            "location",
                        )?;
                    }
                }
            }
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &args.id))
}

fn set_image(core: &AppCore, actor: &Actor, args: CatalogImageArgs) -> AppResult<CatalogItemDto> {
    let s = core.project()?;
    let summary = if args.path.is_some() {
        "Added a catalog image"
    } else {
        "Removed a catalog image"
    };
    s.store.mutate(
        actor,
        MutationMeta::new("catalog.set_image", summary, Capability::Edit)
            .target("catalog_item", &args.id),
        |tx| {
            let asset = match &args.path {
                Some(p) => {
                    let a = ingest_file(tx, &PathBuf::from(p))?;
                    if !a.media_type.starts_with("image/") {
                        return Err(AppError::invalid_input(
                            "Please choose an image file (JPG, PNG, GIF, WebP).",
                        ));
                    }
                    Some(a.id)
                }
                None => None,
            };
            // The previous image stays in the project so Undo can bring it back.
            update_fields(
                tx.conn(),
                "catalog_item",
                &args.id,
                &[("image_asset_id", opt_text(asset))],
                &["image_asset_id"],
                None,
                "catalog item",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &args.id))
}

fn add_alias(core: &AppCore, actor: &Actor, args: CatalogAliasArgs) -> AppResult<CatalogItemDto> {
    let alias = required_text(&args.alias, "Also known as", 200)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "catalog.add_alias",
            format!("Added “{alias}” as another name"),
            Capability::Edit,
        )
        .target("catalog_item", &args.id),
        |tx| {
            let ok: bool = tx.conn().query_row(
                "SELECT EXISTS(SELECT 1 FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL)",
                [&args.id],
                |r| r.get(0),
            )?;
            if !ok {
                return Err(AppError::not_found("catalog item"));
            }
            add_alias_row(tx, &args.id, &alias)?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &args.id))
}

fn remove_alias(core: &AppCore, actor: &Actor, args: CatalogIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (item, alias): (String, String) = s.store.read(|c| {
        c.query_row(
            "SELECT catalog_item_id, alias FROM catalog_alias WHERE id = ?1",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("name"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "catalog.remove_alias",
            format!("Removed the name “{alias}”"),
            Capability::Edit,
        )
        .target("catalog_item", &item),
        |tx| {
            tx.conn()
                .execute("DELETE FROM catalog_alias WHERE id = ?1", [&args.id])?;
            tx.reindex("catalog_item", &item);
            Ok(())
        },
    )
}

fn set_archived(
    core: &AppCore,
    actor: &Actor,
    args: CatalogArchiveArgs,
) -> AppResult<CatalogItemDto> {
    let s = core.project()?;
    let name: String = s.store.read(|c| {
        c.query_row(
            "SELECT name FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
            [&args.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("catalog item"))
    })?;
    let summary = if args.archived {
        format!("Archived “{name}”")
    } else {
        format!("Restored “{name}” from the archive")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("catalog.set_archived", summary, Capability::Edit)
            .target("catalog_item", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "catalog_item",
                &args.id,
                &[("archived", int(args.archived as i64))],
                &["archived"],
                None,
                "catalog item",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| load_item(c, &root, &args.id))
}

/// Recoverable delete. Scene associations are kept and show the item as
/// removed until it is restored (Domain §12).
fn delete(core: &AppCore, actor: &Actor, args: CatalogIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (name, category): (String, String) = s.store.read(|c| {
        c.query_row(
            "SELECT name, category FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
            [&args.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("catalog item"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "catalog.delete",
            format!("Deleted catalog item “{name}”"),
            Capability::SoftDelete,
        )
        .target("catalog_item", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "catalog_item",
                    table: "catalog_item",
                    id: &args.id,
                    title: Some(name.clone()),
                    parent_type: Some("catalog_category"),
                    parent_id: Some(category.clone()),
                    position: None,
                },
            )
        },
    )
}

/// Move scene associations from one item to another for the given scenes.
/// The old item is kept (FSD §98). Returns the number of scenes changed.
pub(crate) fn replace_item_in_scenes(
    tx: &Tx<'_>,
    from_ids: &[String],
    to_id: &str,
    scene_ids: &[String],
) -> AppResult<i64> {
    let (to_name, to_cat): (String, String) = tx
        .conn()
        .query_row(
            "SELECT name, category FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
            [to_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("catalog item"))?;
    let mut changed = 0i64;
    for scene in scene_ids {
        let mut touched = false;
        for from in from_ids {
            let mut stmt = tx.conn().prepare(&format!(
                "SELECT id, display_name FROM breakdown_element WHERE scene_id = ?1 AND catalog_item_id = ?2
                   AND deleted_at IS NULL AND confirmation_state IN {}",
                super::PRODUCTION_STATES
            ))?;
            let rows: Vec<(String, String)> = stmt
                .query_map(params![scene, from], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            for (el, display) in rows {
                let already: bool = tx.conn().query_row(
                    &format!(
                        "SELECT EXISTS(SELECT 1 FROM breakdown_element WHERE scene_id = ?1 AND catalog_item_id = ?2
                           AND deleted_at IS NULL AND confirmation_state IN {})",
                        super::PRODUCTION_STATES
                    ),
                    params![scene, to_id],
                    |r| r.get(0),
                )?;
                if already {
                    // The scene already requires the replacement: the old association goes to
                    // Recently Deleted (recoverable) instead of being duplicated.
                    soft_delete(
                        tx,
                        DeleteSpec {
                            object_type: "breakdown_element",
                            table: "breakdown_element",
                            id: &el,
                            title: Some(display),
                            parent_type: Some("screenplay_scene"),
                            parent_id: Some(scene.clone()),
                            position: None,
                        },
                    )?;
                } else {
                    update_fields(
                        tx.conn(),
                        "breakdown_element",
                        &el,
                        &[
                            ("catalog_item_id", text(to_id)),
                            ("display_name", text(to_name.clone())),
                            ("category", text(to_cat.clone())),
                        ],
                        &["catalog_item_id", "display_name", "category"],
                        None,
                        "breakdown element",
                    )?;
                }
                touched = true;
            }
        }
        if touched {
            changed += 1;
        }
    }
    Ok(changed)
}

fn replace_in_scenes(
    core: &AppCore,
    actor: &Actor,
    args: CatalogReplaceArgs,
) -> AppResult<CatalogReplaceResult> {
    if args.from_id == args.to_id {
        return Err(AppError::invalid_input(
            "Choose a different item to replace it with.",
        ));
    }
    if args.scene_ids.is_empty() {
        return Err(AppError::required("At least one scene"));
    }
    let s = core.project()?;
    let (from_name, from_cat, to_name, to_cat, to_archived): (String, String, String, String, bool) = s.store.read(|c| {
        let from: (String, String) = c
            .query_row("SELECT name, category FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL", [&args.from_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?
            .ok_or_else(|| AppError::not_found("catalog item"))?;
        let to: (String, String, bool) = c
            .query_row(
                "SELECT name, category, archived FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL",
                [&args.to_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("catalog item"))?;
        Ok((from.0, from.1, to.0, to.1, to.2))
    })?;
    if from_cat != to_cat {
        return Err(AppError::validation(
            "category",
            "The replacement must be in the same category.",
        ));
    }
    if to_archived {
        return Err(AppError::validation(
            "catalog_item",
            format!("“{to_name}” is archived. Restore it before using it."),
        ));
    }
    let n = s.store.mutate(
        actor,
        MutationMeta::new(
            "catalog.replace_in_scenes",
            format!("Replaced “{from_name}” with “{to_name}” in selected scenes"),
            Capability::Edit,
        )
        .target("catalog_item", &args.from_id),
        |tx| {
            replace_item_in_scenes(
                tx,
                std::slice::from_ref(&args.from_id),
                &args.to_id,
                &args.scene_ids,
            )
        },
    )?;
    Ok(CatalogReplaceResult { scenes_updated: n })
}

// ------------------------------------------------------- search / trash

#[allow(clippy::type_complexity)]
fn index_item(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>, String, Option<i64>)> = c
        .query_row(
            "SELECT name, category, description, notes, contact, status, deleted_at FROM catalog_item WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?;
    let Some((name, category, description, notes, contact, status, None)) = row else {
        return Ok(None);
    };
    let mut stmt = c.prepare("SELECT alias FROM catalog_alias WHERE catalog_item_id = ?1")?;
    let aliases: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(Some(SearchDoc {
        entity_type: "catalog_item".into(),
        title: name,
        body: [
            category,
            status,
            description.unwrap_or_default(),
            notes.unwrap_or_default(),
            contact.unwrap_or_default(),
            aliases.join(" "),
        ]
        .join(" "),
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "catalog", "catalogItemId": id }),
        owner_user_id: None,
    }))
}

fn purge_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let id = &row.object_id;
    let image: Option<String> = tx
        .conn()
        .query_row(
            "SELECT image_asset_id FROM catalog_item WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    // Affected breakdown entries keep their name and show "catalog item removed" (Domain §12).
    tx.conn().execute(
        "UPDATE breakdown_element SET catalog_item_id = NULL, catalog_removed_at = ?1, updated_at = ?1, rev = rev + 1
         WHERE catalog_item_id = ?2",
        params![now_ms(), id],
    )?;
    tx.conn()
        .execute("DELETE FROM catalog_alias WHERE catalog_item_id = ?1", [id])?;
    tx.conn()
        .execute("DELETE FROM catalog_item WHERE id = ?1", [id])?;
    if let Some(a) = image {
        crate::modules::files::purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_rules() {
        assert!(similar("red folder", "folder"));
        assert!(similar("pistol", "pistl"));
        assert!(!similar("pistol", "knife"));
        assert!(!similar("car", "cat"), "short words need an exact match");
    }
}
