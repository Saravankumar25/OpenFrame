//! Production: locations (and where a scene is shot), tasks, permits and call sheet status.
//! Reads `location`, `catalog_item`, `breakdown_element`, `task`, `call_sheet`.

use openframe_domain::AppResult;
use serde_json::Value;

use super::{files, schedule};
use crate::modules::ai::dossier::asked::{self, best_rows, names_row, row_words};
use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::guard;
use crate::modules::ai::dossier::words::{clip, date_is, date_of, dates_in};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID, P_PICKED};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::production::availability::{self as av, AvailabilityOwner};

/// Words that only name the location list.
const LOCATION_WORDS: &[&str] = &[
    "location", "locations", "place", "places", "status", "confirmed", "shortlisted", "idea",
    "rejected", "backup", "shoot", "shot", "set",
];
/// Words that only name the task list.
const TASK_WORDS: &[&str] = &[
    "task", "tasks", "todo", "due", "date", "deadline", "overdue", "open", "done", "pending",
    "outstanding", "finished", "completed", "left", "first", "earliest", "soon", "march",
    "february", "april",
];
/// Rows of the location list or the task list one question can demand by its words.
const ROWS_PICKED: usize = 3;

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("production", || fetch(ctx, read, r))
}

struct Loc {
    id: String,
    name: String,
    status: String,
    address: String,
    notes: String,
    hospital: String,
    contact: String,
    emergency: String,
}

