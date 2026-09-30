//! Vault item commands: create (any type, no required metadata), add files
//! (copy or link, partial success), recorded bytes (voice notes, sketches),
//! edit with autosave coalescing, pin, folder, collections, tags, delete, relink.

use std::path::PathBuf;

use base64::Engine as _;
use openframe_domain::enums::VaultItemType;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{int, opt_int, opt_text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::model::{VaultItemDto, detect_type, display_name, is_file_type, load_item, type_label};
use super::{mutate_vault, read_vault};
use crate::core::AppCore;
use crate::store::{DeleteSpec, MutationMeta, Tx, soft_delete};
use crate::util::{
    AssetInfo, StoreSel, body_text, ingest_bytes, ingest_file, load_asset_opt, optional_text,
    reference_external,
};

pub const MAX_TITLE: usize = 300;
pub const MAX_CAPTION: usize = 5_000;
pub const MAX_BODY_BYTES: usize = 2 << 20;
pub const MAX_URL: usize = 4_096;
pub const MAX_TAG: usize = 60;
/// Recorded voice notes / sketches sent from the webview.
pub const MAX_INGEST_BYTES: usize = 512 << 20;
pub const MAX_BATCH: usize = 1_000;

// ------------------------------------------------------------------ args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultCreateArgs {
    #[serde(default)]
    pub store: StoreSel,
    /// note | url | quote (file-based types are added with add_files / ingest_bytes).
    pub item_type: VaultItemType,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub source_text: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultAddFilesArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub paths: Vec<String>,
    /// "copy" (stored inside the vault, default) or "link" (external reference).
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
    /// Override the detected type (e.g. "sketch" for a scanned drawing).
    #[serde(default)]
    pub item_type: Option<VaultItemType>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultFailedFile {
    pub path: String,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultAddFilesResult {
    pub added: Vec<VaultItemDto>,
    pub failed: Vec<VaultFailedFile>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultIngestBytesArgs {
    #[serde(default)]
    pub store: StoreSel,
    /// voice | audio | sketch | image | screenshot | video | file
    pub item_type: VaultItemType,
    pub file_name: String,
    #[serde(default)]
    pub media_type: Option<String>,
    /// File content, base64 encoded (standard alphabet).
    pub data_base64: String,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub collection_id: Option<String>,
}

/// Partial edit: omitted fields are unchanged; an empty string clears a field.
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultUpdateArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub source_text: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultItemIdArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultIdsArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultSetPinnedArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
    pub pinned: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultMoveArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
    /// None = top level (no folder).
    #[serde(default)]
    pub folder_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultMembershipArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
    pub collection_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultTagsArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultRemoveTagArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub ids: Vec<String>,
    pub tag: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultRelinkArgs {
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
    pub path: String,
}

// ---------------------------------------------------------------- helpers

/// Normalise a user-entered link. Only web links are stored as URL items so they
/// can always be opened (the URL itself is authoritative, FSD §88.6).
pub fn normalize_url(raw: &str) -> AppResult<String> {
    let v = raw.trim();
    if v.is_empty() {
        return Err(AppError::required("Link"));
    }
    if v.chars().count() > MAX_URL || v.chars().any(char::is_whitespace) {
        return Err(AppError::invalid_input(
            "That doesn't look like a web link.",
        ));
    }
    let lower = v.to_ascii_lowercase();
    let with_scheme = if lower.starts_with("http://") || lower.starts_with("https://") {
        v.to_string()
    } else if lower.contains("://")
        || lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("file:")
    {
        return Err(AppError::invalid_input(
            "Only web links starting with http:// or https:// can be saved as URL items.",
        ));
    } else {
        format!("https://{v}")
    };
    let host = with_scheme
        .split("://")
        .nth(1)
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if host.is_empty() {
        return Err(AppError::invalid_input(
            "That doesn't look like a web link.",
        ));
    }
    Ok(with_scheme)
}

/// Trim, drop a leading '#', collapse inner whitespace, dedupe case-insensitively.
pub fn normalize_tags(tags: &[String]) -> AppResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let clean = t
            .trim()
            .trim_start_matches('#')
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if clean.is_empty() {
            continue;
        }
        if clean.chars().count() > MAX_TAG {
            return Err(AppError::invalid_input(format!(
                "Tags can be at most {MAX_TAG} characters."
            )));
        }
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&clean)) {
            out.push(clean);
        }
    }
    Ok(out)
}

