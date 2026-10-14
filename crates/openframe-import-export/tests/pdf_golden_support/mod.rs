//! Golden PDF image-compare harness (build plan item PDF image comparison, feature XFER-010).
//!
//! For each corpus case [`run_case`]:
//!
//! 1. exports the screenplay twice through the real export path
//!    ([`openframe_import_export::export_to_path`], the function the
//!    application's interchange export calls) and requires byte-identical PDFs
//!    (ADR-0008 decision 3);
//! 2. renders every page of both PDFs with the Windows built-in PDF renderer
//!    at [`DPI`] and requires pixel-identical renders;
//! 3. checks the page count (PDF pages and reported script pages) and each
//!    page's paper size;
//! 4. checks that no ink falls outside the case's margins (clipping);
//! 5. compares each page with the committed golden PNG under
//!    `tests/fixtures/pdf-golden/` using [`compare::TOLERANCE`].
//!
//! Goldens are never written implicitly. Run with `OPENFRAME_UPDATE_GOLDEN=1`
//! to regenerate them deliberately (review the PNGs before committing). On a
//! mismatch the expected, actual and diff images are written to
//! `<target>/tmp/pdf-golden-diff/`.

pub mod compare;
pub mod raster;

use std::path::{Path, PathBuf};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, GrayImage, ImageEncoder, ImageFormat};
use openframe_import_export::{ExportFormat, ExportOptions, ScreenplayDoc, export_to_path};

use compare::{TOLERANCE, Tolerance};
use raster::RenderedPage;

/// Render resolution: 200 px per inch (one Courier cell = 20 px). Chosen so a
/// single changed glyph clearly exceeds the per-tile limit (see `compare`).
pub const DPI: u32 = 200;
/// Set to `1` to regenerate goldens instead of comparing.
pub const UPDATE_ENV: &str = "OPENFRAME_UPDATE_GOLDEN";

/// US Letter in PDF points.
pub const LETTER_PT: (f64, f64) = (612.0, 792.0);

/// Minimum distance, in inches, between each page edge and any ink.
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

/// Screenplay text area: body text from 1.5" to 7.5" (60 Courier cells), the
/// page number right-aligned to 7.5" in the header band from 0.5", and the
/// last of the 55 body lines ending above 10.2". 0.05" of slack covers glyph
/// side bearings and the title underline.
pub const TEXT_AREA: Margins = Margins {
    left: 1.45,
    right: 0.95,
    top: 0.5,
    bottom: 0.8,
};

/// A4 with the same 1.5" to 7.5" columns as US Letter (right margin 0.77") and a
/// footer whose baseline sits half an inch above the bottom edge.
pub const A4_WITH_FOOTER: Margins = Margins {
    left: 1.45,
    right: 0.72,
    top: 0.5,
    bottom: 0.45,
};

/// Text area plus the margin furniture: scene numbers (left of 1.0" and from
/// 7.6") and revision asterisks (at 7.7"). Allows scene numbers up to three
/// characters on the left.
pub const WITH_MARGIN_MARKS: Margins = Margins {
    left: 0.65,
    right: 0.5,
    top: 0.5,
    bottom: 0.8,
};

pub struct Case {
    pub name: &'static str,
    pub doc: ScreenplayDoc,
    pub options: ExportOptions,
    pub paper_pt: (f64, f64),
    /// Every PDF page, title page included.
    pub pdf_pages: usize,
    /// Script pages as reported by the export (title page excluded).
    pub script_pages: usize,
    pub margins: Margins,
}

pub fn update_mode() -> bool {
    std::env::var(UPDATE_ENV).is_ok_and(|v| v == "1")
}

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pdf-golden")
}

fn diff_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pdf-golden-diff")
}

fn golden_path(case: &str, page: usize) -> PathBuf {
    golden_dir().join(format!("{case}-p{:02}.png", page + 1))
}

