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
//! Nothing here is ever fatal. A rename that fails (no permission, a file held open on Windows) keeps
//! the old path and the next start tries again. A project another process holds open (its `.lock`)
//! is not renamed, and the library root is not renamed while any project inside it is held.
//!
//! The rename comes first and the stored paths after, with the library root's own record last, so a
//! crash between them leaves a stored path that is gone and a sibling with the new name. The next
//! start adopts the sibling only with evidence that it is the same library, and a project only when
//! the id in its manifest matches. A step that fails instead of crashing puts the folder and the
//! records back, so this run works at the old path.
//!
//! The order for one folder: validate it, see that nobody holds it, and only then change anything.
//! A project that is damaged or from a newer version keeps its name, and opening it reports why.
//! Once a scan finds nothing left it is recorded, and later starts skip it: a project that turns up
//! under the earlier name after that is renamed when it is opened.
//!
//! All this runs before the first window, so it must never hold up a start: a path on a network
//! volume is left alone (the Mac asks `statfs` for the volume, `pf::is_network_path`; no file on it is
//! opened), and the scans stop at [`START_SCAN_TIME`] and try at a later start.
//!
//! Stored paths that follow: `sys_recent_project.path`, `storage.library_root`, the library pointer
//! and the folders in `library.json`. Files a project links to by absolute path (`external_path` in a
//! project database) are the person's own and do not follow; a link into the old folder shows as a
//! missing file and can be relinked.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
/// How long the scans of one start may take.
const START_SCAN_TIME: Duration = Duration::from_secs(3);
/// Set once a scan of the projects folder finds nothing left under the earlier name.
const SCAN_DONE: &str = "storage.earlier_projects_done";

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
    if !may_rename(&root, &config.app_data_dir) {
        return;
    }
    let Some(parent) = root.parent() else {
        return;
    };
    // A folder is a library only when it holds one: the old app data folder has the same name, and
    // so may a code checkout.
    if root.exists() && !is_library_root(&root) {
        return;
    }
    let target = parent.join(LIBRARY_FOLDER);
    if root.is_dir() {
        if fs::symlink_metadata(&target).is_ok() {
            tracing::warn!("the library keeps its earlier name: a folder with the new name exists");
            return;
        }
        if a_project_is_held_inside(&root, START_SCAN_TIME) {
            tracing::info!("the library keeps its earlier name while a project in it is open");
            return;
        }
        if let Err(e) = atomic::rename_with_retry(&root, &target) {
            tracing::warn!(error = %e.kind(), "the library folder could not be renamed yet");
            return;
        }
        atomic::sync_folder(parent);
        if let Err(e) = update_stored_paths(conn, config, &root, &target) {
            tracing::warn!(
                code = e.code_str(),
                "the library keeps its earlier name for now"
            );
            put_back(config, &root, &target);
        }
    } else if same_library(conn, config, &root, &target)
        && let Err(e) = update_stored_paths(conn, config, &root, &target)
    {
        tracing::error!(
            code = e.code_str(),
            "the renamed library was found but its stored paths could not all be updated; the next start finishes it"
        );
    }
}

/// A remembered root this start may rename, judged before anything in it is read: a network volume
/// can block for a long time. The same refusal as the place the library is read from.
fn may_rename(root: &Path, app_data_dir: &Path) -> bool {
    root.file_name().is_some_and(|n| {
        n.to_string_lossy()
            .eq_ignore_ascii_case(legacy_names::PRODUCT_NAME)
    }) && !pf::is_network_path(root)
        && !library::is_inside(app_data_dir, root)
        && !library::is_inside(root, app_data_dir)
}

/// A rename whose records could not follow: the folder and the two files go back, best effort, so this
/// run works at the old path and the next start tries again. The database never changed.
fn put_back(config: &AppConfig, old: &Path, new: &Path) {
    if let Err(e) = atomic::rename_with_retry(new, old) {
        tracing::error!(error = %e.kind(), "the renamed library folder could not be put back");
    } else if let Err(e) = update_files(config, new, old) {
        tracing::error!(
            code = e.code_str(),
            "the library records could not be put back"
        );
    }
}

