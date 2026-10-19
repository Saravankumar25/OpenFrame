//! Episodic / Series structure (FSD §25, UX §3.20): seasons, episodes, the
//! Season Board order and episode duplication. Each episode scopes its own
//! Story Board (story rows carry `episode_id`).

use std::collections::HashMap;

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{opt_text, renumber, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tree::short;
use super::{StoryCreated, StoryIdArgs, ensure_live};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{DeleteSpec, MutationMeta, soft_delete};
use crate::util::{optional_text, required_text};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.query("story.series", series)
        .meta(M::read("Seasons and episodes of a series."));
    r.command("story.create_season", create_season)
        .meta(M::edit("Create a season."));
    r.command("story.update_season", update_season)
        .meta(M::edit("Rename a season or edit its note."));
    r.command("story.move_season", move_season)
        .meta(M::edit("Move a season to another position."));
    r.command("story.delete_season", delete_season)
        .meta(M::soft_delete("Move a season to Recently Deleted.").confirm());
    r.command("story.create_episode", create_episode)
        .meta(M::edit("Create an episode."));
    r.command("story.update_episode", update_episode)
        .meta(M::edit("Edit an episode's title, summary or status."));
    r.command("story.move_episode", move_episode)
        .meta(M::edit("Move an episode to another season or position."));
    r.command("story.delete_episode", delete_episode)
        .meta(M::soft_delete("Move an episode to Recently Deleted.").confirm());
    r.command("story.duplicate_episode", duplicate_episode)
        .meta(M::edit(
            "Duplicate an episode (optionally with its Story Board).",
        ));
}

