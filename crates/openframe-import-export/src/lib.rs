//! OpenFrame import/export engine (Import/Export spec; FSD §19, §20, §60,
//! §117, §122–123; FSD-SCRIPT-038..050).
//!
//! A pure library — no database, no application state. The application layer
//! (`openframe-application::modules::interchange`) turns a parsed
//! [`ScreenplayDoc`] into screenplay rows after an explicit preview/confirm
//! step, and reads a draft back into a `ScreenplayDoc` for export.
//!
//! ```text
//!  import:  file / pasted text ─► detect format ─► parser ─► ScreenplayDoc + warnings + confidence
//!  export:  ScreenplayDoc ─► writer (PDF · FDX · Fountain · DOCX · TXT) ─► atomic file write
//!  reports: report::Document (headings, tables) ─► PDF · CSV · XLSX
//! ```
//!
//! Every parser is defensive: size limits, safe ZIP handling (via
//! `openframe_security::archive`), no XML entity expansion, bounded XML depth,
//! panic isolation around the PDF parser, and human `import.*` errors.

pub mod docx;
pub mod fdx;
pub mod fountain;
pub mod heuristics;
pub mod layout;
pub mod model;
pub mod pdf_export;
pub mod pdf_import;
pub mod pdfgen;
pub mod report;
pub mod text;
pub mod winansi;
pub mod xml;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult};
use serde::Serialize;

pub use heuristics::{Confidence, ConfidenceLevel};
pub use layout::{Layout, LayoutOptions};
pub use model::{Element, ElementKind, Scene, ScreenplayDoc, TitlePage, Warning, WarningLevel};

/// Largest screenplay source file accepted for import.
pub const MAX_IMPORT_BYTES: u64 = 100 << 20;
/// Largest pasted/plain text accepted.
pub const MAX_TEXT_BYTES: usize = 20 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Pdf,
    Fdx,
    Fountain,
    Txt,
    Docx,
    Pasted,
}

impl SourceFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceFormat::Pdf => "pdf",
            SourceFormat::Fdx => "fdx",
            SourceFormat::Fountain => "fountain",
            SourceFormat::Txt => "txt",
            SourceFormat::Docx => "docx",
            SourceFormat::Pasted => "pasted",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SourceFormat::Pdf => "PDF",
            SourceFormat::Fdx => "Final Draft (.fdx)",
            SourceFormat::Fountain => "Fountain",
            SourceFormat::Txt => "Plain text",
            SourceFormat::Docx => "Word document (.docx)",
            SourceFormat::Pasted => "Pasted text",
        }
    }
    pub fn parse(s: &str) -> Option<SourceFormat> {
        Some(match s.to_ascii_lowercase().as_str() {
            "pdf" => SourceFormat::Pdf,
            "fdx" => SourceFormat::Fdx,
            "fountain" | "spmd" => SourceFormat::Fountain,
            "txt" | "text" => SourceFormat::Txt,
            "docx" => SourceFormat::Docx,
            "pasted" | "paste" => SourceFormat::Pasted,
            _ => return None,
        })
    }
}

/// Result of parsing a source: the document (with warnings), how sure we are,
/// and the format that was actually read.
#[derive(Debug, Clone)]
pub struct ImportOutcome {
    pub doc: ScreenplayDoc,
    pub confidence: Confidence,
    pub format: SourceFormat,
}

/// Standard "could not interpret" failure for a format.
pub fn not_a_screenplay(format: SourceFormat) -> AppError {
    let what = match format {
        SourceFormat::Pdf => "The PDF",
        SourceFormat::Docx => "The Word document",
        SourceFormat::Fdx => "The Final Draft file",
        SourceFormat::Fountain => "The Fountain file",
        SourceFormat::Txt => "The text file",
        SourceFormat::Pasted => "The pasted text",
    };
    AppError::import(
        "not_a_screenplay",
        format!(
            "{what} could not be interpreted as a screenplay. Your current project was not changed."
        ),
    )
    .with_detail(format!(
        "no scene headings or character cues found ({})",
        format.as_str()
    ))
}

fn unsupported(message: &str, detail: String) -> AppError {
    AppError::import("unsupported_format", message.to_string()).with_detail(detail)
}

