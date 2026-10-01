//! Screenplay workspace (FSD §15–17, §21–24, §92–95; Domain "Screenplay").
//!
//! Model: a project (or an episode) has one Screenplay with named Drafts; exactly
//! one draft is Current (`screenplay.current_draft_id`). A draft owns ordered
//! Scenes (`position` → derived display number, never stored) and each scene
//! owns ordered Elements. The scene heading lives on the scene row; body
//! elements never have the `scene_heading` type (a heading always starts a new
//! scene). New drafts copy scenes/elements with new ids and keep `lineage_id`,
//! the stable cross-draft scene identity used by comparison and production.
//!
//! The React editor sends element-level edit operations (debounced) through
//! `screenplay.apply_edits`; Rust validates and stays the source of truth. Undo
//! is the generic pipeline undo, grouped by coalesce keys while typing.
//!
//! Submodules: `edit` (content operations), `drafts` (drafts, lock, revisions,
//! automatic history), `compare` (draft comparison), `review` (review rounds),
//! `characters` (character-cue usage), `extras` (search, trash, scene hub,
//! story reference, title page, view state).

use std::collections::{HashMap, HashSet};

use openframe_domain::enums::{DraftStatus, ElementType, ProjectType};
use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;

pub mod characters;
pub mod compare;
pub mod drafts;
pub mod edit;
pub mod extras;
pub mod review;

pub use drafts::{NewElement, NewScene, create_screenplay_tx, insert_draft_tx};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Screenplay");
    r.query("screenplay.overview", overview).meta(M::read(
        "Screenplays and drafts with scene counts (per episode for series).",
    ));
    r.query("screenplay.document", document).meta(M::read(
        "A draft's full document: scenes and their elements.",
    ));
    r.query("screenplay.locate", locate).meta(
        M::read("Resolve a screenplay deep link (scene or draft) to its draft and episode.")
            .class(crate::registry::OpClass::Navigate),
    );
    edit::register(r);
    drafts::register(r);
    compare::register(r);
    review::register(r);
    characters::register(r);
    extras::register(r);
}