fn check_ids(ids: &[String]) -> AppResult<()> {
    if ids.is_empty() {
        return Err(AppError::invalid_input("Select at least one item."));
    }
    if ids.len() > MAX_BATCH {
        return Err(AppError::invalid_input(format!(
            "Select at most {MAX_BATCH} items at a time."
        )));
    }
    Ok(())
}

pub(crate) fn check_folder(c: &Connection, folder_id: Option<&str>) -> AppResult<()> {
    if let Some(f) = folder_id {
        let ok: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM vault_folder WHERE id=?1 AND deleted_at IS NULL)",
            [f],
            |r| r.get(0),
        )?;
        if !ok {
            return Err(AppError::not_found("folder"));
        }
    }
    Ok(())
}

pub(crate) fn check_collection(c: &Connection, collection_id: &str) -> AppResult<String> {
    c.query_row(
        "SELECT name FROM vault_collection WHERE id=?1 AND deleted_at IS NULL",
        [collection_id],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("collection"))
}

fn check_live_item(c: &Connection, id: &str) -> AppResult<()> {
    let ok: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM vault_item WHERE id=?1 AND deleted_at IS NULL)",
        [id],
        |r| r.get(0),
    )?;
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found("Idea Vault item"))
    }
}

/// Everything needed to insert one item row.
#[derive(Debug, Default, Clone)]
pub(crate) struct NewItem {
    pub item_type: Option<VaultItemType>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub caption: Option<String>,
    pub url: Option<String>,
    pub source_text: Option<String>,
    pub asset_id: Option<String>,
    pub folder_id: Option<String>,
    pub source_global_item_id: Option<String>,
}

/// Insert an item (plus optional collection membership and tags). Returns its id.
pub(crate) fn insert_item(
    tx: &Tx<'_>,
    item: &NewItem,
    collection_id: Option<&str>,
    tags: &[String],
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let ty = item.item_type.unwrap_or(VaultItemType::Note);
    tx.conn().execute(
        "INSERT INTO vault_item(id, item_type, title, body, caption, url, source_text, asset_id, folder_id,
                                source_global_item_id, created_by, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            id,
            ty.as_str(),
            item.title,
            item.body,
            item.caption,
            item.url,
            item.source_text,
            item.asset_id,
            item.folder_id,
            item.source_global_item_id,
            tx.actor().user_id,
            now
        ],
    )?;
    if let Some(coll) = collection_id {
        add_membership(tx.conn(), &id, coll)?;
    }
    for t in tags {
        insert_tag(tx.conn(), &id, t)?;
    }
    Ok(id)
}

fn add_membership(c: &Connection, item_id: &str, collection_id: &str) -> AppResult<bool> {
    let now = now_ms();
    let n = c.execute(
        "INSERT INTO vault_item_collection(id, item_id, collection_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4) ON CONFLICT(item_id, collection_id) DO NOTHING",
        params![new_id(), item_id, collection_id, now],
    )?;
    Ok(n > 0)
}

fn insert_tag(c: &Connection, item_id: &str, tag: &str) -> AppResult<bool> {
    let now = now_ms();
    let n = c.execute(
        "INSERT INTO vault_item_tag(id, item_id, tag, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(item_id, tag) DO NOTHING",
        params![new_id(), item_id, tag, now],
    )?;
    Ok(n > 0)
}

/// Mark an item modified (tags are part of its searchable content, so undoing a
/// tag change must also touch — and reindex — the item row).
fn touch(c: &Connection, id: &str) -> AppResult<()> {
    c.execute(
        "UPDATE vault_item SET updated_at=?1, rev=rev+1 WHERE id=?2",
        params![now_ms(), id],
    )?;
    Ok(())
}

