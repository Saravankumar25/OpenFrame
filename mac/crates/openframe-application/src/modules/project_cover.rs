//! Cover photos for the project cards on Application Home (owner request 2026-10-24).
//!
//! Home lists projects that are closed, so the picture belongs to the app, not to the project:
//! `<app data>/covers/assets/<projectId>-<rev>.jpg`, named by `sys_recent_project.cover_file`. The
//! chosen file is untrusted input: it is size-capped before it is read, recognised by its first
//! bytes (PNG, JPEG or WebP only, never SVG), measured from its header, decoded under allocation
//! limits, centre-cropped to 16:9, scaled to 960 x 540 and written again as a JPEG. The webview
//! reads the result through the `ofasset` protocol, whose scope names this one `assets` folder.

use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, GenericImageView, ImageFormat};
use openframe_domain::ids::is_valid_id;
use openframe_domain::{Actor, AppError, AppResult, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::project::{RecentProject, recent_entry, recent_list_changed, require_local};
use crate::core::AppCore;
use crate::registry::Registry;

const MAX_SOURCE_BYTES: u64 = 25 << 20;
const MIN_SIDE: u32 = 64;
const MAX_SIDE: u32 = 12_000;
const MAX_ALLOC: u64 = 512 << 20;
const COVER_W: u32 = 960;
const COVER_H: u32 = 540;
const JPEG_QUALITY: u8 = 82;
/// Most folder entries the start-up sweep looks at.
const SWEEP_LIMIT: usize = 10_000;

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.command("project.set_cover", set_cover).meta(
        M::app_command("Choose a cover photo for a project card on Home.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.command("project.clear_cover", clear_cover).meta(
        M::app_command("Remove a project's cover photo so Home shows the generated cover.")
            .hidden(h::APP_LIFECYCLE),
    );
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCover {
    /// Full path of the picture, for the `ofasset` protocol.
    pub path: String,
    /// Changes with every new picture.
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetCoverArgs {
    pub project_id: String,
    pub path: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearCoverArgs {
    pub project_id: String,
}

fn covers_dir(core: &AppCore) -> PathBuf {
    folder(&core.config.app_data_dir)
}

fn folder(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("covers").join("assets")
}

/// A stored name is `<project id>-<number>.jpg` and nothing else, so it can never hold a
/// separator, a dot segment or a drive letter.
fn is_cover_name(name: &str) -> bool {
    let Some((id, rev)) = name.strip_suffix(".jpg").and_then(|s| s.rsplit_once('-')) else {
        return false;
    };
    id.len() == 36
        && is_valid_id(id)
        && (1..=12).contains(&rev.len())
        && rev.bytes().all(|b| b.is_ascii_digit())
}

fn cover_name(project_id: &str, rev: i64) -> AppResult<String> {
    let name = format!("{project_id}-{rev}.jpg");
    if is_cover_name(&name) {
        Ok(name)
    } else {
        Err(AppError::invalid_input("That project could not be found."))
    }
}

/// The cover to show for a list row, or `None` when there is none or its file is gone.
pub(super) fn cover_for(core: &AppCore, file: Option<String>, rev: i64) -> Option<ProjectCover> {
    let name = file.filter(|n| is_cover_name(n))?;
    let path = covers_dir(core).join(name);
    path.is_file().then(|| ProjectCover {
        path: path.to_string_lossy().into_owned(),
        rev,
    })
}

/// The cover file name stored for a project, read before its row is removed.
pub(super) fn file_of(core: &AppCore, project_id: &str) -> Option<String> {
    core.with_app_db(|c| cover_row(c, project_id))
        .ok()
        .flatten()
        .and_then(|r| r.0)
}

/// Delete a cover file by its stored name (best effort; a leftover is swept at start-up).
pub(super) fn discard(core: &AppCore, name: Option<String>) {
    let Some(name) = name.filter(|n| is_cover_name(n)) else {
        return;
    };
    let path = covers_dir(core).join(name);
    if let Err(e) = std::fs::remove_file(&path)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!(error = %e, "could not remove a cover photo");
    }
}

/// `(cover_file, cover_rev)` of a project row, or `None` when the project is not in the list.
fn cover_row(c: &Connection, project_id: &str) -> AppResult<Option<(Option<String>, i64)>> {
    Ok(c.query_row(
        "SELECT cover_file, cover_rev FROM sys_recent_project WHERE project_id=?1",
        [project_id],
        |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
    )
    .optional()?)
}

fn unreadable() -> AppError {
    AppError::invalid_input("That image could not be read. Choose a PNG, JPEG or WebP picture.")
}

fn sniff(head: &[u8]) -> Option<ImageFormat> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else {
        None
    }
}

fn decode(bytes: &[u8], format: ImageFormat) -> AppResult<DynamicImage> {
    let reader = || image::ImageReader::with_format(Cursor::new(bytes), format);
    let (w, h) = reader()
        .into_dimensions()
        .map_err(|e| unreadable().with_detail(e.to_string()))?;
    if w < MIN_SIDE || h < MIN_SIDE {
        return Err(AppError::invalid_input(format!(
            "That picture is too small for a cover. Choose one at least {MIN_SIDE} pixels wide and tall."
        )));
    }
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(AppError::invalid_input(
            "That picture is too large for a cover (more than 12,000 pixels on a side). Choose a smaller one.",
        ));
    }
    let mut reader = reader();
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| unreadable().with_detail(e.to_string()))
}

