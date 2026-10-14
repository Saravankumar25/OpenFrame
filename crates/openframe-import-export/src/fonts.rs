//! Bundled screenplay fonts, font selection and complex-script shaping (ADR-0016).
//!
//! The screenplay PDF prints Latin text in Courier Prime (a Courier-metric face,
//! 0.6 em advance like base-14 Courier) and Indian scripts in the Noto Sans
//! families for Devanagari (Hindi, Marathi, Sanskrit), Telugu, Tamil, Malayalam
//! and Kannada. All eight files ship inside the app under the SIL Open Font
//! Licence (`crates/openframe-import-export/fonts/`, listed in `fonts.json` with
//! their SHA-256), so a PDF never depends on what is installed on the computer.
//!
//! ```text
//!  line text ─► grapheme clusters ─► font per cluster (script, coverage, fallback)
//!            ─► runs of one font ─► rustybuzz shaping ─► glyph ids, advances, offsets
//! ```
//!
//! Layout measures text with the same shaped advances the PDF renderer uses
//! ([`columns`]), so page breaks agree with what is printed. Text made only of
//! Windows-1252 characters takes a fast path that counts characters, which is
//! exactly the Courier measure the layout used before embedded fonts existed.
//!
//! Everything here is deterministic: a fixed font set, no system font lookup,
//! no clock, no randomness.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::OnceLock;

use unicode_segmentation::UnicodeSegmentation;

/// One bundled font file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontFace {
    CourierPrime,
    CourierPrimeBold,
    CourierPrimeItalic,
    NotoSansDevanagari,
    NotoSansTelugu,
    NotoSansTamil,
    NotoSansMalayalam,
    NotoSansKannada,
}

/// Print size of the Noto faces relative to the page font size (see [`FontFace::size_factor`]).
pub const INDIC_SIZE_FACTOR: f32 = 0.9;

/// Every bundled face, in a fixed order (fallback order after the script choice).
pub const ALL_FACES: [FontFace; 8] = [
    FontFace::CourierPrime,
    FontFace::CourierPrimeBold,
    FontFace::CourierPrimeItalic,
    FontFace::NotoSansDevanagari,
    FontFace::NotoSansTelugu,
    FontFace::NotoSansTamil,
    FontFace::NotoSansMalayalam,
    FontFace::NotoSansKannada,
];

impl FontFace {
    /// Position in [`ALL_FACES`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// File name under `fonts/`.
    pub fn file_name(self) -> &'static str {
        match self {
            FontFace::CourierPrime => "CourierPrime-Regular.ttf",
            FontFace::CourierPrimeBold => "CourierPrime-Bold.ttf",
            FontFace::CourierPrimeItalic => "CourierPrime-Italic.ttf",
            FontFace::NotoSansDevanagari => "NotoSansDevanagari-Regular.ttf",
            FontFace::NotoSansTelugu => "NotoSansTelugu-Regular.ttf",
            FontFace::NotoSansTamil => "NotoSansTamil-Regular.ttf",
            FontFace::NotoSansMalayalam => "NotoSansMalayalam-Regular.ttf",
            FontFace::NotoSansKannada => "NotoSansKannada-Regular.ttf",
        }
    }

    /// PostScript name written into the PDF (after the subset tag).
    pub fn postscript_name(self) -> &'static str {
        match self {
            FontFace::CourierPrime => "CourierPrime",
            FontFace::CourierPrimeBold => "CourierPrime-Bold",
            FontFace::CourierPrimeItalic => "CourierPrime-Italic",
            FontFace::NotoSansDevanagari => "NotoSansDevanagari-Regular",
            FontFace::NotoSansTelugu => "NotoSansTelugu-Regular",
            FontFace::NotoSansTamil => "NotoSansTamil-Regular",
            FontFace::NotoSansMalayalam => "NotoSansMalayalam-Regular",
            FontFace::NotoSansKannada => "NotoSansKannada-Regular",
        }
    }

    /// The raw font file.
    pub fn data(self) -> &'static [u8] {
        match self {
            FontFace::CourierPrime => include_bytes!("../fonts/CourierPrime-Regular.ttf"),
            FontFace::CourierPrimeBold => include_bytes!("../fonts/CourierPrime-Bold.ttf"),
            FontFace::CourierPrimeItalic => include_bytes!("../fonts/CourierPrime-Italic.ttf"),
            FontFace::NotoSansDevanagari => {
                include_bytes!("../fonts/NotoSansDevanagari-Regular.ttf")
            }
            FontFace::NotoSansTelugu => include_bytes!("../fonts/NotoSansTelugu-Regular.ttf"),
            FontFace::NotoSansTamil => include_bytes!("../fonts/NotoSansTamil-Regular.ttf"),
            FontFace::NotoSansMalayalam => include_bytes!("../fonts/NotoSansMalayalam-Regular.ttf"),
            FontFace::NotoSansKannada => include_bytes!("../fonts/NotoSansKannada-Regular.ttf"),
        }
    }

    /// A Courier Prime face (monospaced Latin).
    pub fn is_courier(self) -> bool {
        matches!(
            self,
            FontFace::CourierPrime | FontFace::CourierPrimeBold | FontFace::CourierPrimeItalic
        )
    }

    /// The font face, parsed once per process.
    pub fn face(self) -> &'static rustybuzz::Face<'static> {
        &faces()[self.index()]
    }

    pub fn units_per_em(self) -> u16 {
        self.face().units_per_em() as u16
    }

    /// Size of this face relative to the page font size. Indian scripts stack marks above and
    /// below the letters; at the full Courier size those stacks touch the line above or below
    /// on the fixed screenplay line grid, so the Noto faces print slightly smaller. Layout and
    /// drawing both apply it, so widths and page breaks agree with the printed page.
    pub fn size_factor(self) -> f32 {
        if self.is_courier() {
            1.0
        } else {
            INDIC_SIZE_FACTOR
        }
    }

    /// Does the font map this character to a glyph?
    pub fn covers(self, c: char) -> bool {
        self.face().glyph_index(c).is_some()
    }
}

