//! Colour arithmetic for the theme engine: the CSS `oklch()` and `color-mix(in oklab, ...)`
//! functions the webview uses, reproduced so contrast is computed in Rust from validated numbers
//! (docs/engineering/30-appearance-themes.md section 5). Pure functions, no allocation-heavy work.

/// A colour in the OKLab space (L 0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Lab {
    /// `oklch(L% C H)` with `l_pct` in percent.
    pub fn from_oklch(l_pct: f64, c: f64, h_deg: f64) -> Lab {
        let h = h_deg.to_radians();
        Lab {
            l: l_pct / 100.0,
            a: c * h.cos(),
            b: c * h.sin(),
        }
    }

    /// An sRGB colour given as 0..255 channels.
    pub fn from_srgb8(r: u8, g: u8, b: u8) -> Lab {
        let lin = [r, g, b].map(|v| to_linear(f64::from(v) / 255.0));
        linear_to_lab(lin)
    }

    /// `color-mix(in oklab, self p, other)` for opaque colours, `p` in 0..1.
    pub fn mix(self, p: f64, other: Lab) -> Lab {
        Lab {
            l: self.l * p + other.l * (1.0 - p),
            a: self.a * p + other.a * (1.0 - p),
            b: self.b * p + other.b * (1.0 - p),
        }
    }

    /// The displayed sRGB channels (0..1), clipped to the sRGB gamut per channel.
    pub fn to_srgb(self) -> [f64; 3] {
        lab_to_linear(self).map(|v| from_linear(v.clamp(0.0, 1.0)).clamp(0.0, 1.0))
    }

    /// `#rrggbb` of the displayed colour.
    pub fn to_hex(self) -> String {
        let [r, g, b] = self.to_srgb().map(|v| (v * 255.0).round() as u8);
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    /// WCAG 2 relative luminance of the displayed colour.
    pub fn luminance(self) -> f64 {
        let [r, g, b] = self.to_srgb().map(to_linear);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

/// WCAG 2 contrast ratio between two opaque colours (1..21).
pub fn contrast(a: Lab, b: Lab) -> f64 {
    let (x, y) = (a.luminance(), b.luminance());
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(v: f64) -> f64 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn lab_to_linear(c: Lab) -> [f64; 3] {
    let l_ = c.l + 0.396_337_777_4 * c.a + 0.215_803_757_3 * c.b;
    let m_ = c.l - 0.105_561_345_8 * c.a - 0.063_854_172_8 * c.b;
    let s_ = c.l - 0.089_484_177_5 * c.a - 1.291_485_548 * c.b;
    let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
    ]
}

fn linear_to_lab([r, g, b]: [f64; 3]) -> Lab {
    let l = 0.412_221_470_8 * r + 0.536_337_721_2 * g + 0.051_445_992_9 * b;
    let m = 0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b;
    let s = 0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b;
    let (l_, m_, s_) = (l.cbrt(), m.cbrt(), s.cbrt());
    Lab {
        l: 0.210_454_255_3 * l_ + 0.793_617_785 * m_ - 0.004_072_046_8 * s_,
        a: 1.977_998_495_1 * l_ - 2.428_592_205 * m_ + 0.450_593_709_9 * s_,
        b: 0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766 * s_,
    }
}

/// A colour value a theme may give a token: `#rgb`, `#rrggbb` or `oklch(L% C H)`. Parsed into
/// numbers; the style layer is regenerated from these numbers, never from the original text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorValue {
    Hex([u8; 3]),
    Oklch { l: f64, c: f64, h: f64 },
}

/// Bounds of an `oklch()` override.
pub const OKLCH_MAX_CHROMA: f64 = 0.4;

impl ColorValue {
    /// Strict parser. Accepts only the two documented forms, with bounded numbers and nothing
    /// else (no other functions, no `url(`, no separators, no comments).
    pub fn parse(text: &str) -> Option<ColorValue> {
        if text.len() > 40 {
            return None;
        }
        if let Some(hex) = text.strip_prefix('#') {
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let full: String = match hex.len() {
                3 => hex.chars().flat_map(|c| [c, c]).collect(),
                6 => hex.to_string(),
                _ => return None,
            };
            let byte = |i: usize| u8::from_str_radix(&full[i..i + 2], 16).ok();
            return Some(ColorValue::Hex([byte(0)?, byte(2)?, byte(4)?]));
        }
        let inner = text.strip_prefix("oklch(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split_ascii_whitespace().collect();
        let [l, c, h] = parts.as_slice() else {
            return None;
        };
        let l = parse_plain_number(l.strip_suffix('%')?)?;
        let c = parse_plain_number(c)?;
        let h = parse_plain_number(h)?;
        if !(0.0..=100.0).contains(&l)
            || !(0.0..=OKLCH_MAX_CHROMA).contains(&c)
            || !(0.0..=360.0).contains(&h)
        {
            return None;
        }
        Some(ColorValue::Oklch { l, c, h })
    }

    pub fn lab(self) -> Lab {
        match self {
            ColorValue::Hex([r, g, b]) => Lab::from_srgb8(r, g, b),
            ColorValue::Oklch { l, c, h } => Lab::from_oklch(l, c, h),
        }
    }

    /// Canonical CSS text, generated from the numbers.
    pub fn css(self) -> String {
        match self {
            ColorValue::Hex([r, g, b]) => format!("#{r:02x}{g:02x}{b:02x}"),
            ColorValue::Oklch { l, c, h } => {
                format!("oklch({}% {} {})", trim(l, 3), trim(c, 4), trim(h, 3))
            }
        }
    }

    /// The displayed colour as `#rrggbb` (used for corrected values).
    pub fn from_lab(lab: Lab) -> ColorValue {
        let [r, g, b] = lab.to_srgb().map(|v| (v * 255.0).round() as u8);
        ColorValue::Hex([r, g, b])
    }
}

/// A decimal number without sign, exponent or anything else: `12`, `0.5`, `.5`, `12.25`.
fn parse_plain_number(s: &str) -> Option<f64> {
    if s.is_empty() || s.len() > 10 {
        return None;
    }
    let mut dots = 0;
    for b in s.bytes() {
        match b {
            b'0'..=b'9' => {}
            b'.' => dots += 1,
            _ => return None,
        }
    }
    if dots > 1 || s == "." {
        return None;
    }
    let v: f64 = s.parse().ok()?;
    v.is_finite().then_some(v)
}

fn trim(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() { "0".into() } else { s.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_black_have_the_extreme_contrast() {
        let w = Lab::from_srgb8(255, 255, 255);
        let k = Lab::from_srgb8(0, 0, 0);
        assert!((contrast(w, k) - 21.0).abs() < 0.01);
        assert!((contrast(w, w) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn srgb_round_trips_through_oklab() {
        for (r, g, b) in [(217, 130, 43), (30, 35, 48), (243, 241, 236), (1, 2, 3)] {
            let hex = Lab::from_srgb8(r, g, b).to_hex();
            assert_eq!(hex, format!("#{r:02x}{g:02x}{b:02x}"));
        }
    }

    #[test]
    fn known_contrast_of_the_default_button() {
        // White on the accent #d9822b is 2.93 to 1, which is why OpenFrame Default uses dark button text.
        let r = contrast(
            Lab::from_srgb8(255, 255, 255),
            Lab::from_srgb8(0xd9, 0x82, 0x2b),
        );
        assert!((r - 2.93).abs() < 0.01, "{r}");
    }

    #[test]
    fn colour_values_parse_only_the_documented_forms() {
        assert_eq!(
            ColorValue::parse("#abc"),
            Some(ColorValue::Hex([0xaa, 0xbb, 0xcc]))
        );
        assert_eq!(ColorValue::parse("#A0b1C2").unwrap().css(), "#a0b1c2");
        assert_eq!(
            ColorValue::parse("oklch(62% 0.15 62)").unwrap().css(),
            "oklch(62% 0.15 62)"
        );
        assert_eq!(
            ColorValue::parse("oklch(62.5% .15 62.25)").unwrap().css(),
            "oklch(62.5% 0.15 62.25)"
        );
        for bad in [
            "",
            "#",
            "#ab",
            "#abcd",
            "#abcdef12",
            "#ggg",
            "red",
            "rgb(1,2,3)",
            "oklch(62% 0.15)",
            "oklch(62 0.15 62)",
            "oklch(101% 0.1 10)",
            "oklch(50% 0.5 10)",
            "oklch(50% 0.1 361)",
            "oklch(50% -0.1 10)",
            "oklch(50% 1e-1 10)",
            "oklch(NaN% 0.1 10)",
            "oklch(50% 0.1 10);",
            "oklch(50% 0.1 10) url(x)",
            "url(#a)",
            "#fff;",
            "#fff}",
            "@import",
            "var(--ink)",
            "oklch(50%  0.1 10 / 0.5)",
            "calc(1)",
            "oklch(5..0% 0.1 1)",
            " #fff",
        ] {
            assert_eq!(ColorValue::parse(bad), None, "{bad:?} must be refused");
        }
    }
}
