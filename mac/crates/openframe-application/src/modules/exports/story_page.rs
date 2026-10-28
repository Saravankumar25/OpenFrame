//! Story page export (Story > Story tab): the writer's free text as plain text, PDF or Word.
//!
//! The page is a manuscript, so none of these files carries the OpenFrame brand line (see
//! `Output::unbranded`). TXT is the text exactly as written with Windows line endings (without
//! formatting). PDF and DOCX print the heading "Story" and then every line of the page as it was
//! written, with its bold, italic, underline, highlight and sizes; blank lines keep their height.
//! An empty page gives a valid document that holds only the heading.

use openframe_domain::{Actor, AppResult};
use openframe_import_export::docx::prose_lines_to_bytes;
use openframe_import_export::report::{Block, Document};
use serde::Deserialize;
use ts_rs::TS;

use super::{Fmt, Job, Output, deliver, destination, footer, plural, require_export, subtitle};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::story::page;

const DOCUMENT: &str = "Story";
const PDF_HINT: &str = "Export it as plain text or Word to keep them, or remove those characters and export the PDF again.";

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStoryPageArgs {
    /// "txt" | "pdf" | "docx"
    pub format: String,
    /// Absolute destination chosen in the Save dialog.
    pub path: String,
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportStoryPageArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Txt, Fmt::Pdf, Fmt::Docx])?;
    let dest = destination(core, &a.path, fmt)?;
    let (project, page) = core
        .project()?
        .store
        .read(|c| Ok((project_title(c)?, page::read(c)?)))?;
    let mut lines = page::lines(&page);
    // Trailing empty lines print nothing.
    while lines.len() > 1
        && lines
            .last()
            .is_some_and(|l| l.iter().all(|r| r.t.is_empty()))
    {
        lines.pop();
    }
    let text = page.text;
    let docx = if fmt == Fmt::Docx {
        Some(prose_lines_to_bytes(DOCUMENT, &lines)?)
    } else {
        None
    };

    let mut doc = Document::new(DOCUMENT)
        .a4()
        .footer(footer(&project, DOCUMENT))
        .subtitle(subtitle(&project, None));
    doc.push(Block::Prose(lines));
    let mut out = Output::new(doc, PDF_HINT);
    out.unbranded = true;
    out.text = Some(text.replace("\r\n", "\n"));
    out.docx = docx;
    let words = text.split_whitespace().count();
    deliver(
        core,
        actor,
        &dest,
        fmt,
        Job {
            action: "story.page.export",
            document: DOCUMENT.into(),
            source_label: "Story page".into(),
            scope_label: "The whole page".into(),
            contents_label: plural(words, "word", "words"),
            target: None,
        },
        out,
    )
}
