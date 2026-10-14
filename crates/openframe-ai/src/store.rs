//! Writable Offline AI data in the user's app data folder (ADR-0025 decision 3). The runtime and
//! the models are read-only app resources (`bundle.rs`); only small records and logs live here.
//!
//! ```text
//! %LOCALAPPDATA%/OpenFrame/                  (macOS: ~/Library/Application Support/OpenFrame)
//! ├─ ai/bundle-check.json                    result of the last full check of the bundled files
//! ├─ engines.json                            engines launched and not yet stopped
//! └─ logs/ai-runtime.log, ai-embedding.log   sanitized engine logs
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AiPaths {
    root: PathBuf,
}

impl AiPaths {
    /// `app_data_dir` = `%LOCALAPPDATA%/OpenFrame` (or an isolated test root).
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            root: app_data_dir.to_path_buf(),
        }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Result of the last full check of the bundled AI files.
    pub fn bundle_check(&self) -> PathBuf {
        self.root.join("ai").join("bundle-check.json")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn runtime_log(&self) -> PathBuf {
        self.logs_dir().join("ai-runtime.log")
    }
    pub fn embedding_log(&self) -> PathBuf {
        self.logs_dir().join("ai-embedding.log")
    }
    /// Engines launched by OpenFrame and not yet stopped (`engines::EngineRegistry`).
    pub fn engine_registry(&self) -> PathBuf {
        self.root.join("engines.json")
    }

    /// Folders where builds before ADR-0025 kept downloaded runtimes and models.
    fn retired_download_dirs(&self) -> [PathBuf; 2] {
        [self.root.join("models"), self.root.join("runtimes")]
    }

    /// Remove what the retired download flow left in the app data folder (up to about 1.1 GB of
    /// runtime and model copies, partial downloads and the old manifest cache). Links and
    /// junctions are removed themselves and never followed, so nothing outside these two folders
    /// is touched. Returns the bytes freed.
    pub fn remove_retired_downloads(&self) -> u64 {
        let mut freed = 0;
        for dir in self.retired_download_dirs() {
            let Ok(meta) = fs::symlink_metadata(&dir) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                let _ = fs::remove_dir(&dir).or_else(|_| fs::remove_file(&dir));
                continue;
            }
            if !meta.is_dir() {
                continue;
            }
            freed += remove_tree(&dir);
        }
        freed
    }
}

/// Delete a folder tree without following links. Returns the bytes of the regular files removed.
fn remove_tree(dir: &Path) -> u64 {
    let mut freed = 0;
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let path = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                let _ = fs::remove_dir(&path).or_else(|_| fs::remove_file(&path));
            } else if ft.is_dir() {
                freed += remove_tree(&path);
            } else {
                let len = e.metadata().map(|m| m.len()).unwrap_or(0);
                if fs::remove_file(&path).is_ok() {
                    freed += len;
                }
            }
        }
    }
    let _ = fs::remove_dir(dir);
    freed
}

/// Atomically replace a small JSON file through the shared atomic replace
/// (`openframe_platform::atomic`: unique temp in the same folder, fsync, rename, cleanup).
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| AppError::internal(e.to_string()))?;
    openframe_platform::atomic::replace_bytes(path, &bytes)?;
    Ok(())
}

/// Read a small JSON record (bounded: records over 1 MB are ignored).
pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    if fs::metadata(path).ok()?.len() > 1 << 20 {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}
