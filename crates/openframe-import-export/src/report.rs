//! Generic report documents for every module's document exports (Breakdown
//! report, Catalog, Schedule, Shot List, Call Sheet summaries …):
//!
//! ```ignore
//! use openframe_import_export::report::{Document, Block, Table, Column};
//! let mut t = Table::new("Catalog", vec![Column::new("Item"), Column::new("Category"), Column::right("Qty")]);
//! t.push(vec!["Umbrella".into(), "Props".into(), "2".into()]);
//! let doc = Document::new("Production Catalog").with(Block::Table(t));
//! let pdf = doc.to_pdf();          // Helvetica, tables with repeated headers, page numbers
//! let csv = doc.tables()[0].to_csv()?;
//! let xlsx = doc.to_xlsx()?;       // one sheet per table
//! ```
//!
//! PDFs print Windows-1252 text with the base-14 Helvetica fonts, exactly as
//! before embedded fonts existed. Any other text (Telugu, Hindi, Tamil,
//! Malayalam, Kannada, Latin letters such as "ā") is shaped and drawn with the
//! bundled fonts of ADR-0016, embedded as subsets next to Helvetica, and is
//! measured with the same shaped advances so wrapping and centring agree with
//! what is printed. Characters no bundled font covers (emoji, Chinese) are
//! counted in [`ReportPdf::replaced_chars`]; document exports use
//! [`Document::to_pdf_strict`], which never writes such a PDF and instead names
//! the characters. CSV/XLSX keep full Unicode.
//!
//! Besides headings, paragraphs, label/value lists and tables, a document can
//! hold picture grids ([`Grid`], e.g. storyboard panels or moodboard images),
//! dark section bars (paper call sheet), centered lines and spacers, and can
//! be laid out landscape with a centered header and a custom footer.
//!
//! Branding (production depth spec §43; doc 33 section 8; PDF design system section 2): a
//! document with [`Document::brand`] prints the official OpenFrame logo (embedded once, lossless),
//! the brand line under it and "Page X of Y" at the lower right of every page, and a workbook made
//! with [`tables_to_xlsx_branded`]
//! names OpenFrame Studio in its document properties and the first sheet's
//! footer. Screenplay documents never use this module.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::OnceLock;

use openframe_domain::{AppError, AppResult};
use pdf_writer::Content;
use unicode_segmentation::UnicodeSegmentation;

#[cfg(test)]
mod band_matrix_tests;
mod design;
#[cfg(test)]
mod furniture_tests;
pub use design::{
    Banner, Callout, CalloutCol, Card, CardHead, CardPart, Cards, Cell, CellItem, Chip, Col, Field,
    Group, GroupItem, HeaderStyle, Legend, LegendItem, Masthead, Metric, Metrics, Rgb, Row,
    Section, Side, StyledTable, Tile, Tone, Txt, Width, pal,
};
pub use design::{
    FactLine, FactRun, Matrix, MatrixCode, MatrixCount, MatrixDay, MatrixKey, MatrixRow,
    MatrixTotals, TableBand,
};

use crate::fonts;
use crate::pdf_fonts::{FontPack, ShapedText};
use crate::pdfgen::{self, LosslessImage, PdfBuilder, PdfImage};
use crate::winansi::{encode_char, text_width_pt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub header: String,
    /// Relative width in the PDF.
    pub weight: f32,
    pub align: Align,
}

impl Column {
    pub fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            weight: 1.0,
            align: Align::Left,
        }
    }
    pub fn right(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            weight: 0.6,
            align: Align::Right,
        }
    }
    pub fn weight(mut self, w: f32) -> Self {
        self.weight = w.max(0.1);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    /// Caption in the PDF and sheet name in XLSX.
    pub name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(name: impl Into<String>, columns: Vec<Column>) -> Self {
        Self {
            name: name.into(),
            columns,
            rows: Vec::new(),
        }
    }
    pub fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// UTF-8 CSV with a byte-order mark (so spreadsheet apps detect UTF-8) and a header row.
    pub fn to_csv(&self) -> AppResult<Vec<u8>> {
        let mut w = csv::WriterBuilder::new().from_writer(Vec::from(&b"\xEF\xBB\xBF"[..]));
        let err = |e: csv::Error| {
            AppError::export("csv_failed", "The CSV file could not be created.")
                .with_detail(e.to_string())
        };
        w.write_record(
            self.columns
                .iter()
                .map(|c| csv_safe(&c.header).into_owned()),
        )
        .map_err(err)?;
        for r in &self.rows {
            let cells: Vec<Cow<'_, str>> = (0..self.columns.len())
                .map(|i| csv_safe(r.get(i).map(|s| s.as_str()).unwrap_or("")))
                .collect();
            w.write_record(cells.iter().map(|c| c.as_ref()))
                .map_err(err)?;
        }
        w.into_inner().map_err(|e| {
            AppError::export("csv_failed", "The CSV file could not be created.")
                .with_detail(e.to_string())
        })
    }
}

/// A CSV cell that a spreadsheet app shows as text and never runs as a formula. Text such as a
/// cast or location name that starts with `=`, `+`, `-`, `@`, a tab or a carriage return gets a
/// leading apostrophe (the usual spreadsheet text marker). Plain numbers (for example `-1500`)
/// and a lone `-` (an empty value or a Day Out of Days code) are kept as they are.
pub fn csv_safe(cell: &str) -> Cow<'_, str> {
    let risky = matches!(
        cell.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    );
    if !risky || cell == "-" || cell.trim().parse::<f64>().is_ok() {
        Cow::Borrowed(cell)
    } else {
        Cow::Owned(format!("'{cell}"))
    }
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        text: String,
        level: u8,
    },
    Paragraph(String),
    /// Label/value pairs (e.g. document header facts).
    KeyValues(Vec<(String, String)>),
    Table(Table),
    PageBreak,
    /// Pictures with captions laid out in columns (storyboard sheet, moodboard).
    Grid(Grid),
    /// Full-width dark bar with a white label (paper call-sheet sections).
    SectionBar(String),
    /// A centered line (wrapped when long).
    Centered {
        text: String,
        size: f32,
        bold: bool,
    },
    /// Small grey text (source lines, footnotes).
    Note(String),
    /// Outline entry indented by `level` steps (18 pt each); `muted` = grey 8.5 pt.
    Indented {
        text: String,
        level: u8,
        muted: bool,
    },
    /// Vertical space in points.
    Spacer(f32),
    // ---- design system blocks (production documents), see `report/design.rs`
    Masthead(Masthead),
    Metrics(Metrics),
    Section(Section),
    Banner(Banner),
    Callout(Callout),
    Cards(Cards),
    Styled(StyledTable),
    Legend(Legend),
    Text(Txt),
    /// Blocks laid out as one unit: they stay on one page when they fit the space left, else the
    /// group starts a new page; a group taller than a page starts fresh and then splits only
    /// where its own blocks split (between rows and cards).
    Together(Vec<Block>),
    /// The Day Out of Days matrix (frozen columns, day groups, code cells).
    Matrix(Matrix),
}

// ------------------------------------------------------------------ images

/// A picture prepared for a PDF: decoded with size limits, scaled down to at
/// most [`ReportImage::MAX_EDGE`] pixels, flattened onto white and re-encoded
/// as a baseline JPEG.
#[derive(Debug, Clone)]
pub struct ReportImage {
    pub width: u32,
    pub height: u32,
    gray: bool,
    jpeg: Vec<u8>,
}

impl ReportImage {
    /// Longest edge kept in the PDF (about 300 dpi at 5 inches).
    pub const MAX_EDGE: u32 = 1600;
    /// Largest source file accepted.
    pub const MAX_SOURCE_BYTES: usize = 64 << 20;