fn item_name(c: &Connection, id: &str) -> AppResult<String> {
    #[allow(clippy::type_complexity)]
    let row: Option<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = c
        .query_row(
            "SELECT item_type, title, body, caption, url, asset_id FROM vault_item WHERE id=?1",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?;
    let (ty, title, body, caption, url, asset_id) =
        row.ok_or_else(|| AppError::not_found("Idea Vault item"))?;
    let asset = load_asset_opt(c, std::path::Path::new(""), asset_id.as_deref())?;
    let t = VaultItemType::parse(&ty).unwrap_or(VaultItemType::File);
    Ok(display_name(
        t,
        title.as_deref(),
        body.as_deref(),
        caption.as_deref(),
        url.as_deref(),
        asset.as_ref(),
    ))
}

fn summary_for(what: &str, c: &Connection, ids: &[String]) -> AppResult<String> {
    Ok(if ids.len() == 1 {
        format!("{what} “{}”", item_name(c, &ids[0])?)
    } else {
        format!("{what} {} items", ids.len())
    })
}

fn load_one(core: &AppCore, actor: &Actor, store: StoreSel, id: &str) -> AppResult<VaultItemDto> {
    read_vault(core, actor, store, |c, root| load_item(c, root, store, id))
}

fn pre_summary(
    core: &AppCore,
    actor: &Actor,
    store: StoreSel,
    what: &str,
    ids: &[String],
) -> AppResult<String> {
    read_vault(core, actor, store, |c, _| summary_for(what, c, ids))
}

// --------------------------------------------------------------- commands

/// FSD-IDEA-001: create a note/URL/quote with nothing but its type.
pub(crate) fn create(
    core: &AppCore,
    actor: &Actor,
    args: VaultCreateArgs,
) -> AppResult<VaultItemDto> {
    if is_file_type(args.item_type) {
        return Err(AppError::invalid_input(format!(
            "Add a {} by choosing or dropping a file.",
            type_label(args.item_type).to_lowercase()
        )));
    }
    let url = match (args.item_type, args.url.as_deref()) {
        (VaultItemType::Url, Some(u)) => Some(normalize_url(u)?),
        (VaultItemType::Url, None) => return Err(AppError::required("Link")),
        (_, Some(u)) if !u.trim().is_empty() => Some(normalize_url(u)?),
        _ => None,
    };
    let item = NewItem {
        item_type: Some(args.item_type),
        title: optional_text(args.title, "Title", MAX_TITLE)?,
        body: args
            .body
            .map(|b| body_text(b, "Text", MAX_BODY_BYTES))
            .transpose()?
            .filter(|b| !b.trim().is_empty()),
        caption: optional_text(args.caption, "Caption", MAX_CAPTION)?,
        url,
        source_text: optional_text(args.source_text, "Source", MAX_TITLE)?,
        folder_id: args.folder_id.clone(),
        ..Default::default()
    };
    let tags = normalize_tags(&args.tags.unwrap_or_default())?;
    let label = type_label(args.item_type);
    let summary = match item.title.as_deref() {
        Some(t) => format!("Added {} “{t}”", label.to_lowercase()),
        None => format!("Added {}", label.to_lowercase()),
    };
    let id = mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.create", summary, Capability::Edit),
        |tx| {
            check_folder(tx.conn(), item.folder_id.as_deref())?;
            if let Some(c) = &args.collection_id {
                check_collection(tx.conn(), c)?;
            }
            insert_item(tx, &item, args.collection_id.as_deref(), &tags)
        },
    )?;
    load_one(core, actor, args.store, &id)
}

