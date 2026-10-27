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

/// Everything under `root` except the top-level names in `skip_top` (matched without case).
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
            } else if kind.is_file() {
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
        let _ = fs::remove_dir(root.join(dir));
    }
    fs::remove_dir(root).is_ok()
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
