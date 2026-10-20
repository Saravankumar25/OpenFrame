//! The Story page (owner request 2026-10-20): one free page per project to write the story in
//! prose, like a notepad. It has no link to the Beat Board, the screenplay or any other page:
//! the text is stored as it is typed and read back as it was saved. Export writes the page as
//! plain text, PDF or a Word document.
//!
//! Saving is a project change like any other (undoable, in the activity history) and is refused
//! when the text was changed elsewhere since it was read (`base_rev`), so a second window never
//! overwrites the first one's words.

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use openframe_import_export::report::{Block, Document};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::modules::exports::{
    ExportResult, Fmt, HINT_TEXT, Job, Output, deliver, destination, footer, require_export,
};
use crate::modules::schedule::board::project_title;
use crate::registry::Registry;
use crate::store::MutationMeta;

/// The longest page kept (characters); about 300,000 words.
pub(crate) const BODY_MAX_CHARS: usize = 2_000_000;

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.query("story.page", get)
        .meta(M::read("The text of the project's Story page."));
    r.command("story.save_page", save)
        .meta(M::edit("Save the text of the project's Story page."));
    r.query("story.export_page", export).meta(
        M::query(
            Capability::Export,
            "Export the Story page as plain text, PDF or a Word document to a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .hidden(h::USER_PATH),
    );
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryPageDto {
    pub body: String,
    /// 0 while nothing has been saved yet.
    #[ts(type = "number")]
    pub rev: i64,
    #[ts(type = "number | null")]
    pub updated_at: Option<i64>,
    #[ts(type = "number")]
    pub words: i64,
    pub can_edit: bool,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryPageArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorySavePageArgs {
    pub body: String,
    /// The revision the text was read at; a save over a newer revision is refused.
    #[serde(default)]
    #[ts(optional, type = "number")]
    pub base_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStoryPageArgs {
    /// "txt" | "pdf" | "docx"
    pub format: String,
    pub path: String,
}

/// Words in the text (runs of letters or digits), for the count under the page.
pub fn word_count(text: &str) -> i64 {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '\u{2019}')
        .filter(|w| w.chars().any(|c| c.is_alphanumeric()))
        .count() as i64
}

fn load(c: &Connection) -> AppResult<Option<(String, String, i64, i64)>> {
    Ok(c.query_row(
        "SELECT id, body, rev, updated_at FROM story_page ORDER BY created_at LIMIT 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()?)
}

fn dto(row: Option<(String, String, i64, i64)>, can_edit: bool) -> StoryPageDto {
    match row {
        Some((_, body, rev, at)) => StoryPageDto {
            words: word_count(&body),
            body,
            rev,
            updated_at: Some(at),
            can_edit,
        },
        None => StoryPageDto {
            body: String::new(),
            rev: 0,
            updated_at: None,
            words: 0,
            can_edit,
        },
    }
}

fn get(core: &AppCore, actor: &Actor, _a: StoryPageArgs) -> AppResult<StoryPageDto> {
    actor.require(Capability::View, "read the Story page")?;
    let can_edit = actor.can(Capability::Edit);
    core.project()?.store.read(|c| Ok(dto(load(c)?, can_edit)))
}

fn save(core: &AppCore, actor: &Actor, a: StorySavePageArgs) -> AppResult<StoryPageDto> {
    if a.body.chars().count() > BODY_MAX_CHARS {
        return Err(AppError::validation(
            "body",
            "The Story page is too long to save. Move part of it into another document.",
        ));
    }
    let body = a.body.replace("\r\n", "\n");
    let session = core.project()?;
    let meta = MutationMeta::new("story.save_page", "Edited the Story page", Capability::Edit);
    session.store.mutate(actor, meta, |tx| {
        let c = tx.conn();
        let now = now_ms();
        match load(c)? {
            Some((id, old, rev, _)) => {
                if let Some(base) = a.base_rev
                    && base != rev
                {
                    return Err(AppError::new(
                        "conflict.stale",
                        "The Story page was changed in another window. Reload it before saving.",
                    ));
                }
                if old != body {
                    c.execute(
                        "UPDATE story_page SET body = ?1, updated_at = ?2, rev = rev + 1 WHERE id = ?3",
                        params![body, now, id],
                    )?;
                }
            }
            None => {
                c.execute(
                    "INSERT INTO story_page(id, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                    params![openframe_domain::new_id(), body, now],
                )?;
            }
        }
        Ok(())
    })?;
    session.store.read(|c| Ok(dto(load(c)?, true)))
}

fn export(core: &AppCore, actor: &Actor, a: ExportStoryPageArgs) -> AppResult<ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Txt, Fmt::Pdf, Fmt::Docx])?;
    let dest = destination(core, &a.path, fmt)?;
    let (project, page) = core
        .project()?
        .store
        .read(|c| Ok((project_title(c)?, dto(load(c)?, false))))?;
    if page.body.trim().is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "The Story page is empty. Write something first.",
        ));
    }
    let title = if project.trim().is_empty() {
        "Story".to_string()
    } else {
        project.trim().to_string()
    };
    let paragraphs = openframe_import_export::docx::prose_paragraphs(&page.body);
    let mut doc = Document::new("Story").footer(footer(&title, "Story"));
    doc.push(Block::Heading {
        text: title.clone(),
        level: 1,
    });
    for p in &paragraphs {
        doc.push(Block::Paragraph(p.clone()));
    }
    let mut out = Output::new(doc, HINT_TEXT);
    out.text = Some(format!("{title}\n\n{}", paragraphs.join("\n\n")));
    out.brand_text = false;
    if fmt == Fmt::Docx {
        out.docx = Some(openframe_import_export::docx::prose_to_bytes(
            &title, &page.body,
        )?);
    }
    out.revised_at = page.updated_at;
    let job = Job {
        action: "story.export_page",
        document: "Story".into(),
        source_label: "Story page".into(),
        scope_label: "Whole page".into(),
        contents_label: format!("{} words", page.words),
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_counted_in_any_script() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("  The sea, before the story.  "), 5);
        assert_eq!(word_count("Anjali's reel - 14-11-1999"), 5);
        assert_eq!(word_count("అంజలి రేడియో"), 2);
    }
}