/// FSD-IDEA-002/003, §88.2: add any files in one operation; valid files are kept
/// even when others fail (mock 035 partial result).
pub(crate) fn add_files(
    core: &AppCore,
    actor: &Actor,
    args: VaultAddFilesArgs,
) -> AppResult<VaultAddFilesResult> {
    if args.paths.is_empty() {
        return Ok(VaultAddFilesResult {
            added: vec![],
            failed: vec![],
        });
    }
    if args.paths.len() > MAX_BATCH {
        return Err(AppError::invalid_input(format!(
            "Add at most {MAX_BATCH} files at a time."
        )));
    }
    let link = match args.mode.as_deref().unwrap_or("copy") {
        "copy" => false,
        "link" => true,
        _ => return Err(AppError::invalid_input("Unknown storage mode.")),
    };
    let name_of = |p: &str| {
        PathBuf::from(p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(p)
            .to_string()
    };
    let summary = if args.paths.len() == 1 {
        format!("Added “{}”", name_of(&args.paths[0]))
    } else {
        format!("Added {} files", args.paths.len())
    };
    let (ids, failed) = mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.add_files", summary, Capability::Edit),
        |tx| {
            check_folder(tx.conn(), args.folder_id.as_deref())?;
            if let Some(c) = &args.collection_id {
                check_collection(tx.conn(), c)?;
            }
            let mut ids = Vec::new();
            let mut failed = Vec::new();
            for p in &args.paths {
                // Each file is isolated: a failure rolls back only that file.
                tx.conn().execute_batch("SAVEPOINT vault_add_one")?;
                let one = (|| -> AppResult<String> {
                    let path = PathBuf::from(p);
                    if !path.is_absolute() {
                        return Err(AppError::invalid_input(
                            "OpenFrame needs the full location of the file.",
                        ));
                    }
                    let asset: AssetInfo = if link {
                        reference_external(tx, &path)?
                    } else {
                        ingest_file(tx, &path)?
                    };
                    let ty = match args.item_type {
                        Some(t) if args.paths.len() == 1 && is_file_type(t) => t,
                        _ => detect_type(&asset.original_name, &asset.media_type),
                    };
                    let item = NewItem {
                        item_type: Some(ty),
                        asset_id: Some(asset.id.clone()),
                        folder_id: args.folder_id.clone(),
                        ..Default::default()
                    };
                    insert_item(tx, &item, args.collection_id.as_deref(), &[])
                })();
                match one {
                    Ok(id) => {
                        tx.conn().execute_batch("RELEASE vault_add_one")?;
                        ids.push(id);
                    }
                    Err(e)
                        if matches!(
                            e.code_str(),
                            "storage.disk_full" | "storage.write_failed" | "storage.busy"
                        ) =>
                    {
                        // Disk full / database trouble: stop the whole operation.
                        return Err(e);
                    }
                    Err(e) => {
                        tx.conn()
                            .execute_batch("ROLLBACK TO vault_add_one; RELEASE vault_add_one")?;
                        failed.push(VaultFailedFile {
                            path: p.clone(),
                            name: name_of(p),
                            reason: file_failure_reason(&e),
                        });
                    }
                }
            }
            Ok((ids, failed))
        },
    )?;
    let added = read_vault(core, actor, args.store, |c, root| {
        ids.iter()
            .map(|id| load_item(c, root, args.store, id))
            .collect::<AppResult<Vec<_>>>()
    })?;
    Ok(VaultAddFilesResult { added, failed })
}

fn file_failure_reason(e: &AppError) -> String {
    let code = e.code_str();
    if code.starts_with("not_found") {
        "The file could not be found. It may have been moved or deleted.".into()
    } else if code == "storage.drive_unavailable" {
        "The drive holding this file is not available. Reconnect it and try again.".into()
    } else if code.starts_with("storage") {
        "The file is in use by another program and could not be read.".into()
    } else {
        e.message.clone()
    }
}

/// FSD-IDEA-013: recorded voice notes and drawn sketches arrive as bytes.
pub(crate) fn ingest(
    core: &AppCore,
    actor: &Actor,
    args: VaultIngestBytesArgs,
) -> AppResult<VaultItemDto> {
    if !is_file_type(args.item_type) {
        return Err(AppError::invalid_input(
            "Only recordings, drawings and files can be stored this way.",
        ));
    }
    if args.data_base64.len() > MAX_INGEST_BYTES / 3 * 4 + 8 {
        return Err(AppError::invalid_input("That recording is too large."));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(args.data_base64.trim())
        .map_err(|e| {
            AppError::invalid_input("The recording could not be read.").with_detail(e.to_string())
        })?;
    if bytes.is_empty() {
        return Err(AppError::invalid_input("Nothing was recorded."));
    }
    let file_name = crate::util::required_text(&args.file_name, "File name", 200)?;
    let media_type = args
        .media_type
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(|m| {
            // "audio/webm;codecs=opus" → "audio/webm"
            m.split(';').next().unwrap_or(m).trim().to_ascii_lowercase()
        });
    if let Some(m) = &media_type
        && (m.len() > 100 || !m.contains('/'))
    {
        return Err(AppError::invalid_input("Unknown file format."));
    }
    if let Some(d) = args.duration_ms
        && !(0..=24 * 3_600_000).contains(&d)
    {
        return Err(AppError::invalid_input(
            "The recording length is not valid.",
        ));
    }
    let title = optional_text(args.title, "Name", MAX_TITLE)?;
    let caption = optional_text(args.caption, "Caption", MAX_CAPTION)?;
    let label = type_label(args.item_type).to_lowercase();
    let summary = match &title {
        Some(t) => format!("Added {label} “{t}”"),
        None => format!("Added {label}"),
    };
    let id = mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.ingest_bytes", summary, Capability::Edit),
        |tx| {
            check_folder(tx.conn(), args.folder_id.as_deref())?;
            if let Some(c) = &args.collection_id {
                check_collection(tx.conn(), c)?;
            }
            let asset = ingest_bytes(tx, &bytes, &file_name, media_type.as_deref())?;
            if let Some(d) = args.duration_ms {
                tx.conn().execute(
                    "UPDATE asset SET duration_ms=?1 WHERE id=?2",
                    params![d, asset.id],
                )?;
            }
            let item = NewItem {
                item_type: Some(args.item_type),
                title: title.clone(),
                caption: caption.clone(),
                asset_id: Some(asset.id),
                folder_id: args.folder_id.clone(),
                ..Default::default()
            };
            insert_item(tx, &item, args.collection_id.as_deref(), &[])
        },
    )?;
    load_one(core, actor, args.store, &id)
}

