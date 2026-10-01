//! Characters: a lightweight story directory (FSD §13, UX §3.9) with typed
//! relationships and optional manual links to Scene Cards. Scene appearances
//! are derived from screenplay character cues — never stored here — and
//! renaming a character never rewrites screenplay text (FSD §13.5).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, opt_text, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::build::scope_current_draft;
use super::tree::{self, short};
use super::{NOTES_MAX_BYTES, StoryCreated, StoryIdArgs, ensure_live};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{DeleteSpec, MutationMeta, soft_delete};
use crate::util::{
    AssetInfo, body_text, ingest_file, load_asset_opt, optional_text, required_text,
};

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.query("story.characters", list)
        .meta(M::read("Character directory (names, roles, descriptions)."));
    r.query("story.character", detail).meta(M::read(
        "One character with description, notes, relationships and linked Scene Cards.",
    ));
    r.query("story.character_usage", usage).meta(M::compute(
        "Where a character is used (Scene Cards, screenplay cues, cast).",
    ));
    r.query("story.relationships", relationships)
        .meta(M::read("Character relationships."));
    r.command("story.create_character", create).meta(
        M::edit("Create a character (optionally with an image the user picked).")
            .fs(Fs::ReadsUserFile),
    );
    r.command("story.update_character", update).meta(M::edit(
        "Edit a character's name, role, description or notes.",
    ));
    r.command("story.set_character_archived", set_archived)
        .meta(M::edit("Archive or unarchive a character."));
    r.command("story.set_character_image", set_image).meta(
        M::edit("Set a character's image from a file the user picked.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::MEDIA_INPUT),
    );
    r.command("story.clear_character_image", clear_image)
        .meta(M::edit("Remove a character's image."));
    r.command("story.delete_character", delete)
        .meta(M::soft_delete("Move a character to Recently Deleted.").confirm());
    r.command("story.create_relationship", create_relationship)
        .meta(M::edit("Create a relationship between two characters."));
    r.command("story.update_relationship", update_relationship)
        .meta(M::edit("Edit a character relationship."));
    r.command("story.delete_relationship", delete_relationship)
        .meta(M::edit("Remove a character relationship.").destructive());
    r.command("story.link_character_card", link_card)
        .meta(M::edit("Link a character to a Scene Card."));
    r.command("story.unlink_character_card", unlink_card)
        .meta(M::edit("Unlink a character from a Scene Card."));
}

const NAME_MAX: usize = 120;
const ROLE_MAX: usize = 120;
const DESC_MAX: usize = 2000;
const REL_MAX: usize = 80;

