//! Production screens that had no fetcher before the section picker (ADR-0041): Overview, Catalog,
//! Day Out of Days, Daily View, Sides and Reports. Each one reads through the same code the screen
//! uses, so the answer and the screen say the same thing. Plain facts, typed queries, no writes.

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::production::source::read_overview;
use crate::modules::schedule::docs::{REPORT_TYPES, ScheduleSidesContent, build_report};
use crate::modules::schedule::dood::{self, ScheduleDoodArgs};
use crate::modules::schedule::ops::daily_view;

/// The most rows of one report given to the writer.
const REPORT_ROWS: usize = 40;
/// The rows of a report nobody named: enough to tell what it holds.
const REPORT_PREVIEW_ROWS: usize = 3;

// ------------------------------------------------------------------ Overview

pub fn overview(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("production overview", || {
        let root = ctx
            .core
            .and_then(|c| c.project().ok())
            .map(|s| s.layout.root().to_path_buf())
            .unwrap_or_default();
        let o = read_overview(ctx.conn, &root)?;
        let mut f = Fact::new("Production overview", "")
            .f(
                "source draft",
                o.source.map(|s| s.draft.name).unwrap_or_default(),
            )
            .f("scenes", o.scene_count.to_string())
            .f(
                "scenes with a finished breakdown",
                o.complete_count.to_string(),
            )
            .f("scenes with planned elements", o.planned_count.to_string())
            .f("suggestions to review", o.pending_suggestions.to_string())
            .f("scenes needing review", o.needs_review.len().to_string())
            .f(
                "scenes needing a breakdown",
                o.needs_breakdown.len().to_string(),
            )
            .f("locations", o.location_count.to_string())
            .f("locations not settled", o.unresolved_locations.to_string())
            .f("characters", o.character_count.to_string())
            .f("catalog items", o.catalog_count.to_string())
            .prio(P_HIGH);
        if !o.unresolved_characters.is_empty() {
            f = f.f(
                "characters without an actor",
                clip(&o.unresolved_characters.join(", "), 300),
            );
        }
        Ok(vec![f])
    })
}

// ------------------------------------------------------------------ Catalog

pub fn catalog(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("catalog", || {
        let mut st = ctx.conn.prepare(
            "SELECT id, name, category, status, COALESCE(description, '') FROM catalog_item
             WHERE deleted_at IS NULL AND archived = 0 ORDER BY category, name LIMIT 400",
        )?;
        let rows: Vec<(String, String, String, String, String)> = st
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;
        if rows.is_empty() {
            return Ok(vec![Fact::absent("any catalog items", "the Catalog")]);
        }
        let mut count = Fact::new("Count", "")
            .f("what", "catalog items")
            .f("value", rows.len().to_string())
            .prio(P_HIGH);
        if read.answer_type == AnswerType::Count {
            count = count.require(rows.len().to_string());
        }
        let mut out = vec![count];
        for (id, name, category, status, description) in rows {
            let named = {
                let n = normalize(&name);
                !n.is_empty() && ctx.m.has(&n)
            };
            let mut f = Fact::new("Catalog item", id)
                .f("name", &name)
                .f("category", &category)
                .f("status", &status)
                .prio(if named { P_HIGH } else { P_LOW });
            if named {
                f = f.f("description", clip(&description, 260)).require(&name);
            }
            out.push(f);
        }
        Ok(out)
    })
}

// ------------------------------------------------------------------ Day Out of Days

