//! Embedded, subset fonts for screenplay PDFs (ADR-0016).
//!
//! Two passes keep the output deterministic:
//!
//! 1. every printed line is itemized into font runs and shaped
//!    ([`ShapedText::new`]); the glyphs each font needs are collected;
//! 2. [`FontPack::build`] sorts those glyphs, subsets each font with the same
//!    ordering, and then [`FontPack::draw`] writes the glyphs of each line.
//!
//! Each font is written as a Type 0 font with a CIDFontType2 (TrueType)
//! descendant, `Identity-H` encoding, an exact `W` width array, a `ToUnicode`
//! map and the subset font file (compressed). Every printed line is wrapped in
//! a marked-content span whose `ActualText` is the line's exact text, so copy,
//! search, screen readers and OpenFrame's own PDF import get back every
//! character, including conjuncts, reordered vowel signs and zero-width
//! joiners that no glyph-to-character map can describe. Indian scripts have no
//! bold or italic face: bold lines are filled and stroked, italic lines are
//! slanted with the text matrix. No clock, no randomness: the subset tag is a
//! hash of the font and its glyph list.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, TextRenderingMode, UnicodeCmap};
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref, Str, TextStr};

use crate::fonts::{self, FontFace, FontRun, ShapedRun};

/// Slant of synthetic italics (tan of about 11 degrees).
const SYNTHETIC_SLANT: f32 = 0.2;
/// Stroke width of synthetic bold, as a share of the font size.
const SYNTHETIC_BOLD: f32 = 0.03;

/// One printed line, itemized and shaped, ready to draw.
#[derive(Debug, Clone)]
pub struct ShapedText {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    runs: Vec<(FontRun, ShapedRun)>,
}

impl ShapedText {
    /// Itemize and shape `text`. Characters no bundled font covers are drawn with
    /// the font's missing-glyph box; callers refuse such text before rendering
    /// (see `crate::check_pdf_characters`).
    pub fn new(text: &str, bold: bool, italic: bool) -> Self {
        let latin = fonts::latin_face(bold, italic);
        let runs = match fonts::itemize(text, latin) {
            Ok(runs) => runs,
            Err(_) => vec![FontRun {
                face: latin,
                range: 0..text.len(),
            }],
        };
        let runs = runs
            .into_iter()
            .map(|r| {
                let shaped = fonts::shape(r.face, &text[r.range.clone()]);
                (r, shaped)
            })
            .collect();
        Self {
            text: text.to_string(),
            bold,
            italic,
            runs,
        }
    }

    /// Printed width in points at `size`.
    pub fn width_pt(&self, size: f32) -> f32 {
        self.runs
            .iter()
            .map(|(r, s)| {
                s.advance as f32 * size * r.face.size_factor() / r.face.units_per_em() as f32
            })
            .sum()
    }
}

#[derive(Debug, Clone)]
struct PackedFont {
    face: FontFace,
    /// Resource name, e.g. `E0`.
    resource: Vec<u8>,
    /// Old glyph id -> glyph id in the embedded font (CID). Identity when the
    /// whole font is embedded because subsetting failed.
    remap: BTreeMap<u16, u16>,
    /// Embedded font file (subset or whole).
    file: Vec<u8>,
    /// CID -> text, from the first cluster each glyph starts.
    to_unicode: BTreeMap<u16, String>,
}

impl PackedFont {
    /// Advance of an original glyph in PDF glyph space (1/1000 em), rounded as written in `W`.
    fn width(&self, old: u16) -> f32 {
        let face = self.face.face();
        let adv = face
            .glyph_hor_advance(rustybuzz::ttf_parser::GlyphId(old))
            .unwrap_or(0) as f32;
        (adv * 1000.0 / self.face.units_per_em() as f32).round()
    }
}

/// The fonts one PDF embeds, with their glyph subsets.
#[derive(Debug, Clone, Default)]
pub struct FontPack {
    fonts: Vec<PackedFont>,
}

