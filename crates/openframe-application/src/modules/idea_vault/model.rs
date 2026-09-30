//! Vault read models: item DTOs, generated display names, list views, overview.

use std::collections::HashMap;
use std::path::Path;

use openframe_domain::enums::VaultItemType;
use openframe_domain::{Actor, AppError, AppResult};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params_from_iter};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::read_vault;
use super::search;
use crate::core::AppCore;
use crate::util::{AssetInfo, StoreSel, load_asset_opt};

/// Most items returned by the time-ordered "Recently …" views.
pub const RECENT_LIMIT: i64 = 100;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultItemDto {
    pub id: String,
    pub store: StoreSel,
    pub item_type: VaultItemType,
    /// The user's own title. None when the item is untitled (valid, FSD §5.4).
    pub title: Option<String>,
    /// Generated local name for navigation; never stored, never user content.
    pub display_name: String,
    pub body: Option<String>,
    pub caption: Option<String>,
    /// URL of a URL item, or the optional "Source link" of any other item.
    pub url: Option<String>,
    /// Optional source of a quote ("overheard, bus stand").
    pub source_text: Option<String>,
    pub asset: Option<AssetInfo>,
    pub folder_id: Option<String>,
    pub collection_ids: Vec<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    /// Informational: this project item was copied from that Global item (no sync).
    pub source_global_item_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
    /// For search results: which fields matched, e.g. "Matched in caption & tag".
    pub match_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultFolderDto {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    #[ts(type = "number")]
    pub item_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultCollectionDto {
    pub id: String,
    pub name: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub item_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultTagCount {
    pub tag: String,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultOverview {
    pub store: StoreSel,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub pinned: i64,
    /// Items that are not in any folder (Folder view top level).
    #[ts(type = "number")]
    pub unfiled: i64,
    pub folders: Vec<VaultFolderDto>,
    pub collections: Vec<VaultCollectionDto>,
    pub tags: Vec<VaultTagCount>,
}

/// Which slice of the vault to show (left panel selection).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum VaultView {
    All,
    Pinned,
    RecentlyAdded,
    RecentlyModified,
    /// Items directly inside a folder; `folderId: null` = items in no folder.
    #[serde(rename_all = "camelCase")]
    Folder {
        #[serde(default)]
        folder_id: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Collection {
        collection_id: String,
    },
    Tag {
        tag: String,
    },
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultListArgs {
    #[serde(default)]
    pub store: StoreSel,
    #[serde(default)]
    pub view: Option<VaultView>,
    /// Vault search (FSD §5.9): titles, note text, captions, filenames, tags.
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultOverviewArgs {
    #[serde(default)]
    pub store: StoreSel,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultGetArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
}

// ------------------------------------------------------------------ helpers

/// UI label for an item type (mock 024 meta lines, mock 047).
pub fn type_label(t: VaultItemType) -> &'static str {
    match t {
        VaultItemType::Note => "Note",
        VaultItemType::Image => "Image",
        VaultItemType::Url => "URL",
        VaultItemType::Pdf => "PDF",
        VaultItemType::Document => "Document",
        VaultItemType::Audio => "Audio",
        VaultItemType::Voice => "Voice note",
        VaultItemType::Video => "Video",
        VaultItemType::Sketch => "Sketch",
        VaultItemType::Quote => "Quote",
        VaultItemType::Screenshot => "Screenshot",
        VaultItemType::File => "File",
    }
}

/// Item types whose content is a stored file.
pub fn is_file_type(t: VaultItemType) -> bool {
    !matches!(
        t,
        VaultItemType::Note | VaultItemType::Url | VaultItemType::Quote
    )
}

/// Classify an added file without asking the user (FSD §5.1: "Add", not "classify").
pub fn detect_type(name: &str, media_type: &str) -> VaultItemType {
    let lower = name.to_ascii_lowercase();
    let ext = Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    if media_type.starts_with("image/") {
        if lower.starts_with("screenshot")
            || lower.starts_with("screen shot")
            || lower.contains("screen shot")
        {
            return VaultItemType::Screenshot;
        }
        return VaultItemType::Image;
    }
    if media_type.starts_with("audio/") {
        return VaultItemType::Audio;
    }
    if media_type.starts_with("video/") {
        return VaultItemType::Video;
    }
    if media_type == "application/pdf" {
        return VaultItemType::Pdf;
    }
    const DOCS: &[&str] = &[
        "doc", "docx", "odt", "rtf", "txt", "md", "pages", "xls", "xlsx", "ods", "csv", "numbers",
        "ppt", "pptx", "odp", "key", "fdx", "fountain", "celtx", "highland",
    ];
    if DOCS.contains(&ext.as_str()) {
        return VaultItemType::Document;
    }
    VaultItemType::File
}

/// "0:42", "12:05", "1:02:03".
pub fn format_duration(ms: i64) -> String {
    let total = (ms.max(0) + 500) / 1000;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn first_line(text: &str, max_chars: usize) -> Option<String> {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let mut out: String = line.chars().take(max_chars).collect();
    if line.chars().count() > max_chars {
        out = out.trim_end().to_string();
        out.push('…');
    }
    Some(out)
}

/// Generated local display name (FSD §5.4). Derived from the item's own content
/// so it adds nothing searchable that the user did not write; never stored.
pub fn display_name(
    item_type: VaultItemType,
    title: Option<&str>,
    body: Option<&str>,
    caption: Option<&str>,
    url: Option<&str>,
    asset: Option<&AssetInfo>,
) -> String {
    if let Some(t) = title.map(str::trim).filter(|t| !t.is_empty()) {
        return t.to_string();
    }
    let from_text = |v: Option<&str>| v.and_then(|b| first_line(b, 80));
    match item_type {
        VaultItemType::Note => from_text(body).unwrap_or_else(|| "Untitled note".into()),
        VaultItemType::Quote => from_text(body)
            .map(|q| format!("“{q}”"))
            .unwrap_or_else(|| "Untitled quote".into()),
        VaultItemType::Url => from_text(url)
            .or_else(|| from_text(body))
            .unwrap_or_else(|| "Untitled link".into()),
        VaultItemType::Voice => {
            let dur = asset.and_then(|a| a.duration_ms).map(format_duration);
            from_text(caption).unwrap_or_else(|| match dur {
                Some(d) => format!("Voice note {d}"),
                None => "Voice note".into(),
            })
        }
        VaultItemType::Sketch => from_text(caption).unwrap_or_else(|| "Sketch".into()),
        _ => asset
            .map(|a| a.original_name.clone())
            .filter(|n| !n.trim().is_empty())
            .or_else(|| from_text(caption))
            .unwrap_or_else(|| format!("Untitled {}", type_label(item_type).to_lowercase())),
    }
}

// ------------------------------------------------------------------ loading

struct RawItem {
    id: String,
    item_type: String,
    title: Option<String>,
    body: Option<String>,
    caption: Option<String>,
    url: Option<String>,
    source_text: Option<String>,
    asset_id: Option<String>,
    folder_id: Option<String>,
    pinned: bool,
    source_global_item_id: Option<String>,
    created_at: i64,
    updated_at: i64,
    rev: i64,
}

const ITEM_COLS: &str =
    "i.id, i.item_type, i.title, i.body, i.caption, i.url, i.source_text, i.asset_id, i.folder_id,
     i.pinned, i.source_global_item_id, i.created_at, i.updated_at, i.rev";

fn raw(r: &rusqlite::Row<'_>) -> rusqlite::Result<RawItem> {
    Ok(RawItem {
        id: r.get(0)?,
        item_type: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        caption: r.get(4)?,
        url: r.get(5)?,
        source_text: r.get(6)?,
        asset_id: r.get(7)?,
        folder_id: r.get(8)?,
        pinned: r.get::<_, i64>(9)? != 0,
        source_global_item_id: r.get(10)?,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
        rev: r.get(13)?,
    })
}

fn tags_by_item(c: &Connection, only: Option<&str>) -> AppResult<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let (sql, p): (&str, Vec<SqlValue>) = match only {
        Some(id) => (
            "SELECT item_id, tag FROM vault_item_tag WHERE item_id=?1 ORDER BY tag",
            vec![SqlValue::Text(id.into())],
        ),
        None => (
            "SELECT item_id, tag FROM vault_item_tag ORDER BY tag",
            vec![],
        ),
    };
    let mut stmt = c.prepare(sql)?;
    let rows = stmt.query_map(params_from_iter(p), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (item, tag) = row?;
        map.entry(item).or_default().push(tag);
    }
    Ok(map)
}

fn collections_by_item(
    c: &Connection,
    only: Option<&str>,
) -> AppResult<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let base = "SELECT m.item_id, m.collection_id FROM vault_item_collection m
                JOIN vault_collection c ON c.id = m.collection_id AND c.deleted_at IS NULL";
    let (sql, p): (String, Vec<SqlValue>) = match only {
        Some(id) => (
            format!("{base} WHERE m.item_id=?1 ORDER BY c.position"),
            vec![SqlValue::Text(id.into())],
        ),
        None => (format!("{base} ORDER BY c.position"), vec![]),
    };
    let mut stmt = c.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(p), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (item, coll) = row?;
        map.entry(item).or_default().push(coll);
    }
    Ok(map)
}

fn to_dto(
    c: &Connection,
    root: &Path,
    store: StoreSel,
    r: RawItem,
    tags: &mut HashMap<String, Vec<String>>,
    colls: &mut HashMap<String, Vec<String>>,
) -> AppResult<VaultItemDto> {
    let item_type = VaultItemType::parse(&r.item_type).unwrap_or(VaultItemType::File);
    let asset = load_asset_opt(c, root, r.asset_id.as_deref())?;
    let display = display_name(
        item_type,
        r.title.as_deref(),
        r.body.as_deref(),
        r.caption.as_deref(),
        r.url.as_deref(),
        asset.as_ref(),
    );
    Ok(VaultItemDto {
        tags: tags.remove(&r.id).unwrap_or_default(),
        collection_ids: colls.remove(&r.id).unwrap_or_default(),
        id: r.id,
        store,
        item_type,
        title: r.title,
        display_name: display,
        body: r.body,
        caption: r.caption,
        url: r.url,
        source_text: r.source_text,
        asset,
        folder_id: r.folder_id,
        pinned: r.pinned,
        source_global_item_id: r.source_global_item_id,
        created_at: r.created_at,
        updated_at: r.updated_at,
        rev: r.rev,
        match_reason: None,
    })
}

/// Load live (not deleted) items matching `where_sql` (`i.` alias, `?1..` params).
pub(crate) fn load_items(
    c: &Connection,
    root: &Path,
    store: StoreSel,
    where_sql: &str,
    params: Vec<SqlValue>,
    order_sql: &str,
    limit: Option<i64>,
) -> AppResult<Vec<VaultItemDto>> {
    let limit_sql = limit.map(|l| format!(" LIMIT {l}")).unwrap_or_default();
    let sql = format!(
        "SELECT {ITEM_COLS} FROM vault_item i WHERE i.deleted_at IS NULL AND ({where_sql}) ORDER BY {order_sql}{limit_sql}"
    );
    let rows: Vec<RawItem> = {
        let mut stmt = c.prepare(&sql)?;
        stmt.query_map(params_from_iter(params), raw)?
            .collect::<Result<_, _>>()?
    };
    let mut tags = tags_by_item(c, None)?;
    let mut colls = collections_by_item(c, None)?;
    rows.into_iter()
        .map(|r| to_dto(c, root, store, r, &mut tags, &mut colls))
        .collect()
}

/// Load one live item.
pub(crate) fn load_item(
    c: &Connection,
    root: &Path,
    store: StoreSel,
    id: &str,
) -> AppResult<VaultItemDto> {
    let row = c
        .query_row(
            &format!("SELECT {ITEM_COLS} FROM vault_item i WHERE i.id=?1 AND i.deleted_at IS NULL"),
            [id],
            raw,
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("Idea Vault item"))?;
    let mut tags = tags_by_item(c, Some(id))?;
    let mut colls = collections_by_item(c, Some(id))?;
    to_dto(c, root, store, row, &mut tags, &mut colls)
}

pub(crate) fn view_sql(view: &VaultView) -> (String, Vec<SqlValue>, &'static str, Option<i64>) {
    match view {
        VaultView::All => ("1=1".into(), vec![], "i.created_at DESC, i.id DESC", None),
        VaultView::Pinned => ("i.pinned = 1".into(), vec![], "i.pinned_at DESC, i.created_at DESC", None),
        VaultView::RecentlyAdded => ("1=1".into(), vec![], "i.created_at DESC, i.id DESC", Some(RECENT_LIMIT)),
        VaultView::RecentlyModified => ("1=1".into(), vec![], "i.updated_at DESC, i.id DESC", Some(RECENT_LIMIT)),
        VaultView::Folder { folder_id } => (
            "i.folder_id IS ?1".into(),
            vec![folder_id.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null)],
            "i.created_at DESC, i.id DESC",
            None,
        ),
        VaultView::Collection { collection_id } => (
            "EXISTS (SELECT 1 FROM vault_item_collection m WHERE m.item_id = i.id AND m.collection_id = ?1)".into(),
            vec![SqlValue::Text(collection_id.clone())],
            "i.created_at DESC, i.id DESC",
            None,
        ),
        VaultView::Tag { tag } => (
            "EXISTS (SELECT 1 FROM vault_item_tag t WHERE t.item_id = i.id AND t.tag = ?1)".into(),
            vec![SqlValue::Text(tag.trim().trim_start_matches('#').to_string())],
            "i.created_at DESC, i.id DESC",
            None,
        ),
    }
}

// ------------------------------------------------------------------- queries

pub(crate) fn list(
    core: &AppCore,
    actor: &Actor,
    args: VaultListArgs,
) -> AppResult<Vec<VaultItemDto>> {
    let view = args.view.unwrap_or(VaultView::All);
    let text = args.search.unwrap_or_default();
    if text.chars().count() > 500 {
        return Err(AppError::invalid_input("That search is too long."));
    }
    read_vault(core, actor, args.store, |c, root| {
        let (w, p, order, limit) = view_sql(&view);
        if text.trim().is_empty() {
            return load_items(c, root, args.store, &w, p, order, limit);
        }
        let ranked = search::ranked_ids(c, &text)?;
        if ranked.is_empty() {
            return Ok(vec![]);
        }
        let rank: HashMap<&str, usize> = ranked
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();
        let mut items: Vec<VaultItemDto> = load_items(c, root, args.store, &w, p, order, None)?
            .into_iter()
            .filter(|i| rank.contains_key(i.id.as_str()))
            .collect();
        items.sort_by_key(|i| rank.get(i.id.as_str()).copied().unwrap_or(usize::MAX));
        for item in &mut items {
            item.match_reason = search::match_reason(item, &text);
        }
        Ok(items)
    })
}

pub(crate) fn get(core: &AppCore, actor: &Actor, args: VaultGetArgs) -> AppResult<VaultItemDto> {
    read_vault(core, actor, args.store, |c, root| {
        load_item(c, root, args.store, &args.id)
    })
}

pub(crate) fn overview(
    core: &AppCore,
    actor: &Actor,
    args: VaultOverviewArgs,
) -> AppResult<VaultOverview> {
    read_vault(core, actor, args.store, |c, _| {
        let (total, pinned, unfiled): (i64, i64, i64) = c.query_row(
            "SELECT count(*), COALESCE(sum(pinned), 0), COALESCE(sum(folder_id IS NULL), 0)
             FROM vault_item WHERE deleted_at IS NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let folders = {
            let mut stmt = c.prepare(
                "SELECT f.id, f.name, f.parent_id, f.rev,
                        (SELECT count(*) FROM vault_item i WHERE i.folder_id = f.id AND i.deleted_at IS NULL)
                 FROM vault_folder f WHERE f.deleted_at IS NULL ORDER BY f.position, f.name COLLATE NOCASE",
            )?;
            stmt.query_map([], |r| {
                Ok(VaultFolderDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    parent_id: r.get(2)?,
                    rev: r.get(3)?,
                    item_count: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        let collections = {
            let mut stmt = c.prepare(
                "SELECT c.id, c.name, c.note, c.rev,
                        (SELECT count(*) FROM vault_item_collection m JOIN vault_item i ON i.id = m.item_id
                         WHERE m.collection_id = c.id AND i.deleted_at IS NULL)
                 FROM vault_collection c WHERE c.deleted_at IS NULL ORDER BY c.position, c.name COLLATE NOCASE",
            )?;
            stmt.query_map([], |r| {
                Ok(VaultCollectionDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    note: r.get(2)?,
                    rev: r.get(3)?,
                    item_count: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        let tags = {
            let mut stmt = c.prepare(
                "SELECT min(t.tag), count(*) FROM vault_item_tag t JOIN vault_item i ON i.id = t.item_id
                 WHERE i.deleted_at IS NULL GROUP BY t.tag COLLATE NOCASE ORDER BY min(t.tag) COLLATE NOCASE",
            )?;
            stmt.query_map([], |r| {
                Ok(VaultTagCount {
                    tag: r.get(0)?,
                    count: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        Ok(VaultOverview {
            store: args.store,
            total,
            pinned,
            unfiled,
            folders,
            collections,
            tags,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, dur: Option<i64>) -> AssetInfo {
        AssetInfo {
            id: "a".into(),
            storage_mode: "managed".into(),
            original_name: name.into(),
            media_type: "x".into(),
            byte_size: None,
            width: None,
            height: None,
            duration_ms: dur,
            path: None,
            available: true,
        }
    }

    #[test]
    fn generated_names_come_from_content_and_titles_win() {
        use VaultItemType as T;
        assert_eq!(
            display_name(T::Note, None, None, None, None, None),
            "Untitled note"
        );
        assert_eq!(
            display_name(
                T::Note,
                Some("  "),
                Some("\n  First idea\nmore"),
                None,
                None,
                None
            ),
            "First idea"
        );
        assert_eq!(
            display_name(T::Note, Some("Core concept"), Some("x"), None, None, None),
            "Core concept"
        );
        assert_eq!(
            display_name(T::Quote, None, Some("Rain moves it"), None, None, None),
            "“Rain moves it”"
        );
        assert_eq!(
            display_name(
                T::Image,
                None,
                None,
                None,
                None,
                Some(&asset("rain.jpg", None))
            ),
            "rain.jpg"
        );
        assert_eq!(
            display_name(
                T::Voice,
                None,
                None,
                None,
                None,
                Some(&asset("v.webm", Some(42_000)))
            ),
            "Voice note 0:42"
        );
        assert_eq!(
            display_name(T::Url, None, None, None, Some("https://a.example/x"), None),
            "https://a.example/x"
        );
        let long = "a".repeat(200);
        assert!(display_name(T::Note, None, Some(&long), None, None, None).ends_with('…'));
    }

    #[test]
    fn file_types_are_detected_without_asking() {
        use VaultItemType as T;
        assert_eq!(
            detect_type("Screenshot 2024-05-01.png", "image/png"),
            T::Screenshot
        );
        assert_eq!(detect_type("rain.jpg", "image/jpeg"), T::Image);
        assert_eq!(detect_type("a.pdf", "application/pdf"), T::Pdf);
        assert_eq!(detect_type("notes.docx", "application/x"), T::Document);
        assert_eq!(detect_type("song.mp3", "audio/mpeg"), T::Audio);
        assert_eq!(detect_type("chase.mp4", "video/mp4"), T::Video);
        assert_eq!(
            detect_type("model.blend", "application/octet-stream"),
            T::File
        );
    }

    #[test]
    fn durations() {
        assert_eq!(format_duration(42_000), "0:42");
        assert_eq!(format_duration(725_000), "12:05");
        assert_eq!(format_duration(3_723_000), "1:02:03");
    }
}
