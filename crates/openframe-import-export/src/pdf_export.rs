//! Screenplay PDF: renders [`crate::layout`] pages with base-14 Courier 12pt on
//! US Letter. Printing uses exactly the same layout (FSD §123).

use pdf_writer::Content;

use crate::layout::{self, Layout, LayoutOptions, LineStyle, PAGE_HEIGHT_IN, PAGE_WIDTH_IN};
use crate::model::ScreenplayDoc;
use crate::pdfgen::{self, PdfBuilder};

#[derive(Debug, Clone)]
pub struct PdfOutput {
    pub bytes: Vec<u8>,
    /// Script pages (title page excluded).
    pub pages: usize,
    /// Characters that the standard PDF font could not represent (printed as '?').
    pub replaced_chars: usize,
}

pub fn render_layout(lay: &Layout, title: &str, author: Option<&str>) -> PdfOutput {
    let mut b = PdfBuilder::new(PAGE_WIDTH_IN * 72.0, PAGE_HEIGHT_IN * 72.0);
    for page in &lay.pages {
        let mut c = Content::new();
        for l in &page.lines {
            let font = if l.style.bold() {
                pdfgen::COURIER_BOLD
            } else if l.style.oblique() {
                pdfgen::COURIER_OBLIQUE
            } else {
                pdfgen::COURIER
            };
            let x = l.x_in * 72.0;
            let y = (PAGE_HEIGHT_IN - l.y_in()) * 72.0;
            b.text(&mut c, font, 12.0, x, y, &l.text);
            if l.style == LineStyle::TitleMain {
                // Underline the title.
                let w = l.text.chars().count() as f32 * 7.2;
                c.set_line_width(0.6);
                c.move_to(x, y - 2.0);
                c.line_to(x + w, y - 2.0);
                c.stroke();
            }
        }
        b.add_page(c);
    }
    let replaced = b.replaced_chars;
    PdfOutput {
        bytes: b.finish(title, author),
        pages: lay.script_page_count(),
        replaced_chars: replaced,
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
