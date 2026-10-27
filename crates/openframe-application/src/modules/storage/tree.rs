//! Copying a folder safely, for the vault and project moves (doc 40 sections 4 and 5): list what
//! is there, copy it with a hash of every file, check the copy, and later remove only what was
//! copied. Links and junctions are never followed or copied.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, now_ms};
use openframe_security::sha256_file;

use super::STAGING_MARKER;

/// More files than this in one move is not a vault or a project; stop instead of running for hours.
const MAX_FILES: usize = 500_000;
const CHUNK: usize = 1 << 20;

pub(super) struct FileEntry {
    /// Path inside the folder, with forward slashes.
    pub rel: String,
    pub path: PathBuf,
}

#[derive(Default)]
pub(super) struct Tree {
    pub files: Vec<FileEntry>,
    /// Every folder (relative, forward slashes), so empty ones are recreated too.
    pub dirs: Vec<String>,
    pub bytes: u64,
}

/// What was copied, for the check and for the removal of the source.
pub(super) struct Copied {
    pub rel: String,
    pub size: u64,
    pub sha256: String,
}

/// Files Finder writes into any folder it shows (`.DS_Store`, and `._name` AppleDouble files on
/// drives without extended attributes). They are not OpenFrame data: a move neither copies nor
/// checks them, and they never make a folder count as holding something.
pub(crate) fn is_finder_metadata(name: &str) -> bool {
    name == ".DS_Store" || name.starts_with("._")
}

/// True when `dir` holds nothing, or only Finder metadata files.
pub(super) fn is_empty_but_for_finder_files(dir: &Path) -> std::io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if !(entry.file_type()?.is_file() && is_finder_metadata(&name.to_string_lossy())) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Remove the Finder metadata files directly inside `dir`, so an otherwise empty folder can be
/// removed with `remove_dir`. Nothing else is touched.
pub(super) fn clear_finder_files(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_file())
            && is_finder_metadata(&entry.file_name().to_string_lossy())
        {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Everything under `root` except the top-level names in `skip_top` (matched without case) and
/// Finder metadata files.
pub(super) fn walk(root: &Path, skip_top: &[&str]) -> AppResult<Tree> {
    let mut tree = Tree::default();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel_dir)) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if rel_dir.is_empty() && skip_top.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                continue;
            }
            let rel = if rel_dir.is_empty() {
                name
            } else {
                format!("{rel_dir}/{name}")
            };
            let kind = entry.file_type()?;
            if kind.is_dir() {
                tree.dirs.push(rel.clone());
                pending.push((entry.path(), rel));
            } else if kind.is_file() && !is_finder_metadata(&rel_name(&rel)) {
                let size = entry.metadata()?.len();
                tree.bytes += size;
                tree.files.push(FileEntry {
                    rel,
                    path: entry.path(),
                });
                if tree.files.len() > MAX_FILES {
                    return Err(AppError::invalid_input(
                        "That folder holds too many files to move safely.",
                    ));
                }
            }
        }
    }
    tree.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(tree)
}