fn fnv(bytes: impl IntoIterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Six capital letters naming a subset, derived from the font and its glyphs.
fn subset_tag(face: FontFace, glyphs: &BTreeSet<u16>) -> String {
    let mut h = fnv(face
        .postscript_name()
        .bytes()
        .chain(glyphs.iter().flat_map(|g| g.to_be_bytes())));
    (0..6)
        .map(|_| {
            let c = (b'A' + (h % 26) as u8) as char;
            h /= 26;
            c
        })
        .collect()
}

impl FontPack {
    /// Collect the glyphs every line needs and subset each font once.
    pub fn build(lines: &[&ShapedText]) -> Self {
        let mut used: BTreeMap<FontFace, BTreeSet<u16>> = BTreeMap::new();
        for line in lines {
            for (r, s) in &line.runs {
                let set = used.entry(r.face).or_default();
                set.extend(s.glyphs.iter().map(|g| g.id));
            }
        }
        let mut fonts = Vec::new();
        for (i, (face, glyphs)) in used.into_iter().enumerate() {
            let list: Vec<u16> = glyphs.iter().copied().collect();
            let mapper = subsetter::GlyphRemapper::new_from_glyphs_sorted(&list);
            let (remap, file) = match subsetter::subset(face.data(), 0, &mapper) {
                Ok(file) => (
                    list.iter()
                        .map(|g| (*g, mapper.get(*g).unwrap_or(0)))
                        .collect(),
                    file,
                ),
                // Never expected for the bundled fonts (tests subset each of them).
                // Embedding the whole file keeps the PDF correct, only larger.
                Err(_) => (
                    list.iter().map(|g| (*g, *g)).collect(),
                    face.data().to_vec(),
                ),
            };
            fonts.push(PackedFont {
                face,
                resource: format!("E{i}").into_bytes(),
                remap,
                file,
                to_unicode: BTreeMap::new(),
            });
        }
        let mut pack = FontPack { fonts };
        // Glyph -> text for the ToUnicode maps, in document order (first use wins).
        for line in lines {
            for (r, s) in &line.runs {
                let text = &line.text[r.range.clone()];
                let Some(font) = pack.fonts.iter_mut().find(|f| f.face == r.face) else {
                    continue;
                };
                let mut k = 0;
                while k < s.glyphs.len() {
                    let cluster = s.glyphs[k].cluster as usize;
                    let mut end_k = k + 1;
                    while end_k < s.glyphs.len() && s.glyphs[end_k].cluster as usize == cluster {
                        end_k += 1;
                    }
                    let next_cluster = s.glyphs[end_k..]
                        .iter()
                        .map(|g| g.cluster as usize)
                        .find(|c| *c > cluster)
                        .unwrap_or(text.len());
                    if let Some(cluster_text) = text.get(cluster..next_cluster.max(cluster))
                        && let Some(cid) = font.remap.get(&s.glyphs[k].id).copied()
                        && cid != 0
                        && !cluster_text.is_empty()
                    {
                        font.to_unicode
                            .entry(cid)
                            .or_insert_with(|| cluster_text.to_string());
                    }
                    k = end_k;
                }
            }
        }
        pack
    }

    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }

    /// Draw a shaped line with its left edge at (`x`, `y`) points, inside an
    /// `ActualText` span carrying the exact line text. Returns the width drawn.
    pub fn draw(&self, c: &mut Content, line: &ShapedText, x: f32, y: f32, size: f32) -> f32 {
        c.begin_marked_content_with_properties(Name(b"Span"))
            .properties()
            .actual_text(TextStr(&line.text));
        let mut pen = x;
        for (r, s) in &line.runs {
            let Some(font) = self.fonts.iter().find(|f| f.face == r.face) else {
                continue;
            };
            // Noto faces print slightly smaller than the page size (FontFace::size_factor).
            let size = size * r.face.size_factor();
            let scale = size / r.face.units_per_em() as f32;
            let synthetic = !r.face.is_courier();
            let slant = if synthetic && line.italic {
                SYNTHETIC_SLANT
            } else {
                0.0
            };
            let fake_bold = synthetic && line.bold;
            if fake_bold {
                c.save_state();
                c.set_line_width(size * SYNTHETIC_BOLD);
            }
            c.begin_text();
            c.set_font(Name(&font.resource), size);
            c.set_text_rendering_mode(if fake_bold {
                TextRenderingMode::FillStroke
            } else {
                TextRenderingMode::Fill
            });
            // Glyphs are written in TJ arrays; a vertical offset (a mark above or
            // below its base) starts a new text matrix.
            let mut cursor: Option<(f32, f32)> = None;
            let mut items: Vec<Item> = Vec::new();
            for g in &s.glyphs {
                let gx = pen + g.x_offset as f32 * scale;
                let gy = y + g.y_offset as f32 * scale;
                let cid = font.remap.get(&g.id).copied().unwrap_or(0);
                match cursor {
                    Some((cx, cy)) if (cy - gy).abs() < 1e-4 => {
                        let d = gx - cx;
                        if d.abs() > 1e-3 {
                            items.push(Item::Adjust(-d * 1000.0 / size));
                        }
                    }
                    _ => {
                        flush(c, &mut items);
                        c.set_text_matrix([1.0, 0.0, slant, 1.0, gx, gy]);
                    }
                }
                items.push(Item::Glyph(cid));
                cursor = Some((gx + font.width(g.id) * size / 1000.0, gy));
                pen += g.x_advance as f32 * scale;
            }
            flush(c, &mut items);
            c.end_text();
            if fake_bold {
                c.restore_state();
            }
        }
        c.end_marked_content();
        pen - x
    }

    /// Resource name and object of every embedded font, for page resources.
    pub fn resources(&self, first_ref: i32) -> Vec<(Vec<u8>, Ref)> {
        self.fonts
            .iter()
            .enumerate()
            .map(|(i, f)| (f.resource.clone(), Ref::new(first_ref + 5 * i as i32)))
            .collect()
    }

    /// Objects this pack writes (five per font).
    pub fn object_count(&self) -> i32 {
        5 * self.fonts.len() as i32
    }

    /// Write every font (Type 0, CIDFont, descriptor, font file, ToUnicode)
    /// starting at object `first_ref`.
    pub fn write(&self, pdf: &mut Pdf, first_ref: i32) {
        for (i, f) in self.fonts.iter().enumerate() {
            let base = first_ref + 5 * i as i32;
            let (type0, cid_ref, desc_ref, file_ref, cmap_ref) = (
                Ref::new(base),
                Ref::new(base + 1),
                Ref::new(base + 2),
                Ref::new(base + 3),
                Ref::new(base + 4),
            );
            let glyphs: BTreeSet<u16> = f.remap.keys().copied().collect();
            let name = format!(
                "{}+{}",
                subset_tag(f.face, &glyphs),
                f.face.postscript_name()
            );
            let system_info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };
            pdf.type0_font(type0)
                .base_font(Name(name.as_bytes()))
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid_ref)
                .to_unicode(cmap_ref);

            let mut cid = pdf.cid_font(cid_ref);
            cid.subtype(CidFontType::Type2)
                .base_font(Name(name.as_bytes()))
                .system_info(system_info)
                .font_descriptor(desc_ref)
                .default_width(0.0)
                .cid_to_gid_map_predefined(Name(b"Identity"));
            {
                let mut w = cid.widths();
                let mut by_cid: Vec<(u16, f32)> = f
                    .remap
                    .iter()
                    .map(|(old, new)| (*new, f.width(*old)))
                    .collect();
                by_cid.sort_by_key(|(c, _)| *c);
                let mut k = 0;
                while k < by_cid.len() {
                    let start = by_cid[k].0;
                    let mut run = vec![by_cid[k].1];
                    let mut j = k + 1;
                    while j < by_cid.len() && by_cid[j].0 == by_cid[j - 1].0 + 1 {
                        run.push(by_cid[j].1);
                        j += 1;
                    }
                    w.consecutive(start, run);
                    k = j;
                }
            }
            cid.finish();

            let face = f.face.face();
            let upem = f.face.units_per_em() as f32;
            let to_pdf = |v: f32| (v * 1000.0 / upem).round();
            let bbox = face.global_bounding_box();
            let mut flags = FontFlags::SYMBOLIC;
            if f.face.is_courier() {
                flags |= FontFlags::FIXED_PITCH | FontFlags::SERIF;
            }
            if f.face == FontFace::CourierPrimeItalic {
                flags |= FontFlags::ITALIC;
            }
            let cap = face
                .capital_height()
                .map(|h| h as f32)
                .unwrap_or(face.ascender() as f32);
            let weight = face.weight().to_number() as f32;
            pdf.font_descriptor(desc_ref)
                .name(Name(name.as_bytes()))
                .flags(flags)
                .bbox(Rect::new(
                    to_pdf(bbox.x_min as f32),
                    to_pdf(bbox.y_min as f32),
                    to_pdf(bbox.x_max as f32),
                    to_pdf(bbox.y_max as f32),
                ))
                .italic_angle(face.italic_angle())
                .ascent(to_pdf(face.ascender() as f32))
                .descent(to_pdf(face.descender() as f32))
                .cap_height(to_pdf(cap))
                .stem_v((10.0 + 0.244 * (weight - 50.0)).round())
                .font_file2(file_ref);

            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            let (compressed, deflated) = match z.write_all(&f.file).and_then(|_| z.finish()) {
                Ok(bytes) => (bytes, true),
                Err(_) => (f.file.clone(), false),
            };
            let mut stream = pdf.stream(file_ref, &compressed);
            if deflated {
                stream.filter(Filter::FlateDecode);
            }
            stream.pair(Name(b"Length1"), f.file.len() as i32);
            stream.finish();

            let mut cmap = UnicodeCmap::new(Name(b"Custom"), system_info);
            for (cid, text) in &f.to_unicode {
                cmap.pair_with_multiple(*cid, text.chars());
            }
            let cmap_bytes = cmap.finish();
            pdf.cmap(cmap_ref, &cmap_bytes);
        }
    }
}

