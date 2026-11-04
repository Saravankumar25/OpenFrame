//! The first start after the product was renamed: the app data folder and the library pointer folder
//! of the earlier version are brought over before `app.sqlite` is opened, so a person loses nothing.
//!
//! The rule (owner's): copy the old folder into the new one, once, and leave the old folder in place.
//!
//! 1. Everything except what is listed below is copied into a sibling `<new>.migrating-<pid>-<ms>`,
//!    every file synced and read back against its hash, the copied `app.sqlite` checked.
//! 2. The sibling is renamed to the final name in one step. Until then the new folder holds no copied
//!    data, so an interrupted copy leaves exactly what was there before and the next start begins
//!    again. A new folder counts as in use only when it holds the real data (`app.sqlite`, or the
//!    pointer file), never just because it exists: an earlier failed start may have left its log
//!    folder there. What such a folder holds is taken along into the copy.
//! 3. Only then are the heavy folders (`MOVED_NOT_COPIED`, the downloaded AI models) renamed across.
//!    They are gigabytes and can be downloaded again, so they are never copied. A crash before this
//!    step is finished by the next start; a rename that cannot work (another volume) leaves them in
//!    the old folder and the person downloads the models again, which costs time and loses nothing.
//!
//! When the copy fails (full drive, unreadable file, no permission) the temporary folder is removed
//! and this run **uses the old folder as it is**. That cannot lose anything: the person sees all their
//! settings and recent projects, nothing has been written to a half-finished place, and the next
//! start tries the copy again with whatever changed meanwhile. A fresh start would be worse: it would
//! show an empty app, write a new `app.sqlite` and so stop every later attempt. The reason is logged
//! once logging is up ([`log_notes`]).
//!
//! Nothing inside these folders names the old folder: `app.sqlite` stores the library folder (the
//! person's own, which does not move), project folders and a cover's file name only, `engines.json`
//! and `ai/bundle-check.json` name the installed engine, and the model set is named after the model
//! hashes. So no path is rewritten, and a library pointer to `Documents/<earlier name>` stays right.
//!
//! This never runs on a test data root: `AppConfig::for_current_user` is the only caller, the shells
//! do not call that with `STRAFT_DATA_ROOT` set, and [`allowed`] refuses as well.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use straft_domain::{AppError, AppResult, now_ms};
use straft_platform::{atomic, dirs, legacy_names};

use super::tree;

/// Marker in the name of the temporary sibling.
const TEMP_MARKER: &str = ".migrating-";

/// Top-level names of the app data folder that are renamed across, never copied.
const MOVED_NOT_COPIED: &[&str] = &["models"];
/// Top-level names copied when they can be: losing a log file never holds the person back.
const BEST_EFFORT: &[&str] = &["logs"];
/// Top-level names left behind: the import staging folder, rebuilt on demand.
const LEFT_BEHIND: &[&str] = &["tmp"];

/// What to do about one pair of folders. Separate from the real locations for the tests.
#[derive(Debug, Clone, Copy)]
pub struct Plan<'a> {
    pub moved_not_copied: &'a [&'a str],
    pub best_effort: &'a [&'a str],
    pub left_behind: &'a [&'a str],
    /// The file that shows a folder holds real data. Without it the new folder is not in use.
    pub in_use_marker: &'a str,
    /// A temporary sibling younger than this may belong to another start that is still copying.
    pub stale_after: Duration,
}

impl Plan<'_> {
    /// The app data folder.
    pub const APP_DATA: Plan<'static> = Plan {
        moved_not_copied: MOVED_NOT_COPIED,
        best_effort: BEST_EFFORT,
        left_behind: LEFT_BEHIND,
        in_use_marker: "app.sqlite",
        stale_after: atomic::STALE_AFTER,
    };
    /// The library pointer folder: one small file.
    pub const POINTER: Plan<'static> = Plan {
        moved_not_copied: &[],
        best_effort: &[],
        left_behind: &[],
        in_use_marker: super::library_ops::POINTER_FILE,
        stale_after: atomic::STALE_AFTER,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// There is no earlier folder.
    NoEarlierFolder,
    /// The new folder already holds real data; it is not touched and nothing is merged into it.
    AlreadyThere,
    /// The earlier folder was copied to the new name.
    Copied,
    /// The copy failed. The earlier folder is as it was; use it for this run and try again at the
    /// next start.
    UseEarlierFolder { reason: String },
}

