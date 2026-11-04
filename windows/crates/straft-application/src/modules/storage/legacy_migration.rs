//! The first start after the product was renamed: the app data folder and the library pointer folder
//! of the earlier version are brought over before `app.sqlite` is opened, so a person loses nothing.
//!
//! The rule (owner's): copy the old folder into the new one, once, and leave the old folder in place.
//!
//! 1. Everything except what is listed below is copied into a sibling `<new>.migrating-<pid>-<ms>`,
//!    every file synced and read back against its hash. The earlier version may still be running, so
//!    `app.sqlite` is taken as one consistent snapshot through a read-only connection, never as raw
//!    files, and its side files are not copied. `engines.json` is not copied either: it would let
//!    this start stop an engine the earlier version is still running. The heavy folders
//!    (`SHARED_NOT_COPIED`, the downloaded AI models) are hard linked into the sibling: gigabytes
//!    would mean a long wait at start and twice the disk use. A link holds the same bytes, so there is
//!    nothing to read back; the AI checks every model file against the sizes and hashes compiled into
//!    the app before it uses it, and a set the app replaces is replaced by rename, never in place.
//! 2. The sibling is renamed to the final name in one step. Until then the new folder holds no copied
//!    data, so an interrupted copy leaves exactly what was there before and the next start begins
//!    again. A new folder counts as in use only when it holds the real data (`app.sqlite`, or the
//!    pointer file), never just because it exists: an earlier failed start may have left its log
//!    folder there. Such a folder is renamed aside first and put back if the final rename fails;
//!    only after that rename worked is what it holds taken into the new folder.
//! 3. A hard link that cannot work (another volume, a drive without them) leaves the models in the
//!    old folder only, and the person downloads them again, which costs time and loses nothing.
//!    Downloads in progress are not linked: they are appended to in place. The links arrive with the
//!    rename or not at all, so there is no later step: a person who removes the AI from the new folder
//!    never gets it back from the old one. A models folder that is itself a link (people move big
//!    folders to another drive that way) is renamed across after the swap, so it keeps working.
//!
//! Other links inside the old folder are not followed, so the copy can never reach outside it. Each
//! one is logged, with the count: what it pointed at stays where it is, under the old folder.
//!
//! This runs before the shell's single-instance check, so two launches can reach it at once. A lock
//! file beside the new folder makes them take turns, and a folder that holds real data is never moved.
//!
//! When the copy fails (full drive, unreadable file, no permission) the temporary folder is removed
//! and this run **uses the old folder as it is**. That cannot lose anything: the person sees all their
//! settings and recent projects, nothing has been written to a half-finished place, and the next
//! start tries the copy again with whatever changed meanwhile. A fresh start would be worse: it would
//! show an empty app, write a new `app.sqlite` and so stop every later attempt. The reason is logged
//! once logging is up ([`log_notes`]).
//!
//! The person's own folders under the earlier name (the library, project folders) are renamed after
//! this, in `legacy_rename`.
//!
//! Nothing inside these folders names the old folder: `app.sqlite` stores the library folder (the
//! person's own, which does not move), project folders and a cover's file name only,
//! `ai/bundle-check.json` names the installed engine, and the model set is named after the model
//! hashes. So no path is rewritten, and a library pointer to `Documents/<earlier name>` stays right.
//!
//! This never runs on a test data root: `AppConfig::for_current_user` is the only caller, the shells
//! do not call that with `STRAFT_DATA_ROOT` set, and [`allowed`] refuses as well. It never runs in a
//! store package either: an MSIX process has a package identity ([`allowed`]) and keeps its data in
//! `LocalState`, and store builds call `AppConfig::for_store_build`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use fs4::fs_std::FileExt;
use parking_lot::Mutex;
use straft_domain::{AppError, AppResult, now_ms};
use straft_platform::{atomic, dirs, identity, legacy_names, process};

use super::tree;

/// Marker in the name of the temporary sibling.
const TEMP_MARKER: &str = ".migrating-";
/// Marker in the name a new folder without real data has while the copy takes its place.
const ASIDE_MARKER: &str = ".replaced-";
/// Suffix of the lock file beside the new folder.
const LOCK_SUFFIX: &str = ".migrate-lock";
/// The app database, copied as a snapshot.
const DATABASE: &str = "app.sqlite";
/// Not copied: which engines the earlier version launched, and may still be running.
const ENGINE_RECORD: &str = "engines.json";

