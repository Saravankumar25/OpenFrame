//! One shared atomic file replace (build plan item "One shared atomic file replace").
//!
//! Rules, the same on Windows and macOS:
//!
//! 1. The new content is written to a temp file **in the destination's folder** with a unique
//!    name, so the final step is a same-volume rename and never a cross-volume copy.
//! 2. The temp file is flushed and synced before the rename.
//! 3. The old file is **never deleted before the rename**. A rename over an existing file is
//!    atomic on NTFS and APFS; if it fails the old file is untouched.
//! 4. On any failure (including a panic inside the writer) the temp file is removed. Nothing with
//!    the temp suffix is left behind by a completed call.
//! 5. A read-only destination fails with the original error and keeps the old file.
//!
//! A rename that fails with a sharing violation or permission error is retried a few times, because
//! on Windows an antivirus scanner or the search indexer can hold the destination for a moment.
//!
//! Callers never see the temp name. Project format, import and export, the AI store, zip writing,
//! backup and the persistence checkpoint all use this.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Suffix of the staging files this module creates. Other code must not create files ending in
/// this suffix next to user data, and the cleanup in [`remove_stale_temps`] only removes files
/// that carry it, match the full pattern and are older than [`STALE_AFTER`].
pub const TEMP_SUFFIX: &str = ".openframe-tmp";

/// A staging file younger than this is assumed to belong to a write in progress.
pub const STALE_AFTER: Duration = Duration::from_secs(10 * 60);

const RENAME_ATTEMPTS: u32 = 5;
const RENAME_BACKOFF: Duration = Duration::from_millis(40);

/// Removes the temp file on drop unless disarmed; covers the panic path of the writer.
struct TempGuard<'a> {
    path: &'a Path,
    armed: bool,
}

impl Drop for TempGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(self.path);
        }
    }
}

/// Replace `dest` with the bytes produced by `write`.
///
/// `write` receives the open temp file. If it returns an error or panics, or the sync or rename
/// fails, the temp file is removed and `dest` is left exactly as it was.
pub fn replace_with(
    dest: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "destination has no folder"))?;
    if dest.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination is a folder",
        ));
    }
    let (temp, mut file) = create_temp(parent, dest)?;
    let mut guard = TempGuard {
        path: &temp,
        armed: true,
    };
    write(&mut file)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    rename_with_retry(&temp, dest)?;
    guard.armed = false;
    Ok(())
}

/// Replace `dest` with `bytes`.
pub fn replace_bytes(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    replace_with(dest, |f| f.write_all(bytes))
}

/// Move an already written staging file onto `dest`. The staging file must have been created by
/// [`unique_temp_path`] in the same folder as `dest`; it is synced before the rename and removed
/// when anything fails.
pub fn replace_from_staged(staged: &Path, dest: &Path) -> io::Result<()> {
    if !is_our_temp_name(staged) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "staging file was not created by unique_temp_path",
        ));
    }
    let same_folder = match (staged.parent(), dest.parent()) {
        (Some(a), Some(b)) => {
            a == b
                || (fs::canonicalize(a).ok() == fs::canonicalize(b).ok()
                    && fs::canonicalize(a).is_ok())
        }
        _ => false,
    };
    if !same_folder {
        let _ = fs::remove_file(staged);
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "staging file must be in the destination folder",
        ));
    }
    let r = (|| {
        // Write access is needed for FlushFileBuffers on Windows.
        OpenOptions::new().write(true).open(staged)?.sync_all()?;
        rename_with_retry(staged, dest)
    })();
    if r.is_err() {
        let _ = fs::remove_file(staged);
    }
    r
}

/// A unique staging path in `parent` for a write that will end at `dest`. The name carries a
/// shortened form of the destination's file name, a random tag and [`TEMP_SUFFIX`]. The file does
/// not exist yet; the caller creates it (use `create_new` so a race cannot reuse it).
pub fn unique_temp_path(parent: &Path, dest: &Path) -> io::Result<PathBuf> {
    for _ in 0..64 {
        let candidate = parent.join(temp_name(dest));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not find a free staging name",
    ))
}

