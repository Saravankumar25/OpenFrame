//! Bounded, domain-named read tools (agentic spec §27). Each tool answers from
//! canonical data through the caller's read connection with the requesting
//! user's permissions, returns at most a page of results, and never mutates
//! anything. Private notes are only ever read for their owner.

use std::collections::BTreeMap;

use openframe_domain::{Actor, AppError, AppResult};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::super::queries::{self, join_and};
use super::super::scope::ResolvedScope;
use super::super::tools::{ToolCtx, ToolOutput, nav_from_json, plural, scene_nav};
use super::super::types::*;
use super::resolve::{self as rv, Found};
use super::schema as sc;
use crate::core::AppCore;

pub use super::reads_production::*;

/// A read/compute tool offered to the model.
pub struct ReadSpec {
    pub name: &'static str,
    pub class: OperationClass,
    pub description: &'static str,
    pub schema: fn() -> Value,
    /// Registry queries whose facts this tool exposes (coverage matrix).
    pub covers: &'static [&'static str],
    pub run: fn(&ToolCtx<'_>, &Value) -> AppResult<ToolOutput>,
}

macro_rules! read {
    ($name:literal, $class:ident, $desc:literal, $schema:expr, [$($cov:literal),*], $run:expr) => {
        ReadSpec {
            name: $name,
            class: OperationClass::$class,
            description: $desc,
            schema: $schema,
            covers: &[$($cov),*],
            run: $run,
        }
    };
}

pub const READS: &[ReadSpec] = &[
    read!(
        "project_settings",
        Read,
        "Project settings (title, type, status, logline, genre, language, creator) and the people on the project with their roles.",
        sc::none,
        ["project.get_settings", "project.members"],
        project_settings
    ),
    read!(
        "recently_deleted",
        Read,
        "Items in Recently Deleted that can still be restored.",
        limit_schema,
        ["project.deleted_items", "trash.list"],
        recently_deleted
    ),
    read!(
        "recent_activity",
        Read,
        "Recent project activity: who changed what, and when.",
        limit_schema,
        ["history.activity"],
        recent_activity
    ),
    read!(
        "project_files",
        Read,
        "Project Files: folders and files with their notes (optionally one folder).",
        s_files,
        ["files.list"],
        project_files
    ),
    read!(
        "project_notes",
        Search,
        "Project Notes: list them (optionally matching words) or read one note in full.",
        s_notes,
        ["notes.list", "notes.get"],
        project_notes
    ),
    read!(
        "project_tasks",
        Read,
        "Project tasks with status, due date and related item.",
        s_tasks,
        ["tasks.list"],
        project_tasks
    ),
    read!(
        "comments",
        Read,
        "Comment threads (open or all), optionally for one screenplay scene.",
        s_comments,
        ["comment.list"],
        comments
    ),
    read!(
        "templates",
        Read,
        "Templates available in this project (project templates and built-in ones).",
        s_templates,
        ["templates.list", "templates.get"],
        templates
    ),
    read!(
        "package_history",
        Read,
        "Package history: packages exported/imported, received packages and the review queue (metadata only).",
        sc::none,
        [
            "packages.log",
            "packages.sessions",
            "packages.review_queue",
            "packages.records"
        ],
        package_history
    ),
    read!(
        "idea_vault",
        Search,
        "Idea Vault items (filter by words, tag, folder, collection, type or pinned), or one item in full.",
        s_vault,
        ["vault.list", "vault.overview", "vault.get"],
        idea_vault
    ),
    read!(
        "story_outline",
        Read,
        "Story Board outline: acts, sequences, beats and Scene Cards in order (optionally one act), with their screenplay scenes.",
        s_outline,
        ["story.board", "story.build_preview", "story.order_preview"],
        story_outline
    ),
    read!(
        "story_card",
        Read,
        "One Scene Card in detail: description, heading, notes, place, linked characters and screenplay scene.",
        s_card,
        ["story.card"],
        story_card
    ),
    read!(
        "story_characters",
        Read,
        "Character directory, or one character in detail (description, notes, relationships, where they appear, cast).",
        s_characters,
        [
            "story.characters",
            "story.character",
            "story.character_usage",
            "story.relationships"
        ],
        story_characters
    ),
    read!(
        "story_series",
        Read,
        "Seasons and episodes of a series.",
        sc::none,
        ["story.series"],
        story_series
    ),
    read!(
        "story_timeline",
        Read,
        "Story timeline: the Story Day and time note of each screenplay scene.",
        s_draft,
        ["story.timeline"],
        story_timeline
    ),
    read!(
        "screenplay_scenes",
        Read,
        "Numbered scene list of a draft with headings, synopses and Story Days (paged).",
        s_scene_list,
        ["screenplay.document"],
        screenplay_scenes
    ),
    read!(
        "screenplay_scene",
        Read,
        "One screenplay scene in detail: heading, synopsis, notes, numbered lines of text, speaking characters, and what is linked to it across modules.",
        s_scene,
        [
            "screenplay.scene_hub",
            "screenplay.story_reference",
            "visual.scene_characters"
        ],
        screenplay_scene
    ),
    read!(
        "draft_status",
        Read,
        "A draft's status: locked or not, revision, note, open comments, history points and review rounds.",
        s_draft,
        [
            "screenplay.lock_summary",
            "screenplay.history_points",
            "screenplay.review_rounds"
        ],
        draft_status
    ),
    read!(
        "character_cues",
        Read,
        "Character cues in a draft and the Character records they are linked to.",
        s_draft,
        ["screenplay.characters"],
        character_cues
    ),
    read!(
        "breakdown_scene",
        Read,
        "Breakdown of one scene (Production Source): elements by category with their state, completion and review flags.",
        s_scene,
        ["breakdown.scene", "breakdown.scene_changes"],
        breakdown_scene
    ),
    read!(
        "production_catalog",
        Search,
        "Production Catalog items (by category or words), or one item with aliases and the scenes using it.",
        s_catalog,
        ["catalog.list", "catalog.get", "catalog.find_matches"],
        production_catalog
    ),
    read!(
        "production_locations",
        Search,
        "Production locations (by status or words), or one location in detail with its scenes.",
        s_locations,
        ["locations.list", "locations.get"],
        production_locations
    ),
    read!(
        "cast_and_crew",
        Read,
        "Cast (performers and the characters they play) and crew (roles, departments).",
        s_people,
        ["cast.list", "crew.list"],
        cast_and_crew
    ),
    read!(
        "production_source",
        Read,
        "The Production Source draft, other drafts, and what updating to the newest draft would change.",
        sc::none,
        [
            "production.source",
            "production.drafts",
            "production.preview_update"
        ],
        production_source
    ),
    read!(
        "visual_planning",
        Compute,
        "Visual planning coverage per scene: shots, storyboards, moodboards and scenes needing review.",
        sc::none,
        ["visual.scenes"],
        visual_planning
    ),
    read!(
        "moodboards",
        Read,
        "Moodboards, or one moodboard's notes, images, links and captions.",
        s_moodboard,
        ["moodboard.list", "moodboard.get"],
        moodboards
    ),
    read!(
        "storyboards",
        Read,
        "Storyboards, or one storyboard's panels with framing, movement, angle, sound and linked shots.",
        s_storyboard,
        ["storyboard.list", "storyboard.get"],
        storyboards
    ),
    read!(
        "shot_list",
        Read,
        "Shot list of one scene (or counts for all scenes) with size, movement, angle, lens and characters.",
        s_shots,
        ["shot.list", "shot.get"],
        shot_list
    ),
    read!(
        "schedule_overview",
        Read,
        "Shooting schedule: status, days with scene counts and pages, unscheduled scenes, and simple grouping hints.",
        sc::none,
        ["schedule.get", "schedule.suggestions"],
        schedule_overview
    ),
    read!(
        "schedule_day",
        Read,
        "One shooting day: date, notes, scenes in order with pages and estimates, breaks, and cast needed.",
        s_day,
        ["schedule.daily", "sides.preview"],
        schedule_day
    ),
    read!(
        "call_sheets",
        Read,
        "Call sheets, or one call sheet's content (crew call, notes, practical details, scenes).",
        s_call_sheet,
        [
            "callsheets.list",
            "callsheets.get",
            "callsheets.refresh_preview"
        ],
        call_sheets
    ),
    read!(
        "sides_and_reports",
        Read,
        "Saved sides and saved production reports.",
        sc::none,
        ["sides.list", "sides.get", "reports.list", "reports.get"],
        sides_and_reports
    ),
    read!(
        "production_report",
        Compute,
        "Generate a production report now (scene, location, cast/scene, prop, schedule, breakdown completeness).",
        s_report,
        ["reports.generate"],
        production_report
    ),
    read!(
        "budget_summary",
        Read,
        "Current budget: currency, planned total, lines by category, totals and saved snapshots.",
        sc::none,
        ["budget.get", "budget.get_snapshot"],
        budget_summary
    ),
];

