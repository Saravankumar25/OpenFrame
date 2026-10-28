//! The `.ofvault` file (doc 40 section 4): a backup of the Global Idea Vault, or a package of it
//! for another computer, and the restore from either.
//!
//! The container is the one packages use (`manifest.json`, `checksums.json` with a SHA-256 for
//! every other entry, written through `openframe_security::archive` and moved into place
//! atomically), with a manifest of its own. A restore validates everything in a staging folder
//! beside the vault before the live vault is touched, and the vault it replaces is kept.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, now_ms};
use openframe_security::archive::{self, ArchiveLimits};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::{
    ROLLBACK_FAILED, STAGING_MARKER, StagingNote, StorageTaskStarted, existing_vault, library, tree,
};
use crate::core::{AppCore, StorageOp, storage_busy};
use crate::modules::packages::format::{
    self, CHECKSUMS_ENTRY, Checksums, EntryData, MANIFEST_ENTRY, Staging, WriteOutcome,
};
use openframe_domain::Actor;

pub const EXTENSION: &str = "ofvault";
const FORMAT: &str = "openframe-vault";
const FORMAT_VERSION: u32 = 1;
const VAULT_DB: &str = "vault.sqlite";
/// Metadata entries are small; anything larger is not a real manifest.
const MAX_META_BYTES: u64 = 4 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum VaultArchiveKind {
    /// Everything, Recently Deleted included.
    Backup,
    /// For another computer: without Recently Deleted.
    Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultManifest {
    format: String,
    format_version: u32,
    kind: VaultArchiveKind,
    app_version: String,
    created_at: i64,
    schema_version: u32,
    /// Ideas in the vault (not counting Recently Deleted), for the confirmation screen.
    idea_count: u32,
    /// Files under `assets/`.
    asset_count: u32,
    /// Managed files the vault's own database named but the vault folder no longer had when the
    /// archive was made. A restore does not insist on them (the archive is as complete as the
    /// vault was).
    #[serde(default)]
    missing_assets: Vec<String>,
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, serde::Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultFileArgs {
    /// The `.ofvault` file: where to save it, or which one to read.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultArchiveReport {
    pub path: String,
    pub file_name: String,
    pub kind: VaultArchiveKind,
    #[ts(type = "number")]
    pub bytes: i64,
    pub idea_count: u32,
    pub asset_count: u32,
    /// Files the vault was already missing; they are not in the archive.
    pub missing_assets: u32,
}

/// What a vault file holds, for the screen that asks before a restore.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultInspection {
    pub file_name: String,
    pub kind: VaultArchiveKind,
    #[ts(type = "number")]
    pub created_at: i64,
    pub app_version: String,
    pub idea_count: u32,
    pub asset_count: u32,
    #[ts(type = "number")]
    pub bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultRestoreReport {
    /// Where the vault that was replaced is kept. `None` when there was none.
    pub previous_kept_at: Option<String>,
}

// ------------------------------------------------------------------ errors

fn not_vault() -> AppError {
    AppError::import(
        "not_vault",
        "This file isn't an OpenFrame vault backup, so nothing was restored. Your vault was not changed.",
    )
}

fn damaged(detail: impl Into<String>) -> AppError {
    AppError::import(
        "vault_incomplete",
        "This vault file is incomplete or damaged, so nothing was restored. Your vault was not changed.",
    )
    .with_detail(detail)
}

fn changed(detail: impl Into<String>) -> AppError {
    AppError::import(
        "vault_checksum_mismatch",
        "This vault file is damaged or was changed after it was saved, so nothing was restored. Your vault was not changed.",
    )
    .with_detail(detail)
}

fn too_new(detail: impl Into<String>) -> AppError {
    AppError::import(
        "vault_too_new",
        "This vault file was saved by a newer version of OpenFrame. Update OpenFrame to restore it. Nothing was changed.",
    )
    .with_detail(detail)
}

fn supported_schema() -> u32 {
    crate::schema::GLOBAL_MIGRATIONS
        .last()
        .map_or(0, |m| m.version)
}

// ------------------------------------------------------------------ reading

/// Entries a vault file may hold; anything else (a `-wal` file SQLite would replay, a program)
/// makes the file unacceptable.
fn is_allowed(name: &str) -> bool {
    matches!(name, MANIFEST_ENTRY | CHECKSUMS_ENTRY | VAULT_DB)
        || name.strip_prefix("assets/").is_some_and(|r| !r.is_empty())
}

fn parse_manifest(bytes: &[u8]) -> AppResult<VaultManifest> {
    let raw: Value = serde_json::from_slice(bytes).map_err(|_| not_vault())?;
    if raw.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(not_vault());
    }
    let version = raw
        .get("formatVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| damaged("manifest has no format version"))?;
    if version > u64::from(FORMAT_VERSION) {
        return Err(too_new(format!("vault format {version}")));
    }
    if version < u64::from(FORMAT_VERSION) {
        return Err(damaged(format!("unknown vault format {version}")));
    }
    let manifest: VaultManifest =
        serde_json::from_value(raw).map_err(|e| damaged(format!("manifest: {e}")))?;
    if manifest.schema_version > supported_schema() {
        return Err(too_new(format!(
            "vault schema {} > supported {}",
            manifest.schema_version,
            supported_schema()
        )));
    }
    Ok(manifest)
}

/// Structure, format version, manifest and the list of entries, without extracting anything.
/// The manifest, the entry names and the total size the entries declare once unpacked.
fn read_header(path: &Path) -> AppResult<(VaultManifest, Vec<String>, u64)> {
    // The file is one the person picked: absolute, no device or reserved names, a real file.
    openframe_security::validate_input_file(path, u64::MAX).map_err(|e| {
        if e.is("security") {
            e
        } else {
            AppError::new(
                "not_found.file",
                "This vault file can't be found. If it's on an external drive, reconnect it and try again.",
            )
        }
    })?;
    let (names, unpacked) = archive::inspect_sized(path, ArchiveLimits::PACKAGE)?;
    if let Some(stray) = names.iter().find(|n| !is_allowed(n)) {
        return Err(damaged(format!("unexpected entry: {stray}")));
    }
    let manifest_bytes =
        archive::read_entry(path, MANIFEST_ENTRY, MAX_META_BYTES)?.ok_or_else(|| {
            if names.is_empty() {
                not_vault()
            } else {
                damaged("manifest.json missing")
            }
        })?;
    let manifest = parse_manifest(&manifest_bytes)?;
    let checksum_bytes = archive::read_entry(path, CHECKSUMS_ENTRY, MAX_META_BYTES)?
        .ok_or_else(|| damaged("checksums.json missing"))?;
    let sums: Checksums =
        serde_json::from_slice(&checksum_bytes).map_err(|e| damaged(format!("checksums: {e}")))?;
    if sums.algorithm != "sha256" {
        return Err(damaged("unsupported checksum algorithm"));
    }
    let listed: BTreeSet<&str> = sums.files.keys().map(String::as_str).collect();
    let present: BTreeSet<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|n| *n != CHECKSUMS_ENTRY)
        .collect();
    if listed != present {
        return Err(damaged("the entries do not match checksums.json"));
    }
    if !present.contains(VAULT_DB) {
        return Err(damaged("vault.sqlite missing"));
    }
    Ok((manifest, names, unpacked))
}

pub(crate) fn inspect(
    _core: &AppCore,
    _a: &Actor,
    args: VaultFileArgs,
) -> AppResult<VaultInspection> {
    let path = PathBuf::from(args.path.trim());
    let (manifest, names, _) = read_header(&path)?;
    Ok(VaultInspection {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        kind: manifest.kind,
        created_at: manifest.created_at,
        app_version: manifest.app_version,
        idea_count: manifest.idea_count,
        asset_count: names.iter().filter(|n| n.starts_with("assets/")).count() as u32,
        bytes: fs::metadata(&path).map_or(0, |m| m.len() as i64),
    })
}

// ------------------------------------------------------------------ writing

pub(crate) fn start_archive(
    core: &AppCore,
    args: VaultFileArgs,
    kind: VaultArchiveKind,
) -> AppResult<StorageTaskStarted> {
    let guard = core.begin_storage_op(StorageOp::Vault)?;
    let dest = format::output_destination(&args.path, EXTENSION)?;
    crate::util::check_output_file(core, &dest)?;
    existing_vault(core, "back up")?;
    let label = match kind {
        VaultArchiveKind::Backup => "Backing up the vault",
        VaultArchiveKind::Package => "Exporting the vault package",
    };
    let task_id = core.spawn_task("storage.vault_archive", label, move |core, h| {
        let _guard = guard;
        let report = write_archive(&core, &dest, kind, &|f, m| h.progress(f, m), &|| {
            h.is_cancelled()
        })?
        .ok_or_else(AppError::cancelled)?;
        serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
    });
    Ok(StorageTaskStarted { task_id })
}

/// Remove what Recently Deleted holds from a copy of the vault database, and the history
/// (undo steps keep full copies of deleted rows; the activity log names them). The search index
/// is rebuilt and the copy vacuumed afterwards, so the removed text does not stay in the file's
/// index pages or free pages.
fn drop_recently_deleted(conn: &rusqlite::Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM vault_item_tag WHERE item_id IN (SELECT id FROM vault_item WHERE deleted_at IS NOT NULL);
         DELETE FROM vault_item_collection
             WHERE item_id IN (SELECT id FROM vault_item WHERE deleted_at IS NOT NULL)
                OR collection_id IN (SELECT id FROM vault_collection WHERE deleted_at IS NOT NULL);
         DELETE FROM vault_item WHERE deleted_at IS NOT NULL;
         UPDATE vault_item SET folder_id = NULL
             WHERE folder_id IN (SELECT id FROM vault_folder WHERE deleted_at IS NOT NULL);
         UPDATE vault_folder SET parent_id = NULL
             WHERE parent_id IN (SELECT id FROM vault_folder WHERE deleted_at IS NOT NULL);
         DELETE FROM vault_folder WHERE deleted_at IS NOT NULL;
         DELETE FROM vault_collection WHERE deleted_at IS NOT NULL;
         DELETE FROM asset WHERE id NOT IN (SELECT asset_id FROM vault_item WHERE asset_id IS NOT NULL);
         -- A linked file's full path names folders on this computer (often the person's account
         -- name) and means nothing on another one. Keep only the file name, so the item shows as
         -- a missing link the person can point at the file again.
         UPDATE asset SET external_path = original_name WHERE storage_mode = 'external';
         DELETE FROM search_doc WHERE source_table = 'vault_item' AND entity_id NOT IN (SELECT id FROM vault_item);
         INSERT INTO search_fts(search_fts) VALUES('rebuild');
         DELETE FROM deleted_item;
         DELETE FROM sys_undo;
         DELETE FROM sys_activity;",
    )?;
    Ok(())
}

