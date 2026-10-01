//! Turn the model's references ("Railway Station", "Act 2", an id it saw in an
//! earlier result, "Scene 12", "Day 3") into canonical rows — or into one focused
//! clarification. Never guesses between several matches (AI spec §6.3).
//!
//! Every lookup filters deleted rows; private rows are only visible to their owner.
//! Table names and label expressions are static strings from this file, never
//! model text.

use openframe_domain::{Actor, AppError, AppResult};
use rusqlite::{Connection, OptionalExtension, params};

use super::super::queries::{self, DraftRef, SceneRow, join_or};
use super::super::scope::ResolvedScope;

/// A resolved project object.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub id: String,
    pub label: String,
    pub rev: i64,
}

/// How to look up one kind of object.
pub struct Entity {
    pub table: &'static str,
    /// Human name, e.g. "Scene Card".
    pub what: &'static str,
    /// SQL expression producing the display label.
    pub label: &'static str,
    /// Extra WHERE condition (always includes the deleted filter where applicable).
    pub filter: &'static str,
    /// Rows are private: only the requesting user's own rows are visible.
    pub owned: bool,
}

macro_rules! entity {
    ($name:ident, $table:literal, $what:literal, $label:literal, $filter:literal) => {
        pub const $name: Entity = Entity {
            table: $table,
            what: $what,
            label: $label,
            filter: $filter,
            owned: false,
        };
    };
}

entity!(ACT, "story_act", "act", "title", "deleted_at IS NULL");
entity!(
    SEQUENCE,
    "story_sequence",
    "sequence",
    "title",
    "deleted_at IS NULL"
);
entity!(
    BEAT,
    "story_beat",
    "beat",
    "text",
    "deleted_at IS NULL AND state='active'"
);
entity!(
    CARD,
    "story_scene_card",
    "Scene Card",
    "COALESCE(NULLIF(trim(short_description),''), NULLIF(trim(scene_heading),''), 'Untitled Scene Card')",
    "deleted_at IS NULL"
);
entity!(
    CHARACTER,
    "story_character",
    "character",
    "name",
    "deleted_at IS NULL"
);
entity!(SEASON, "season", "season", "title", "deleted_at IS NULL");
entity!(EPISODE, "episode", "episode", "title", "deleted_at IS NULL");
entity!(
    FILE,
    "project_file",
    "file",
    "display_name",
    "deleted_at IS NULL"
);
entity!(
    FOLDER,
    "project_file_folder",
    "folder",
    "name",
    "deleted_at IS NULL"
);
entity!(
    NOTE,
    "project_note",
    "Project Note",
    "COALESCE(NULLIF(trim(title),''), substr(body,1,80))",
    "deleted_at IS NULL"
);
entity!(TASK, "task", "task", "title", "deleted_at IS NULL");
entity!(
    TEMPLATE,
    "template",
    "project template",
    "name",
    "deleted_at IS NULL"
);
entity!(
    COMMENT,
    "comment",
    "comment",
    "substr(body,1,80)",
    "deleted_at IS NULL"
);
entity!(
    VAULT_ITEM,
    "vault_item",
    "Idea Vault item",
    "COALESCE(NULLIF(trim(title),''), NULLIF(substr(trim(body),1,80),''), NULLIF(trim(caption),''), url, item_type)",
    "deleted_at IS NULL"
);
entity!(
    VAULT_FOLDER,
    "vault_folder",
    "Idea Vault folder",
    "name",
    "deleted_at IS NULL"
);
entity!(
    VAULT_COLLECTION,
    "vault_collection",
    "Idea Vault collection",
    "name",
    "deleted_at IS NULL"
);
entity!(
    CATALOG_ITEM,
    "catalog_item",
    "catalog item",
    "name",
    "deleted_at IS NULL"
);
entity!(
    LOCATION,
    "location",
    "location",
    "name",
    "deleted_at IS NULL"
);
entity!(
    CAST,
    "cast_member",
    "cast member",
    "person_name",
    "deleted_at IS NULL"
);
entity!(
    CREW,
    "crew_member",
    "crew member",
    "person_name",
    "deleted_at IS NULL"
);
entity!(
    MOODBOARD,
    "moodboard",
    "moodboard",
    "name",
    "deleted_at IS NULL"
);
entity!(
    MOODBOARD_ITEM,
    "moodboard_item",
    "moodboard item",
    "COALESCE(NULLIF(trim(caption),''), NULLIF(substr(trim(body),1,80),''), NULLIF(trim(link_title),''), url, kind)",
    "deleted_at IS NULL"
);
entity!(
    STORYBOARD,
    "storyboard",
    "storyboard",
    "name",
    "deleted_at IS NULL"
);
entity!(
    MARKER,
    "schedule_marker",
    "schedule break",
    "label",
    "deleted_at IS NULL AND day_id IN (SELECT id FROM shooting_day WHERE deleted_at IS NULL)"
);
entity!(
    CALL_SHEET,
    "call_sheet",
    "call sheet",
    "title",
    "deleted_at IS NULL"
);
entity!(SIDE, "side", "sides", "title", "deleted_at IS NULL");
entity!(
    REPORT,
    "production_report",
    "saved report",
    "title",
    "deleted_at IS NULL"
);
entity!(
    BUDGET_LINE,
    "budget_line",
    "budget line",
    "description",
    "deleted_at IS NULL AND budget_id IN (SELECT id FROM budget_snapshot WHERE is_current=1 AND deleted_at IS NULL)"
);
entity!(
    REVIEW_ROUND,
    "review_round",
    "review round",
    "name",
    "deleted_at IS NULL"
);

