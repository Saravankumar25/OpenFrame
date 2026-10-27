//! What a folder picked for "Locate Existing Vault" holds, read before anything changes: a vault,
//! the folder around a vault (the OpenFrame folder that holds "Global Idea Vault"), a project, or
//! nothing. The counts come from the vault database opened read-only, so the person sees what
//! they are about to connect and an empty or wrong vault is never connected by accident.

use std::path::{Path, PathBuf};
use std::time::Duration;

use openframe_domain::{Actor, AppError, AppResult};
use openframe_project_format as pf;
use rusqlite::OpenFlags;
use serde::Serialize;
use ts_rs::TS;

use super::library;
use super::relocate::LocateVaultArgs;
use crate::core::AppCore;
use crate::modules::project;
use crate::registry::Registry;

const VAULT_DB: &str = "vault.sqlite";
/// Bounds the look at the folders inside the chosen one.
const MAX_CHILDREN: usize = 500;

pub(super) fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.query("vault.inspect_folder", inspect_folder).meta(
        M::app_query("What a folder holds before it is connected as the Global Idea Vault.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum VaultFolderKind {
    /// The folder holds `vault.sqlite`.
    Vault,
    /// A folder inside it holds `vault.sqlite`.
    ParentOfVault,
    /// An OpenFrame project, not a vault.
    Project,
    /// No vault here.
    Nothing,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultFolderInspection {
    pub kind: VaultFolderKind,
    /// The vault folder to connect: the chosen folder or the vault inside it.
    pub vault_dir: Option<String>,
    /// Ideas not in Recently Deleted.
    pub ideas: u32,
    /// Files those ideas use that are in the vault folder.
    pub files_present: u32,
    /// Files those ideas use that are not in the vault folder.
    pub files_missing: u32,
    /// This is the vault OpenFrame uses now.
    pub active: bool,
}

fn inspect_folder(
    core: &AppCore,
    _a: &Actor,
    args: LocateVaultArgs,
) -> AppResult<VaultFolderInspection> {
    let dir = PathBuf::from(args.dir.trim());
    openframe_security::check_user_path(&dir, openframe_security::NetworkPaths::Refuse)
        .map_err(project::network_or)?;
    if !dir.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    let empty = |kind| VaultFolderInspection {
        kind,
        vault_dir: None,
        ideas: 0,
        files_present: 0,
        files_missing: 0,
        active: false,
    };
    if dir.join(pf::MANIFEST_FILE).is_file() {
        return Ok(empty(VaultFolderKind::Project));
    }
    let (kind, vault) = if dir.join(VAULT_DB).is_file() {
        (VaultFolderKind::Vault, dir)
    } else if let Some(child) = vault_inside(&dir, &core.config.global_vault_dir) {
        (VaultFolderKind::ParentOfVault, child)
    } else {
        return Ok(empty(VaultFolderKind::Nothing));
    };
    let (ideas, files_present, files_missing) = count(&vault)?;
    Ok(VaultFolderInspection {
        kind,
        active: library::same_place(&vault, &core.vault_dir()),
        vault_dir: Some(vault.to_string_lossy().into_owned()),
        ideas,
        files_present,
        files_missing,
    })
}

/// A folder directly inside `dir` that holds a vault, preferring the default vault folder name.
fn vault_inside(dir: &Path, default_vault: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .take(MAX_CHILDREN)
        .map(|e| e.path())
        .filter(|p| p.join(VAULT_DB).is_file())
        .collect();
    found.sort();
    let default_name = default_vault.file_name();
    let preferred = found.iter().position(|p| p.file_name() == default_name);
    match preferred {
        Some(i) => Some(found.swap_remove(i)),
        None => found.into_iter().next(),
    }
}

/// Ideas, files present and files missing, read without changing the vault.
fn count(vault: &Path) -> AppResult<(u32, u32, u32)> {
    let not_a_vault = |e: rusqlite::Error| {
        AppError::new(
            "storage.not_a_vault",
            "That folder has a vault file OpenFrame can't read. It may be damaged or from a newer version of OpenFrame.",
        )
        .with_detail(e.to_string())
    };
    let conn = openframe_persistence::open_sqlite(
        &vault.join(VAULT_DB),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(not_a_vault)?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(not_a_vault)?;
    let ideas: u32 = conn
        .query_row(
            "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL",
            [],
            |r| r.get(0),
        )
        .map_err(not_a_vault)?;
    let rels: Vec<String> = conn
        .prepare(
            "SELECT DISTINCT a.rel_path FROM vault_item v JOIN asset a ON a.id = v.asset_id
             WHERE v.deleted_at IS NULL AND a.storage_mode = 'managed' AND a.rel_path IS NOT NULL",
        )
        .and_then(|mut s| s.query_map([], |r| r.get(0))?.collect())
        .map_err(not_a_vault)?;
    let present = rels
        .iter()
        .filter(|rel| openframe_security::confine(vault, rel).is_ok_and(|p| p.is_file()))
        .count() as u32;
    Ok((ideas, present, rels.len() as u32 - present))
}