/// The finished cover: a 960 x 540 JPEG cut from the middle of the chosen picture.
pub(crate) fn render_cover(path: &Path) -> AppResult<Vec<u8>> {
    openframe_security::validate_input_file(path, MAX_SOURCE_BYTES)?;
    let mut bytes = Vec::new();
    // Read one byte past the cap so a file that grew after the check is still refused.
    std::fs::File::open(path)?
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(unreadable());
    }
    let format = sniff(&bytes).ok_or_else(unreadable)?;
    let decoded = std::panic::catch_unwind(|| decode(&bytes, format))
        .map_err(|_| unreadable().with_detail("decoder panicked"))??;
    let (w, h) = decoded.dimensions();
    let (cw, ch) = if u64::from(w) * 9 > u64::from(h) * 16 {
        ((u64::from(h) * 16 / 9) as u32, h)
    } else {
        (w, (u64::from(w) * 9 / 16) as u32)
    };
    let scaled = decoded
        .crop_imm((w - cw) / 2, (h - ch) / 2, cw, ch)
        .resize_exact(COVER_W, COVER_H, FilterType::Lanczos3)
        .to_rgba8();
    // A picture with see-through parts is laid on white, as the page behind the card is light.
    let mut rgb = Vec::with_capacity((COVER_W * COVER_H * 3) as usize);
    for p in scaled.pixels() {
        let a = u32::from(p.0[3]);
        for &c in &p.0[..3] {
            rgb.push(((u32::from(c) * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode(&rgb, COVER_W, COVER_H, ExtendedColorType::Rgb8)
        .map_err(|e| unreadable().with_detail(e.to_string()))?;
    Ok(out)
}

fn set_cover(core: &AppCore, actor: &Actor, args: SetCoverArgs) -> AppResult<RecentProject> {
    require_local(actor)?;
    let (old, rev) = core
        .with_app_db(|c| cover_row(c, &args.project_id))?
        .ok_or_else(|| AppError::not_found("project"))?;
    let jpeg = render_cover(Path::new(&args.path))?;
    let name = cover_name(&args.project_id, rev + 1)?;
    let dir = covers_dir(core);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(&name);
    openframe_platform::atomic::replace_with(&dest, |f| f.write_all(&jpeg))?;
    let changed = core.with_app_db(|c| {
        Ok(c.execute(
            "UPDATE sys_recent_project SET cover_file=?1, cover_rev=?2, updated_at=?3 WHERE project_id=?4",
            params![name, rev + 1, now_ms(), args.project_id],
        )?)
    });
    if !matches!(changed, Ok(n) if n > 0) {
        let _ = std::fs::remove_file(&dest);
        return Err(changed
            .err()
            .unwrap_or_else(|| AppError::not_found("project")));
    }
    discard(core, old);
    recent_list_changed(core, actor, &args.project_id);
    recent_entry(core, &args.project_id)
}

fn clear_cover(core: &AppCore, actor: &Actor, args: ClearCoverArgs) -> AppResult<RecentProject> {
    require_local(actor)?;
    let (old, _) = core
        .with_app_db(|c| cover_row(c, &args.project_id))?
        .ok_or_else(|| AppError::not_found("project"))?;
    core.with_app_db(|c| {
        c.execute(
            "UPDATE sys_recent_project SET cover_file=NULL, updated_at=?1 WHERE project_id=?2",
            params![now_ms(), args.project_id],
        )?;
        Ok(())
    })?;
    discard(core, old);
    recent_list_changed(core, actor, &args.project_id);
    recent_entry(core, &args.project_id)
}

/// Start-up clean-up: remove cover files no list row names (left by a crash between the file and
/// the row). Looks at plain files with a cover name only, never follows links, stops at a limit.
pub(crate) fn sweep(conn: &Connection, app_data_dir: &Path) {
    let dir = folder(app_data_dir);
    openframe_platform::atomic::remove_stale_temps(&dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT cover_file FROM sys_recent_project WHERE cover_file IS NOT NULL")
    else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) else {
        return;
    };
    let named: std::collections::HashSet<String> = rows.flatten().collect();
    for entry in entries.flatten().take(SWEEP_LIMIT) {
        let is_file = entry.file_type().is_ok_and(|t| t.is_file());
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_file && is_cover_name(&name) && !named.contains(&name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0198b2a0-0000-7000-8000-000000000001";

    fn write_image(dir: &Path, name: &str, w: u32, h: u32, format: ImageFormat) -> PathBuf {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 251) as u8, (y % 241) as u8, 90])
        });
        let path = dir.join(name);
        DynamicImage::ImageRgb8(img)
            .save_with_format(&path, format)
            .unwrap();
        path
    }

    #[test]
    fn names_are_project_id_dash_number_dot_jpg_only() {
        assert!(is_cover_name(&format!("{ID}-3.jpg")));
        for bad in [
            "../x-1.jpg",
            &format!("{ID}-.jpg"),
            &format!("{ID}-1.png"),
            &format!("{ID}-1.jpg.exe"),
            &format!("{ID}\\..-1.jpg"),
            &format!("{{{ID}}}-1.jpg"),
            &format!("{ID}-1a.jpg"),
            &format!("{}-1.jpg", ID.replace('-', "")),
        ] {
            assert!(!is_cover_name(bad), "{bad}");
        }
        assert!(cover_name("..\\..\\evil", 1).is_err());
    }

    #[test]
    fn accepts_png_jpeg_and_webp_and_crops_to_16_by_9() {
        let dir = tempfile::tempdir().unwrap();
        for (name, format) in [
            ("a.png", ImageFormat::Png),
            ("b.jpg", ImageFormat::Jpeg),
            ("c.webp", ImageFormat::WebP),
        ] {
            // Tall, wide and square sources all come out 960 x 540.
            for (w, h) in [(300, 900), (1400, 300), (500, 500)] {
                let p = write_image(dir.path(), name, w, h, format);
                let out = render_cover(&p).unwrap_or_else(|e| panic!("{name} {w}x{h}: {e:?}"));
                assert_eq!(sniff(&out), Some(ImageFormat::Jpeg));
                let back = image::load_from_memory(&out).unwrap();
                assert_eq!(back.dimensions(), (960, 540), "{name} {w}x{h}");
            }
        }
    }

    #[test]
    fn rejects_other_kinds_too_small_too_large_and_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let svg = dir.path().join("a.png");
        std::fs::write(&svg, "<svg xmlns='http://www.w3.org/2000/svg'/>").unwrap();
        let gif = write_image(dir.path(), "b.gif", 200, 200, ImageFormat::Gif);
        let bmp = write_image(dir.path(), "c.bmp", 200, 200, ImageFormat::Bmp);
        let tiny = write_image(dir.path(), "d.png", 63, 500, ImageFormat::Png);
        let huge = write_image(dir.path(), "e.png", 12_001, 70, ImageFormat::Png);
        let mut corrupt =
            std::fs::read(write_image(dir.path(), "f.png", 200, 200, ImageFormat::Png)).unwrap();
        corrupt.truncate(corrupt.len() / 2);
        let corrupt_path = dir.path().join("g.png");
        std::fs::write(&corrupt_path, corrupt).unwrap();
        let big = dir.path().join("h.png");
        std::fs::write(&big, vec![0x89; (MAX_SOURCE_BYTES + 1) as usize]).unwrap();
        for p in [&svg, &gif, &bmp, &corrupt_path] {
            let e = render_cover(p).unwrap_err();
            assert!(
                e.message.contains("PNG, JPEG or WebP"),
                "{p:?}: {}",
                e.message
            );
        }
        assert!(
            render_cover(&tiny)
                .unwrap_err()
                .message
                .contains("too small")
        );
        assert!(
            render_cover(&huge)
                .unwrap_err()
                .message
                .contains("too large")
        );
        assert!(
            render_cover(&big)
                .unwrap_err()
                .message
                .contains("too large")
        );
        assert!(render_cover(dir.path()).is_err(), "a folder is refused");
    }

    #[test]
    fn sweep_removes_only_unnamed_cover_files() {
        let app = tempfile::tempdir().unwrap();
        let dir = folder(app.path());
        std::fs::create_dir_all(&dir).unwrap();
        let mut conn = openframe_persistence::open_in_memory().unwrap();
        openframe_persistence::migrate::apply(&mut conn, crate::schema::APP_MIGRATIONS).unwrap();
        let kept = format!("{ID}-2.jpg");
        let orphan = format!("{ID}-1.jpg");
        conn.execute(
            "INSERT INTO sys_recent_project(project_id, path, title, project_type, status, archived, pinned, last_opened_at, updated_at, cover_file, cover_rev)
             VALUES (?1, 'p', 't', 'Short Film', 'Idea', 0, 0, 1, 1, ?2, 2)",
            params![ID, kept],
        )
        .unwrap();
        for n in [
            &kept,
            &orphan,
            &"notes.txt".to_string(),
            &"x-1.jpg".to_string(),
        ] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        std::fs::create_dir(dir.join(format!("{ID}-9.jpg"))).unwrap();
        sweep(&conn, app.path());
        assert!(dir.join(&kept).exists());
        assert!(!dir.join(&orphan).exists());
        assert!(dir.join("notes.txt").exists() && dir.join("x-1.jpg").exists());
        assert!(
            dir.join(format!("{ID}-9.jpg")).is_dir(),
            "a folder with a cover name is left alone"
        );
    }
}