/// Remove staging files left in `folder` by a process that stopped mid-write. Only files whose
/// name matches the full temp pattern and that are older than [`STALE_AFTER`] are touched, so a
/// write in progress from another window is never disturbed. Returns how many were removed.
pub fn remove_stale_temps(folder: &Path) -> usize {
    let Ok(entries) = fs::read_dir(folder) else {
        return 0;
    };
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_our_temp_name(&path) {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let old_enough = meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .map(|age| age >= STALE_AFTER)
            .unwrap_or(false);
        if old_enough && fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

fn create_temp(parent: &Path, dest: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..64 {
        let candidate = parent.join(temp_name(dest));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(f) => return Ok((candidate, f)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not find a free staging name",
    ))
}

/// `.<stem>.<16 hex>.openframe-tmp`, with the stem cut so the name stays well under 255 bytes.
fn temp_name(dest: &Path) -> String {
    let stem = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let stem: String = stem.chars().take(120).collect();
    let tag: u64 = rand::random();
    format!(".{stem}.{tag:016x}{TEMP_SUFFIX}")
}

/// Whether `path` has the exact shape produced by [`temp_name`].
fn is_our_temp_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(body) = name
        .strip_prefix('.')
        .and_then(|n| n.strip_suffix(TEMP_SUFFIX))
    else {
        return false;
    };
    match body.rsplit_once('.') {
        Some((stem, tag)) => {
            !stem.is_empty() && tag.len() == 16 && tag.chars().all(|c| c.is_ascii_hexdigit())
        }
        None => false,
    }
}

fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let mut attempt = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if attempt + 1 < RENAME_ATTEMPTS && is_transient_share_error(&e) => {
                attempt += 1;
                std::thread::sleep(RENAME_BACKOFF * attempt);
            }
            Err(e) => return Err(e),
        }
    }
}