pub const PRIVATE_NOTE: Entity = Entity {
    table: "private_note",
    what: "private note",
    label: "substr(body,1,80)",
    filter: "deleted_at IS NULL",
    owned: true,
};

/// Maximum candidates considered for name matching.
const MAX_CANDIDATES: usize = 2_000;

fn select(e: &Entity, extra: &str) -> String {
    format!(
        "SELECT id, {}, rev FROM {} WHERE {}{}{}",
        e.label,
        e.table,
        e.filter,
        if e.owned {
            " AND owner_user_id = :owner"
        } else {
            ""
        },
        extra
    )
}

fn rows(c: &Connection, sql: &str, owner: &str, id: Option<&str>) -> AppResult<Vec<Found>> {
    let mut stmt = c.prepare(sql)?;
    let mut named: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
    if sql.contains(":owner") {
        named.push((":owner", &owner));
    }
    let id_val;
    if let Some(i) = id {
        id_val = i.to_string();
        named.push((":id", &id_val));
    }
    let out = stmt
        .query_map(named.as_slice(), |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(out)
}

/// Resolve `reference` (id or name) to exactly one object of kind `e`.
pub fn find(c: &Connection, actor: &Actor, e: &Entity, reference: &str) -> AppResult<Found> {
    let r = reference.trim();
    if r.is_empty() {
        return Err(queries::ambiguous(format!("Which {} do you mean?", e.what)));
    }
    let by_id = rows(c, &select(e, " AND id = :id"), &actor.user_id, Some(r))?;
    if let Some(f) = by_id.into_iter().next() {
        return Ok(f);
    }
    let all = rows(
        c,
        &select(e, &format!(" LIMIT {MAX_CANDIDATES}")),
        &actor.user_id,
        None,
    )?;
    pick(e.what, r, all)
}

/// Choose one of `candidates` by exact (case-insensitive) label, then by a unique
/// partial match; several matches become one clarification question.
pub fn pick(what: &str, reference: &str, candidates: Vec<Found>) -> AppResult<Found> {
    let lower = reference.trim().to_lowercase();
    let exact: Vec<&Found> = candidates
        .iter()
        .filter(|f| f.label.trim().to_lowercase() == lower)
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0].clone());
    }
    let matches: Vec<&Found> = if exact.len() > 1 {
        exact
    } else {
        candidates
            .iter()
            .filter(|f| f.label.to_lowercase().contains(&lower))
            .collect()
    };
    match matches.len() {
        1 => Ok(matches[0].clone()),
        0 => Err(AppError::ai(
            "not_found",
            format!(
                "I couldn't find a {what} called “{}”.",
                queries::truncate_chars(reference.trim(), 80)
            ),
        )),
        n => {
            let names: Vec<String> = matches
                .iter()
                .take(5)
                .map(|f| format!("“{}”", queries::truncate_chars(f.label.trim(), 60)))
                .collect();
            Err(queries::ambiguous(format!(
                "There are {n} {what}s that match “{}”: {}{}. Which one do you mean?",
                queries::truncate_chars(reference.trim(), 60),
                join_or(&names),
                if n > 5 { " (and more)" } else { "" }
            )))
        }
    }
}