/// Decide the format from the extension and the first bytes (content wins when unambiguous).
pub fn detect_format(path: &Path, head: &[u8]) -> AppResult<SourceFormat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let head_str = String::from_utf8_lossy(&head[..head.len().min(2048)]);
    if head.starts_with(b"%PDF-") {
        return Ok(SourceFormat::Pdf);
    }
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        return match ext.as_str() {
            "docx" | "" => Ok(SourceFormat::Docx),
            "fdx" => Ok(SourceFormat::Fdx), // will fail as malformed XML with a clear message
            _ => Ok(SourceFormat::Docx),
        };
    }
    if head.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
        return Err(unsupported(
            "Older Word documents (.doc) can't be read. Open it in Word, save it as .docx, and import that file. Your current project was not changed.",
            format!("ole2 container, ext {ext}"),
        ));
    }
    if head_str.contains("<FinalDraft") {
        return Ok(SourceFormat::Fdx);
    }
    match ext.as_str() {
        "pdf" => Ok(SourceFormat::Pdf),
        "fdx" => Ok(SourceFormat::Fdx),
        "docx" => Ok(SourceFormat::Docx),
        "fountain" | "spmd" => Ok(SourceFormat::Fountain),
        "txt" | "text" | "md" | "" => Ok(SourceFormat::Txt),
        "doc" | "rtf" | "celtx" | "wps" | "pages" | "odt" | "sexp" | "fadein" | "highland" => {
            Err(unsupported(
                "This file type can't be imported as a screenplay. Supported types are PDF, Final Draft (.fdx), Fountain, TXT and DOCX. Your current project was not changed.",
                format!("extension {ext}"),
            ))
        }
        _ => {
            if std::str::from_utf8(&head[..head.len().min(4096)]).is_ok() {
                Ok(SourceFormat::Txt)
            } else {
                Err(unsupported(
                    "This file type can't be imported as a screenplay. Supported types are PDF, Final Draft (.fdx), Fountain, TXT and DOCX. Your current project was not changed.",
                    format!("unknown binary, extension {ext}"),
                ))
            }
        }
    }
}

/// Parse a screenplay file. Nothing is written anywhere.
pub fn import_file(path: &Path, hint: Option<SourceFormat>) -> AppResult<ImportOutcome> {
    let size = openframe_security::validate_input_file(path, MAX_IMPORT_BYTES)?;
    if size == 0 {
        return Err(AppError::import(
            "empty_source",
            "This file is empty. Your current project was not changed.",
        ));
    }
    let mut head = vec![0u8; 4096];
    let n = {
        use std::io::Read;
        let mut f = std::fs::File::open(path)?;
        f.read(&mut head)?
    };
    head.truncate(n);
    let detected = detect_format(path, &head)?;
    // The content decides for binary formats; an explicit hint can pick Fountain vs TXT.
    let format = match (hint, detected) {
        (Some(SourceFormat::Fountain), SourceFormat::Txt) => SourceFormat::Fountain,
        (Some(SourceFormat::Txt), SourceFormat::Fountain) => SourceFormat::Txt,
        _ => detected,
    };
    match format {
        SourceFormat::Docx => docx::parse(path),
        other => {
            let bytes = std::fs::read(path)?;
            import_bytes(&bytes, other)
        }
    }
}

/// Parse in-memory bytes of a non-archive format.
pub fn import_bytes(bytes: &[u8], format: SourceFormat) -> AppResult<ImportOutcome> {
    match format {
        SourceFormat::Pdf => pdf_import::parse(bytes),
        SourceFormat::Fdx => fdx::parse(bytes),
        SourceFormat::Docx => Err(AppError::internal("docx must be parsed from a file")),
        SourceFormat::Fountain | SourceFormat::Txt | SourceFormat::Pasted => {
            if bytes.len() > MAX_TEXT_BYTES {
                return Err(AppError::import(
                    "too_large",
                    "This text is too long to be a screenplay. Your current project was not changed.",
                ));
            }
            let (text, warn) = text::decode_text(bytes);
            let mut out = if format == SourceFormat::Fountain {
                fountain::parse(&text)?
            } else {
                text::parse(&text, format)?
            };
            if let Some(w) = warn {
                out.doc.warnings.insert(0, w);
            }
            Ok(out)
        }
    }
}

/// Parse pasted screenplay text (same heuristics as TXT; Fountain when asked).
/// Pasted Fountain is recognised by its title page (`Title:`, `Author:` … as
/// the first line). Plain-text heuristics would otherwise turn the title page
/// into action lines of the first scene.
fn starts_with_fountain_title_page(text: &str) -> bool {
    const KEYS: &[&str] = &[
        "title",
        "credit",
        "author",
        "authors",
        "source",
        "draft date",
        "date",
        "contact",
        "copyright",
        "notes",
    ];
    let Some(first) = text.lines().map(str::trim).find(|l| !l.is_empty()) else {
        return false;
    };
    let Some((key, _)) = first.split_once(':') else {
        return false;
    };
    let key = key.trim().to_ascii_lowercase();
    KEYS.contains(&key.as_str())
}