fn faces() -> &'static Vec<rustybuzz::Face<'static>> {
    static FACES: OnceLock<Vec<rustybuzz::Face<'static>>> = OnceLock::new();
    FACES.get_or_init(|| {
        ALL_FACES
            .iter()
            .map(|f| {
                // The bundled files are checked by tests (fonts.json hashes), so a parse
                // failure here is a build defect, not user input.
                rustybuzz::Face::from_slice(f.data(), 0).expect("bundled font parses")
            })
            .collect()
    })
}

/// Latin face for a line style: bold for the title, italic for lyrics and notes.
pub fn latin_face(bold: bool, italic: bool) -> FontFace {
    if bold {
        FontFace::CourierPrimeBold
    } else if italic {
        FontFace::CourierPrimeItalic
    } else {
        FontFace::CourierPrime
    }
}

/// Indian scripts with a bundled font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicScript {
    Devanagari,
    Telugu,
    Tamil,
    Malayalam,
    Kannada,
}

impl IndicScript {
    pub fn face(self) -> FontFace {
        match self {
            IndicScript::Devanagari => FontFace::NotoSansDevanagari,
            IndicScript::Telugu => FontFace::NotoSansTelugu,
            IndicScript::Tamil => FontFace::NotoSansTamil,
            IndicScript::Malayalam => FontFace::NotoSansMalayalam,
            IndicScript::Kannada => FontFace::NotoSansKannada,
        }
    }
    /// BCP 47 language most often written in the script (used as a DOCX hint).
    pub fn language_tag(self) -> &'static str {
        match self {
            IndicScript::Devanagari => "hi-IN",
            IndicScript::Telugu => "te-IN",
            IndicScript::Tamil => "ta-IN",
            IndicScript::Malayalam => "ml-IN",
            IndicScript::Kannada => "kn-IN",
        }
    }
}

/// The Indian script a character belongs to. The dandas (U+0964, U+0965) are
/// shared punctuation and count as common.
pub fn indic_script(c: char) -> Option<IndicScript> {
    let cp = c as u32;
    Some(match cp {
        0x0964 | 0x0965 => return None,
        0x0900..=0x097F | 0xA8E0..=0xA8FF | 0x11B00..=0x11B5F => IndicScript::Devanagari,
        0x0B80..=0x0BFF | 0x11FC0..=0x11FFF => IndicScript::Tamil,
        0x0C00..=0x0C7F => IndicScript::Telugu,
        0x0C80..=0x0CFF => IndicScript::Kannada,
        0x0D00..=0x0D7F => IndicScript::Malayalam,
        _ => return None,
    })
}

/// The first Indian script found in `text`, if any.
pub fn first_indic_script(text: &str) -> Option<IndicScript> {
    text.chars().find_map(indic_script)
}

