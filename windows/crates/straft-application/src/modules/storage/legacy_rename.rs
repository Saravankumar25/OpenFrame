//! Folders a person already has under the product's earlier name take the new name. This runs after
//! the app data folder came across (`legacy_migration`), on the person's own data only:
//! `AppConfig::rename_earlier_names` is off on a test data root, under cargo and in store builds, and
//! a read-only or package path never comes through here.
//!
//! - The library root (`<anywhere>/<earlier name>`, by name only) becomes a sibling `Straft Studio` at
//!   start, and everything that stores a path follows. If that sibling exists, it is left alone.
//! - Project folders (`<title>.<earlier extension>`) become `<title>.straft`: the direct children of
//!   the library's `Projects` folder at start, and any other one when it is opened. A taken name gets
//!   ` (2)` as for a new project.
//!
//! Nothing here is ever fatal. A rename that fails (a file held open on Windows, no permission) keeps
//! the old path and the next start tries again. A project another process holds open (its `.lock`)
//! is not renamed, and the library root is not renamed while any project inside it is held.
//!
//! The rename comes first and the stored paths after, with the library root's own record last, so a
//! crash between them leaves a stored path that is gone and a sibling with the new name. The next
//! start adopts the sibling only when its identity matches: `library.json` for the library, the
//! project id in the manifest for a project.
//!
//! Stored paths that follow: `sys_recent_project.path`, `storage.library_root`, the library pointer
//! and the folders in `library.json`. Files a project links to by absolute path (`external_path` in a
//! project database) are the person's own and do not follow; a link into the old folder shows as a
//! missing file and can be relinked.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use straft_domain::{AppResult, now_ms};
use straft_platform::{atomic, legacy_names};
use straft_project_format::{self as pf, ProjectLayout, ProjectLock};

use super::library;
use super::library_ops::{
    self, LIBRARY_FOLDER, LibraryPlace, PROJECTS_FOLDER, SCAN_LIMIT, is_library_root,
};
use crate::core::{AppConfig, AppCore};

/// The ` (n)` names looked at when a project's new folder is searched for after a crash.
const CLASH_LIMIT: u32 = 50;

fn has_earlier_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(legacy_names::PROJECT_EXTENSION))
}

// ------------------------------------------------------------------ the library root

/// Rename the remembered library root when it has the earlier name, or adopt the sibling a crash
/// left. Runs before the library is read, so everything after it sees the new place.
pub(crate) fn library_at_start(conn: &Connection, config: &AppConfig) {
    let Some(setup) = &config.library_setup else {
        return;
    };
    let remembered =
        library_ops::read_db_root(conn).or_else(|| library_ops::read_pointer(&setup.pointer_file));
    let Some(root) = remembered else {
        return;
    };
    if root.file_name() != Some(std::ffi::OsStr::new(legacy_names::PRODUCT_NAME)) {
        return;
    }
    let Some(parent) = root.parent() else {
        return;
    };
    // The same refusal as the place the library is read from, and a folder is a library only when
    // it holds one: the old app data folder has the same name.
    if library::is_inside(&config.app_data_dir, &root) || (root.exists() && !is_library_root(&root))
    {
        return;
    }
    let target = parent.join(LIBRARY_FOLDER);
    if root.is_dir() {
        if fs::symlink_metadata(&target).is_ok() {
            tracing::warn!("the library keeps its earlier name: a folder with the new name exists");
            return;
        }
        if a_project_is_held_inside(&root) {
            tracing::info!("the library keeps its earlier name while a project in it is open");
            return;
        }
        if let Err(e) = atomic::rename_with_retry(&root, &target) {
            tracing::warn!(error = %e.kind(), "the library folder could not be renamed yet");
            return;
        }
        atomic::sync_folder(parent);
    } else if !same_library(conn, &target, &config.app_data_dir) {
        return;
    }
    if let Err(e) = update_stored_paths(conn, config, &root, &target) {
        tracing::error!(
            code = e.code_str(),
            "the library was renamed but its stored paths could not all be updated; the next start finishes it"
        );
    }
}