/// FSD-IDEA-011: text edits autosave; consecutive edits coalesce into one undo step.
pub(crate) fn update(
    core: &AppCore,
    actor: &Actor,
    args: VaultUpdateArgs,
) -> AppResult<VaultItemDto> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(t) = args.title {
        fields.push((
            "title",
            opt_text(optional_text(Some(t), "Title", MAX_TITLE)?),
        ));
    }
    if let Some(b) = args.body {
        let b = body_text(b, "Text", MAX_BODY_BYTES)?;
        fields.push((
            "body",
            if b.trim().is_empty() {
                SqlValue::Null
            } else {
                SqlValue::Text(b)
            },
        ));
    }
    if let Some(c) = args.caption {
        fields.push((
            "caption",
            opt_text(optional_text(Some(c), "Caption", MAX_CAPTION)?),
        ));
    }
    if let Some(s) = args.source_text {
        fields.push((
            "source_text",
            opt_text(optional_text(Some(s), "Source", MAX_TITLE)?),
        ));
    }
    let url_change = args.url;
    let (ty, name) = read_vault(core, actor, args.store, |c, _| {
        let ty: Option<String> = c
            .query_row(
                "SELECT item_type FROM vault_item WHERE id=?1 AND deleted_at IS NULL",
                [&args.id],
                |r| r.get(0),
            )
            .optional()?;
        let ty = ty.ok_or_else(|| AppError::not_found("Idea Vault item"))?;
        Ok((
            VaultItemType::parse(&ty).unwrap_or(VaultItemType::File),
            item_name(c, &args.id)?,
        ))
    })?;
    if let Some(u) = url_change {
        if u.trim().is_empty() {
            if ty == VaultItemType::Url {
                return Err(AppError::required("Link"));
            }
            fields.push(("url", SqlValue::Null));
        } else {
            fields.push(("url", SqlValue::Text(normalize_url(&u)?)));
        }
    }
    if fields.is_empty() {
        return load_one(core, actor, args.store, &args.id);
    }
    let meta = MutationMeta::new("vault.update", format!("Edited “{name}”"), Capability::Edit)
        .target("vault_item", &args.id)
        .coalesce(format!("vault.edit:{}", args.id));
    mutate_vault(core, actor, args.store, meta, |tx| {
        update_fields(
            tx.conn(),
            "vault_item",
            &args.id,
            &fields,
            &["title", "body", "caption", "url", "source_text"],
            args.expected_rev,
            "Idea Vault item",
        )?;
        Ok(())
    })?;
    load_one(core, actor, args.store, &args.id)
}

/// FSD-IDEA-008: pin changes display priority only.
pub(crate) fn set_pinned(core: &AppCore, actor: &Actor, args: VaultSetPinnedArgs) -> AppResult<()> {
    check_ids(&args.ids)?;
    let what = if args.pinned { "Pinned" } else { "Unpinned" };
    let summary = pre_summary(core, actor, args.store, what, &args.ids)?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.set_pinned", summary, Capability::Edit),
        |tx| {
            let now = now_ms();
            for id in &args.ids {
                check_live_item(tx.conn(), id)?;
                update_fields(
                    tx.conn(),
                    "vault_item",
                    id,
                    &[
                        ("pinned", int(args.pinned as i64)),
                        ("pinned_at", opt_int(args.pinned.then_some(now))),
                    ],
                    &["pinned", "pinned_at"],
                    None,
                    "Idea Vault item",
                )?;
            }
            Ok(())
        },
    )
}

