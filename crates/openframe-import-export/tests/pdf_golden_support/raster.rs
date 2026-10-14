//! PDF page rasterizer over the Windows built-in WinRT PDF renderer
//! (`Windows.Data.Pdf.PdfDocument`, `PdfPage.RenderWithOptionsToStreamAsync`).
//!
//! No extra crate: the `windows` crate (0.61, already in the lockfile) is a
//! Windows-only dev-dependency with the `Data_Pdf`, `Foundation`,
//! `Graphics_Imaging`, `Storage_Streams` and `UI` features. The PDF bytes go in
//! through an in-memory stream (no temporary file), each page comes back as a
//! PNG in memory and is decoded with the crate's existing `image` dependency.
//!
//! Rendering is pinned so it does not depend on the machine's display state:
//! exact output pixel size from a fixed DPI, opaque white background, the PNG
//! encoder, and high-contrast themes ignored.
//!
//! **Display scale.** WinRT multiplies `DestinationWidth`/`DestinationHeight`
//! by the display's resolution-scale bucket (for example 140 % on a 150 %
//! laptop panel), whatever the thread's DPI awareness. The rasterizer measures
//! that factor from the first render and asks for the size that comes out at
//! the target pixel size (cropping at most two blank edge rows/columns when a
//! size is unreachable at that scale), so goldens have the same dimensions on
//! every machine. The remaining sub-pixel scale difference between machines
//! (at most about one pixel across a page) is inside the 1 px compare
//! neighbourhood.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use image::{GrayImage, ImageFormat, Luma, RgbaImage};
use windows::Data::Pdf::{PdfDocument, PdfPage, PdfPageRenderOptions};
use windows::Graphics::Imaging::BitmapEncoder;
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows::UI::Color;

type BoxError = Box<dyn std::error::Error>;

/// WinRT reports page sizes in device-independent pixels (1/96 inch).
const DIPS_PER_INCH: f64 = 96.0;

/// Lowest alpha accepted in a render (see the tile-seam note in `rasterize_inner`).
const MIN_ALPHA: u8 = 250;

/// One rendered page.
pub struct RenderedPage {
    /// Page size in PDF points (1/72 inch) as the renderer reports it.
    pub width_pt: f64,
    pub height_pt: f64,
    /// 8-bit luminance, 255 = white paper.
    pub image: GrayImage,
}

/// Target pixel size -> destination size to request, learned per process.
type SizeMap = HashMap<(u32, u32), (u32, u32)>;

fn request_sizes() -> &'static Mutex<SizeMap> {
    static SIZES: OnceLock<Mutex<SizeMap>> = OnceLock::new();
    SIZES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Render every page of `pdf` at `dpi` pixels per inch.
pub fn rasterize(pdf: &[u8], dpi: u32) -> Result<Vec<RenderedPage>, String> {
    rasterize_inner(pdf, dpi).map_err(|e| format!("WinRT PDF renderer failed: {e}"))
}

fn rasterize_inner(pdf: &[u8], dpi: u32) -> Result<Vec<RenderedPage>, BoxError> {
    let input = InMemoryRandomAccessStream::new()?;
    {
        let writer = DataWriter::CreateDataWriter(&input.GetOutputStreamAt(0)?)?;
        writer.WriteBytes(pdf)?;
        let stored = writer.StoreAsync()?.get()?;
        if stored as usize != pdf.len() {
            return Err(format!("stored {stored} of {} PDF bytes", pdf.len()).into());
        }
        writer.FlushAsync()?.get()?;
        writer.DetachStream()?;
    }
    input.Seek(0)?;
    let doc = PdfDocument::LoadFromStreamAsync(&input)?.get()?;
    if doc.IsPasswordProtected()? {
        return Err("the PDF is password protected".into());
    }
    let mut pages = Vec::new();
    for index in 0..doc.PageCount()? {
        let page = doc.GetPage(index)?;
        let size = page.Size()?;
        let width_in = f64::from(size.Width) / DIPS_PER_INCH;
        let height_in = f64::from(size.Height) / DIPS_PER_INCH;
        let target = (
            (width_in * f64::from(dpi)).round() as u32,
            (height_in * f64::from(dpi)).round() as u32,
        );
        let rgba = render_exact(&page, target).map_err(|e| format!("page {}: {e}", index + 1))?;
        page.Close()?;
        let mut gray = GrayImage::new(target.0, target.1);
        for (x, y, p) in rgba.enumerate_pixels() {
            let [r, g, b, a] = p.0;
            // The opaque white background covers the whole page, but the
            // renderer's tile seams come back as white at alpha 254 (seen on
            // every render, at any size). Anything more transparent means the
            // background option was not honoured.
            if a < MIN_ALPHA {
                return Err(format!(
                    "page {} pixel ({x},{y}) is not opaque (alpha {a})",
                    index + 1
                )
                .into());
            }
            // Rec. 601 luma in integer arithmetic (deterministic), then
            // composited over white paper.
            let luma = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114 + 500) / 1000;
            let a = u32::from(a);
            let over_white = (luma * a + 255 * (255 - a) + 127) / 255;
            gray.put_pixel(x, y, Luma([over_white as u8]));
        }
        pages.push(RenderedPage {
            width_pt: width_in * 72.0,
            height_pt: height_in * 72.0,
            image: gray,
        });
    }
    Ok(pages)
}