/// Episode status labels shown on the series home (UX §3.20).
pub const EPISODE_STATUSES: &[&str] = &[
    "Idea",
    "Development",
    "Writing",
    "Rewrite",
    "Locked",
    "Shooting",
    "Complete",
];

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryEpisodeDto {
    pub id: String,
    pub season_id: Option<String>,
    pub title: String,
    pub summary: Option<String>,
    pub status: Option<String>,
    /// Episode number = order within its season (derived, never stored).
    #[ts(type = "number")]
    pub number: i64,
    #[ts(type = "number")]
    pub card_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorySeasonDto {
    pub id: String,
    pub title: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub number: i64,
    pub episodes: Vec<StoryEpisodeDto>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorySeriesDto {
    pub project_type: String,
    pub is_episodic: bool,
    pub seasons: Vec<StorySeasonDto>,
    /// Episodes not in any season.
    pub unseasoned: Vec<StoryEpisodeDto>,
    pub statuses: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorySeriesArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateSeasonArgs {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateSeasonArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryMoveSeasonArgs {
    pub id: String,
    #[serde(default)]
    pub before_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateEpisodeArgs {
    #[serde(default)]
    pub season_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateEpisodeArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryMoveEpisodeArgs {
    pub id: String,
    /// Destination season (None = no season).
    #[serde(default)]
    pub season_id: Option<String>,
    #[serde(default)]
    pub before_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryDuplicateEpisodeArgs {
    pub id: String,
    /// Also copy the episode's Story Board (new identities; no screenplay links).
    #[serde(default)]
    pub copy_story: bool,
}

fn check_status(s: Option<String>) -> AppResult<Option<String>> {
    match s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(v) => EPISODE_STATUSES
            .iter()
            .find(|x| x.eq_ignore_ascii_case(&v))
            .map(|x| Some(x.to_string()))
            .ok_or_else(|| AppError::invalid_input("Choose one of the offered episode statuses.")),
    }
}

fn episodes_of(c: &Connection, season: Option<&str>) -> AppResult<Vec<StoryEpisodeDto>> {
    let mut stmt = c.prepare(
        "SELECT e.id, e.season_id, e.title, e.summary, e.status, e.rev,
                (SELECT count(*) FROM story_scene_card k WHERE k.episode_id = e.id AND k.deleted_at IS NULL)
         FROM episode e WHERE e.season_id IS ?1 AND e.deleted_at IS NULL ORDER BY e.position, e.id",
    )?;
    let rows = stmt
        .query_map([season], |r| {
            Ok(StoryEpisodeDto {
                id: r.get(0)?,
                season_id: r.get(1)?,
                title: r.get(2)?,
                summary: r.get(3)?,
                status: r.get(4)?,
                rev: r.get(5)?,
                card_count: r.get(6)?,
                number: 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, mut e)| {
            e.number = i as i64 + 1;
            e
        })
        .collect())
}

pub(crate) fn load_series(c: &Connection) -> AppResult<StorySeriesDto> {
    let project_type: String =
        c.query_row("SELECT project_type FROM project LIMIT 1", [], |r| r.get(0))?;
    let seasons: Vec<(String, String, Option<String>, i64)> = {
        let mut stmt = c.prepare("SELECT id, title, note, rev FROM season WHERE deleted_at IS NULL ORDER BY position, id")?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<_, _>>()?
    };
    let mut out = Vec::new();
    for (i, (id, title, note, rev)) in seasons.into_iter().enumerate() {
        let episodes = episodes_of(c, Some(&id))?;
        out.push(StorySeasonDto {
            id,
            title,
            note,
            number: i as i64 + 1,
            episodes,
            rev,
        });
    }
    Ok(StorySeriesDto {
        is_episodic: matches!(project_type.as_str(), "Episodic" | "Series"),
        project_type,
        seasons: out,
        unseasoned: episodes_of(c, None)?,
        statuses: EPISODE_STATUSES.iter().map(|s| s.to_string()).collect(),
    })
}

fn series(core: &AppCore, actor: &Actor, _: StorySeriesArgs) -> AppResult<StorySeriesDto> {
    actor.require(Capability::View, "view episodes")?;
    core.project()?.store.read(load_series)
}

fn season_ids(c: &Connection) -> AppResult<Vec<String>> {
    let mut stmt =
        c.prepare("SELECT id FROM season WHERE deleted_at IS NULL ORDER BY position, id")?;
    let ids = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

fn episode_ids(c: &Connection, season: Option<&str>) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare(
        "SELECT id FROM episode WHERE season_id IS ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let ids = stmt
        .query_map([season], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

fn place_in(
    mut ids: Vec<String>,
    id: &str,
    before: Option<&str>,
    what: &str,
) -> AppResult<Vec<String>> {
    ids.retain(|x| x != id);
    let idx = match before {
        Some(b) => ids
            .iter()
            .position(|x| x == b)
            .ok_or_else(|| AppError::not_found(what))?,
        None => ids.len(),
    };
    ids.insert(idx, id.to_string());
    Ok(ids)
}

fn create_season(
    core: &AppCore,
    actor: &Actor,
    a: StoryCreateSeasonArgs,
) -> AppResult<StoryCreated> {
    let s = core.project()?;
    let n = s.store.read(|c| Ok(season_ids(c)?.len()))? + 1;
    let title = match a.title {
        Some(t) => required_text(&t, "Season title", 120)?,
        None => format!("Season {n}"),
    };
    s.store.mutate(actor, MutationMeta::new("story.create_season", format!("Added “{}”", short(&title)), Capability::Edit), |tx| {
        let c = tx.conn();
        let id = new_id();
        let now = now_ms();
        let pos = season_ids(c)?.len() as i64 + 1;
        c.execute(
            "INSERT INTO season(id, title, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, title, pos, now],
        )?;
        Ok(StoryCreated { id })
    })
}

fn update_season(core: &AppCore, actor: &Actor, a: StoryUpdateSeasonArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(t) = &a.title {
        fields.push(("title", text(required_text(t, "Season title", 120)?)));
    }
    if let Some(n) = &a.note {
        fields.push((
            "note",
            opt_text(optional_text(Some(n.clone()), "Note", 2000)?),
        ));
    }
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.update_season", "Edited season", Capability::Edit)
            .target("season", &a.id)
            .coalesce(format!("story.season:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "season", &a.id, "season")?;
            update_fields(
                tx.conn(),
                "season",
                &a.id,
                &fields,
                &["title", "note"],
                None,
                "season",
            )?;
            Ok(())
        },
    )
}

fn move_season(core: &AppCore, actor: &Actor, a: StoryMoveSeasonArgs) -> AppResult<()> {
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.move_season", "Reordered seasons", Capability::Edit),
        |tx| {
            let c = tx.conn();
            ensure_live(c, "season", &a.id, "season")?;
            let ids = place_in(season_ids(c)?, &a.id, a.before_id.as_deref(), "season")?;
            renumber(c, "season", &ids)
        },
    )
}

/// Deleting a season never deletes episodes: they move to "No season" first.
fn delete_season(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let title: String = s.store.read(|c| {
        c.query_row(
            "SELECT title FROM season WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("season"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.delete_season",
            format!("Deleted “{}”", short(&title)),
            Capability::SoftDelete,
        )
        .target("season", &a.id),
        |tx| {
            let c = tx.conn();
            let pos = season_ids(c)?.iter().position(|x| x == &a.id).unwrap_or(0);
            let moving = episode_ids(c, Some(&a.id))?;
            let mut rest = episode_ids(c, None)?;
            let now = now_ms();
            for e in &moving {
                c.execute(
                    "UPDATE episode SET season_id=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
                    params![now, e],
                )?;
                rest.push(e.clone());
            }
            renumber(c, "episode", &rest)?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "season",
                    table: "season",
                    id: &a.id,
                    title: Some(title.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: Some(pos as i64),
                },
            )
        },
    )
}

fn check_season(c: &Connection, season: Option<&str>) -> AppResult<()> {
    if let Some(s) = season {
        ensure_live(c, "season", s, "season")?;
    }
    Ok(())
}

fn create_episode(
    core: &AppCore,
    actor: &Actor,
    a: StoryCreateEpisodeArgs,
) -> AppResult<StoryCreated> {
    let title = required_text(&a.title, "Episode title", 200)?;
    let summary = optional_text(a.summary, "Summary", 2000)?;
    let status = check_status(a.status)?.or_else(|| Some("Idea".to_string()));
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.create_episode", format!("Added episode “{}”", short(&title)), Capability::Edit),
        |tx| {
            let c = tx.conn();
            check_season(c, a.season_id.as_deref())?;
            let id = new_id();
            let now = now_ms();
            let pos = episode_ids(c, a.season_id.as_deref())?.len() as i64 + 1;
            c.execute(
                "INSERT INTO episode(id, season_id, title, summary, status, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![id, a.season_id, title, summary, status, pos, now],
            )?;
            Ok(StoryCreated { id })
        },
    )
}

fn update_episode(core: &AppCore, actor: &Actor, a: StoryUpdateEpisodeArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut summary_text = "Edited episode".to_string();
    if let Some(t) = &a.title {
        let t = required_text(t, "Episode title", 200)?;
        summary_text = format!("Renamed episode to “{}”", short(&t));
        fields.push(("title", text(t)));
    }
    if let Some(s) = &a.summary {
        fields.push((
            "summary",
            opt_text(optional_text(Some(s.clone()), "Summary", 2000)?),
        ));
    }
    if let Some(s) = &a.status {
        let st = check_status(Some(s.clone()))?;
        summary_text = format!(
            "Set episode status to {}",
            st.clone().unwrap_or_else(|| "none".into())
        );
        fields.push(("status", opt_text(st)));
    }
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.update_episode", summary_text, Capability::Edit)
            .target("episode", &a.id)
            .coalesce(format!("story.episode:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "episode", &a.id, "episode")?;
            update_fields(
                tx.conn(),
                "episode",
                &a.id,
                &fields,
                &["title", "summary", "status"],
                a.expected_rev,
                "episode",
            )?;
            Ok(())
        },
    )
}

/// Season Board drag & drop (FSD §25.6).
fn move_episode(core: &AppCore, actor: &Actor, a: StoryMoveEpisodeArgs) -> AppResult<()> {
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.move_episode", "Reordered episodes", Capability::Edit)
            .target("episode", &a.id),
        |tx| {
            let c = tx.conn();
            ensure_live(c, "episode", &a.id, "episode")?;
            check_season(c, a.season_id.as_deref())?;
            let old: Option<String> =
                c.query_row("SELECT season_id FROM episode WHERE id=?1", [&a.id], |r| {
                    r.get(0)
                })?;
            if old != a.season_id {
                c.execute(
                    "UPDATE episode SET season_id=?1, updated_at=?2, rev=rev+1 WHERE id=?3",
                    params![a.season_id, now_ms(), a.id],
                )?;
                let rest = episode_ids(c, old.as_deref())?;
                renumber(c, "episode", &rest)?;
            }
            let ids = place_in(
                episode_ids(c, a.season_id.as_deref())?,
                &a.id,
                a.before_id.as_deref(),
                "episode",
            )?;
            renumber(c, "episode", &ids)
        },
    )
}

/// Recoverable; the episode's story content stays scoped to it and returns on restore.
fn delete_episode(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    let s = core.project()?;
    let (title, season): (String, Option<String>) = s.store.read(|c| {
        c.query_row(
            "SELECT title, season_id FROM episode WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("episode"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.delete_episode",
            format!("Deleted episode “{}”", short(&title)),
            Capability::SoftDelete,
        )
        .target("episode", &a.id),
        |tx| {
            let pos = episode_ids(tx.conn(), season.as_deref())?
                .iter()
                .position(|x| x == &a.id)
                .unwrap_or(0);
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "episode",
                    table: "episode",
                    id: &a.id,
                    title: Some(title.clone()),
                    parent_type: Some("season"),
                    parent_id: season.clone(),
                    position: Some(pos as i64),
                },
            )
        },
    )
}

/// Duplicate an episode (FSD §25.7): new identity; optional copy of its Story
/// Board with new identities for every act/sequence/beat/card.
fn duplicate_episode(
    core: &AppCore,
    actor: &Actor,
    a: StoryDuplicateEpisodeArgs,
) -> AppResult<StoryCreated> {
    let s = core.project()?;
    let title: String = s.store.read(|c| {
        c.query_row(
            "SELECT title FROM episode WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("episode"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.duplicate_episode", format!("Duplicated episode “{}”", short(&title)), Capability::Edit),
        |tx| {
            let c = tx.conn();
            let (season, summary, status): (Option<String>, Option<String>, Option<String>) = c.query_row(
                "SELECT season_id, summary, status FROM episode WHERE id=?1",
                [&a.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let new_ep = new_id();
            let now = now_ms();
            c.execute(
                "INSERT INTO episode(id, season_id, title, summary, status, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
                params![new_ep, season, format!("{title} (copy)"), summary, status, now],
            )?;
            let mut ids = episode_ids(c, season.as_deref())?;
            ids.retain(|x| x != &new_ep);
            let at = ids.iter().position(|x| x == &a.id).map(|i| i + 1).unwrap_or(ids.len());
            ids.insert(at, new_ep.clone());
            renumber(c, "episode", &ids)?;
            if a.copy_story {
                copy_story(c, &a.id, &new_ep)?;
            }
            Ok(StoryCreated { id: new_ep })
        },
    )
}

fn copy_story(c: &Connection, from: &str, to: &str) -> AppResult<()> {
    let now = now_ms();
    let mut map: HashMap<String, String> = HashMap::new();
    let acts: Vec<(String, String, Option<String>, i64)> = {
        let mut stmt = c.prepare("SELECT id, title, note, position FROM story_act WHERE episode_id=?1 AND deleted_at IS NULL")?;
        stmt.query_map([from], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<_, _>>()?
    };
    for (id, title, note, pos) in acts {
        let nid = new_id();
        c.execute(
            "INSERT INTO story_act(id, episode_id, title, note, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![nid, to, title, note, pos, now],
        )?;
        map.insert(id, nid);
    }
    let seqs: Vec<(String, Option<String>, String, Option<String>, i64)> = {
        let mut stmt = c.prepare(
            "SELECT id, act_id, title, note, position FROM story_sequence WHERE episode_id=?1 AND deleted_at IS NULL",
        )?;
        stmt.query_map([from], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?
    };
    for (id, act, title, note, pos) in seqs {
        let nid = new_id();
        let nact = act.and_then(|a| map.get(&a).cloned());
        c.execute(
            "INSERT INTO story_sequence(id, act_id, title, note, position, episode_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![nid, nact, title, note, pos, to, now],
        )?;
        copy_attachments(c, "sequence", &id, &nid, now)?;
        map.insert(id, nid);
    }
    let beats: Vec<(
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        i64,
    )> = {
        let mut stmt = c.prepare(
            "SELECT id, parent_type, parent_id, text, note, color, position FROM story_beat WHERE episode_id=?1 AND deleted_at IS NULL",
        )?;
        stmt.query_map([from], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?
    };
    let mut new_beats = Vec::new();
    for (id, pt, pid, t, note, color, pos) in beats {
        let nid = new_id();
        let (npt, npid) = remap(&map, &pt, pid);
        c.execute(
            "INSERT INTO story_beat(id, episode_id, parent_type, parent_id, text, note, color, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![nid, to, npt, npid, t, note, color, pos, now],
        )?;
        copy_attachments(c, "beat", &id, &nid, now)?;
        new_beats.push((id.clone(), nid.clone()));
        map.insert(id, nid);
    }
    type CardRow = (
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
    );
    let cards: Vec<CardRow> = {
        let mut stmt = c.prepare(
            "SELECT id, parent_type, parent_id, short_description, scene_heading, notes, color, source_beat_id, position
             FROM story_scene_card WHERE episode_id=?1 AND deleted_at IS NULL",
        )?;
        stmt.query_map([from], |r| {
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
        })?
        .collect::<Result<_, _>>()?
    };
    for (id, pt, pid, d, h, n, col, src, pos) in cards {
        let nid = new_id();
        let (npt, npid) = remap(&map, &pt, pid);
        let nsrc = src.and_then(|s| map.get(&s).cloned());
        c.execute(
            "INSERT INTO story_scene_card(id, episode_id, parent_type, parent_id, short_description, scene_heading, notes, color,
                                          source_beat_id, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![nid, to, npt, npid, d, h, n, col, nsrc, pos, now],
        )?;
        copy_attachments(c, "scene_card", &id, &nid, now)?;
        map.insert(id, nid);
    }
    // Converted beats keep pointing at their (copied) scene card.
    for (old, new) in new_beats {
        let (state, conv): (String, Option<String>) = c.query_row(
            "SELECT state, converted_scene_card_id FROM story_beat WHERE id=?1",
            [&old],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if state == "converted" {
            c.execute(
                "UPDATE story_beat SET state='converted', converted_scene_card_id=?1 WHERE id=?2",
                params![conv.and_then(|x| map.get(&x).cloned()), new],
            )?;
        }
    }
    Ok(())
}

fn remap(map: &HashMap<String, String>, pt: &str, pid: Option<String>) -> (String, Option<String>) {
    match (pt, pid.and_then(|p| map.get(&p).cloned())) {
        ("act", Some(p)) => ("act".into(), Some(p)),
        ("sequence", Some(p)) => ("sequence".into(), Some(p)),
        ("parking", _) => ("parking".into(), None),
        _ => ("unassigned".into(), None),
    }
}

fn copy_attachments(
    c: &Connection,
    owner_type: &str,
    from: &str,
    to: &str,
    now: i64,
) -> AppResult<()> {
    let rows: Vec<(String, i64)> = {
        let mut stmt = c.prepare(
            "SELECT asset_id, position FROM story_attachment WHERE owner_type=?1 AND owner_id=?2",
        )?;
        stmt.query_map(params![owner_type, from], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    for (asset, pos) in rows {
        c.execute(
            "INSERT INTO story_attachment(id, owner_type, owner_id, asset_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![new_id(), owner_type, to, asset, pos, now],
        )?;
    }
    Ok(())
}