/// Export through the real path into a fresh temporary folder and read the file back.
pub fn export_pdf(doc: &ScreenplayDoc, options: &ExportOptions) -> (Vec<u8>, Option<usize>) {
    let dir = tempfile::tempdir().expect("temp dir");
    let dest = dir.path().join("golden.pdf");
    let outcome = export_to_path(doc, ExportFormat::Pdf, options, &dest).expect("PDF export");
    let bytes = std::fs::read(&dest).expect("read exported PDF");
    assert_eq!(outcome.bytes_written, bytes.len() as u64);
    (bytes, outcome.pages)
}

pub fn render(pdf: &[u8]) -> Vec<RenderedPage> {
    raster::rasterize(pdf, DPI).unwrap_or_else(|e| panic!("{e}"))
}

/// Ink outside the margins, as a message per offending page.
pub fn clipping_violations(pages: &[RenderedPage], m: &Margins) -> Vec<String> {
    let mut out = Vec::new();
    let px = |inches: f64| (inches * f64::from(DPI)).round() as u32;
    for (i, page) in pages.iter().enumerate() {
        let (w, h) = page.image.dimensions();
        let Some((l, t, r, b)) = compare::ink_bbox(&page.image) else {
            out.push(format!("page {}: blank page", i + 1));
            continue;
        };
        let (min_x, min_y) = (px(m.left), px(m.top));
        let (max_x, max_y) = (w - 1 - px(m.right), h - 1 - px(m.bottom));
        if l < min_x || t < min_y || r > max_x || b > max_y {
            let inch = |v: u32| f64::from(v) / f64::from(DPI);
            out.push(format!(
                "page {}: ink spans x {:.2}\"..{:.2}\", y {:.2}\"..{:.2}\"; allowed x {:.2}\"..{:.2}\", y {:.2}\"..{:.2}\"",
                i + 1,
                inch(l),
                inch(r + 1),
                inch(t),
                inch(b + 1),
                inch(min_x),
                inch(max_x + 1),
                inch(min_y),
                inch(max_y + 1)
            ));
        }
    }
    out
}

