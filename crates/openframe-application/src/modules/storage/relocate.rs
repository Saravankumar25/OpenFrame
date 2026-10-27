//! Moving the Global Idea Vault and the open project to another folder, and connecting a vault
//! folder that already exists (doc 40 sections 4 and 5).
//!
//! A move copies into a staging folder named `<target>.openframe-staging-<millis>` beside the
//! destination, checks every file (size and SHA-256) and the database, renames the staging folder
//! into place, switches the app to it, confirms that it opens, and only then deletes the source.
//! Until the switch the source stays the live copy, and a failure puts everything back.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use openframe_project_format::{self as pf, ProjectLayout};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::vault_archive::check_vault_database;
use super::{
    ROLLBACK_FAILED, STAGING_MARKER, StagingNote, StorageInfo, StorageTaskStarted, checked_folder,
    existing_vault, library, tree,
};
use crate::core::{AppCore, ProjectSession, StorageOp, TaskHandle, storage_busy};
use crate::modules::packages::format::Staging;
use crate::modules::project;

/// Folders of a project that a move leaves behind: the lock belongs to this location and the
/// cache is rebuilt on demand.
const PROJECT_SKIP: &[&str] = &["cache", ".lock"];

/// The vault's safety copies (made before an upgrade) are not copied and not removed by a move.
const VAULT_SKIP: &[&str] = &["backups"];

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveToFolderArgs {
    /// The folder that will hold the moved folder (the moved folder keeps its name).
    pub dest_parent: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocateVaultArgs {
    /// A folder that already holds a `vault.sqlite`.
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultMoveReport {
    pub path: String,
    /// Set when the old folder still holds something: the vault's safety backups (never copied,
    /// never deleted by a move) or a file that could not be removed.
    pub source_left_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMoveReport {
    pub path: String,
    /// Files the project links to (not copies of). They stay where they were.
    pub linked_files: u32,
    pub source_left_at: Option<String>,
}

fn same_place_error(what: &str) -> AppError {
    AppError::new(
        "conflict.same_place",
        format!("{what} is already in that folder."),
    )
}

/// The destination must not exist, or be an empty folder (which the move then replaces).
fn check_destination_free(dest: &Path, allow_empty_folder: bool) -> AppResult<()> {
    if !dest.exists() {
        return Ok(());
    }
    let empty_folder = allow_empty_folder && dest.is_dir() && fs::read_dir(dest)?.next().is_none();
    if empty_folder {
        return Ok(());
    }
    Err(AppError::new(
        "conflict.exists",
        format!(
            "There is already something named “{}” in that folder. Choose another folder, or rename or remove it first.",
            dest.file_name().unwrap_or_default().to_string_lossy()
        ),
    ))
}

fn staging_path(parent: &Path, name: &OsString) -> PathBuf {
    parent.join(format!(
        "{}{STAGING_MARKER}{}",
        name.to_string_lossy(),
        now_ms()
    ))
}

// ------------------------------------------------------------------ vault.move

pub(crate) fn start_move_vault(
    core: &AppCore,
    args: MoveToFolderArgs,
) -> AppResult<StorageTaskStarted> {
    let guard = core.begin_storage_op(StorageOp::Vault)?;
    let parent = checked_folder(core, &args.dest_parent)?;
    let vault = existing_vault(core, "move")?;
    let Some(name) = vault.file_name().map(OsString::from) else {
        return Err(AppError::invalid_input(
            "The vault folder is the top folder of a drive, so it can't be moved.",
        ));
    };
    let dest = parent.join(&name);
    if library::same_place(&dest, &vault) {
        return Err(same_place_error("Your Global Idea Vault"));
    }
    if library::is_inside(&dest, &vault) {
        return Err(AppError::invalid_input(
            "Choose a folder that is not your vault folder or inside it.",
        ));
    }
    check_destination_free(&dest, true)?;
    // The backups folder (safety copies made before an upgrade) stays behind.
    tree::ensure_space(&parent, tree::walk(&vault, VAULT_SKIP)?.bytes)?;
    let task_id = core.spawn_task("storage.vault_move", "Moving the vault", move |core, h| {
        let _guard = guard;
        let report = move_vault(&core, &vault, &parent, &dest, &name, &h)?;
        serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
    });
    Ok(StorageTaskStarted { task_id })
}

/// The undo of a move or a connect did not work. Says where the data is.
fn rollback_failed(message: String, detail: impl Into<String>) -> AppError {
    AppError::new(ROLLBACK_FAILED, message).with_detail(detail)
}

fn move_vault(
    core: &AppCore,
    vault: &Path,
    parent: &Path,
    dest: &Path,
    name: &OsString,
    h: &TaskHandle,
) -> AppResult<VaultMoveReport> {
    let progress = |f: f64, m: &str| h.progress(f, m);
    let cancelled = || h.is_cancelled();
    let staging_path = staging_path(parent, name);
    // Declared before the staging folder, so it is dropped after it.
    let _note = StagingNote::record(core, &staging_path);
    let staging = Staging::at(staging_path);
    let mut previous: Option<PathBuf> = None;
    progress(0.02, "Closing the vault");
    // Closed and kept closed for the whole copy, so nothing is written to the old files after
    // they were copied.
    let source = core.with_vault_closed(|| {
        // Read again now that nothing can change it: the vault is still where this move planned.
        if !library::same_place(&core.vault_dir(), vault) {
            return Err(storage_busy(
                "Your Global Idea Vault was moved or reconnected while this move was starting. Nothing was changed. Try the move again.",
            ));
        }
        let source = tree::walk(vault, VAULT_SKIP)?;
        fs::create_dir_all(staging.path())?;
        let copied = tree::copy(&source, staging.path(), &progress, &cancelled)?;
        tree::verify(staging.path(), &copied, &progress, &cancelled)?;
        check_vault_database(&staging.path().join("vault.sqlite"))?;
        if h.is_cancelled() {
            return Err(AppError::cancelled());
        }
        check_destination_free(dest, true)?;
        if dest.exists() {
            fs::remove_dir(dest)?;
        }
        fs::rename(staging.path(), dest)?;
        let default = library::same_place(dest, &core.config.global_vault_dir);
        let changed = core.change_locations(|l| {
            previous = l.vault_dir.clone();
            l.vault_dir = (!default).then(|| dest.to_path_buf());
        });
        if let Err(e) = changed {
            // The places did not change (a failed write leaves them as they were), the old vault
            // is still live and complete, so the verified copy is not needed.
            tree::discard(dest);
            return Err(e);
        }
        Ok(source)
    })?;
    progress(0.9, "Opening the vault in its new place");
    if let Err(e) = core.global_store() {
        // The places go back first; the copy is deleted only after that worked, so the places
        // never name a folder that is gone.
        let undone = core.with_vault_closed(|| {
            core.change_locations(|l| l.vault_dir = previous.clone())
                .map_err(|r| both_places_error(vault, dest, r.message))?;
            tree::discard(dest);
            Ok(())
        });
        if let Err(r) = undone {
            return Err(if r.code_str() == ROLLBACK_FAILED {
                r
            } else {
                both_places_error(vault, dest, r.message)
            });
        }
        return Err(e);
    }
    progress(0.95, "Removing the old folder");
    // The safety backups were not copied, so they stay where they are and the folder is reported.
    let left = !tree::remove_source(vault, &source, &[], VAULT_SKIP);
    Ok(VaultMoveReport {
        path: dest.to_string_lossy().into_owned(),
        source_left_at: left.then(|| vault.to_string_lossy().into_owned()),
    })
}

/// A moved vault would not open and the places could not be switched back: both folders hold the
/// vault, and neither is deleted.
fn both_places_error(old: &Path, new: &Path, detail: String) -> AppError {
    rollback_failed(
        format!(
            "Your Global Idea Vault was copied to “{}” but would not open there, and OpenFrame could not switch back. Your vault is safe in both “{}” and “{}”. Go to Settings > Storage and choose the folder you want.",
            new.display(),
            old.display(),
            new.display()
        ),
        detail,
    )
}

// ----------------------------------------------------------------- vault.locate

pub(crate) fn locate_vault(core: &AppCore, args: LocateVaultArgs) -> AppResult<StorageInfo> {
    let _guard = core.begin_storage_op(StorageOp::Vault)?;
    let dir = PathBuf::from(args.dir.trim());
    openframe_security::check_user_path(&dir, openframe_security::NetworkPaths::Refuse)
        .map_err(project::network_or)?;
    if !dir.is_dir() {
        return Err(AppError::new(
            "not_found.folder",
            "That folder can't be found. If it's on an external drive, reconnect it and try again.",
        ));
    }
    if dir.join(pf::MANIFEST_FILE).is_file() {
        return Err(AppError::invalid_input(
            "That folder is a project, not a vault. Choose the folder that holds your Global Idea Vault.",
        ));
    }
    if !dir.join("vault.sqlite").is_file() {
        return Err(AppError::new(
            "not_found.vault",
            "There is no Global Idea Vault in that folder. Choose the folder that holds your vault.",
        ));
    }
    let current = core.vault_dir();
    let same = library::same_place(&dir, &current);
    if !same {
        super::refuse_unsuitable_folder(core, &dir)?;
    }
    check_vault_database(&dir.join("vault.sqlite"))?;
    if !same {
        let default = library::same_place(&dir, &core.config.global_vault_dir);
        let mut previous = None;
        core.with_vault_closed(|| {
            core.change_locations(|l| {
                previous = l.vault_dir.clone();
                l.vault_dir = (!default).then(|| dir.clone());
            })
        })?;
        // Opened for real before the change counts; otherwise the old place comes back.
        if let Err(e) = core.global_store() {
            // Nothing was copied or deleted; only the place changes back. When even that fails
            // the person is told where the vault is.
            let back = core.with_vault_closed(|| core.change_locations(|l| l.vault_dir = previous));
            if let Err(r) = back {
                return Err(rollback_failed(
                    format!(
                        "OpenFrame could not open the vault in “{}” and could not switch back to your earlier vault folder “{}”. Go to Settings > Storage and choose the vault folder.",
                        dir.display(),
                        current.display()
                    ),
                    r.message,
                ));
            }
            return Err(e);
        }
    }
    Ok(super::storage_info(core))
}

// ----------------------------------------------------------------- project.move

pub(crate) fn start_move_project(
    core: &AppCore,
    actor: &Actor,
    args: MoveToFolderArgs,
) -> AppResult<StorageTaskStarted> {
    actor.require(Capability::ManageProject, "move this project")?;
    // From here until the task ends no project can be opened, created or closed (doc 40 section 5).
    let guard = core.begin_storage_op(StorageOp::ProjectMove)?;
    let session = core.project()?;
    let root = session.layout.root().to_path_buf();
    let Some(name) = root.file_name().map(OsString::from) else {
        return Err(AppError::invalid_input("This project can't be moved."));
    };
    let parent = checked_folder(core, &args.dest_parent)?;
    let dest = parent.join(&name);
    if library::same_place(&dest, &root)
        || root
            .parent()
            .is_some_and(|p| library::same_place(p, &parent))
    {
        return Err(same_place_error("This project"));
    }
    check_destination_free(&dest, false)?;
    tree::ensure_space(&parent, tree::walk(&root, PROJECT_SKIP)?.bytes)?;
    let linked: u32 = session.store.read(|c| {
        Ok(c.query_row(
            "SELECT count(*) FROM asset WHERE storage_mode = 'external'",
            [],
            |r| r.get(0),
        )?)
    })?;
    let actor = actor.clone();
    let task_id = core.spawn_task(
        "storage.project_move",
        "Moving the project",
        move |core, h| {
            let _guard = guard;
            let report = move_project(&core, &actor, session, &parent, &dest, &name, linked, &h)?;
            serde_json::to_value(&report).map_err(|e| AppError::internal(e.to_string()))
        },
    );
    Ok(StorageTaskStarted { task_id })
}

#[allow(clippy::too_many_arguments)]
fn move_project(
    core: &AppCore,
    actor: &Actor,
    session: Arc<ProjectSession>,
    parent: &Path,
    dest: &Path,
    name: &OsString,
    linked_files: u32,
    h: &TaskHandle,
) -> AppResult<ProjectMoveReport> {
    let progress = |f: f64, m: &str| h.progress(f, m);
    let cancelled = || h.is_cancelled();
    let root = session.layout.root().to_path_buf();
    let id = session.project_id();
    // Back to where it was, whatever went wrong. A failure here is logged: the original error
    // is the one the person needs to see. (Not the guarded open: this task holds the guard.)
    let reopen_old = || {
        if let Err(e) = project::open_at_unguarded(core, actor, &root) {
            tracing::error!(
                code = e.code_str(),
                "the project could not be reopened after a failed move"
            );
        }
    };
    progress(0.02, "Saving and closing the project");
    // The same clean close as Close Project: the data is folded into its file and the crash
    // marker cleared. The files are then complete and nothing writes to them.
    if let Err(e) = project::close_current(core) {
        tracing::warn!(
            code = e.code_str(),
            "the project did not close cleanly before a move"
        );
    }
    let give_up = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while Arc::strong_count(&session) > 1 {
        if std::time::Instant::now() >= give_up {
            drop(session);
            reopen_old();
            return Err(storage_busy(
                "The project is in use right now. Wait a moment and try again. Nothing was moved.",
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(session);

    let staging_path = staging_path(parent, name);
    // Declared before the staging folder, so it is dropped after it.
    let _note = StagingNote::record(core, &staging_path);
    let staging = Staging::at(staging_path);
    let copied = (|| -> AppResult<tree::Tree> {
        let source = tree::walk(&root, PROJECT_SKIP)?;
        fs::create_dir_all(staging.path())?;
        let copied = tree::copy(&source, staging.path(), &progress, &cancelled)?;
        tree::verify(staging.path(), &copied, &progress, &cancelled)?;
        check_project_copy(staging.path(), &id)?;
        if h.is_cancelled() {
            return Err(AppError::cancelled());
        }
        check_destination_free(dest, false)?;
        fs::rename(staging.path(), dest)?;
        if let Err(e) = set_recent_path(core, &id, dest) {
            // Recent Projects still names the old folder, which is complete.
            tree::discard(dest);
            return Err(e);
        }
        Ok(source)
    })();
    let source = match copied {
        Ok(source) => source,
        Err(e) => {
            drop(staging);
            reopen_old();
            return Err(e);
        }
    };
    progress(0.92, "Opening the project in its new place");
    // The old folder is deleted only after the session is proven to be on the new one.
    let opened =
        project::open_at_unguarded(core, actor, dest).and_then(|_| confirm_open_at(core, dest));
    if let Err(e) = opened {
        // Whatever opened at the new place is closed first, so nothing is deleted under it.
        if core
            .project_opt()
            .is_some_and(|s| library::same_place(s.layout.root(), dest))
            && let Err(c) = project::close_current(core)
        {
            tracing::warn!(code = c.code_str(), "the project did not close cleanly");
        }
        let recent_back = set_recent_path(core, &id, &root);
        reopen_old();
        let on_old = core
            .project_opt()
            .is_some_and(|s| library::same_place(s.layout.root(), &root));
        // The copy goes only when Recent Projects name the old folder again (set back, or set by
        // opening it): otherwise the copy is the one thing the list points at.
        if recent_back.is_ok() || on_old {
            tree::discard(dest);
            return Err(e);
        }
        return Err(rollback_failed(
            format!(
                "Your project was copied to “{}” but would not open there, and OpenFrame could not switch back. Your project is safe in both “{}” and “{}”. Use Open Project to choose the one you want.",
                dest.display(),
                root.display(),
                dest.display()
            ),
            e.message,
        ));
    }
    progress(0.96, "Removing the old folder");
    let left = !tree::remove_source(&root, &source, PROJECT_SKIP, &[]);
    Ok(ProjectMoveReport {
        path: dest.to_string_lossy().into_owned(),
        linked_files,
        source_left_at: left.then(|| root.to_string_lossy().into_owned()),
    })
}

/// The open project is the one at the new place. Opening a project whose id is already open does
/// nothing, so a session that stayed on the old folder would look like a success.
fn confirm_open_at(core: &AppCore, dest: &Path) -> AppResult<()> {
    match core.project_opt() {
        Some(s) if library::same_place(s.layout.root(), dest) => Ok(()),
        _ => Err(storage_busy(
            "The project did not open in its new place, so the old folder was kept. Nothing was lost. Try the move again.",
        )),
    }
}

/// The copy is a project: its manifest and its database both carry the same identity, and the
/// database passes the full integrity check.
fn check_project_copy(root: &Path, id: &str) -> AppResult<()> {
    let wrong = |what: &str| {
        AppError::new(
            "storage.copy_mismatch",
            "The copy didn't match the original, so nothing was moved. Your project is where it was.",
        )
        .with_detail(what.to_string())
    };
    let layout = ProjectLayout::new(root);
    if pf::read_manifest(&layout)?.project_id != id {
        return Err(wrong("the project identity changed"));
    }
    let conn = openframe_persistence::open_connection(&layout.database(), false)?;
    if !openframe_persistence::integrity_problems(&conn, true)?.is_empty() {
        return Err(wrong("the database failed its integrity check"));
    }
    let in_db: String = conn.query_row("SELECT id FROM project LIMIT 1", [], |r| r.get(0))?;
    if in_db != id {
        return Err(wrong("the database names another project"));
    }
    Ok(())
}

fn set_recent_path(core: &AppCore, project_id: &str, path: &Path) -> AppResult<()> {
    core.with_app_db(|c| {
        c.execute(
            "UPDATE sys_recent_project SET path = ?1, updated_at = ?2 WHERE project_id = ?3",
            params![path.to_string_lossy(), now_ms(), project_id],
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::core::AppConfig;

    #[test]
    fn a_session_that_stayed_on_the_old_folder_is_not_a_finished_move() {
        let dir = tempfile::tempdir().unwrap();
        let core = AppCore::new(
            AppConfig::isolated(dir.path()),
            Arc::new(crate::events::NullSink),
        )
        .unwrap();
        let actor = core.local_actor();
        let name =
            confirm_open_at(&core, &dir.path().join("elsewhere")).expect_err("no project is open");
        assert_eq!(name.code_str(), "storage.busy");
        core.dispatch(
            &actor,
            "project.create",
            json!({ "title": "Salt Road", "projectType": "Feature Film" }),
        )
        .unwrap();
        let root = core.project().unwrap().layout.root().to_path_buf();
        // Opening the same project at another path is a no-op: the session is still on `root`.
        let elsewhere = dir.path().join("elsewhere").join("Salt Road.openframe");
        assert!(confirm_open_at(&core, &elsewhere).is_err());
        confirm_open_at(&core, &root).expect("the session is on the folder it was asked about");
    }
}
