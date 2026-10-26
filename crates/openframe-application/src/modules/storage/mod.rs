//! Where things live (doc 40, ADR-0038): the library file and the person's durable identity,
//! the folder for new projects, projects found on this computer that Recent Projects does not
//! list, and private notes left behind by people who are no longer on a project.
//!
//! Every operation here is a direct user action in Settings or Project Settings. None of them is
//! offered to the assistant, which never chooses or touches file system locations.

pub mod library;
pub mod relocate;
mod tree;
pub mod vault_archive;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id};
use openframe_project_format::{self as pf, ProjectLayout};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::packages::backup::adopt_private_notes;
use super::project::{Empty, data_modified_at};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::MutationMeta;
use relocate::{LocateVaultArgs, MoveToFolderArgs};
use vault_archive::{VaultArchiveKind, VaultFileArgs};

/// A staging folder is `<target>.openframe-staging-<millis>` (doc 40 section 5).
const STAGING_MARKER: &str = ".openframe-staging-";
/// Project folders are looked for two levels down; this bounds the work on a huge folder.
const DISCOVER_MAX_FOLDERS: usize = 5_000;
const DISCOVER_MAX_RESULTS: usize = 200;

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.module("Storage");
    r.query("storage.info", info).meta(
        M::app_query("Where the library, the projects folder, the vault and the app data are.")
            .fs(Fs::ProjectStorage)
            .hidden(h::ASSET_PATH),
    );
    r.command("storage.set_projects_dir", set_projects_dir)
        .meta(
        M::app_command(
            "Choose the default folder for new projects. Existing projects stay where they are.",
        )
        .fs(Fs::ProjectStorage)
        .hidden(h::USER_PATH),
    );
    r.query("project.discover", discover).meta(
        M::app_query("Projects in the projects folder that Recent Projects does not list.")
            .fs(Fs::ProjectStorage)
            .hidden(h::APP_LIFECYCLE),
    );
    r.command("project.move", project_move).meta(
        M::command(
            Capability::ManageProject,
            "Move the open project to another folder, keeping everything in it.",
        )
        .fs(Fs::ProjectStorage)
        .long_running()
        .destructive()
        .confirm()
        .hidden(h::APP_LIFECYCLE),
    );
    r.command("vault.backup", vault_backup).meta(
        M::app_command("Save a backup of the Global Idea Vault to a file the user picked.")
            .fs(Fs::WritesUserFile)
            .long_running()
            .hidden(h::USER_PATH),
    );
    r.command("vault.export_package", vault_export_package)
        .meta(
            M::app_command(
                "Save the Global Idea Vault for another computer, without Recently Deleted items.",
            )
            .fs(Fs::WritesUserFile)
            .long_running()
            .hidden(h::USER_PATH),
        );
    r.query("vault.inspect", vault_inspect).meta(
        M::app_query("What a vault backup file holds, before it is restored.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    // `vault.restore` already brings an item back from Recently Deleted.
    r.command("vault.restore_backup", vault_restore).meta(
        M::app_command(
            "Replace the Global Idea Vault with a backup file; the current vault is kept beside it.",
        )
        .fs(Fs::ProjectStorage)
        .long_running()
        .destructive()
        .confirm()
        .hidden(h::USER_PATH),
    );
    r.command("vault.move", vault_move).meta(
        M::app_command("Move the Global Idea Vault to another folder.")
            .fs(Fs::ProjectStorage)
            .long_running()
            .destructive()
            .confirm()
            .hidden(h::USER_PATH),
    );
    r.command("vault.locate", vault_locate).meta(
        M::app_command("Connect a folder that already holds a Global Idea Vault.")
            .fs(Fs::ProjectStorage)
            .hidden(h::USER_PATH),
    );
    r.query("project.orphan_notes", orphan_notes).meta(
        M::query(
            Capability::ManageProject,
            "People who left private notes in this project and are no longer on it.",
        )
        .hidden(h::PRIVATE_BOUNDARY),
    );
    r.command("project.adopt_orphan_notes", adopt_orphan_notes)
        .meta(
        M::command(
            Capability::ManageProject,
            "The Owner takes over the private notes of one person who is no longer on the project.",
        )
        .hidden(h::PRIVATE_BOUNDARY),
    );
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    /// Settings, recents, AI records, logs and models (removed with the app data).
    pub app_data_dir: String,
    /// The folder to back up: it holds the library file.
    pub library_dir: String,
    pub library_file: Option<String>,
    /// Where new projects go.
    pub projects_dir: String,
    pub projects_dir_is_default: bool,
    pub vault_dir: String,
    pub vault_dir_is_default: bool,
    /// False when the vault folder has no vault in it (a drive that is not connected, or a vault
    /// not created yet).
    pub vault_exists: bool,
    /// The open project's folder.
    pub project_dir: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetProjectsDirArgs {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredProject {
    pub path: String,
    pub title: String,
    pub project_id: String,
    #[ts(type = "number | null")]
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct OrphanNoteOwner {
    pub owner_user_id: String,
    #[ts(type = "number")]
    pub note_count: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdoptOrphanNotesArgs {
    pub owner_user_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedNotes {
    /// How many notes changed owner.
    pub adopted: u32,
}

/// A long operation was started; its progress and result arrive as task events (the result is
/// the report of that operation).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorageTaskStarted {
    pub task_id: String,
}

// -------------------------------------------------------------------- info

fn info(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<StorageInfo> {
    Ok(storage_info(core))
}

fn storage_info(core: &AppCore) -> StorageInfo {
    let text = |p: &Path| p.to_string_lossy().into_owned();
    let (projects, vault) = (core.projects_dir(), core.vault_dir());
    StorageInfo {
        app_data_dir: text(&core.config.app_data_dir),
        library_dir: text(
            core.config
                .library_file
                .as_deref()
                .and_then(Path::parent)
                .unwrap_or(&core.data_root()),
        ),
        library_file: core.config.library_file.as_deref().map(text),
        projects_dir_is_default: library::same_place(&projects, &core.config.projects_dir),
        projects_dir: text(&projects),
        vault_dir_is_default: library::same_place(&vault, &core.config.global_vault_dir),
        vault_exists: vault.join("vault.sqlite").is_file(),
        vault_dir: text(&vault),
        project_dir: core.project_opt().map(|s| text(s.layout.root())),
    }
}

// ------------------------------------------------------- projects folder

fn set_projects_dir(
    core: &AppCore,
    _a: &Actor,
    args: SetProjectsDirArgs,
) -> AppResult<StorageInfo> {
    let dir = checked_folder(core, &args.path)?;
    let default = library::same_place(&dir, &core.config.projects_dir);
    core.change_locations(|l| l.projects_dir = (!default).then_some(dir))?;
    Ok(storage_info(core))
}

/// A folder the person chose to receive something (new projects, a moved vault or project).
fn checked_folder(core: &AppCore, raw: &str) -> AppResult<PathBuf> {
    let dir = PathBuf::from(raw.trim());
    // Absolute, well formed and local first, so a relative or hostile path never reaches the disk.
    openframe_security::check_user_path(&dir, openframe_security::NetworkPaths::Refuse)
        .map_err(crate::modules::project::network_or)?;
    if !dir.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    refuse_unsuitable_folder(core, &dir)?;
    probe_writable(&dir)?;
    Ok(dir)
}

/// Not inside app data, the vault or the open project (the same rule as a project's parent), and
/// not inside a project folder.
fn refuse_unsuitable_folder(core: &AppCore, dir: &Path) -> AppResult<()> {
    crate::util::check_project_parent(core, dir).map_err(crate::modules::project::network_or)?;
    refuse_inside_project(dir)
}

/// The vault folder, when a vault is there. Otherwise the plain reason: not created yet, or the
/// chosen folder is missing (a drive that is not connected).
fn existing_vault(core: &AppCore, what: &str) -> AppResult<PathBuf> {
    let dir = core.vault_dir();
    if dir.join("vault.sqlite").is_file() {
        return Ok(dir);
    }
    if library::same_place(&dir, &core.config.global_vault_dir) {
        return Err(AppError::new(
            "not_found.vault",
            format!(
                "Your Global Idea Vault doesn't exist yet, so there is nothing to {what}. It is created when you add your first idea."
            ),
        ));
    }
    Err(crate::core::vault_missing_error(&dir))
}

/// A project folder holds one project; a folder for new projects must not be inside one.
fn refuse_inside_project(dir: &Path) -> AppResult<()> {
    let resolved = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    for place in [dir, resolved.as_path()] {
        for folder in place.ancestors() {
            let named = folder
                .file_name()
                .is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with(".openframe"));
            if named || folder.join(pf::MANIFEST_FILE).is_file() {
                return Err(AppError::invalid_input(
                    "That folder is inside a project. Choose a folder outside your projects.",
                ));
            }
        }
    }
    Ok(())
}

/// Create and delete a small file: the only honest test that the person can save there.
fn probe_writable(dir: &Path) -> AppResult<()> {
    let probe = dir.join(format!(".openframe-write-test-{}", new_id()));
    let result = std::fs::File::create_new(&probe).map(|_| ());
    // Removed even after a partial failure; a leftover test file would be clutter in their folder.
    let _ = std::fs::remove_file(&probe);
    result.map_err(|e| {
        AppError::new(
            "storage.not_writable",
            "OpenFrame can't save files in that folder. Choose a different folder.",
        )
        .with_detail(e.to_string())
    })
}

// -------------------------------------------------------------- discovery

fn discover(core: &AppCore, _a: &Actor, _: Empty) -> AppResult<Vec<DiscoveredProject>> {
    let (ids, paths) = core.with_app_db(|c| {
        let mut stmt = c.prepare("SELECT project_id, path FROM sys_recent_project")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        let ids: HashSet<String> = rows.iter().map(|(id, _)| id.clone()).collect();
        let paths: HashSet<String> = rows
            .iter()
            .map(|(_, p)| library::place_key(Path::new(p)))
            .collect();
        Ok((ids, paths))
    })?;
    let mut found = Vec::new();
    let mut visited = 0usize;
    // A folder with a manifest is a project and is not searched further; any other folder is
    // searched once more, one level down.
    let mut level = vec![core.projects_dir()];
    'levels: for depth in 0..2 {
        let mut next = Vec::new();
        for parent in &level {
            let Ok(entries) = std::fs::read_dir(parent) else {
                continue;
            };
            for entry in entries.flatten() {
                // Links are not followed: a project folder is a real folder.
                if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                visited += 1;
                if visited > DISCOVER_MAX_FOLDERS {
                    break 'levels;
                }
                let path = entry.path();
                let layout = ProjectLayout::new(&path);
                match pf::read_manifest(&layout) {
                    Ok(m) => {
                        if !ids.contains(&m.project_id)
                            && !paths.contains(&library::place_key(&path))
                        {
                            found.push(DiscoveredProject {
                                path: path.to_string_lossy().into_owned(),
                                title: m.title,
                                project_id: m.project_id,
                                modified_at: data_modified_at(&layout),
                            });
                        }
                    }
                    Err(_) if depth == 0 && !layout.manifest().exists() => next.push(path),
                    Err(_) => {}
                }
            }
        }
        level = next;
    }
    found.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    found.truncate(DISCOVER_MAX_RESULTS);
    Ok(found)
}

// ------------------------------------------------- vault and project files

fn vault_backup(core: &AppCore, _a: &Actor, args: VaultFileArgs) -> AppResult<StorageTaskStarted> {
    vault_archive::start_archive(core, args, VaultArchiveKind::Backup)
}

fn vault_export_package(
    core: &AppCore,
    _a: &Actor,
    args: VaultFileArgs,
) -> AppResult<StorageTaskStarted> {
    vault_archive::start_archive(core, args, VaultArchiveKind::Package)
}

fn vault_inspect(
    core: &AppCore,
    actor: &Actor,
    args: VaultFileArgs,
) -> AppResult<vault_archive::VaultInspection> {
    vault_archive::inspect(core, actor, args)
}

fn vault_restore(core: &AppCore, _a: &Actor, args: VaultFileArgs) -> AppResult<StorageTaskStarted> {
    vault_archive::start_restore(core, args)
}

fn vault_move(core: &AppCore, _a: &Actor, args: MoveToFolderArgs) -> AppResult<StorageTaskStarted> {
    relocate::start_move_vault(core, args)
}

fn vault_locate(core: &AppCore, _a: &Actor, args: LocateVaultArgs) -> AppResult<StorageInfo> {
    relocate::locate_vault(core, args)
}

fn project_move(
    core: &AppCore,
    actor: &Actor,
    args: MoveToFolderArgs,
) -> AppResult<StorageTaskStarted> {
    relocate::start_move_project(core, actor, args)
}

// ---------------------------------------------------------- orphan notes

fn orphan_notes(core: &AppCore, actor: &Actor, _: Empty) -> AppResult<Vec<OrphanNoteOwner>> {
    actor.require(Capability::ManageProject, "see who left private notes")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT owner_user_id, count(*) FROM private_note
             WHERE deleted_at IS NULL AND owner_user_id <> ?1
               AND owner_user_id NOT IN (SELECT user_id FROM project_member WHERE deleted_at IS NULL)
             GROUP BY owner_user_id ORDER BY owner_user_id",
        )?;
        let rows = stmt
            .query_map([&actor.user_id], |r| {
                Ok(OrphanNoteOwner {
                    owner_user_id: r.get(0)?,
                    note_count: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn adopt_orphan_notes(
    core: &AppCore,
    actor: &Actor,
    args: AdoptOrphanNotesArgs,
) -> AppResult<AdoptedNotes> {
    actor.require(Capability::ManageProject, "take over private notes")?;
    let owner = args.owner_user_id.trim();
    if owner.is_empty() {
        return Err(AppError::required("The person"));
    }
    if owner == actor.user_id {
        return Err(AppError::invalid_input("Those notes are already yours."));
    }
    let s = core.project()?;
    // The notes become the Owner's, so this is not an undo step: undoing it would hand the notes
    // back to someone who is not here. The activity entry says it happened, never what is in them.
    let meta = MutationMeta::new(
        "project.adopt_orphan_notes",
        "Took over the private notes of a former member",
        Capability::ManageProject,
    )
    .not_undoable();
    let adopted = s.store.mutate(actor, meta, |tx| {
        let c = tx.conn();
        let active: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_member WHERE user_id=?1 AND deleted_at IS NULL)",
            [owner],
            |r| r.get(0),
        )?;
        if active {
            return Err(AppError::conflict(
                "That person is still on this project, so their private notes stay theirs.",
            ));
        }
        adopt_private_notes(c, &actor.user_id, Some(owner))
    })?;
    if adopted == 0 {
        return Err(AppError::new(
            "not_found.private_notes",
            "That person has no private notes in this project.",
        ));
    }
    Ok(AdoptedNotes { adopted })
}

// ----------------------------------------------------- interrupted work

/// Remove staging folders an interrupted move or restore left in these folders (doc 40 section
/// 5). Only folders named `<name>.openframe-staging-<digits>` are touched, and links are never
/// followed. Best effort: what cannot be removed now is tried again at the next start.
pub(crate) fn remove_staging_leftovers(dirs: &[&Path]) {
    let mut seen = HashSet::new();
    let (mut removed, mut failed) = (0u32, 0u32);
    for dir in dirs {
        if !seen.insert(library::place_key(dir)) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_staging = name.rsplit_once(STAGING_MARKER).is_some_and(|(_, millis)| {
                !millis.is_empty() && millis.bytes().all(|b| b.is_ascii_digit())
            });
            if !is_staging || !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            match std::fs::remove_dir_all(entry.path()) {
                Ok(()) => removed += 1,
                Err(_) => failed += 1,
            }
        }
    }
    if removed + failed > 0 {
        tracing::info!(
            removed,
            failed,
            "removed staging folders left by an interrupted move or restore"
        );
    }
}