/// Copy every file and folder of `tree` into `dest` (which exists) and record the SHA-256 of each
/// source file.
/// `cancelled` is polled between chunks; the caller removes `dest` on any error.
pub(super) fn copy(
    tree: &Tree,
    dest: &Path,
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<Vec<Copied>> {
    for dir in &tree.dirs {
        fs::create_dir_all(dest.join(openframe_security::validate_relative(dir)?))?;
    }
    let total = tree.bytes.max(1);
    let mut done = 0u64;
    let mut copied = Vec::with_capacity(tree.files.len());
    let mut buf = vec![0u8; CHUNK];
    for file in &tree.files {
        let target = dest.join(openframe_security::validate_relative(&file.rel)?);
        let mut input = fs::File::open(&file.path)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let mut size = 0u64;
        loop {
            if cancelled() {
                return Err(AppError::cancelled());
            }
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            output.write_all(&buf[..n])?;
            size += n as u64;
            done += n as u64;
            progress(0.05 + 0.75 * (done as f64 / total as f64), "Copying files");
        }
        output.sync_all()?;
        copied.push(Copied {
            rel: file.rel.clone(),
            size,
            sha256: sha256_file(&file.path)?,
        });
    }
    Ok(copied)
}

/// Read the copy back: every file has the size and SHA-256 of its source.
pub(super) fn verify(
    dest: &Path,
    copied: &[Copied],
    progress: &dyn Fn(f64, &str),
    cancelled: &dyn Fn() -> bool,
) -> AppResult<()> {
    let wrong = |rel: &str| {
        AppError::new(
            "storage.copy_mismatch",
            "The copy didn't match the original, so nothing was moved. Your files are where they were.",
        )
        .with_detail(format!("changed: {rel}"))
    };
    for (i, file) in copied.iter().enumerate() {
        if cancelled() {
            return Err(AppError::cancelled());
        }
        let path = dest.join(openframe_security::validate_relative(&file.rel)?);
        if fs::metadata(&path)?.len() != file.size {
            return Err(wrong(&file.rel));
        }
        if !sha256_file(&path)?.eq_ignore_ascii_case(&file.sha256) {
            return Err(wrong(&file.rel));
        }
        progress(
            0.8 + 0.1 * ((i + 1) as f64 / copied.len().max(1) as f64),
            "Checking the copy",
        );
    }
    Ok(())
}

/// Remove the source after a verified move: the files that were copied, the top-level names in
/// `discard` (rebuildable leftovers such as a cache, deleted whole), then every folder that is
/// empty by now. Anything else stays: the top-level names in `kept` were deliberately not copied
/// (a vault's safety backups) and are only removed when they are already empty. A folder that
/// still holds something OpenFrame did not copy is left as it is. Returns true when the whole
/// source folder is gone.
pub(super) fn remove_source(root: &Path, tree: &Tree, discard: &[&str], kept: &[&str]) -> bool {
    for file in &tree.files {
        let _ = fs::remove_file(&file.path);
    }
    for name in discard {
        let path = root.join(name);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir()) {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
    for name in kept {
        // `remove_dir` only succeeds on an empty folder, so a safety backup is never deleted.
        let _ = fs::remove_dir(root.join(name));
    }
    // Deepest first; `remove_dir` only succeeds on an empty folder.
    let mut dirs: Vec<&String> = tree.dirs.iter().collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    for dir in dirs {
        clear_finder_files(&root.join(dir));
        let _ = fs::remove_dir(root.join(dir));
    }
    clear_finder_files(root);
    fs::remove_dir(root).is_ok()
}

fn rel_name(rel: &str) -> String {
    rel.rsplit('/').next().unwrap_or(rel).to_string()
}

/// `<folder name>.openframe-staging-<ms>` beside `dir`, a name that does not exist yet. Start
/// removes folders with this name (doc 40 section 5).
pub(super) fn aside_name(dir: &Path) -> PathBuf {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    let parent = dir.parent().unwrap_or(Path::new(""));
    let mut stamp = now_ms();
    loop {
        let candidate = parent.join(format!("{name}{STAGING_MARKER}{stamp}"));
        if !candidate.exists() {
            return candidate;
        }
        stamp += 1;
    }
}

/// Delete a folder that is not wanted any more (a verified copy whose switch was undone). It is
/// renamed to a staging name first, so a delete that stops half way never leaves a half folder
/// under the real name. Best effort: start removes what is left.
pub(super) fn discard(dir: &Path) {
    let aside = aside_name(dir);
    let target = if fs::rename(dir, &aside).is_ok() {
        aside
    } else {
        dir.to_path_buf()
    };
    let _ = fs::remove_dir_all(target);
}

/// Refuse a move that would run the destination drive out of space half way.
pub(super) fn ensure_space(dest_parent: &Path, needed: u64) -> AppResult<()> {
    let margin = 64 << 20;
    let Some(free) = openframe_platform::drives::free_bytes(dest_parent) else {
        return Ok(());
    };
    if free >= needed.saturating_add(margin) {
        return Ok(());
    }
    let mb = |b: u64| b.div_ceil(1 << 20);
    Err(AppError::new(
        "storage.not_enough_space",
        format!(
            "There isn't enough free space on that drive. The move needs about {} MB and {} MB is free. Free some space or choose another drive.",
            mb(needed),
            mb(free)
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_metadata_names_are_recognised() {
        assert!(is_finder_metadata(".DS_Store"));
        assert!(is_finder_metadata("._vault.sqlite"));
        assert!(!is_finder_metadata("vault.sqlite"));
        assert!(!is_finder_metadata(".hidden"));
        assert!(!is_finder_metadata("DS_Store"));
    }

    #[test]
    fn a_move_neither_copies_nor_counts_finder_files_and_still_removes_the_source() {
        let src = tempfile::tempdir().unwrap();
        let root = src.path().join("OpenFrame");
        fs::create_dir_all(root.join("Projects/Night Bus.openframe")).unwrap();
        fs::write(root.join("library.json"), b"{}").unwrap();
        fs::write(
            root.join("Projects/Night Bus.openframe/project.sqlite"),
            b"db",
        )
        .unwrap();
        for dir in ["", "Projects", "Projects/Night Bus.openframe"] {
            fs::write(root.join(dir).join(".DS_Store"), b"finder").unwrap();
        }
        fs::write(root.join("Projects/._Night Bus.openframe"), b"apple double").unwrap();

        let tree = walk(&root, &[]).unwrap();
        let rels: Vec<&str> = tree.files.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(
            rels,
            [
                "Projects/Night Bus.openframe/project.sqlite",
                "library.json"
            ]
        );
        assert_eq!(tree.bytes, 4);

        // Removing the source after a verified copy leaves no folder behind for Finder files.
        assert!(remove_source(&root, &tree, &[], &[]));
        assert!(!root.exists());
    }

    #[test]
    fn a_folder_with_only_finder_files_counts_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(is_empty_but_for_finder_files(dir.path()).unwrap());
        fs::write(dir.path().join(".DS_Store"), b"finder").unwrap();
        assert!(is_empty_but_for_finder_files(dir.path()).unwrap());
        fs::create_dir(dir.path().join("._sub")).unwrap();
        assert!(!is_empty_but_for_finder_files(dir.path()).unwrap());
        fs::remove_dir(dir.path().join("._sub")).unwrap();
        fs::write(dir.path().join("notes.txt"), b"mine").unwrap();
        assert!(!is_empty_but_for_finder_files(dir.path()).unwrap());
        clear_finder_files(dir.path());
        assert!(!dir.path().join(".DS_Store").exists());
        assert!(dir.path().join("notes.txt").exists());
    }
}
