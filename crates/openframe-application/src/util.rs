//! Shared helpers for modules: store selection, asset ingestion (managed copy /
//! external reference), and small validation utilities.

use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, new_id, now_ms};
use openframe_project_format::ProjectLayout;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::events::StoreKind;
use crate::store::{Store, Tx};

/// Which store an operation targets. Most operations are project-scoped; the
/// Idea Vault and history operations can also target the Global Idea Vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoreSel {
    #[default]
    Project,
    Global,
}

impl StoreSel {
    pub fn kind(self) -> StoreKind {
        match self {
            StoreSel::Project => StoreKind::Project,
            StoreSel::Global => StoreKind::Global,
        }
    }
}

/// Run `f` against the selected store.
pub fn with_store<R>(
    core: &AppCore,
    sel: StoreSel,
    f: impl FnOnce(&Store) -> AppResult<R>,
) -> AppResult<R> {
    core.with_store(sel.kind(), f)
}

/// Required, trimmed, length-limited text.
pub fn required_text(value: &str, what: &str, max_chars: usize) -> AppResult<String> {
    let v = value.trim();
    if v.is_empty() {
        return Err(AppError::required(what));
    }
    if v.chars().count() > max_chars {
        return Err(AppError::invalid_input(format!(
            "{what} is too long (maximum {max_chars} characters)."
        )));
    }
    Ok(v.to_string())
}

/// Optional text: trimmed, empty → None, length-limited.
pub fn optional_text(
    value: Option<String>,
    what: &str,
    max_chars: usize,
) -> AppResult<Option<String>> {
    match value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        Some(v) if v.chars().count() > max_chars => Err(AppError::invalid_input(format!(
            "{what} is too long (maximum {max_chars} characters)."
        ))),
        other => Ok(other),
    }
}

/// Body text (notes, screenplay text): preserved as typed except for a size cap.
pub fn body_text(value: String, what: &str, max_bytes: usize) -> AppResult<String> {
    if value.len() > max_bytes {
        return Err(AppError::invalid_input(format!("{what} is too long.")));
    }
    Ok(value)
}

pub fn require_id(value: &str, what: &str) -> AppResult<()> {
    if openframe_domain::ids::is_valid_id(value) {
        Ok(())
    } else {
        Err(AppError::not_found(what))
    }
}

// ------------------------------------------------------------------ assets

/// Largest single file OpenFrame will copy into a project in one operation.
pub const MAX_MANAGED_FILE_BYTES: u64 = 8 << 30;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub id: String,
    /// "managed" (stored in the project) or "external" (linked file).
    pub storage_mode: String,
    pub original_name: String,
    pub media_type: String,
    #[ts(type = "number | null")]
    pub byte_size: Option<i64>,
    #[ts(type = "number | null")]
    pub width: Option<i64>,
    #[ts(type = "number | null")]
    pub height: Option<i64>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    /// Absolute path for display through the restricted asset protocol.
    pub path: Option<String>,
    /// False when an external file (or a managed file) is missing.
    pub available: bool,
}

pub fn media_type_for(name: &str) -> &'static str {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "heic" => "image/heic",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" | "pptx" => "application/vnd.ms-powerpoint",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "rtf" => "application/rtf",
        "fdx" => "application/xml",
        "fountain" => "text/plain",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" | "oga" => "audio/ogg",
        "webm" => "audio/webm",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        _ => "application/octet-stream",
    }
}

// ------------------------------------------------------------------ user-chosen paths

/// Folders OpenFrame owns. User-chosen outputs are never written inside them
/// (Security review 2026-09-30, PATH-02/PATH-04).
pub fn protected_roots(core: &AppCore) -> Vec<PathBuf> {
    let mut v = vec![
        core.config.app_data_dir.clone(),
        core.config.global_vault_dir.clone(),
    ];
    if let Some(p) = core.project_opt() {
        v.push(p.layout.root().to_path_buf());
    }
    v
}

/// Final check for a user-chosen output file (export, package, file copy) after the caller
/// normalised its extension: well-formed local or network path, no device/ADS/reserved
/// names, existing folder, not inside a protected folder. The path is remembered so the
/// UI may later reveal it in Explorer (`of_reveal_path` kind `exported`).
pub fn check_output_file(core: &AppCore, path: &Path) -> AppResult<()> {
    let roots = protected_roots(core);
    let refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    openframe_security::validate_output_file(path, &refs)?;
    remember_output(core, path);
    Ok(())
}