/// A project in the library's `Projects` folder, or one level below, is open in another process.
/// More folders than the scan limit counts as yes: the rename waits rather than guess.
fn a_project_is_held_inside(root: &Path) -> bool {
    let mut level = vec![root.join(PROJECTS_FOLDER)];
    let mut visited = 0usize;
    for _ in 0..2 {
        let mut next = Vec::new();
        for parent in &level {
            let Ok(entries) = fs::read_dir(parent) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                visited += 1;
                if visited > SCAN_LIMIT {
                    return true;
                }
                let path = entry.path();
                if !pf::is_project_folder(&path) {
                    next.push(path);
                } else if ProjectLock::acquire(&ProjectLayout::new(&path))
                    .is_err_and(|e| e.is("project_format.in_use"))
                {
                    return true;
                }
            }
        }
        level = next;
    }
    false
}

/// The sibling is this person's library: a valid `library.json` with this profile's id.
fn same_library(conn: &Connection, target: &Path, app_data_dir: &Path) -> bool {
    if !is_library_root(target) {
        return false;
    }
    let library::Read::Valid(found) =
        library::read(&target.join(library::LIBRARY_FILE_NAME), app_data_dir)
    else {
        return false;
    };
    let profile: Option<String> = conn
        .query_row("SELECT user_id FROM sys_local_profile LIMIT 1", [], |r| {
            r.get(0)
        })
        .optional()
        .ok()
        .flatten();
    profile.is_none_or(|id| id == found.user_id)
}

/// Every stored path into the library follows it. The root's own record goes last: until then the
/// next start sees the old root gone and runs this again, and each step can run twice.
fn update_stored_paths(
    conn: &Connection,
    config: &AppConfig,
    old: &Path,
    new: &Path,
) -> AppResult<()> {
    for (id, path) in recent_rows(conn)? {
        if let Some(moved) = library::rebase(Path::new(&path), old, new) {
            set_recent_path(conn, &id, &moved)?;
        }
    }
    let file = new.join(library::LIBRARY_FILE_NAME);
    if let library::Read::Valid(found) = library::read(&file, &config.app_data_dir) {
        let follow = |p: &Option<PathBuf>| {
            p.as_deref()
                .map(|p| library::rebase(p, old, new).unwrap_or_else(|| p.to_path_buf()))
        };
        let (projects, vault) = (follow(&found.projects_dir), follow(&found.vault_dir));
        if projects != found.projects_dir || vault != found.vault_dir {
            library::write(
                &file,
                &found.user_id,
                &found.display_name,
                projects.as_deref(),
                vault.as_deref(),
            )?;
        }
    }
    if let Some(setup) = &config.library_setup {
        library_ops::write_pointer(&setup.pointer_file, new)?;
    }
    library_ops::write_db_root(conn, new)
}

// ------------------------------------------------------------------ project folders

/// Rename the projects directly in the library's `Projects` folder, then repair the recent projects
/// a crash left pointing at a folder that has moved.
pub(crate) fn projects_at_start(conn: &Connection, place: &LibraryPlace) {
    if let Ok(entries) = fs::read_dir(&place.projects_dir) {
        for entry in entries.flatten().take(SCAN_LIMIT) {
            let path = entry.path();
            if !has_earlier_extension(&path) || !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            if let Some(new) = rename_project(&path)
                && let Err(e) = record_new_path(conn, &path, &new)
            {
                tracing::warn!(
                    code = e.code_str(),
                    "a renamed project could not be recorded"
                );
            }
        }
    }
    if let Err(e) = adopt_renamed_projects(conn) {
        tracing::warn!(code = e.code_str(), "recent projects could not be repaired");
    }
}

