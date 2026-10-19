//! Small helpers shared by the visual planning submodules: image intake,
//! patch-style text fields, derived shot letters and the guarded Idea Vault
//! lookup (the vault module owns `vault_item`; we only read its asset link).

use std::path::PathBuf;

use base64::Engine;
use openframe_domain::{AppError, AppResult};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use crate::store::Tx;
use crate::util::{AssetInfo, ingest_bytes, ingest_file, load_asset, optional_text};

/// Largest image accepted as inline data (sketches, pasted images).
pub const MAX_INLINE_IMAGE_BYTES: usize = 25 << 20;

pub const NAME_MAX: usize = 120;
pub const SHORT_MAX: usize = 200;
pub const TEXT_MAX: usize = 4_000;

/// Image formats a visual tile/panel can display.
pub fn is_displayable_image(media_type: &str) -> bool {
    matches!(
        media_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp" | "image/svg+xml"
    )
}

/// Copy a user-chosen image file into the project (managed copy).
pub fn ingest_image_file(tx: &Tx<'_>, path: &str) -> AppResult<AssetInfo> {
    let p = PathBuf::from(path);
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    if !is_displayable_image(crate::util::media_type_for(&name)) {
        return Err(AppError::invalid_input(
            "That file isn't an image OpenFrame can show. Choose a PNG, JPEG, GIF, WebP, BMP or SVG image.",
        ));
    }
    ingest_file(tx, &p)
}

/// Store an image sent as base64 data (a sketch drawn in OpenFrame or a pasted image).
pub fn ingest_image_data(tx: &Tx<'_>, data_base64: &str, file_name: &str) -> AppResult<AssetInfo> {
    let raw = data_base64
        .split_once(";base64,")
        .map(|(_, d)| d)
        .unwrap_or(data_base64);
    if raw.len() > MAX_INLINE_IMAGE_BYTES / 3 * 4 + 8 {
        return Err(AppError::invalid_input("That image is too large."));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|e| {
            AppError::invalid_input("The image data could not be read.").with_detail(e.to_string())
        })?;
    let media = sniff_image(&bytes).ok_or_else(|| {
        AppError::invalid_input(
            "The image data could not be read. Use a PNG, JPEG, GIF, WebP or BMP image.",
        )
    })?;
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    let stem = if stem.trim().is_empty() {
        "image"
    } else {
        stem.trim()
    };
    let name = format!("{stem}.{}", media.1);
    ingest_bytes(tx, &bytes, &name, Some(media.0))
}

/// Recognise an image by its magic bytes → (media type, extension).
pub fn sniff_image(b: &[u8]) -> Option<(&'static str, &'static str)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else if b.starts_with(b"BM") && b.len() > 26 {
        Some(("image/bmp", "bmp"))
    } else {
        None
    }
}

/// Patch semantics for optional text: `None` = leave unchanged; `Some("")` = clear.
pub fn patch_text(
    fields: &mut Vec<(&'static str, SqlValue)>,
    col: &'static str,
    value: Option<String>,
    what: &str,
    max: usize,
) -> AppResult<()> {
    if let Some(v) = value {
        let v = optional_text(Some(v), what, max)?;
        fields.push((col, v.map(SqlValue::Text).unwrap_or(SqlValue::Null)));
    }
    Ok(())
}

/// Shot letter from a zero-based order index: A…Z, AA, AB, … (never stored).
pub fn shot_letters(index: usize) -> String {
    let mut n = index + 1;
    let mut out = Vec::new();
    while n > 0 {
        let r = (n - 1) % 26;
        out.push((b'A' + r as u8) as char);
        n = (n - 1) / 26;
    }
    out.iter().rev().collect()
}

/// Normalise a user-entered link: add https:// when no scheme; only web links are allowed.
pub fn normalize_url(raw: &str) -> AppResult<String> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(AppError::required("Link"));
    }
    if v.chars().count() > 2_000 {
        return Err(AppError::invalid_input("That link is too long."));
    }
    if v.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AppError::invalid_input("A link can't contain spaces."));
    }
    let lower = v.to_ascii_lowercase();
    let url = if lower.starts_with("http://") || lower.starts_with("https://") {
        v.to_string()
    } else if lower.contains("://")
        || lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("file:")
    {
        return Err(AppError::invalid_input(
            "Only web links (http or https) can be added.",
        ));
    } else {
        format!("https://{v}")
    };
    let host = url
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if host.is_empty() {
        return Err(AppError::invalid_input(
            "That doesn't look like a web link.",
        ));
    }
    Ok(url)
}

