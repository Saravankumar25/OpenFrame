//! Story page export (Story > Story tab): the writer's free text as plain text, PDF or Word.
//!
//! The page is a manuscript, so none of these files carries the OpenFrame brand line (see
//! `Output::unbranded`). TXT is the text exactly as written with Windows line endings. PDF and
//! DOCX print the heading "Story" and then one paragraph per paragraph of text; line breaks inside
//! a paragraph are kept and blank lines separate paragraphs. An empty page gives a valid document
//! that holds only the heading.

use openframe_domain::{Actor, AppResult};
use openframe_import_export::docx::{prose_paragraphs, prose_to_bytes};
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
    let text = page.text;

    let mut doc = Document::new(DOCUMENT)
        .footer(footer(&project, DOCUMENT))
        .subtitle(subtitle(&project, None));
    for p in prose_paragraphs(&text) {
        doc.push(if p.is_empty() {
            Block::Spacer(10.0)
        } else {
            Block::Paragraph(p)
        });
    }
    let mut out = Output::new(doc, PDF_HINT);
    out.unbranded = true;
    out.text = Some(text.replace("\r\n", "\n"));
    if fmt == Fmt::Docx {
        out.docx = Some(prose_to_bytes(DOCUMENT, &text)?);
    }
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