fn write_archive(
    core: &AppCore,
    dest: &Path,
    kind: VaultArchiveKind,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<Option<VaultArchiveReport>> {
    let vault = existing_vault(core, "back up")?;
    let store = core.global_store()?;
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AppError::internal("the vault file has no folder"))?;
    // Beside the destination, like packages: the copy is as big as the database and the final
    // file lands on that drive too.
    let staging = Staging::create(parent, "openframe-vault")?;
    let copy = staging.path().join(VAULT_DB);
    progress(0.02, "Copying the vault database");
    // The online backup API: a consistent copy while the vault is in use.
    store.with_writer(|c| openframe_persistence::backup_to(c, &copy))?;
    drop(store);
    if cancelled() {
        return Ok(None);
    }
    let (idea_count, managed) = {
        let conn = openframe_persistence::open_connection(&copy, false)?;
        if kind == VaultArchiveKind::Package {
            drop_recently_deleted(&conn)?;
            // VACUUM works through a temporary attached database, which OpenFrame's connections
            // never allow; this one is a scratch copy of ours.
            conn.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_ATTACHED, 2)?;
            conn.execute_batch("VACUUM;")?;
        }
        let ideas: u32 = conn.query_row(
            "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, rel_path FROM asset WHERE storage_mode = 'managed' AND rel_path IS NOT NULL",
        )?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        // Leave a plain file behind: no write-ahead log, nothing waiting to be folded in.
        openframe_persistence::checkpoint(&conn)?;
        conn.execute_batch("PRAGMA journal_mode=DELETE;")?;
        (ideas, rows)
    };
    let assets_dir = vault.join("assets");
    let files = if assets_dir.is_dir() {
        tree::walk(&assets_dir, &[])?
    } else {
        tree::Tree::default()
    };
    let referenced: HashSet<String> = managed
        .iter()
        .map(|(_, rel)| rel.replace('\\', "/"))
        .collect();
    let mut entries = vec![(VAULT_DB.to_string(), EntryData::File(copy.clone()))];
    for file in files.files {
        let name = format!("assets/{}", file.rel);
        // A package carries only the files the remaining ideas use.
        if kind == VaultArchiveKind::Package && !referenced.contains(&name) {
            continue;
        }
        if openframe_security::validate_relative(&name).is_ok() {
            entries.push((name, EntryData::File(file.path)));
        }
    }
    let included: HashSet<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    let missing_assets: Vec<String> = managed
        .iter()
        .filter(|(_, rel)| !included.contains(rel.replace('\\', "/").as_str()))
        .map(|(id, _)| id.clone())
        .collect();
    let asset_count = (entries.len() - 1) as u32;
    let manifest = VaultManifest {
        format: FORMAT.to_string(),
        format_version: FORMAT_VERSION,
        kind,
        app_version: core.config.app_version.clone(),
        created_at: now_ms(),
        schema_version: crate::schema::GLOBAL_MIGRATIONS
            .last()
            .map_or(0, |m| m.version),
        idea_count,
        asset_count,
        missing_assets: missing_assets.clone(),
    };
    let outcome = format::write_container(
        dest,
        &format::to_json_bytes(&manifest)?,
        entries,
        &|f, _| progress(f, "Writing the file"),
        cancelled,
    )?;
    Ok(match outcome {
        WriteOutcome::Cancelled => None,
        WriteOutcome::Written { bytes } => Some(VaultArchiveReport {
            path: dest.to_string_lossy().into_owned(),
            file_name: dest
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            kind,
            bytes: bytes as i64,
            idea_count,
            asset_count,
            missing_assets: missing_assets.len() as u32,
        }),
    })
}