pub fn day_out_of_days(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("day out of days", || {
        let args = ScheduleDoodArgs {
            cast_keys: None,
            from_day_id: None,
            to_day_id: None,
        };
        let Some((_, view)) = dood::report(ctx.conn, &args)? else {
            return Ok(vec![Fact::absent(
                "a shooting schedule to count days from",
                "Day Out of Days",
            )]);
        };
        if view.rows.is_empty() {
            return Ok(vec![Fact::absent(
                "any cast working on a shoot day",
                "Day Out of Days",
            )]);
        }
        let day = ctx.picked_day();
        let names: Vec<String> = r
            .characters
            .iter()
            .chain(r.persons.iter())
            .map(|h| normalize(&h.name))
            .collect();
        let mut out = Vec::new();
        for row in &view.rows {
            let named = names.iter().any(|n| {
                !n.is_empty()
                    && (normalize(&row.character) == *n
                        || row.actor.as_deref().is_some_and(|a| normalize(a) == *n))
            });
            let mut f = Fact::new("Day out of days", "")
                .f("cast", &row.label)
                .f("work days", row.work_days.to_string())
                .f("hold days", row.hold_days.to_string())
                .f("travel days", row.travel_days.to_string())
                .f("start", row.start_label.as_deref().unwrap_or_default())
                .f("finish", row.finish_label.as_deref().unwrap_or_default())
                .prio(if named { P_HIGH } else { P_MID });
            if let Some(d) = day {
                let code = row
                    .cells
                    .iter()
                    .find(|c| c.day_id == d)
                    .map_or("", |c| c.code.as_str());
                f = f.f("code on this day", code);
            }
            if named {
                f = f.require(row.work_days.to_string());
            }
            out.push(f);
        }
        if read.answer_type == AnswerType::Count {
            out.insert(
                0,
                Fact::new("Count", "")
                    .f("what", "cast members on the day out of days")
                    .f("value", view.rows.len().to_string())
                    .prio(P_HIGH),
            );
        }
        Ok(out)
    })
}

// ------------------------------------------------------------------ Daily View

pub fn daily(ctx: &FetchCtx<'_>, _read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("daily view", || {
        // The day the person chose, the day the question names, else the day the screen opens on.
        let named = r.days.first().map(|h| h.id.as_str());
        let view = daily_view(ctx.conn, ctx.picked_day().or(named))?;
        if !view.has_schedule {
            return Ok(vec![Fact::absent("a shooting schedule", "the Daily View")]);
        }
        let Some(d) = view.day else {
            return Ok(vec![Fact::absent("any shoot days", "the Daily View")]);
        };
        let scenes: Vec<u32> = d
            .scenes
            .iter()
            .filter_map(|s| s.number.map(|n| n as u32))
            .collect();
        let cast: Vec<String> = d
            .cast
            .iter()
            .map(|c| match &c.actor {
                Some(a) => format!("{} ({a})", c.character),
                None => c.character.clone(),
            })
            .collect();
        let places: Vec<&str> = d.locations.iter().map(|l| l.name.as_str()).collect();
        let items: Vec<String> = d
            .breakdown_items
            .iter()
            .map(|i| format!("{}: {}", i.category, i.name))
            .collect();
        let mut f = Fact::new("Daily view", &d.id)
            .f("day", &d.label)
            .f("title", &d.title)
            .f(
                "date",
                d.date_label
                    .as_deref()
                    .or(d.date.as_deref())
                    .unwrap_or_default(),
            )
            .f("scenes", numbers(scenes.iter().copied()))
            .f("cast", clip(&cast.join(", "), 400))
            .f("crew call", d.crew_call.as_deref().unwrap_or_default())
            .f("locations", clip(&places.join(", "), 300))
            .f("items", clip(&items.join(", "), 400))
            .f("notes", clip(d.notes.as_deref().unwrap_or_default(), 300))
            .f("warnings", d.warnings.len().to_string())
            .prio(P_HIGH);
        if d.is_off_day {
            f = f.f("off day", "yes");
        }
        if let Some(sheet) = &d.call_sheet {
            f = f.f(
                "call sheet",
                format!("revision {}, {}", sheet.revision, sheet.status),
            );
        }
        Ok(vec![f])
    })
}

// ------------------------------------------------------------------ Sides