/// Unicode default-ignorable characters (zero-width joiner and non-joiner,
/// zero-width space, soft hyphen, variation selectors, bidi controls, BOM).
/// They are kept in the text, shaped where they matter (ZWJ, ZWNJ) and never
/// need a glyph of their own.
pub fn is_ignorable(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180B..=0x180F
        | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0x3164 | 0xFE00..=0xFE0F
        | 0xFEFF | 0xFFA0 | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0000..=0xE0FFF)
}

/// Characters the PDF never prints (line breaks and other controls are handled by layout).
fn is_invisible(c: char) -> bool {
    c.is_control() || is_ignorable(c)
}

/// Kind of a grapheme cluster for font choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClusterClass {
    Indic(IndicScript),
    /// Letters of any other script (Latin, Greek, ...).
    Letter,
    /// Spaces, digits, punctuation, symbols and ignorables: they follow their neighbours.
    Common,
}

fn classify(cluster: &str) -> ClusterClass {
    if let Some(s) = cluster.chars().find_map(indic_script) {
        return ClusterClass::Indic(s);
    }
    if cluster.chars().any(|c| c.is_alphabetic()) {
        ClusterClass::Letter
    } else {
        ClusterClass::Common
    }
}

fn face_covers(face: FontFace, cluster: &str) -> bool {
    cluster.chars().all(|c| is_invisible(c) || face.covers(c))
}

fn note_missing(cluster: &str, missing: &mut Vec<char>) {
    for c in cluster.chars() {
        if !is_invisible(c) && !ALL_FACES.iter().any(|f| f.covers(c)) && !missing.contains(&c) {
            missing.push(c);
        }
    }
}

/// A stretch of text printed with one font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontRun {
    pub face: FontFace,
    /// Byte range in the itemized text.
    pub range: Range<usize>,
}

/// Split `text` into runs of one font. `latin` is the Courier Prime face for the
/// line style. Returns the characters no bundled font can print as the error
/// (each listed once, in order of first appearance).
pub fn itemize(text: &str, latin: FontFace) -> Result<Vec<FontRun>, Vec<char>> {
    let clusters: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    let mut chosen: Vec<Option<FontFace>> = Vec::with_capacity(clusters.len());
    let mut missing: Vec<char> = Vec::new();
    // Pass 1: clusters with a script of their own.
    for (_, cl) in &clusters {
        let preferred = match classify(cl) {
            ClusterClass::Indic(s) => Some(s.face()),
            ClusterClass::Letter => Some(latin),
            ClusterClass::Common => None,
        };
        chosen.push(match preferred {
            Some(face) if face_covers(face, cl) => Some(face),
            Some(_) => {
                let fallback = ALL_FACES.iter().copied().find(|f| face_covers(*f, cl));
                if fallback.is_none() {
                    note_missing(cl, &mut missing);
                }
                Some(fallback.unwrap_or(latin))
            }
            None => None,
        });
    }
    // Pass 2: common clusters join the run before them, else the run after them.
    let mut prev: Option<FontFace> = None;
    for i in 0..clusters.len() {
        if let Some(f) = chosen[i] {
            prev = Some(f);
            continue;
        }
        let cl = clusters[i].1;
        let next = chosen[i + 1..].iter().flatten().next().copied();
        let candidates = [prev, next, Some(latin)];
        let face = candidates
            .into_iter()
            .flatten()
            .chain(ALL_FACES.iter().copied())
            .find(|f| face_covers(*f, cl));
        if face.is_none() {
            note_missing(cl, &mut missing);
        }
        let face = face.unwrap_or(latin);
        chosen[i] = Some(face);
        prev = Some(face);
    }
    if !missing.is_empty() {
        return Err(missing);
    }
    let mut runs: Vec<FontRun> = Vec::new();
    for (i, (start, cl)) in clusters.iter().enumerate() {
        let face = chosen[i].unwrap_or(latin);
        let end = start + cl.len();
        match runs.last_mut() {
            Some(r) if r.face == face => r.range.end = end,
            _ => runs.push(FontRun {
                face,
                range: *start..end,
            }),
        }
    }
    Ok(runs)
}

/// Characters in `text` that no bundled font can print, in order of first appearance.
pub fn unprintable_chars(text: &str) -> Vec<char> {
    let mut out = Vec::new();
    for c in text.chars() {
        if !is_invisible(c) && !out.contains(&c) && !ALL_FACES.iter().any(|f| f.covers(c)) {
            out.push(c);
        }
    }
    out
}

/// One positioned glyph, in font units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedGlyph {
    /// Glyph id in the original (not subset) font.
    pub id: u16,
    /// Byte offset of the glyph's cluster in the shaped text.
    pub cluster: u32,
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

