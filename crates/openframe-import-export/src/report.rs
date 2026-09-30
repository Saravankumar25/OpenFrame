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
//! PDFs use the base-14 Helvetica fonts: text outside the WinAnsi character
//! set is printed as `?` and counted in [`ReportPdf::replaced_chars`] so the
//! caller can warn the user. Document exports use [`Document::to_pdf_strict`],
//! which never writes such a PDF and instead names the characters. CSV/XLSX
//! keep full Unicode.
//!
//! Besides headings, paragraphs, label/value lists and tables, a document can
//! hold picture grids ([`Grid`], e.g. storyboard panels or moodboard images),
//! dark section bars (paper call sheet), centered lines and spacers, and can
//! be laid out landscape with a centered header and a custom footer.

use std::io::Cursor;

use openframe_domain::{AppError, AppResult};
use pdf_writer::Content;

use crate::pdfgen::{self, PdfBuilder, PdfImage};
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
        w.write_record(self.columns.iter().map(|c| c.header.as_str()))
            .map_err(err)?;
        for r in &self.rows {
            let cells: Vec<&str> = (0..self.columns.len())
                .map(|i| r.get(i).map(|s| s.as_str()).unwrap_or(""))
                .collect();
            w.write_record(&cells).map_err(err)?;
        }
        w.into_inner().map_err(|e| {
            AppError::export("csv_failed", "The CSV file could not be created.")
                .with_detail(e.to_string())
        })
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
    /// Footer text before "Page n of m" (defaults to the title).
    pub footer: Option<String>,
    pub author: Option<String>,
    pub blocks: Vec<Block>,
}

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
            blocks: Vec::new(),
        }
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
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Table(t) => Some(t),
                _ => None,
            })
            .collect()
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
                    "This document contains characters the standard PDF font can't print{such_as}. \
                     No PDF was written. {suggestion}"
                ),
            )
            .with_detail(format!(
                "{} non-WinAnsi character(s), {} distinct",
                out.replaced_chars,
                bad.len()
            )));
        }
        Ok(out)
    }

    /// Distinct characters (first-appearance order) that the base-14 PDF fonts
    /// can't print. Empty = the PDF will be exact.
    pub fn unsupported_chars(&self) -> Vec<char> {
        let mut out: Vec<char> = Vec::new();
        let mut scan = |t: &str| {
            for c in t.chars() {
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
        for b in &self.blocks {
            match b {
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
            }
        }
        out
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
    use rust_xlsxwriter::{Format, Workbook};
    let err = |e: rust_xlsxwriter::XlsxError| {
        AppError::export("xlsx_failed", "The spreadsheet could not be created.")
            .with_detail(e.to_string())
    };
    let mut wb = Workbook::new();
    let bold = Format::new().set_bold();
    let mut used = Vec::new();
    if tables.is_empty() {
        wb.add_worksheet();
    }
    for t in tables {
        let ws = wb.add_worksheet();
        ws.set_name(sheet_name(&t.name, &mut used)).map_err(err)?;
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

const MARGIN: f32 = 54.0;
const FOOTER_SPACE: f32 = 30.0;

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
}

/// Wrap text to a width in points.
pub fn wrap_pt(text: &str, width: f32, size: f32, bold: bool) -> Vec<String> {
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
    pages: Vec<Vec<Op>>,
    y: f32,
    images: Vec<&'a ReportImage>,
}

impl Pager<'_> {
    fn new_page(&mut self) {
        self.pages.push(Vec::new());
        self.y = self.h - MARGIN;
    }
    fn ops(&mut self) -> &mut Vec<Op> {
        self.pages.last_mut().unwrap()
    }
    fn room(&self) -> f32 {
        self.y - (MARGIN + FOOTER_SPACE)
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
            let tw = text_width_pt(&line, size, bold);
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
            let tw = text_width_pt(s, size, true);
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
                let tw = text_width_pt(s, size, false);
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
    let max_h = p.h - 2.0 * MARGIN - FOOTER_SPACE;
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
                                let tw = text_width_pt(&l, 9.0, false);
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

fn render_pdf(doc: &Document) -> ReportPdf {
    let (w, h) = if doc.landscape {
        (792.0, 612.0)
    } else {
        (612.0, 792.0)
    };
    let mut p = Pager {
        w,
        h,
        pages: Vec::new(),
        y: 0.0,
        images: Vec::new(),
    };
    p.new_page();
    if doc.centered_header {
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
    p.y -= 6.0;
    for b in &doc.blocks {
        match b {
            Block::Heading { text, level } => {
                let size = if *level <= 1 { 13.0 } else { 11.0 };
                // Keep headings with what follows them.
                p.ensure(size * 1.3 + 40.0);
                p.y -= 4.0;
                p.text_block(text, size, true, 0.0, 4.0);
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
            Block::Table(t) => render_table(&mut p, t),
            Block::PageBreak => p.new_page(),
            Block::Grid(g) => render_grid(&mut p, g),
            Block::SectionBar(text) => {
                let bar = 17.0;
                p.ensure(bar + 40.0);
                p.y -= 6.0;
                let top = p.y;
                p.ops().push(Op::Rect {
                    x: MARGIN,
                    y: top - bar,
                    w: w - 2.0 * MARGIN,
                    h: bar,
                    gray: 0.18,
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
    let total = p.pages.len();
    let mut b = PdfBuilder::new(w, h);
    let images: Vec<usize> = p.images.iter().map(|img| b.add_image(img.pdf())).collect();
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
                Op::Text {
                    bold,
                    size,
                    x,
                    y,
                    text,
                    gray,
                } => {
                    c.set_fill_gray(gray);
                    b.text(
                        &mut c,
                        if bold {
                            pdfgen::HELVETICA_BOLD
                        } else {
                            pdfgen::HELVETICA
                        },
                        size,
                        x,
                        y,
                        &text,
                    );
                    c.set_fill_gray(0.0);
                }
            }
        }
        let footer = format!(
            "{} · Page {} of {}",
            doc.footer.as_deref().unwrap_or(&doc.title),
            i + 1,
            total
        );
        let fw = text_width_pt(&footer, 8.0, false);
        c.set_fill_gray(0.45);
        b.text(
            &mut c,
            pdfgen::HELVETICA,
            8.0,
            (w - fw) / 2.0,
            MARGIN - 20.0,
            &footer,
        );
        b.add_page_with_images(c, used);
    }
    let replaced = b.replaced_chars;
    let pages = b.page_count();
    ReportPdf {
        bytes: b.finish(&doc.title, doc.author.as_deref()),
        pages,
        replaced_chars: replaced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
