//! Files: Project Files (names, folders, the note on each) and Project Notes. A file is named, never
//! located: the asset path and the source field of a file are not read. Reads `project_file`,
//! `project_file_folder` and `project_note`.

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("files", || fetch(ctx, read, r))
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out = Vec::new();
    let asks_files =
        m.stem("file") || m.stem("folder") || m.stem("attachment") || !r.files.is_empty();
    if asks_files {
        out.extend(files(ctx, read, r)?);
    }
    // Rules, safety plans and permits are written in project notes.
    if m.any(&[
        "rules",
        "rule",
        "safety",
        "policy",
        "restrictions",
        "requirements",
        "plan",
    ]) && !m.stem("note")
    {
        let words = m.keywords();
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        out.extend(notes_matching(ctx, &refs, 3)?);
    }
    if m.stem("note") && !m.any(&["private note", "private notes"]) {
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
            if t != AnswerType::Count {
                let words = m.keywords();
                let refs: Vec<&str> = words.iter().map(String::as_str).collect();
                out.extend(notes_matching(ctx, &refs, 5)?);
            }
        }
    }
    Ok(out)
}

fn files(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut st = ctx.conn.prepare(
        "SELECT f.id, f.display_name, COALESCE(d.name, ''), COALESCE(f.notes, '')
         FROM project_file f LEFT JOIN project_file_folder d ON d.id = f.folder_id AND d.deleted_at IS NULL
         WHERE f.deleted_at IS NULL ORDER BY d.name, f.position, f.id LIMIT 300",
    )?;
    let rows: Vec<(String, String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent("any files", "Project Files")]);
    }
    let mut out = Vec::new();
    let folder_named = rows
        .iter()
        .map(|r| r.2.clone())
        .filter(|f| !f.is_empty() && m.has(&normalize(f)))
        .collect::<std::collections::BTreeSet<_>>();
    let named: Vec<&str> = r.files.iter().map(|h| h.id.as_str()).collect();
    if t == AnswerType::Count {
        out.push(
            Fact::new("Count", "")
                .f("what", "files in Project Files")
                .f("value", rows.len().to_string())
                .require(rows.len().to_string())
                .prio(P_HIGH),
        );
    }
    for (id, name, folder, notes) in rows.iter().take(80) {
        let by_folder = folder_named.contains(folder);
        let is_named = named.contains(&id.as_str());
        let mut f = Fact::new("File", id)
            .f("name", name)
            .f("folder", folder)
            .prio(if by_folder || is_named { P_HIGH } else { P_LOW });
        if by_folder || is_named {
            f = f.f("note", clip(notes, 200));
            if matches!(t, AnswerType::List | AnswerType::Lookup) && by_folder {
                f = f.require(name);
            }
        }
        out.push(f);
    }
    Ok(out)
}

/// The notes whose title the message spells out ("Note 0001").
fn notes_by_title(ctx: &FetchCtx<'_>, limit: usize) -> AppResult<Vec<Fact>> {
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