/// A folder chosen to receive a new project (create / open package as copy).
pub fn check_project_parent(core: &AppCore, dir: &Path) -> AppResult<()> {
    let roots = protected_roots(core);
    let refs: Vec<&Path> = roots.iter().map(PathBuf::as_path).collect();
    openframe_security::validate_output_dir(dir, &refs)
}

/// Paths OpenFrame wrote for the user in this session (exports, packages, copies).
#[derive(Default)]
struct OutputRegistry(parking_lot::Mutex<std::collections::HashSet<String>>);

fn output_key(path: &Path) -> String {
    openframe_security::display_path(path)
        .to_string_lossy()
        .to_lowercase()
}

pub fn remember_output(core: &AppCore, path: &Path) {
    let reg = core.service(OutputRegistry::default);
    let mut set = reg.0.lock();
    if set.len() < 10_000 {
        set.insert(output_key(path));
    }
}

/// True if `path` is a file (or its folder) that OpenFrame wrote for the user this session.
/// `of_reveal_path` only reveals such paths — never arbitrary locations from the webview.
pub fn is_remembered_output(core: &AppCore, path: &Path) -> bool {
    let Some(reg) = core.existing_service::<OutputRegistry>() else {
        return false;
    };
    let set = reg.0.lock();
    set.contains(&output_key(path))
}

fn row_to_info(root: &Path, r: &rusqlite::Row<'_>) -> rusqlite::Result<AssetInfo> {
    let mode: String = r.get(1)?;
    let rel: Option<String> = r.get(2)?;
    let ext: Option<String> = r.get(3)?;
    let path = if mode == "managed" {
        rel.as_deref()
            .and_then(|p| openframe_security::validate_relative(p).ok())
            .map(|p| root.join(p))
    } else {
        ext.map(PathBuf::from)
    };
    // Linked (external) paths are content — possibly from a received package. Only
    // well-formed local-disk paths are ever probed: a UNC path would make Windows connect
    // to that server (and send the user's NTLM hash) just by listing files (PKG-01).
    let available = path
        .as_ref()
        .map(|p| openframe_security::is_local_disk_path(p) && p.is_file())
        .unwrap_or(false);
    Ok(AssetInfo {
        id: r.get(0)?,
        storage_mode: mode,
        original_name: r.get(4)?,
        media_type: r.get(5)?,
        byte_size: r.get(6)?,
        width: r.get(7)?,
        height: r.get(8)?,
        duration_ms: r.get(9)?,
        path: path.map(|p| p.to_string_lossy().into_owned()),
        available,
    })
}

const ASSET_COLS: &str = "id, storage_mode, rel_path, external_path, original_name, media_type, byte_size, width, height, duration_ms";

pub fn load_asset(conn: &Connection, root: &Path, id: &str) -> AppResult<AssetInfo> {
    conn.query_row(
        &format!("SELECT {ASSET_COLS} FROM asset WHERE id=?1"),
        [id],
        |r| row_to_info(root, r),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("file"))
}

pub fn load_asset_opt(
    conn: &Connection,
    root: &Path,
    id: Option<&str>,
) -> AppResult<Option<AssetInfo>> {
    match id {
        Some(id) => Ok(conn
            .query_row(
                &format!("SELECT {ASSET_COLS} FROM asset WHERE id=?1"),
                [id],
                |r| row_to_info(root, r),
            )
            .optional()?),
        None => Ok(None),
    }
}

/// Absolute path of an asset's bytes (managed or external).
pub fn asset_file_path(conn: &Connection, root: &Path, id: &str) -> AppResult<PathBuf> {
    let (mode, rel, ext): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT storage_mode, rel_path, external_path FROM asset WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("file"))?;
    if mode == "managed" {
        openframe_security::confine(root, rel.as_deref().unwrap_or(""))
    } else {
        let p = PathBuf::from(ext.unwrap_or_default());
        if !openframe_security::is_local_disk_path(&p) {
            return Err(AppError::new(
                "not_found.file",
                "This linked file isn't available on this computer. Relink it to a file on this computer.",
            )
            .with_detail("stored external path is not a local-disk path"));
        }
        Ok(p)
    }
}

fn image_dimensions(path: &Path, media_type: &str) -> (Option<i64>, Option<i64>) {
    if !media_type.starts_with("image/") || media_type == "image/svg+xml" {
        return (None, None);
    }
    // Read only the header; never decode full images on the command thread.
    match imagesize(path) {
        Some((w, h)) => (Some(w as i64), Some(h as i64)),
        None => (None, None),
    }
}