/// Bring `old` over to `new`.
pub fn bring_over(old: &Path, new: &Path, plan: &Plan<'_>) -> Outcome {
    if !old.is_dir() {
        return Outcome::NoEarlierFolder;
    }
    remove_stale_temporaries(new, plan.stale_after);
    let outcome = if in_use(new, plan) {
        Outcome::AlreadyThere
    } else {
        match copy_to_new_name(old, new, plan) {
            Ok(outcome) => outcome,
            Err(e) => Outcome::UseEarlierFolder {
                reason: reason_of(&e),
            },
        }
    };
    if !matches!(outcome, Outcome::UseEarlierFolder { .. }) {
        for name in plan.moved_not_copied {
            move_heavy_folder(old, new, name);
        }
    }
    outcome
}

/// Logs carry the code and the detail of the error; neither holds a project name or user data
/// (`io::Error` carries no path; `AppError` details here name files of the app data folder).
fn reason_of(e: &AppError) -> String {
    match &e.detail {
        Some(d) => format!("{}: {d}", e.code_str()),
        None => e.code_str().to_string(),
    }
}

/// True when `new` holds real data. A link or a file at that name is not ours to replace either.
fn in_use(new: &Path, plan: &Plan<'_>) -> bool {
    fs::symlink_metadata(new).is_ok_and(|m| !m.is_dir()) || new.join(plan.in_use_marker).exists()
}

/// Move what a new folder without real data holds (the log folder of a failed start, say) into the
/// copy, and remove it. The earlier version's file wins where both have one.
fn take_along(from: &Path, to: &Path) -> AppResult<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        let is_dir = entry.file_type()?.is_dir();
        match fs::symlink_metadata(&dest) {
            Err(_) => fs::rename(entry.path(), &dest)?,
            Ok(m) if m.is_dir() && is_dir => take_along(&entry.path(), &dest)?,
            Ok(_) if is_dir => fs::remove_dir_all(entry.path())?,
            Ok(_) => fs::remove_file(entry.path())?,
        }
    }
    fs::remove_dir(from)?;
    Ok(())
}

/// `<new>.migrating-<pid>-<ms>` beside `new`.
fn temporary_prefix(new: &Path) -> Option<String> {
    let name = new.file_name()?.to_string_lossy().into_owned();
    Some(format!("{name}{TEMP_MARKER}"))
}

/// Remove what an interrupted copy left beside `new`. A young one may be another start copying right
/// now (two starts in the same moment): it is left for a later start. Links are never followed. The
/// heavy folders are moved only after the final rename, so a temporary folder never holds them.
fn remove_stale_temporaries(new: &Path, stale_after: Duration) {
    let (Some(parent), Some(prefix)) = (new.parent(), temporary_prefix(new)) else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(&prefix)
            || !entry.file_type().is_ok_and(|t| t.is_dir())
        {
            continue;
        }
        let old_enough = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age >= stale_after);
        if old_enough && fs::remove_dir_all(entry.path()).is_err() {
            note(
                Level::Warn,
                "a temporary folder of an interrupted copy could not be removed yet".into(),
            );
        }
    }
}

fn copy_to_new_name(old: &Path, new: &Path, plan: &Plan<'_>) -> AppResult<Outcome> {
    let parent = new
        .parent()
        .ok_or_else(|| AppError::internal("the new folder has no parent"))?;
    fs::create_dir_all(parent)?;
    let prefix = temporary_prefix(new)
        .ok_or_else(|| AppError::internal("the new folder has no name"))?;
    let temp = parent.join(format!("{prefix}{}-{}", std::process::id(), now_ms()));
    fs::create_dir(&temp)?;
    let copied = fill_temporary(old, &temp, plan).and_then(|()| {
        // Another start of the app may have finished first: then use its copy.
        if in_use(new, plan) {
            return Ok(Outcome::AlreadyThere);
        }
        if new.exists() {
            take_along(new, &temp)?;
        }
        atomic::sync_folder(&temp);
        match atomic::rename_with_retry(&temp, new) {
            Ok(()) => {
                atomic::sync_folder(parent);
                Ok(Outcome::Copied)
            }
            Err(_) if in_use(new, plan) => Ok(Outcome::AlreadyThere),
            Err(e) => Err(e.into()),
        }
    });
    // A failed or lost rename leaves the temporary folder; success has renamed it away.
    if temp.exists() {
        let _ = fs::remove_dir_all(&temp);
    }
    copied
}