/// Windows sharing violation (32) and lock violation (33), or a PermissionDenied that a scanner
/// holding the destination produces. A read-only destination also reports PermissionDenied; the
/// retries then cost at most a few hundred milliseconds before the honest error is returned.
fn is_transient_share_error(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(32) | Some(33)) || e.kind() == io::ErrorKind::PermissionDenied
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temps_in(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(TEMP_SUFFIX))
            .collect()
    }

    #[test]
    fn replaces_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("settings.json");
        fs::write(&dest, b"old").unwrap();
        replace_bytes(&dest, b"new content").unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new content");
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn creates_a_missing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("fresh.bin");
        replace_bytes(&dest, &[1, 2, 3]).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), vec![1, 2, 3]);
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn a_failing_writer_keeps_the_old_file_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("project.json");
        fs::write(&dest, b"old").unwrap();
        let err = replace_with(&dest, |f| {
            f.write_all(b"half")?;
            Err(io::Error::other("disk full (simulated)"))
        })
        .unwrap_err();
        assert_eq!(err.to_string(), "disk full (simulated)");
        assert_eq!(fs::read(&dest).unwrap(), b"old");
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn a_panicking_writer_keeps_the_old_file_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("project.json");
        fs::write(&dest, b"old").unwrap();
        let r = std::panic::catch_unwind(|| {
            let _ = replace_with(&dest, |_| panic!("writer bug"));
        });
        assert!(r.is_err());
        assert_eq!(fs::read(&dest).unwrap(), b"old");
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn read_only_destination_keeps_the_old_file_and_reports_the_error() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("locked.txt");
        fs::write(&dest, b"keep me").unwrap();
        let mut perms = fs::metadata(&dest).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&dest, perms).unwrap();

        let result = replace_bytes(&dest, b"replaced");

        let mut perms = fs::metadata(&dest).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        fs::set_permissions(&dest, perms).unwrap();

        if cfg!(windows) {
            assert!(result.is_err(), "rename over a read-only file must fail");
            assert_eq!(fs::read(&dest).unwrap(), b"keep me");
        } else {
            assert!(result.is_ok());
        }
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn a_folder_as_destination_is_refused_without_side_effects() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("folder");
        fs::create_dir(&sub).unwrap();
        assert!(replace_bytes(&sub, b"x").is_err());
        assert!(sub.is_dir());
        assert!(temps_in(dir.path()).is_empty());
    }

    #[test]
    fn missing_folder_fails_without_creating_anything() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("gone").join("file.txt");
        assert!(replace_bytes(&dest, b"x").is_err());
        assert!(!dir.path().join("gone").exists());
    }

    #[test]
    fn staged_file_must_be_ours_and_in_the_destination_folder() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.zip");

        // Not our pattern: refused and NOT deleted (it could be a user's file).
        let foreign = dir.path().join("stage.partial");
        fs::write(&foreign, b"x").unwrap();
        assert!(replace_from_staged(&foreign, &dest).is_err());
        assert!(foreign.exists());

        // Our pattern but another folder: refused and removed.
        let staged = unique_temp_path(other.path(), &dest).unwrap();
        fs::write(&staged, b"x").unwrap();
        assert!(replace_from_staged(&staged, &dest).is_err());
        assert!(!staged.exists());
        assert!(!dest.exists());
    }

    #[test]
    fn staged_file_in_place_is_renamed_over_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("out.zip");
        fs::write(&dest, b"old").unwrap();
        let staged = unique_temp_path(dir.path(), &dest).unwrap();
        fs::write(&staged, b"new").unwrap();
        replace_from_staged(&staged, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert!(!staged.exists());
    }

    #[test]
    fn temp_names_are_unique_hidden_bounded_and_in_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("report.pdf");
        let a = unique_temp_path(dir.path(), &dest).unwrap();
        let b = unique_temp_path(dir.path(), &dest).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.parent().unwrap(), dir.path());
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with(".report.pdf."));
        assert!(name.ends_with(TEMP_SUFFIX));
        assert!(is_our_temp_name(&a));

        let long = dir.path().join("x".repeat(240) + ".txt");
        let t = unique_temp_path(dir.path(), &long).unwrap();
        assert!(t.file_name().unwrap().len() < 160);
        replace_bytes(&dir.path().join("y".repeat(200)), b"ok").unwrap();
    }

    #[test]
    fn stale_temps_are_removed_and_fresh_or_user_files_are_not() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("a.txt");
        let stale = unique_temp_path(dir.path(), &dest).unwrap();
        fs::write(&stale, b"left behind").unwrap();
        let fresh = unique_temp_path(dir.path(), &dest).unwrap();
        fs::write(&fresh, b"being written").unwrap();
        fs::write(dir.path().join("a.txt"), b"user").unwrap();
        fs::write(dir.path().join("notes.openframe-tmp.txt"), b"user too").unwrap();
        fs::write(
            dir.path().join(".draft.openframe-tmp"),
            b"user, wrong shape",
        )
        .unwrap();
        let old = SystemTime::now() - STALE_AFTER - Duration::from_secs(60);
        OpenOptions::new()
            .write(true)
            .open(&stale)
            .unwrap()
            .set_modified(old)
            .unwrap();

        assert_eq!(remove_stale_temps(dir.path()), 1);
        assert!(!stale.exists());
        assert!(fresh.exists());
        assert!(dir.path().join("a.txt").exists());
        assert!(dir.path().join("notes.openframe-tmp.txt").exists());
        assert!(dir.path().join(".draft.openframe-tmp").exists());
        assert_eq!(remove_stale_temps(&dir.path().join("missing")), 0);
    }

    #[test]
    fn large_write_is_complete_or_absent() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("big.bin");
        let data = vec![7u8; 4 * 1024 * 1024];
        replace_bytes(&dest, &data).unwrap();
        assert_eq!(fs::metadata(&dest).unwrap().len(), data.len() as u64);
    }
}