// ------------------------------------------------------------------ restore

pub(crate) fn start_restore(core: &AppCore, args: VaultFileArgs) -> AppResult<StorageTaskStarted> {
    let guard = core.begin_storage_op(StorageOp::Vault)?;
    let path = PathBuf::from(args.path.trim());
    // The structure is checked here so the person gets the answer at once; unpacking and the
    // rest run in the task.
    read_header(&path)?;
    let task_id = core.spawn_task(
        "storage.vault_restore",
        "Restoring the vault",
        move |core, h| {
            let _guard = guard;
            let report = restore(&core, &path, &|f, m| h.progress(f, m), &|| h.is_cancelled())?
                .ok_or_else(AppError::cancelled)?;
            serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
        },
    );
    Ok(StorageTaskStarted { task_id })
}

fn restore(
    core: &AppCore,
    path: &Path,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<Option<VaultRestoreReport>> {
    let (manifest, _, unpacked) = read_header(path)?;
    let live = core.vault_dir();
    let (Some(parent), Some(name)) = (
        live.parent().filter(|p| !p.as_os_str().is_empty()),
        live.file_name(),
    ) else {
        return Err(AppError::invalid_input(
            "The vault folder is the top folder of a drive, so it can't be replaced.",
        ));
    };
    if !parent.is_dir() {
        if library::same_place(&live, &core.config.global_vault_dir) {
            fs::create_dir_all(parent)?;
        } else {
            return Err(AppError::new(
                "not_found.folder",
                "That folder can't be found. If it's on an external drive, reconnect it and try again.",
            ));
        }
    }
    // The vault it replaces stays until the restored one opens, so all of it must fit beside.
    tree::ensure_space(parent, unpacked)?;
    let staging_path = parent.join(format!(
        "{}{STAGING_MARKER}{}",
        name.to_string_lossy(),
        now_ms()
    ));
    // Declared before the staging folder, so it is dropped after it.
    let _note = StagingNote::record(core, &staging_path);
    let staging = Staging::at(staging_path);
    progress(0.05, "Unpacking the file");
    archive::extract_all(path, staging.path(), ArchiveLimits::PACKAGE)?;
    if cancelled() {
        return Ok(None);
    }
    progress(0.5, "Checking every file");
    verify_hashes(staging.path())?;
    progress(0.75, "Checking the vault data");
    check_restored(staging.path(), &manifest)?;
    if cancelled() {
        return Ok(None);
    }
    progress(0.9, "Switching to the restored vault");
    let kept = core.with_vault_closed(|| {
        // Read again now that nothing can change it: the place this restore planned for is
        // still the vault's place.
        if !library::same_place(&core.vault_dir(), &live) {
            return Err(storage_busy(
                "Your Global Idea Vault was moved or reconnected while this restore was running. Nothing was changed. Try the restore again.",
            ));
        }
        swap_in(&live, staging.path())
    })?;
    // The vault is opened for real (and upgraded when it is older) before the restore counts as
    // done; if that fails, the vault it replaced goes back.
    if let Err(e) = core.global_store() {
        core.with_vault_closed(|| put_back(&live, kept.as_deref()))
            .map_err(|r| {
                if r.code_str() == ROLLBACK_FAILED {
                    r
                } else {
                    rollback_failed(kept.as_deref(), r.message)
                }
            })?;
        return Err(e);
    }
    Ok(Some(VaultRestoreReport {
        previous_kept_at: kept.map(|k| k.to_string_lossy().into_owned()),
    }))
}

/// Every file `checksums.json` lists must be there and match.
fn verify_hashes(root: &Path) -> AppResult<()> {
    let sums: Checksums = serde_json::from_slice(
        &fs::read(root.join(CHECKSUMS_ENTRY)).map_err(|_| damaged("checksums.json missing"))?,
    )
    .map_err(|e| damaged(format!("checksums: {e}")))?;
    for (name, expected) in &sums.files {
        let p = root.join(openframe_security::validate_relative(name)?);
        if !p.is_file() {
            return Err(damaged(format!("entry missing: {name}")));
        }
        if !openframe_security::sha256_file(&p)?.eq_ignore_ascii_case(expected) {
            return Err(changed(format!("entry changed: {name}")));
        }
    }
    // Nothing else may be there: a file the list does not name (a `vault.sqlite-wal` that SQLite
    // would replay) never reaches the vault.
    let listed: HashSet<&str> = sums
        .files
        .keys()
        .map(String::as_str)
        .chain([MANIFEST_ENTRY, CHECKSUMS_ENTRY])
        .collect();
    if let Some(stray) = tree::walk(root, &[])?
        .files
        .iter()
        .find(|f| !listed.contains(f.rel.as_str()))
    {
        return Err(damaged(format!("unlisted file: {}", stray.rel)));
    }
    Ok(())
}

/// The extracted vault database is sound and every managed file it names is there.
fn check_restored(root: &Path, manifest: &VaultManifest) -> AppResult<()> {
    let db = root.join(VAULT_DB);
    check_vault_database(&db)?;
    let conn = openframe_persistence::open_connection(&db, false)?;
    let mut stmt = conn.prepare(
        "SELECT id, rel_path FROM asset WHERE storage_mode = 'managed' AND rel_path IS NOT NULL",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, rel) in rows {
        if manifest.missing_assets.contains(&id) {
            continue;
        }
        let rel = openframe_security::validate_relative(&rel)
            .map_err(|_| damaged(format!("asset {id} has an unsafe path")))?;
        if !rel.starts_with("assets") || !root.join(&rel).is_file() {
            return Err(damaged(format!("asset file missing: {id}")));
        }
    }
    Ok(())
}

/// A database that is an OpenFrame Global Idea Vault this version can open: the vault tables are
/// there, it passes the full integrity check, it is not newer than this version and it carries no
/// code OpenFrame does not create. Used for a restore, a located vault and a moved copy.
pub(super) fn check_vault_database(db: &Path) -> AppResult<()> {
    let not_a_vault = || {
        AppError::new(
            "storage.not_a_vault",
            "That isn't an OpenFrame Global Idea Vault, so nothing was changed.",
        )
    };
    let conn = openframe_persistence::open_connection(db, false).map_err(|_| not_a_vault())?;
    let tables: Result<Vec<String>, _> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .and_then(|mut s| s.query_map([], |r| r.get(0))?.collect());
    let tables = tables.map_err(|_| not_a_vault())?;
    if ![
        "vault_item",
        "vault_folder",
        "vault_collection",
        "asset",
        "deleted_item",
    ]
    .iter()
    .all(|t| tables.iter().any(|x| x == t))
    {
        return Err(not_a_vault());
    }
    let problems = openframe_persistence::integrity_problems(&conn, true)?;
    if !problems.is_empty() {
        return Err(AppError::new(
            "storage.vault_damaged",
            "The vault file is damaged, so nothing was changed. Restore it from a backup instead.",
        )
        .with_detail(problems.join("; ")));
    }
    let found = openframe_persistence::migrate::current_version(&conn)?;
    if found > supported_schema() {
        return Err(AppError::new(
            "storage.vault_too_new",
            "This vault was saved by a newer version of OpenFrame. Update OpenFrame to use it. Nothing was changed.",
        )
        .with_detail(format!("schema {found} > supported {}", supported_schema())));
    }
    let sql: Vec<&str> = crate::schema::GLOBAL_MIGRATIONS
        .iter()
        .map(|m| m.sql)
        .collect();
    let problems = openframe_persistence::untrusted_schema_problems(&conn, &sql)?;
    if !problems.is_empty() {
        return Err(AppError::new(
            "storage.untrusted_schema",
            "This vault contains something OpenFrame doesn't create, so nothing was changed.",
        )
        .with_detail(problems.join("; ")));
    }
    Ok(())
}

/// The undo of a swap did not work. Says where the earlier vault is, so it is never lost.
fn rollback_failed(kept: Option<&Path>, detail: impl Into<String>) -> AppError {
    let message = match kept {
        Some(k) => format!(
            "OpenFrame could not put your earlier Global Idea Vault back in place. It is safe in the folder “{}”. Close OpenFrame, then rename that folder to the vault folder name, or choose it in Settings > Storage.",
            k.display()
        ),
        None => "OpenFrame could not remove the restored vault that would not open. Close OpenFrame and delete the vault folder by hand.".to_string(),
    };
    AppError::new(ROLLBACK_FAILED, message).with_detail(detail)
}

/// Undo a swap whose new vault is not wanted: the folder at `live` is renamed aside, the folder
/// that was kept goes back to `live`, and only then is the unwanted one deleted. A step that
/// fails stops here and nothing is deleted; the error names where the earlier vault is.
fn put_back(live: &Path, kept: Option<&Path>) -> AppResult<()> {
    let aside = tree::aside_name(live);
    if live.exists() {
        fs::rename(live, &aside).map_err(|e| rollback_failed(kept, e.to_string()))?;
    }
    if let Some(k) = kept
        && let Err(e) = fs::rename(k, live)
    {
        // The unwanted folder goes back to the vault's place, so that place is never empty.
        if aside.exists() {
            let _ = fs::rename(&aside, live);
        }
        return Err(rollback_failed(Some(k), e.to_string()));
    }
    // Best effort: a folder left here has a staging name, which start removes.
    let _ = fs::remove_dir_all(&aside);
    Ok(())
}

/// Put the staged vault in the live place. The vault that was there is renamed beside it, never
/// deleted. Returns where it was kept. On a failed second step the first is undone, and when that
/// fails too the error says where the earlier vault is.
fn swap_in(live: &Path, staged: &Path) -> AppResult<Option<PathBuf>> {
    let kept = if live.exists() {
        let to = before_restore_path(live);
        fs::rename(live, &to).map_err(|e| {
            AppError::from(e).with_detail("the current vault folder could not be renamed")
        })?;
        Some(to)
    } else {
        None
    };
    if let Err(e) = fs::rename(staged, live) {
        if let Some(k) = &kept {
            fs::rename(k, live).map_err(|undo| {
                rollback_failed(Some(k), format!("{e}; putting it back: {undo}"))
            })?;
        }
        return Err(AppError::from(e).with_detail("the restored vault could not be put in place"));
    }
    Ok(kept)
}

/// `<vault name> (before restore YYYY-MM-DD HHMM)` beside the vault, made unique when it exists.
fn before_restore_path(live: &Path) -> PathBuf {
    let name = live
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let base = format!(
        "{name} (before restore {})",
        crate::modules::packages::backup::local_stamp()
    );
    let parent = live.parent().unwrap_or(Path::new(""));
    let mut candidate = parent.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base} {n}"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// An unpacked vault file whose checksums match what is in it.
    fn staged() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(MANIFEST_ENTRY), b"{}").unwrap();
        fs::write(dir.path().join(VAULT_DB), b"database").unwrap();
        fs::create_dir(dir.path().join("assets")).unwrap();
        fs::write(dir.path().join("assets").join("a.bin"), b"asset").unwrap();
        let hash = |rel: &str| openframe_security::sha256_file(&dir.path().join(rel)).unwrap();
        let sums = json!({
            "algorithm": "sha256",
            "files": {
                MANIFEST_ENTRY: hash(MANIFEST_ENTRY),
                VAULT_DB: hash(VAULT_DB),
                "assets/a.bin": hash("assets/a.bin"),
            }
        });
        fs::write(dir.path().join(CHECKSUMS_ENTRY), sums.to_string()).unwrap();
        dir
    }

    #[test]
    fn an_unpacked_file_the_checksums_do_not_list_is_refused_before_the_vault_opens() {
        let dir = staged();
        verify_hashes(dir.path()).expect("exactly what the list names");
        for stray in ["vault.sqlite-wal", "assets/extra.bin", "program.exe"] {
            fs::write(dir.path().join(stray), b"x").unwrap();
            let e = verify_hashes(dir.path()).expect_err(stray);
            assert_eq!(e.code_str(), "import.vault_incomplete", "{stray}");
            assert!(e.detail.unwrap_or_default().contains(stray), "{stray}");
            fs::remove_file(dir.path().join(stray)).unwrap();
        }
        verify_hashes(dir.path()).expect("clean again");
    }

    fn folder_with(dir: &Path, name: &str, file: &str) -> PathBuf {
        let p = dir.join(name);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(file), name).unwrap();
        p
    }

    #[test]
    fn put_back_returns_the_kept_vault_and_deletes_the_unwanted_one_last() {
        let dir = tempfile::tempdir().unwrap();
        let live = folder_with(dir.path(), "vault", "new.txt");
        let kept = folder_with(dir.path(), "vault (before restore)", "old.txt");
        put_back(&live, Some(&kept)).unwrap();
        assert!(
            live.join("old.txt").is_file(),
            "the earlier vault is live again"
        );
        assert!(!live.join("new.txt").exists());
        assert_eq!(
            listing(dir.path()),
            vec!["vault".to_string()],
            "nothing left over"
        );

        // With no earlier vault the unwanted one is simply removed.
        put_back(&live, None).unwrap();
        assert!(listing(dir.path()).is_empty());
    }

    /// Hold a folder open the way another program would, so it cannot be renamed.
    #[cfg(windows)]
    fn hold(dir: &Path) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        const BACKUP_SEMANTICS: u32 = 0x0200_0000;
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(BACKUP_SEMANTICS)
            .open(dir)
            .expect("a folder can be held open")
    }

    #[cfg(windows)]
    #[test]
    fn put_back_that_cannot_move_the_new_vault_aside_deletes_nothing_and_says_where_the_old_one_is()
    {
        let dir = tempfile::tempdir().unwrap();
        let live = folder_with(dir.path(), "vault", "new.txt");
        let kept = folder_with(dir.path(), "vault (before restore)", "old.txt");
        let held = hold(&live);
        let e = put_back(&live, Some(&kept)).expect_err("the folder is in use");
        drop(held);
        assert_eq!(e.code_str(), ROLLBACK_FAILED);
        assert!(
            e.message.contains("vault (before restore)"),
            "{}",
            e.message
        );
        assert!(live.join("new.txt").is_file(), "nothing was deleted");
        assert!(
            kept.join("old.txt").is_file(),
            "the earlier vault is untouched"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_swap_that_fails_puts_the_earlier_vault_back() {
        let dir = tempfile::tempdir().unwrap();
        let live = folder_with(dir.path(), "vault", "old.txt");
        let staged = folder_with(dir.path(), "vault.openframe-staging-1", "new.txt");
        let held = hold(&staged);
        let e = swap_in(&live, &staged).expect_err("the staged folder is in use");
        drop(held);
        assert_ne!(e.code_str(), ROLLBACK_FAILED, "the undo worked");
        assert!(
            live.join("old.txt").is_file(),
            "the earlier vault is live again"
        );
        assert!(staged.join("new.txt").is_file());
        assert_eq!(
            listing(dir.path()).len(),
            2,
            "no renamed copy left: {:?}",
            listing(dir.path())
        );
    }
}
