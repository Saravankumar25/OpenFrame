//! Writable Offline AI data in the user's app data folder. The engine is a read-only app resource
//! (`bundle.rs`); the downloaded models live in `models/` (`install.rs`, ADR-0026 decision 6).
//!
//! ```text
//! %LOCALAPPDATA%/OpenFrame/                  (macOS: ~/Library/Application Support/OpenFrame)
//! ├─ ai/bundle-check.json                    result of the last full check of the engine and models
//! ├─ models/                                 the downloaded model sets (install.rs)
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
    /// The graphics card setting and what the probes found (ADR-0029, `placement.rs`).
    pub fn graphics(&self) -> PathBuf {
        self.root.join("ai").join("graphics.json")
    }
    /// Engines launched by OpenFrame and not yet stopped (`engines::EngineRegistry`).
    pub fn engine_registry(&self) -> PathBuf {
        self.root.join("engines.json")
    }
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