pub fn spec(name: &str) -> Option<&'static ReadSpec> {
    READS.iter().find(|r| r.name == name)
}

// ------------------------------------------------------------------ shared

fn limit_schema() -> Value {
    sc::obj(&[("limit", sc::int(1, 50))], &[])
}
fn s_files() -> Value {
    sc::obj(&[("folder", sc::reference())], &[])
}
fn s_notes() -> Value {
    sc::obj(&[("note", sc::reference()), ("query", sc::s(200))], &[])
}
fn s_tasks() -> Value {
    sc::obj(&[("status", sc::en(&["Open", "Done", "all"]))], &[])
}
fn s_comments() -> Value {
    sc::obj(
        &[
            ("status", sc::en(&["open", "all"])),
            ("sceneNumber", sc::scene_number()),
            ("draft", sc::s(120)),
        ],
        &[],
    )
}
fn s_templates() -> Value {
    sc::obj(&[("type", sc::s(40))], &[])
}
fn s_vault() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            ("query", sc::s(200)),
            ("tag", sc::s(40)),
            ("folder", sc::reference()),
            ("collection", sc::reference()),
            ("type", sc::s(20)),
            ("pinned", sc::boolean()),
            ("limit", sc::int(1, 50)),
        ],
        &[],
    )
}
fn s_outline() -> Value {
    sc::obj(
        &[("act", sc::reference()), ("episode", sc::reference())],
        &[],
    )
}
fn s_card() -> Value {
    sc::obj(&[("card", sc::reference())], &["card"])
}
fn s_characters() -> Value {
    sc::obj(
        &[
            ("character", sc::reference()),
            ("includeArchived", sc::boolean()),
        ],
        &[],
    )
}
pub(super) fn s_draft() -> Value {
    sc::obj(&[("draft", sc::s(120))], &[])
}
fn s_scene_list() -> Value {
    sc::obj(
        &[
            ("draft", sc::s(120)),
            ("fromScene", sc::scene_number()),
            ("limit", sc::int(1, 80)),
        ],
        &[],
    )
}
pub(super) fn s_scene() -> Value {
    sc::obj(
        &[("sceneNumber", sc::scene_number()), ("draft", sc::s(120))],
        &[],
    )
}