pub fn import_text(text: &str, as_fountain: bool) -> AppResult<ImportOutcome> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(AppError::import(
            "too_large",
            "This text is too long to be a screenplay. Your current project was not changed.",
        ));
    }
    if as_fountain || starts_with_fountain_title_page(text) {
        let mut o = fountain::parse(text)?;
        o.format = SourceFormat::Pasted;
        Ok(o)
    } else {
        text::parse(text, SourceFormat::Pasted)
    }
}

// ------------------------------------------------------------------ export

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Pdf,
    Fdx,
    Fountain,
    Docx,
    Txt,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Option<ExportFormat> {
        Some(match s.to_ascii_lowercase().as_str() {
            "pdf" => ExportFormat::Pdf,
            "fdx" => ExportFormat::Fdx,
            "fountain" => ExportFormat::Fountain,
            "docx" => ExportFormat::Docx,
            "txt" => ExportFormat::Txt,
            _ => return None,
        })
    }
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Pdf => "pdf",
            ExportFormat::Fdx => "fdx",
            ExportFormat::Fountain => "fountain",
            ExportFormat::Docx => "docx",
            ExportFormat::Txt => "txt",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Pdf => "PDF",
            ExportFormat::Fdx => "Final Draft (.fdx)",
            ExportFormat::Fountain => "Fountain",
            ExportFormat::Docx => "DOCX",
            ExportFormat::Txt => "Plain text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
    pub title_page: bool,
    pub scene_numbers: bool,
    pub include_notes: bool,
    pub revision_marks: bool,
    pub revision_info: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            title_page: true,
            scene_numbers: false,
            include_notes: false,
            revision_marks: false,
            revision_info: true,
        }
    }
}