/// The folder to open: renamed first when it has the earlier extension. Anything that stops the
/// rename leaves the path as it is, and the project opens there.
pub(crate) fn project_on_open(core: &AppCore, path: &Path) -> PathBuf {
    if !core.config.rename_earlier_names
        || !has_earlier_extension(path)
        || pf::is_network_path(path)
        || straft_security::check_user_path(path, straft_security::NetworkPaths::Refuse).is_err()
    {
        return path.to_path_buf();
    }
    let Some(new) = rename_project(path) else {
        return path.to_path_buf();
    };
    // Opening records the new path by project id too, so a failure here costs nothing.
    if let Err(e) = core.with_app_db(|c| record_new_path(c, path, &new)) {
        tracing::warn!(
            code = e.code_str(),
            "a renamed project could not be recorded"
        );
    }
    new
}

/// Rename one project folder when no other process holds it. The lock is released before the rename:
/// Windows cannot rename a folder with a file open inside it. The path holds the project's name, so
/// it is never logged.
fn rename_project(folder: &Path) -> Option<PathBuf> {
    let layout = ProjectLayout::new(folder);
    if !layout.has_manifest() {
        return None;
    }
    if let Err(e) = ProjectLock::acquire(&layout) {
        tracing::info!(
            code = e.code_str(),
            "a project keeps its earlier extension for now"
        );
        return None;
    }
    let parent = folder.parent()?;
    let title = folder.file_stem()?.to_string_lossy();
    let new = pf::unique_project_dir(parent, &title);
    if let Err(e) = atomic::rename_with_retry(folder, &new) {
        tracing::warn!(error = %e.kind(), "a project folder could not be renamed yet");
        return None;
    }
    atomic::sync_folder(parent);
    Some(new)
}

fn record_new_path(conn: &Connection, old: &Path, new: &Path) -> AppResult<()> {
    conn.execute(
        "UPDATE sys_recent_project SET path=?1, updated_at=?2 WHERE path=?3",
        params![
            new.to_string_lossy(),
            now_ms(),
            old.to_string_lossy().as_ref()
        ],
    )?;
    Ok(())
}

/// A recent project whose folder with the earlier extension is gone, and whose renamed folder is right
/// beside it with the same project id, is that folder.
fn adopt_renamed_projects(conn: &Connection) -> AppResult<()> {
    // The default theme's stored id takes the new name as well, so no row keeps the earlier one.
    conn.execute(
        "UPDATE sys_appearance_pref SET value='straft-default', updated_at=?1 WHERE value=?2",
        params![now_ms(), legacy_names::DEFAULT_THEME_ID],
    )?;
    for (id, path) in recent_rows(conn)? {
        let path = PathBuf::from(path);
        if has_earlier_extension(&path)
            && !path.exists()
            && let Some(found) = renamed_sibling(&path, &id)
        {
            set_recent_path(conn, &id, &found)?;
        }
    }
    Ok(())
}

fn renamed_sibling(path: &Path, project_id: &str) -> Option<PathBuf> {
    let parent = path.parent()?;
    let title = straft_security::sanitize_file_name(&path.file_stem()?.to_string_lossy());
    let names = std::iter::once(format!("{title}.{}", pf::PROJECT_EXTENSION))
        .chain((2..=CLASH_LIMIT).map(|n| format!("{title} ({n}).{}", pf::PROJECT_EXTENSION)));
    names.map(|name| parent.join(name)).find(|candidate| {
        pf::read_manifest(&ProjectLayout::new(candidate)).is_ok_and(|m| m.project_id == project_id)
    })
}

fn recent_rows(conn: &Connection) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT project_id, path FROM sys_recent_project LIMIT ?1")?;
    let rows = stmt
        .query_map([SCAN_LIMIT as i64], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn set_recent_path(conn: &Connection, project_id: &str, path: &Path) -> AppResult<()> {
    conn.execute(
        "UPDATE sys_recent_project SET path=?1, updated_at=?2 WHERE project_id=?3",
        params![path.to_string_lossy(), now_ms(), project_id],
    )?;
    Ok(())
}