pub(super) fn a_str(a: &Value, k: &str) -> Option<String> {
    a.get(k)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
pub(super) fn a_u32(a: &Value, k: &str) -> Option<u32> {
    a.get(k)
        .and_then(|v| v.as_u64())
        .and_then(|n| u32::try_from(n).ok())
}
pub(super) fn a_bool(a: &Value, k: &str) -> bool {
    a.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}
pub(super) fn a_limit(a: &Value, default: usize) -> usize {
    a.get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(default)
}

pub(super) fn item(
    label: impl Into<String>,
    detail: Option<String>,
    nav: Option<NavTarget>,
) -> ResultItem {
    ResultItem {
        label: label.into(),
        detail: detail.filter(|d| !d.trim().is_empty()),
        nav,
    }
}

pub(super) fn clip(s: &str, n: usize) -> String {
    queries::truncate_chars(s.trim(), n)
}

pub(super) fn when(ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp(ms / 1000)
        .map(|t| t.date().to_string())
        .unwrap_or_default()
}

pub(super) fn found(c: &ToolCtx<'_>, e: &rv::Entity, reference: &str) -> AppResult<Found> {
    rv::find(c.conn, c.actor, e, reference)
}

// ------------------------------------------------------------------ retrieval

/// Keyword retrieval with the Global Search privacy rule, used when the hybrid
/// retrieval service is not reachable (spec §33 fallback: FTS + SQL).
pub fn retrieve_lexical(ctx: &ToolCtx<'_>, args: &Value) -> AppResult<ToolOutput> {
    let q = a_str(args, "query").ok_or_else(|| queries::ambiguous("What should I look for?"))?;
    let limit = a_limit(args, 8).min(24);
    let hits = lexical_hits(ctx.conn, ctx.actor, &q, limit)?;
    let mut o = ToolOutput::exact(if hits.is_empty() {
        format!("Nothing in this project matches “{}”.", clip(&q, 80))
    } else {
        format!(
            "{} related to “{}”.",
            plural(hits.len(), "passage", "passages"),
            clip(&q, 80)
        )
    });
    for (etype, title, context, body, nav) in &hits {
        o.items.push(item(
            if title.is_empty() {
                etype.clone()
            } else {
                title.clone()
            },
            Some(format!("{context}: {}", clip(body, 240))),
            nav_from_json(nav),
        ));
        o.provenance.push(Provenance::new(
            if context.is_empty() {
                "Project"
            } else {
                context.as_str()
            },
            if title.is_empty() {
                etype.clone()
            } else {
                clip(title, 80)
            },
        ));
    }
    o.structured = Some(json!({"route": "Lexical", "degraded": true, "indexState": "Unavailable"}));
    Ok(o.detail("Keyword retrieval (project context is still being prepared)."))
}

/// (entity type, title, context, body, nav) of the best keyword matches visible to `actor`.
fn lexical_hits(
    c: &Connection,
    actor: &Actor,
    text: &str,
    limit: usize,
) -> AppResult<Vec<(String, String, String, String, Value)>> {
    let terms: Vec<String> = text
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|t| t.chars().count() >= 3)
        .take(10)
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    // OR-matching ranked by BM25: a question's words rarely all appear together.
    let q = terms.join(" OR ");
    let mut stmt = c.prepare(
        "SELECT d.entity_type, d.title, d.context, d.body, d.nav_json FROM search_fts f JOIN search_doc d ON d.rowid=f.rowid
         WHERE search_fts MATCH ?1 AND (d.owner_user_id IS NULL OR d.owner_user_id=?2)
         ORDER BY bm25(search_fts) LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![q, actor.user_id, limit as i64], |r| {
            let nav: String = r.get(4)?;
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                serde_json::from_str(&nav).unwrap_or(Value::Null),
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Hybrid retrieval through the retrieval service (contract C2: FTS5 + semantic
/// vectors + context graph, with its own privacy/permission filters and
/// fallbacks). Must be called with no store lock held (the service reads the
/// project itself).
pub fn retrieve_hybrid(
    core: &AppCore,
    actor: &Actor,
    scope: &ResolvedScope,
    args: &Value,
) -> AppResult<Option<ToolOutput>> {
    use super::super::retrieval::{RetrieveRequest, retrieve};
    let q = a_str(args, "query").ok_or_else(|| queries::ambiguous("What should I look for?"))?;
    let packet = retrieve(
        core,
        actor,
        &RetrieveRequest {
            query: q.clone(),
            scope: Some(scope.clone()),
            limit: a_limit(args, 8).min(24),
            route: None,
        },
    )?;
    let mut o = ToolOutput::exact(if packet.items.is_empty() {
        format!("Nothing in this project matches “{}”.", clip(&q, 80))
    } else {
        format!(
            "{} related to “{}”.",
            plural(packet.items.len(), "passage", "passages"),
            clip(&q, 80)
        )
    });
    for (i, it) in packet.items.iter().enumerate() {
        let nav = packet.sources.get(i).and_then(|s| nav_from_json(&s.nav));
        o.items
            .push(item(it.source.clone(), Some(clip(&it.text, 240)), nav));
    }
    o.provenance = packet.provenance.clone();
    if let Some(n) = &packet.notice {
        o = o.detail(n.clone());
    }
    o.structured = Some(json!({
        "route": format!("{:?}", packet.route),
        "degraded": packet.degraded,
        "indexState": format!("{:?}", packet.index_state),
    }));
    Ok(Some(o))
}

// ------------------------------------------------------------------ project

fn project_settings(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let (title, ptype, status, language, genre, creator, logline): (String, String, String, Option<String>, Option<String>, Option<String>, Option<String>) =
        c.query_row(
            "SELECT title, project_type, status, language, genre, creator, logline FROM project LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )?;
    let mut o = ToolOutput::exact(format!("{title} is a {ptype}; status {status}."));
    for (label, v) in [
        ("Logline", logline),
        ("Genre", genre),
        ("Language", language),
        ("Creator", creator),
    ] {
        if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
            o = o.detail(format!("{label}: {}", clip(&v, 400)));
        }
    }
    let mut stmt = c.prepare(
        "SELECT display_name, role, professional_label FROM project_member WHERE deleted_at IS NULL ORDER BY display_name",
    )?;
    let members: Vec<(String, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (name, role, label) in members.iter().take(50) {
        o.items.push(item(
            name.clone(),
            Some(match label {
                Some(l) if !l.is_empty() => format!("{role} · {l}"),
                _ => role.clone(),
            }),
            None,
        ));
    }
    o.nav = Some(NavTarget::to("settings"));
    Ok(o.prov("Scope", "Project settings"))
}

fn recently_deleted(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let limit = a_limit(a, 30) as i64;
    let mut stmt = ctx.conn.prepare(
        "SELECT COALESCE(title, object_type), object_type, deleted_at, restore_until FROM deleted_item
         WHERE table_name <> 'private_note' OR object_id IN (SELECT id FROM private_note WHERE owner_user_id=?1)
         ORDER BY deleted_at DESC LIMIT ?2",
    )?;
    let rows: Vec<(String, String, i64, Option<i64>)> = stmt
        .query_map(params![ctx.actor.user_id, limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(if rows.is_empty() {
        "Recently Deleted is empty.".to_string()
    } else {
        format!(
            "{} in Recently Deleted (newest first).",
            plural(rows.len(), "item", "items")
        )
    });
    for (title, kind, at, until) in rows {
        o.items.push(item(
            title,
            Some(format!(
                "{} · deleted {}{}",
                kind.replace('_', " "),
                when(at),
                until
                    .map(|u| format!(" · restorable until {}", when(u)))
                    .unwrap_or_default()
            )),
            Some(NavTarget::to("trash")),
        ));
    }
    Ok(o.prov("Scope", "Recently Deleted"))
}

fn recent_activity(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let limit = a_limit(a, 20) as i64;
    let mut stmt = ctx.conn.prepare(
        "SELECT at, COALESCE(actor_name, ''), summary, origin FROM sys_activity ORDER BY at DESC LIMIT ?1",
    )?;
    let rows: Vec<(i64, String, String, String)> = stmt
        .query_map([limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "The last {} in this project.",
        plural(rows.len(), "change", "changes")
    ));
    for (at, who, summary, origin) in rows {
        o.items.push(item(
            clip(&summary, 200),
            Some(format!(
                "{}{} · {}",
                if who.is_empty() {
                    "Someone".to_string()
                } else {
                    who
                },
                if origin == "ai" || origin.starts_with("ai") {
                    " (AI-assisted)"
                } else {
                    ""
                },
                when(at)
            )),
            None,
        ));
    }
    o.nav = Some(NavTarget::to("activity"));
    Ok(o.prov("Scope", "Project activity"))
}

fn project_files(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let folder = match a_str(a, "folder") {
        Some(r) => Some(found(ctx, &rv::FOLDER, &r)?),
        None => None,
    };
    let mut stmt = c.prepare(
        "SELECT f.display_name, COALESCE(fo.name, ''), COALESCE(f.notes, ''), a.media_type FROM project_file f
         JOIN asset a ON a.id = f.asset_id LEFT JOIN project_file_folder fo ON fo.id = f.folder_id AND fo.deleted_at IS NULL
         WHERE f.deleted_at IS NULL AND (?1 IS NULL OR f.folder_id = ?1) ORDER BY fo.name, f.position LIMIT 200",
    )?;
    let files: Vec<(String, String, String, String)> = stmt
        .query_map([folder.as_ref().map(|f| f.id.clone())], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let folders = queries::count(
        c,
        "SELECT count(*) FROM project_file_folder WHERE deleted_at IS NULL",
        [],
    )?;
    let mut o = ToolOutput::exact(match &folder {
        Some(f) => format!(
            "Folder “{}” has {}.",
            f.label,
            plural(files.len(), "file", "files")
        ),
        None => format!(
            "Project Files has {} in {}.",
            plural(files.len(), "file", "files"),
            plural(folders as usize, "folder", "folders")
        ),
    });
    for (name, fo, notes, media) in files {
        let mut d = if fo.is_empty() {
            "Top level".to_string()
        } else {
            format!("Folder {fo}")
        };
        d.push_str(&format!(" · {media}"));
        if !notes.trim().is_empty() {
            d.push_str(&format!(" · {}", clip(&notes, 120)));
        }
        o.items
            .push(item(name, Some(d), Some(NavTarget::to("files"))));
    }
    Ok(o.prov("Scope", "Project Files"))
}

fn project_notes(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "note") {
        let n = found(ctx, &rv::NOTE, &r)?;
        let (title, body, pinned, updated): (Option<String>, String, bool, i64) = c.query_row(
            "SELECT title, body, pinned, updated_at FROM project_note WHERE id=?1",
            [&n.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let mut o = ToolOutput::exact(format!(
            "{}{}",
            title
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "Untitled note".into()),
            if pinned { " (pinned)" } else { "" }
        ))
        .detail(format!("Last edited {}", when(updated)));
        o.items.push(item(
            "Text",
            Some(clip(&body, 3500)),
            Some(NavTarget::to("notes").with("noteId", &n.id)),
        ));
        return Ok(o.prov("Project Note", n.label));
    }
    let q = a_str(a, "query").map(|q| format!("%{}%", q.replace('%', "")));
    let mut stmt = c.prepare(
        "SELECT id, COALESCE(NULLIF(title,''), substr(body,1,60)), substr(body,1,200), pinned FROM project_note
         WHERE deleted_at IS NULL AND (?1 IS NULL OR title LIKE ?1 OR body LIKE ?1) ORDER BY pinned DESC, updated_at DESC LIMIT 50",
    )?;
    let rows: Vec<(String, String, String, bool)> = stmt
        .query_map([q], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{} found.",
        plural(rows.len(), "Project Note", "Project Notes")
    ));
    for (id, title, excerpt, pinned) in rows {
        o.items.push(item(
            format!("{title}{}", if pinned { " (pinned)" } else { "" }),
            Some(clip(&excerpt, 200)),
            Some(NavTarget::to("notes").with("noteId", &id)),
        ));
    }
    Ok(o.prov("Scope", "Project Notes"))
}

fn project_tasks(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let status = a_str(a, "status").unwrap_or_else(|| "Open".into());
    let filter = if status == "all" {
        None
    } else {
        Some(status.clone())
    };
    let mut stmt = ctx.conn.prepare(
        "SELECT t.id, t.title, t.status, t.due_at, COALESCE(m.display_name, ''), COALESCE(t.notes, ''), t.target_type, t.target_id
         FROM task t LEFT JOIN project_member m ON m.user_id = t.owner_user_id AND m.deleted_at IS NULL
         WHERE t.deleted_at IS NULL AND (?1 IS NULL OR t.status = ?1) ORDER BY t.status, t.due_at IS NULL, t.due_at, t.position LIMIT 100",
    )?;
    let rows: Vec<(
        String,
        String,
        String,
        Option<i64>,
        String,
        String,
        Option<String>,
        Option<String>,
    )> = stmt
        .query_map([filter], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{}{}.",
        plural(rows.len(), "task", "tasks"),
        if status == "all" {
            String::new()
        } else {
            format!(" ({})", status.to_lowercase())
        }
    ));
    for (id, title, st, due, owner, notes, tt, tid) in rows {
        let mut d = vec![st];
        if let Some(due) = due {
            d.push(format!("due {}", when(due)));
        }
        if !owner.is_empty() {
            d.push(format!("owner {owner}"));
        }
        if let (Some(tt), Some(tid)) = (tt, tid) {
            let title: Option<String> = ctx
                .conn
                .query_row(
                    "SELECT title FROM search_doc WHERE source_table=?1 AND entity_id=?2 AND owner_user_id IS NULL",
                    params![tt, tid],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(t) = title {
                d.push(format!("about {}", clip(&t, 60)));
            }
        }
        if !notes.trim().is_empty() {
            d.push(clip(&notes, 120));
        }
        o.items.push(item(
            title,
            Some(d.join(" · ")),
            Some(NavTarget::to("notes").with("taskId", &id)),
        ));
    }
    Ok(o.prov("Scope", "Tasks"))
}

fn comments(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let open_only = a_str(a, "status").as_deref() != Some("all");
    let scene = match a_u32(a, "sceneNumber") {
        Some(n) => {
            let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
            Some(rv::scene(c, &d, n)?)
        }
        None => None,
    };
    let mut stmt = c.prepare(
        "SELECT id, body, status, author_name, target_type, created_at,
                (SELECT count(*) FROM comment r WHERE r.parent_id = c.id AND r.deleted_at IS NULL)
         FROM comment c WHERE deleted_at IS NULL AND parent_id IS NULL
           AND (?1 = 0 OR status <> 'Resolved') AND (?2 IS NULL OR scene_id = ?2 OR target_id = ?2)
         ORDER BY created_at DESC LIMIT 60",
    )?;
    let rows: Vec<(String, String, String, String, String, i64, i64)> = stmt
        .query_map(
            params![open_only as i64, scene.as_ref().map(|s| s.id.clone())],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{}{}{}.",
        plural(rows.len(), "comment thread", "comment threads"),
        if open_only { " still open" } else { "" },
        scene
            .as_ref()
            .map(|s| format!(" on {}", s.label()))
            .unwrap_or_default()
    ));
    for (_, body, status, author, target, at, replies) in rows {
        o.items.push(item(
            clip(&body, 200),
            Some(format!(
                "{author} · {status} · on {} · {}{}",
                target.replace('_', " "),
                when(at),
                if replies > 0 {
                    format!(" · {}", plural(replies as usize, "reply", "replies"))
                } else {
                    String::new()
                }
            )),
            None,
        ));
    }
    Ok(o.prov("Scope", "Comments"))
}

fn templates(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let t = a_str(a, "type");
    let mut stmt = ctx.conn.prepare(
        "SELECT name, template_type, content_json FROM template WHERE deleted_at IS NULL AND (?1 IS NULL OR template_type=?1) ORDER BY template_type, name LIMIT 100",
    )?;
    let rows: Vec<(String, String, String)> = stmt
        .query_map([t.clone()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{} in this project.",
        plural(rows.len(), "project template", "project templates")
    ));
    for (name, kind, content) in rows {
        let desc = serde_json::from_str::<Value>(&content).ok().and_then(|v| {
            v.get("description")
                .and_then(|d| d.as_str())
                .map(|d| clip(d, 160))
        });
        o.items.push(item(
            name,
            Some(format!(
                "{}{}",
                kind.replace('_', " "),
                desc.map(|d| format!(" · {d}")).unwrap_or_default()
            )),
            Some(NavTarget::to("settings").with_sub("templates")),
        ));
    }
    let types: Vec<String> = crate::modules::notes::templates::TEMPLATE_TYPES
        .iter()
        .map(|(_, l)| (*l).to_string())
        .collect();
    o = o.detail(format!("Template types: {}.", join_and(&types)));
    o = o.detail("Built-in and personal templates are listed in Settings › Templates.");
    Ok(o.prov("Scope", "Templates"))
}

fn package_history(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let mut stmt = c.prepare(
        "SELECT direction, package_type, file_name, summary, at, COALESCE(actor_name,'') FROM sys_package_log ORDER BY at DESC LIMIT 20",
    )?;
    let log: Vec<(String, String, String, String, i64, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let sessions = queries::count(
        c,
        "SELECT count(*) FROM sys_import_session WHERE status IN ('Previewed','PendingReview')",
        [],
    )?;
    let queue = queries::count(
        c,
        "SELECT count(*) FROM review_queue_item WHERE deleted_at IS NULL AND status='Pending'",
        [],
    )?;
    let records = queries::count(
        c,
        "SELECT count(*) FROM exchange_review_record WHERE deleted_at IS NULL",
        [],
    )?;
    let mut o = ToolOutput::exact(format!(
        "{} in the package history; {} waiting for review; {} in the review queue; {}.",
        plural(log.len(), "entry", "entries"),
        plural(sessions as usize, "received package", "received packages"),
        plural(queue as usize, "comment", "comments"),
        plural(
            records as usize,
            "saved review record",
            "saved review records"
        )
    ));
    for (dir, kind, file, summary, at, who) in log {
        o.items.push(item(
            format!(
                "{} {kind} package",
                if dir == "export" {
                    "Exported"
                } else {
                    "Imported"
                }
            ),
            Some(format!(
                "{file} · {} · {who} · {}",
                clip(&summary, 120),
                when(at)
            )),
            None,
        ));
    }
    Ok(o.detail(
        "Opening, reviewing and applying packages stays in Packages (the assistant can't do it).",
    )
    .prov("Scope", "Package history"))
}

// ------------------------------------------------------------------ Idea Vault

fn idea_vault(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "item") {
        let it = found(ctx, &rv::VAULT_ITEM, &r)?;
        let (kind, title, body, caption, url, source, pinned, folder): (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, bool, Option<String>) =
            c.query_row(
                "SELECT i.item_type, i.title, i.body, i.caption, i.url, i.source_text, i.pinned, f.name FROM vault_item i
                 LEFT JOIN vault_folder f ON f.id = i.folder_id AND f.deleted_at IS NULL WHERE i.id=?1",
                [&it.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
            )?;
        let mut o = ToolOutput::exact(format!(
            "{} ({kind}{})",
            title
                .clone()
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| it.label.clone()),
            if pinned { ", pinned" } else { "" }
        ));
        for (label, v) in [
            ("Text", body),
            ("Caption", caption),
            ("Link", url),
            ("Source", source),
            ("Folder", folder),
        ] {
            if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
                o.items.push(item(label, Some(clip(&v, 3000)), None));
            }
        }
        let tags: Vec<String> = c
            .prepare("SELECT tag FROM vault_item_tag WHERE item_id=?1 ORDER BY tag")?
            .query_map([&it.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        if !tags.is_empty() {
            o = o.detail(format!("Tags: {}", tags.join(", ")));
        }
        let cols: Vec<String> = c
            .prepare("SELECT co.name FROM vault_item_collection ic JOIN vault_collection co ON co.id = ic.collection_id AND co.deleted_at IS NULL WHERE ic.item_id=?1")?
            .query_map([&it.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        if !cols.is_empty() {
            o = o.detail(format!("Collections: {}", cols.join(", ")));
        }
        o.nav = Some(NavTarget::to("vault").with("itemId", &it.id));
        return Ok(o.prov("Idea Vault", it.label));
    }
    let folder = match a_str(a, "folder") {
        Some(r) => Some(found(ctx, &rv::VAULT_FOLDER, &r)?.id),
        None => None,
    };
    let collection = match a_str(a, "collection") {
        Some(r) => Some(found(ctx, &rv::VAULT_COLLECTION, &r)?.id),
        None => None,
    };
    let q = a_str(a, "query").map(|q| format!("%{}%", q.replace('%', "")));
    let limit = a_limit(a, 30) as i64;
    let mut stmt = c.prepare(
        "SELECT i.id, i.item_type, COALESCE(NULLIF(i.title,''), substr(i.body,1,60), i.caption, i.url, i.item_type),
                substr(COALESCE(i.body, i.caption, i.url, ''),1,160), i.pinned
         FROM vault_item i WHERE i.deleted_at IS NULL
           AND (?1 IS NULL OR i.title LIKE ?1 OR i.body LIKE ?1 OR i.caption LIKE ?1 OR i.url LIKE ?1
                OR i.id IN (SELECT item_id FROM vault_item_tag WHERE tag LIKE ?1))
           AND (?2 IS NULL OR i.id IN (SELECT item_id FROM vault_item_tag WHERE tag = ?2))
           AND (?3 IS NULL OR i.folder_id = ?3)
           AND (?4 IS NULL OR i.id IN (SELECT item_id FROM vault_item_collection WHERE collection_id = ?4))
           AND (?5 IS NULL OR i.item_type = ?5)
           AND (?6 = 0 OR i.pinned = 1)
         ORDER BY i.pinned DESC, i.updated_at DESC LIMIT ?7",
    )?;
    let rows: Vec<(String, String, String, String, bool)> = stmt
        .query_map(
            params![
                q,
                a_str(a, "tag"),
                folder,
                collection,
                a_str(a, "type"),
                a_bool(a, "pinned") as i64,
                limit
            ],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?
        .collect::<Result<_, _>>()?;
    let total = queries::count(
        c,
        "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL",
        [],
    )?;
    let mut o = ToolOutput::exact(format!(
        "{} shown of {} in the Idea Vault.",
        plural(rows.len(), "item", "items"),
        total
    ));
    for (id, kind, title, excerpt, pinned) in rows {
        o.items.push(item(
            format!(
                "{}{}",
                clip(&title, 100),
                if pinned { " (pinned)" } else { "" }
            ),
            Some(format!("{kind} · {}", clip(&excerpt, 160))),
            Some(NavTarget::to("vault").with("itemId", &id)),
        ));
    }
    Ok(o.prov("Scope", "Idea Vault (this project)"))
}

// ------------------------------------------------------------------ story

fn story_outline(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let episode = match a_str(a, "episode") {
        Some(r) => Some(found(ctx, &rv::EPISODE, &r)?),
        None => None,
    };
    let only_act = match a_str(a, "act") {
        Some(r) => Some(found(ctx, &rv::ACT, &r)?),
        None => None,
    };
    let ep = episode.as_ref().map(|e| e.id.clone());
    let mut acts_stmt = c.prepare(
        "SELECT id, title FROM story_act WHERE deleted_at IS NULL AND episode_id IS ?1 ORDER BY position",
    )?;
    let acts: Vec<(String, String)> = acts_stmt
        .query_map([&ep], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let scene_numbers = scene_number_map(c)?;
    let mut o = ToolOutput::exact(String::new());
    let mut lines = 0usize;
    for (i, (act_id, title)) in acts.iter().enumerate() {
        if only_act.as_ref().is_some_and(|x| &x.id != act_id) {
            continue;
        }
        o.items.push(item(
            format!("Act {} — {title}", i + 1),
            None,
            Some(NavTarget::to("story")),
        ));
        for (kind, label, extra) in container_children(c, "act", act_id, &scene_numbers)? {
            if lines > 150 {
                break;
            }
            lines += 1;
            o.items.push(item(
                format!("  {kind}: {}", clip(&label, 120)),
                extra,
                None,
            ));
            if kind == "Sequence" {
                let seq_id: Option<String> = c
                    .query_row(
                        "SELECT id FROM story_sequence WHERE act_id=?1 AND title=?2 AND deleted_at IS NULL LIMIT 1",
                        params![act_id, label],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(sid) = seq_id {
                    for (k2, l2, e2) in container_children(c, "sequence", &sid, &scene_numbers)? {
                        lines += 1;
                        o.items
                            .push(item(format!("    {k2}: {}", clip(&l2, 120)), e2, None));
                    }
                }
            }
        }
    }
    let parked = queries::count(
        c,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND parent_type='parking' AND episode_id IS ?1",
        [&ep],
    )?;
    let unassigned = queries::count(
        c,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND parent_type='unassigned' AND episode_id IS ?1",
        [&ep],
    )?;
    o.content = format!(
        "The Story Board has {}{}.",
        plural(acts.len(), "act", "acts"),
        episode
            .as_ref()
            .map(|e| format!(" in {}", e.label))
            .unwrap_or_default()
    );
    o = o.detail(format!(
        "Parking Lot: {}. Unassigned: {}.",
        plural(parked as usize, "Scene Card", "Scene Cards"),
        plural(unassigned as usize, "Scene Card", "Scene Cards")
    ));
    Ok(o.prov("Scope", "Story Board"))
}

/// Screenplay scene id → "Scene N" in its draft (current drafts only).
fn scene_number_map(c: &Connection) -> AppResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if let Some(d) = queries::current_draft(c)? {
        for s in queries::scenes(c, &d.id)? {
            out.insert(s.id.clone(), format!("Scene {}", s.number));
        }
    }
    Ok(out)
}

/// Children of an act or sequence in shared sibling order: (kind, label, detail).
fn container_children(
    c: &Connection,
    parent_type: &str,
    parent_id: &str,
    scenes: &BTreeMap<String, String>,
) -> AppResult<Vec<(String, String, Option<String>)>> {
    let mut rows: Vec<(i64, String, String, Option<String>)> = Vec::new();
    if parent_type == "act" {
        let mut st = c.prepare(
            "SELECT position, title FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL",
        )?;
        for r in st.query_map([parent_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })? {
            let (p, t) = r?;
            rows.push((p, "Sequence".into(), t, None));
        }
    }
    let mut st = c.prepare(
        "SELECT position, text FROM story_beat WHERE parent_type=?1 AND parent_id=?2 AND deleted_at IS NULL AND state='active'",
    )?;
    for r in st.query_map(params![parent_type, parent_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })? {
        let (p, t) = r?;
        rows.push((p, "Beat".into(), t, None));
    }
    let mut st = c.prepare(
        "SELECT position, COALESCE(NULLIF(short_description,''), 'Untitled Scene Card'), scene_heading, screenplay_scene_id
         FROM story_scene_card WHERE parent_type=?1 AND parent_id=?2 AND deleted_at IS NULL",
    )?;
    for r in st.query_map(params![parent_type, parent_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })? {
        let (p, t, h, sid) = r?;
        let mut d = Vec::new();
        if let Some(h) = h.filter(|h| !h.trim().is_empty()) {
            d.push(h);
        }
        match sid.and_then(|s| scenes.get(&s).cloned()) {
            Some(n) => d.push(format!("→ {n}")),
            None => d.push("not in the screenplay yet".into()),
        }
        rows.push((p, "Scene Card".into(), t, Some(d.join(" · "))));
    }
    rows.sort_by_key(|r| r.0);
    Ok(rows.into_iter().map(|(_, k, l, d)| (k, l, d)).collect())
}

fn story_card(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let card = found(ctx, &rv::CARD, &a_str(a, "card").unwrap_or_default())?;
    let (desc, heading, notes, color, pt, pid, sid): (String, Option<String>, Option<String>, Option<String>, String, Option<String>, Option<String>) =
        c.query_row(
            "SELECT short_description, scene_heading, notes, color, parent_type, parent_id, screenplay_scene_id FROM story_scene_card WHERE id=?1",
            [&card.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )?;
    let place = match (pt.as_str(), pid) {
        ("act", Some(id)) => format!("Act “{}”", rv::column(c, "story_act", "title", &id)?),
        ("sequence", Some(id)) => format!(
            "Sequence “{}”",
            rv::column(c, "story_sequence", "title", &id)?
        ),
        ("parking", _) => "Parking Lot".into(),
        _ => "Unassigned".into(),
    };
    let mut o = ToolOutput::exact(format!("Scene Card: {}", clip(&desc, 600)));
    o = o.detail(format!("Place: {place}"));
    if let Some(h) = heading.filter(|h| !h.trim().is_empty()) {
        o = o.detail(format!("Scene heading: {h}"));
    }
    if let Some(col) = color {
        o = o.detail(format!("Colour: {col}"));
    }
    if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
        o.items.push(item("Notes", Some(clip(&n, 2000)), None));
    }
    let chars: Vec<String> = c
        .prepare("SELECT ch.name FROM story_character_card_link l JOIN story_character ch ON ch.id = l.character_id AND ch.deleted_at IS NULL WHERE l.scene_card_id=?1 ORDER BY ch.name")?
        .query_map([&card.id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if !chars.is_empty() {
        o = o.detail(format!("Characters: {}", join_and(&chars)));
    }
    match sid.and_then(|s| queries::scene_by_id(c, &s).ok().flatten()) {
        Some((d, s)) => {
            o = o.detail(format!("Screenplay: {} ({})", s.label(), d.label()));
            o.nav = Some(scene_nav(&d, &s));
        }
        None => o = o.detail("Not built into the screenplay yet."),
    }
    Ok(o.prov("Scene Card", clip(&card.label, 80)))
}

fn story_characters(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "character") {
        let ch = found(ctx, &rv::CHARACTER, &r)?;
        let (role, desc, notes, archived): (Option<String>, Option<String>, Option<String>, bool) =
            c.query_row(
                "SELECT role_label, description, notes, archived FROM story_character WHERE id=?1",
                [&ch.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
        let mut o = ToolOutput::exact(format!(
            "{}{}{}",
            ch.label,
            role.filter(|r| !r.is_empty())
                .map(|r| format!(" — {r}"))
                .unwrap_or_default(),
            if archived { " (archived)" } else { "" }
        ));
        if let Some(d) = desc.filter(|d| !d.trim().is_empty()) {
            o.items
                .push(item("Description", Some(clip(&d, 1500)), None));
        }
        if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
            o.items.push(item("Notes", Some(clip(&n, 1500)), None));
        }
        let mut rel = c.prepare(
            "SELECT r.relationship_type, a.name, b.name FROM story_character_relationship r
             JOIN story_character a ON a.id = r.from_character_id JOIN story_character b ON b.id = r.to_character_id
             WHERE (r.from_character_id=?1 OR r.to_character_id=?1) AND a.deleted_at IS NULL AND b.deleted_at IS NULL",
        )?;
        for row in rel.query_map([&ch.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (t, x, y) = row?;
            o.items
                .push(item(format!("Relationship: {x} — {t} — {y}"), None, None));
        }
        let cards = queries::count(
            c,
            "SELECT count(*) FROM story_character_card_link l JOIN story_scene_card s ON s.id = l.scene_card_id AND s.deleted_at IS NULL WHERE l.character_id=?1",
            [&ch.id],
        )?;
        o = o.detail(format!("Linked Scene Cards: {cards}."));
        if let Some(d) = queries::current_draft(c)? {
            let cues = queries::character_cues(c, &d.id)?;
            let n = cues
                .get(&queries::normalize_cue(&ch.label))
                .map(|v| v.len())
                .unwrap_or(0);
            o = o.detail(format!(
                "Speaks in {} of {}.",
                plural(n, "scene", "scenes"),
                d.label()
            ));
        }
        let cast: Vec<(String, bool)> = c
            .prepare("SELECT person_name, is_primary FROM cast_member WHERE character_id=?1 AND deleted_at IS NULL AND archived=0")?
            .query_map([&ch.id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (p, primary) in cast {
            o = o.detail(format!(
                "Played by {p}{}",
                if primary { "" } else { " (alternate)" }
            ));
        }
        o.nav = Some(
            NavTarget::to("story")
                .with_sub("characters")
                .with("characterId", &ch.id),
        );
        return Ok(o.prov("Character", ch.label));
    }
    let archived = a_bool(a, "includeArchived");
    let mut stmt = c.prepare(
        "SELECT id, name, COALESCE(role_label,''), COALESCE(description,''), archived FROM story_character
         WHERE deleted_at IS NULL AND (?1 = 1 OR archived = 0) ORDER BY position, name LIMIT 200",
    )?;
    let rows: Vec<(String, String, String, String, bool)> = stmt
        .query_map([archived as i64], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "The Character directory has {}.",
        plural(rows.len(), "character", "characters")
    ));
    for (id, name, role, desc, arch) in rows {
        let mut d = Vec::new();
        if !role.is_empty() {
            d.push(role);
        }
        if !desc.trim().is_empty() {
            d.push(clip(&desc, 140));
        }
        if arch {
            d.push("archived".into());
        }
        o.items.push(item(
            name,
            Some(d.join(" · ")),
            Some(
                NavTarget::to("story")
                    .with_sub("characters")
                    .with("characterId", &id),
            ),
        ));
    }
    Ok(o.prov("Scope", "Character directory"))
}

fn story_series(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let mut stmt = c.prepare(
        "SELECT e.title, COALESCE(s.title, ''), COALESCE(e.status, ''), COALESCE(e.summary, '') FROM episode e
         LEFT JOIN season s ON s.id = e.season_id AND s.deleted_at IS NULL
         WHERE e.deleted_at IS NULL ORDER BY s.position, e.position LIMIT 200",
    )?;
    let rows: Vec<(String, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let seasons = queries::count(
        c,
        "SELECT count(*) FROM season WHERE deleted_at IS NULL",
        [],
    )?;
    let mut o = ToolOutput::exact(format!(
        "{} in {}.",
        plural(rows.len(), "episode", "episodes"),
        plural(seasons as usize, "season", "seasons")
    ));
    for (t, s, st, sum) in rows {
        let mut d = Vec::new();
        if !s.is_empty() {
            d.push(s);
        }
        if !st.is_empty() {
            d.push(st);
        }
        if !sum.trim().is_empty() {
            d.push(clip(&sum, 140));
        }
        o.items.push(item(
            t,
            Some(d.join(" · ")),
            Some(NavTarget::to("story").with_sub("episodes")),
        ));
    }
    Ok(o.prov("Scope", "Seasons & episodes"))
}

fn story_timeline(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
    let scenes = queries::scenes(c, &d.id)?;
    let mut by_day: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut o = ToolOutput::exact(String::new());
    for s in &scenes {
        let (day, note): (Option<String>, Option<String>) = c.query_row(
            "SELECT story_day, time_note FROM screenplay_scene WHERE id=?1",
            [&s.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let day = day
            .filter(|x| !x.trim().is_empty())
            .unwrap_or_else(|| "Unassigned".into());
        by_day
            .entry(day.clone())
            .or_default()
            .push(s.number.to_string());
        if let Some(n) = note.filter(|n| !n.trim().is_empty()) {
            o = o.detail(format!("Scene {}: {n}", s.number));
        }
    }
    o.content = format!(
        "{} of {} across {}.",
        plural(scenes.len(), "scene", "scenes"),
        d.label(),
        plural(by_day.len(), "Story Day", "Story Days")
    );
    for (day, nums) in by_day {
        o.items
            .push(item(day, Some(format!("Scenes {}", join_and(&nums))), None));
    }
    Ok(o.prov("Draft", d.label()))
}

// ------------------------------------------------------------------ screenplay

fn screenplay_scenes(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
    let scenes = queries::scenes(c, &d.id)?;
    let from = a_u32(a, "fromScene").unwrap_or(1).max(1) as usize;
    let limit = a_limit(a, 40);
    let mut o = ToolOutput::exact(format!(
        "{} has {}{} ({}).",
        d.label(),
        plural(scenes.len(), "scene", "scenes"),
        if scenes.len() > limit {
            format!(
                "; showing {} from Scene {from}",
                limit.min(scenes.len().saturating_sub(from - 1))
            )
        } else {
            String::new()
        },
        d.status
    ));
    for s in scenes.iter().skip(from - 1).take(limit) {
        let (synopsis, day): (Option<String>, Option<String>) = c.query_row(
            "SELECT synopsis, story_day FROM screenplay_scene WHERE id=?1",
            [&s.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut det = Vec::new();
        if s.omitted {
            det.push("omitted".to_string());
        }
        if let Some(day) = day.filter(|x| !x.trim().is_empty()) {
            det.push(format!("Story Day {day}"));
        }
        if let Some(syn) = synopsis.filter(|x| !x.trim().is_empty()) {
            det.push(clip(&syn, 160));
        }
        o.items.push(item(
            s.label(),
            Some(det.join(" · ")),
            Some(scene_nav(&d, s)),
        ));
    }
    Ok(o.prov("Draft", d.label()))
}

fn screenplay_scene(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
    let s = rv::scene_or_scope(c, ctx.scope, &d, a_u32(a, "sceneNumber"))?;
    let (synopsis, notes, day, time_note, card): (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) = c.query_row(
        "SELECT synopsis, notes, story_day, time_note, source_scene_card_id FROM screenplay_scene WHERE id=?1",
        [&s.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    )?;
    let mut o = ToolOutput::exact(format!(
        "{} ({}{}).",
        s.label(),
        d.label(),
        if s.omitted { ", omitted" } else { "" }
    ));
    for (label, v) in [
        ("Synopsis", synopsis),
        ("Scene note", notes),
        ("Story Day", day),
        ("Time note", time_note),
    ] {
        if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
            o = o.detail(format!("{label}: {}", clip(&v, 400)));
        }
    }
    let cues = queries::character_cues(c, &d.id)?;
    let speaking: Vec<String> = cues
        .iter()
        .filter(|(_, sc)| sc.contains(&s.id))
        .map(|(n, _)| n.clone())
        .collect();
    if !speaking.is_empty() {
        o = o.detail(format!("Speaking characters: {}", join_and(&speaking)));
    }
    let mut stmt = c.prepare(
        "SELECT element_type, text FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
    )?;
    let lines: Vec<(String, String)> = stmt
        .query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut budget = 3_000usize;
    for (i, (t, text)) in lines.iter().enumerate() {
        if budget == 0 {
            o = o.detail(format!("{} more lines not shown.", lines.len() - i));
            break;
        }
        let shown = clip(text, budget.min(600));
        budget = budget.saturating_sub(shown.len());
        o.items.push(item(
            format!("Line {} ({})", i + 1, t.replace('_', " ")),
            Some(shown),
            None,
        ));
    }
    // Where this scene is used across modules (by scene identity).
    let breakdown = queries::count(
        c,
        "SELECT count(*) FROM breakdown_element WHERE scene_lineage_id=?1 AND deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')",
        [&s.lineage_id],
    )?;
    let shots = queries::count(
        c,
        "SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
        [&s.lineage_id],
    )?;
    let boards = queries::count(
        c,
        "SELECT count(*) FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
        [&s.lineage_id],
    )?;
    let comments = queries::count(
        c,
        "SELECT count(*) FROM comment WHERE scene_id=?1 AND deleted_at IS NULL AND parent_id IS NULL AND status <> 'Resolved'",
        [&s.id],
    )?;
    let day_pos: Option<i64> = c
        .query_row(
            "SELECT d.position FROM schedule_strip st JOIN shooting_day d ON d.id = st.day_id AND d.deleted_at IS NULL
             WHERE st.scene_lineage_id=?1 AND st.deleted_at IS NULL AND st.archived=0 LIMIT 1",
            [&s.lineage_id],
            |r| r.get(0),
        )
        .optional()?;
    let day_label = match day_pos {
        Some(p) => {
            let n = queries::count(
                c,
                "SELECT count(*) FROM shooting_day d2 WHERE d2.deleted_at IS NULL AND d2.position <= ?1 AND d2.schedule_id = (SELECT schedule_id FROM shooting_day WHERE position=?1 AND deleted_at IS NULL LIMIT 1)",
                [p],
            )?;
            format!("shooting Day {n}")
        }
        None => "not on a shooting day".into(),
    };
    o = o.detail(format!(
        "Linked: {} confirmed breakdown elements · {} · {} · {} · {}.",
        breakdown,
        plural(shots as usize, "shot", "shots"),
        plural(boards as usize, "storyboard", "storyboards"),
        plural(comments as usize, "open comment", "open comments"),
        day_label
    ));
    if let Some(card) = card
        .and_then(|id| rv::column(c, "story_scene_card", "short_description", &id).ok())
        .filter(|t| !t.is_empty())
    {
        o = o.detail(format!("From Scene Card: {}", clip(&card, 200)));
    }
    o.nav = Some(scene_nav(&d, &s));
    o.targets.push(ObjRef {
        table: "screenplay_scene".into(),
        id: s.id.clone(),
        label: s.label(),
    });
    Ok(o.prov("Draft", d.label()).prov("Scene", s.label()))
}

fn draft_status(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
    let (note, locked_at, locked_by, label, color, reason, from): (Option<String>, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) =
        c.query_row(
            "SELECT note, locked_at, locked_by_name, revision_label, revision_color, revision_reason, created_from_draft_id FROM screenplay_draft WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )?;
    let scenes = queries::scenes(c, &d.id)?.len();
    let mut o = ToolOutput::exact(format!(
        "{} is {}{} with {}.",
        d.label(),
        d.status,
        if d.is_current { " (current draft)" } else { "" },
        plural(scenes, "scene", "scenes")
    ));
    if let Some(at) = locked_at {
        o = o.detail(format!(
            "Locked {}{}.",
            when(at),
            locked_by.map(|b| format!(" by {b}")).unwrap_or_default()
        ));
    }
    if let Some(l) = label {
        o = o.detail(format!(
            "Revision: {l}{}{}",
            color.map(|c| format!(" ({c})")).unwrap_or_default(),
            reason
                .map(|r| format!(" — {}", clip(&r, 200)))
                .unwrap_or_default()
        ));
    }
    if let Some(f) = from.and_then(|f| queries::draft_by_id(c, &f).ok().flatten()) {
        o = o.detail(format!("Created from {}.", f.label()));
    }
    if let Some(n) = note.filter(|n| !n.trim().is_empty()) {
        o = o.detail(format!("Note: {}", clip(&n, 300)));
    }
    let open = queries::count(
        c,
        "SELECT count(*) FROM comment cm JOIN screenplay_scene s ON s.id = cm.scene_id WHERE s.draft_id=?1 AND cm.deleted_at IS NULL AND cm.parent_id IS NULL AND cm.status <> 'Resolved'",
        [&d.id],
    )?;
    let points = queries::count(
        c,
        "SELECT count(*) FROM screenplay_history_point WHERE draft_id=?1",
        [&d.id],
    )?;
    o = o.detail(format!(
        "{} · {} (1 = most recent).",
        plural(open as usize, "open comment", "open comments"),
        plural(points as usize, "history point", "history points")
    ));
    let mut rr = c.prepare("SELECT name, status, reviewers_json, COALESCE(deadline,'') FROM review_round WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 10")?;
    for row in rr.query_map([&d.id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        let (name, st, reviewers, deadline) = row?;
        let rv: Vec<String> = serde_json::from_str(&reviewers).unwrap_or_default();
        o.items.push(item(
            format!("Review round “{name}”"),
            Some(format!(
                "{st}{}{}",
                if rv.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", rv.join(", "))
                },
                if deadline.is_empty() {
                    String::new()
                } else {
                    format!(" · due {deadline}")
                }
            )),
            None,
        ));
    }
    Ok(o.prov("Draft", d.label()))
}

fn character_cues(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::draft(c, ctx.scope, a_str(a, "draft").as_deref())?;
    let cues = queries::character_cues(c, &d.id)?;
    let sp: String = c.query_row(
        "SELECT screenplay_id FROM screenplay_draft WHERE id=?1",
        [&d.id],
        |r| r.get(0),
    )?;
    let mut links: BTreeMap<String, (Option<String>, bool)> = BTreeMap::new();
    let mut st = c.prepare(
        "SELECT l.cue_name, ch.name, l.ignored FROM screenplay_character_link l
         LEFT JOIN story_character ch ON ch.id = l.character_id AND ch.deleted_at IS NULL WHERE l.screenplay_id=?1",
    )?;
    for row in st.query_map([&sp], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, bool>(2)?,
        ))
    })? {
        let (cue, ch, ignored) = row?;
        links.insert(queries::normalize_cue(&cue), (ch, ignored));
    }
    let records = queries::character_records(c)?;
    let mut o = ToolOutput::exact(format!(
        "{} in {}.",
        plural(cues.len(), "character cue", "character cues"),
        d.label()
    ));
    for (cue, scenes) in &cues {
        let link = match links.get(cue) {
            Some((_, true)) => "not a character (ignored)".to_string(),
            Some((Some(ch), false)) => format!("linked to Character “{ch}”"),
            _ => match records
                .iter()
                .find(|(_, n, _)| queries::normalize_cue(n) == *cue)
            {
                Some((_, n, _)) => format!("matches Character “{n}” by name"),
                None => "no Character record".into(),
            },
        };
        o.items.push(item(
            cue.clone(),
            Some(format!(
                "{} · {link}",
                plural(scenes.len(), "scene", "scenes")
            )),
            None,
        ));
    }
    Ok(o.prov("Draft", d.label()))
}

// ------------------------------------------------------------------ helpers for other files

pub(super) fn not_found(what: impl Into<String>) -> AppError {
    AppError::ai("not_found", what)
}