/// FSD-IDEA-006: folders change organization only.
pub(crate) fn move_to_folder(core: &AppCore, actor: &Actor, args: VaultMoveArgs) -> AppResult<()> {
    check_ids(&args.ids)?;
    let summary = pre_summary(core, actor, args.store, "Moved", &args.ids)?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.move_to_folder", summary, Capability::Edit),
        |tx| {
            check_folder(tx.conn(), args.folder_id.as_deref())?;
            for id in &args.ids {
                check_live_item(tx.conn(), id)?;
                let current: Option<String> = tx.conn().query_row(
                    "SELECT folder_id FROM vault_item WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )?;
                if current == args.folder_id {
                    continue;
                }
                update_fields(
                    tx.conn(),
                    "vault_item",
                    id,
                    &[("folder_id", opt_text(args.folder_id.clone()))],
                    &["folder_id"],
                    None,
                    "Idea Vault item",
                )?;
            }
            Ok(())
        },
    )
}

/// FSD-IDEA-007: collection membership is a reference, never a copy.
pub(crate) fn add_to_collection(
    core: &AppCore,
    actor: &Actor,
    args: VaultMembershipArgs,
) -> AppResult<()> {
    check_ids(&args.ids)?;
    let coll = read_vault(core, actor, args.store, |c, _| {
        check_collection(c, &args.collection_id)
    })?;
    let summary = if args.ids.len() == 1 {
        format!(
            "Added “{}” to {coll}",
            read_vault(core, actor, args.store, |c, _| item_name(c, &args.ids[0]))?
        )
    } else {
        format!("Added {} items to {coll}", args.ids.len())
    };
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.add_to_collection", summary, Capability::Edit),
        |tx| {
            check_collection(tx.conn(), &args.collection_id)?;
            for id in &args.ids {
                check_live_item(tx.conn(), id)?;
                add_membership(tx.conn(), id, &args.collection_id)?;
            }
            Ok(())
        },
    )
}

/// Removing from a collection never deletes the item (FSD §88.7).
pub(crate) fn remove_from_collection(
    core: &AppCore,
    actor: &Actor,
    args: VaultMembershipArgs,
) -> AppResult<()> {
    check_ids(&args.ids)?;
    let coll = read_vault(core, actor, args.store, |c, _| {
        c.query_row(
            "SELECT name FROM vault_collection WHERE id=?1",
            [&args.collection_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("collection"))
    })?;
    let summary = if args.ids.len() == 1 {
        format!(
            "Removed “{}” from {coll}",
            read_vault(core, actor, args.store, |c, _| item_name(c, &args.ids[0]))?
        )
    } else {
        format!("Removed {} items from {coll}", args.ids.len())
    };
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.remove_from_collection", summary, Capability::Edit),
        |tx| {
            for id in &args.ids {
                tx.conn().execute(
                    "DELETE FROM vault_item_collection WHERE item_id=?1 AND collection_id=?2",
                    params![id, args.collection_id],
                )?;
            }
            Ok(())
        },
    )
}

pub(crate) fn add_tags(core: &AppCore, actor: &Actor, args: VaultTagsArgs) -> AppResult<()> {
    check_ids(&args.ids)?;
    let tags = normalize_tags(&args.tags)?;
    if tags.is_empty() {
        return Err(AppError::required("Tag"));
    }
    let summary = format!(
        "Tagged {} with {}",
        pre_summary(core, actor, args.store, "", &args.ids)?.trim(),
        tags.join(", ")
    );
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.add_tags", summary, Capability::Edit),
        |tx| {
            for id in &args.ids {
                check_live_item(tx.conn(), id)?;
                let mut changed = false;
                for t in &tags {
                    changed |= insert_tag(tx.conn(), id, t)?;
                }
                if changed {
                    touch(tx.conn(), id)?;
                }
            }
            Ok(())
        },
    )
}

