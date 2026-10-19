//! Screenplay workspace (FSD §15–17, §21–24, §92–95; Domain "Screenplay").
//!
//! Model: a project (or an episode) holds one or more Screenplays (ADR-0012,
//! amendment of 2026-10-08). Each keeps its own named Drafts, locked drafts,
//! production numbers and page setup, and exactly one draft of each is Current
//! (`screenplay.current_draft_id`). Reads and commands name the screenplay or
//! draft they work on, so nothing picks a screenplay silently. A draft owns ordered
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
//! automatic history), `delete` (a whole screenplay to Recently Deleted and
//! back), `compare` (draft comparison), `review` (review rounds), `characters`
//! (character-cue usage), `extras` (search, trash, scene hub, story reference,
//! title page, view state).

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
pub mod delete;
pub mod drafts;
pub mod edit;
pub mod extras;
pub mod numbering;
pub mod pages;
pub mod review;
pub mod setup;

pub use drafts::{NewElement, NewScene, create_screenplay_tx, insert_draft_tx};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Screenplay");
    r.query("screenplay.overview", overview).meta(M::read(
        "The screenplays of the project (or an episode) and the drafts of the chosen one.",
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
    delete::register(r);
    compare::register(r);
    review::register(r);
    characters::register(r);
    extras::register(r);
    setup::register(r);
    numbering::register(r);
    pages::register(r);
}

// ------------------------------------------------------------------- DTOs

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenplayTitlePage {
    pub title: String,
    pub written_by: String,
    /// Credit line above the authors ("Written by" when empty).
    pub credit: String,
    /// Source material ("Based on …").
    pub source: String,
    pub contact_name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    /// Free contact text (older title pages kept everything here).
    pub contact: String,
    /// Draft name line ("Shooting Draft").
    pub draft_line: String,
    /// Revision label line; empty means the open draft's revision label and colour.
    pub revision_line: String,
    pub date: String,
    /// Agent or manager block.
    pub representation: String,
    pub notes: String,
    pub copyright: String,
}

impl ScreenplayTitlePage {
    /// (value, field name, longest allowed) for validation.
    pub(crate) fn limits(&self) -> [(&String, &'static str, usize); 15] {
        [
            (&self.title, "Title", 200),
            (&self.written_by, "Written by", 300),
            (&self.credit, "Credit", 80),
            (&self.source, "Source", 300),
            (&self.contact_name, "Contact name", 200),
            (&self.address, "Address", 500),
            (&self.phone, "Phone", 80),
            (&self.email, "Email", 200),
            (&self.contact, "Contact", 1_000),
            (&self.draft_line, "Draft", 200),
            (&self.revision_line, "Revision", 200),
            (&self.date, "Date", 80),
            (&self.representation, "Representation", 500),
            (&self.notes, "Notes on the title page", 2_000),
            (&self.copyright, "Copyright", 200),
        ]
    }
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
    /// Paper, font, spacing, header and footer and export defaults.
    pub page_setup: setup::ScreenplayPageSetup,
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

/// One screenplay of a project or episode, for pickers and lists.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySummary {
    pub id: String,
    pub episode_id: Option<String>,
    pub title: String,
    /// Feature / Short / Episodic.
    pub format: String,
    pub current_draft_id: Option<String>,
    pub current_draft_name: Option<String>,
    #[ts(type = "number")]
    pub draft_count: i64,
    #[ts(type = "number")]
    pub created_at: i64,
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
    /// Set for `custom` elements.
    pub custom_element_id: Option<String>,
    /// Revision label of a revised line (shown as an asterisk in revision output).
    pub revision_mark: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneDto {
    pub id: String,
    pub lineage_id: String,
    /// Position among the draft's scenes (1-based).
    pub number: u32,
    /// Scene number shown everywhere: derived from order until the draft is
    /// numbered (import, lock or by hand), then the production label such as "12A".
    pub label: String,
    /// "imported" | "locked" | "inserted" | "manual" when the label is stored.
    pub number_source: Option<String>,
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
    /// Numbered scenes omitted from this draft (kept for their labels; printed as OMITTED).
    pub omitted_scenes: Vec<ScreenplayOmittedScene>,
    /// The draft's scene numbers are frozen labels (imported, locked or set by hand).
    pub numbering_frozen: bool,
    /// Custom elements the editor offers for this screenplay.
    pub custom_elements: Vec<setup::ScreenplayCustomElementDto>,
    /// Edit sequence of the draft: lets the editor ignore reads older than its own saves.
    #[ts(type = "number")]
    pub seq: i64,
    /// The actor may edit and the draft is not locked.
    pub can_edit: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayOmittedScene {
    pub id: String,
    pub label: String,
    pub heading: String,
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
    /// Episodic/Series projects keep their screenplays inside episodes (FSD §25.3).
    pub episodic: bool,
    pub episodes: Vec<ScreenplayEpisodeRef>,
    pub episode_id: Option<String>,
    /// Every screenplay of the project (or of the chosen episode), oldest first.
    pub screenplays: Vec<ScreenplaySummary>,
    /// The screenplay shown: the one asked for, else the oldest in scope.
    pub screenplay: Option<ScreenplayDto>,
    /// Drafts of `screenplay`.
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
    /// The screenplay to show. An id that is not a live screenplay of this
    /// scope (a stale saved view) falls back to the oldest one; the picker
    /// always shows which screenplay is open.
    #[serde(default)]
    #[ts(optional)]
    pub screenplay_id: Option<String>,
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

/// Keeps a draft query to drafts whose screenplay is live: a screenplay in Recently Deleted
/// takes its drafts with it, so nothing reads or edits them until it is restored.
const LIVE_SCREENPLAY: &str = "EXISTS(SELECT 1 FROM screenplay sp
        WHERE sp.id = screenplay_draft.screenplay_id AND sp.deleted_at IS NULL)";

/// A live (not deleted) draft of a live screenplay.
pub(crate) fn load_draft(c: &Connection, id: &str) -> AppResult<DraftRow> {
    let row = c
        .query_row(
            &format!(
                "SELECT {DRAFT_COLS} FROM screenplay_draft WHERE id=?1 AND deleted_at IS NULL AND {LIVE_SCREENPLAY}"
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
        "SELECT {DRAFT_COLS} FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL AND {LIVE_SCREENPLAY}
         ORDER BY created_at, id"
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
        "SELECT id, episode_id, title, format, current_draft_id, title_page_json, rev, page_setup_json FROM screenplay WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| {
            let tp: String = r.get(5)?;
            let ps: String = r.get(7)?;
            Ok(ScreenplayDto {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                title: r.get(2)?,
                format: r.get(3)?,
                current_draft_id: r.get(4)?,
                title_page: serde_json::from_str(&tp).unwrap_or_default(),
                page_setup: serde_json::from_str(&ps).unwrap_or_default(),
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
    let row: Option<(String, String, Option<String>)> = c
        .query_row(
            "SELECT s.draft_id, sp.id, sp.episode_id FROM screenplay_scene s
               JOIN screenplay_draft d ON d.id = s.draft_id JOIN screenplay sp ON sp.id = d.screenplay_id
              WHERE s.id=?1",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let mut v = serde_json::json!({ "workspace": "screenplay", "sceneId": scene_id });
    if let Some((draft, screenplay, episode)) = row {
        v["draftId"] = serde_json::json!(draft);
        v["screenplayId"] = serde_json::json!(screenplay);
        if let Some(e) = episode {
            v["episodeId"] = serde_json::json!(e);
        }
    }
    Ok(v)
}

/// Navigation target for a screenplay draft (opens it in its screenplay and episode).
pub(crate) fn draft_nav(c: &Connection, draft_id: &str) -> AppResult<serde_json::Value> {
    let row: Option<(String, Option<String>)> = c
        .query_row(
            "SELECT sp.id, sp.episode_id FROM screenplay_draft d JOIN screenplay sp ON sp.id = d.screenplay_id WHERE d.id=?1",
            [draft_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let mut v = serde_json::json!({ "workspace": "screenplay", "draftId": draft_id });
    if let Some((screenplay, episode)) = row {
        v["screenplayId"] = serde_json::json!(screenplay);
        if let Some(e) = episode {
            v["episodeId"] = serde_json::json!(e);
        }
    }
    Ok(v)
}

/// Every live screenplay of a scope (project level when `episode_id` is None),
/// oldest first. A project may hold several (ADR-0012, 2026-10-08).
pub(crate) fn screenplays_in_scope(
    c: &Connection,
    episode_id: Option<&str>,
) -> AppResult<Vec<ScreenplaySummary>> {
    let mut stmt = c.prepare(
        "SELECT s.id, s.episode_id, s.title, s.format, s.current_draft_id,
                (SELECT d.name FROM screenplay_draft d WHERE d.id = s.current_draft_id AND d.deleted_at IS NULL),
                (SELECT count(*) FROM screenplay_draft d WHERE d.screenplay_id = s.id AND d.deleted_at IS NULL),
                s.created_at
         FROM screenplay s WHERE s.episode_id IS ?1 AND s.deleted_at IS NULL ORDER BY s.created_at, s.id",
    )?;
    let rows = stmt
        .query_map([episode_id], |r| {
            Ok(ScreenplaySummary {
                id: r.get(0)?,
                episode_id: r.get(1)?,
                title: r.get(2)?,
                format: r.get(3)?,
                current_draft_id: r.get(4)?,
                current_draft_name: r.get(5)?,
                draft_count: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// A title no other live screenplay of the scope uses: `base`, else
/// `base (2)`, `base (3)`… Screenplay pickers list titles, so they stay apart.
pub(crate) fn unique_screenplay_title(
    c: &Connection,
    episode_id: Option<&str>,
    base: &str,
) -> AppResult<String> {
    let base: String = base.trim().chars().take(MAX_SCREENPLAY_TITLE - 6).collect();
    let base = if base.is_empty() {
        "Screenplay".to_string()
    } else {
        base
    };
    if !screenplay_title_taken(c, episode_id, &base, None)? {
        return Ok(base);
    }
    for n in 2..1000 {
        let candidate = format!("{base} ({n})");
        if !screenplay_title_taken(c, episode_id, &candidate, None)? {
            return Ok(candidate);
        }
    }
    Err(AppError::conflict(
        "Too many screenplays share this title. Choose a different title.",
    ))
}

/// Longest screenplay title.
pub(crate) const MAX_SCREENPLAY_TITLE: usize = 200;

/// Another live screenplay of the scope already uses this title (case and
/// surrounding spaces ignored).
pub(crate) fn screenplay_title_taken(
    c: &Connection,
    episode_id: Option<&str>,
    title: &str,
    except: Option<&str>,
) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay WHERE episode_id IS ?1 AND deleted_at IS NULL
                         AND lower(trim(title)) = lower(trim(?2)) AND id IS NOT ?3)",
        params![episode_id, title, except],
        |r| r.get(0),
    )?)
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

/// Full ordered content of a draft: the scenes the editor shows (omitted
/// scenes of a numbered draft are left out; see [`load_scenes_all`]).
pub(crate) fn load_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<ScreenplaySceneDto>> {
    Ok(load_scenes_all(c, draft_id)?
        .into_iter()
        .filter(|(_, omitted)| !omitted)
        .map(|(s, _)| s)
        .collect())
}

/// Every live scene with its omitted flag, numbered and labelled.
pub(crate) fn load_scenes_all(
    c: &Connection,
    draft_id: &str,
) -> AppResult<Vec<(ScreenplaySceneDto, bool)>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, notes, synopsis, story_day, time_note, source_scene_card_id, rev,
                production_number, production_number_source, omitted
         FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let mut stored: Vec<numbering::NumberRow> = Vec::new();
    let mut omitted: Vec<bool> = Vec::new();
    let mut scenes: Vec<ScreenplaySceneDto> = stmt
        .query_map([draft_id], |r| {
            Ok((
                ScreenplaySceneDto {
                    id: r.get(0)?,
                    lineage_id: r.get(1)?,
                    number: 0,
                    label: String::new(),
                    number_source: r.get(10)?,
                    heading: r.get(2)?,
                    notes: r.get(3)?,
                    synopsis: r.get(4)?,
                    story_day: r.get(5)?,
                    time_note: r.get(6)?,
                    source_scene_card_id: r.get(7)?,
                    elements: vec![],
                    rev: r.get(8)?,
                },
                r.get::<_, Option<String>>(9)?,
                r.get::<_, i64>(11)? != 0,
            ))
        })?
        .map(|row| {
            row.map(|(s, label, o)| {
                stored.push(numbering::NumberRow { stored: label });
                omitted.push(o);
                s
            })
        })
        .collect::<Result<_, _>>()?;
    for (s, l) in scenes.iter_mut().zip(numbering::production_labels(&stored)) {
        s.label = l;
    }
    let index: HashMap<String, usize> = scenes
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    let mut el = c.prepare(
        "SELECT e.id, e.scene_id, e.element_type, e.text, e.dual, e.custom_element_id, e.revision_mark
         FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL ORDER BY e.position, e.id",
    )?;
    let rows = el.query_map([draft_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, bool>(4)?,
            r.get::<_, Option<String>>(5)?,
            r.get::<_, Option<String>>(6)?,
        ))
    })?;
    for row in rows {
        let (id, scene_id, t, text, dual, custom_element_id, revision_mark) = row?;
        let element_type = ElementType::parse_field(&t, "element type")?;
        if let Some(&i) = index.get(&scene_id) {
            scenes[i].elements.push(ScreenplayElementDto {
                id,
                element_type,
                text,
                dual,
                custom_element_id,
                revision_mark,
            });
        }
    }
    for (i, s) in scenes.iter_mut().enumerate() {
        s.number = (i + 1) as u32;
    }
    Ok(scenes.into_iter().zip(omitted).collect())
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
        let screenplays = if episodic && episode_id.is_none() {
            Vec::new()
        } else {
            screenplays_in_scope(c, episode_id.as_deref())?
        };
        let chosen = args
            .screenplay_id
            .as_deref()
            .and_then(|want| screenplays.iter().find(|s| s.id == want))
            .or_else(|| screenplays.first());
        let screenplay = match chosen {
            Some(s) => Some(load_screenplay(c, &s.id)?),
            None => None,
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
            screenplays,
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
        let all = load_scenes_all(c, &d.id)?;
        let numbering_frozen = all.iter().any(|(s, _)| s.number_source.is_some());
        let (omitted, scenes): (Vec<_>, Vec<_>) = all.into_iter().partition(|(_, o)| *o);
        Ok(ScreenplayDocument {
            can_edit: actor.role.allows(Capability::Edit) && d.status != DraftStatus::Locked,
            scenes: scenes.into_iter().map(|(s, _)| s).collect(),
            omitted_scenes: omitted
                .into_iter()
                .map(|(s, _)| ScreenplayOmittedScene {
                    id: s.id,
                    label: s.label,
                    heading: s.heading,
                })
                .collect(),
            numbering_frozen,
            custom_elements: setup::custom_elements_of(c, &screenplay.id)?,
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