    /// Decode PNG, JPEG, GIF, WebP or BMP bytes. Errors are `export.image_unreadable`.
    pub fn decode(bytes: &[u8]) -> AppResult<ReportImage> {
        let unreadable = |detail: String| {
            AppError::export("image_unreadable", "An image could not be read.").with_detail(detail)
        };
        if bytes.len() > Self::MAX_SOURCE_BYTES {
            return Err(unreadable(format!("{} bytes", bytes.len())));
        }
        let decoded = std::panic::catch_unwind(|| -> Result<image::DynamicImage, String> {
            let mut reader = image::ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(20_000);
            limits.max_image_height = Some(20_000);
            limits.max_alloc = Some(512 << 20);
            reader.limits(limits);
            reader.decode().map_err(|e| e.to_string())
        })
        .map_err(|_| unreadable("decoder panicked".into()))?
        .map_err(unreadable)?;
        let img = if decoded.width().max(decoded.height()) > Self::MAX_EDGE {
            decoded.thumbnail(Self::MAX_EDGE, Self::MAX_EDGE)
        } else {
            decoded
        };
        let (w, h) = (img.width().max(1), img.height().max(1));
        let gray = !img.color().has_color();
        let (samples, color) = if gray {
            let la = img.to_luma_alpha8();
            let px: Vec<u8> = la.pixels().map(|p| blend(p.0[0], p.0[1])).collect();
            (px, image::ExtendedColorType::L8)
        } else {
            let rgba = img.to_rgba8();
            let mut px = Vec::with_capacity((w * h * 3) as usize);
            for p in rgba.pixels() {
                let [r, g, b, a] = p.0;
                px.extend_from_slice(&[blend(r, a), blend(g, a), blend(b, a)]);
            }
            (px, image::ExtendedColorType::Rgb8)
        };
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
            .encode(&samples, w, h, color)
            .map_err(|e| unreadable(e.to_string()))?;
        Ok(ReportImage {
            width: w,
            height: h,
            gray,
            jpeg,
        })
    }

    fn pdf(&self) -> PdfImage {
        PdfImage {
            width: self.width,
            height: self.height,
            gray: self.gray,
            jpeg: self.jpeg.clone(),
        }
    }
}

/// Composite a channel over white.
fn blend(v: u8, alpha: u8) -> u8 {
    let a = alpha as u32;
    ((v as u32 * a + 255 * (255 - a) + 127) / 255) as u8
}

/// One picture cell: a frame (image, or a grey placeholder with a label) and
/// caption text under it. A cell without image and placeholder has no frame.
#[derive(Debug, Clone, Default)]
pub struct GridCell {
    pub image: Option<ReportImage>,
    pub placeholder: Option<String>,
    /// Bold first caption line (e.g. "Panel 2 · Shot 12A").
    pub title: Option<String>,
    pub lines: Vec<String>,
    /// Grey lines (optional notes).
    pub notes: Vec<String>,
}