enum Item {
    Glyph(u16),
    Adjust(f32),
}

fn flush(c: &mut Content, items: &mut Vec<Item>) {
    if items.is_empty() {
        return;
    }
    let mut op = c.show_positioned();
    let mut arr = op.items();
    let mut glyphs: Vec<u8> = Vec::new();
    for item in items.drain(..) {
        match item {
            Item::Glyph(cid) => glyphs.extend_from_slice(&cid.to_be_bytes()),
            Item::Adjust(a) => {
                if !glyphs.is_empty() {
                    arr.show(Str(&glyphs));
                    glyphs.clear();
                }
                arr.adjust(a);
            }
        }
    }
    if !glyphs.is_empty() {
        arr.show(Str(&glyphs));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_font_subsets() {
        for face in fonts::ALL_FACES {
            let mapper = subsetter::GlyphRemapper::new_from_glyphs_sorted(&[1, 2, 3]);
            let out = subsetter::subset(face.data(), 0, &mapper).expect("subset");
            assert!(!out.is_empty() && out.len() < face.data().len(), "{face:?}");
        }
    }

    #[test]
    fn subset_tags_are_stable_and_depend_on_the_glyphs() {
        let a: BTreeSet<u16> = [1, 2, 3].into_iter().collect();
        let b: BTreeSet<u16> = [1, 2, 4].into_iter().collect();
        let t = subset_tag(FontFace::NotoSansTelugu, &a);
        assert_eq!(t.len(), 6);
        assert!(t.chars().all(|c| c.is_ascii_uppercase()));
        assert_eq!(t, subset_tag(FontFace::NotoSansTelugu, &a));
        assert_ne!(t, subset_tag(FontFace::NotoSansTelugu, &b));
    }

    #[test]
    fn to_unicode_maps_the_first_glyph_of_each_cluster_to_its_text() {
        let line = ShapedText::new("కి A", false, false);
        let pack = FontPack::build(&[&line]);
        let telugu = pack
            .fonts
            .iter()
            .find(|f| f.face == FontFace::NotoSansTelugu)
            .unwrap();
        let texts: Vec<&str> = telugu.to_unicode.values().map(|s| s.as_str()).collect();
        assert!(texts.contains(&"కి"), "{texts:?}");
    }
}