// ===================================================================== DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacterDto {
    pub id: String,
    pub name: String,
    pub role_label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub archived: bool,
    /// None = series/project-level character.
    pub episode_id: Option<String>,
    pub image: Option<AssetInfo>,
    /// Screenplay scenes (current draft) whose character cues match this name.
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacterScene {
    pub scene_id: String,
    /// Derived from screenplay order.
    #[ts(type = "number")]
    pub number: i64,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryRelationshipDto {
    pub id: String,
    pub from_character_id: String,
    pub from_name: String,
    pub to_character_id: String,
    pub to_name: String,
    /// Freeform relation, e.g. "father of".
    pub relationship_type: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryLinkedCard {
    pub card_id: String,
    pub short_description: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacterDetail {
    pub character: StoryCharacterDto,
    pub scenes: Vec<StoryCharacterScene>,
    pub relationships: Vec<StoryRelationshipDto>,
    pub linked_cards: Vec<StoryLinkedCard>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCharacterUsage {
    #[ts(type = "number")]
    pub screenplay_scenes: i64,
    #[ts(type = "number")]
    pub cast_links: i64,
    #[ts(type = "number")]
    pub catalog_links: i64,
    #[ts(type = "number")]
    pub card_links: i64,
    #[ts(type = "number")]
    pub relationships: i64,
    /// True when deleting should offer "Archive" instead (FSD §13.6).
    pub in_use: bool,
}

// ===================================================================== args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCharacterListArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCharacterArgs {
    pub id: String,
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateCharacterArgs {
    pub name: String,
    #[serde(default)]
    pub role_label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    /// Only for episode-specific characters; None = series/project level.
    #[serde(default)]
    pub episode_id: Option<String>,
    /// Optional image file chosen in the dialog.
    #[serde(default)]
    pub image_path: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateCharacterArgs {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Empty string clears optional fields.
    #[serde(default)]
    pub role_label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryArchiveArgs {
    pub id: String,
    pub archived: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryImageArgs {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateRelationshipArgs {
    pub from_character_id: String,
    pub to_character_id: String,
    pub relationship_type: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateRelationshipArgs {
    pub id: String,
    #[serde(default)]
    pub relationship_type: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCardLinkArgs {
    pub character_id: String,
    pub card_id: String,
}

// ================================================================ scene cues

/// Normalise a screenplay character cue: `RAVI (V.O.)` → `RAVI`, `MAYA (CONT'D)` → `MAYA`.
pub fn normalize_cue(cue: &str) -> String {
    let base = cue.split('(').next().unwrap_or("");
    let base = base.trim().trim_end_matches('^').trim();
    let upper = base.to_uppercase();
    let upper = upper
        .trim_end_matches("CONT'D")
        .trim_end_matches("CONT’D")
        .trim();
    upper.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_name(name: &str) -> String {
    name.trim()
        .to_uppercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Map normalised cue → scenes (in screenplay order) of the scope's current draft.
fn cue_scenes(
    c: &Connection,
    episode: Option<&str>,
) -> AppResult<BTreeMap<String, Vec<StoryCharacterScene>>> {
    let mut out: BTreeMap<String, Vec<StoryCharacterScene>> = BTreeMap::new();
    let Some(draft) = scope_current_draft(c, episode)? else {
        return Ok(out);
    };
    let scenes: Vec<(String, String)> = {
        let mut stmt = c.prepare(
            "SELECT id, heading FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        stmt.query_map([&draft], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    let number: HashMap<&str, (i64, &str)> = scenes
        .iter()
        .enumerate()
        .map(|(i, (id, h))| (id.as_str(), (i as i64 + 1, h.as_str())))
        .collect();
    let cues: Vec<(String, String)> = {
        let mut stmt = c.prepare(
            "SELECT e.scene_id, e.text FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
             WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type='character'",
        )?;
        stmt.query_map([&draft], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for (scene, cue) in cues {
        let key = normalize_cue(&cue);
        if key.is_empty() || !seen.insert((key.clone(), scene.clone())) {
            continue;
        }
        if let Some((n, h)) = number.get(scene.as_str()) {
            out.entry(key).or_default().push(StoryCharacterScene {
                scene_id: scene.clone(),
                number: *n,
                heading: h.to_string(),
            });
        }
    }
    for v in out.values_mut() {
        v.sort_by_key(|s| s.number);
    }
    Ok(out)
}

// ================================================================== queries

type CharRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    Option<String>,
    i64,
);

const CHAR_COLS: &str =
    "id, name, role_label, description, notes, archived, episode_id, image_asset_id, rev";

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<CharRow> {
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
    ))
}

fn to_dto(
    c: &Connection,
    root: &Path,
    row: CharRow,
    cues: &BTreeMap<String, Vec<StoryCharacterScene>>,
) -> AppResult<StoryCharacterDto> {
    let (id, name, role_label, description, notes, archived, episode_id, image, rev) = row;
    let scene_count = cues
        .get(&normalize_name(&name))
        .map(|v| v.len() as i64)
        .unwrap_or(0);
    Ok(StoryCharacterDto {
        image: load_asset_opt(c, root, image.as_deref())?,
        id,
        name,
        role_label,
        description,
        notes,
        archived,
        episode_id,
        scene_count,
        rev,
    })
}

fn list(
    core: &AppCore,
    actor: &Actor,
    a: StoryCharacterListArgs,
) -> AppResult<Vec<StoryCharacterDto>> {
    actor.require(Capability::View, "view characters")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let ep = a.episode_id.as_deref();
        let cues = cue_scenes(c, ep)?;
        let rows: Vec<CharRow> = {
            let mut stmt = c.prepare(&format!(
                "SELECT {CHAR_COLS} FROM story_character
                 WHERE deleted_at IS NULL AND (episode_id IS NULL OR episode_id IS ?1) AND (?2 OR archived = 0)
                 ORDER BY archived, position, name COLLATE NOCASE"
            ))?;
            stmt.query_map(params![ep, a.include_archived], map_row)?.collect::<Result<_, _>>()?
        };
        rows.into_iter().map(|r| to_dto(c, &root, r, &cues)).collect()
    })
}

fn load_relationships(
    c: &Connection,
    character: Option<&str>,
) -> AppResult<Vec<StoryRelationshipDto>> {
    let mut stmt = c.prepare(
        "SELECT r.id, r.from_character_id, f.name, r.to_character_id, t.name, r.relationship_type, r.note, r.rev
         FROM story_character_relationship r
         JOIN story_character f ON f.id = r.from_character_id
         JOIN story_character t ON t.id = r.to_character_id
         WHERE f.deleted_at IS NULL AND t.deleted_at IS NULL
           AND (?1 IS NULL OR r.from_character_id = ?1 OR r.to_character_id = ?1)
         ORDER BY r.position, r.id",
    )?;
    let rows = stmt
        .query_map([character], |r| {
            Ok(StoryRelationshipDto {
                id: r.get(0)?,
                from_character_id: r.get(1)?,
                from_name: r.get(2)?,
                to_character_id: r.get(3)?,
                to_name: r.get(4)?,
                relationship_type: r.get(5)?,
                note: r.get(6)?,
                rev: r.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn detail(core: &AppCore, actor: &Actor, a: StoryCharacterArgs) -> AppResult<StoryCharacterDetail> {
    actor.require(Capability::View, "view characters")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let cues = cue_scenes(c, a.episode_id.as_deref())?;
        let row = c
            .query_row(&format!("SELECT {CHAR_COLS} FROM story_character WHERE id=?1 AND deleted_at IS NULL"), [&a.id], map_row)
            .optional()?
            .ok_or_else(|| AppError::not_found("character"))?;
        let scenes = cues.get(&normalize_name(&row.1)).cloned().unwrap_or_default();
        let character = to_dto(c, &root, row, &cues)?;
        let linked_cards = {
            let mut stmt = c.prepare(
                "SELECT k.id, k.short_description, k.parent_type, k.parent_id FROM story_character_card_link l
                 JOIN story_scene_card k ON k.id = l.scene_card_id
                 WHERE l.character_id=?1 AND k.deleted_at IS NULL ORDER BY k.created_at",
            )?;
            let rows: Vec<(String, String, String, Option<String>)> = stmt
                .query_map([&a.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
                .collect::<Result<_, _>>()?;
            rows.into_iter()
                .map(|(id, d, pt, pid)| StoryLinkedCard {
                    card_id: id,
                    short_description: d,
                    location: super::board::location_label(
                        c,
                        tree::StoryContainerType::parse(&pt).unwrap_or(tree::StoryContainerType::Unassigned),
                        pid.as_deref(),
                    ),
                })
                .collect()
        };
        Ok(StoryCharacterDetail { character, scenes, relationships: load_relationships(c, Some(&a.id))?, linked_cards })
    })
}

fn compute_usage(c: &Connection, id: &str) -> AppResult<StoryCharacterUsage> {
    let name: String = c
        .query_row("SELECT name FROM story_character WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?
        .ok_or_else(|| AppError::not_found("character"))?;
    let count = |sql: &str| -> AppResult<i64> { Ok(c.query_row(sql, [id], |r| r.get(0))?) };
    let cast_links =
        count("SELECT count(*) FROM cast_member WHERE character_id=?1 AND deleted_at IS NULL")?;
    let catalog_links =
        count("SELECT count(*) FROM catalog_item WHERE character_id=?1 AND deleted_at IS NULL")?;
    let card_links = count(
        "SELECT count(*) FROM story_character_card_link l JOIN story_scene_card k ON k.id=l.scene_card_id
         WHERE l.character_id=?1 AND k.deleted_at IS NULL",
    )?;
    let relationships = count(
        "SELECT count(*) FROM story_character_relationship WHERE from_character_id=?1 OR to_character_id=?1",
    )?;
    // Appearances in any live screenplay draft (not only the current one).
    let key = normalize_name(&name);
    let mut screenplay_scenes = 0;
    {
        let mut stmt = c.prepare(
            "SELECT DISTINCT e.scene_id, e.text FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id
             JOIN screenplay_draft d ON d.id=s.draft_id
             WHERE e.element_type='character' AND s.deleted_at IS NULL AND d.deleted_at IS NULL",
        )?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut scenes = BTreeSet::new();
        for (scene, cue) in rows {
            if normalize_cue(&cue) == key {
                scenes.insert(scene);
            }
        }
        screenplay_scenes += scenes.len() as i64;
    }
    Ok(StoryCharacterUsage {
        in_use: screenplay_scenes > 0 || cast_links > 0 || catalog_links > 0,
        screenplay_scenes,
        cast_links,
        catalog_links,
        card_links,
        relationships,
    })
}

fn usage(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<StoryCharacterUsage> {
    actor.require(Capability::View, "view characters")?;
    core.project()?.store.read(|c| compute_usage(c, &a.id))
}

fn relationships(
    core: &AppCore,
    actor: &Actor,
    _a: StoryCharacterListArgs,
) -> AppResult<Vec<StoryRelationshipDto>> {
    actor.require(Capability::View, "view characters")?;
    core.project()?.store.read(|c| load_relationships(c, None))
}

// ================================================================= commands

fn opt_body(v: Option<String>, what: &str) -> AppResult<Option<String>> {
    match v {
        Some(n) if n.trim().is_empty() => Ok(None),
        Some(n) => Ok(Some(body_text(n, what, NOTES_MAX_BYTES)?)),
        None => Ok(None),
    }
}

fn create(core: &AppCore, actor: &Actor, a: StoryCreateCharacterArgs) -> AppResult<StoryCreated> {
    let name = required_text(&a.name, "Name", NAME_MAX)?;
    let role = optional_text(a.role_label, "Role", ROLE_MAX)?;
    let desc = optional_text(a.description, "Short description", DESC_MAX)?;
    let notes = opt_body(a.notes, "Notes")?;
    let s = core.project()?;
    s.store.mutate(actor, MutationMeta::new("story.create_character", format!("Added character “{}”", short(&name)), Capability::Edit), |tx| {
        let c = tx.conn();
        tree::check_episode(c, a.episode_id.as_deref())?;
        let image = match &a.image_path {
            Some(p) if !p.trim().is_empty() => Some(image_asset(tx, p)?),
            _ => None,
        };
        let id = new_id();
        let now = now_ms();
        let pos: i64 = c.query_row("SELECT COALESCE(MAX(position),0)+1 FROM story_character", [], |r| r.get(0))?;
        c.execute(
            "INSERT INTO story_character(id, name, role_label, description, notes, image_asset_id, episode_id, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![id, name, role, desc, notes, image, a.episode_id, pos, now],
        )?;
        Ok(StoryCreated { id })
    })
}

fn image_asset(tx: &crate::store::Tx<'_>, path: &str) -> AppResult<String> {
    let p = PathBuf::from(path);
    let media = crate::util::media_type_for(p.file_name().and_then(|n| n.to_str()).unwrap_or(""));
    if !media.starts_with("image/") {
        return Err(AppError::invalid_input(
            "Choose an image file (PNG, JPEG, GIF, WebP or BMP).",
        ));
    }
    Ok(ingest_file(tx, &p)?.id)
}

fn update(core: &AppCore, actor: &Actor, a: StoryUpdateCharacterArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut summary = "Edited character".to_string();
    if let Some(n) = &a.name {
        let n = required_text(n, "Name", NAME_MAX)?;
        summary = format!("Renamed character to “{}”", short(&n));
        fields.push(("name", text(n)));
    }
    if let Some(v) = &a.role_label {
        fields.push((
            "role_label",
            opt_text(optional_text(Some(v.clone()), "Role", ROLE_MAX)?),
        ));
    }
    if let Some(v) = &a.description {
        fields.push((
            "description",
            opt_text(optional_text(
                Some(v.clone()),
                "Short description",
                DESC_MAX,
            )?),
        ));
    }
    if let Some(v) = &a.notes {
        fields.push(("notes", opt_text(opt_body(Some(v.clone()), "Notes")?)));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.update_character", summary, Capability::Edit)
            .target("story_character", &a.id)
            .coalesce(format!("story.character:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "story_character", &a.id, "character")?;
            update_fields(
                tx.conn(),
                "story_character",
                &a.id,
                &fields,
                &["name", "role_label", "description", "notes"],
                a.expected_rev,
                "character",
            )?;
            Ok(())
        },
    )
}

fn char_name(core: &AppCore, id: &str) -> AppResult<String> {
    core.project()?.store.read(|c| {
        c.query_row(
            "SELECT name FROM story_character WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("character"))
    })
}

fn set_archived(core: &AppCore, actor: &Actor, a: StoryArchiveArgs) -> AppResult<()> {
    let name = char_name(core, &a.id)?;
    let verb = if a.archived { "Archived" } else { "Unarchived" };
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.set_character_archived",
            format!("{verb} character “{}”", short(&name)),
            Capability::Edit,
        )
        .target("story_character", &a.id),
        |tx| {
            update_fields(
                tx.conn(),
                "story_character",
                &a.id,
                &[("archived", int(a.archived as i64))],
                &["archived"],
                None,
                "character",
            )?;
            Ok(())
        },
    )
}

/// Replacing/clearing an image keeps the old file so Undo can bring it back;
/// unreferenced files are removed when the character is permanently deleted.
fn set_image(core: &AppCore, actor: &Actor, a: StoryImageArgs) -> AppResult<()> {
    let name = char_name(core, &a.id)?;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.set_character_image",
            format!("Changed image of “{}”", short(&name)),
            Capability::Edit,
        )
        .target("story_character", &a.id),
        |tx| {
            let asset = image_asset(tx, &a.path)?;
            update_fields(
                tx.conn(),
                "story_character",
                &a.id,
                &[("image_asset_id", text(asset))],
                &["image_asset_id"],
                None,
                "character",
            )?;
            Ok(())
        },
    )
}

fn clear_image(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    let name = char_name(core, &a.id)?;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.clear_character_image",
            format!("Removed image of “{}”", short(&name)),
            Capability::Edit,
        )
        .target("story_character", &a.id),
        |tx| {
            update_fields(
                tx.conn(),
                "story_character",
                &a.id,
                &[("image_asset_id", SqlValue::Null)],
                &["image_asset_id"],
                None,
                "character",
            )?;
            Ok(())
        },
    )
}

/// Delete moves the character to Recently Deleted. The UI checks
/// `story.character_usage` first and offers Archive when it is in use.
fn delete(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    let name = char_name(core, &a.id)?;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.delete_character",
            format!("Deleted character “{}”", short(&name)),
            Capability::SoftDelete,
        )
        .target("story_character", &a.id),
        |tx| {
            let pos: i64 = tx.conn().query_row(
                "SELECT position FROM story_character WHERE id=?1",
                [&a.id],
                |r| r.get(0),
            )?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "story_character",
                    table: "story_character",
                    id: &a.id,
                    title: Some(name.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: Some(pos),
                },
            )
        },
    )
}

fn create_relationship(
    core: &AppCore,
    actor: &Actor,
    a: StoryCreateRelationshipArgs,
) -> AppResult<StoryCreated> {
    let kind = required_text(&a.relationship_type, "Relationship", REL_MAX)?;
    let note = optional_text(a.note, "Note", DESC_MAX)?;
    if a.from_character_id == a.to_character_id {
        return Err(AppError::invalid_input("Choose two different characters."));
    }
    let s = core.project()?;
    let (from, to) = s.store.read(|c| {
        let name = |id: &str| -> AppResult<String> {
            c.query_row(
                "SELECT name FROM story_character WHERE id=?1 AND deleted_at IS NULL",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("character"))
        };
        Ok((name(&a.from_character_id)?, name(&a.to_character_id)?))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.create_relationship",
            format!("Added relationship {} — {} — {}", short(&from), kind, short(&to)),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let id = new_id();
            let now = now_ms();
            let pos: i64 =
                c.query_row("SELECT COALESCE(MAX(position),0)+1 FROM story_character_relationship", [], |r| r.get(0))?;
            c.execute(
                "INSERT INTO story_character_relationship(id, from_character_id, to_character_id, relationship_type, note, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![id, a.from_character_id, a.to_character_id, kind, note, pos, now],
            )?;
            Ok(StoryCreated { id })
        },
    )
}

fn update_relationship(
    core: &AppCore,
    actor: &Actor,
    a: StoryUpdateRelationshipArgs,
) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(k) = &a.relationship_type {
        fields.push((
            "relationship_type",
            text(required_text(k, "Relationship", REL_MAX)?),
        ));
    }
    if let Some(n) = &a.note {
        fields.push((
            "note",
            opt_text(optional_text(Some(n.clone()), "Note", DESC_MAX)?),
        ));
    }
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.update_relationship",
            "Edited relationship",
            Capability::Edit,
        )
        .coalesce(format!("story.relationship:{}", a.id)),
        |tx| {
            update_fields(
                tx.conn(),
                "story_character_relationship",
                &a.id,
                &fields,
                &["relationship_type", "note"],
                None,
                "relationship",
            )?;
            Ok(())
        },
    )
}

fn delete_relationship(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.delete_relationship",
            "Removed relationship",
            Capability::Edit,
        ),
        |tx| {
            let n = tx.conn().execute(
                "DELETE FROM story_character_relationship WHERE id=?1",
                [&a.id],
            )?;
            if n == 0 {
                return Err(AppError::not_found("relationship"));
            }
            Ok(())
        },
    )
}

fn link_card(core: &AppCore, actor: &Actor, a: StoryCardLinkArgs) -> AppResult<()> {
    let name = char_name(core, &a.character_id)?;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.link_character_card", format!("Linked “{}” to a Scene Card", short(&name)), Capability::Edit)
            .target("story_scene_card", &a.card_id),
        |tx| {
            let c = tx.conn();
            ensure_live(c, "story_scene_card", &a.card_id, "scene card")?;
            let now = now_ms();
            c.execute(
                "INSERT INTO story_character_card_link(id, character_id, scene_card_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4) ON CONFLICT(character_id, scene_card_id) DO NOTHING",
                params![new_id(), a.character_id, a.card_id, now],
            )?;
            Ok(())
        },
    )
}

fn unlink_card(core: &AppCore, actor: &Actor, a: StoryCardLinkArgs) -> AppResult<()> {
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.unlink_character_card",
            "Unlinked character from a Scene Card",
            Capability::Edit,
        )
        .target("story_scene_card", &a.card_id),
        |tx| {
            tx.conn().execute(
                "DELETE FROM story_character_card_link WHERE character_id=?1 AND scene_card_id=?2",
                params![a.character_id, a.card_id],
            )?;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::normalize_cue;

    #[test]
    fn cues() {
        assert_eq!(normalize_cue("RAVI (V.O.)"), "RAVI");
        assert_eq!(normalize_cue("  maya  (cont'd)"), "MAYA");
        assert_eq!(normalize_cue("MAYA CONT'D"), "MAYA");
        assert_eq!(normalize_cue("DR.  MEERA ^"), "DR. MEERA");
    }
}