// ------------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenplayTitlePage {
    pub title: String,
    pub written_by: String,
    pub contact: String,
    pub draft_line: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDto {
    pub id: String,
    pub episode_id: Option<String>,
    pub title: String,
    /// Feature / Short / Episodic.
    pub format: String,
    pub current_draft_id: Option<String>,
    pub title_page: ScreenplayTitlePage,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDraftRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDraftDto {
    pub id: String,
    pub screenplay_id: String,
    pub name: String,
    pub note: Option<String>,
    pub status: DraftStatus,
    pub is_current: bool,
    pub created_from_draft_id: Option<String>,
    pub created_from_name: Option<String>,
    /// Ancestors from the oldest to this draft (FSD §21.4: "Draft 1 → Draft 2 → …").
    pub lineage: Vec<ScreenplayDraftRef>,
    #[ts(type = "number | null")]
    pub locked_at: Option<i64>,
    pub locked_by_name: Option<String>,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    pub revision_reason: Option<String>,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub open_comments: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    /// Last change to the draft or any of its content.
    #[ts(type = "number")]
    pub last_modified: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayElementDto {
    pub id: String,
    pub element_type: ElementType,
    pub text: String,
    pub dual: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneDto {
    pub id: String,
    pub lineage_id: String,
    /// Display number derived from order (FSD §16.3 — the writer never types it).
    pub number: u32,
    pub heading: String,
    pub notes: Option<String>,
    pub synopsis: Option<String>,
    pub story_day: Option<String>,
    pub time_note: Option<String>,
    pub source_scene_card_id: Option<String>,
    pub elements: Vec<ScreenplayElementDto>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDocument {
    pub screenplay: ScreenplayDto,
    pub draft: ScreenplayDraftDto,
    pub scenes: Vec<ScreenplaySceneDto>,
    /// Edit sequence of the draft: lets the editor ignore reads older than its own saves.
    #[ts(type = "number")]
    pub seq: i64,
    /// The actor may edit and the draft is not locked.
    pub can_edit: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayEpisodeRef {
    pub id: String,
    pub title: String,
    pub season_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayPermissions {
    pub edit: bool,
    pub comment: bool,
    pub resolve_comments: bool,
    pub lock: bool,
    pub unlock: bool,
    pub delete: bool,
}

impl ScreenplayPermissions {
    pub fn for_actor(actor: &Actor) -> Self {
        let r = actor.role;
        Self {
            edit: r.allows(Capability::Edit),
            comment: r.allows(Capability::Comment),
            resolve_comments: r.allows(Capability::ResolveComments),
            lock: r.allows(Capability::LockOrFinalize),
            unlock: r.allows(Capability::ManageProject),
            delete: r.allows(Capability::SoftDelete),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayOverview {
    pub project_type: String,
    /// Episodic/Series projects keep one screenplay per episode (FSD §25.3).
    pub episodic: bool,
    pub episodes: Vec<ScreenplayEpisodeRef>,
    pub episode_id: Option<String>,
    pub screenplay: Option<ScreenplayDto>,
    pub drafts: Vec<ScreenplayDraftDto>,
    pub permissions: ScreenplayPermissions,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayOverviewArgs {
    #[serde(default)]
    #[ts(optional)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayDraftIdArgs {
    pub draft_id: String,
}

/// A scene or draft id from another workspace (schedule strip, character,
/// breakdown…) to resolve into what the Screenplay workspace needs to show it.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayLocateArgs {
    #[serde(default)]
    #[ts(optional)]
    pub scene_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub draft_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayLocation {
    pub screenplay_id: String,
    pub draft_id: String,
    /// The episode whose screenplay this is (series).
    pub episode_id: Option<String>,
    /// The scene, when one was asked for and it still exists.
    pub scene_id: Option<String>,
}

// ---------------------------------------------------------------- rows

#[derive(Debug, Clone)]
pub(crate) struct DraftRow {
    pub id: String,
    pub screenplay_id: String,
    pub name: String,
    pub note: Option<String>,
    pub status: DraftStatus,
    pub created_from: Option<String>,
    pub locked_at: Option<i64>,
    pub locked_by_name: Option<String>,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    pub revision_reason: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub rev: i64,
}

const DRAFT_COLS: &str = "id, screenplay_id, name, note, status, created_from_draft_id, locked_at, locked_by_name,
                          revision_label, revision_color, revision_reason, created_at, updated_at, rev";

fn map_draft(r: &rusqlite::Row<'_>) -> rusqlite::Result<(DraftRow, String)> {
    let status: String = r.get(4)?;
    Ok((
        DraftRow {
            id: r.get(0)?,
            screenplay_id: r.get(1)?,
            name: r.get(2)?,
            note: r.get(3)?,
            status: DraftStatus::Draft,
            created_from: r.get(5)?,
            locked_at: r.get(6)?,
            locked_by_name: r.get(7)?,
            revision_label: r.get(8)?,
            revision_color: r.get(9)?,
            revision_reason: r.get(10)?,
            created_at: r.get(11)?,
            updated_at: r.get(12)?,
            rev: r.get(13)?,
        },
        status,
    ))
}

fn finish_draft((mut d, status): (DraftRow, String)) -> AppResult<DraftRow> {
    d.status = DraftStatus::parse_field(&status, "status")?;
    Ok(d)
}

/// A live (not deleted) draft.
pub(crate) fn load_draft(c: &Connection, id: &str) -> AppResult<DraftRow> {
    let row = c
        .query_row(
            &format!(
                "SELECT {DRAFT_COLS} FROM screenplay_draft WHERE id=?1 AND deleted_at IS NULL"
            ),
            [id],
            map_draft,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("draft"))?;
    finish_draft(row)
}

/// Live drafts of a screenplay, oldest first.
pub(crate) fn drafts_of(c: &Connection, screenplay_id: &str) -> AppResult<Vec<DraftRow>> {
    let mut stmt = c.prepare(&format!(
        "SELECT {DRAFT_COLS} FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL ORDER BY created_at, id"
    ))?;
    let rows = stmt
        .query_map([screenplay_id], map_draft)?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(finish_draft).collect()
}

/// A draft that may receive content edits (FSD §24.3: locked drafts only via "Start Revision").
pub(crate) fn editable_draft(c: &Connection, id: &str) -> AppResult<DraftRow> {
    let d = load_draft(c, id)?;
    if d.status == DraftStatus::Locked {
        return Err(AppError::locked_draft());
    }
    Ok(d)
}

pub(crate) fn load_screenplay(c: &Connection, id: &str) -> AppResult<ScreenplayDto> {
    c.query_row(
        "SELECT id, episode_id, title, format, current_draft_id, title_page_json, rev FROM screenplay WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| {
            let tp: String = r.get(5)?;
            Ok(ScreenplayDto {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                title: r.get(2)?,
                format: r.get(3)?,
                current_draft_id: r.get(4)?,
                title_page: serde_json::from_str(&tp).unwrap_or_default(),
                rev: r.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("screenplay"))
}

/// Navigation target for a screenplay scene (search hits, comments, Continue).
/// The Screenplay workspace needs the draft to open it and, in a series, the
/// episode to choose the screenplay; without them the scene cannot be shown.
pub(crate) fn scene_nav(c: &Connection, scene_id: &str) -> AppResult<serde_json::Value> {
    let row: Option<(String, Option<String>)> = c
        .query_row(
            "SELECT s.draft_id, sp.episode_id FROM screenplay_scene s
               JOIN screenplay_draft d ON d.id = s.draft_id JOIN screenplay sp ON sp.id = d.screenplay_id
              WHERE s.id=?1",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let mut v = serde_json::json!({ "workspace": "screenplay", "sceneId": scene_id });
    if let Some((draft, episode)) = row {
        v["draftId"] = serde_json::json!(draft);
        if let Some(e) = episode {
            v["episodeId"] = serde_json::json!(e);
        }
    }
    Ok(v)
}

/// Navigation target for a screenplay draft (opens it in its episode).
pub(crate) fn draft_nav(c: &Connection, draft_id: &str) -> AppResult<serde_json::Value> {
    let episode: Option<Option<String>> = c
        .query_row(
            "SELECT sp.episode_id FROM screenplay_draft d JOIN screenplay sp ON sp.id = d.screenplay_id WHERE d.id=?1",
            [draft_id],
            |r| r.get(0),
        )
        .optional()?;
    let mut v = serde_json::json!({ "workspace": "screenplay", "draftId": draft_id });
    if let Some(Some(e)) = episode {
        v["episodeId"] = serde_json::json!(e);
    }
    Ok(v)
}

/// The screenplay for a scope (project-level when `episode_id` is None).
pub(crate) fn screenplay_for_scope(
    c: &Connection,
    episode_id: Option<&str>,
) -> AppResult<Option<String>> {
    Ok(c
        .query_row(
            "SELECT id FROM screenplay WHERE episode_id IS ?1 AND deleted_at IS NULL ORDER BY created_at LIMIT 1",
            [episode_id],
            |r| r.get(0),
        )
        .optional()?)
}

pub(crate) fn project_type(c: &Connection) -> AppResult<ProjectType> {
    let t: String = c.query_row("SELECT project_type FROM project LIMIT 1", [], |r| r.get(0))?;
    ProjectType::parse_field(&t, "project type")
}

/// Ordered live scene ids of a draft.
pub(crate) fn live_scene_ids(c: &Connection, draft_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare("SELECT id FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id")?;
    let ids = stmt
        .query_map([draft_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(ids)
}

/// Advance and return the draft's edit sequence (infrastructure, not undo-tracked).
pub(crate) fn bump_seq(c: &Connection, draft_id: &str) -> AppResult<i64> {
    c.execute(
        "INSERT INTO sys_screenplay_sync(draft_id, seq) VALUES (?1, 1) ON CONFLICT(draft_id) DO UPDATE SET seq = seq + 1",
        [draft_id],
    )?;
    current_seq(c, draft_id)
}

pub(crate) fn current_seq(c: &Connection, draft_id: &str) -> AppResult<i64> {
    Ok(c.query_row(
        "SELECT seq FROM sys_screenplay_sync WHERE draft_id=?1",
        [draft_id],
        |r| r.get(0),
    )
    .optional()?
    .unwrap_or(0))
}

/// Full ordered content of a draft.
pub(crate) fn load_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<ScreenplaySceneDto>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, notes, synopsis, story_day, time_note, source_scene_card_id, rev
         FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let mut scenes: Vec<ScreenplaySceneDto> = stmt
        .query_map([draft_id], |r| {
            Ok(ScreenplaySceneDto {
                id: r.get(0)?,
                lineage_id: r.get(1)?,
                number: 0,
                heading: r.get(2)?,
                notes: r.get(3)?,
                synopsis: r.get(4)?,
                story_day: r.get(5)?,
                time_note: r.get(6)?,
                source_scene_card_id: r.get(7)?,
                elements: vec![],
                rev: r.get(8)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let index: HashMap<String, usize> = scenes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    let mut el = c.prepare(
        "SELECT e.id, e.scene_id, e.element_type, e.text, e.dual FROM screenplay_element e
         JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL ORDER BY e.position, e.id",
    )?;
    let rows = el.query_map([draft_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, bool>(4)?,
        ))
    })?;
    for row in rows {
        let (id, scene_id, t, text, dual) = row?;
        let element_type = ElementType::parse_field(&t, "element type")?;
        if let Some(&i) = index.get(&scene_id) {
            scenes[i].elements.push(ScreenplayElementDto {
                id,
                element_type,
                text,
                dual,
            });
        }
    }
    for (i, s) in scenes.iter_mut().enumerate() {
        s.number = (i + 1) as u32;
    }
    Ok(scenes)
}

/// Last change to a draft or its content.
fn last_modified(c: &Connection, d: &DraftRow) -> AppResult<i64> {
    let content: Option<i64> = c.query_row(
        "SELECT max(m) FROM (
            SELECT max(updated_at) AS m FROM screenplay_scene WHERE draft_id=?1
            UNION ALL
            SELECT max(e.updated_at) FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id WHERE s.draft_id=?1)",
        [&d.id],
        |r| r.get(0),
    )?;
    Ok(content.unwrap_or(0).max(d.updated_at))
}

/// Build draft DTOs with lineage for all given drafts of one screenplay.
pub(crate) fn draft_dtos(
    c: &Connection,
    rows: &[DraftRow],
    current: Option<&str>,
) -> AppResult<Vec<ScreenplayDraftDto>> {
    // Names of all drafts (including deleted ones) for lineage display.
    let mut names: HashMap<String, (String, Option<String>)> = HashMap::new();
    if let Some(first) = rows.first() {
        let mut stmt = c.prepare(
            "SELECT id, name, created_from_draft_id FROM screenplay_draft WHERE screenplay_id=?1",
        )?;
        for r in stmt.query_map([&first.screenplay_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?))
        })? {
            let (id, name, from) = r?;
            names.insert(id, (name, from));
        }
    }
    rows.iter()
        .map(|d| {
            let mut lineage = vec![ScreenplayDraftRef {
                id: d.id.clone(),
                name: d.name.clone(),
            }];
            let mut seen: HashSet<String> = HashSet::from([d.id.clone()]);
            let mut cursor = d.created_from.clone();
            while let Some(pid) = cursor {
                if !seen.insert(pid.clone()) {
                    break;
                }
                match names.get(&pid) {
                    Some((name, from)) => {
                        lineage.push(ScreenplayDraftRef {
                            id: pid.clone(),
                            name: name.clone(),
                        });
                        cursor = from.clone();
                    }
                    None => break,
                }
            }
            lineage.reverse();
            let scene_count: i64 = c.query_row(
                "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL",
                [&d.id],
                |r| r.get(0),
            )?;
            Ok(ScreenplayDraftDto {
                id: d.id.clone(),
                screenplay_id: d.screenplay_id.clone(),
                name: d.name.clone(),
                note: d.note.clone(),
                status: d.status,
                is_current: current == Some(d.id.as_str()),
                created_from_draft_id: d.created_from.clone(),
                created_from_name: d
                    .created_from
                    .as_ref()
                    .and_then(|p| names.get(p))
                    .map(|(n, _)| n.clone()),
                lineage,
                locked_at: d.locked_at,
                locked_by_name: d.locked_by_name.clone(),
                revision_label: d.revision_label.clone(),
                revision_color: d.revision_color.clone(),
                revision_reason: d.revision_reason.clone(),
                scene_count,
                open_comments: crate::modules::comments::open_count_for_draft(c, &d.id)?,
                created_at: d.created_at,
                last_modified: last_modified(c, d)?,
                rev: d.rev,
            })
        })
        .collect()
}

pub(crate) fn draft_dto(c: &Connection, d: &DraftRow) -> AppResult<ScreenplayDraftDto> {
    let current: Option<String> = c.query_row(
        "SELECT current_draft_id FROM screenplay WHERE id=?1",
        [&d.screenplay_id],
        |r| r.get(0),
    )?;
    Ok(draft_dtos(c, std::slice::from_ref(d), current.as_deref())?.remove(0))
}

// ---------------------------------------------------------------- queries

fn overview(
    core: &AppCore,
    actor: &Actor,
    args: ScreenplayOverviewArgs,
) -> AppResult<ScreenplayOverview> {
    actor.require(Capability::View, "view the screenplay")?;
    let s = core.project()?;
    s.store.read(|c| {
        let pt = project_type(c)?;
        let episodic = pt.is_episodic();
        let mut stmt = c.prepare(
            "SELECT e.id, e.title, se.title FROM episode e LEFT JOIN season se ON se.id = e.season_id
             WHERE e.deleted_at IS NULL ORDER BY se.position, e.position, e.id",
        )?;
        let episodes: Vec<ScreenplayEpisodeRef> = stmt
            .query_map([], |r| Ok(ScreenplayEpisodeRef { id: r.get(0)?, title: r.get(1)?, season_title: r.get(2)? }))?
            .collect::<Result<_, _>>()?;
        let episode_id = if episodic {
            match args.episode_id {
                Some(e) if episodes.iter().any(|x| x.id == e) => Some(e),
                Some(_) => return Err(AppError::not_found("episode")),
                None => None,
            }
        } else {
            None
        };
        let screenplay = if episodic && episode_id.is_none() {
            None
        } else {
            match screenplay_for_scope(c, episode_id.as_deref())? {
                Some(id) => Some(load_screenplay(c, &id)?),
                None => None,
            }
        };
        let drafts = match &screenplay {
            Some(sp) => draft_dtos(c, &drafts_of(c, &sp.id)?, sp.current_draft_id.as_deref())?,
            None => vec![],
        };
        Ok(ScreenplayOverview {
            project_type: pt.as_str().to_string(),
            episodic,
            episodes,
            episode_id,
            screenplay,
            drafts,
            permissions: ScreenplayPermissions::for_actor(actor),
        })
    })
}

fn locate(
    core: &AppCore,
    actor: &Actor,
    args: ScreenplayLocateArgs,
) -> AppResult<ScreenplayLocation> {
    actor.require(Capability::View, "view the screenplay")?;
    let s = core.project()?;
    s.store.read(|c| {
        let (draft_id, scene_id) = match (&args.scene_id, &args.draft_id) {
            (Some(scene), _) => {
                let draft: Option<String> = c
                    .query_row(
                        "SELECT draft_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
                        [scene],
                        |r| r.get(0),
                    )
                    .optional()?;
                match draft {
                    Some(d) => (d, Some(scene.clone())),
                    None => return Err(AppError::not_found("scene")),
                }
            }
            (None, Some(d)) => (d.clone(), None),
            (None, None) => return Err(AppError::required("A scene or draft")),
        };
        let row: Option<(String, Option<String>)> = c
            .query_row(
                "SELECT sp.id, sp.episode_id FROM screenplay_draft d JOIN screenplay sp ON sp.id = d.screenplay_id
                  WHERE d.id=?1 AND d.deleted_at IS NULL AND sp.deleted_at IS NULL",
                [&draft_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (screenplay_id, episode_id) = row.ok_or_else(|| AppError::not_found("draft"))?;
        Ok(ScreenplayLocation {
            screenplay_id,
            draft_id,
            episode_id,
            scene_id,
        })
    })
}

fn document(
    core: &AppCore,
    actor: &Actor,
    args: ScreenplayDraftIdArgs,
) -> AppResult<ScreenplayDocument> {
    actor.require(Capability::View, "view the screenplay")?;
    let s = core.project()?;
    s.store.read(|c| {
        let d = load_draft(c, &args.draft_id)?;
        let screenplay = load_screenplay(c, &d.screenplay_id)?;
        let draft = draft_dto(c, &d)?;
        Ok(ScreenplayDocument {
            can_edit: actor.role.allows(Capability::Edit) && d.status != DraftStatus::Locked,
            scenes: load_scenes(c, &d.id)?,
            seq: current_seq(c, &d.id)?,
            screenplay,
            draft,
        })
    })
}

/// Display number of a live scene (derived from order).
pub(crate) fn scene_number(c: &Connection, scene_id: &str) -> AppResult<Option<u32>> {
    let row: Option<(String, i64, bool)> = c
        .query_row(
            "SELECT draft_id, position, deleted_at IS NOT NULL FROM screenplay_scene WHERE id=?1",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((draft, pos, deleted)) = row else {
        return Ok(None);
    };
    if deleted {
        return Ok(None);
    }
    let before: i64 = c.query_row(
        "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL
           AND (position < ?2 OR (position = ?2 AND id < ?3))",
        params![draft, pos, scene_id],
        |r| r.get(0),
    )?;
    Ok(Some(before as u32 + 1))
}
