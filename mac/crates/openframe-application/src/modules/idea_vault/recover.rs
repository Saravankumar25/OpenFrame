//! Files in a vault's assets folder that no item uses any more: copies left behind by an add that
//! failed before the fix that removes them (a sync client such as OneDrive can hold a new file just
//! long enough for the add to fail). They can be listed and added back as new items.

use std::collections::HashSet;
use std::path::Path;

use openframe_domain::ids::is_valid_id;
use openframe_domain::{Actor, AppResult, Capability};
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use super::items::{NewItem, insert_item};
use super::model::{VaultOverviewArgs, detect_type};
use super::{mutate_vault, read_vault};
use crate::core::AppCore;
use crate::store::MutationMeta;
use crate::util::adopt_managed_file;

/// How many file names `vault.orphan_files` returns as examples.
const SAMPLES: usize = 5;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultOrphanFiles {
    pub count: u32,
    #[ts(type = "number")]
    pub bytes: u64,
    /// A few of the file names as they are on disk.
    pub samples: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultRecoverResult {
    /// Files that are items again.
    pub added: u32,
    /// Files that could not be read (in use or unreadable); they stay where they are.
    pub failed: u32,
}

struct Orphan {
    rel: String,
    name: String,
    bytes: u64,
}

/// Managed files under `assets/<shard>/` that no asset row points at. Only names OpenFrame gives
/// (`<id>` or `<id>.<ext>`) count, so staging files, temp files and files a sync client or the
/// system put there are ignored. Files a redo would bring back belong to the history and are
/// left alone.
fn find_orphans(conn: &Connection, root: &Path) -> AppResult<Vec<Orphan>> {
    let assets = root.join("assets");
    if !assets.is_dir() {
        return Ok(Vec::new());
    }
    let known: HashSet<String> = conn
        .prepare("SELECT rel_path FROM asset WHERE rel_path IS NOT NULL")?
        .query_map([], |r| r.get::<_, String>(0))?
        .map(|r| r.map(|p| p.replace('\\', "/").to_ascii_lowercase()))
        .collect::<Result<_, _>>()?;
    let redo: Vec<String> = conn
        .prepare("SELECT changes_json FROM sys_undo WHERE state = 'undone'")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    for shard in std::fs::read_dir(&assets)? {
        let shard = shard?;
        if !shard.file_type()?.is_dir() {
            continue;
        }
        let Some(shard_name) = shard.file_name().to_str().map(str::to_string) else {
            continue;
        };
        for file in std::fs::read_dir(shard.path())? {
            let file = file?;
            if !file.file_type()?.is_file() {
                continue;
            }
            let Some(name) = file.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let stem = name.split('.').next().unwrap_or("");
            if !is_valid_id(stem) {
                continue;
            }
            let rel = format!("assets/{shard_name}/{name}");
            if known.contains(&rel.to_ascii_lowercase()) || redo.iter().any(|j| j.contains(&name)) {
                continue;
            }
            out.push(Orphan {
                rel,
                name,
                bytes: file.metadata()?.len(),
            });
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(out)
}

pub(crate) fn orphan_files(
    core: &AppCore,
    actor: &Actor,
    args: VaultOverviewArgs,
) -> AppResult<VaultOrphanFiles> {
    let orphans = read_vault(core, actor, args.store, find_orphans)?;
    Ok(VaultOrphanFiles {
        count: orphans.len() as u32,
        bytes: orphans.iter().map(|o| o.bytes).sum(),
        samples: orphans
            .iter()
            .take(SAMPLES)
            .map(|o| o.name.clone())
            .collect(),
    })
}

/// Add every orphan file back as a new item, in one undoable change. The files stay where they
/// are; a file that cannot be read is skipped and stays listed.
pub(crate) fn recover_files(
    core: &AppCore,
    actor: &Actor,
    args: VaultOverviewArgs,
) -> AppResult<VaultRecoverResult> {
    let (added, failed) = mutate_vault(
        core,
        actor,
        args.store,
        MutationMeta::new(
            "vault.recover_files",
            "Added back files found in the vault folder",
            Capability::Edit,
        ),
        |tx| {
            // Scanned inside the change, so a file added meanwhile is never recorded twice.
            let orphans = find_orphans(tx.conn(), tx.root())?;
            let (mut added, mut failed) = (0u32, 0u32);
            for o in &orphans {
                // The original name is gone; the file is named after what it is.
                let name = match Path::new(&o.name).extension().and_then(|e| e.to_str()) {
                    Some(ext) => format!("Recovered file {}.{ext}", added + failed + 1),
                    None => format!("Recovered file {}", added + failed + 1),
                };
                let one = tx.savepoint(|| {
                    let asset = adopt_managed_file(tx, &o.rel, &name)?;
                    let item = NewItem {
                        item_type: Some(detect_type(&asset.original_name, &asset.media_type)),
                        asset_id: Some(asset.id),
                        ..Default::default()
                    };
                    insert_item(tx, &item, None, &[])
                });
                match one {
                    Ok(_) => added += 1,
                    Err(e)
                        if matches!(
                            e.code_str(),
                            "storage.disk_full" | "storage.write_failed" | "storage.busy"
                        ) =>
                    {
                        return Err(e);
                    }
                    Err(_) => failed += 1,
                }
            }
            Ok((added, failed))
        },
    )?;
    Ok(VaultRecoverResult { added, failed })
}
