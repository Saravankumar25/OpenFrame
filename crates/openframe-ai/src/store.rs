//! Application-level Offline AI store (Local AI Runtime spec §8). Shared by all
//! projects; projects never contain model copies.
//!
//! ```text
//! %LOCALAPPDATA%/OpenFrame/
//! ├─ models/
//! │  ├─ manifest-cache.json (+ .sig)   last verified manifest
//! │  ├─ active.json                    the active install (atomic pointer)
//! │  ├─ .downloads/<model-id>.gguf.partial (+ .json)   resumable downloads
//! │  ├─ .quarantine/                   rejected downloads (hash mismatch)
//! │  └─ <model-id>/{model.gguf, metadata.json, integrity.json}   chat + embedding model
//! ├─ runtimes/llama/<runtime-id>/      extracted, verified llama.cpp build
//! └─ logs/ai-runtime.log               sanitized sidecar log
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, now_ms};
use serde::{Deserialize, Serialize};

use crate::manifest::{Backend, ModelEntry, RuntimeEntry};

/// Format of `active.json` (format 1 was the retired single-model tier pointer).
pub const ACTIVE_SCHEMA: u32 = 2;

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
    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }
    pub fn downloads_dir(&self) -> PathBuf {
        self.models_dir().join(".downloads")
    }
    pub fn quarantine_dir(&self) -> PathBuf {
        self.models_dir().join(".quarantine")
    }
    pub fn manifest_cache(&self) -> PathBuf {
        self.models_dir().join("manifest-cache.json")
    }
    pub fn manifest_cache_sig(&self) -> PathBuf {
        self.models_dir().join("manifest-cache.json.sig")
    }
    pub fn active_file(&self) -> PathBuf {
        self.models_dir().join("active.json")
    }
    pub fn model_dir(&self, model_id: &str) -> PathBuf {
        self.models_dir().join(model_id)
    }
    pub fn model_file(&self, model_id: &str) -> PathBuf {
        self.model_dir(model_id).join("model.gguf")
    }
    pub fn model_partial(&self, model_id: &str) -> PathBuf {
        self.downloads_dir()
            .join(format!("{model_id}.gguf.partial"))
    }
    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes").join("llama")
    }
    pub fn runtime_dir(&self, runtime_id: &str) -> PathBuf {
        self.runtimes_dir().join(runtime_id)
    }
    pub fn runtime_partial(&self, runtime_id: &str) -> PathBuf {
        self.runtimes_dir()
            .join(".downloads")
            .join(format!("{runtime_id}.zip.partial"))
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn runtime_log(&self) -> PathBuf {
        self.logs_dir().join("ai-runtime.log")
    }
}

/// `metadata.json` next to an installed model: the manifest entry it was installed from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelMetadata {
    pub entry: ModelEntry,
    pub installed_at: i64,
    /// Set when the runtime could not start with this model; the model is kept
    /// (it passed verification) but is not activated.
    #[serde(default)]
    pub health_check_failed: bool,
}

/// `integrity.json` next to an installed model or runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityRecord {
    pub algorithm: String,
    pub sha256: String,
    pub bytes: u64,
    pub verified_at: i64,
}

/// `active.json`: the atomic pointer to the Offline AI install in use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveInstall {
    pub schema: u32,
    pub profile_id: String,
    pub profile_version: String,
    pub runtime_id: String,
    pub backend: Backend,
    pub chat_model_id: String,
    pub embedding_model_id: String,
    pub activated_at: i64,
}