fn write_png(img: &GrayImage, path: &Path) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("create folder");
    }
    // Best compression keeps the committed goldens small (lossless either way).
    let mut encoded = Vec::new();
    PngEncoder::new_with_quality(&mut encoded, CompressionType::Best, FilterType::Adaptive)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::L8,
        )
        .unwrap_or_else(|e| panic!("encode {}: {e}", path.display()));
    std::fs::write(path, encoded).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn read_golden(path: &Path) -> Option<GrayImage> {
    if !path.exists() {
        return None;
    }
    let img = image::open(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    Some(img.to_luma8())
}

/// Golden page files for `case` (`<case>-pNN.png`), sorted.
fn golden_files(case: &str) -> Vec<PathBuf> {
    let prefix = format!("{case}-p");
    let mut files: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with(&prefix) && n.ends_with(".png"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Compare rendered pages with the committed goldens of `case`. Returns one
/// message per failing page; writes expected/actual/diff images when `write_diffs`.
pub fn golden_mismatches(
    case: &str,
    pages: &[RenderedPage],
    tol: &Tolerance,
    write_diffs: bool,
) -> Vec<String> {
    let mut failures = Vec::new();
    let committed = golden_files(case).len();
    if committed != pages.len() {
        failures.push(format!(
            "{case}: {} rendered pages but {committed} golden files",
            pages.len()
        ));
    }
    for (i, page) in pages.iter().enumerate() {
        let path = golden_path(case, i);
        let Some(expected) = read_golden(&path) else {
            failures.push(format!(
                "{case} page {}: no golden at {} (run with {UPDATE_ENV}=1 to create it, then review it)",
                i + 1,
                path.display()
            ));
            continue;
        };
        if expected.dimensions() != page.image.dimensions() {
            failures.push(format!(
                "{case} page {}: rendered {:?} px, golden {:?} px",
                i + 1,
                page.image.dimensions(),
                expected.dimensions()
            ));
            continue;
        }
        let cmp = compare::compare(&expected, &page.image, tol);
        if !cmp.passes(tol) {
            let mut msg = format!("{case} page {}: {}", i + 1, cmp.summary(tol));
            if write_diffs {
                let stem = diff_dir().join(format!("{case}-p{:02}", i + 1));
                write_png(&expected, &stem.with_extension("expected.png"));
                write_png(&page.image, &stem.with_extension("actual.png"));
                let diff = compare::diff_image(&expected, &page.image, &cmp);
                let diff_path = stem.with_extension("diff.png");
                std::fs::create_dir_all(diff_dir()).expect("create diff folder");
                diff.save_with_format(&diff_path, ImageFormat::Png)
                    .expect("write diff image");
                msg.push_str(&format!("; diff image {}", diff_path.display()));
            }
            failures.push(msg);
        }
    }
    failures
}

fn write_goldens(case: &str, pages: &[RenderedPage]) {
    for stale in golden_files(case) {
        std::fs::remove_file(&stale).expect("remove stale golden");
    }
    for (i, page) in pages.iter().enumerate() {
        write_png(&page.image, &golden_path(case, i));
    }
    eprintln!(
        "{UPDATE_ENV}=1: wrote {} golden page(s) for {case} to {}; review them before committing",
        pages.len(),
        golden_dir().display()
    );
}

/// Run every check for one corpus case. Panics with all failures listed.
pub fn run_case(case: &Case) {
    let mut failures: Vec<String> = Vec::new();

    // Determinism of the export: same input, same bytes.
    let (pdf, script_pages) = export_pdf(&case.doc, &case.options);
    let (again, _) = export_pdf(&case.doc, &case.options);
    assert!(pdf.starts_with(b"%PDF-"), "{}: not a PDF", case.name);
    assert!(
        pdf == again,
        "{}: two exports of the same screenplay differ (ADR-0008 requires byte-identical PDFs)",
        case.name
    );
    if script_pages != Some(case.script_pages) {
        failures.push(format!(
            "{}: export reports {script_pages:?} script pages, expected {}",
            case.name, case.script_pages
        ));
    }

    // Determinism of the render.
    let pages = render(&pdf);
    let pages_again = render(&again);
    assert_eq!(pages.len(), pages_again.len(), "{}: page count", case.name);
    for (i, (a, b)) in pages.iter().zip(&pages_again).enumerate() {
        assert!(
            a.image == b.image,
            "{} page {}: two renders of identical bytes differ",
            case.name,
            i + 1
        );
    }

    // Page count and paper size.
    if pages.len() != case.pdf_pages {
        failures.push(format!(
            "{}: {} PDF pages, expected {}",
            case.name,
            pages.len(),
            case.pdf_pages
        ));
    }
    for (i, p) in pages.iter().enumerate() {
        if (p.width_pt - case.paper_pt.0).abs() > 0.5 || (p.height_pt - case.paper_pt.1).abs() > 0.5
        {
            failures.push(format!(
                "{} page {}: paper {:.1}x{:.1} pt, expected {:.1}x{:.1} pt",
                case.name,
                i + 1,
                p.width_pt,
                p.height_pt,
                case.paper_pt.0,
                case.paper_pt.1
            ));
        }
    }

    // No clipping: all ink inside the margins.
    failures.extend(
        clipping_violations(&pages, &case.margins)
            .into_iter()
            .map(|m| format!("{}: {m}", case.name)),
    );

    // Golden image compare (or deliberate regeneration).
    if update_mode() {
        write_goldens(case.name, &pages);
    } else {
        failures.extend(golden_mismatches(case.name, &pages, &TOLERANCE, true));
    }

    assert!(
        failures.is_empty(),
        "golden PDF case `{}` failed:\n  {}",
        case.name,
        failures.join("\n  ")
    );
}
