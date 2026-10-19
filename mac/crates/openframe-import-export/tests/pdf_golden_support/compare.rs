//! Image comparison and ink checks for the golden PDF harness.
//!
//! ## Tolerance (documented in docs/engineering/22-qa-test-strategy.md)
//!
//! Pages are 8-bit grayscale at [`super::DPI`] (200 px per inch, so one
//! Courier character cell is 20 px wide and one screenplay line is 33.3 px).
//!
//! * **Per-pixel threshold.** A pixel *differs* when its value is more than
//!   [`Tolerance::pixel_threshold`] (64 of 255) away from **every** pixel within
//!   [`Tolerance::radius`] (1 px, 0.005") of the same position in the other
//!   image, in either direction. This absorbs anti-aliasing and sub-pixel glyph
//!   placement differences between renderer builds and display scales, but not
//!   a 2 px (0.01") shift.
//! * **Max differing-pixel ratio.** A page fails when more than
//!   [`Tolerance::max_diff_ratio`] (0.02 %, about 748 px of a Letter page) of
//!   its pixels differ.
//! * **Local cluster limit.** A page also fails when any
//!   [`Tolerance::tile`] x `tile` (16 x 16 px, 0.08") tile holds more than
//!   [`Tolerance::max_diff_per_tile`] (8) differing pixels, so a single missing
//!   or changed glyph fails even though it is far below the page-wide ratio.
//!   Measured on Windows 11 at 200 DPI: one letter changed ("a" to "A") gives a
//!   worst tile of 20 px, "s" to "z" 27 px, a missing revision asterisk 30 px;
//!   a 1 px shift of the whole page gives 0 differing px and a 2 px shift
//!   about 104,000.

use image::{GrayImage, Rgb, RgbImage};

#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    pub pixel_threshold: u8,
    pub radius: u32,
    pub max_diff_ratio: f64,
    pub tile: u32,
    pub max_diff_per_tile: u32,
}

pub const TOLERANCE: Tolerance = Tolerance {
    pixel_threshold: 64,
    radius: 1,
    max_diff_ratio: 0.0002,
    tile: 16,
    max_diff_per_tile: 8,
};

/// Luminance below this is ink (anything visibly not white paper).
pub const INK_THRESHOLD: u8 = 192;

#[derive(Debug)]
pub struct Comparison {
    pub width: u32,
    pub height: u32,
    pub differing: u64,
    pub worst_tile: u32,
    /// Row-major mask of differing pixels.
    pub mask: Vec<bool>,
}

impl Comparison {
    pub fn ratio(&self) -> f64 {
        self.differing as f64 / (u64::from(self.width) * u64::from(self.height)) as f64
    }

    pub fn passes(&self, tol: &Tolerance) -> bool {
        self.ratio() <= tol.max_diff_ratio && self.worst_tile <= tol.max_diff_per_tile
    }

    pub fn summary(&self, tol: &Tolerance) -> String {
        format!(
            "{} differing px ({:.5}% of page, limit {:.5}%), worst {}x{} tile {} px (limit {})",
            self.differing,
            self.ratio() * 100.0,
            tol.max_diff_ratio * 100.0,
            tol.tile,
            tol.tile,
            self.worst_tile,
            tol.max_diff_per_tile
        )
    }
}

/// Is `value` within the threshold of any pixel near (x, y) in `other`?
fn matched_nearby(value: u8, other: &GrayImage, x: u32, y: u32, tol: &Tolerance) -> bool {
    let r = tol.radius;
    let x0 = x.saturating_sub(r);
    let y0 = y.saturating_sub(r);
    let x1 = (x + r).min(other.width() - 1);
    let y1 = (y + r).min(other.height() - 1);
    for yy in y0..=y1 {
        for xx in x0..=x1 {
            if value.abs_diff(other.get_pixel(xx, yy).0[0]) <= tol.pixel_threshold {
                return true;
            }
        }
    }
    false
}

/// Compare two same-sized pages. Panics on a size mismatch (callers check sizes first).
pub fn compare(expected: &GrayImage, actual: &GrayImage, tol: &Tolerance) -> Comparison {
    assert_eq!(
        expected.dimensions(),
        actual.dimensions(),
        "compare() needs same-sized images"
    );
    let (w, h) = expected.dimensions();
    let mut mask = vec![false; (w as usize) * (h as usize)];
    let mut differing = 0u64;
    for y in 0..h {
        for x in 0..w {
            let e = expected.get_pixel(x, y).0[0];
            let a = actual.get_pixel(x, y).0[0];
            if e.abs_diff(a) <= tol.pixel_threshold {
                continue;
            }
            if !matched_nearby(e, actual, x, y, tol) || !matched_nearby(a, expected, x, y, tol) {
                mask[(y * w + x) as usize] = true;
                differing += 1;
            }
        }
    }
    let mut worst_tile = 0u32;
    for ty in (0..h).step_by(tol.tile as usize) {
        for tx in (0..w).step_by(tol.tile as usize) {
            let mut n = 0u32;
            for y in ty..(ty + tol.tile).min(h) {
                for x in tx..(tx + tol.tile).min(w) {
                    if mask[(y * w + x) as usize] {
                        n += 1;
                    }
                }
            }
            worst_tile = worst_tile.max(n);
        }
    }
    Comparison {
        width: w,
        height: h,
        differing,
        worst_tile,
        mask,
    }
}

/// Visual diff: expected content faded to light gray, actual ink in blue,
/// differing pixels in red.
pub fn diff_image(expected: &GrayImage, actual: &GrayImage, cmp: &Comparison) -> RgbImage {
    let mut out = RgbImage::new(cmp.width, cmp.height);
    for (x, y, p) in out.enumerate_pixels_mut() {
        let i = (y * cmp.width + x) as usize;
        let e = expected.get_pixel(x, y).0[0];
        let a = actual.get_pixel(x, y).0[0];
        *p = if cmp.mask[i] {
            Rgb([220, 0, 0])
        } else if a < INK_THRESHOLD {
            Rgb([a / 2, a / 2, 255])
        } else {
            let faded = 255 - (255 - e) / 4;
            Rgb([faded, faded, faded])
        };
    }
    out
}

/// Bounding box of ink as (left, top, right, bottom) in pixels, inclusive.
/// `None` for a blank page.
pub fn ink_bbox(img: &GrayImage) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for (x, y, p) in img.enumerate_pixels() {
        if p.0[0] < INK_THRESHOLD {
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
            });
        }
    }
    bbox
}

/// `img` moved right/down by (dx, dy) pixels, uncovered area white. Used by the
/// harness self-tests to prove where the tolerance stops.
pub fn shifted(img: &GrayImage, dx: u32, dy: u32) -> GrayImage {
    let (w, h) = img.dimensions();
    GrayImage::from_fn(w, h, |x, y| {
        if x >= dx && y >= dy {
            *img.get_pixel(x - dx, y - dy)
        } else {
            image::Luma([255])
        }
    })
}