/// Short host shown under a link tile, e.g. "imdb.com".
pub fn url_host(url: &str) -> String {
    url.split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(url)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string()
}

pub fn table_exists(c: &Connection, table: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |r| r.get(0),
    )?)
}

fn columns(c: &Connection, table: &str) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare("SELECT name FROM pragma_table_info(?1)")?;
    let cols = stmt
        .query_map([table], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cols)
}

/// An Idea Vault item that carries an image the user can place on a moodboard.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultImageRef {
    pub vault_item_id: String,
    pub title: String,
    pub asset: AssetInfo,
}

/// Guarded read of `vault_item` (owned by the Idea Vault module): only the id,
/// a title-like column and `asset_id` are used, and only if they exist.
pub fn vault_images(
    c: &Connection,
    root: &std::path::Path,
    limit: i64,
) -> AppResult<Vec<VaultImageRef>> {
    let Some(q) = vault_query(c)? else {
        return Ok(vec![]);
    };
    let sql = format!(
        "SELECT v.id, {title}, v.asset_id FROM vault_item v JOIN asset a ON a.id = v.asset_id
         WHERE {alive} AND a.media_type LIKE 'image/%' ORDER BY v.rowid DESC LIMIT ?1",
        title = q.title,
        alive = q.alive
    );
    let mut stmt = c.prepare(&sql)?;
    let rows = stmt
        .query_map([limit], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, title, asset_id) in rows {
        let asset = load_asset(c, root, &asset_id)?;
        let title = title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| asset.original_name.clone());
        out.push(VaultImageRef {
            vault_item_id: id,
            title,
            asset,
        });
    }
    Ok(out)
}

/// The asset id (and title) behind one vault item, if the vault exists and the item has an image.
pub fn vault_item_image(
    c: &Connection,
    vault_item_id: &str,
) -> AppResult<Option<(String, Option<String>)>> {
    let Some(q) = vault_query(c)? else {
        return Ok(None);
    };
    let sql = format!(
        "SELECT v.asset_id, {title} FROM vault_item v JOIN asset a ON a.id = v.asset_id
         WHERE v.id = ?1 AND {alive} AND a.media_type LIKE 'image/%'",
        title = q.title,
        alive = q.alive
    );
    Ok(
        c.query_row(&sql, [vault_item_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?,
    )
}

struct VaultQuery {
    title: &'static str,
    alive: &'static str,
}

fn vault_query(c: &Connection) -> AppResult<Option<VaultQuery>> {
    if !table_exists(c, "vault_item")? {
        return Ok(None);
    }
    let cols = columns(c, "vault_item")?;
    let has = |n: &str| cols.iter().any(|c| c == n);
    if !has("id") || !has("asset_id") {
        return Ok(None);
    }
    let title = if has("title") {
        "v.title"
    } else if has("name") {
        "v.name"
    } else {
        "NULL"
    };
    let alive = if has("deleted_at") {
        "v.deleted_at IS NULL"
    } else {
        "1=1"
    };
    Ok(Some(VaultQuery { title, alive }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_follow_order() {
        assert_eq!(shot_letters(0), "A");
        assert_eq!(shot_letters(3), "D");
        assert_eq!(shot_letters(25), "Z");
        assert_eq!(shot_letters(26), "AA");
        assert_eq!(shot_letters(27), "AB");
        assert_eq!(shot_letters(26 * 27), "AAA");
    }

    #[test]
    fn urls_are_web_only() {
        assert_eq!(
            normalize_url("imdb.com/title/x").unwrap(),
            "https://imdb.com/title/x"
        );
        assert_eq!(normalize_url(" https://a.b/c ").unwrap(), "https://a.b/c");
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("file:///c:/x").is_err());
        assert!(normalize_url("ftp://x").is_err());
        assert!(normalize_url("   ").is_err());
        assert_eq!(url_host("https://www.imdb.com/title"), "imdb.com");
    }

    #[test]
    fn sniffs_images() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n...").unwrap().0, "image/png");
        assert!(sniff_image(b"hello").is_none());
    }
}