/// Resolve an optional reference; `None`/blank stays `None`.
pub fn find_opt(
    c: &Connection,
    actor: &Actor,
    e: &Entity,
    reference: Option<&str>,
) -> AppResult<Option<Found>> {
    match reference.map(str::trim).filter(|r| !r.is_empty()) {
        Some(r) => find(c, actor, e, r).map(Some),
        None => Ok(None),
    }
}

/// Current value of one column of a row (for "old → new" previews).
pub fn column(
    c: &Connection,
    table: &'static str,
    col: &'static str,
    id: &str,
) -> AppResult<String> {
    let v: Option<Option<String>> = c
        .query_row(
            &format!("SELECT CAST({col} AS TEXT) FROM {table} WHERE id=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(v.flatten().unwrap_or_default())
}

// ------------------------------------------------------------ screenplay scenes

/// The draft screenplay tools work on: the named draft, else the scope's draft,
/// else the current draft.
pub fn draft(
    c: &Connection,
    scope: &ResolvedScope,
    reference: Option<&str>,
) -> AppResult<DraftRef> {
    queries::resolve_draft(c, reference, scope.draft.as_ref())
}

/// The draft production tools work on: the named draft, else the active
/// Production Source, else the scope/current draft.
pub fn production_draft(
    c: &Connection,
    scope: &ResolvedScope,
    reference: Option<&str>,
) -> AppResult<DraftRef> {
    if reference.is_some_and(|r| !r.trim().is_empty()) {
        return draft(c, scope, reference);
    }
    if let Some((_, d)) = queries::production_source_draft(c)?
        && let Some(dr) = queries::draft_by_id(c, &d)?
    {
        return Ok(dr);
    }
    draft(c, scope, None)
}

/// Scene `number` of `d` (display numbering follows position).
pub fn scene(c: &Connection, d: &DraftRef, number: u32) -> AppResult<SceneRow> {
    let scenes = queries::scenes(c, &d.id)?;
    let n = scenes.len();
    scenes
        .into_iter()
        .find(|s| s.number == number as usize)
        .ok_or_else(|| {
            AppError::ai(
                "not_found",
                format!("{} has {n} scenes; there is no Scene {number}.", d.label()),
            )
        })
}

/// A scene given either by number (in the tool's draft) or by the scene in scope.
pub fn scene_or_scope(
    c: &Connection,
    scope: &ResolvedScope,
    d: &DraftRef,
    number: Option<u32>,
) -> AppResult<SceneRow> {
    match number {
        Some(n) => scene(c, d, n),
        None => match &scope.scene {
            Some(s) if scope.draft.as_ref().is_some_and(|sd| sd.id == d.id) => Ok(s.clone()),
            _ => Err(queries::ambiguous(
                "Which scene do you mean? Tell me its number or open it first.",
            )),
        },
    }
}

// ------------------------------------------------------------ schedule

/// The active shooting schedule (id, name) of the Production Source.
pub fn schedule(c: &Connection) -> AppResult<(String, String)> {
    c.query_row(
        "SELECT sc.id, sc.name FROM shooting_schedule sc JOIN production_source p ON p.id = sc.source_id
         WHERE sc.deleted_at IS NULL AND p.active = 1 ORDER BY sc.created_at DESC LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| {
        AppError::ai(
            "not_found",
            "There is no shooting schedule yet. Create one in Schedule first.",
        )
    })
}

/// A shooting day of the active schedule: "3", "Day 3", or a date "YYYY-MM-DD", or its id.
pub fn day(c: &Connection, reference: &str) -> AppResult<Found> {
    let (schedule_id, _) = schedule(c)?;
    let mut stmt = c.prepare(
        "SELECT id, shoot_date, rev FROM shooting_day WHERE schedule_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let days: Vec<(String, Option<String>, i64)> = stmt
        .query_map([&schedule_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let r = reference.trim();
    let label = |i: usize, date: &Option<String>| match date {
        Some(d) => format!("Day {} ({d})", i + 1),
        None => format!("Day {}", i + 1),
    };
    for (i, (id, date, rev)) in days.iter().enumerate() {
        if id == r || date.as_deref() == Some(r) {
            return Ok(Found {
                id: id.clone(),
                label: label(i, date),
                rev: *rev,
            });
        }
    }
    let digits: String = r
        .to_lowercase()
        .trim_start_matches("day")
        .trim()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if let Ok(n) = digits.parse::<usize>()
        && n >= 1
        && let Some((id, date, rev)) = days.get(n - 1)
    {
        return Ok(Found {
            id: id.clone(),
            label: label(n - 1, date),
            rev: *rev,
        });
    }
    Err(AppError::ai(
        "not_found",
        format!(
            "The schedule has {} days; I couldn't find “{}”. Use a day number like “Day 3” or a date like 2026-10-14.",
            days.len(),
            queries::truncate_chars(r, 40)
        ),
    ))
}

/// Schedule strip of a scene (by scene identity) in the active schedule.
pub fn strip_for_scene(c: &Connection, scene: &SceneRow) -> AppResult<Found> {
    let (schedule_id, _) = schedule(c)?;
    c.query_row(
        "SELECT id, source_heading, rev FROM schedule_strip WHERE schedule_id=?1 AND scene_lineage_id=?2
           AND deleted_at IS NULL AND archived=0 LIMIT 1",
        params![schedule_id, scene.lineage_id],
        |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| {
        AppError::ai(
            "not_found",
            format!(
                "{} isn't on the shooting schedule. Reconcile the schedule with the Production Source first.",
                scene.label()
            ),
        )
    })
}

// ------------------------------------------------------------ visual planning

/// Shot `number` (1-based, shot-list order) of a scene, by scene identity.
pub fn shot(c: &Connection, scene: &SceneRow, number: u32) -> AppResult<Found> {
    let mut stmt = c.prepare(
        "SELECT id, description, rev FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let shots: Vec<Found> = stmt
        .query_map([&scene.lineage_id], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let n = shots.len();
    shots
        .into_iter()
        .nth((number as usize).saturating_sub(1))
        .filter(|_| number >= 1)
        .ok_or_else(|| {
            AppError::ai(
                "not_found",
                format!(
                    "{} has {n} shots; there is no Shot {number}.",
                    scene.label()
                ),
            )
        })
}

/// Panel `number` (1-based) of a storyboard.
pub fn panel(c: &Connection, storyboard: &Found, number: u32) -> AppResult<Found> {
    let mut stmt = c.prepare(
        "SELECT id, description, rev FROM storyboard_panel WHERE storyboard_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let panels: Vec<Found> = stmt
        .query_map([&storyboard.id], |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let n = panels.len();
    panels
        .into_iter()
        .nth((number as usize).saturating_sub(1))
        .filter(|_| number >= 1)
        .map(|mut p| {
            p.label = format!("Panel {number}");
            p
        })
        .ok_or_else(|| {
            AppError::ai(
                "not_found",
                format!(
                    "“{}” has {n} panels; there is no Panel {number}.",
                    storyboard.label
                ),
            )
        })
}

/// The current budget (id, currency).
pub fn budget(c: &Connection) -> AppResult<Found> {
    c.query_row(
        "SELECT id, currency, rev FROM budget_snapshot WHERE is_current=1 AND deleted_at IS NULL LIMIT 1",
        [],
        |r| {
            Ok(Found {
                id: r.get(0)?,
                label: r.get(1)?,
                rev: r.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| {
        AppError::ai(
            "not_found",
            "This project doesn't have a budget yet. Create one in Budget first.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(label: &str) -> Found {
        Found {
            id: label.to_lowercase(),
            label: label.into(),
            rev: 1,
        }
    }

    #[test]
    fn exact_match_wins_over_partial() {
        let got = pick("location", "station", vec![f("Station"), f("Old Station")]).unwrap();
        assert_eq!(got.label, "Station");
    }

    #[test]
    fn several_partial_matches_ask_one_question() {
        let e = pick("location", "stat", vec![f("Station"), f("Old Station")]).unwrap_err();
        assert_eq!(e.code_str(), "ai.ambiguous");
        assert!(
            e.message.contains("“Station” or “Old Station”"),
            "{}",
            e.message
        );
        let e = pick("location", "harbour", vec![f("Station")]).unwrap_err();
        assert_eq!(e.code_str(), "ai.not_found");
    }
}