/// Top-level names of the app data folder that are hard linked, never copied.
const SHARED_NOT_COPIED: &[&str] = &["models"];
/// Inside those, left behind: downloads in progress are written in place, and a set-aside file is
/// never used again.
const NOT_SHARED: &[&str] = &["staging", "quarantine"];
/// How long a second start waits for the first one to finish copying. After that it uses the earlier
/// folder for this run, as for any copy that did not work.
const LOCK_WAIT: Duration = Duration::from_secs(120);
/// Top-level names copied when they can be: losing a log file never holds the person back.
const BEST_EFFORT: &[&str] = &["logs"];
/// Top-level names left behind: the import staging folder, rebuilt on demand.
const LEFT_BEHIND: &[&str] = &["tmp"];

/// What to do about one pair of folders. Separate from the real locations for the tests.
#[derive(Debug, Clone, Copy)]
pub struct Plan<'a> {
    pub shared_not_copied: &'a [&'a str],
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
        shared_not_copied: SHARED_NOT_COPIED,
        best_effort: BEST_EFFORT,
        left_behind: LEFT_BEHIND,
        in_use_marker: DATABASE,
        stale_after: atomic::STALE_AFTER,
    };
    /// The library pointer folder: one small file.
    pub const POINTER: Plan<'static> = Plan {
        shared_not_copied: &[],
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
    let outcome =
        copy_unless_in_use(old, new, plan).unwrap_or_else(|e| Outcome::UseEarlierFolder {
            reason: reason_of(&e),
        });
    if !matches!(outcome, Outcome::UseEarlierFolder { .. }) {
        for name in plan.shared_not_copied {
            move_heavy_link(old, new, name);
        }
    }
    outcome
}

fn copy_unless_in_use(old: &Path, new: &Path, plan: &Plan<'_>) -> AppResult<Outcome> {
    if in_use(new, plan) {
        return Ok(Outcome::AlreadyThere);
    }
    let _turn = TurnLock::wait(new)?;
    // The start that held the lock before this one may have finished the job.
    if in_use(new, plan) {
        return Ok(Outcome::AlreadyThere);
    }
    copy_to_new_name(old, new, plan)
}

/// An exclusive lock on `<new>.migrate-lock`, held until dropped. The wait is bounded: a start that
/// cannot get its turn must never hang before it has a window.
struct TurnLock {
    file: fs::File,
    path: PathBuf,
}

