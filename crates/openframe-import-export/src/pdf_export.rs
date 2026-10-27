//! Screenplay PDF: renders [`crate::layout`] pages on the paper and size of the
//! screenplay's page setup. Printing and the print preview use exactly the
//! same layout (FSD §123).
//!
//! Fonts (ADR-0016): every PDF embeds subset fonts, so the page never depends on the viewer's
//! substitute for base-14 Courier: Courier Prime (regular, bold, oblique) for Latin text,
//! even when the whole script is plain ASCII, and the Noto Sans families for Indian scripts,
//! all shaped with rustybuzz ([`crate::pdf_fonts`]). Text is measured with the same shaped
//! advances it is drawn with, and Courier Prime keeps the 0.6 em advance of Courier, so the
//! screenplay columns, wrapping and page breaks do not move.
//!
//! Screenplay files carry no OpenFrame branding (Screenplay and Production
//! Depth Specification §43): the document information names only the title
//! and the author. Sides are pages of the same script and carry none either (owner, 2026-10-27).

use pdf_writer::Content;

use crate::layout::{self, Layout, LayoutOptions};
use crate::model::ScreenplayDoc;
use crate::pdf_fonts::{FontPack, ShapedText};
use crate::pdfgen::PdfBuilder;

#[derive(Debug, Clone)]
pub struct PdfOutput {
    pub bytes: Vec<u8>,
    /// Script pages (title page excluded).
    pub pages: usize,
    /// Characters that the standard PDF font could not represent (printed as '?').
    pub replaced_chars: usize,
}

/// Does this layout print any character outside Windows-1252? (Informational: every PDF now
/// embeds its fonts, so this no longer selects a different drawing path.)
pub fn needs_embedded_fonts(lay: &Layout) -> bool {
    lay.pages
        .iter()
        .flat_map(|p| p.lines.iter())
        .any(|l| crate::fonts::needs_embedding(&l.text))
}

/// Render the laid-out pages with embedded, shaped fonts (see the module notes).
pub fn render_layout(lay: &Layout, title: &str, author: Option<&str>) -> PdfOutput {
    let g = &lay.geometry;
    // Pass 1: shape every line so each font is subset once, in a fixed order.
    let shaped: Vec<Vec<ShapedText>> = lay
        .pages
        .iter()
        .map(|p| {
            p.lines
                .iter()
                .map(|l| ShapedText::new(&l.text, l.style.bold(), l.style.oblique()))
                .collect()
        })
        .collect();
    let all: Vec<&ShapedText> = shaped.iter().flatten().collect();
    let pack = FontPack::build(&all);
    // Pass 2: draw.
    let mut b = PdfBuilder::new(g.page_width_in * 72.0, g.page_height_in * 72.0);
    for (page, lines) in lay.pages.iter().zip(&shaped) {
        let mut c = Content::new();
        for (l, st) in page.lines.iter().zip(lines) {
            if l.text.is_empty() {
                continue;
            }
            let x = l.x_in * 72.0;
            let y = (g.page_height_in - lay.line_y_in(l)) * 72.0;
            let w = pack.draw(&mut c, st, x, y, g.font_size_pt);
            if l.style.underlined() {
                c.set_line_width(0.6);
                c.move_to(x, y - 2.0);
                c.line_to(x + w, y - 2.0);
                c.stroke();
            }
        }
        b.add_page(c);
    }
    b.embed_fonts(pack);
    PdfOutput {
        bytes: b.finish_unbranded(title, author),
        pages: lay.script_page_count(),
        replaced_chars: 0,
    }
}

pub fn write(doc: &ScreenplayDoc, opts: &LayoutOptions) -> PdfOutput {
    let lay = layout::layout(doc, opts);
    let title = doc
        .title_page
        .title()
        .unwrap_or("Screenplay")
        .replace('\n', " ");
    render_layout(&lay, &title, doc.title_page.author())
}