/// A project in the library's `Projects` folder, or one level below, is open in another process.
/// More folders than the scan limit, or more time than `limit`, counts as yes: the rename waits
/// rather than guess, and a later start tries again.
fn a_project_is_held_inside(root: &Path, limit: Duration) -> bool {
    let started = Instant::now();
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
                if visited > SCAN_LIMIT || started.elapsed() > limit {
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

/// The sibling with the new name is the library that was recorded at `old`: it holds a library, and
/// the pointer already names it, a recent project under `old` is there, or its `library.json` belongs
/// to the person this database holds. Any other folder with this name is someone else's.
fn same_library(conn: &Connection, config: &AppConfig, old: &Path, target: &Path) -> bool {
    if !is_library_root(target) {
        return false;
    }
    let pointer_names_it = config
        .library_setup
        .as_ref()
        .and_then(|s| library_ops::read_pointer(&s.pointer_file))
        .is_some_and(|p| library::same_place(&p, target));
    let a_recent_project_is_there = || {
        recent_rows(conn).is_ok_and(|rows| {
            rows.iter().any(|(_, path)| {
                library::rebase(Path::new(path), old, target).is_some_and(|moved| {
                    moved.exists()
                        || (has_earlier_extension(&moved)
                            && moved.with_extension(pf::PROJECT_EXTENSION).exists())
                })
            })
        })
    };
    let same_person = || {
        let profile: Option<String> = conn
            .query_row("SELECT user_id FROM sys_local_profile LIMIT 1", [], |r| {
                r.get(0)
            })
            .optional()
            .ok()
            .flatten();
        matches!(
            library::read(&target.join(library::LIBRARY_FILE_NAME), &config.app_data_dir),
            library::Read::Valid(found) if Some(&found.user_id) == profile.as_ref()
        )
    };
    pointer_names_it || a_recent_project_is_there() || same_person()
}

/// Every stored path into the library follows it. The two files go first; the recent projects and
/// the root's own record change in one transaction, so the database is wholly old or wholly new. Until
/// it commits the next start sees the old root gone and runs this again, and each step can run twice.
fn update_stored_paths(
    conn: &Connection,
    config: &AppConfig,
    old: &Path,
    new: &Path,
) -> AppResult<()> {
    update_files(config, old, new)?;
    // Taken for writing from the start: a second start doing the same work waits here, where a read
    // that turned into a write would fail at once and send the folder back behind the other's record.
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let moved = (|| {
        for (id, path) in recent_rows(conn)? {
            if let Some(moved) = library::rebase(Path::new(&path), old, new) {
                set_recent_path(conn, &id, &moved)?;
            }
        }
        library_ops::write_db_root(conn, new)
    })();
    match moved {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    }
    Ok(())
}

/// The library file and the pointer follow the library from `old` to `new`.
fn update_files(config: &AppConfig, old: &Path, new: &Path) -> AppResult<()> {
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
    Ok(())
}

// ------------------------------------------------------------------ project folders

/// Rename the projects directly in the library's `Projects` folder, then repair the recent projects
/// a crash left pointing at a folder that has moved. Once nothing is left this is not done again.
pub(crate) fn projects_at_start(conn: &Connection, place: &LibraryPlace) {
    let done: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sys_settings WHERE key=?1)",
            [SCAN_DONE],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if done || pf::is_network_path(&place.projects_dir) {
        return;
    }
    let started = Instant::now();
    // A folder that cannot be read (a drive that is not connected) is looked at again next start.
    let Ok(entries) = fs::read_dir(&place.projects_dir) else {
        return;
    };
    let mut left = 0usize;
    for entry in entries.flatten().take(SCAN_LIMIT) {
        if started.elapsed() > START_SCAN_TIME {
            left += 1;
            break;
        }
        let path = entry.path();
        if !has_earlier_extension(&path) || !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Some(new) = rename_project(&path) else {
            left += 1;
            continue;
        };
        if let Err(e) = record_new_path(conn, &path, &new) {
            left += 1;
            tracing::warn!(
                code = e.code_str(),
                "a renamed project could not be recorded"
            );
        }
    }
    if let Err(e) = adopt_renamed_projects(conn) {
        left += 1;
        tracing::warn!(code = e.code_str(), "recent projects could not be repaired");
    }
    if left == 0
        && let Err(e) = conn.execute(
            "INSERT INTO sys_settings(key, value_json, updated_at) VALUES (?1, 'true', ?2)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![SCAN_DONE, now_ms()],
        )
    {
        tracing::warn!(error = %e, "the finished scan could not be recorded");
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

/// Rename one project folder when it is a project this version can open and no other process holds
/// it. The lock is released before the rename: Windows cannot rename a folder with a file open inside
/// it. The path holds the project's name, so it is never logged.
fn rename_project(folder: &Path) -> Option<PathBuf> {
    let layout = ProjectLayout::new(folder);
    if let Err(e) = pf::read_manifest(&layout) {
        tracing::info!(
            code = e.code_str(),
            "a project keeps its earlier extension: it cannot be read"
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn only_a_local_folder_with_the_earlier_name_may_be_renamed() {
        let dir = tempfile::tempdir().unwrap();
        let app_data = dir.path().join("appdata");
        let named = dir.path().join(legacy_names::PRODUCT_NAME);
        assert!(may_rename(&named, &app_data));
        assert!(may_rename(
            &dir.path().join(legacy_names::PRODUCT_NAME.to_lowercase()),
            &app_data
        ));
        assert!(!may_rename(&dir.path().join("Films"), &app_data));
        assert!(!may_rename(Path::new("/"), &app_data));
        assert!(!may_rename(
            &app_data.join(legacy_names::PRODUCT_NAME),
            &app_data
        ));
    }

    /// The scan is bounded by time: at the limit the rename waits for a later start.
    #[test]
    fn a_scan_out_of_time_counts_as_held() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join(PROJECTS_FOLDER).join("Plain folder");
        fs::create_dir_all(&project).unwrap();
        assert!(!a_project_is_held_inside(dir.path(), START_SCAN_TIME));
        std::thread::sleep(Duration::from_millis(5));
        assert!(a_project_is_held_inside(dir.path(), Duration::ZERO));
    }

    /// A library folder that holds the app data folder is the app's own place, whatever it is called.
    /// Nothing here holds a file open, so only the refusal keeps the folder where it is.
    #[test]
    fn a_folder_that_holds_the_app_data_is_never_renamed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(legacy_names::PRODUCT_NAME);
        fs::create_dir_all(root.join(PROJECTS_FOLDER)).unwrap();
        fs::create_dir_all(root.join(library_ops::VAULT_FOLDER)).unwrap();
        let config = AppConfig::isolated_needing_setup(&root);
        assert!(library::is_inside(&root, &config.app_data_dir));
        assert!(!may_rename(&root, &config.app_data_dir));
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE sys_settings(key TEXT PRIMARY KEY, value_json TEXT, updated_at INTEGER);",
        )
        .unwrap();
        library_ops::write_db_root(&conn, &root).unwrap();
        library_at_start(&conn, &config);
        assert!(root.is_dir());
        assert!(!dir.path().join(LIBRARY_FOLDER).exists());
    }
}