/// The result of shaping one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapedRun {
    pub face: FontFace,
    pub glyphs: Vec<ShapedGlyph>,
    /// Total advance in font units.
    pub advance: i32,
}

/// Shape `text` (one run, one font) left to right with the font's default features.
pub fn shape(face: FontFace, text: &str) -> ShapedRun {
    let mut buf = rustybuzz::UnicodeBuffer::new();
    buf.push_str(text);
    buf.guess_segment_properties();
    buf.set_direction(rustybuzz::Direction::LeftToRight);
    let out = rustybuzz::shape(face.face(), &[], buf);
    let glyphs: Vec<ShapedGlyph> = out
        .glyph_infos()
        .iter()
        .zip(out.glyph_positions())
        .map(|(i, p)| ShapedGlyph {
            id: u16::try_from(i.glyph_id).unwrap_or(0),
            cluster: i.cluster,
            x_advance: p.x_advance,
            x_offset: p.x_offset,
            y_offset: p.y_offset,
        })
        .collect();
    let advance = glyphs.iter().map(|g| g.x_advance).sum();
    ShapedRun {
        face,
        glyphs,
        advance,
    }
}

/// Text that the layout can measure by counting characters: every character is
/// in Windows-1252, which Courier and Courier Prime print at 0.6 em.
fn is_fast_path(text: &str) -> bool {
    text.chars()
        .all(|c| c == '\t' || crate::winansi::encode_char(c).is_some())
}

const COURIER_EM: f32 = 0.6;
const CACHE_LIMIT: usize = 20_000;

thread_local! {
    static WIDTH_CACHE: RefCell<HashMap<String, f32>> = RefCell::new(HashMap::new());
}

fn shaped_columns(text: &str) -> f32 {
    let latin = FontFace::CourierPrime;
    let runs = match itemize(text, latin) {
        Ok(r) => r,
        // Unprintable characters block the PDF anyway; measure them as one column each.
        Err(_) => {
            return text
                .graphemes(true)
                .filter(|g| !g.chars().all(is_invisible))
                .count() as f32;
        }
    };
    let mut em = 0.0f32;
    for r in runs {
        let shaped = shape(r.face, &text[r.range.clone()]);
        em += shaped.advance as f32 / r.face.units_per_em() as f32 * r.face.size_factor();
    }
    em / COURIER_EM
}

/// Width of `text` in Courier columns (one column = 0.6 em at the page font size).
/// Windows-1252 text is counted character by character, exactly as before
/// embedded fonts; anything else is shaped with the bundled fonts.
pub fn columns(text: &str) -> f32 {
    if is_fast_path(text) {
        return text.chars().count() as f32;
    }
    if let Some(w) = WIDTH_CACHE.with(|c| c.borrow().get(text).copied()) {
        return w;
    }
    let w = shaped_columns(text);
    WIDTH_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= CACHE_LIMIT {
            c.clear();
        }
        c.insert(text.to_string(), w);
    });
    w
}

/// Does printing this text need the embedded fonts (anything outside Windows-1252)?
pub fn needs_embedding(text: &str) -> bool {
    text.chars().any(|c| {
        !c.is_control()
            && c != '\u{00A0}'
            && crate::winansi::encode_char(c).is_none()
            && !matches!(
                c,
                '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' | '\u{2032}' | '\u{2033}'
            )
    })
}