pub fn sides(ctx: &FetchCtx<'_>, _read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("sides", || {
        let mut st = ctx.conn.prepare(
            "SELECT id, title, source_label, content_json FROM side
             WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT 60",
        )?;
        let rows: Vec<(String, String, String, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<_, _>>()?;
        let wanted_scenes: Vec<&str> = r
            .scenes
            .iter()
            .filter_map(|s| s.scene_id.as_deref())
            .collect();
        let mut out = Vec::new();
        for (id, title, source, content) in &rows {
            let Ok(c) = serde_json::from_str::<ScheduleSidesContent>(content) else {
                continue;
            };
            let holds_scene = wanted_scenes.is_empty()
                || c.scenes
                    .iter()
                    .any(|s| wanted_scenes.contains(&s.scene_id.as_str()));
            if !holds_scene {
                continue;
            }
            let numbers: Vec<&str> = c.scenes.iter().map(|s| s.number.as_str()).collect();
            out.push(
                Fact::new("Sides", id)
                    .f("title", title)
                    .f("source", source)
                    .f("day", c.day_label.as_deref().unwrap_or_default())
                    .f("date", c.date_label.as_deref().unwrap_or_default())
                    .f("scenes", clip(&numbers.join(", "), 200))
                    .prio(P_MID),
            );
        }
        if out.is_empty() {
            let what = if wanted_scenes.is_empty() {
                "any saved sides"
            } else {
                "saved sides for that scene"
            };
            return Ok(vec![Fact::absent(what, "Sides")]);
        }
        Ok(out)
    })
}

// ------------------------------------------------------------------ Reports

/// The report types a question names ("the prop report"); none named reads every type shortly.
fn reports_named(ctx: &FetchCtx<'_>) -> Vec<&'static str> {
    let asked = |code: &str| match code {
        "scene" => ctx.m.any(&["scene report", "scene reports"]),
        "location" => ctx.m.stem("location"),
        "cast_scene" => ctx.m.any(&["cast", "actor", "actors"]),
        "prop" => ctx.m.stem("prop"),
        "schedule" => ctx.m.any(&["schedule report", "schedule reports"]),
        "breakdown_completeness" => ctx.m.any(&["breakdown", "completeness", "complete"]),
        _ => false,
    };
    REPORT_TYPES
        .iter()
        .map(|(code, _)| *code)
        .filter(|code| asked(code))
        .collect()
}

pub fn reports(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("reports", || {
        let named = reports_named(ctx);
        let mut out = Vec::new();
        for (code, _) in REPORT_TYPES {
            let is_named = named.contains(&code);
            let data = build_report(ctx.conn, code, None)?;
            let shown = if is_named {
                REPORT_ROWS
            } else {
                REPORT_PREVIEW_ROWS
            };
            let rows: Vec<String> = data
                .rows
                .iter()
                .take(shown)
                .map(|row| row.join(" / "))
                .collect();
            out.push(
                Fact::new("Report", "")
                    .f("report", &data.title)
                    .f("source", &data.source_label)
                    .f("rows", data.rows.len().to_string())
                    .f("columns", data.columns.join(", "))
                    .f(
                        "first rows",
                        clip(&rows.join("; "), if is_named { 1_400 } else { 240 }),
                    )
                    .f("note", data.note.as_deref().unwrap_or_default())
                    .prio(if is_named { P_HIGH } else { P_MID }),
            );
        }
        let mut st = ctx.conn.prepare(
            "SELECT id, title, report_type FROM production_report
             WHERE deleted_at IS NULL ORDER BY generated_at DESC LIMIT 20",
        )?;
        let saved: Vec<(String, String, String)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        for (id, title, kind) in saved {
            let name = REPORT_TYPES
                .iter()
                .find(|(c, _)| *c == kind)
                .map_or(kind.as_str(), |(_, n)| n);
            out.push(
                Fact::new("Saved report", id)
                    .f("title", title)
                    .f("report", name)
                    .prio(P_LOW),
            );
        }
        Ok(out)
    })
}