/// A SQLite side file rebuilt on the first open, or a staging file a crash left behind.
fn not_needed_file(rel: &str) -> bool {
    rel.ends_with(".sqlite-shm")
        || rel.ends_with(atomic::TEMP_SUFFIX)
        || rel.ends_with(legacy_names::TEMP_SUFFIX)
}

fn fill_temporary(old: &Path, temp: &Path, plan: &Plan<'_>) -> AppResult<()> {
    let skip: Vec<&str> = plan
        .moved_not_copied
        .iter()
        .chain(plan.best_effort)
        .chain(plan.left_behind)
        .copied()
        .collect();
    let mut source = tree::walk(old, &skip)?;
    source.files.retain(|f| !not_needed_file(&f.rel));
    let quiet = |_: f64, _: &str| {};
    let never = || false;
    let copied = tree::copy(&source, temp, &quiet, &never)?;
    tree::verify(temp, &copied, &quiet, &never)?;
    for dir in &source.dirs {
        atomic::sync_folder(&temp.join(dir));
    }
    for name in plan.best_effort {
        copy_best_effort(&old.join(name), &temp.join(name));
    }
    check_copied_database(temp)?;
    Ok(())
}

/// A copy of a database that another program is writing can be torn. Find out now, while the earlier
/// folder is still the one in use. The connection is closed again before the rename.
fn check_copied_database(temp: &Path) -> AppResult<()> {
    let db = temp.join("app.sqlite");
    if !db.is_file() {
        return Ok(());
    }
    if straft_persistence::copied_database_is_sound(&db)? {
        Ok(())
    } else {
        Err(AppError::storage("the copied app database failed its check"))
    }
}

/// Copy the plain files of `from`. Whatever cannot be copied stays behind in the earlier folder.
fn copy_best_effort(from: &Path, to: &Path) {
    let Ok(entries) = fs::read_dir(from) else {
        return;
    };
    if fs::create_dir_all(to).is_err() {
        return;
    }
    let mut missed = 0u32;
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_file())
            && fs::copy(entry.path(), to.join(entry.file_name())).is_err()
        {
            missed += 1;
        }
    }
    if missed > 0 {
        note(
            Level::Warn,
            format!("{missed} log files of the earlier version were not copied"),
        );
    }
}

/// Rename `old/name` to `new/name` when the new folder has no such folder yet (or an empty one). A
/// failure leaves everything where it is and is tried again at the next start.
fn move_heavy_folder(old: &Path, new: &Path, name: &str) {
    let (from, to) = (old.join(name), new.join(name));
    if fs::symlink_metadata(&from).is_err() {
        return;
    }
    if fs::symlink_metadata(&to).is_ok() {
        // `remove_dir` only succeeds on an empty folder: a model set is never deleted here.
        if fs::remove_dir(&to).is_err() {
            return;
        }
    }
    match atomic::rename_with_retry(&from, &to) {
        Ok(()) => {
            atomic::sync_folder(new);
            atomic::sync_folder(old);
        }
        Err(e) => note(
            Level::Warn,
            format!(
                "the downloaded models were left in the earlier folder and can be downloaded again: {}",
                e.kind()
            ),
        ),
    }
}

// ------------------------------------------------------------ the person's own folders

/// False when this run must not touch the person's real folders: a test data root is set, or the
/// process was started by cargo (`cargo test` reaches `AppConfig::for_current_user` from the FFI
/// crate's own tests, and a developer's real data sits in these folders).
pub fn allowed(data_root: Option<&OsStr>, started_by_cargo: Option<&OsStr>) -> bool {
    data_root.is_none() && started_by_cargo.is_none()
}

/// The folders to use in this run: `(app data, library pointer)`.
pub(crate) fn folders_for_current_user() -> (Option<PathBuf>, Option<PathBuf>) {
    let app_data = dirs::app_data_dir();
    let pointer = dirs::roaming_data_dir();
    if !allowed(
        std::env::var_os("STRAFT_DATA_ROOT").as_deref(),
        std::env::var_os("CARGO_MANIFEST_DIR").as_deref(),
    ) {
        return (app_data, pointer);
    }
    (
        use_folder(app_data, dirs::legacy_app_data_dir(), &Plan::APP_DATA),
        use_folder(pointer, dirs::legacy_roaming_data_dir(), &Plan::POINTER),
    )
}