/// Minimal header-only PNG/JPEG/GIF/BMP/WebP dimension reader.
fn imagesize(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 64 * 1024];
    let n = f.read(&mut buf).ok()?;
    let b = &buf[..n];
    if b.len() >= 24 && &b[..8] == b"\x89PNG\r\n\x1a\n" {
        return Some((
            u32::from_be_bytes(b[16..20].try_into().ok()?),
            u32::from_be_bytes(b[20..24].try_into().ok()?),
        ));
    }
    if b.len() >= 10 && (&b[..6] == b"GIF87a" || &b[..6] == b"GIF89a") {
        return Some((
            u16::from_le_bytes([b[6], b[7]]) as u32,
            u16::from_le_bytes([b[8], b[9]]) as u32,
        ));
    }
    if b.len() >= 26 && &b[..2] == b"BM" {
        let w = i32::from_le_bytes(b[18..22].try_into().ok()?);
        let h = i32::from_le_bytes(b[22..26].try_into().ok()?);
        return Some((w.unsigned_abs(), h.unsigned_abs()));
    }
    if b.len() >= 30 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        match &b[12..16] {
            b"VP8X" => {
                let w = 1 + (b[24] as u32 | (b[25] as u32) << 8 | (b[26] as u32) << 16);
                let h = 1 + (b[27] as u32 | (b[28] as u32) << 8 | (b[29] as u32) << 16);
                return Some((w, h));
            }
            b"VP8 " if b.len() >= 30 => {
                let w = u16::from_le_bytes([b[26], b[27]]) as u32 & 0x3fff;
                let h = u16::from_le_bytes([b[28], b[29]]) as u32 & 0x3fff;
                return Some((w, h));
            }
            b"VP8L" if b.len() >= 25 => {
                let bits = u32::from_le_bytes(b[21..25].try_into().ok()?);
                return Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1));
            }
            _ => return None,
        }
    }
    if b.len() >= 4 && b[0] == 0xFF && b[1] == 0xD8 {
        let mut i = 2;
        while i + 9 < b.len() {
            if b[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = b[i + 1];
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC
            {
                let h = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
                let w = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
                return Some((w, h));
            }
            i += 2 + len;
        }
    }
    None
}

fn insert_asset(
    tx: &Tx<'_>,
    id: &str,
    mode: &str,
    rel: Option<&str>,
    external: Option<&str>,
    original_name: &str,
    media_type: &str,
    size: Option<i64>,
    sha: Option<&str>,
    dims: (Option<i64>, Option<i64>),
) -> AppResult<()> {
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO asset(id, storage_mode, rel_path, external_path, original_name, media_type, byte_size, sha256,
                           width, height, last_seen_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11, ?11)",
        params![id, mode, rel, external, original_name, media_type, size, sha, dims.0, dims.1, now],
    )?;
    Ok(())
}

/// Copy a user-chosen file into the store's managed assets (FSD §40, ESD §9).
/// The copy is removed automatically if the surrounding transaction rolls back.
pub fn ingest_file(tx: &Tx<'_>, source: &Path) -> AppResult<AssetInfo> {
    let size = openframe_security::validate_input_file(source, MAX_MANAGED_FILE_BYTES)?;
    let original_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let media_type = media_type_for(&original_name);
    let id = new_id();
    let rel = ProjectLayout::new_asset_rel(&id, source.extension().and_then(|e| e.to_str()));
    let dest = openframe_security::confine(tx.root(), &rel)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("importing");
    tx.created_file(tmp.clone());
    tx.created_file(dest.clone());
    std::fs::copy(source, &tmp)?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&tmp)?
        .sync_all()?;
    std::fs::rename(&tmp, &dest)?;
    let sha = openframe_security::sha256_file(&dest)?;
    let dims = image_dimensions(&dest, media_type);
    insert_asset(
        tx,
        &id,
        "managed",
        Some(&rel),
        None,
        &original_name,
        media_type,
        Some(size as i64),
        Some(&sha),
        dims,
    )?;
    load_asset(tx.conn(), tx.root(), &id)
}

