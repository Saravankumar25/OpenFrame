//! Cast & Crew (FSD §30, §100): a small-team directory — no payroll, no contracts.
//!
//! - The Character is the story object; the cast member is the real person
//!   attached to it. One primary performer per character by default;
//!   alternates / double casting are representable.
//! - Scenes involving a character are derived from character cues in the
//!   Production Source and from Cast breakdown elements.
//! - Availability is free-form notes only.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, opt_text, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{
    PRODUCTION_STATES, ProductionSceneRef, SourceScenes, active_source, character_key, patch_text,
    scene_ref, script, sort_refs,
};
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, ingest_file, load_asset_opt, optional_text, required_text};

pub fn register(r: &mut Registry) {
    r.query("cast.list", cast_list);
    r.command("cast.create", cast_create);
    r.command("cast.update", cast_update);
    r.command("cast.assign", cast_assign);
    r.command("cast.set_photo", cast_set_photo);
    r.command("cast.set_archived", cast_set_archived);
    r.command("cast.delete", cast_delete);
    r.query("crew.list", crew_list);
    r.command("crew.create", crew_create);
    r.command("crew.update", crew_update);
    r.command("crew.set_archived", crew_set_archived);
    r.command("crew.delete", crew_delete);
    r.indexer("cast_member", index_cast);
    r.indexer("crew_member", index_crew);
    r.trash_handler(TrashHandler {
        object_type: "cast_member",
        table: "cast_member",
        label: "Cast member",
        restore: None,
        purge: purge_cast,
    });
    r.trash_handler(TrashHandler {
        object_type: "crew_member",
        table: "crew_member",
        label: "Crew member",
        restore: None,
        purge: purge_crew,
    });
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionCastMemberDto {
    pub id: String,
    pub person_name: String,
    pub character_id: Option<String>,
    /// Character as shown in the script (upper case), e.g. "ARJUN".
    pub character_name: Option<String>,
    pub is_primary: bool,
    pub contact: Option<String>,
    pub photo: Option<AssetInfo>,
    pub notes: Option<String>,
    pub availability_notes: Option<String>,
    pub archived: bool,
    pub scenes: Vec<ProductionSceneRef>,
    #[ts(type = "number")]
    pub rev: i64,
}

/// A character production knows about (script cues, Story characters, Cast elements).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionCastCharacterDto {
    pub name: String,
    pub story_character_id: Option<String>,
    pub scenes: Vec<ProductionSceneRef>,
    /// Primary performer; None = "Needs casting".
    pub primary_cast_id: Option<String>,
    pub cast_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionCastDirectory {
    pub members: Vec<ProductionCastMemberDto>,
    pub characters: Vec<ProductionCastCharacterDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionCrewMemberDto {
    pub id: String,
    pub person_name: String,
    pub role: String,
    pub department: Option<String>,
    pub contact: Option<String>,
    pub notes: Option<String>,
    pub archived: bool,
    #[ts(type = "number")]
    pub rev: i64,
}

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionPeopleListArgs {
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionPeopleIdArgs {
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCastCreateArgs {
    pub person_name: String,
    /// A Story character, or…
    #[serde(default)]
    pub character_id: Option<String>,
    /// …a character name as used in the script (e.g. "THE STATION MASTER").
    #[serde(default)]
    pub character_name: Option<String>,
    /// Default: primary when the character has no primary performer yet.
    #[serde(default)]
    pub primary: Option<bool>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub availability_notes: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCastUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub person_name: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub availability_notes: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number", optional = nullable)]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCastAssignArgs {
    pub id: String,
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub character_name: Option<String>,
    /// Primary performer (others for the character become alternates) or an alternate.
    pub primary: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCastPhotoArgs {
    pub id: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionPeopleArchiveArgs {
    pub id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCrewCreateArgs {
    pub person_name: String,
    pub role: String,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionCrewUpdateArgs {
    pub id: String,
    #[serde(default)]
    pub person_name: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub contact: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number", optional = nullable)]
    pub expected_rev: Option<i64>,
}

// ------------------------------------------------------------ characters

#[derive(Debug, Clone)]
pub(crate) struct CharacterEntry {
    pub key: String,
    pub story_character_id: Option<String>,
    pub scene_ids: BTreeSet<String>,
    order: usize,
}

/// Characters known to production, in script order: cues in the Production
/// Source, Story characters, and characters of Cast breakdown elements.
pub(crate) fn known_characters(
    c: &Connection,
    scenes: &SourceScenes,
    draft_id: Option<&str>,
) -> AppResult<Vec<CharacterEntry>> {
    let mut map: HashMap<String, CharacterEntry> = HashMap::new();
    let mut next = 0usize;
    let mut entry = |map: &mut HashMap<String, CharacterEntry>, key: &str| -> String {
        if !map.contains_key(key) {
            map.insert(
                key.to_string(),
                CharacterEntry {
                    key: key.to_string(),
                    story_character_id: None,
                    scene_ids: BTreeSet::new(),
                    order: next,
                },
            );
            next += 1;
        }
        key.to_string()
    };
    if let Some(d) = draft_id {
        let els = script::draft_elements(c, d)?;
        for row in &scenes.rows {
            for e in els.get(&row.id).map(|v| v.as_slice()).unwrap_or(&[]) {
                if e.element_type != "character" {
                    continue;
                }
                if let Some(name) = script::cue_name(&e.text) {
                    let k = entry(&mut map, &name);
                    map.get_mut(&k).unwrap().scene_ids.insert(row.id.clone());
                }
            }
        }
    }
    for (id, name) in script::story_characters(c)? {
        let k = entry(&mut map, &character_key(&name));
        let e = map.get_mut(&k).unwrap();
        if e.story_character_id.is_none() {
            e.story_character_id = Some(id);
        }
    }
    // Cast breakdown elements (production truth) also place a character in a scene.
    let mut stmt = c.prepare(&format!(
        "SELECT e.scene_id, ci.name, ci.character_id FROM breakdown_element e JOIN catalog_item ci ON ci.id = e.catalog_item_id
         WHERE e.category = ?1 AND e.deleted_at IS NULL AND e.archived = 0 AND e.confirmation_state IN {PRODUCTION_STATES}"
    ))?;
    let rows: Vec<(String, String, Option<String>)> = stmt
        .query_map([BreakdownCategory::Cast.as_str()], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<Result<_, _>>()?;
    for (scene, name, char_id) in rows {
        if scenes.get(&scene).is_none() {
            continue;
        }
        let by_story = char_id.as_ref().and_then(|cid| {
            map.values()
                .find(|e| e.story_character_id.as_ref() == Some(cid))
                .map(|e| e.key.clone())
        });
        let k = match by_story {
            Some(k) => k,
            None => entry(&mut map, &character_key(&name)),
        };
        map.get_mut(&k).unwrap().scene_ids.insert(scene);
    }
    let mut out: Vec<CharacterEntry> = map.into_values().collect();
    out.sort_by_key(|e| e.order);
    Ok(out)
}

struct CastRow {
    id: String,
    person_name: String,
    character_id: Option<String>,
    character_name: Option<String>,
    is_primary: bool,
    contact: Option<String>,
    photo_asset_id: Option<String>,
    notes: Option<String>,
    availability_notes: Option<String>,
    archived: bool,
    rev: i64,
}

fn cast_rows(c: &Connection) -> AppResult<Vec<CastRow>> {
    let mut stmt = c.prepare(
        "SELECT id, person_name, character_id, character_name, is_primary, contact, photo_asset_id, notes,
                availability_notes, archived, rev
         FROM cast_member WHERE deleted_at IS NULL ORDER BY person_name COLLATE NOCASE, id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(CastRow {
                id: r.get(0)?,
                person_name: r.get(1)?,
                character_id: r.get(2)?,
                character_name: r.get(3)?,
                is_primary: r.get(4)?,
                contact: r.get(5)?,
                photo_asset_id: r.get(6)?,
                notes: r.get(7)?,
                availability_notes: r.get(8)?,
                archived: r.get(9)?,
                rev: r.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The character key a cast member plays.
fn member_key(
    c: &Connection,
    row_character_id: Option<&str>,
    row_character_name: Option<&str>,
) -> AppResult<Option<String>> {
    if let Some(n) = row_character_id
        .map(|id| script::story_character_name(c, id))
        .transpose()?
        .flatten()
    {
        return Ok(Some(character_key(&n)));
    }
    Ok(row_character_name
        .map(character_key)
        .filter(|k| !k.is_empty()))
}

pub(crate) fn directory(
    c: &Connection,
    root: &std::path::Path,
    include_archived: bool,
) -> AppResult<ProductionCastDirectory> {
    let source = active_source(c)?;
    let scenes = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
    let chars = known_characters(c, &scenes, source.as_ref().map(|s| s.draft_id.as_str()))?;
    let refs = |set: &BTreeSet<String>| -> AppResult<Vec<ProductionSceneRef>> {
        let mut v = set
            .iter()
            .map(|id| scene_ref(c, &scenes, id))
            .collect::<AppResult<Vec<_>>>()?;
        sort_refs(&mut v);
        Ok(v)
    };
    let rows = cast_rows(c)?;
    let mut members = Vec::new();
    let mut by_key: HashMap<String, Vec<(String, bool, bool)>> = HashMap::new();
    for row in rows {
        let key = member_key(
            c,
            row.character_id.as_deref(),
            row.character_name.as_deref(),
        )?;
        if let Some(k) = &key {
            by_key.entry(k.clone()).or_default().push((
                row.id.clone(),
                row.is_primary,
                row.archived,
            ));
        }
        if row.archived && !include_archived {
            continue;
        }
        let scene_set = key
            .as_ref()
            .and_then(|k| chars.iter().find(|e| &e.key == k))
            .map(|e| e.scene_ids.clone())
            .unwrap_or_default();
        members.push(ProductionCastMemberDto {
            photo: load_asset_opt(c, root, row.photo_asset_id.as_deref())?,
            scenes: refs(&scene_set)?,
            character_name: key.clone(),
            id: row.id,
            person_name: row.person_name,
            character_id: row.character_id,
            is_primary: row.is_primary,
            contact: row.contact,
            notes: row.notes,
            availability_notes: row.availability_notes,
            archived: row.archived,
            rev: row.rev,
        });
    }
    let mut characters = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for e in &chars {
        seen.insert(e.key.clone());
        let cast = by_key.get(&e.key).cloned().unwrap_or_default();
        let active: Vec<&(String, bool, bool)> =
            cast.iter().filter(|(_, _, archived)| !archived).collect();
        characters.push(ProductionCastCharacterDto {
            name: e.key.clone(),
            story_character_id: e.story_character_id.clone(),
            scenes: refs(&e.scene_ids)?,
            primary_cast_id: active
                .iter()
                .find(|(_, p, _)| *p)
                .map(|(id, _, _)| id.clone()),
            cast_ids: active.iter().map(|(id, _, _)| id.clone()).collect(),
        });
    }
    // Characters that exist only through a cast assignment.
    for (k, cast) in by_key {
        if seen.contains(&k) {
            continue;
        }
        let active: Vec<&(String, bool, bool)> = cast.iter().filter(|(_, _, a)| !a).collect();
        if active.is_empty() {
            continue;
        }
        characters.push(ProductionCastCharacterDto {
            name: k,
            story_character_id: None,
            scenes: vec![],
            primary_cast_id: active
                .iter()
                .find(|(_, p, _)| *p)
                .map(|(id, _, _)| id.clone()),
            cast_ids: active.iter().map(|(id, _, _)| id.clone()).collect(),
        });
    }
    Ok(ProductionCastDirectory {
        members,
        characters,
    })
}

fn cast_list(
    core: &AppCore,
    actor: &Actor,
    args: ProductionPeopleListArgs,
) -> AppResult<ProductionCastDirectory> {
    actor.require(Capability::View, "view cast and crew")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| directory(c, &root, args.include_archived))
}

fn cast_member(
    c: &Connection,
    root: &std::path::Path,
    id: &str,
) -> AppResult<ProductionCastMemberDto> {
    directory(c, root, true)?
        .members
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| AppError::not_found("cast member"))
}

// ------------------------------------------------------------ cast writes

/// (story character id, character key) from the arguments.
fn resolve_character(
    c: &Connection,
    character_id: Option<&str>,
    character_name: Option<&str>,
) -> AppResult<(Option<String>, String)> {
    if let Some(id) = character_id.filter(|s| !s.is_empty()) {
        let name =
            script::story_character_name(c, id)?.ok_or_else(|| AppError::not_found("character"))?;
        return Ok((Some(id.to_string()), character_key(&name)));
    }
    let name = character_name
        .map(character_key)
        .filter(|k| !k.is_empty())
        .ok_or_else(|| AppError::required("Character"))?;
    if name.chars().count() > 200 {
        return Err(AppError::invalid_input(
            "Character name is too long (maximum 200 characters).",
        ));
    }
    let story = script::story_characters(c)?
        .into_iter()
        .find(|(_, n)| character_key(n) == name)
        .map(|(id, _)| id);
    Ok((story, name))
}

/// Make `member_id` the only primary performer of `key` (others become alternates).
fn demote_others(tx: &Tx<'_>, key: &str, member_id: &str) -> AppResult<()> {
    for row in cast_rows(tx.conn())? {
        if row.id == member_id || !row.is_primary {
            continue;
        }
        if member_key(
            tx.conn(),
            row.character_id.as_deref(),
            row.character_name.as_deref(),
        )?
        .as_deref()
            == Some(key)
        {
            update_fields(
                tx.conn(),
                "cast_member",
                &row.id,
                &[("is_primary", int(0))],
                &["is_primary"],
                None,
                "cast member",
            )?;
        }
    }
    Ok(())
}

fn has_primary(c: &Connection, key: &str, except: Option<&str>) -> AppResult<bool> {
    for row in cast_rows(c)? {
        if Some(row.id.as_str()) == except || !row.is_primary || row.archived {
            continue;
        }
        if member_key(
            c,
            row.character_id.as_deref(),
            row.character_name.as_deref(),
        )?
        .as_deref()
            == Some(key)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn cast_create(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCastCreateArgs,
) -> AppResult<ProductionCastMemberDto> {
    let person = required_text(&args.person_name, "Person name", 200)?;
    let contact = optional_text(args.contact, "Contact", 500)?;
    let availability = optional_text(args.availability_notes, "Availability notes", 4_000)?;
    let notes = optional_text(args.notes, "Notes", 20_000)?;
    let s = core.project()?;
    let id = new_id();
    s.store.mutate(actor, MutationMeta::new("cast.create", format!("Added {person} to the cast"), Capability::Edit), |tx| {
        let (char_id, key) = resolve_character(tx.conn(), args.character_id.as_deref(), args.character_name.as_deref())?;
        let primary = match args.primary {
            Some(p) => p,
            None => !has_primary(tx.conn(), &key, None)?,
        };
        let now = now_ms();
        tx.conn().execute(
            "INSERT INTO cast_member(id, person_name, character_id, character_name, is_primary, contact, availability_notes, notes,
                                     created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![id, person, char_id, key, primary, contact, availability, notes, now],
        )?;
        if primary {
            demote_others(tx, &key, &id)?;
        }
        Ok(())
    })?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| cast_member(c, &root, &id))
}

fn cast_label(c: &Connection, id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT person_name FROM cast_member WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("cast member"))
}

fn cast_update(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCastUpdateArgs,
) -> AppResult<ProductionCastMemberDto> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(n) = &args.person_name {
        fields.push(("person_name", text(required_text(n, "Person name", 200)?)));
    }
    if let Some(v) = patch_text(args.contact, "Contact", 500)? {
        fields.push(("contact", v));
    }
    if let Some(v) = patch_text(args.availability_notes, "Availability notes", 4_000)? {
        fields.push(("availability_notes", v));
    }
    if let Some(v) = patch_text(args.notes, "Notes", 20_000)? {
        fields.push(("notes", v));
    }
    let s = core.project()?;
    let label = s.store.read(|c| cast_label(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new("cast.update", format!("Edited {label}"), Capability::Edit)
            .target("cast_member", &args.id)
            .coalesce(format!("cast.update:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "cast_member",
                &args.id,
                &fields,
                &["person_name", "contact", "availability_notes", "notes"],
                args.expected_rev,
                "cast member",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| cast_member(c, &root, &args.id))
}

fn cast_assign(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCastAssignArgs,
) -> AppResult<ProductionCastMemberDto> {
    let s = core.project()?;
    let label = s.store.read(|c| cast_label(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "cast.assign",
            format!("Assigned a character to {label}"),
            Capability::Edit,
        )
        .target("cast_member", &args.id),
        |tx| {
            let (char_id, key) = resolve_character(
                tx.conn(),
                args.character_id.as_deref(),
                args.character_name.as_deref(),
            )?;
            update_fields(
                tx.conn(),
                "cast_member",
                &args.id,
                &[
                    ("character_id", opt_text(char_id)),
                    ("character_name", text(key.clone())),
                    ("is_primary", int(args.primary as i64)),
                ],
                &["character_id", "character_name", "is_primary"],
                None,
                "cast member",
            )?;
            if args.primary {
                demote_others(tx, &key, &args.id)?;
            }
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| cast_member(c, &root, &args.id))
}

fn cast_set_photo(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCastPhotoArgs,
) -> AppResult<ProductionCastMemberDto> {
    let s = core.project()?;
    let label = s.store.read(|c| cast_label(c, &args.id))?;
    let summary = if args.path.is_some() {
        format!("Added a photo of {label}")
    } else {
        format!("Removed the photo of {label}")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("cast.set_photo", summary, Capability::Edit)
            .target("cast_member", &args.id),
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
            update_fields(
                tx.conn(),
                "cast_member",
                &args.id,
                &[("photo_asset_id", opt_text(asset))],
                &["photo_asset_id"],
                None,
                "cast member",
            )?;
            Ok(())
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| cast_member(c, &root, &args.id))
}

fn cast_set_archived(
    core: &AppCore,
    actor: &Actor,
    args: ProductionPeopleArchiveArgs,
) -> AppResult<()> {
    let s = core.project()?;
    let label = s.store.read(|c| cast_label(c, &args.id))?;
    let summary = if args.archived {
        format!("Archived {label}")
    } else {
        format!("Restored {label} from the archive")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("cast.set_archived", summary, Capability::Edit)
            .target("cast_member", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "cast_member",
                &args.id,
                &[("archived", int(args.archived as i64))],
                &["archived"],
                None,
                "cast member",
            )?;
            Ok(())
        },
    )
}

/// Recoverable. Exported documents and snapshots keep their copies (FSD §100).
fn cast_delete(core: &AppCore, actor: &Actor, args: ProductionPeopleIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let label = s.store.read(|c| cast_label(c, &args.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "cast.delete",
            format!("Removed {label} from the cast"),
            Capability::SoftDelete,
        )
        .target("cast_member", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "cast_member",
                    table: "cast_member",
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

// ------------------------------------------------------------------- crew

const CREW_COLS: &str = "id, person_name, role, department, contact, notes, archived, rev";

fn crew_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ProductionCrewMemberDto> {
    Ok(ProductionCrewMemberDto {
        id: r.get(0)?,
        person_name: r.get(1)?,
        role: r.get(2)?,
        department: r.get(3)?,
        contact: r.get(4)?,
        notes: r.get(5)?,
        archived: r.get(6)?,
        rev: r.get(7)?,
    })
}

fn crew_get(c: &Connection, id: &str) -> AppResult<ProductionCrewMemberDto> {
    c.query_row(
        &format!("SELECT {CREW_COLS} FROM crew_member WHERE id = ?1 AND deleted_at IS NULL"),
        [id],
        crew_row,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("crew member"))
}

fn crew_list(
    core: &AppCore,
    actor: &Actor,
    args: ProductionPeopleListArgs,
) -> AppResult<Vec<ProductionCrewMemberDto>> {
    actor.require(Capability::View, "view cast and crew")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(&format!(
            "SELECT {CREW_COLS} FROM crew_member WHERE deleted_at IS NULL AND (?1 OR archived = 0)
             ORDER BY COALESCE(department, '~') COLLATE NOCASE, person_name COLLATE NOCASE, id"
        ))?;
        let rows = stmt
            .query_map([args.include_archived], crew_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn crew_create(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCrewCreateArgs,
) -> AppResult<ProductionCrewMemberDto> {
    let person = required_text(&args.person_name, "Person name", 200)?;
    let role = required_text(&args.role, "Role", 120)?;
    let department = optional_text(args.department, "Department", 120)?;
    let contact = optional_text(args.contact, "Contact", 500)?;
    let notes = optional_text(args.notes, "Notes", 20_000)?;
    let s = core.project()?;
    let id = new_id();
    s.store.mutate(actor, MutationMeta::new("crew.create", format!("Added {person} ({role}) to the crew"), Capability::Edit), |tx| {
        let now = now_ms();
        tx.conn().execute(
            "INSERT INTO crew_member(id, person_name, role, department, contact, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, person, role, department, contact, notes, now],
        )?;
        Ok(())
    })?;
    s.store.read(|c| crew_get(c, &id))
}

fn crew_update(
    core: &AppCore,
    actor: &Actor,
    args: ProductionCrewUpdateArgs,
) -> AppResult<ProductionCrewMemberDto> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(n) = &args.person_name {
        fields.push(("person_name", text(required_text(n, "Person name", 200)?)));
    }
    if let Some(r) = &args.role {
        fields.push(("role", text(required_text(r, "Role", 120)?)));
    }
    if let Some(v) = patch_text(args.department, "Department", 120)? {
        fields.push(("department", v));
    }
    if let Some(v) = patch_text(args.contact, "Contact", 500)? {
        fields.push(("contact", v));
    }
    if let Some(v) = patch_text(args.notes, "Notes", 20_000)? {
        fields.push(("notes", v));
    }
    let s = core.project()?;
    let label = s.store.read(|c| crew_get(c, &args.id))?.person_name;
    s.store.mutate(
        actor,
        MutationMeta::new("crew.update", format!("Edited {label}"), Capability::Edit)
            .target("crew_member", &args.id)
            .coalesce(format!("crew.update:{}", args.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "crew_member",
                &args.id,
                &fields,
                &["person_name", "role", "department", "contact", "notes"],
                args.expected_rev,
                "crew member",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| crew_get(c, &args.id))
}

fn crew_set_archived(
    core: &AppCore,
    actor: &Actor,
    args: ProductionPeopleArchiveArgs,
) -> AppResult<ProductionCrewMemberDto> {
    let s = core.project()?;
    let label = s.store.read(|c| crew_get(c, &args.id))?.person_name;
    let summary = if args.archived {
        format!("Archived {label}")
    } else {
        format!("Restored {label} from the archive")
    };
    s.store.mutate(
        actor,
        MutationMeta::new("crew.set_archived", summary, Capability::Edit)
            .target("crew_member", &args.id),
        |tx| {
            update_fields(
                tx.conn(),
                "crew_member",
                &args.id,
                &[("archived", int(args.archived as i64))],
                &["archived"],
                None,
                "crew member",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| crew_get(c, &args.id))
}

fn crew_delete(core: &AppCore, actor: &Actor, args: ProductionPeopleIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let label = s.store.read(|c| crew_get(c, &args.id))?.person_name;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "crew.delete",
            format!("Removed {label} from the crew"),
            Capability::SoftDelete,
        )
        .target("crew_member", &args.id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "crew_member",
                    table: "crew_member",
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

// ------------------------------------------------------- search / trash

#[allow(clippy::type_complexity)]
fn index_cast(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT person_name, character_id, character_name, contact, availability_notes, notes, deleted_at FROM cast_member WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?;
    let Some((person, char_id, char_name, contact, availability, notes, None)) = row else {
        return Ok(None);
    };
    let character = member_key(c, char_id.as_deref(), char_name.as_deref())?.unwrap_or_default();
    Ok(Some(SearchDoc {
        entity_type: "cast_member".into(),
        title: person,
        body: format!(
            "{character} {} {} {}",
            contact.unwrap_or_default(),
            availability.unwrap_or_default(),
            notes.unwrap_or_default()
        ),
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "cast-crew", "castMemberId": id }),
        owner_user_id: None,
    }))
}

#[allow(clippy::type_complexity)]
fn index_crew(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT person_name, role, department, contact, notes, deleted_at FROM crew_member WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let Some((person, role, department, contact, notes, None)) = row else {
        return Ok(None);
    };
    Ok(Some(SearchDoc {
        entity_type: "crew_member".into(),
        title: person,
        body: format!(
            "{role} {} {} {}",
            department.unwrap_or_default(),
            contact.unwrap_or_default(),
            notes.unwrap_or_default()
        ),
        context: "Production".into(),
        nav: json!({ "workspace": "production", "sub": "cast-crew", "crewMemberId": id }),
        owner_user_id: None,
    }))
}

fn purge_cast(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let photo: Option<String> = tx
        .conn()
        .query_row(
            "SELECT photo_asset_id FROM cast_member WHERE id = ?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    tx.conn()
        .execute("DELETE FROM cast_member WHERE id = ?1", [&row.object_id])?;
    if let Some(a) = photo {
        crate::modules::files::purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

fn purge_crew(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn()
        .execute("DELETE FROM crew_member WHERE id = ?1", [&row.object_id])?;
    Ok(())
}
