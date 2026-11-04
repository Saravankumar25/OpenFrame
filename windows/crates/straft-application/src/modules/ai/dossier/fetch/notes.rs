//! Notes & Tasks: Project Notes, the person's own private notes, and Tasks.
//! Reads `project_note`, `private_note` (the actor's only) and `task`.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, date_of};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};

/// Notes & Tasks > Notes: how many project notes there are, the ones the message spells out or
/// matches, and the person's own private notes (nobody else's, ever).
pub fn notes_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    let ctx = &ctx.topic(&["notes"]);
    safe("notes", || notes(ctx, read, r))
}

/// Notes & Tasks > Tasks: open and done counts and the tasks with their due dates.
pub fn tasks_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let ctx = &ctx.topic(&["tasks"]);
    safe("tasks", || tasks(ctx, read, true, None))
}

fn notes(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out = Vec::new();
    let n: i64 = ctx.conn.query_row(
        "SELECT count(*) FROM project_note WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    if n == 0 {
        out.push(Fact::absent("any project notes", "Notes and Tasks"));
    } else {
        let mut f = Fact::new("Count", "")
            .f("what", "project notes")
            .f("value", n.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count {
            f = f.require(n.to_string());
        }
        out.push(f);
        out.extend(notes_by_title(ctx, 4)?);
        if m.any(&["pinned", "pin"]) {
            out.extend(pinned_notes(ctx)?);
        }
        if t != AnswerType::Count {
            let words = m.keywords();
            let refs: Vec<&str> = words.iter().map(String::as_str).collect();
            out.extend(notes_matching(ctx, &refs, 5)?);
        }
    }
    out.extend(private_notes(ctx)?);
    Ok(out)
}

/// The person's own private notes, newest first. Another person's are not in reach of the query.
fn private_notes(ctx: &FetchCtx<'_>) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, body FROM private_note WHERE owner_user_id = ?1 AND deleted_at IS NULL
         ORDER BY updated_at DESC LIMIT 6",
    )?;
    let rows: Vec<(String, String)> = st
        .query_map([&ctx.actor.user_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, body)| {
            Fact::new("Your private note", id)
                .f("text", clip(&body, 300))
                .private_to(&ctx.actor.user_id)
                .prio(P_MID)
        })
        .collect())
}

pub fn tasks(
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
    if listed || t == AnswerType::Count {
        let mut f = Fact::new("Count", "")
            .f("what", "open tasks")
            .f("value", open.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && !m.has("done") {
            f = f.require(open.to_string());
        }
        out.push(f);
        let mut f = Fact::new("Count", "")
            .f("what", "done tasks")
            .f("value", done.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && m.has("done") {
            f = f.require(done.to_string());
        }
        out.push(f);
    }
    let want_done = m.any(&["done", "finished", "completed"]);
    let want_open = m.any(&["open", "pending", "outstanding", "left", "todo"]);
    let due_first = m.any(&[
        "due first",
        "first due",
        "earliest",
        "next due",
        "due soon",
        "due next",
    ]);
    let mut first_open = true;
    for (id, title, status, due, notes) in &rows {
        let shown = (want_done && status == "Done")
            || (want_open && status == "Open")
            || (!want_done && !want_open);
        let mut f = Fact::new("Task", id)
            .f("title", title)
            .f("status", status)
            .f("due", due.map(date_of).unwrap_or_default())
            .f("notes", clip(notes, 200))
            .prio(if shown { P_HIGH } else { P_LOW });
        if shown && (want_done || want_open) && matches!(t, AnswerType::List | AnswerType::Lookup) {
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

/// The notes a person pinned to the top of the list.
fn pinned_notes(ctx: &FetchCtx<'_>) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(title, ''), body FROM project_note WHERE deleted_at IS NULL AND pinned = 1
         ORDER BY updated_at DESC LIMIT 6",
    )?;
    let rows: Vec<(String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, title, body)| {
            Fact::new("Project note", id)
                .f("title", &title)
                .f("pinned", "yes")
                .f("text", clip(&body, 360))
                .prio(P_HIGH)
                .require(&title)
        })
        .collect())
}

/// The notes whose title the message spells out ("Note 0001").
pub fn notes_by_title(ctx: &FetchCtx<'_>, limit: usize) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(title, ''), body FROM project_note WHERE deleted_at IS NULL
         ORDER BY pinned DESC, updated_at DESC LIMIT 6000",
    )?;
    let rows: Vec<(String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, title, _)| {
            let n = normalize(title);
            !n.is_empty() && ctx.m.has(&n)
        })
        .take(limit)
        .map(|(id, title, body)| {
            Fact::new("Project note", id)
                .f("title", &title)
                .f("text", clip(&body, 360))
                .prio(P_HIGH)
                .require(&title)
        })
        .collect())
}

/// Project notes with any of the words in the title or the text.
pub fn notes_matching(ctx: &FetchCtx<'_>, words: &[&str], limit: usize) -> AppResult<Vec<Fact>> {
    let words: Vec<String> = words
        .iter()
        .filter(|w| w.chars().count() >= 3)
        .map(|w| format!("%{}%", w.trim_end_matches('s').replace(['%', '_'], "")))
        .take(6)
        .collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let clause = words
        .iter()
        .enumerate()
        .map(|(i, _)| format!("(title LIKE ?{n} OR body LIKE ?{n})", n = i + 1))
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!(
        "SELECT id, COALESCE(title, ''), body FROM project_note WHERE deleted_at IS NULL AND ({clause})
         ORDER BY pinned DESC, updated_at DESC LIMIT {limit}"
    );
    let mut st = ctx.conn.prepare(&sql)?;
    let rows: Vec<(String, String, String)> = st
        .query_map(rusqlite::params_from_iter(words.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .map(|(id, title, body)| {
            Fact::new("Project note", id)
                .f("title", title)
                .f("text", clip(&body, 360))
                .prio(P_MID)
        })
        .collect())
}