fn use_folder(new: Option<PathBuf>, old: Option<PathBuf>, plan: &Plan<'_>) -> Option<PathBuf> {
    let (Some(new), Some(old)) = (new.clone(), old) else {
        return new;
    };
    match bring_over(&old, &new, plan) {
        Outcome::NoEarlierFolder | Outcome::AlreadyThere => Some(new),
        Outcome::Copied => {
            note(Level::Info, "brought an earlier folder over".into());
            Some(new)
        }
        Outcome::UseEarlierFolder { reason } => {
            note(
                Level::Error,
                format!("an earlier folder could not be copied; it stays in use and the copy is tried again at the next start: {reason}"),
            );
            Some(old)
        }
    }
}

// ------------------------------------------------------------ notes for the log

#[derive(Clone, Copy)]
enum Level {
    Info,
    Warn,
    Error,
}

/// This runs before the shell starts logging (the shell's log folder depends on the answer), so what
/// happened is kept here until [`log_notes`].
static NOTES: Mutex<Vec<(Level, String)>> = Mutex::new(Vec::new());

fn note(level: Level, text: String) {
    NOTES.lock().push((level, text));
}

/// Write what happened to the log. `AppCore::new` calls this, when logging is up.
pub(crate) fn log_notes() {
    for (level, text) in NOTES.lock().drain(..) {
        match level {
            Level::Info => tracing::info!("{text}"),
            Level::Warn => tracing::warn!("{text}"),
            Level::Error => tracing::error!("{text}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: Plan<'static> = Plan {
        moved_not_copied: MOVED_NOT_COPIED,
        best_effort: BEST_EFFORT,
        left_behind: LEFT_BEHIND,
        in_use_marker: "settings.txt",
        stale_after: Duration::ZERO,
    };

    /// `old` and `new` side by side under a temp folder, `old` holding a realistic app data folder.
    fn folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("Old Name");
        let new = dir.path().join("New Name");
        for sub in ["entitlement", "ai", "covers/assets", "logs", "tmp/x", "models/m-abc"] {
            fs::create_dir_all(old.join(sub)).unwrap();
        }
        for (rel, text) in [
            ("settings.txt", "settings"),
            ("entitlement/notices.json", "{}"),
            ("ai/graphics.json", "{}"),
            ("covers/assets/one.png", "picture"),
            ("logs/earlier.2026-10-05.log", "log line"),
            ("tmp/x/half.bin", "rebuilt"),
            ("models/active.json", "{\"version\":\"m-abc\"}"),
            ("models/m-abc/chat.gguf", "gigabytes"),
        ] {
            fs::write(old.join(rel), text).unwrap();
        }
        (dir, old, new)
    }

    fn listing(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(d) = pending.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                if e.file_type().unwrap().is_dir() {
                    pending.push(e.path());
                } else {
                    let rel = e.path().strip_prefix(dir).unwrap().to_path_buf();
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        out.sort();
        out
    }

    fn temporaries(parent: &Path) -> Vec<String> {
        fs::read_dir(parent)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains(TEMP_MARKER))
            .collect()
    }

    #[test]
    fn the_folder_is_copied_the_models_are_moved_and_the_old_folder_stays() {
        let (dir, old, new) = folders();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert_eq!(
            listing(&new),
            [
                "ai/graphics.json",
                "covers/assets/one.png",
                "entitlement/notices.json",
                "logs/earlier.2026-10-05.log",
                "models/active.json",
                "models/m-abc/chat.gguf",
                "settings.txt",
            ]
        );
        assert_eq!(fs::read(new.join("settings.txt")).unwrap(), b"settings");
        // The old folder keeps everything except the models.
        assert!(old.join("settings.txt").is_file());
        assert!(old.join("tmp/x/half.bin").is_file());
        assert!(!old.join("models").exists(), "moved, not copied");
        assert!(temporaries(dir.path()).is_empty());
    }

    #[test]
    fn a_second_start_changes_nothing() {
        let (_dir, old, new) = folders();
        bring_over(&old, &new, &NOW);
        fs::write(new.join("settings.txt"), "changed in the new folder").unwrap();
        fs::write(old.join("settings.txt"), "the old folder is not read again").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert_eq!(
            fs::read_to_string(new.join("settings.txt")).unwrap(),
            "changed in the new folder"
        );
    }

    #[test]
    fn no_earlier_folder_means_nothing_happens() {
        let dir = tempfile::tempdir().unwrap();
        let new = dir.path().join("New Name");
        assert_eq!(
            bring_over(&dir.path().join("absent"), &new, &NOW),
            Outcome::NoEarlierFolder
        );
        assert!(!new.exists());
    }

    /// The destination exists: nothing is merged into it and nothing in it is overwritten.
    #[test]
    fn a_new_folder_in_use_is_left_alone() {
        let (_dir, old, new) = folders();
        fs::create_dir_all(new.join("models/m-new")).unwrap();
        fs::write(new.join("models/m-new/mine.gguf"), "already downloaded").unwrap();
        fs::write(new.join("settings.txt"), "new").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert_eq!(fs::read_to_string(new.join("settings.txt")).unwrap(), "new");
        assert!(!new.join("ai").exists());
        assert!(new.join("models/m-new/mine.gguf").is_file());
        assert!(old.join("models/m-abc/chat.gguf").is_file(), "not moved over a set");
    }

    #[test]
    fn an_empty_new_folder_counts_as_free() {
        let (_dir, old, new) = folders();
        fs::create_dir_all(&new).unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(new.join("settings.txt").is_file());
    }

    /// A folder that merely exists is not migrated data: a failed start's log folder, say.
    #[test]
    fn a_new_folder_without_real_data_does_not_stop_the_copy() {
        let (dir, old, new) = folders();
        fs::create_dir_all(new.join("logs")).unwrap();
        fs::write(new.join("logs/failed-start.log"), "kept").unwrap();
        fs::write(new.join("logs/earlier.2026-10-05.log"), "the old file wins").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert_eq!(fs::read(new.join("settings.txt")).unwrap(), b"settings");
        assert_eq!(fs::read(new.join("logs/failed-start.log")).unwrap(), b"kept");
        assert_eq!(
            fs::read(new.join("logs/earlier.2026-10-05.log")).unwrap(),
            b"log line"
        );
        assert!(new.join("models/active.json").is_file());
        assert!(temporaries(dir.path()).is_empty());
    }

    /// A crash after the final rename and before the models moved: the next start finishes it.
    #[test]
    fn models_left_behind_by_a_crash_are_moved_at_the_next_start() {
        let (_dir, old, new) = folders();
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join("settings.txt"), "copied").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert!(new.join("models/m-abc/chat.gguf").is_file());
        assert!(!old.join("models").exists());
        // An empty models folder (the app made it before the move) is no obstacle either.
        let (_dir, old, new) = folders();
        fs::create_dir_all(new.join("models")).unwrap();
        fs::write(new.join("settings.txt"), "copied").unwrap();
        bring_over(&old, &new, &NOW);
        assert!(new.join("models/active.json").is_file());
    }

    /// A crash during the copy leaves a half-filled sibling and no new folder. The next start
    /// removes it and copies again from the old folder, which was never touched.
    #[test]
    fn an_interrupted_copy_is_removed_and_started_again() {
        let (dir, old, new) = folders();
        let left = dir.path().join(format!("New Name{TEMP_MARKER}4242-1"));
        fs::create_dir_all(left.join("ai")).unwrap();
        fs::write(left.join("settings.txt"), "half").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert_eq!(fs::read(new.join("settings.txt")).unwrap(), b"settings");
        assert!(temporaries(dir.path()).is_empty());
    }

    /// Two starts at once: a young sibling may be the other one copying. It is not taken away from
    /// under it; this start copies on its own and whichever renames first wins.
    #[test]
    fn a_young_temporary_folder_is_left_for_a_later_start() {
        let (dir, old, new) = folders();
        let left = dir.path().join(format!("New Name{TEMP_MARKER}4242-1"));
        fs::create_dir_all(&left).unwrap();
        assert_eq!(bring_over(&old, &new, &Plan::APP_DATA), Outcome::Copied);
        assert!(left.exists());
        // A later start, long after: gone, even though the new folder is in use by now.
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert!(!left.exists());
    }

    /// A folder that merely starts like our name is not ours to remove.
    #[test]
    fn other_folders_beside_the_new_one_are_never_touched() {
        let (dir, old, new) = folders();
        for name in ["New Name.backup", "New Name2", "Other.migrating-1"] {
            fs::create_dir_all(dir.path().join(name)).unwrap();
        }
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        for name in ["New Name.backup", "New Name2", "Other.migrating-1"] {
            assert!(dir.path().join(name).exists(), "{name}");
        }
    }

    #[test]
    fn side_files_and_staging_files_are_not_copied_but_the_write_ahead_log_is() {
        let (_dir, old, new) = folders();
        fs::write(old.join("app.sqlite-shm"), "rebuilt").unwrap();
        fs::write(old.join("app.sqlite-wal"), "unfolded changes").unwrap();
        fs::write(old.join(format!(".x.0123456789abcdef{}", atomic::TEMP_SUFFIX)), "x").unwrap();
        fs::write(old.join(format!(".x.0123456789abcdef{}", legacy_names::TEMP_SUFFIX)), "x").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(new.join("app.sqlite-wal").is_file());
        assert!(!new.join("app.sqlite-shm").exists());
        assert!(listing(&new).iter().all(|n| !n.contains("0123456789abcdef")));
    }

    #[test]
    fn links_in_the_old_folder_are_not_followed() {
        let (dir, old, new) = folders();
        let outside = dir.path().join("private");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "not ours").unwrap();
        if straft_test_support_symlink(&outside, &old.join("link")).is_err() {
            return; // A Windows account without the right to make links.
        }
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(!new.join("link").exists());
    }

    #[cfg(unix)]
    fn straft_test_support_symlink(to: &Path, at: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(to, at)
    }

    #[cfg(windows)]
    fn straft_test_support_symlink(to: &Path, at: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(to, at)
    }

    /// A file that cannot be read: no new folder, no temporary folder, the old folder whole, the
    /// models unmoved. The run uses the old folder and the next start tries again.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_file_leaves_everything_as_it_was() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, old, new) = folders();
        let locked = old.join("entitlement/notices.json");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&locked).is_ok() {
            return; // Running as root: the permission does not apply.
        }
        let before = listing(&old);
        let outcome = bring_over(&old, &new, &NOW);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(outcome, Outcome::UseEarlierFolder { .. }), "{outcome:?}");
        assert!(!new.exists());
        assert!(temporaries(dir.path()).is_empty());
        assert_eq!(listing(&old), before, "models stay too");
        // The next start, with the file readable again, succeeds.
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(new.join("entitlement/notices.json").is_file());
    }

    /// A log file that cannot be read never holds the person back.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_log_file_is_skipped() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, old, new) = folders();
        let locked = old.join("logs/locked.log");
        fs::write(&locked, "x").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&locked).is_ok() {
            return;
        }
        let outcome = bring_over(&old, &new, &NOW);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(outcome, Outcome::Copied);
        assert!(new.join("logs/earlier.2026-10-05.log").is_file());
        assert!(!new.join("logs/locked.log").exists());
    }

    /// No permission to write beside the old folder.
    #[cfg(unix)]
    #[test]
    fn no_permission_to_create_the_new_folder_keeps_the_old_one_in_use() {
        use std::os::unix::fs::PermissionsExt;
        let (dir, old, new) = folders();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();
        if fs::write(dir.path().join("probe"), "x").is_ok() {
            fs::remove_file(dir.path().join("probe")).unwrap();
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
            return; // Running as root.
        }
        let outcome = bring_over(&old, &new, &NOW);
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(outcome, Outcome::UseEarlierFolder { .. }), "{outcome:?}");
        assert!(!new.exists());
        assert!(old.join("models/m-abc/chat.gguf").is_file());
    }

    /// A torn copy of the database (the earlier version still running, a bad sector) is found before
    /// the rename, while the old folder is still the one in use.
    #[test]
    fn a_damaged_database_stops_the_copy() {
        let (dir, old, new) = folders();
        fs::write(old.join("app.sqlite"), vec![0x5a; 8192]).unwrap();
        let outcome = bring_over(&old, &new, &NOW);
        assert!(matches!(outcome, Outcome::UseEarlierFolder { .. }), "{outcome:?}");
        assert!(!new.exists());
        assert!(temporaries(dir.path()).is_empty());
        assert!(old.join("models/active.json").is_file());
    }

    #[test]
    fn test_data_roots_and_cargo_runs_never_touch_the_real_folders() {
        let set = OsStr::new("/tmp/x");
        assert!(allowed(None, None));
        assert!(!allowed(Some(set), None));
        assert!(!allowed(None, Some(set)));
        assert!(!allowed(Some(set), Some(set)));
    }

    #[test]
    fn this_test_process_is_refused_by_the_real_entry_point() {
        // `cargo test` sets the variable, so this run can never reach the real folders.
        assert!(std::env::var_os("CARGO_MANIFEST_DIR").is_some());
        let (app_data, pointer) = folders_for_current_user();
        assert_eq!(app_data, dirs::app_data_dir());
        assert_eq!(pointer, dirs::roaming_data_dir());
    }
}