/// Split a word into pieces of at most `width` columns at grapheme boundaries,
/// so combining marks and conjuncts are never cut apart.
pub fn break_word(word: &str, width: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut piece = String::new();
    let mut used = 0.0f32;
    for g in word.graphemes(true) {
        let w = columns(g);
        if !piece.is_empty() && used + w > width + 1e-3 {
            out.push(std::mem::take(&mut piece));
            used = 0.0;
        }
        piece.push_str(g);
        used += w;
    }
    if !piece.is_empty() {
        out.push(piece);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_face_parses_and_courier_prime_is_courier_metric() {
        for f in ALL_FACES {
            assert!(f.units_per_em() > 0, "{f:?}");
        }
        for f in [
            FontFace::CourierPrime,
            FontFace::CourierPrimeBold,
            FontFace::CourierPrimeItalic,
        ] {
            let s = shape(f, "MW il.");
            let em = s.advance as f32 / f.units_per_em() as f32;
            assert!((em / 6.0 - 0.6).abs() < 0.001, "{f:?} advance {em}");
        }
    }

    #[test]
    fn each_indian_script_uses_its_own_font() {
        let cases = [
            ("नमस्ते", FontFace::NotoSansDevanagari),
            ("నమస్కారం", FontFace::NotoSansTelugu),
            ("வணக்கம்", FontFace::NotoSansTamil),
            ("നമസ്കാരം", FontFace::NotoSansMalayalam),
            ("ನಮಸ್ಕಾರ", FontFace::NotoSansKannada),
        ];
        for (text, face) in cases {
            let runs = itemize(text, FontFace::CourierPrime).unwrap();
            assert_eq!(runs.len(), 1, "{text}");
            assert_eq!(runs[0].face, face, "{text}");
            assert_eq!(runs[0].range, 0..text.len());
        }
    }

    #[test]
    fn mixed_lines_split_into_font_runs_and_spaces_follow_their_neighbours() {
        let text = "RAVI (V.O.) నమస్కారం, Meera!";
        let runs = itemize(text, FontFace::CourierPrime).unwrap();
        let faces: Vec<FontFace> = runs.iter().map(|r| r.face).collect();
        assert_eq!(
            faces,
            vec![
                FontFace::CourierPrime,
                FontFace::NotoSansTelugu,
                FontFace::CourierPrime
            ]
        );
        // Runs cover the text exactly, in order.
        let joined: String = runs.iter().map(|r| &text[r.range.clone()]).collect();
        assert_eq!(joined, text);
    }

    #[test]
    fn shaping_forms_conjuncts_and_reorders_vowel_signs() {
        // Devanagari "कि": the i-matra is drawn before the consonant.
        let s = shape(FontFace::NotoSansDevanagari, "कि");
        assert_eq!(s.glyphs.len(), 2);
        let ka = FontFace::NotoSansDevanagari
            .face()
            .glyph_index('क')
            .unwrap()
            .0;
        assert_eq!(
            s.glyphs[1].id, ka,
            "consonant comes second after reordering"
        );
        // Telugu "క్ష" (ka + virama + ssa) shapes into fewer glyphs than characters.
        let t = shape(FontFace::NotoSansTelugu, "క్ష");
        assert!(t.glyphs.len() < 3, "{:?}", t.glyphs);
        // A zero-width non-joiner keeps the half form apart: different shaping.
        let joined = shape(FontFace::NotoSansDevanagari, "क्ष");
        let split = shape(FontFace::NotoSansDevanagari, "क्\u{200C}ष");
        assert_ne!(joined.glyphs, split.glyphs);
    }

    #[test]
    fn emoji_and_cjk_are_reported_as_unprintable() {
        let err = itemize("Hello 東京 🎬", FontFace::CourierPrime).unwrap_err();
        assert_eq!(err, vec!['東', '京', '🎬']);
        assert!(unprintable_chars("తెలుగు हिंदी Tamil தமிழ்").is_empty());
        // Joiners and the zero-width space never need a glyph.
        assert!(unprintable_chars("a\u{200D}b\u{200C}c\u{200B}d\u{FEFF}").is_empty());
    }

    #[test]
    fn latin_text_measures_one_column_per_character() {
        assert_eq!(columns("INT. HOUSE - DAY"), 16.0);
        assert_eq!(columns("Café déjà vu"), 12.0);
        // Latin outside Windows-1252 goes through Courier Prime: still about one column each.
        let w = columns("Ārya Śiva");
        assert!((w - 9.0).abs() < 0.05, "{w}");
    }

    #[test]
    fn indian_text_is_measured_by_its_shaped_width() {
        let w = columns("నమస్కారం");
        assert!(w > 2.0 && w < 12.0, "{w}");
        // Deterministic and cached.
        assert_eq!(columns("నమస్కారం"), w);
    }

    #[test]
    fn long_words_break_between_graphemes() {
        let word = "క్షత్రియ".repeat(12);
        let pieces = break_word(&word, 10.0);
        assert!(pieces.len() > 1);
        assert_eq!(pieces.concat(), word);
        for p in &pieces {
            assert!(columns(p) <= 10.0 + 1e-3, "{p}");
            // No piece starts with a combining mark or a virama.
            let first = p.chars().next().unwrap();
            assert!(!matches!(first as u32, 0x0C3E..=0x0C56), "{p}");
        }
    }

    #[test]
    fn embedding_is_needed_only_outside_windows_1252() {
        assert!(!needs_embedding("Plain “quoted” text… café"));
        assert!(needs_embedding("తెలుగు"));
        assert!(needs_embedding("Ārya"));
        assert!(needs_embedding("a\u{200D}b"));
    }
}