/// Render `page` to exactly `target` pixels, compensating for the display scale.
fn render_exact(page: &PdfPage, target: (u32, u32)) -> Result<RgbaImage, BoxError> {
    let known = request_sizes()
        .lock()
        .map_err(|_| "size cache poisoned")?
        .get(&target)
        .copied();
    let first_request = known.unwrap_or(target);
    let first = render_at(page, first_request)?;
    if covers(&first, target) {
        remember(target, first_request)?;
        return crop_to(first, target);
    }
    if known.is_some() {
        return Err(format!(
            "asked for {first_request:?} expecting at least {target:?}, got {:?}",
            first.dimensions()
        )
        .into());
    }
    // Measure the factor WinRT applied, start from the nearest request and
    // grow until the output covers the target. Some sizes cannot be hit
    // exactly (at 140 % no request yields 2200 px), so at most two extra
    // pixel rows/columns of blank paper are cropped off the far edges.
    let sx = f64::from(first.width()) / f64::from(first_request.0);
    let sy = f64::from(first.height()) / f64::from(first_request.1);
    let mut request = (
        (f64::from(target.0) / sx).round().max(1.0) as u32,
        (f64::from(target.1) / sy).round().max(1.0) as u32,
    );
    let mut tried = Vec::new();
    for _ in 0..4 {
        let img = render_at(page, request)?;
        if covers(&img, target) {
            remember(target, request)?;
            return crop_to(img, target);
        }
        tried.push((request, img.dimensions()));
        if img.width() < target.0 {
            request.0 += 1;
        }
        if img.height() < target.1 {
            request.1 += 1;
        }
    }
    Err(format!(
        "display scale {sx:.3}x{sy:.3}: no destination size covers {target:?} (tried {tried:?})"
    )
    .into())
}

/// The render is at least `target` and at most two pixels larger in each direction.
fn covers(img: &RgbaImage, target: (u32, u32)) -> bool {
    let (w, h) = img.dimensions();
    (target.0..=target.0 + 2).contains(&w) && (target.1..=target.1 + 2).contains(&h)
}

/// Crop the extra right columns / bottom rows, which must be blank paper.
fn crop_to(img: RgbaImage, target: (u32, u32)) -> Result<RgbaImage, BoxError> {
    if img.dimensions() == target {
        return Ok(img);
    }
    for (x, y, p) in img.enumerate_pixels() {
        if (x >= target.0 || y >= target.1) && p.0[..3].iter().any(|&c| c < 250) {
            return Err(format!("ink at ({x},{y}) in the cropped page edge").into());
        }
    }
    Ok(image::imageops::crop_imm(&img, 0, 0, target.0, target.1).to_image())
}

fn remember(target: (u32, u32), request: (u32, u32)) -> Result<(), BoxError> {
    request_sizes()
        .lock()
        .map_err(|_| "size cache poisoned")?
        .insert(target, request);
    Ok(())
}

fn render_at(page: &PdfPage, request: (u32, u32)) -> Result<RgbaImage, BoxError> {
    let options = PdfPageRenderOptions::new()?;
    options.SetDestinationWidth(request.0)?;
    options.SetDestinationHeight(request.1)?;
    options.SetBackgroundColor(Color {
        A: 255,
        R: 255,
        G: 255,
        B: 255,
    })?;
    options.SetIsIgnoringHighContrast(true)?;
    options.SetBitmapEncoderId(BitmapEncoder::PngEncoderId()?)?;

    let output = InMemoryRandomAccessStream::new()?;
    page.RenderWithOptionsToStreamAsync(&output, &options)?
        .get()?;

    let len = u32::try_from(output.Size()?)?;
    let reader = DataReader::CreateDataReader(&output.GetInputStreamAt(0)?)?;
    let loaded = reader.LoadAsync(len)?.get()?;
    if loaded != len {
        return Err(format!("read {loaded} of {len} PNG bytes").into());
    }
    let mut encoded = vec![0u8; len as usize];
    reader.ReadBytes(&mut encoded)?;
    Ok(image::load_from_memory_with_format(&encoded, ImageFormat::Png)?.to_rgba8())
}
