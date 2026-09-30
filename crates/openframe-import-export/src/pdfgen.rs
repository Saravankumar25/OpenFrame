//! Minimal PDF assembly over `pdf-writer` using only base-14 fonts (nothing is
//! embedded). Shared by the screenplay PDF writer and the report writer.
//! Pictures (storyboard panels, moodboard images) are embedded as baseline
//! JPEG image XObjects (`DCTDecode`).

use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};

/// Font resource names used in content streams.
pub const COURIER: &[u8] = b"F1";
pub const COURIER_OBLIQUE: &[u8] = b"F2";
pub const COURIER_BOLD: &[u8] = b"F3";
pub const HELVETICA: &[u8] = b"F4";
pub const HELVETICA_BOLD: &[u8] = b"F5";

const FONTS: [(&[u8], &[u8]); 5] = [
    (COURIER, b"Courier"),
    (COURIER_OBLIQUE, b"Courier-Oblique"),
    (COURIER_BOLD, b"Courier-Bold"),
    (HELVETICA, b"Helvetica"),
    (HELVETICA_BOLD, b"Helvetica-Bold"),
];

/// A JPEG picture ready to embed (see [`crate::report::ReportImage`]).
#[derive(Debug, Clone)]
pub struct PdfImage {
    pub width: u32,
    pub height: u32,
    pub gray: bool,
    pub jpeg: Vec<u8>,
}

pub struct PdfBuilder {
    pub width_pt: f32,
    pub height_pt: f32,
    pages: Vec<(Vec<u8>, Vec<usize>)>,
    images: Vec<PdfImage>,
    /// Characters replaced with '?' because WinAnsi cannot represent them.
    pub replaced_chars: usize,
}

/// Resource name of the n-th embedded image ("Im0", "Im1", …).
pub fn image_name(index: usize) -> Vec<u8> {
    format!("Im{index}").into_bytes()
}

impl PdfBuilder {
    pub fn new(width_pt: f32, height_pt: f32) -> Self {
        Self {
            width_pt,
            height_pt,
            pages: Vec::new(),
            images: Vec::new(),
            replaced_chars: 0,
        }
    }

    pub fn add_page(&mut self, content: Content) {
        self.pages.push((content.finish().into_vec(), Vec::new()));
    }

    /// Register a picture; draw it with [`PdfBuilder::draw_image`] and list its
    /// index in [`PdfBuilder::add_page_with_images`].
    pub fn add_image(&mut self, img: PdfImage) -> usize {
        self.images.push(img);
        self.images.len() - 1
    }

    /// Draw a registered image into the rectangle (points, bottom-left origin).
    pub fn draw_image(c: &mut Content, index: usize, x: f32, y: f32, w: f32, h: f32) {
        c.save_state();
        c.transform([w, 0.0, 0.0, h, x, y]);
        c.x_object(Name(&image_name(index)));
        c.restore_state();
    }

    /// A page that draws the given registered images.
    pub fn add_page_with_images(&mut self, content: Content, images: Vec<usize>) {
        self.pages.push((content.finish().into_vec(), images));
    }

    /// Draw WinAnsi-encoded text at (x, y) in points from the bottom-left.
    pub fn text(&mut self, c: &mut Content, font: &[u8], size: f32, x: f32, y: f32, text: &str) {
        let (bytes, replaced) = crate::winansi::encode(text);
        self.replaced_chars += replaced;
        if bytes.is_empty() {
            return;
        }
        c.begin_text();
        c.set_font(Name(font), size);
        c.set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
        c.show(Str(&bytes));
        c.end_text();
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn finish(self, title: &str, author: Option<&str>) -> Vec<u8> {
        let mut pdf = Pdf::new();
        let catalog = Ref::new(1);
        let tree = Ref::new(2);
        let info = Ref::new(3);
        let font_base = 10;
        let page_base = 100;
        let image_base = page_base + (self.pages.len() as i32) * 2 + 10;
        pdf.catalog(catalog).pages(tree);
        let page_ids: Vec<Ref> = (0..self.pages.len())
            .map(|i| Ref::new(page_base + (i as i32) * 2))
            .collect();
        pdf.pages(tree)
            .kids(page_ids.iter().copied())
            .count(self.pages.len() as i32);
        for (i, (_, base)) in FONTS.iter().enumerate() {
            pdf.type1_font(Ref::new(font_base + i as i32))
                .base_font(Name(base))
                .encoding_predefined(Name(b"WinAnsiEncoding"));
        }
        for (i, img) in self.images.iter().enumerate() {
            let mut x = pdf.image_xobject(Ref::new(image_base + i as i32), &img.jpeg);
            x.filter(Filter::DctDecode);
            x.width(img.width as i32);
            x.height(img.height as i32);
            if img.gray {
                x.color_space().device_gray();
            } else {
                x.color_space().device_rgb();
            }
            x.bits_per_component(8);
            x.finish();
        }
        for (i, (bytes, used)) in self.pages.iter().enumerate() {
            let page_id = page_ids[i];
            let content_id = Ref::new(page_base + (i as i32) * 2 + 1);
            let mut page = pdf.page(page_id);
            page.media_box(Rect::new(0.0, 0.0, self.width_pt, self.height_pt));
            page.parent(tree);
            page.contents(content_id);
            {
                let mut res = page.resources();
                let mut fonts = res.fonts();
                for (k, (name, _)) in FONTS.iter().enumerate() {
                    fonts.pair(Name(name), Ref::new(font_base + k as i32));
                }
                fonts.finish();
                if !used.is_empty() {
                    let mut xs = res.x_objects();
                    for idx in used {
                        xs.pair(Name(&image_name(*idx)), Ref::new(image_base + *idx as i32));
                    }
                    xs.finish();
                }
            }
            page.finish();
            pdf.stream(content_id, bytes);
        }
        {
            let mut d = pdf.document_info(info);
            d.title(TextStr(title));
            if let Some(a) = author {
                d.author(TextStr(a));
            }
            d.creator(TextStr("OpenFrame Studio"));
            d.producer(TextStr("OpenFrame Studio"));
            d.finish();
        }
        pdf.finish()
    }
}