impl ExportOptions {
    pub fn layout(&self) -> LayoutOptions {
        LayoutOptions {
            title_page: self.title_page,
            scene_numbers: self.scene_numbers,
            include_notes: self.include_notes,
            revision_marks: self.revision_marks,
            revision_info: self.revision_info,
            page_numbers: true,
            paginate: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExportOutcome {
    pub path: PathBuf,
    pub bytes_written: u64,
    /// Script pages for paginated formats.
    pub pages: Option<usize>,
    pub warnings: Vec<Warning>,
}

/// Write bytes atomically: temporary file in the destination folder, flush,
/// then rename over the destination. A failure never leaves a partial file.
pub fn write_atomic(dest: &Path, bytes: &[u8]) -> AppResult<()> {
    let dir = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("export");
    let tmp = dir.join(format!(".{name}.{}.partial", std::process::id()));
    let result = (|| -> AppResult<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn export_warnings(
    doc: &ScreenplayDoc,
    format: ExportFormat,
    opts: &ExportOptions,
) -> Vec<Warning> {
    let mut w = Vec::new();
    let has = |k: ElementKind| {
        doc.scenes
            .iter()
            .any(|s| s.elements.iter().any(|e| e.kind == k))
    };
    if opts.include_notes
        && (has(ElementKind::Section) || has(ElementKind::Synopsis))
        && format != ExportFormat::Fountain
    {
        w.push(Warning::info("outline_omitted", "Sections and synopses are planning material and were left out of the screenplay document."));
    }
    let revised = doc
        .scenes
        .iter()
        .any(|s| s.elements.iter().any(|e| e.revision.is_some()));
    if opts.revision_marks
        && revised
        && matches!(format, ExportFormat::Fountain | ExportFormat::Txt)
    {
        w.push(Warning::info(
            "revision_marks_unsupported",
            format!(
                "{} has no way to mark revised lines, so revision marks were left out.",
                format.label()
            ),
        ));
    }
    if opts.include_notes && has(ElementKind::Note) && format == ExportFormat::Fdx {
        w.push(Warning::info("notes_as_text", "Notes were written as bracketed “[Note: …]” paragraphs because Final Draft notes can't be created from here."));
    }
    w
}

/// Characters on the laid-out pages that the standard screenplay PDF font
/// (base-14 Courier, WinAnsi) cannot print, in first-appearance order, with
/// the 1-based script page where each first appears (None for the title page).
pub fn unsupported_pdf_characters(lay: &Layout) -> Vec<(char, Option<u32>)> {
    let mut out: Vec<(char, Option<u32>)> = Vec::new();
    for page in &lay.pages {
        for line in &page.lines {
            for c in line.text.chars() {
                if c.is_control() || c == '\u{00A0}' || winansi::encode_char(c).is_some() {
                    continue;
                }
                if !out.iter().any(|(x, _)| *x == c) {
                    out.push((c, if page.title_page { None } else { page.number }));
                }
            }
        }
    }
    out
}

/// A screenplay PDF is never written with characters silently replaced or
/// dropped: text the Courier font can't print is a clear `export.*` error that
/// names the characters and suggests formats that keep them.
pub fn check_pdf_characters(lay: &Layout) -> AppResult<()> {
    let bad = unsupported_pdf_characters(lay);
    if bad.is_empty() {
        return Ok(());
    }
    let sample: String = bad
        .iter()
        .take(6)
        .map(|(c, _)| format!("“{c}”"))
        .collect::<Vec<_>>()
        .join(" ");
    let where_ = match bad[0].1 {
        Some(p) => format!(" (first on page {p})"),
        None => " (on the title page)".to_string(),
    };
    Err(AppError::export(
        "pdf_unsupported_characters",
        format!(
            "This screenplay contains characters the standard screenplay PDF font (Courier) can't print, such as {sample}{where_}. \
             No PDF was written. Export to DOCX, Final Draft or Fountain to keep them, or replace those characters and export the PDF again."
        ),
    )
    .with_detail(format!("{} distinct non-WinAnsi character(s)", bad.len())))
}

/// Render to bytes (all formats except DOCX, which is a package written straight to disk).
pub fn render_bytes(
    doc: &ScreenplayDoc,
    format: ExportFormat,
    opts: &ExportOptions,
) -> AppResult<(Vec<u8>, Option<usize>, Vec<Warning>)> {
    let warnings = export_warnings(doc, format, opts);
    let (bytes, pages) = match format {
        ExportFormat::Pdf => {
            let lay = layout::layout(doc, &opts.layout());
            check_pdf_characters(&lay)?;
            let title = doc
                .title_page
                .title()
                .unwrap_or("Screenplay")
                .replace('\n', " ");
            let out = pdf_export::render_layout(&lay, &title, doc.title_page.author());
            debug_assert_eq!(out.replaced_chars, 0);
            (out.bytes, Some(out.pages))
        }
        ExportFormat::Fdx => (
            fdx::write(
                doc,
                &fdx::FdxOptions {
                    scene_numbers: opts.scene_numbers,
                    include_notes: opts.include_notes,
                    revision_marks: opts.revision_marks,
                    title_page: opts.title_page,
                },
            )
            .into_bytes(),
            None,
        ),
        ExportFormat::Fountain => {
            let mut d = doc.clone();
            if !opts.title_page {
                d.title_page = TitlePage::default();
            } else if !opts.revision_info {
                d.title_page
                    .0
                    .retain(|(k, _)| !k.eq_ignore_ascii_case("Revision"));
            }
            (
                fountain::write(&d, opts.include_notes, opts.scene_numbers).into_bytes(),
                None,
            )
        }
        ExportFormat::Txt => {
            let lay = LayoutOptions {
                page_numbers: false,
                paginate: false,
                ..opts.layout()
            };
            (text::write(doc, &lay).into_bytes(), None)
        }
        ExportFormat::Docx => {
            let d = docx::DocxOptions {
                title_page: opts.title_page,
                scene_numbers: opts.scene_numbers,
                include_notes: opts.include_notes,
                revision_marks: opts.revision_marks,
                revision_info: opts.revision_info,
            };
            (docx::to_bytes(doc, &d)?, None)
        }
    };
    Ok((bytes, pages, warnings))
}

/// Export `doc` to `dest` atomically in `format`.
pub fn export_to_path(
    doc: &ScreenplayDoc,
    format: ExportFormat,
    opts: &ExportOptions,
    dest: &Path,
) -> AppResult<ExportOutcome> {
    let (bytes, pages, warnings) = render_bytes(doc, format, opts)?;
    write_atomic(dest, &bytes)?;
    Ok(ExportOutcome {
        path: dest.to_path_buf(),
        bytes_written: bytes.len() as u64,
        pages,
        warnings,
    })
}