/// Atomically replace a small JSON file (write temp + fsync + rename).
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| AppError::internal(e.to_string()))?;
    {
        use std::io::Write;
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Total size of the regular files below `dir` (0 when missing).
pub fn dir_size(dir: &Path) -> u64 {
    let Ok(rd) = fs::read_dir(dir) else { return 0 };
    rd.flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

impl AiPaths {
    /// The active install. A pointer in an older format (the retired tier system) reads as
    /// "not installed"; its files are cleaned up by the next install or by uninstall.
    pub fn active(&self) -> Option<ActiveInstall> {
        read_json::<ActiveInstall>(&self.active_file()).filter(|a| a.schema == ACTIVE_SCHEMA)
    }

    pub fn set_active(&self, sel: Option<&ActiveInstall>) -> AppResult<()> {
        match sel {
            Some(s) => write_json_atomic(&self.active_file(), s),
            None => {
                let p = self.active_file();
                if p.exists() {
                    fs::remove_file(p)?;
                }
                Ok(())
            }
        }
    }

    pub fn model_metadata(&self, model_id: &str) -> Option<ModelMetadata> {
        read_json(&self.model_dir(model_id).join("metadata.json"))
    }

    /// True when the model file exists with the recorded, expected integrity.
    /// (Cheap check: size + recorded hash; the full hash was verified at install.)
    pub fn model_installed(&self, entry: &ModelEntry) -> bool {
        let Some(integ) =
            read_json::<IntegrityRecord>(&self.model_dir(&entry.model_id).join("integrity.json"))
        else {
            return false;
        };
        integ.sha256 == entry.sha256
            && integ.bytes == entry.bytes
            && fs::metadata(self.model_file(&entry.model_id))
                .map(|m| m.len() == entry.bytes)
                .unwrap_or(false)
    }

    /// The manifest entry an installed model was verified against (independent of later
    /// manifests), when the file on disk still matches it.
    pub fn installed_model(&self, model_id: &str) -> Option<ModelEntry> {
        let md = self.model_metadata(model_id)?;
        (md.entry.model_id == model_id && self.model_installed(&md.entry)).then_some(md.entry)
    }

    pub fn runtime_executable(&self, entry: &RuntimeEntry) -> PathBuf {
        let mut p = self.runtime_dir(&entry.runtime_id);
        for part in entry.executable.split(['/', '\\']) {
            p.push(part);
        }
        p
    }

    pub fn runtime_installed(&self, entry: &RuntimeEntry) -> bool {
        let Some(integ) = read_json::<IntegrityRecord>(
            &self.runtime_dir(&entry.runtime_id).join("integrity.json"),
        ) else {
            return false;
        };
        integ.sha256 == entry.sha256 && self.runtime_executable(entry).is_file()
    }

    /// Record a verified model in its final directory (metadata + integrity).
    pub fn record_model(&self, entry: &ModelEntry) -> AppResult<()> {
        let dir = self.model_dir(&entry.model_id);
        write_json_atomic(
            &dir.join("integrity.json"),
            &IntegrityRecord {
                algorithm: "sha256".into(),
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                verified_at: now_ms(),
            },
        )?;
        write_json_atomic(
            &dir.join("metadata.json"),
            &ModelMetadata {
                entry: entry.clone(),
                installed_at: now_ms(),
                health_check_failed: false,
            },
        )
    }

    /// The runtime build recorded at install time (independent of later manifests).
    pub fn installed_runtime(&self, runtime_id: &str) -> Option<RuntimeEntry> {
        let entry: RuntimeEntry = read_json(&self.runtime_dir(runtime_id).join("runtime.json"))?;
        (entry.runtime_id == runtime_id && self.runtime_installed(&entry)).then_some(entry)
    }

    pub fn mark_health(&self, model_id: &str, failed: bool) -> AppResult<()> {
        if let Some(mut md) = self.model_metadata(model_id) {
            md.health_check_failed = failed;
            write_json_atomic(&self.model_dir(model_id).join("metadata.json"), &md)?;
        }
        Ok(())
    }

    /// Bytes used by installed Offline AI components (models, runtimes, partial downloads).
    pub fn installed_bytes(&self) -> u64 {
        dir_size(&self.models_dir()) + dir_size(&self.runtimes_dir())
    }

    /// Model and runtime folders (names only) currently on disk, including leftovers of older
    /// installs. Bookkeeping folders (`.downloads`, `.quarantine`) are excluded.
    fn component_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
        let Ok(rd) = fs::read_dir(dir) else {
            return Vec::new();
        };
        rd.flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .filter(|(name, _)| !name.starts_with('.'))
            .collect()
    }

    /// Delete every model/runtime folder the active install does not use: superseded versions,
    /// a graphics-card runtime that fell back to the processor build, and leftovers of the
    /// retired multi-profile system. Only called after a new install passed its health check
    /// (the previous working install is kept until then). Returns the bytes freed.
    pub fn remove_unreferenced(&self, active: &ActiveInstall) -> u64 {
        let mut freed = 0;
        for (name, path) in Self::component_dirs(&self.models_dir()) {
            if name != active.chat_model_id && name != active.embedding_model_id {
                let size = dir_size(&path);
                if fs::remove_dir_all(&path).is_ok() {
                    freed += size;
                }
            }
        }
        for (name, path) in Self::component_dirs(&self.runtimes_dir()) {
            if name != active.runtime_id {
                let size = dir_size(&path);
                if fs::remove_dir_all(&path).is_ok() {
                    freed += size;
                }
            }
        }
        freed
    }

    /// Remove every installed Offline AI component and partial download (uninstall). The
    /// verified manifest cache and the sanitized log are kept. Returns the bytes freed.
    pub fn remove_all_components(&self) -> AppResult<u64> {
        let mut freed = 0;
        self.set_active(None)?;
        for dir in [self.models_dir(), self.runtimes_dir()] {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let path = e.path();
                let Ok(ft) = e.file_type() else { continue };
                if ft.is_symlink() {
                    // A link (or Windows junction) is removed itself; its target is never
                    // entered. Directory links need remove_dir, file links remove_file.
                    fs::remove_dir(&path).or_else(|_| fs::remove_file(&path))?;
                } else if ft.is_dir() {
                    let size = dir_size(&path);
                    fs::remove_dir_all(&path)?;
                    freed += size;
                } else {
                    let name = e.file_name().to_string_lossy().into_owned();
                    // Keep the verified manifest cache; remove stale pointers/temp files.
                    if !name.starts_with("manifest-cache.json") {
                        freed += e.metadata().map(|m| m.len()).unwrap_or(0);
                        fs::remove_file(&path)?;
                    }
                }
            }
        }
        Ok(freed)
    }
}