pub(crate) fn remove_tag(core: &AppCore, actor: &Actor, args: VaultRemoveTagArgs) -> AppResult<()> {
    check_ids(&args.ids)?;
    let tag = args.tag.trim().trim_start_matches('#').to_string();
    let summary = format!("Removed tag {tag}");
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.remove_tag", summary, Capability::Edit),
        |tx| {
            for id in &args.ids {
                let n = tx.conn().execute(
                    "DELETE FROM vault_item_tag WHERE item_id=?1 AND tag=?2",
                    params![id, tag],
                )?;
                if n > 0 {
                    touch(tx.conn(), id)?;
                }
            }
            Ok(())
        },
    )
}

/// FSD-IDEA-018 / §5.11: delete moves items to Recently Deleted (recoverable).
pub(crate) fn delete(core: &AppCore, actor: &Actor, args: VaultIdsArgs) -> AppResult<()> {
    check_ids(&args.ids)?;
    let summary = pre_summary(core, actor, args.store, "Deleted", &args.ids)?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new("vault.delete", summary, Capability::SoftDelete),
        |tx| {
            for id in &args.ids {
                let folder: Option<String> = tx
                    .conn()
                    .query_row(
                        "SELECT folder_id FROM vault_item WHERE id=?1 AND deleted_at IS NULL",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| AppError::not_found("Idea Vault item"))?;
                let name = item_name(tx.conn(), id)?;
                soft_delete(
                    tx,
                    DeleteSpec {
                        object_type: "vault_item",
                        table: "vault_item",
                        id,
                        title: Some(name),
                        parent_type: Some("vault_folder"),
                        parent_id: folder,
                        position: None,
                    },
                )?;
            }
            Ok(())
        },
    )
}

/// FSD-IDEA-020: relink an external file that moved. Keeps the item and its notes.
pub(crate) fn relink(
    core: &AppCore,
    actor: &Actor,
    args: VaultRelinkArgs,
) -> AppResult<VaultItemDto> {
    let name = read_vault(core, actor, args.store, |c, _| item_name(c, &args.id))?;
    mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.relink",
            format!("Relinked “{name}”"),
            Capability::Edit,
        )
        .target("vault_item", &args.id),
        |tx| {
            let asset_id: Option<String> = tx
                .conn()
                .query_row(
                    "SELECT asset_id FROM vault_item WHERE id=?1 AND deleted_at IS NULL",
                    [&args.id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| AppError::not_found("Idea Vault item"))?;
            let asset_id = asset_id
                .ok_or_else(|| AppError::invalid_input("This item has no file to relink."))?;
            let mode: String = tx.conn().query_row(
                "SELECT storage_mode FROM asset WHERE id=?1",
                [&asset_id],
                |r| r.get(0),
            )?;
            if mode != "external" {
                return Err(AppError::invalid_input(
                    "Only linked files can be relinked.",
                ));
            }
            let path = PathBuf::from(&args.path);
            let meta = std::fs::metadata(&path)?;
            if !meta.is_file() {
                return Err(AppError::invalid_input(
                    "Please choose a file, not a folder.",
                ));
            }
            let abs = path
                .canonicalize()
                .unwrap_or(path)
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_string();
            // Relinking keeps the same logical asset identity (Domain §22).
            tx.conn().execute(
                "UPDATE asset SET external_path=?1, byte_size=?2, last_seen_at=?3, updated_at=?3, rev=rev+1 WHERE id=?4",
                params![abs, meta.len() as i64, now_ms(), asset_id],
            )?;
            touch(tx.conn(), &args.id)?;
            Ok(())
        },
    )?;
    load_one(core, actor, args.store, &args.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_normalised_and_unsafe_schemes_refused() {
        assert_eq!(
            normalize_url(" example.com/a ").unwrap(),
            "https://example.com/a"
        );
        assert_eq!(
            normalize_url("http://x.example").unwrap(),
            "http://x.example"
        );
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("file:///c:/x").is_err());
        assert!(normalize_url("   ").is_err());
        assert!(normalize_url("https://").is_err());
    }

    #[test]
    fn tags_are_cleaned_and_deduplicated() {
        let t = normalize_tags(&[
            "#Rain".into(),
            "rain".into(),
            "  night   bus ".into(),
            "".into(),
        ])
        .unwrap();
        assert_eq!(t, vec!["Rain".to_string(), "night bus".to_string()]);
        assert!(normalize_tags(&["x".repeat(61)]).is_err());
    }
}