fn locations(ctx: &FetchCtx<'_>) -> AppResult<Vec<Loc>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, name, status, COALESCE(address, ''), COALESCE(notes_json, '{}'), COALESCE(nearest_hospital, ''),
                COALESCE(contact, ''), COALESCE(emergency_contact, '')
         FROM location WHERE deleted_at IS NULL AND archived = 0 ORDER BY name LIMIT 300",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(Loc {
                id: r.get(0)?,
                name: r.get(1)?,
                status: r.get(2)?,
                address: r.get(3)?,
                notes: r.get(4)?,
                hospital: r.get(5)?,
                contact: r.get(6)?,
                emergency: r.get(7)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The notes of a location as `kind: text; kind: text`.
fn notes_text(notes_json: &str) -> String {
    match serde_json::from_str::<Value>(notes_json) {
        Ok(Value::Object(o)) => o
            .iter()
            .filter_map(|(k, v)| {
                v.as_str()
                    .filter(|t| !t.trim().is_empty())
                    .map(|t| format!("{k}: {}", clip(t, 260)))
            })
            .collect::<Vec<_>>()
            .join("; "),
        _ => String::new(),
    }
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out: Vec<Fact> = Vec::new();
    let all = locations(ctx)?;
    let scenes = ctx.focus_scenes(r);
    let asks_loc = m.stem("location") || !r.locations.is_empty();
    let asks_where = m.any(&[
        "where",
        "location",
        "locations",
        "shot",
        "shoot",
        "filmed",
        "set",
    ]);
    let asks_notes = m.any(&[
        "safety", "safe", "hospital", "issue", "issues", "rules", "permit", "permits", "parking",
        "access", "notes", "risk", "hazard",
    ]);
    let asks_contact = m.any(&["contact", "phone", "email", "reach", "emergency"]);
    let asks_avail = m.any(&["available", "availability", "free", "unavailable"]);
    let days = schedule::named_days(ctx, r);
    // The scenes of a day the question names, when no scene is named: where the day is shot.
    let of_days = if scenes.is_empty() && (asks_loc || asks_where) {
        schedule::scenes_of_days(ctx, &days)?
    } else {
        Vec::new()
    };

    // ---- where a scene is shot
    if !scenes.is_empty() && (asks_loc || asks_where) {
        for s in scenes.iter().take(4) {
            let (fact, ids) = scene_location(ctx, s, &all)?;
            out.extend(fact);
            if asks_notes || asks_contact || asks_avail {
                for l in all.iter().filter(|l| ids.contains(&l.id)) {
                    let mut card = location_card(ctx, l, asks_contact, asks_avail, true)?;
                    if asks_avail && let Some(w) = card.field("availability").map(str::to_string) {
                        card = card.require(w);
                    }
                    out.push(card);
                }
            }
        }
    }
    for s in of_days.iter().take(8) {
        let (fact, _) = scene_location(ctx, s, &all)?;
        out.extend(fact);
    }

    // ---- a named location
    for h in &r.locations {
        if let Some(l) = all.iter().find(|l| l.id == h.id) {
            out.push(location_card(ctx, l, asks_contact, asks_avail, true)?);
        }
    }
    if asks_contact && !ctx.guard.contacts && (!r.locations.is_empty() || m.stem("location")) {
        out.push(Fact::missing(guard::NOT_FOR_YOUR_ROLE));
    }
    // "Is the beach confirmed", then "who is the contact": the place the last message picked by a word.
    if asks_contact
        && ctx.guard.contacts
        && r.locations.is_empty()
        && scenes.is_empty()
        && of_days.is_empty()
        && let Some(previous) = &ctx.previous
    {
        let names: Vec<String> = all.iter().map(|l| l.name.clone()).collect();
        if let [i] = best_rows(&row_words(previous, LOCATION_WORDS), &names, 1)[..] {
            out.push(location_card(ctx, &all[i], true, false, true)?);
        }
    }

    // ---- the locations as a whole
    if r.locations.is_empty() && scenes.is_empty() && of_days.is_empty() && m.stem("location") {
        if all.is_empty() {
            // "Locations used in the script" are the scene headings' places (the Screenplay
            // fetcher); an empty Production list says nothing about them.
            if !(m.stem("script") || m.stem("screenplay")) {
                out.push(Fact::absent("any locations", "Production"));
            }
        } else {
            out.push(
                Fact::new("Count", "")
                    .f("what", "locations")
                    .f("value", all.len().to_string())
                    .prio(P_HIGH)
                    .require(if t == AnswerType::Count && !status_asked(ctx) {
                        all.len().to_string()
                    } else {
                        String::new()
                    }),
            );
            let mut by_status: Vec<(String, usize)> = Vec::new();
            for l in &all {
                match by_status.iter_mut().find(|(s, _)| *s == l.status) {
                    Some(e) => e.1 += 1,
                    None => by_status.push((l.status.clone(), 1)),
                }
            }
            for (status, n) in &by_status {
                let asked = ctx.m.has(&status.to_lowercase());
                let mut f = Fact::new("Count", "")
                    .f("what", format!("locations with status {status}"))
                    .f("value", n.to_string())
                    .prio(P_HIGH);
                // A list by status says how many too, so a wrong total cannot stand beside it.
                if asked && matches!(t, AnswerType::Count | AnswerType::List | AnswerType::Lookup) {
                    f = f.require(n.to_string());
                }
                out.push(f);
            }
            let wanted_status: Vec<&str> =
                ["idea", "shortlisted", "confirmed", "rejected", "backup"]
                    .into_iter()
                    .filter(|s| m.has(s))
                    .collect();
            // A word of the question in a row's name or notes picks it. A status then only
            // describes the row; with no such word the status is the filter.
            let words = row_words(&ctx.message, LOCATION_WORDS);
            let texts: Vec<String> = all
                .iter()
                .map(|l| format!("{} {}", l.name, notes_text(&l.notes)))
                .collect();
            let picked = best_rows(&words, &texts, ROWS_PICKED);
            for (i, l) in all.iter().enumerate() {
                let by_status = wanted_status.iter().any(|s| l.status.to_lowercase() == *s);
                let by_word = picked.contains(&i);
                let hit = by_word || (picked.is_empty() && by_status);
                let mut f = Fact::new("Location", &l.id)
                    .f("name", &l.name)
                    .f("status", &l.status)
                    .prio(if by_word {
                        P_PICKED
                    } else if hit {
                        P_HIGH
                    } else {
                        P_MID
                    });
                if by_word {
                    if !name_holds_a_word(&words, &l.name) {
                        f = f.f("notes", notes_text(&l.notes));
                    }
                    f = f.require(&l.name);
                    if asked::status_asked(&ctx.message) {
                        f = f.require(&l.status);
                    }
                } else if hit && matches!(t, AnswerType::List | AnswerType::Lookup) {
                    f = f.require(&l.name);
                }
                out.push(f);
            }
        }
    }

    // ---- tasks and permits
    let tasks_asked = m.stem("task") || m.any(&["todo", "to do", "due", "overdue", "deadline"]);
    let permit_asked = m.stem("permit");
    if tasks_asked || permit_asked {
        // A permit question reads the tasks that mention a permit, not the whole list.
        let only = (!tasks_asked).then_some("permit");
        out.extend(tasks(ctx, read, tasks_asked, only)?);
    }
    if permit_asked {
        out.extend(files::notes_matching(ctx, &["permit", "permits"], 4)?);
    }

    // ---- call sheet status
    if m.any(&[
        "call sheet",
        "call sheets",
        "callsheet",
        "callsheets",
        "sides",
    ]) {
        out.extend(schedule::call_sheets(
            ctx,
            read,
            &schedule::named_days(ctx, r),
            false,
        )?);
    }
    Ok(out)
}

/// The words pick the row by its name, not by what its notes say.
fn name_holds_a_word(words: &[String], name: &str) -> bool {
    best_rows(words, std::slice::from_ref(&name.to_string()), 1) == [0]
}

fn status_asked(ctx: &FetchCtx<'_>) -> bool {
    ctx.m.any(&[
        "idea",
        "shortlisted",
        "confirmed",
        "rejected",
        "backup",
        "status",
    ])
}

fn location_card(
    ctx: &FetchCtx<'_>,
    l: &Loc,
    contact: bool,
    availability: bool,
    full: bool,
) -> AppResult<Fact> {
    let mut f = Fact::new("Location", &l.id)
        .f("name", &l.name)
        .f("status", &l.status)
        .prio(P_HIGH);
    if full {
        f = f
            .f("address", clip(&l.address, 160))
            .f("nearest hospital", clip(&l.hospital, 160))
            .f("notes", notes_text(&l.notes));
    }
    if availability {
        let w = av::windows_for(ctx.conn, AvailabilityOwner::Location, &l.id)?;
        if !w.is_empty() {
            f = f.f("availability", av::summary(&w));
        }
    }
    if contact && ctx.guard.contacts {
        f = f
            .f("contact", &l.contact)
            .f("emergency contact", &l.emergency);
        for v in [&l.contact, &l.emergency] {
            for part in v.split([';', ',']).map(str::trim).filter(|p| !p.is_empty()) {
                f = f.require(part);
            }
        }
    }
    Ok(f)
}

/// The places a scene is shot: its Location / Set breakdown elements, with the status of the
/// location each points to; else the place in the scene heading.
fn scene_location(
    ctx: &FetchCtx<'_>,
    s: &SceneRef,
    all: &[Loc],
) -> AppResult<(Option<Fact>, Vec<String>)> {
    let mut st = ctx.conn.prepare(
        "SELECT b.display_name, c.location_id FROM breakdown_element b
         LEFT JOIN catalog_item c ON c.id = b.catalog_item_id
         WHERE b.scene_id = ?1 AND b.category = 'Location / Set' AND b.deleted_at IS NULL AND b.archived = 0
           AND b.confirmation_state <> 'Rejected' ORDER BY b.display_name",
    )?;
    let rows: Vec<(String, Option<String>)> = st
        .query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut ids = Vec::new();
    let mut parts = Vec::new();
    let mut f = Fact::new("Scene location", &s.id)
        .f("scene", s.number.to_string())
        .prio(P_HIGH);
    for (name, loc) in &rows {
        let status = loc
            .as_ref()
            .and_then(|id| all.iter().find(|l| l.id == *id))
            .map(|l| l.status.clone());
        if let Some(id) = loc {
            ids.push(id.clone());
        }
        if let Some(st) = status.as_ref().filter(|_| asked::status_asked(&ctx.message)) {
            f = f.require(st);
        }
        parts.push(match status {
            Some(s) => format!("{name} ({s})"),
            None => name.clone(),
        });
        f = f.require(name);
    }
    if parts.is_empty() {
        let h = queries::parse_heading(&s.heading);
        if h.location.is_empty() {
            return Ok((None, ids));
        }
        f = f
            .f("location (from the heading)", &h.location)
            .require(&h.location);
    } else {
        f = f.f("locations", parts.join("; "));
    }
    Ok((Some(f), ids))
}

fn tasks(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    listed: bool,
    only: Option<&str>,
) -> AppResult<Vec<Fact>> {
    let t = read.answer_type;
    let mut st = ctx.conn.prepare(
        "SELECT id, title, status, due_at, COALESCE(notes, '') FROM task WHERE deleted_at IS NULL
         ORDER BY (due_at IS NULL), due_at, position LIMIT 60",
    )?;
    let rows: Vec<(String, String, String, Option<i64>, String)> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    if rows.is_empty() {
        out.push(Fact::absent("any tasks", "Notes and Tasks"));
        return Ok(out);
    }
    let rows: Vec<_> = match only {
        Some(w) => rows
            .into_iter()
            .filter(|r| format!("{} {}", r.1, r.4).to_lowercase().contains(w))
            .collect(),
        None => rows,
    };
    if rows.is_empty() {
        return Ok(out);
    }
    let m = &ctx.m;
    let open = rows.iter().filter(|r| r.2 == "Open").count();
    let done = rows.len() - open;
    let want_done = m.any(&["done", "finished", "completed"]);
    let want_open = m.any(&["open", "pending", "outstanding", "left", "todo"]);
    if listed || t == AnswerType::Count {
        let mut f = Fact::new("Count", "")
            .f("what", "tasks in all")
            .f("value", rows.len().to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && !want_open && !want_done {
            f = f.require(rows.len().to_string());
        }
        out.push(f);
        let mut f = Fact::new("Count", "")
            .f("what", "open tasks")
            .f("value", open.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && want_open {
            f = f.require(open.to_string());
        }
        out.push(f);
        let mut f = Fact::new("Count", "")
            .f("what", "done tasks")
            .f("value", done.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && want_done {
            f = f.require(done.to_string());
        }
        out.push(f);
    }
    let due_first = m.any(&[
        "due first",
        "first due",
        "earliest",
        "next due",
        "due soon",
        "due next",
    ]);
    // What picks a row, strongest first: a date against its due date, "no due date", then the words
    // of the question in its title or notes. A status word alone filters only when nothing else does.
    let dates = dates_in(&ctx.message);
    let by_date: Vec<usize> = (0..rows.len())
        .filter(|i| {
            rows[*i]
                .3
                .is_some_and(|d| dates.iter().any(|x| date_is(&date_of(d), *x)))
        })
        .collect();
    let no_due = m.any(&[
        "no due date",
        "no due",
        "no date",
        "no deadline",
        "without a due date",
        "without a date",
    ]);
    let by_no_due: Vec<usize> = (0..rows.len())
        .filter(|i| no_due && rows[*i].3.is_none())
        .collect();
    let words = row_words(&ctx.message, TASK_WORDS);
    let texts: Vec<String> = rows
        .iter()
        .map(|r| format!("{} {}", r.1, r.4))
        .collect();
    let by_word = best_rows(&words, &texts, ROWS_PICKED);
    let picked: Vec<usize> = if !by_date.is_empty() {
        by_date
    } else if !by_no_due.is_empty() {
        by_no_due
    } else {
        by_word
    };
    let mut first_open = true;
    for (i, (id, title, status, due, notes)) in rows.iter().enumerate() {
        let hit = picked.contains(&i);
        let shown = hit
            || (picked.is_empty()
                && ((want_done && status == "Done")
                    || (want_open && status == "Open")
                    || (!want_done && !want_open)));
        let mut f = Fact::new("Task", id)
            .f("title", title)
            .f("status", status)
            .f("due", due.map(date_of).unwrap_or_default())
            .f("notes", clip(notes, 200))
            .prio(if hit {
                P_PICKED
            } else if shown {
                P_HIGH
            } else {
                P_LOW
            });
        if hit {
            if !names_row(&ctx.message, title) {
                f = f.require(title);
            }
            if asked::status_asked(&ctx.message) {
                f = f.require(status);
            }
        } else if picked.is_empty()
            && shown
            && (want_done || want_open)
            && matches!(t, AnswerType::List | AnswerType::Lookup)
        {
            f = f.require(title);
        }
        if due_first && status == "Open" && due.is_some() && first_open {
            f = f.require(title);
            first_open = false;
        }
        out.push(f);
    }
    Ok(out)
}
