//! Story Timeline (FSD §14, UX §3.10): optional Story Day / time-of-day notes
//! on screenplay scenes (hub columns `screenplay_scene.story_day/time_note`).
//! The timeline groups scenes by day; it never reorders anything.

use std::collections::BTreeMap;

use openframe_domain::{Actor, AppError, AppResult, Capability};
use openframe_persistence::rows::{opt_text, update_fields};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::build::screenplays_in_scope;
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.query("story.timeline", timeline).meta(M::read(
        "Story timeline: Story Day of each screenplay scene.",
    ));
    r.command("story.assign_story_day", assign_story_day)
        .meta(M::edit(
            "Assign screenplay scenes to a Story Day (or clear it).",
        ));
    r.command("story.set_time_note", set_time_note)
        .meta(M::edit("Set a screenplay scene's time note."));
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryTimelineScene {
    pub scene_id: String,
    /// Derived from screenplay order.
    #[ts(type = "number")]
    pub number: i64,
    pub heading: String,
    pub story_day: Option<String>,
    pub time_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryDayGroup {
    /// None = "Unassigned".
    pub story_day: Option<String>,
    pub scenes: Vec<StoryTimelineScene>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryScreenplayRef {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryTimelineDto {
    pub screenplays: Vec<StoryScreenplayRef>,
    pub screenplay_id: Option<String>,
    pub draft_id: Option<String>,
    pub draft_name: Option<String>,
    pub groups: Vec<StoryDayGroup>,
    #[ts(type = "number")]
    pub scene_count: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryTimelineArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
    #[serde(default)]
    pub screenplay_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryAssignDayArgs {
    pub scene_ids: Vec<String>,
    /// None or blank clears the Story Day (scene goes back to "Unassigned").
    #[serde(default)]
    pub story_day: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryTimeNoteArgs {
    pub scene_id: String,
    #[serde(default)]
    pub time_note: Option<String>,
}

/// Sort key: numeric day labels ("1", "Day 2") in numeric order, others after, alphabetically.
pub fn day_sort_key(day: &str) -> (u8, i64, String) {
    let digits: String = day
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    match digits.parse::<i64>() {
        Ok(n) => (0, n, day.to_lowercase()),
        Err(_) => (1, 0, day.to_lowercase()),
    }
}

fn load(c: &Connection, a: &StoryTimelineArgs) -> AppResult<StoryTimelineDto> {
    let ep = a.episode_id.as_deref();
    super::tree::check_episode(c, ep)?;
    let options = screenplays_in_scope(c, ep)?;
    let chosen = match &a.screenplay_id {
        Some(id) => Some(
            options
                .iter()
                .find(|s| &s.id == id)
                .ok_or_else(|| AppError::not_found("screenplay"))?
                .clone(),
        ),
        None => options
            .iter()
            .find(|s| s.current_draft_id.is_some())
            .cloned(),
    };
    let screenplays = options
        .iter()
        .map(|s| StoryScreenplayRef {
            id: s.id.clone(),
            title: s.title.clone(),
        })
        .collect();
    let Some(sp) = chosen else {
        return Ok(StoryTimelineDto {
            screenplays,
            screenplay_id: None,
            draft_id: None,
            draft_name: None,
            groups: vec![],
            scene_count: 0,
        });
    };
    let Some(draft) = sp.current_draft_id.clone() else {
        return Ok(StoryTimelineDto {
            screenplays,
            screenplay_id: Some(sp.id),
            draft_id: None,
            draft_name: None,
            groups: vec![],
            scene_count: 0,
        });
    };
    let rows: Vec<(String, String, Option<String>, Option<String>)> = {
        let mut stmt = c.prepare(
            "SELECT id, heading, story_day, time_note FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        stmt.query_map([&draft], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?
    };
    let scene_count = rows.len() as i64;
    let mut days: BTreeMap<(u8, i64, String), (String, Vec<StoryTimelineScene>)> = BTreeMap::new();
    let mut unassigned = Vec::new();
    for (i, (id, heading, day, note)) in rows.into_iter().enumerate() {
        let scene = StoryTimelineScene {
            scene_id: id,
            number: i as i64 + 1,
            heading,
            story_day: day.clone(),
            time_note: note,
        };
        match day.filter(|d| !d.trim().is_empty()) {
            Some(d) => days
                .entry(day_sort_key(&d))
                .or_insert_with(|| (d.clone(), Vec::new()))
                .1
                .push(scene),
            None => unassigned.push(scene),
        }
    }
    let mut groups: Vec<StoryDayGroup> = days
        .into_values()
        .map(|(d, scenes)| StoryDayGroup {
            story_day: Some(d),
            scenes,
        })
        .collect();
    if !unassigned.is_empty() {
        groups.push(StoryDayGroup {
            story_day: None,
            scenes: unassigned,
        });
    }
    Ok(StoryTimelineDto {
        screenplays,
        draft_name: c
            .query_row(
                "SELECT name FROM screenplay_draft WHERE id=?1",
                [&draft],
                |r| r.get(0),
            )
            .optional()?,
        screenplay_id: Some(sp.id),
        draft_id: Some(draft),
        groups,
        scene_count,
    })
}

fn timeline(core: &AppCore, actor: &Actor, a: StoryTimelineArgs) -> AppResult<StoryTimelineDto> {
    actor.require(Capability::View, "view the Story Timeline")?;
    core.project()?.store.read(|c| load(c, &a))
}

fn check_scene(c: &Connection, id: &str) -> AppResult<()> {
    let ok: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_scene s JOIN screenplay_draft d ON d.id=s.draft_id
          WHERE s.id=?1 AND s.deleted_at IS NULL AND d.deleted_at IS NULL)",
        [id],
        |r| r.get(0),
    )?;
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found("scene"))
    }
}

fn assign_story_day(core: &AppCore, actor: &Actor, a: StoryAssignDayArgs) -> AppResult<()> {
    if a.scene_ids.is_empty() {
        return Err(AppError::invalid_input("Select at least one scene."));
    }
    let day = optional_text(a.story_day.clone(), "Story Day", 40)?;
    let summary = match (&day, a.scene_ids.len()) {
        (Some(d), 1) => format!("Assigned Story Day “{d}”"),
        (Some(d), n) => format!("Assigned Story Day “{d}” to {n} scenes"),
        (None, _) => "Cleared Story Day".to_string(),
    };
    core.project()?.store.mutate(
        actor,
        MutationMeta::new("story.assign_story_day", summary, Capability::Edit),
        |tx| {
            for id in &a.scene_ids {
                check_scene(tx.conn(), id)?;
                update_fields(
                    tx.conn(),
                    "screenplay_scene",
                    id,
                    &[("story_day", opt_text(day.clone()))],
                    &["story_day"],
                    None,
                    "scene",
                )?;
            }
            Ok(())
        },
    )
}

fn set_time_note(core: &AppCore, actor: &Actor, a: StoryTimeNoteArgs) -> AppResult<()> {
    let note = optional_text(a.time_note.clone(), "Time note", 80)?;
    core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.set_time_note",
            "Edited time-of-day note",
            Capability::Edit,
        )
        .target("screenplay_scene", &a.scene_id)
        .coalesce(format!("story.time_note:{}", a.scene_id)),
        |tx| {
            check_scene(tx.conn(), &a.scene_id)?;
            update_fields(
                tx.conn(),
                "screenplay_scene",
                &a.scene_id,
                &[("time_note", opt_text(note.clone()))],
                &["time_note"],
                None,
                "scene",
            )?;
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::day_sort_key;

    #[test]
    fn natural_day_order() {
        let mut days = vec!["Day 10", "2", "Day 1", "Flashback"];
        days.sort_by_key(|d| day_sort_key(d));
        assert_eq!(days, vec!["Day 1", "2", "Day 10", "Flashback"]);
    }
}