impl TurnLock {
    fn wait(new: &Path) -> AppResult<TurnLock> {
        let name = new
            .file_name()
            .ok_or_else(|| AppError::internal("the new folder has no name"))?
            .to_string_lossy();
        let path = new.with_file_name(format!("{name}{LOCK_SUFFIX}"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;
        let started = std::time::Instant::now();
        loop {
            match FileExt::try_lock_exclusive(&file) {
                Ok(true) => return Ok(TurnLock { file, path }),
                Ok(false) => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
            if started.elapsed() > LOCK_WAIT {
                return Err(AppError::storage(
                    "another start was still copying the earlier folder",
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for TurnLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
        let _ = fs::remove_file(&self.path);
    }
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

/// After the swap: move what the folder set aside holds (the log folder of a failed start, say) into
/// the new folder, and remove it. The earlier version's file wins where both have one.
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

/// Remove what an interrupted copy left beside `new`, at once when the process that made it is gone.
/// While that process is alive it may be another start copying right now, so the folder waits for
/// `stale_after` (the pid may have been reused). Links are never followed. Removing a hard link leaves
/// the old folder's file as it is.
fn remove_stale_temporaries(new: &Path, stale_after: Duration) {
    let (Some(parent), Some(prefix)) = (new.parent(), temporary_prefix(new)) else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let owner_gone = rest
            .split_once('-')
            .and_then(|(pid, _)| pid.parse::<u32>().ok())
            .is_some_and(|pid| !process::is_running(pid));
        let old_enough = owner_gone
            || entry
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
    let prefix =
        temporary_prefix(new).ok_or_else(|| AppError::internal("the new folder has no name"))?;
    let temp = parent.join(format!("{prefix}{}-{}", std::process::id(), now_ms()));
    fs::create_dir(&temp)?;
    let copied = fill_temporary(old, &temp, plan).and_then(|()| {
        atomic::sync_folder(&temp);
        swap_into_place(&temp, new, parent, plan).or_else(|e| {
            // Another start of the app may have finished in the same moment.
            if in_use(new, plan) {
                Ok(Outcome::AlreadyThere)
            } else {
                Err(e)
            }
        })
    });
    // A failed or lost rename leaves the temporary folder; success has renamed it away. Nothing of
    // the new folder is in it: that went aside and is back in place by now.
    if temp.exists() {
        let _ = fs::remove_dir_all(&temp);
    }
    copied
}

/// Rename `temp` to `new`. A new folder that is already there (without real data) goes aside first
/// and comes back if the rename fails, so a failure takes nothing away from it. A folder that holds
/// real data is never moved: another start finished first, and its copy is the one to use.
fn swap_into_place(temp: &Path, new: &Path, parent: &Path, plan: &Plan<'_>) -> AppResult<Outcome> {
    if in_use(new, plan) {
        return Ok(Outcome::AlreadyThere);
    }
    let name = new.file_name().unwrap_or_default().to_string_lossy();
    let aside = parent.join(format!(
        "{name}{ASIDE_MARKER}{}-{}",
        std::process::id(),
        now_ms()
    ));
    let moved_aside = new.exists();
    if moved_aside {
        atomic::rename_with_retry(new, &aside)?;
    }
    if let Err(e) = atomic::rename_with_retry(temp, new) {
        if moved_aside && atomic::rename_with_retry(&aside, new).is_err() {
            note(
                Level::Error,
                "a folder set aside for the copy could not be put back; it stays beside the new folder".into(),
            );
        }
        return Err(e.into());
    }
    atomic::sync_folder(parent);
    if moved_aside && take_along(&aside, new).is_err() {
        note(
            Level::Warn,
            "some files of a folder set aside for the copy stay beside the new folder".into(),
        );
    }
    Ok(Outcome::Copied)
}

/// Copied some other way or not at all: the database and its side files (see [`DATABASE`]), the
/// engine record, a SQLite side file rebuilt on the first open, or a staging file a crash left behind.
fn not_needed_file(rel: &str) -> bool {
    rel == ENGINE_RECORD
        || rel
            .strip_prefix(DATABASE)
            .is_some_and(|rest| matches!(rest, "" | "-wal" | "-shm" | "-journal"))
        || rel.ends_with(".sqlite-shm")
        || rel.ends_with(atomic::TEMP_SUFFIX)
        || rel.ends_with(legacy_names::TEMP_SUFFIX)
}

fn fill_temporary(old: &Path, temp: &Path, plan: &Plan<'_>) -> AppResult<()> {
    let skip: Vec<&str> = plan
        .shared_not_copied
        .iter()
        .chain(plan.best_effort)
        .chain(plan.left_behind)
        .copied()
        .collect();
    let mut source = tree::walk(old, &skip)?;
    source.files.retain(|f| !not_needed_file(&f.rel));
    // Names inside the app data folder: no project title or person's path.
    for link in &source.links {
        note(
            Level::Warn,
            format!("a link in the earlier folder was not followed: {link}"),
        );
    }
    if !source.links.is_empty() {
        note(
            Level::Warn,
            format!(
                "{} links in the earlier folder were not copied",
                source.links.len()
            ),
        );
    }
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
    for name in plan.shared_not_copied {
        share_heavy_folder(&old.join(name), &temp.join(name));
    }
    if old.join(DATABASE).is_file() {
        straft_persistence::snapshot_database(&old.join(DATABASE), &temp.join(DATABASE))?;
    }
    check_copied_database(temp)?;
    Ok(())
}

/// Find a bad copy now, while the earlier folder is still the one in use. The connection is closed
/// again before the rename.
fn check_copied_database(temp: &Path) -> AppResult<()> {
    let db = temp.join(DATABASE);
    if !db.is_file() {
        return Ok(());
    }
    if straft_persistence::copied_database_is_sound(&db)? {
        Ok(())
    } else {
        Err(AppError::storage(
            "the copied app database failed its check",
        ))
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

/// Hard link a heavy folder into the temporary folder. Anything that stops it leaves no part of the
/// folder behind: the person downloads the models again. A link at this name is not followed.
fn share_heavy_folder(from: &Path, to: &Path) {
    if !fs::symlink_metadata(from).is_ok_and(|m| m.is_dir()) {
        return;
    }
    if let Err(e) = link_files(from, to, NOT_SHARED) {
        let _ = fs::remove_dir_all(to);
        note(
            Level::Warn,
            format!(
                "the downloaded models stay in the earlier folder and can be downloaded again: {}",
                e.kind()
            ),
        );
    }
}

/// A heavy folder that is itself a link to another place: the link moves across after the swap, and
/// the files stay where the person put them. A new folder that already has its own set is left alone,
/// and a failure leaves the link where it is for the next start.
fn move_heavy_link(old: &Path, new: &Path, name: &str) {
    let (from, to) = (old.join(name), new.join(name));
    if !fs::symlink_metadata(&from).is_ok_and(|m| m.file_type().is_symlink()) {
        return;
    }
    if let Ok(m) = fs::symlink_metadata(&to) {
        // `remove_dir` only succeeds on an empty folder: a model set is never deleted here.
        if m.file_type().is_symlink() || fs::remove_dir(&to).is_err() {
            return;
        }
    }
    match atomic::rename_with_retry(&from, &to) {
        Ok(()) => {
            atomic::sync_folder(new);
            atomic::sync_folder(old);
            note(
                Level::Info,
                "the link to the downloaded models moved across".into(),
            );
        }
        Err(e) => note(
            Level::Warn,
            format!(
                "the link to the downloaded models stayed in the earlier folder: {}",
                e.kind()
            ),
        ),
    }
}

/// The folders of `from` recreated in `to`, each of its real files linked. Links are skipped.
fn link_files(from: &Path, to: &Path, skip: &[&str]) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_str().is_some_and(|n| skip.contains(&n)) {
            continue;
        }
        let (kind, target) = (entry.file_type()?, to.join(&name));
        if kind.is_dir() {
            link_files(&entry.path(), &target, &[])?;
        } else if kind.is_file() {
            fs::hard_link(entry.path(), &target)?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------ the person's own folders

/// False when this run must not touch the person's real folders: a test data root is set, the
/// process was started by cargo (`cargo test` reaches `AppConfig::for_current_user` from the FFI
/// crate's own tests, and a developer's real data sits in these folders), or the process runs from a
/// store package. A package keeps its data in `LocalState`, so it would take an installed copy's
/// models beside it and then start empty there.
pub fn allowed(
    data_root: Option<&OsStr>,
    started_by_cargo: Option<&OsStr>,
    package_identity: Option<&str>,
) -> bool {
    data_root.is_none() && started_by_cargo.is_none() && package_identity.is_none()
}

/// [`allowed`] for this process.
pub(crate) fn allowed_here() -> bool {
    allowed(
        std::env::var_os("STRAFT_DATA_ROOT").as_deref(),
        std::env::var_os("CARGO_MANIFEST_DIR").as_deref(),
        identity::package_family_name().as_deref(),
    )
}

/// The folders to use in this run: `(app data, library pointer)`.
pub(crate) fn folders_for_current_user() -> (Option<PathBuf>, Option<PathBuf>) {
    let app_data = dirs::app_data_dir();
    let pointer = dirs::roaming_data_dir();
    if !allowed_here() {
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
                format!(
                    "an earlier folder could not be copied; it stays in use and the copy is tried again at the next start: {reason}"
                ),
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
        shared_not_copied: SHARED_NOT_COPIED,
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
        for sub in [
            "entitlement",
            "ai",
            "covers/assets",
            "logs",
            "tmp/x",
            "models/m-abc",
        ] {
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
    fn the_folder_is_copied_the_models_are_linked_and_the_old_folder_stays() {
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
        // The old folder keeps everything, the models too.
        assert!(old.join("settings.txt").is_file());
        assert!(old.join("tmp/x/half.bin").is_file());
        assert!(old.join("models/m-abc/chat.gguf").is_file());
        assert!(temporaries(dir.path()).is_empty());
        // One file with two names: no second copy, and taking one name away leaves the other.
        let written_through = new.join("models/m-abc/chat.gguf");
        fs::OpenOptions::new()
            .append(true)
            .open(&written_through)
            .and_then(|mut f| std::io::Write::write_all(&mut f, b"!"))
            .unwrap();
        assert_eq!(
            fs::read(old.join("models/m-abc/chat.gguf")).unwrap(),
            b"gigabytes!",
            "a hard link, not a copy"
        );
        fs::remove_file(&written_through).unwrap();
        assert!(old.join("models/m-abc/chat.gguf").is_file());
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
        assert!(
            old.join("models/m-abc/chat.gguf").is_file(),
            "not moved over a set"
        );
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
        assert_eq!(
            fs::read(new.join("logs/failed-start.log")).unwrap(),
            b"kept"
        );
        assert_eq!(
            fs::read(new.join("logs/earlier.2026-10-05.log")).unwrap(),
            b"log line"
        );
        assert!(new.join("models/active.json").is_file());
        assert!(temporaries(dir.path()).is_empty());
    }

    /// The final rename fails: what the new folder held is all still there, and nothing is deleted.
    #[test]
    fn a_failed_final_rename_puts_the_new_folder_back() {
        let dir = tempfile::tempdir().unwrap();
        let new = dir.path().join("New Name");
        fs::create_dir_all(new.join("logs")).unwrap();
        fs::write(new.join("logs/failed-start.log"), "kept").unwrap();
        // A temporary folder that is not there makes the real rename fail.
        let result = swap_into_place(&dir.path().join("absent"), &new, dir.path(), &NOW);
        assert!(result.is_err());
        assert_eq!(
            fs::read(new.join("logs/failed-start.log")).unwrap(),
            b"kept"
        );
        let beside: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(beside, ["New Name"], "nothing is left beside it");
    }

    /// The whole path fails after the new folder was seen: a read-only spot for the rename is not
    /// portable, so this drives a real failure earlier in the copy.
    #[test]
    fn a_failed_copy_leaves_a_new_folder_without_real_data_untouched() {
        let (dir, old, new) = folders();
        fs::create_dir_all(new.join("logs")).unwrap();
        fs::write(new.join("logs/failed-start.log"), "kept").unwrap();
        fs::write(old.join("app.sqlite"), vec![0x5a; 8192]).unwrap();
        let outcome = bring_over(&old, &new, &NOW);
        assert!(
            matches!(outcome, Outcome::UseEarlierFolder { .. }),
            "{outcome:?}"
        );
        assert_eq!(
            fs::read(new.join("logs/failed-start.log")).unwrap(),
            b"kept"
        );
        assert!(temporaries(dir.path()).is_empty());
    }

    /// A person who removed the AI from the new folder does not get it back from the old one.
    #[test]
    fn a_new_folder_in_use_never_takes_the_models_again() {
        let (_dir, old, new) = folders();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        fs::remove_dir_all(new.join("models")).unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert!(!new.join("models").exists());
        assert!(old.join("models/m-abc/chat.gguf").is_file());
    }

    /// Downloads in progress are written in place, so they would write into the old folder too.
    #[test]
    fn a_download_in_progress_is_not_linked() {
        let (_dir, old, new) = folders();
        for sub in ["staging/m-next", "quarantine"] {
            fs::create_dir_all(old.join("models").join(sub)).unwrap();
        }
        fs::write(old.join("models/staging/m-next/chat.gguf.partial"), "half").unwrap();
        fs::write(old.join("models/quarantine/1-chat.gguf.bad"), "bad").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(new.join("models/m-abc/chat.gguf").is_file());
        assert!(!new.join("models/staging").exists());
        assert!(!new.join("models/quarantine").exists());
        assert!(
            old.join("models/staging/m-next/chat.gguf.partial")
                .is_file()
        );
    }

    /// Another volume, a drive without hard links: no part of the set stays, nothing else is lost.
    #[test]
    fn a_link_that_cannot_be_made_leaves_no_half_set() {
        let (_dir, old, new) = folders();
        let to = new.with_file_name("linked");
        fs::create_dir_all(to.join("active.json")).unwrap();
        share_heavy_folder(&old.join("models"), &to);
        assert!(!to.exists());
        assert!(old.join("models/m-abc/chat.gguf").is_file());
    }

    /// Two starts at once: each copies on its own, one wins, and the result is whole either way.
    #[test]
    fn two_starts_at_once_end_with_one_whole_folder() {
        for _ in 0..8 {
            let (dir, old, new) = folders();
            let start = std::sync::Arc::new(std::sync::Barrier::new(2));
            let runs: Vec<_> = (0..2)
                .map(|_| {
                    let (old, new, start) = (old.clone(), new.clone(), start.clone());
                    std::thread::spawn(move || {
                        start.wait();
                        bring_over(&old, &new, &NOW)
                    })
                })
                .collect();
            for run in runs {
                let outcome = run.join().unwrap();
                assert!(
                    matches!(outcome, Outcome::Copied | Outcome::AlreadyThere),
                    "{outcome:?}"
                );
            }
            assert_eq!(fs::read(new.join("settings.txt")).unwrap(), b"settings");
            assert!(new.join("models/m-abc/chat.gguf").is_file());
            assert!(old.join("models/m-abc/chat.gguf").is_file());
            assert!(temporaries(dir.path()).is_empty());
        }
    }

    /// `mklink /J` needs no privilege, unlike a symbolic link. `None` when the tool is not there.
    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) -> Option<()> {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|_| ())
    }

    /// People move big folders to another drive with a link: the old folder itself is followed.
    #[cfg(windows)]
    #[test]
    fn an_earlier_folder_that_is_a_link_is_followed() {
        let (dir, real, new) = folders();
        let link = dir.path().join("Old Link");
        junction(&link, &real).expect("mklink");
        assert_eq!(bring_over(&link, &new, &NOW), Outcome::Copied);
        assert_eq!(fs::read(new.join("settings.txt")).unwrap(), b"settings");
        assert!(new.join("models/m-abc/chat.gguf").is_file());
    }

    /// A link inside it is not followed, so what it points at does not come across. It is reported.
    #[cfg(windows)]
    #[test]
    fn a_junction_inside_the_earlier_folder_is_not_followed_and_is_reported() {
        let (dir, old, new) = folders();
        let outside = dir.path().join("private");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "not ours").unwrap();
        fs::remove_dir_all(old.join("covers")).unwrap();
        junction(&old.join("covers"), &outside).expect("mklink");
        junction(&old.join("ai/link"), &outside).expect("mklink");
        let mut links = tree::walk(&old, &[]).unwrap().links;
        links.sort();
        assert_eq!(links, ["ai/link", "covers"]);
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert!(!new.join("covers").exists());
        assert!(!new.join("ai/link").exists());
        assert!(new.join("ai/graphics.json").is_file());
        assert!(outside.join("secret.txt").is_file());
    }

    /// Models moved to another drive with a junction: the junction comes across, so nothing is
    /// downloaded again and the files stay where the person put them.
    #[cfg(windows)]
    #[test]
    fn a_models_folder_that_is_a_junction_keeps_working() {
        let (dir, old, new) = folders();
        let drive = dir.path().join("Other Drive");
        fs::rename(old.join("models"), &drive).unwrap();
        junction(&old.join("models"), &drive).expect("mklink");
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        assert_eq!(
            fs::read(new.join("models/m-abc/chat.gguf")).unwrap(),
            b"gigabytes"
        );
        assert!(
            fs::symlink_metadata(new.join("models"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(fs::symlink_metadata(old.join("models")).is_err());
        assert!(drive.join("m-abc/chat.gguf").is_file());
    }

    /// A new folder with its own set keeps it; the old link waits where it is.
    #[cfg(windows)]
    #[test]
    fn a_models_junction_never_replaces_a_set_in_the_new_folder() {
        let (dir, old, new) = folders();
        let drive = dir.path().join("Other Drive");
        fs::rename(old.join("models"), &drive).unwrap();
        junction(&old.join("models"), &drive).expect("mklink");
        fs::create_dir_all(new.join("models/m-new")).unwrap();
        fs::write(new.join("models/m-new/mine.gguf"), "mine").unwrap();
        fs::write(new.join("settings.txt"), "new").unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::AlreadyThere);
        assert!(new.join("models/m-new/mine.gguf").is_file());
        assert!(
            fs::symlink_metadata(old.join("models"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    /// A second start that lost the race never moves the winner's folder aside.
    #[test]
    fn a_folder_that_holds_real_data_is_never_swapped_out() {
        let (dir, _old, new) = folders();
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join("settings.txt"), "the first start's").unwrap();
        fs::write(new.join("first.txt"), "written by the first start").unwrap();
        let temp = dir.path().join("temp");
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("settings.txt"), "the second start's").unwrap();
        assert_eq!(
            swap_into_place(&temp, &new, dir.path(), &NOW).unwrap(),
            Outcome::AlreadyThere
        );
        assert_eq!(
            fs::read(new.join("settings.txt")).unwrap(),
            b"the first start's"
        );
        assert!(new.join("first.txt").is_file());
        assert!(temp.exists(), "nothing of the second start went into place");
    }

    /// The second launch waits for its turn and then finds the first launch's folder.
    #[test]
    fn a_second_launch_waits_for_the_first_and_destroys_nothing() {
        let (dir, old, new) = folders();
        let first = TurnLock::wait(&new).unwrap();
        let second = {
            let (old, new) = (old.clone(), new.clone());
            std::thread::spawn(move || bring_over(&old, &new, &NOW))
        };
        std::thread::sleep(Duration::from_millis(300));
        assert!(!second.is_finished(), "it waits for its turn");
        fs::create_dir_all(&new).unwrap();
        fs::write(new.join("settings.txt"), "the first start's").unwrap();
        drop(first);
        assert_eq!(second.join().unwrap(), Outcome::AlreadyThere);
        assert_eq!(
            fs::read(new.join("settings.txt")).unwrap(),
            b"the first start's"
        );
        assert!(!new.join("ai").exists(), "nothing was merged in");
        assert!(temporaries(dir.path()).is_empty());
        assert!(!dir.path().join(format!("New Name{LOCK_SUFFIX}")).exists());
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

    /// A force-quit leaves a temporary folder whose process is gone. It goes at once, even when
    /// young: keeping it would put a second full copy on the drive.
    #[test]
    fn a_temporary_folder_whose_process_is_gone_is_removed_at_once() {
        let (dir, old, new) = folders();
        let left = dir.path().join(format!("New Name{TEMP_MARKER}99999999-1"));
        let odd = dir.path().join(format!("New Name{TEMP_MARKER}no-pid-here"));
        fs::create_dir_all(&left).unwrap();
        fs::create_dir_all(&odd).unwrap();
        assert!(!process::is_running(99_999_999));
        assert_eq!(bring_over(&old, &new, &Plan::APP_DATA), Outcome::Copied);
        assert!(!left.exists());
        assert!(odd.exists(), "no pid to judge by: it waits for its age");
    }

    /// Two starts at once: a young sibling of a running process may be the other one copying. It is
    /// not taken away from under it; this start copies on its own and whichever renames first wins.
    #[test]
    fn a_young_temporary_folder_of_a_running_process_is_left_for_a_later_start() {
        let (dir, old, new) = folders();
        assert!(process::is_running(std::process::id()));
        let left = dir
            .path()
            .join(format!("New Name{TEMP_MARKER}{}-1", std::process::id()));
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
    fn side_files_staging_files_and_the_engine_record_are_not_copied() {
        let (_dir, old, new) = folders();
        for name in ["app.sqlite-shm", "app.sqlite-wal", "app.sqlite-journal"] {
            fs::write(old.join(name), "taken by the snapshot, not as a file").unwrap();
        }
        fs::write(old.join("engines.json"), "the earlier version's engine").unwrap();
        fs::write(
            old.join(format!(".x.0123456789abcdef{}", atomic::TEMP_SUFFIX)),
            "x",
        )
        .unwrap();
        fs::write(
            old.join(format!(".x.0123456789abcdef{}", legacy_names::TEMP_SUFFIX)),
            "x",
        )
        .unwrap();
        assert_eq!(bring_over(&old, &new, &NOW), Outcome::Copied);
        for name in [
            "app.sqlite-shm",
            "app.sqlite-wal",
            "app.sqlite-journal",
            "engines.json",
        ] {
            assert!(!new.join(name).exists(), "{name}");
            assert!(
                old.join(name).exists(),
                "{name} stays for the earlier version"
            );
        }
        assert!(
            listing(&new)
                .iter()
                .all(|n| !n.contains("0123456789abcdef"))
        );
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
        assert!(
            matches!(outcome, Outcome::UseEarlierFolder { .. }),
            "{outcome:?}"
        );
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
        assert!(
            matches!(outcome, Outcome::UseEarlierFolder { .. }),
            "{outcome:?}"
        );
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
        assert!(
            matches!(outcome, Outcome::UseEarlierFolder { .. }),
            "{outcome:?}"
        );
        assert!(!new.exists());
        assert!(temporaries(dir.path()).is_empty());
        assert!(old.join("models/active.json").is_file());
    }

    #[test]
    fn test_data_roots_and_cargo_runs_never_touch_the_real_folders() {
        let set = OsStr::new("/tmp/x");
        assert!(allowed(None, None, None));
        assert!(!allowed(Some(set), None, None));
        assert!(!allowed(None, Some(set), None));
        assert!(!allowed(Some(set), Some(set), None));
    }

    #[test]
    fn a_store_package_never_looks_for_earlier_folders() {
        assert!(!allowed(None, None, Some("Straft.Studio_8wekyb3d8bbwe")));
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
