//! Files: Project Files (names, folders, the note on each, and the words of a small text file kept when
//! it was added) and Project Notes. A file is named, never located, and never opened here: the asset
//! path and the source field of a file are not read. Reads `project_file`, `project_file_folder`,
//! `search_doc` and `project_note`.

use std::collections::HashMap;

use straft_domain::AppResult;

use crate::modules::ai::dossier::asked::{best_rows, names_row, row_words};
use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::guard;
use crate::modules::ai::dossier::words::{Msg, clip};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID, P_PICKED};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};

/// Words that only name the file list.
const FILE_WORDS: &[&str] = &[
    "file",
    "files",
    "folder",
    "folders",
    "attachment",
    "attachments",
    "say",
    "says",
    "letter",
    "document",
    "mention",
    "mentions",
];
/// The text of one file given to the writer.
const FILE_TEXT_CHARS: usize = 900;
/// A note this long is told from its text; a longer one is read, not demanded word for word.
const NOTE_DEMANDED_CHARS: usize = 400;

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
    let stored = indexed_text(ctx)?;
    let words = row_words(&ctx.message, FILE_WORDS);
    let hay: Vec<String> = rows
        .iter()
        .map(|(id, name, _, notes)| {
            format!(
                "{name} {notes} {}",
                stored.get(id).map_or("", String::as_str)
            )
        })
        .collect();
    let picked = best_rows(&words, &hay, 3);
    let asks_which = m.any(&["which file", "what file", "which files", "what files"]);
    // "What does the letter say" is retold from its words.
    let asks_said = Msg::new(&ctx.message).any(&["say", "says", "said"]);
    if t == AnswerType::Count {
        out.push(
            Fact::new("Count", "")
                .f("what", "files in Project Files")
                .f("value", rows.len().to_string())
                .require(rows.len().to_string())
                .prio(P_HIGH),
        );
    }
    for (i, (id, name, folder, notes)) in rows.iter().enumerate().take(80) {
        let by_folder = folder_named.contains(folder);
        let is_named = named.contains(&id.as_str());
        let by_word = picked.contains(&i);
        let mut f = Fact::new("File", id)
            .f("name", name)
            .f("folder", folder)
            .prio(if by_word {
                P_PICKED
            } else if by_folder || is_named {
                P_HIGH
            } else {
                P_LOW
            });
        if by_folder || is_named || by_word {
            f = f.f("note", clip(notes, 200));
            if let Some(words) = stored.get(id) {
                f = f.f("text", clip(words, FILE_TEXT_CHARS));
                if asks_said {
                    f = demand_sentences(f, words);
                }
            }
            if matches!(t, AnswerType::List | AnswerType::Lookup)
                && (by_folder || (asks_which && by_word))
            {
                f = f.require(name);
            }
        }
        out.push(f);
    }
    Ok(out)
}

/// A short text is told by its sentences. A sentence that reads like an instruction is data, and is
/// never demanded back.
fn demand_sentences(mut f: Fact, text: &str) -> Fact {
    if text.chars().count() > NOTE_DEMANDED_CHARS {
        return f;
    }
    for sentence in text.split_inclusive(['.', '!', '?']) {
        let sentence = sentence.trim().trim_end_matches(['.', '!', '?']);
        if sentence.chars().count() >= 8 && !guard::looks_like_instruction(sentence) {
            f = f.require(sentence);
        }
    }
    f
}

/// The words of each small text file, for the files the search index holds under the owner rule. They
/// were kept when the file was added: nothing is opened here.
fn indexed_text(ctx: &FetchCtx<'_>) -> AppResult<HashMap<String, String>> {
    let mut st = ctx.conn.prepare(
        "SELECT f.id, f.search_text FROM project_file f
         JOIN search_doc d ON d.source_table = 'project_file' AND d.entity_id = f.id
         WHERE f.deleted_at IS NULL AND f.search_text IS NOT NULL
           AND (d.owner_user_id IS NULL OR d.owner_user_id = ?1)",
    )?;
    let rows = st.query_map([&ctx.actor.user_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The notes the message names, by their whole title ("Note 0001") or by most of its words ("the
/// dialect notes"). The person knows the title, so its text is what is demanded, sentence by sentence;
/// a sentence that reads like an instruction is data and is never demanded back.
fn notes_by_title(ctx: &FetchCtx<'_>, limit: usize) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(title, ''), body FROM project_note WHERE deleted_at IS NULL
         ORDER BY pinned DESC, updated_at DESC LIMIT 6000",
    )?;
    let rows: Vec<(String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let typed = Msg::new(&ctx.message);
    let asks_note = typed.stem("note") || typed.any(&["say", "says", "plan", "rule", "rules"]);
    Ok(rows
        .into_iter()
        .filter(|(_, title, _)| {
            let n = normalize(title);
            !n.is_empty() && (ctx.m.has(&n) || (asks_note && names_row(&ctx.message, title)))
        })
        .take(limit)
        .map(|(id, title, body)| {
            let f = Fact::new("Project note", id)
                .f("title", &title)
                .f("text", clip(&body, NOTE_DEMANDED_CHARS))
                .prio(P_HIGH);
            demand_sentences(f, &body)
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