impl GridCell {
    fn has_frame(&self) -> bool {
        self.image.is_some() || self.placeholder.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Grid {
    pub columns: usize,
    /// Frame height / width (e.g. 9/16 for a widescreen panel).
    pub frame_ratio: f32,
    pub cells: Vec<GridCell>,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub title: String,
    pub subtitle: Option<String>,
    pub landscape: bool,
    /// Title and subtitle centered (paper documents such as call sheets).
    pub centered_header: bool,
    /// Footer text before "Page n of m" on unbranded documents (defaults to the title). Branded
    /// documents print the logo, the brand line and "Page X of Y" only.
    pub footer: Option<String>,
    pub author: Option<String>,
    /// Brand line printed under the OpenFrame logo at the bottom of every page.
    pub brand: Option<String>,
    /// One line at the top right of every page (6.8 pt slate); wraps inside the top margin.
    pub running_header: Option<String>,
    pub blocks: Vec<Block>,
}

/// The brand line on production exports.
pub const BRAND_LINE: &str = "Created using OpenFrame";

#[derive(Debug, Clone)]
pub struct ReportPdf {
    pub bytes: Vec<u8>,
    pub pages: usize,
    pub replaced_chars: usize,
}

impl Document {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            landscape: false,
            centered_header: false,
            footer: None,
            author: None,
            brand: None,
            running_header: None,
            blocks: Vec::new(),
        }
    }
    /// A small identifying line at the top right of every page, or none.
    pub fn running_header(mut self, line: Option<String>) -> Self {
        self.running_header = line;
        self
    }
    /// Print the OpenFrame logo and `line` at the bottom of every page.
    pub fn brand(mut self, line: impl Into<String>) -> Self {
        self.brand = Some(line.into());
        self
    }
    pub fn centered_header(mut self, c: bool) -> Self {
        self.centered_header = c;
        self
    }
    pub fn footer(mut self, f: impl Into<String>) -> Self {
        self.footer = Some(f.into());
        self
    }
    pub fn author(mut self, a: impl Into<String>) -> Self {
        self.author = Some(a.into());
        self
    }
    pub fn subtitle(mut self, s: impl Into<String>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    pub fn landscape(mut self, l: bool) -> Self {
        self.landscape = l;
        self
    }
    pub fn with(mut self, b: Block) -> Self {
        self.blocks.push(b);
        self
    }
    pub fn push(&mut self, b: Block) {
        self.blocks.push(b);
    }
    pub fn tables(&self) -> Vec<&Table> {
        fn collect<'a>(blocks: &'a [Block], out: &mut Vec<&'a Table>) {
            for b in blocks {
                match b {
                    Block::Table(t) => out.push(t),
                    Block::Together(inner) => collect(inner, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        collect(&self.blocks, &mut out);
        out
    }

    /// XLSX workbook with one sheet per table.
    pub fn to_xlsx(&self) -> AppResult<Vec<u8>> {
        tables_to_xlsx(&self.tables())
    }

    /// PDF where unsupported characters are printed as `?` and counted in
    /// [`ReportPdf::replaced_chars`]; the caller must warn the user.
    pub fn to_pdf(&self) -> ReportPdf {
        render_pdf(self)
    }

    /// PDF that is never written with replaced characters: text outside the
    /// standard PDF font's character set is a clear `export.*` error that
    /// points the user at CSV/XLSX, which keep full Unicode.
    pub fn to_pdf_checked(&self) -> AppResult<ReportPdf> {
        self.to_pdf_strict(
            "Export it as CSV or Excel (XLSX) to keep them, or replace those characters.",
        )
    }

    /// Like [`Document::to_pdf_checked`] with a caller-specific suggestion
    /// ("Export it as CSV…", "Replace those characters in the captions…").
    pub fn to_pdf_strict(&self, suggestion: &str) -> AppResult<ReportPdf> {
        let bad = self.unsupported_chars();
        let out = render_pdf(self);
        if out.replaced_chars > 0 || !bad.is_empty() {
            let sample: Vec<String> = bad.iter().take(6).map(|c| format!("“{c}”")).collect();
            let such_as = if sample.is_empty() {
                String::new()
            } else {
                format!(", such as {}", sample.join(" "))
            };
            return Err(AppError::export(
                "pdf_unsupported_characters",
                format!(
                    "This document contains characters the PDF fonts can't print{such_as}. \
                     The PDF fonts cover {}. No PDF was written. {suggestion}",
                    crate::PDF_SCRIPTS
                ),
            )
            .with_detail(format!(
                "{} character(s) without a PDF glyph, {} distinct",
                out.replaced_chars,
                bad.len()
            )));
        }
        Ok(out)
    }

    /// Distinct characters (first-appearance order) that neither Helvetica nor
    /// the bundled fonts can print. Empty = the PDF will be exact.
    pub fn unsupported_chars(&self) -> Vec<char> {
        let mut out: Vec<char> = Vec::new();
        let mut scan = |t: &str| {
            if !needs_fonts(t) {
                return;
            }
            for c in fonts::unprintable_chars(t) {
                if !printable(c) && !out.contains(&c) {
                    out.push(c);
                }
            }
        };
        scan(&self.title);
        if let Some(s) = &self.subtitle {
            scan(s);
        }
        if let Some(s) = &self.footer {
            scan(s);
        }
        if let Some(s) = &self.brand {
            scan(s);
        }
        if let Some(s) = &self.running_header {
            scan(s);
        }
        scan_blocks(&self.blocks, &mut scan);
        out
    }
}

fn scan_blocks(blocks: &[Block], scan: &mut dyn FnMut(&str)) {
    {
        for b in blocks {
            match b {
                Block::Together(inner) => scan_blocks(inner, scan),
                Block::Heading { text, .. }
                | Block::Paragraph(text)
                | Block::SectionBar(text)
                | Block::Note(text)
                | Block::Indented { text, .. }
                | Block::Centered { text, .. } => scan(text),
                Block::KeyValues(kv) => {
                    for (k, v) in kv {
                        scan(k);
                        scan(v);
                    }
                }
                Block::Table(t) => {
                    scan(&t.name);
                    for c in &t.columns {
                        scan(&c.header);
                    }
                    for r in &t.rows {
                        for cell in r.iter().take(t.columns.len()) {
                            scan(cell);
                        }
                    }
                }
                Block::Grid(g) => {
                    for cell in &g.cells {
                        for t in cell.placeholder.iter().chain(cell.title.iter()) {
                            scan(t);
                        }
                        for t in cell.lines.iter().chain(cell.notes.iter()) {
                            scan(t);
                        }
                    }
                }
                Block::PageBreak | Block::Spacer(_) => {}
                Block::Masthead(_)
                | Block::Metrics(_)
                | Block::Section(_)
                | Block::Banner(_)
                | Block::Callout(_)
                | Block::Cards(_)
                | Block::Styled(_)
                | Block::Legend(_)
                | Block::Matrix(_)
                | Block::Text(_) => design::scan_block(b, scan),
            }
        }
    }
}

/// Printable with the WinAnsi base-14 fonts (or silently normalised, like tabs).
fn printable(c: char) -> bool {
    c.is_control()
        || c == '\u{00A0}'
        || encode_char(c).is_some()
        || matches!(
            c,
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' | '\u{2032}' | '\u{2033}'
        )
}

/// Does this text need the embedded fonts (a character Helvetica can't print)?
fn needs_fonts(text: &str) -> bool {
    text.chars().any(|c| !printable(c))
}

/// Split text at grapheme boundaries into runs Helvetica prints (`false`) and
/// runs drawn with the embedded fonts (`true`). Combining marks and conjuncts
/// stay with their base character.
fn segments(text: &str) -> Vec<(bool, &str)> {
    let mut out: Vec<(bool, &str)> = Vec::new();
    let mut start = 0;
    let mut kind: Option<bool> = None;
    for (i, g) in text.grapheme_indices(true) {
        let embedded = g.chars().any(|c| !printable(c));
        match kind {
            Some(k) if k == embedded => {}
            Some(k) => {
                out.push((k, &text[start..i]));
                start = i;
                kind = Some(embedded);
            }
            None => kind = Some(embedded),
        }
    }
    if let Some(k) = kind {
        out.push((k, &text[start..]));
    }
    out
}

/// Printed width of `text` in points: Helvetica metrics for Windows-1252 runs,
/// shaped advances of the bundled fonts for the rest (the same numbers the
/// renderer draws with).
pub fn text_width(text: &str, size: f32, bold: bool) -> f32 {
    if !needs_fonts(text) {
        return text_width_pt(text, size, bold);
    }
    segments(text)
        .into_iter()
        .map(|(embedded, seg)| {
            if embedded {
                ShapedText::new(seg, bold, false).width_pt(size)
            } else {
                text_width_pt(seg, size, bold)
            }
        })
        .sum()
}

/// Wrap text that needs the embedded fonts: words are measured once and an
/// over-long word breaks between grapheme clusters, never inside one.
fn wrap_shaped(text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
    let space = text_width_pt(" ", size, bold);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        let mut used = 0.0f32;
        for word in para.split_whitespace() {
            let w = text_width(word, size, bold);
            if line.is_empty() {
                if w > width {
                    let mut chunk = String::new();
                    let mut cw = 0.0f32;
                    for g in word.graphemes(true) {
                        let gw = text_width(g, size, bold);
                        if !chunk.is_empty() && cw + gw > width {
                            out.push(std::mem::take(&mut chunk));
                            cw = 0.0;
                        }
                        chunk.push_str(g);
                        cw += gw;
                    }
                    line = chunk;
                    used = cw;
                } else {
                    line = word.to_string();
                    used = w;
                }
            } else if used + space + w <= width {
                line.push(' ');
                line.push_str(word);
                used += space + w;
            } else {
                out.push(std::mem::take(&mut line));
                // Start the next line with this word (it may itself need breaking).
                let rest = wrap_shaped(word, width, size, bold);
                let n = rest.len();
                for (k, piece) in rest.into_iter().enumerate() {
                    if k + 1 < n {
                        out.push(piece);
                    } else {
                        used = text_width(&piece, size, bold);
                        line = piece;
                    }
                }
            }
        }
        out.push(line);
    }
    out
}

fn sheet_name(name: &str, used: &mut Vec<String>) -> String {
    let mut s: String = name
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .collect();
    s = s.trim().trim_matches('\'').to_string();
    if s.is_empty() {
        s = "Sheet".into();
    }
    let base: String = s.chars().take(28).collect();
    let mut candidate: String = s.chars().take(31).collect();
    let mut n = 2;
    while used.iter().any(|u| u.eq_ignore_ascii_case(&candidate)) {
        candidate = format!("{base} {n}");
        n += 1;
    }
    used.push(candidate.clone());
    candidate
}

pub fn tables_to_xlsx(tables: &[&Table]) -> AppResult<Vec<u8>> {
    xlsx(tables, None, None)
}

/// Like [`tables_to_xlsx`], with the workbook's creation date fixed to `created`
/// (Unix seconds). The file then holds no clock time, so the same tables always
/// give a byte-identical file (reproducible exports).
pub fn tables_to_xlsx_at(tables: &[&Table], created: Option<i64>) -> AppResult<Vec<u8>> {
    xlsx(tables, None, created)
}

/// Workbook that says where it came from without touching the data: the document
/// properties name OpenFrame Studio and the first sheet prints `line` in its footer.
/// `created` fixes the creation date as in [`tables_to_xlsx_at`].
pub fn tables_to_xlsx_branded(
    tables: &[&Table],
    line: &str,
    created: Option<i64>,
) -> AppResult<Vec<u8>> {
    xlsx(tables, Some(line), created)
}

fn xlsx(tables: &[&Table], brand: Option<&str>, created: Option<i64>) -> AppResult<Vec<u8>> {
    use rust_xlsxwriter::{DocProperties, ExcelDateTime, Format, Workbook};
    let err = |e: rust_xlsxwriter::XlsxError| {
        AppError::export("xlsx_failed", "The spreadsheet could not be created.")
            .with_detail(e.to_string())
    };
    let mut wb = Workbook::new();
    if brand.is_some() || created.is_some() {
        let mut props = DocProperties::new();
        if let Some(line) = brand {
            props = props
                .set_author("OpenFrame Studio")
                .set_company("OpenFrame Studio")
                .set_comment(line);
        }
        if let Some(t) = created {
            let at = ExcelDateTime::from_timestamp(t.max(0)).map_err(err)?;
            props = props.set_creation_datetime(&at);
        }
        wb.set_properties(&props);
    }
    let bold = Format::new().set_bold();
    let mut used = Vec::new();
    if tables.is_empty() {
        wb.add_worksheet();
    }
    for (i, t) in tables.iter().enumerate() {
        let ws = wb.add_worksheet();
        ws.set_name(sheet_name(&t.name, &mut used)).map_err(err)?;
        if let (Some(line), 0) = (brand, i) {
            // "&C" centres the footer; "&" in the line itself is escaped.
            ws.set_footer(format!("&C{}", line.replace('&', "&&")));
        }
        for (c, col) in t.columns.iter().enumerate() {
            ws.write_string_with_format(0, c as u16, &col.header, &bold)
                .map_err(err)?;
            let longest = t
                .rows
                .iter()
                .map(|r| r.get(c).map(|s| s.chars().count()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            let width = (longest.max(col.header.chars().count()) as f64 + 2.0).clamp(8.0, 60.0);
            ws.set_column_width(c as u16, width).map_err(err)?;
        }
        for (r, row) in t.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate().take(t.columns.len()) {
                if t.columns[c].align == Align::Right
                    && let Ok(n) = cell.trim().parse::<f64>()
                {
                    ws.write_number((r + 1) as u32, c as u16, n).map_err(err)?;
                    continue;
                }
                ws.write_string((r + 1) as u32, c as u16, cell)
                    .map_err(err)?;
            }
        }
        ws.set_freeze_panes(1, 0).map_err(err)?;
    }
    wb.save_to_buffer().map_err(err)
}

// --------------------------------------------------------------------- PDF

pub(crate) const MARGIN: f32 = 46.0;
/// Content stops this far above the page bottom (PDF design system section 4). The footer sits in
/// the space below: logo, brand line and page number, all measured from the page bottom.
pub(crate) const BOTTOM_MARGIN: f32 = 66.0;
/// Unbranded documents (the Story page manuscript) keep the page body they always had.
const PLAIN_BOTTOM_MARGIN: f32 = 80.0;

/// The official logo (699 x 163 px artwork bounds), embedded losslessly.
const LOGO_PNG: &[u8] = include_bytes!("../assets/openframe-logo.png");
const LOGO_H: f32 = 17.0;
/// Bottom edge of the logo above the page bottom.
const LOGO_BOTTOM: f32 = 33.0;
/// Baseline of the brand line and of "Page X of Y" above the page bottom. The design system text
/// says about 22 pt (the CSS padding); the supplied reference PDF measures 24 pt, which is used.
const FOOTER_BASE: f32 = 24.0;
const BRAND_SIZE: f32 = 6.8;
const PAGE_SIZE: f32 = 7.4;
/// Running header: first baseline below the page top, line step, size and tracking (0.04 em).
const HEADER_BASE: f32 = 24.0;
const HEADER_LEADING: f32 = 8.2;
const HEADER_SIZE: f32 = 6.8;
const HEADER_TRACK: f32 = HEADER_SIZE * 0.04;

#[derive(Clone)]
enum Op {
    Text {
        bold: bool,
        size: f32,
        x: f32,
        y: f32,
        text: String,
        gray: f32,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        gray: f32,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },
    Frame {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Image {
        index: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    /// Coloured text with optional letter spacing (design system, `report/design.rs`).
    TextC {
        bold: bool,
        size: f32,
        x: f32,
        y: f32,
        text: String,
        color: Rgb,
        track: f32,
    },
    /// Filled and/or outlined rectangle, optionally with rounded corners.
    Shape {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        round_top: bool,
        round_bottom: bool,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
    LineC {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Rgb,
        width: f32,
    },
    Circle {
        x: f32,
        y: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
}

/// Wrap text to a width in points.
pub fn wrap_pt(text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
    if needs_fonts(text) {
        return wrap_shaped(text, width, size, bold);
    }
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if text_width_pt(&candidate, size, bold) <= width || line.is_empty() {
                if line.is_empty() && text_width_pt(word, size, bold) > width {
                    // Break an over-long word by characters.
                    let mut chunk = String::new();
                    for ch in word.chars() {
                        let next = format!("{chunk}{ch}");
                        if text_width_pt(&next, size, bold) > width && !chunk.is_empty() {
                            out.push(std::mem::take(&mut chunk));
                        }
                        chunk.push(ch);
                    }
                    line = chunk;
                } else {
                    line = candidate;
                }
            } else {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        out.push(line);
    }
    out
}

struct Pager<'a> {
    w: f32,
    h: f32,
    /// Distance from the page top to the first line of content (the top margin, or more when a
    /// long running header needs it).
    inset: f32,
    /// Content stops this far above the page bottom.
    bottom: f32,
    pages: Vec<Vec<Op>>,
    y: f32,
    images: Vec<&'a ReportImage>,
    /// A heading (or other lead-in) just placed that must not end a page alone.
    hold: Option<Hold>,
}

/// The ops of a lead-in on the current page: `start..end` in the page's ops, spanning `top` down
/// to `bottom` on the page.
struct Hold {
    page: usize,
    start: usize,
    end: usize,
    top: f32,
    bottom: f32,
}

impl<'a> Pager<'a> {
    fn new(w: f32, h: f32, inset: f32, bottom: f32) -> Pager<'a> {
        Pager {
            w,
            h,
            inset,
            bottom,
            pages: vec![Vec::new()],
            y: h - inset,
            images: Vec::new(),
            hold: None,
        }
    }
    /// Nothing has been placed on this page yet.
    fn fresh(&self) -> bool {
        self.y >= self.h - self.inset - 0.5
    }
    /// Mark what was just placed (ops from `start`, from `top` down to the current position) as a
    /// lead-in: if the next thing does not fit and a new page starts, it moves along.
    fn keep_with_next(&mut self, start: usize, top: f32) {
        self.hold = Some(Hold {
            page: self.pages.len() - 1,
            start,
            end: self.pages.last().map_or(0, |o| o.len()),
            top,
            bottom: self.y,
        });
    }
    /// Index of a picture in the document, registered once however often it is drawn.
    fn image_index(&mut self, img: &'a ReportImage) -> usize {
        if let Some(i) = self.images.iter().position(|x| std::ptr::eq(*x, img)) {
            return i;
        }
        self.images.push(img);
        self.images.len() - 1
    }
    fn new_page(&mut self) {
        // A heading left alone at the foot of the page goes to the new page with its content.
        let carried = self.hold.take().and_then(|hold| {
            let last = self.pages.len() - 1;
            let alone = hold.page == last && hold.start > 0 && self.pages[last].len() == hold.end;
            alone.then(|| {
                (
                    self.pages[last].split_off(hold.start),
                    hold.top,
                    hold.bottom,
                )
            })
        });
        self.pages.push(Vec::new());
        self.y = self.h - self.inset;
        if let Some((ops, top, bottom)) = carried {
            let dy = self.y - top;
            for mut op in ops {
                design::shift(&mut op, 0.0, dy);
                self.ops().push(op);
            }
            self.y -= top - bottom;
        }
    }
    fn ops(&mut self) -> &mut Vec<Op> {
        self.pages.last_mut().unwrap()
    }
    fn room(&self) -> f32 {
        self.y - self.bottom
    }
    fn ensure(&mut self, needed: f32) {
        if self.room() < needed {
            self.new_page();
        }
    }
    fn text_block(&mut self, text: &str, size: f32, bold: bool, gray: f32, gap_after: f32) {
        self.text_block_at(0.0, text, size, bold, gray, gap_after);
    }
    fn text_block_at(
        &mut self,
        indent: f32,
        text: &str,
        size: f32,
        bold: bool,
        gray: f32,
        gap_after: f32,
    ) {
        let width = self.w - 2.0 * MARGIN - indent;
        for line in wrap_pt(text, width, size, bold) {
            self.ensure(size * 1.3);
            self.y -= size * 1.3;
            let y = self.y + size * 0.3;
            self.ops().push(Op::Text {
                bold,
                size,
                x: MARGIN + indent,
                y,
                text: line,
                gray,
            });
        }
        self.y -= gap_after;
    }
    fn centered(&mut self, text: &str, size: f32, bold: bool, gray: f32) {
        let width = self.w - 2.0 * MARGIN;
        for line in wrap_pt(text, width, size, bold) {
            self.ensure(size * 1.35);
            self.y -= size * 1.35;
            let tw = text_width(&line, size, bold);
            let x = MARGIN + ((width - tw) / 2.0).max(0.0);
            let y = self.y + size * 0.3;
            self.ops().push(Op::Text {
                bold,
                size,
                x,
                y,
                text: line,
                gray,
            });
        }
    }
}

fn table_header(p: &mut Pager, t: &Table, xs: &[f32], widths: &[f32]) {
    let size = 8.5;
    let lines: Vec<Vec<String>> = t
        .columns
        .iter()
        .zip(widths)
        .map(|(c, w)| wrap_pt(&c.header, w - 8.0, size, true))
        .collect();
    let rows = lines.iter().map(|l| l.len()).max().unwrap_or(1) as f32;
    let h = rows * size * 1.25 + 6.0;
    p.ensure(h + 14.0);
    let top = p.y;
    let total_w: f32 = widths.iter().sum();
    p.ops().push(Op::Rect {
        x: MARGIN,
        y: top - h,
        w: total_w,
        h,
        gray: 0.88,
    });
    for (i, l) in lines.iter().enumerate() {
        for (k, s) in l.iter().enumerate() {
            let y = top - 3.0 - (k as f32 + 1.0) * size * 1.25 + 2.0;
            // Headers follow their column's alignment (numbers are right-aligned).
            let tw = text_width(s, size, true);
            let x = match t.columns[i].align {
                Align::Left => xs[i] + 4.0,
                Align::Right => xs[i] + widths[i] - 4.0 - tw,
                Align::Center => xs[i] + (widths[i] - tw) / 2.0,
            };
            p.ops().push(Op::Text {
                bold: true,
                size,
                x,
                y,
                text: s.clone(),
                gray: 0.0,
            });
        }
    }
    p.y -= h;
}

fn render_table(p: &mut Pager, t: &Table) {
    let size = 8.5;
    let width = p.w - 2.0 * MARGIN;
    let total: f32 = t.columns.iter().map(|c| c.weight).sum::<f32>().max(0.1);
    let widths: Vec<f32> = t.columns.iter().map(|c| width * c.weight / total).collect();
    let mut xs = Vec::new();
    let mut x = MARGIN;
    for w in &widths {
        xs.push(x);
        x += w;
    }
    if !t.name.trim().is_empty() {
        p.ensure(60.0);
        p.text_block(&t.name, 11.0, true, 0.0, 4.0);
    }
    table_header(p, t, &xs, &widths);
    if t.rows.is_empty() {
        p.text_block("No rows.", 9.0, false, 0.45, 6.0);
        return;
    }
    for (ri, row) in t.rows.iter().enumerate() {
        let cells: Vec<Vec<String>> = (0..t.columns.len())
            .map(|i| {
                wrap_pt(
                    row.get(i).map(|s| s.as_str()).unwrap_or(""),
                    widths[i] - 8.0,
                    size,
                    false,
                )
            })
            .collect();
        let lines = cells.iter().map(|c| c.len()).max().unwrap_or(1).max(1) as f32;
        let h = lines * size * 1.25 + 6.0;
        if p.room() < h {
            p.new_page();
            table_header(p, t, &xs, &widths);
        }
        let top = p.y;
        if ri % 2 == 1 {
            p.ops().push(Op::Rect {
                x: MARGIN,
                y: top - h,
                w: width,
                h,
                gray: 0.96,
            });
        }
        for (i, c) in cells.iter().enumerate() {
            for (k, s) in c.iter().enumerate() {
                let tw = text_width(s, size, false);
                let cx = match t.columns[i].align {
                    Align::Left => xs[i] + 4.0,
                    Align::Right => xs[i] + widths[i] - 4.0 - tw,
                    Align::Center => xs[i] + (widths[i] - tw) / 2.0,
                };
                let y = top - 3.0 - (k as f32 + 1.0) * size * 1.25 + 2.0;
                p.ops().push(Op::Text {
                    bold: false,
                    size,
                    x: cx,
                    y,
                    text: s.clone(),
                    gray: 0.0,
                });
            }
        }
        p.y -= h;
        let y = p.y;
        p.ops().push(Op::Line {
            x1: MARGIN,
            y1: y,
            x2: MARGIN + width,
            y2: y,
        });
    }
    p.y -= 10.0;
}

const GRID_GAP: f32 = 12.0;
const CAPTION_SIZE: f32 = 8.5;
const CAPTION_LINE: f32 = CAPTION_SIZE * 1.25;

fn render_grid<'a>(p: &mut Pager<'a>, g: &'a Grid) {
    let cols = g.columns.clamp(1, 8);
    let width = p.w - 2.0 * MARGIN;
    let cell_w = (width - GRID_GAP * (cols as f32 - 1.0)) / cols as f32;
    let max_h = p.body_h();
    let frame_h = (cell_w * g.frame_ratio.clamp(0.2, 2.0)).min(max_h - 60.0);
    for row in g.cells.chunks(cols) {
        // Caption lines per cell: (text, bold, gray).
        let captions: Vec<Vec<(String, bool, f32)>> = row
            .iter()
            .map(|c| {
                let mut out = Vec::new();
                if let Some(t) = &c.title {
                    for l in wrap_pt(t, cell_w, CAPTION_SIZE + 0.5, true) {
                        out.push((l, true, 0.0));
                    }
                }
                for t in &c.lines {
                    for l in wrap_pt(t, cell_w, CAPTION_SIZE, false) {
                        out.push((l, false, 0.0));
                    }
                }
                for t in &c.notes {
                    for l in wrap_pt(t, cell_w, CAPTION_SIZE, false) {
                        out.push((l, false, 0.45));
                    }
                }
                out
            })
            .collect();
        let frame_space = |c: &GridCell| if c.has_frame() { frame_h + 5.0 } else { 0.0 };
        // An over-long caption is cut to one page rather than overflowing it.
        let limit = max_h - 4.0;
        let keep =
            |c: &GridCell| (((limit - frame_space(c)) / CAPTION_LINE).floor().max(1.0)) as usize;
        let row_h = row
            .iter()
            .zip(&captions)
            .map(|(c, l)| frame_space(c) + l.len().min(keep(c)) as f32 * CAPTION_LINE)
            .fold(0.0_f32, f32::max)
            .min(limit);
        p.ensure(row_h + 4.0);
        let top = p.y;
        for (i, (cell, lines)) in row.iter().zip(&captions).enumerate() {
            let x = MARGIN + i as f32 * (cell_w + GRID_GAP);
            let mut y = top;
            if cell.has_frame() {
                let fy = y - frame_h;
                match &cell.image {
                    Some(img) => {
                        let scale = (cell_w / img.width as f32).min(frame_h / img.height as f32);
                        let (iw, ih) = (img.width as f32 * scale, img.height as f32 * scale);
                        let index = p.images.len();
                        p.images.push(img);
                        p.ops().push(Op::Image {
                            index,
                            x: x + (cell_w - iw) / 2.0,
                            y: fy + (frame_h - ih) / 2.0,
                            w: iw,
                            h: ih,
                        });
                    }
                    None => {
                        p.ops().push(Op::Rect {
                            x,
                            y: fy,
                            w: cell_w,
                            h: frame_h,
                            gray: 0.94,
                        });
                        if let Some(label) = &cell.placeholder {
                            let lines = wrap_pt(label, cell_w - 12.0, 9.0, false);
                            let n = lines.len() as f32;
                            for (k, l) in lines.into_iter().enumerate() {
                                let tw = text_width(&l, 9.0, false);
                                p.ops().push(Op::Text {
                                    bold: false,
                                    size: 9.0,
                                    x: x + (cell_w - tw) / 2.0,
                                    y: fy + frame_h / 2.0 + (n / 2.0 - k as f32 - 0.8) * 11.0,
                                    text: l,
                                    gray: 0.5,
                                });
                            }
                        }
                    }
                }
                p.ops().push(Op::Frame {
                    x,
                    y: fy,
                    w: cell_w,
                    h: frame_h,
                });
                y = fy - 5.0;
            }
            for (text, bold, gray) in lines.iter().take(keep(cell)) {
                y -= CAPTION_LINE;
                p.ops().push(Op::Text {
                    bold: *bold,
                    size: if *bold {
                        CAPTION_SIZE + 0.5
                    } else {
                        CAPTION_SIZE
                    },
                    x,
                    y: y + 2.5,
                    text: text.clone(),
                    gray: *gray,
                });
            }
        }
        p.y = top - row_h - GRID_GAP;
    }
}

/// Lay one block on the pager.
fn render_block<'a>(p: &mut Pager<'a>, b: &'a Block) {
    let w = p.w;
    match b {
        Block::Masthead(m) => design::masthead(p, m),
        Block::Metrics(m) => design::metrics(p, m),
        Block::Section(s) => design::section(p, s),
        Block::Banner(b) => design::banner(p, b),
        Block::Callout(c) => design::callout(p, c),
        Block::Cards(c) => design::cards(p, c),
        Block::Styled(t) => design::styled_table(p, t),
        Block::Legend(l) => design::legend(p, l),
        Block::Text(t) => design::text(p, t),
        Block::Together(blocks) => together(p, blocks),
        Block::Matrix(m) => design::matrix(p, m),
        Block::Heading { text, level } => {
            let size = if *level <= 1 { 13.0 } else { 11.0 };
            // Keep headings with what follows them.
            p.ensure(size * 1.3 + 40.0);
            p.y -= 4.0;
            let (start, top) = (p.ops().len(), p.y);
            p.text_block(text, size, true, 0.0, 4.0);
            p.keep_with_next(start, top);
        }
        Block::Paragraph(t) => p.text_block(t, 10.0, false, 0.0, 6.0),
        Block::KeyValues(kv) => {
            let key_w = (w - 2.0 * MARGIN) * 0.3;
            for (k, v) in kv {
                let vals = wrap_pt(v, w - 2.0 * MARGIN - key_w, 9.5, false);
                let h = vals.len().max(1) as f32 * 12.5;
                p.ensure(h);
                let top = p.y;
                p.ops().push(Op::Text {
                    bold: true,
                    size: 9.5,
                    x: MARGIN,
                    y: top - 10.0,
                    text: k.clone(),
                    gray: 0.25,
                });
                for (i, line) in vals.iter().enumerate() {
                    p.ops().push(Op::Text {
                        bold: false,
                        size: 9.5,
                        x: MARGIN + key_w,
                        y: top - 10.0 - i as f32 * 12.5,
                        text: line.clone(),
                        gray: 0.0,
                    });
                }
                p.y -= h;
            }
            p.y -= 6.0;
        }
        Block::Table(t) => render_table(p, t),
        Block::PageBreak => {
            p.hold = None;
            p.new_page();
        }
        Block::Grid(g) => render_grid(p, g),
        Block::SectionBar(text) => {
            let bar = 17.0;
            p.ensure(bar + 40.0);
            p.y -= 6.0;
            let (start, top) = (p.ops().len(), p.y);
            p.ops().push(Op::Shape {
                x: MARGIN,
                y: top - bar,
                w: w - 2.0 * MARGIN,
                h: bar,
                r: 3.0,
                round_top: true,
                round_bottom: true,
                fill: Some(pal::NAVY),
                stroke: None,
            });
            p.ops().push(Op::Text {
                bold: true,
                size: 9.5,
                x: MARGIN + 6.0,
                y: top - bar + 5.0,
                text: text.to_uppercase(),
                gray: 1.0,
            });
            p.y -= bar + 6.0;
            p.keep_with_next(start, top);
        }
        Block::Centered { text, size, bold } => p.centered(text, *size, *bold, 0.0),
        Block::Note(text) => p.text_block(text, 8.5, false, 0.4, 4.0),
        Block::Indented { text, level, muted } => {
            let indent = (*level as f32 * 18.0).min(w / 3.0);
            if *muted {
                p.text_block_at(indent, text, 8.5, false, 0.4, 3.0);
            } else {
                p.text_block_at(indent, text, 10.0, false, 0.0, 4.0);
            }
        }
        Block::Spacer(pt) => {
            if p.room() > *pt {
                p.y -= pt;
            }
        }
    }
}

/// A group of blocks laid out as one unit (see [`Block::Together`]): measured on a scratch page
/// first, then placed where the whole group fits, or on a fresh page when it does not. A group
/// taller than a page also starts fresh and then splits only where its own blocks split.
fn together<'a>(p: &mut Pager<'a>, blocks: &'a [Block]) {
    let mut trial = Pager::new(p.w, p.h, p.inset, p.bottom);
    for b in blocks {
        render_block(&mut trial, b);
    }
    let one_page = trial.pages.len() == 1;
    let used = trial.h - trial.inset - trial.y;
    if !p.fresh() && (!one_page || p.room() < used) {
        p.new_page();
    }
    for b in blocks {
        render_block(p, b);
    }
}

/// The running header split into lines that fit the page width (tracking included).
fn header_lines(doc: &Document, page_w: f32) -> Vec<String> {
    match doc.running_header.as_deref().map(str::trim) {
        Some(t) if !t.is_empty() => wrap_pt(t, (page_w - 2.0 * MARGIN) * 0.94, HEADER_SIZE, false),
        _ => Vec::new(),
    }
}

/// Distance from the page top to the first line of content: the top margin, or more when a long
/// running header needs extra lines.
fn top_inset(header_line_count: usize) -> f32 {
    if header_line_count == 0 {
        return MARGIN;
    }
    let last_base = HEADER_BASE + (header_line_count - 1) as f32 * HEADER_LEADING;
    MARGIN.max(last_base + 9.0)
}

/// The official logo, decoded once and compressed once. `None` only if the bundled PNG were
/// unreadable, which the tests rule out.
fn logo() -> Option<&'static LosslessImage> {
    static LOGO: OnceLock<Option<LosslessImage>> = OnceLock::new();
    LOGO.get_or_init(|| {
        let decoded = image::load_from_memory(LOGO_PNG).ok()?;
        let has_alpha = decoded.color().has_alpha();
        let img = decoded.to_rgba8();
        let (w, h) = img.dimensions();
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        let mut alpha = Vec::with_capacity((w * h) as usize);
        for px in img.pixels() {
            rgb.extend_from_slice(&px.0[..3]);
            alpha.push(px.0[3]);
        }
        let deflate = |data: &[u8]| -> Option<Vec<u8>> {
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            z.write_all(data).ok()?;
            z.finish().ok()
        };
        Some(LosslessImage {
            width: w,
            height: h,
            rgb_flate: deflate(&rgb)?,
            // The artwork's alpha channel is kept as a soft mask, whatever it holds.
            alpha_flate: if has_alpha {
                Some(deflate(&alpha)?)
            } else {
                None
            },
        })
    })
    .as_ref()
}

fn render_pdf(doc: &Document) -> ReportPdf {
    let (w, h) = if doc.landscape {
        (792.0, 612.0)
    } else {
        (612.0, 792.0)
    };
    let head = header_lines(doc, w);
    let bottom = if doc.brand.is_some() {
        BOTTOM_MARGIN
    } else {
        PLAIN_BOTTOM_MARGIN
    };
    let mut p = Pager::new(w, h, top_inset(head.len()), bottom);
    let masthead_first = matches!(doc.blocks.first(), Some(Block::Masthead(_)));
    if masthead_first {
        // The design system masthead replaces the plain title lines.
    } else if doc.centered_header {
        p.centered(&doc.title, 16.0, true, 0.0);
        if let Some(s) = &doc.subtitle {
            p.y -= 2.0;
            p.centered(s, 10.0, false, 0.3);
        }
        p.y -= 8.0;
    } else {
        p.text_block(&doc.title, 16.0, true, 0.0, 2.0);
        if let Some(s) = &doc.subtitle {
            p.text_block(s, 9.5, false, 0.4, 6.0);
        }
    }
    if !masthead_first {
        p.y -= 6.0;
    }
    for b in &doc.blocks {
        render_block(&mut p, b);
    }
    let total = p.pages.len();
    let footers: Vec<String> = (0..total)
        .map(|i| {
            if doc.brand.is_some() {
                format!("Page {} of {total}", i + 1)
            } else {
                let name = doc.footer.as_deref().unwrap_or(&doc.title);
                format!("{name} · Page {} of {total}", i + 1)
            }
        })
        .collect();
    // Pass 1: every run that needs the embedded fonts, shaped once (ADR-0016).
    let mut shaped = Shaped::default();
    for (ops, footer) in p.pages.iter().zip(&footers) {
        for op in ops {
            if let Op::Text { bold, text, .. } | Op::TextC { bold, text, .. } = op {
                shaped.collect(text, *bold);
            }
        }
        shaped.collect(footer, false);
    }
    if let Some(line) = &doc.brand {
        shaped.collect(line, false);
    }
    for line in &head {
        shaped.collect(line, false);
    }
    let pack = shaped.pack();
    let mut b = PdfBuilder::new(w, h);
    let images: Vec<usize> = p.images.iter().map(|img| b.add_image(img.pdf())).collect();
    // The logo is one image object shared by every page.
    let logo_ref = match (&doc.brand, logo()) {
        (Some(_), Some(l)) => Some((b.add_lossless_image(l.clone()), l.width, l.height)),
        _ => None,
    };
    for (i, ops) in p.pages.into_iter().enumerate() {
        let mut c = Content::new();
        let mut used: Vec<usize> = Vec::new();
        for op in ops {
            match op {
                Op::Frame { x, y, w, h } => {
                    c.set_stroke_gray(0.7);
                    c.set_line_width(0.5);
                    c.rect(x, y, w, h);
                    c.stroke();
                }
                Op::Image { index, x, y, w, h } => {
                    let id = images[index];
                    if !used.contains(&id) {
                        used.push(id);
                    }
                    PdfBuilder::draw_image(&mut c, id, x, y, w, h);
                }
                Op::Rect { x, y, w, h, gray } => {
                    c.set_fill_gray(gray);
                    c.rect(x, y, w, h);
                    c.fill_nonzero();
                    c.set_fill_gray(0.0);
                }
                Op::Line { x1, y1, x2, y2 } => {
                    c.set_stroke_gray(0.8);
                    c.set_line_width(0.4);
                    c.move_to(x1, y1);
                    c.line_to(x2, y2);
                    c.stroke();
                }
                Op::TextC {
                    bold,
                    size,
                    x,
                    y,
                    text,
                    color,
                    track,
                } => {
                    c.set_fill_rgb(color.0, color.1, color.2);
                    c.set_stroke_rgb(color.0, color.1, color.2);
                    design::draw_text_tracked(
                        &mut b, &mut c, &shaped, &pack, bold, size, x, y, &text, track,
                    );
                    c.set_fill_gray(0.0);
                    c.set_stroke_gray(0.0);
                }
                Op::Shape {
                    x,
                    y,
                    w,
                    h,
                    r,
                    round_top,
                    round_bottom,
                    fill,
                    stroke,
                } => {
                    design::draw_shape(&mut c, x, y, w, h, r, round_top, round_bottom, fill, stroke)
                }
                Op::LineC {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    width,
                } => {
                    c.set_stroke_rgb(color.0, color.1, color.2);
                    c.set_line_width(width);
                    c.move_to(x1, y1);
                    c.line_to(x2, y2);
                    c.stroke();
                    c.set_stroke_gray(0.0);
                }
                Op::Circle {
                    x,
                    y,
                    r,
                    fill,
                    stroke,
                } => design::draw_circle(&mut c, x, y, r, fill, stroke),
                Op::Text {
                    bold,
                    size,
                    x,
                    y,
                    text,
                    gray,
                } => {
                    c.set_fill_gray(gray);
                    c.set_stroke_gray(gray);
                    draw_text(&mut b, &mut c, &shaped, &pack, bold, size, x, y, &text);
                    c.set_fill_gray(0.0);
                    c.set_stroke_gray(0.0);
                }
            }
        }
        for (k, line) in head.iter().enumerate() {
            let track = if needs_fonts(line) { 0.0 } else { HEADER_TRACK };
            let lw = text_width(line, HEADER_SIZE, false) + track * line.chars().count() as f32;
            c.set_fill_rgb(pal::SLATE.0, pal::SLATE.1, pal::SLATE.2);
            c.set_stroke_rgb(pal::SLATE.0, pal::SLATE.1, pal::SLATE.2);
            design::draw_text_tracked(
                &mut b,
                &mut c,
                &shaped,
                &pack,
                false,
                HEADER_SIZE,
                w - MARGIN - lw,
                h - HEADER_BASE - k as f32 * HEADER_LEADING,
                line,
                track,
            );
        }
        let footer = &footers[i];
        if let Some(line) = &doc.brand {
            if let Some((id, lw, lh)) = logo_ref {
                if !used.contains(&id) {
                    used.push(id);
                }
                let logo_w = LOGO_H * lw as f32 / lh as f32;
                PdfBuilder::draw_image(&mut c, id, MARGIN, LOGO_BOTTOM, logo_w, LOGO_H);
            }
            c.set_fill_rgb(pal::SLATE.0, pal::SLATE.1, pal::SLATE.2);
            draw_text(
                &mut b,
                &mut c,
                &shaped,
                &pack,
                false,
                BRAND_SIZE,
                MARGIN,
                FOOTER_BASE,
                line,
            );
            let fw = text_width(footer, PAGE_SIZE, false);
            c.set_fill_rgb(pal::INK.0, pal::INK.1, pal::INK.2);
            draw_text(
                &mut b,
                &mut c,
                &shaped,
                &pack,
                false,
                PAGE_SIZE,
                w - MARGIN - fw,
                FOOTER_BASE,
                footer,
            );
        } else {
            let fw = text_width(footer, 8.0, false);
            c.set_fill_gray(0.45);
            c.set_stroke_gray(0.45);
            draw_text(
                &mut b,
                &mut c,
                &shaped,
                &pack,
                false,
                8.0,
                (w - fw) / 2.0,
                MARGIN - 20.0,
                footer,
            );
        }
        c.set_fill_gray(0.0);
        c.set_stroke_gray(0.0);
        b.add_page_with_images(c, used);
    }
    let replaced = b.replaced_chars + shaped.unprintable;
    let pages = b.page_count();
    if let Some(pack) = pack {
        b.add_embedded_fonts(pack);
    }
    ReportPdf {
        bytes: b.finish(&doc.title, doc.author.as_deref()),
        pages,
        replaced_chars: replaced,
    }
}

/// The OpenFrame mark (the film frame of the app icon: a rounded square with two
/// sprocket rails and a centre line), drawn in grey with its lower left corner at
/// `(x, y)` and `size` points wide.
pub(crate) fn draw_mark(c: &mut Content, x: f32, y: f32, size: f32, color: Rgb) {
    let u = size / 24.0;
    let p = |vx: f32, vy: f32| (x + vx * u, y + (24.0 - vy) * u);
    c.set_stroke_rgb(color.0, color.1, color.2);
    c.set_line_width((size * 0.075).max(0.6));
    let (x0, y0) = p(2.0, 22.0);
    c.rect(x0, y0, 20.0 * u, 20.0 * u);
    c.stroke();
    for (x1, y1, x2, y2) in [
        (7.0, 2.0, 7.0, 22.0),
        (17.0, 2.0, 17.0, 22.0),
        (7.0, 12.0, 17.0, 12.0),
        (2.0, 7.0, 7.0, 7.0),
        (2.0, 17.0, 7.0, 17.0),
        (17.0, 7.0, 22.0, 7.0),
        (17.0, 17.0, 22.0, 17.0),
    ] {
        let (ax, ay) = p(x1, y1);
        let (bx, by) = p(x2, y2);
        c.move_to(ax, ay);
        c.line_to(bx, by);
        c.stroke();
    }
}

/// The runs of a report that need the embedded fonts, shaped once each.
#[derive(Default)]
struct Shaped {
    lines: Vec<ShapedText>,
    index: HashMap<(String, bool), usize>,
    /// Characters drawn without a glyph (no bundled font covers them).
    unprintable: usize,
}

impl Shaped {
    fn collect(&mut self, text: &str, bold: bool) {
        if !needs_fonts(text) {
            return;
        }
        for (embedded, seg) in segments(text) {
            if !embedded {
                continue;
            }
            let key = (seg.to_string(), bold);
            if self.index.contains_key(&key) {
                continue;
            }
            let missing = fonts::unprintable_chars(seg);
            self.unprintable += seg.chars().filter(|c| missing.contains(c)).count();
            self.index.insert(key, self.lines.len());
            self.lines.push(ShapedText::new(seg, bold, false));
        }
    }

    fn get(&self, seg: &str, bold: bool) -> Option<&ShapedText> {
        self.index
            .get(&(seg.to_string(), bold))
            .map(|i| &self.lines[*i])
    }

    fn pack(&self) -> Option<FontPack> {
        if self.lines.is_empty() {
            return None;
        }
        let refs: Vec<&ShapedText> = self.lines.iter().collect();
        Some(FontPack::build(&refs))
    }
}

/// Draw one line of report text: Helvetica for Windows-1252 runs, the
/// embedded fonts for the rest, left to right from `x`.
#[allow(clippy::too_many_arguments)]
fn draw_text(
    b: &mut PdfBuilder,
    c: &mut Content,
    shaped: &Shaped,
    pack: &Option<FontPack>,
    bold: bool,
    size: f32,
    x: f32,
    y: f32,
    text: &str,
) {
    let font = if bold {
        pdfgen::HELVETICA_BOLD
    } else {
        pdfgen::HELVETICA
    };
    if !needs_fonts(text) {
        b.text(c, font, size, x, y, text);
        return;
    }
    let mut pen = x;
    for (embedded, seg) in segments(text) {
        match (embedded, pack, shaped.get(seg, bold)) {
            (true, Some(pack), Some(line)) => pen += pack.draw(c, line, pen, y, size),
            _ => {
                b.text(c, font, size, pen, y, seg);
                pen += text_width_pt(seg, size, bold);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branded_pdfs_and_workbooks_name_openframe_and_plain_ones_do_not() {
        let doc = Document::new("Shot List").with(Block::Paragraph("Scene 1".into()));
        let plain = String::from_utf8_lossy(&doc.to_pdf().bytes).into_owned();
        assert!(!plain.contains(BRAND_LINE));
        let branded = doc.clone().brand(BRAND_LINE).to_pdf();
        assert!(String::from_utf8_lossy(&branded.bytes).contains(BRAND_LINE));
        assert_eq!(branded.replaced_chars, 0);

        let mut t = Table::new("Shots", vec![Column::new("Shot")]);
        t.push(vec!["1".into()]);
        let unzip = |bytes: Vec<u8>, name: &str| -> String {
            let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut f = z.by_name(name).unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };
        let wb = tables_to_xlsx_branded(&[&t], BRAND_LINE, Some(0)).unwrap();
        assert!(unzip(wb.clone(), "docProps/core.xml").contains("OpenFrame Studio"));
        assert!(unzip(wb, "xl/worksheets/sheet1.xml").contains(BRAND_LINE));
        let wb = tables_to_xlsx(&[&t]).unwrap();
        assert!(!unzip(wb, "xl/worksheets/sheet1.xml").contains(BRAND_LINE));
    }

    /// A document that uses every design block, long enough to run onto several pages.
    fn designed_document() -> Document {
        let long =
            "A long synopsis that has to wrap inside its column and never be cut short. ".repeat(6);
        let mut mast = Masthead::new("MONSOON LETTERS").eyebrow("Call sheet", pal::SLATE);
        mast.badges.push(Chip::label("Draft", Tone::Warn));
        mast.side = Some(Side::Dark {
            label: "Shoot day".into(),
            big: Some("01".into()),
            lines: vec![("Monday".into(), true)],
            foot: Some("Day 1 of 4".into()),
        });
        let mut doc = Document::new("Designed").footer("Monsoon Letters · Call sheet");
        doc.push(Block::Masthead(mast));
        doc.push(Block::Metrics(Metrics {
            items: vec![Metric::new("12", "Scenes"), Metric::new("3 3/8", "Pages")],
            compact: false,
        }));
        doc.push(Block::Section(Section::new(
            "Locations",
            Some("2 total".into()),
        )));
        let cards: Vec<Card> = (0..8)
            .map(|i| Card {
                head: Some(CardHead::new(format!("LOCATION {i}"))),
                parts: vec![CardPart::Fields(vec![
                    Field::new("Address", long.clone()),
                    Field::missing("Hospital", "Needs input"),
                ])],
                ..Card::new()
            })
            .collect();
        doc.push(Block::Cards(Cards::new(2, cards)));
        let mut t = StyledTable::new(vec![Col::pt("Sc", 40.0), Col::weight("Heading", 1.0)]);
        t.continued = Some("Schedule".into());
        for i in 0..40 {
            t.rows.push(Row::new(vec![
                Cell::new().item(CellItem::Tile(i.to_string())),
                Cell::text(Txt::new(format!("ROW {i} {long}"))),
            ]));
        }
        doc.push(Block::Styled(t));
        doc.push(Block::Legend(Legend {
            title: None,
            items: vec![LegendItem {
                swatch: pal::ORANGE,
                border: None,
                code: Some(("SW".into(), pal::WHITE)),
                label: "Start work".into(),
            }],
            columns: 0,
        }));
        doc.brand(BRAND_LINE)
    }

    /// Every base-14 text string on the pages, decoded from Windows-1252 (strings with a byte
    /// above ASCII are written as hex, so the raw file bytes cannot be searched for them).
    fn page_strings(bytes: &[u8]) -> String {
        let doc = lopdf::Document::load_mem(bytes).expect("PDF parses");
        let mut out = String::new();
        for (_, page) in doc.get_pages() {
            let content = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap();
            for op in content.operations {
                if let ("Tj", [lopdf::Object::String(b, _), ..]) =
                    (op.operator.as_str(), op.operands.as_slice())
                {
                    out.extend(b.iter().map(|x| crate::winansi::decode_byte(*x)));
                    out.push('\n');
                }
            }
        }
        out
    }

    #[test]
    fn design_blocks_paginate_deterministically_and_keep_every_word() {
        let doc = designed_document();
        let a = doc.to_pdf();
        let b = doc.to_pdf();
        assert_eq!(a.bytes, b.bytes, "identical input gives identical bytes");
        assert_eq!(a.replaced_chars, 0);
        assert!(a.pages >= 3, "{} pages", a.pages);
        let text = String::from_utf8_lossy(&a.bytes).into_owned();
        // Wrapped, never cut: the end of the long text is printed for every row.
        assert!(text.contains("short."), "wrapped text keeps its tail");
        assert!(!text.contains('\u{2026}'), "no ellipsis in design output");
        // The brand line and the page count are on every page.
        assert_eq!(text.matches(BRAND_LINE).count(), a.pages);
        assert!(page_strings(&a.bytes).contains(&format!("Page {} of {}", a.pages, a.pages)));
    }

    #[test]
    fn design_blocks_shape_indian_scripts_with_the_embedded_fonts() {
        let mut doc = Document::new("Telugu").footer("మీరా · Call sheet");
        doc.push(Block::Masthead(Masthead::new("రైలు లేఖలు")));
        doc.push(Block::Text(Txt::new(
            "మీరా రైలు స్టేషన్‌లో వేచి ఉంది. मीरा ने कहा, “क्षमा करें, ट्रेन देर से है।”",
        )));
        let tel = "మీరా రైలు స్టేషన్";
        doc.push(Block::Metrics(Metrics {
            items: vec![Metric::new("12", tel)],
            compact: false,
        }));
        doc.push(Block::Section(Section::new(tel, Some(tel.into()))));
        doc.push(Block::Banner(Banner::new(tel)));
        let card = Card {
            head: Some(CardHead::new(tel)),
            parts: vec![CardPart::Fields(vec![Field::new(tel, tel)])],
            ..Card::new()
        };
        doc.push(Block::Cards(Cards::new(2, vec![card])));
        let mut t = StyledTable::new(vec![Col::weight(tel, 1.0)]);
        t.rows
            .push(Row::new(vec![Cell::text(Txt::new(tel).bold())]));
        t.rows.push(Row::new(vec![
            Cell::new().item(CellItem::Chips(vec![Chip::label(tel, Tone::Info)])),
        ]));
        doc.push(Block::Styled(t));
        let doc = doc.brand(BRAND_LINE);
        let pdf = doc.to_pdf();
        assert_eq!(
            pdf.replaced_chars,
            0,
            "no glyph for {:?}",
            doc.unsupported_chars()
        );
        let text = String::from_utf8_lossy(&pdf.bytes).into_owned();
        assert!(
            text.contains("/FontFile2") || text.contains("/FontFile3"),
            "fonts are embedded"
        );
    }

    #[test]
    fn design_blocks_leave_the_unbranded_report_look_alone() {
        let doc = Document::new("Plain").with(Block::Paragraph("Scene 1".into()));
        let pdf = doc.to_pdf();
        let text = String::from_utf8_lossy(&pdf.bytes).into_owned();
        assert!(
            page_strings(&pdf.bytes).contains("Page 1 of 1"),
            "legacy footer kept"
        );
        assert!(!text.contains(BRAND_LINE));
    }

    #[test]
    fn report_segments_keep_clusters_whole_and_alternate_fonts() {
        let segs = segments("Call 06:00 మీరా Hall");
        assert_eq!(
            segs,
            vec![(false, "Call 06:00 "), (true, "మీరా"), (false, " Hall")]
        );
        assert_eq!(segments("Plain"), vec![(false, "Plain")]);
        assert!(segments("").is_empty());
    }

    #[test]
    fn shaped_wrapping_never_splits_a_cluster_and_fits_the_width() {
        let text = "క్షేత్రం ".repeat(40) + &"ప్రయాణం".repeat(30);
        let lines = wrap_pt(&text, 150.0, 10.0, false);
        assert!(lines.len() > 3);
        let joined: String = lines.concat();
        let original: String = text.split_whitespace().collect();
        assert_eq!(joined.replace(' ', ""), original, "no character lost");
        for l in &lines {
            assert!(
                text_width(l, 10.0, false) <= 150.0 + 1e-3 || l.graphemes(true).count() == 1,
                "{l}"
            );
        }
    }

    #[test]
    fn wraps_by_width() {
        let lines = wrap_pt(
            "The quick brown fox jumps over the lazy dog",
            60.0,
            10.0,
            false,
        );
        assert!(lines.len() > 1);
        assert!(
            lines
                .iter()
                .all(|l| text_width_pt(l, 10.0, false) <= 60.0 || !l.contains(' '))
        );
    }

    #[test]
    fn sheet_names_are_valid_and_unique() {
        let mut used = Vec::new();
        assert_eq!(sheet_name("Cast/Crew: [A]", &mut used), "CastCrew A");
        assert_eq!(sheet_name("CastCrew A", &mut used), "CastCrew A 2");
        assert_eq!(sheet_name("", &mut used), "Sheet");
    }

    #[test]
    fn csv_cells_never_start_a_formula() {
        let mut t = Table::new("Cast", vec![Column::new("Name")]);
        for cell in [
            "=1+1",
            "+SUM(A1)",
            "-2+3",
            "@cmd",
            "\tTab",
            "-",
            "-1500",
            "3.5",
            "Meera",
            "మీరా",
        ] {
            t.rows.push(vec![cell.to_string()]);
        }
        let csv = String::from_utf8(t.to_csv().unwrap()).unwrap();
        let lines: Vec<&str> = csv.trim_start_matches('\u{feff}').lines().collect();
        assert_eq!(
            &lines[1..],
            [
                "'=1+1",
                "'+SUM(A1)",
                "'-2+3",
                "'@cmd",
                "'\tTab",
                "-",
                "-1500",
                "3.5",
                "Meera",
                "మీరా"
            ]
        );
    }
}
