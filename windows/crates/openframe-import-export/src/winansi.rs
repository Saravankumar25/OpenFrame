//! WinAnsi (Windows-1252) encoding for the PDF base-14 fonts, plus Helvetica
//! glyph widths for report layout.
//!
//! OpenFrame's PDFs use the standard base-14 fonts (Courier, Helvetica) so no
//! font files are embedded. Those fonts only cover the WinAnsi character set.
//! **Limitation:** text outside WinAnsi (e.g. Devanagari, Tamil, CJK, Arabic)
//! cannot be drawn with them; such characters are replaced with `?` and the
//! export reports a warning. Use DOCX, FDX or Fountain export for scripts in
//! those writing systems.

const HIGH: [(u8, char); 27] = [
    (0x80, '\u{20AC}'),
    (0x82, '\u{201A}'),
    (0x83, '\u{0192}'),
    (0x84, '\u{201E}'),
    (0x85, '\u{2026}'),
    (0x86, '\u{2020}'),
    (0x87, '\u{2021}'),
    (0x88, '\u{02C6}'),
    (0x89, '\u{2030}'),
    (0x8A, '\u{0160}'),
    (0x8B, '\u{2039}'),
    (0x8C, '\u{0152}'),
    (0x8E, '\u{017D}'),
    (0x91, '\u{2018}'),
    (0x92, '\u{2019}'),
    (0x93, '\u{201C}'),
    (0x94, '\u{201D}'),
    (0x95, '\u{2022}'),
    (0x96, '\u{2013}'),
    (0x97, '\u{2014}'),
    (0x98, '\u{02DC}'),
    (0x99, '\u{2122}'),
    (0x9A, '\u{0161}'),
    (0x9B, '\u{203A}'),
    (0x9C, '\u{0153}'),
    (0x9E, '\u{017E}'),
    (0x9F, '\u{0178}'),
];

/// Encode one character, or None when WinAnsi cannot represent it.
pub fn encode_char(c: char) -> Option<u8> {
    let cp = c as u32;
    match cp {
        0x20..=0x7E => Some(cp as u8),
        0xA0..=0xFF => Some(cp as u8),
        _ => HIGH.iter().find(|(_, ch)| *ch == c).map(|(b, _)| *b),
    }
}

/// Decode one Windows-1252 byte (undefined bytes become U+FFFD).
pub fn decode_byte(b: u8) -> char {
    match b {
        0x80..=0x9F => HIGH
            .iter()
            .find(|(x, _)| *x == b)
            .map(|(_, c)| *c)
            .unwrap_or('\u{FFFD}'),
        _ => b as char,
    }
}

/// Encode a string for a base-14 font. Unsupported characters become `?`;
/// returns the bytes and how many characters were replaced.
pub fn encode(text: &str) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(text.len());
    let mut replaced = 0;
    for c in text.chars() {
        match c {
            '\t' => out.push(b' '),
            '\u{00A0}' => out.push(b' '),
            c if c.is_control() => {}
            c => match encode_char(c) {
                Some(b) => out.push(b),
                None => {
                    // Common typographic substitutes before giving up.
                    let sub = match c {
                        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2212}' => Some(b'-'),
                        '\u{2032}' => Some(b'\''),
                        '\u{2033}' => Some(b'"'),
                        _ => None,
                    };
                    match sub {
                        Some(b) => out.push(b),
                        None => {
                            out.push(b'?');
                            replaced += 1;
                        }
                    }
                }
            },
        }
    }
    (out, replaced)
}

// ------------------------------------------------------------ Helvetica widths

const HELV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

const HELV_BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

fn base_letter(c: char) -> char {
    match c {
        'À'..='Å' => 'A',
        'Ç' => 'C',
        'È'..='Ë' => 'E',
        'Ì'..='Ï' => 'I',
        'Ñ' => 'N',
        'Ò'..='Ö' | 'Ø' => 'O',
        'Ù'..='Ü' => 'U',
        'Ý' | 'Ÿ' => 'Y',
        'à'..='å' => 'a',
        'ç' => 'c',
        'è'..='ë' => 'e',
        'ì'..='ï' => 'i',
        'ñ' => 'n',
        'ò'..='ö' | 'ø' => 'o',
        'ù'..='ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'Š' => 'S',
        'š' => 's',
        'Ž' => 'Z',
        'ž' => 'z',
        _ => c,
    }
}

/// Width of one character in 1/1000 em for Helvetica / Helvetica-Bold.
pub fn helvetica_width(c: char, bold: bool) -> u16 {
    let table = if bold { &HELV_BOLD } else { &HELV };
    let c = base_letter(c);
    match c {
        ' '..='~' => table[(c as usize) - 32],
        '\u{2014}' | '\u{2026}' | '\u{2030}' | '\u{0152}' | '\u{0153}' | '\u{2122}' => 1000,
        '\u{2013}' | '\u{20AC}' => 556,
        '\u{2018}' | '\u{2019}' | '\u{201A}' => 222,
        '\u{201C}' | '\u{201D}' | '\u{201E}' => 333,
        '\u{2022}' => 350,
        'Æ' => 1000,
        'æ' => 889,
        'ß' => 611,
        _ => 556,
    }
}

/// Width of `text` in points at `size`.
pub fn text_width_pt(text: &str, size: f32, bold: bool) -> f32 {
    text.chars()
        .map(|c| helvetica_width(c, bold) as f32)
        .sum::<f32>()
        * size
        / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_latin_and_replaces_others() {
        let (b, r) = encode("Café — “Hi”");
        assert_eq!(r, 0);
        assert_eq!(
            b,
            vec![
                b'C', b'a', b'f', 0xE9, b' ', 0x97, b' ', 0x93, b'H', b'i', 0x94
            ]
        );
        let (b, r) = encode("मीरा A");
        assert_eq!(r, 4);
        assert_eq!(b, b"???? A");
    }

    #[test]
    fn decode_round_trips() {
        for c in ['€', 'é', '—', 'A', 'ÿ'] {
            assert_eq!(decode_byte(encode_char(c).unwrap()), c);
        }
    }

    #[test]
    fn widths() {
        assert_eq!(helvetica_width('A', false), 667);
        assert_eq!(helvetica_width('é', true), 556);
        assert!((text_width_pt("MM", 10.0, false) - 16.66).abs() < 0.01);
    }
}
