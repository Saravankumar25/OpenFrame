//! Production screens that had no fetcher before the section picker (ADR-0041): Overview, Catalog,
//! Day Out of Days, Daily View, Sides and Reports. Each one reads through the same code the screen
//! uses, so the answer and the screen say the same thing. Plain facts, typed queries, no writes.

use super::breakdown::categories_in;
use super::schedule;
use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::pointer;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::production::source::read_overview;
use crate::modules::schedule::board;
use crate::modules::schedule::docs::{REPORT_TYPES, ScheduleSidesContent, build_report};
use crate::modules::schedule::dood::{self, ScheduleDoodArgs};
use crate::modules::schedule::ops::daily_view;

/// The most rows of one report given to the writer.
const REPORT_ROWS: usize = 40;
/// The rows of a report nobody named: enough to tell what it holds.
const REPORT_PREVIEW_ROWS: usize = 3;

// ------------------------------------------------------------------ Overview

pub fn overview(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
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
        Ok(vec![demand_the_asked_field(ctx, read, plan_counts(ctx, f))])
    })
}

/// A count or a lookup demands the field whose name the question says in full: "how many shoot days"
/// is "shoot days", and "scenes unscheduled" wins over "scenes" when both words are there.
fn demand_the_asked_field(ctx: &FetchCtx<'_>, read: &TurnRead, f: Fact) -> Fact {
    if !matches!(read.answer_type, AnswerType::Count | AnswerType::Lookup) {
        return f;
    }
    let typed = pointer::words(&ctx.message);
    let best = f
        .fields
        .iter()
        .filter(|(name, _)| pointer::words(name).iter().all(|w| typed.contains(w)))
        .max_by_key(|(name, _)| pointer::words(name).len())
        .map(|(_, value)| value.clone());
    // A zero is not demanded: with no Production Source the screen's counts are all empty, and the
    // answer may rightly come from the script.
    match best.filter(|v| v != "0") {
        Some(value) => f.require(value),
        None => f,
    }
}

