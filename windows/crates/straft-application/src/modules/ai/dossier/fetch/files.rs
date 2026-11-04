//! Files: Project Files (names, folders, the note on each); the old by-domain path also reads Project
//! Notes (the Notes leaf is `notes.rs`). A file is named, never
//! located: the asset path and the source field of a file are not read. Reads `project_file`,
//! `project_file_folder` and `project_note`.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};

/// Files: the files and folders of the project with the note on each.
pub fn files_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    let ctx = &ctx.topic(&["files"]);
    safe("files", || files(ctx, read, r))
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
        // With a handful of files, what is written about each one is what the question is about.
        let with_note = by_folder || is_named || rows.len() <= 12;
        let mut f = Fact::new("File", id)
            .f("name", name)
            .f("folder", folder)
            .prio(if with_note { P_HIGH } else { P_LOW });
        if with_note {
            f = f.f("note", clip(notes, 200));
            if matches!(t, AnswerType::List | AnswerType::Lookup) && (by_folder || rows.len() <= 12)
            {
                f = f.require(name);
            }
        }
        out.push(f);
    }
    Ok(out)
}