/// Store raw bytes (voice note recording, sketch, pasted image) as a managed asset.
pub fn ingest_bytes(
    tx: &Tx<'_>,
    bytes: &[u8],
    original_name: &str,
    media_type: Option<&str>,
) -> AppResult<AssetInfo> {
    if bytes.len() as u64 > MAX_MANAGED_FILE_BYTES {
        return Err(AppError::invalid_input("That file is too large."));
    }
    let safe_name = openframe_security::sanitize_file_name(original_name);
    let media_type = media_type
        .map(|m| m.to_string())
        .unwrap_or_else(|| media_type_for(&safe_name).to_string());
    let id = new_id();
    let ext = Path::new(&safe_name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_string());
    let rel = ProjectLayout::new_asset_rel(&id, ext.as_deref());
    let dest = openframe_security::confine(tx.root(), &rel)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    tx.created_file(dest.clone());
    openframe_project_format::atomic_write(&dest, bytes)?;
    let sha = openframe_security::sha256_bytes(bytes);
    let dims = image_dimensions(&dest, &media_type);
    insert_asset(
        tx,
        &id,
        "managed",
        Some(&rel),
        None,
        &safe_name,
        &media_type,
        Some(bytes.len() as i64),
        Some(&sha),
        dims,
    )?;
    load_asset(tx.conn(), tx.root(), &id)
}

/// Validate a file the user wants to *link* (add as link, relink): a regular file on a
/// local drive, resolved through links; returns its canonical display path and size.
/// Network locations are refused for links: a stored UNC path would be probed on every
/// listing, and linked files travel as content in packages (PKG-01).
pub fn external_link_target(source: &Path) -> AppResult<(PathBuf, u64)> {
    openframe_security::check_user_path(source, openframe_security::NetworkPaths::Refuse)?;
    let len = openframe_security::validate_input_file(source, u64::MAX)?;
    let abs = openframe_security::display_path(&source.canonicalize()?);
    openframe_security::check_user_path(&abs, openframe_security::NetworkPaths::Refuse)?;
    Ok((abs, len))
}

/// Record a link to an external file without copying it (FSD §40.4).
pub fn reference_external(tx: &Tx<'_>, source: &Path) -> AppResult<AssetInfo> {
    let (abs, len) = external_link_target(source)?;
    let abs_str = abs.to_string_lossy().into_owned();
    let meta_len = len;
    let original_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let media_type = media_type_for(&original_name);
    let id = new_id();
    let dims = image_dimensions(&abs, media_type);
    insert_asset(
        tx,
        &id,
        "external",
        None,
        Some(&abs_str),
        &original_name,
        media_type,
        Some(meta_len as i64),
        None,
        dims,
    )?;
    load_asset(tx.conn(), tx.root(), &id)
}

/// Copy an asset row + bytes from one store to another (Global ↔ Project copies).
/// The copy is independent: a new identity and its own managed file.
pub fn copy_asset_between(
    src_conn: &Connection,
    src_root: &Path,
    asset_id: &str,
    dest: &Tx<'_>,
) -> AppResult<AssetInfo> {
    let info = load_asset(src_conn, src_root, asset_id)?;
    if info.storage_mode == "external" {
        let p = PathBuf::from(info.path.clone().unwrap_or_default());
        return reference_external(dest, &p);
    }
    let path = asset_file_path(src_conn, src_root, asset_id)?;
    if !path.is_file() {
        return Err(AppError::new(
            "not_found.file",
            "The original file is missing, so it could not be copied.",
        ));
    }
    let copied = ingest_file(dest, &path)?;
    // Keep the original human-facing name rather than the internal storage name.
    dest.conn().execute(
        "UPDATE asset SET original_name=?1 WHERE id=?2",
        params![info.original_name, copied.id],
    )?;
    load_asset(dest.conn(), dest.root(), &copied.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_types() {
        assert_eq!(media_type_for("a.JPG"), "image/jpeg");
        assert_eq!(media_type_for("notes.fdx"), "application/xml");
        assert_eq!(media_type_for("noext"), "application/octet-stream");
    }

    #[test]
    fn png_header_dimensions() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.png");
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        std::fs::write(&p, &png).unwrap();
        assert_eq!(imagesize(&p), Some((640, 480)));
    }

    #[test]
    fn text_validation() {
        assert!(required_text("  ", "Title", 10).is_err());
        assert_eq!(required_text(" A ", "Title", 10).unwrap(), "A");
        assert!(required_text("abcdefghijk", "Title", 10).is_err());
        assert_eq!(optional_text(Some("  ".into()), "x", 5).unwrap(), None);
    }
}