/// The rest of what the Overview and Home screens sum up: the shoot, its warnings, the call sheets,
/// the crew, the shots and the budget's currency. A table that cannot be read leaves its number out.
fn plan_counts(ctx: &FetchCtx<'_>, mut f: Fact) -> Fact {
    let number = |sql: &str| -> Option<i64> { ctx.conn.query_row(sql, [], |r| r.get(0)).ok() };
    let mut count = |name: &str, sql: &str| {
        if let Some(n) = number(sql) {
            f = std::mem::replace(&mut f, Fact::new("", "")).f(name, n.to_string());
        }
    };
    count(
        "crew members",
        "SELECT count(*) FROM crew_member WHERE deleted_at IS NULL AND archived = 0",
    );
    count(
        "call sheets",
        "SELECT count(*) FROM call_sheet WHERE deleted_at IS NULL AND superseded_by IS NULL",
    );
    count(
        "shots",
        "SELECT count(*) FROM shot WHERE deleted_at IS NULL",
    );
    let Some(schedule) = board::current_schedule(ctx.conn).ok().flatten() else {
        return f;
    };
    let id = schedule.id.clone();
    let on_board = |test: &str| -> Option<i64> {
        ctx.conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM schedule_strip WHERE schedule_id = ?1
                     AND deleted_at IS NULL AND archived = 0 AND {test}"
                ),
                [&id],
                |r| r.get(0),
            )
            .ok()
    };
    let scheduled = on_board("day_id IS NOT NULL");
    let unscheduled = on_board("day_id IS NULL");
    let days = ctx.days();
    let shoot: Vec<_> = days.iter().filter(|d| d.number.is_some()).collect();
    f = f.f("shoot days", shoot.len().to_string());
    if let (Some(first), Some(last)) = (
        shoot.iter().filter_map(|d| d.date.as_deref()).min(),
        shoot.iter().filter_map(|d| d.date.as_deref()).max(),
    ) {
        f = f.f("shoot dates", format!("{first} to {last}"));
    }
    if let Some(n) = scheduled {
        f = f.f("scenes scheduled", n.to_string());
    }
    if let Some(n) = unscheduled {
        f = f.f("scenes unscheduled", n.to_string());
    }
    if let Ok(b) = board::Board::load(ctx.conn, schedule)
        && let Ok(acks) = board::load_acks(ctx.conn, &b.schedule.id)
    {
        let open = b
            .warnings(&acks)
            .iter()
            .filter(|w| w.decision.is_none())
            .count();
        f = f.f("open schedule warnings", open.to_string());
    }
    let currency: Option<String> = ctx
        .conn
        .query_row(
            "SELECT currency FROM budget_snapshot WHERE deleted_at IS NULL AND is_current = 1 LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    if let Some(c) = currency {
        f = f.f("budget currency", c);
    }
    f
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
        // A category the question names is counted on its own, and the whole catalog is then reading.
        let named_categories = categories_in(&ctx.m);
        if !named_categories.is_empty() {
            out[0].requires.clear();
        }
        let counting = read.answer_type == AnswerType::Count;
        for cat in &named_categories {
            let n = rows.iter().filter(|r| r.2 == *cat).count();
            let mut f = Fact::new("Count", "")
                .f("what", format!("{} in the catalog", cat.to_lowercase()))
                .f("value", n.to_string())
                .prio(P_HIGH);
            if counting {
                f = f.require(n.to_string());
            }
            out.push(f);
        }
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
        let asked_days = schedule::named_days(ctx, r);
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
                // The days worked come from the row's cells, the same cells the screen shows.
                let worked: Vec<String> = view
                    .days
                    .iter()
                    .filter(|d| {
                        row.cells.iter().any(|c| {
                            c.day_id == d.id && worked_code(&c.code)
                        })
                    })
                    .map(|d| d.short_label.clone())
                    .collect();
                f = f.f("days worked", worked.join(", "));
                for n in &asked_days {
                    let Some(d) = view.days.iter().find(|d| day_number(&d.short_label) == Some(*n))
                    else {
                        continue;
                    };
                    let code = row
                        .cells
                        .iter()
                        .find(|c| c.day_id == d.id)
                        .map_or("", |c| c.code.as_str());
                    let works = worked_code(code);
                    f = f.f(
                        &format!("works on {}", d.short_label),
                        if works { "yes" } else { "no" },
                    );
                    if read.answer_type == AnswerType::Lookup {
                        f = f.require(if works { "yes" } else { "no" });
                    }
                }
                match read.answer_type {
                    AnswerType::Count => f = f.require(row.work_days.to_string()),
                    AnswerType::List => {
                        for w in &worked {
                            f = f.require(w);
                        }
                    }
                    _ => {}
                }
                if ctx.m.any(&["first day", "first working day", "start"]) {
                    f = f.require(row.start_label.as_deref().unwrap_or_default());
                }
                if ctx.m.any(&["last day", "last working day", "finish"]) {
                    f = f.require(row.finish_label.as_deref().unwrap_or_default());
                }
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

/// A cell code that means the person works that day: start, work, finish and the like, not a hold, a
/// travel day or an empty cell.
fn worked_code(code: &str) -> bool {
    let c = code.trim().to_uppercase();
    !c.is_empty() && !c.starts_with('H') && !c.starts_with('T') && c != "-"
}

/// The number in a day label: "Day 9" to 9.
fn day_number(label: &str) -> Option<u32> {
    label
        .split(|c: char| !c.is_ascii_digit())
        .find(|p| !p.is_empty())
        .and_then(|p| p.parse().ok())
}

// ------------------------------------------------------------------ Daily View

pub fn daily(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
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
            .f("scene count", scenes.len().to_string())
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
        // Scenes and notes are demanded when the question names them, and a count only states its
        // number.
        if read.answer_type == AnswerType::Count {
            if ctx.m.stem("scene") {
                f = f.require(scenes.len().to_string());
            }
        } else {
            if ctx.m.stem("scene") {
                for n in &scenes {
                    f = f.require(n.to_string());
                }
            }
            if ctx.m.stem("note") {
                f = f.require(clip(d.notes.as_deref().unwrap_or_default(), 300));
            }
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
