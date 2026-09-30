//! Application-level model store (Local AI Runtime spec §8). Shared by all
//! projects; projects never contain model copies.
//!
//! ```text
//! %LOCALAPPDATA%/OpenFrame/
//! ├─ models/
//! │  ├─ manifest-cache.json (+ .sig)   last verified manifest
//! │  ├─ active.json                    active profile + runtime (atomic pointer)
//! │  ├─ .downloads/<profile>.gguf.partial (+ .json)   resumable downloads
//! │  ├─ .quarantine/                   rejected downloads (hash mismatch)
//! │  └─ <profile-id>/{model.gguf, metadata.json, integrity.json}
//! ├─ runtimes/llama/<runtime-id>/      extracted, verified llama.cpp build
//! └─ logs/ai-runtime.log               sanitized sidecar log
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use openframe_domain::{AppError, AppResult, now_ms};
use serde::{Deserialize, Serialize};

use crate::manifest::{ModelEntry, RuntimeEntry, Tier};

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
    pub fn model_dir(&self, profile_id: &str) -> PathBuf {
        self.models_dir().join(profile_id)
    }
    pub fn model_file(&self, profile_id: &str) -> PathBuf {
        self.model_dir(profile_id).join("model.gguf")
    }
    pub fn model_partial(&self, profile_id: &str) -> PathBuf {
        self.downloads_dir()
            .join(format!("{profile_id}.gguf.partial"))
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

/// `metadata.json` next to an installed model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelMetadata {
    pub profile_id: String,
    pub tier: Tier,
    pub display_name: String,
    pub bytes: u64,
    pub license_id: String,
    pub source_url: String,
    pub installed_at: i64,
    pub context_tokens: u32,
    pub gpu_vram_bytes: u64,
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

/// `active.json`: the atomic pointer to the model/runtime in use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSelection {
    pub profile_id: String,
    pub runtime_id: String,
    pub activated_at: i64,
    pub previous_profile_id: Option<String>,
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

impl AiPaths {
    pub fn active(&self) -> Option<ActiveSelection> {
        read_json(&self.active_file())
    }

    pub fn set_active(&self, sel: Option<&ActiveSelection>) -> AppResult<()> {
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

    pub fn model_metadata(&self, profile_id: &str) -> Option<ModelMetadata> {
        read_json(&self.model_dir(profile_id).join("metadata.json"))
    }

    /// True when the model file exists with the recorded, expected integrity.
    /// (Cheap check: size + recorded hash; the full hash was verified at install.)
    pub fn model_installed(&self, entry: &ModelEntry) -> bool {
        let file = self.model_file(&entry.profile_id);
        let Some(integ) =
            read_json::<IntegrityRecord>(&self.model_dir(&entry.profile_id).join("integrity.json"))
        else {
            return false;
        };
        integ.sha256 == entry.sha256
            && integ.bytes == entry.bytes
            && fs::metadata(&file)
                .map(|m| m.len() == entry.bytes)
                .unwrap_or(false)
    }

    /// Profiles that have a model directory with metadata (whatever the manifest says now).
    pub fn installed_profiles(&self) -> Vec<(ModelMetadata, u64)> {
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(self.models_dir()) else {
            return out;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || !e.path().is_dir() {
                continue;
            }
            if let Some(md) = self.model_metadata(&name) {
                let size = fs::metadata(self.model_file(&name))
                    .map(|m| m.len())
                    .unwrap_or(0);
                if size > 0 {
                    out.push((md, size));
                }
            }
        }
        out.sort_by_key(|(m, _)| m.tier.rank());
        out
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
        let dir = self.model_dir(&entry.profile_id);
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
                profile_id: entry.profile_id.clone(),
                tier: entry.tier,
                display_name: entry.display_name.clone(),
                bytes: entry.bytes,
                license_id: entry.license_id.clone(),
                source_url: entry.url.clone(),
                installed_at: now_ms(),
                context_tokens: entry.context_tokens,
                gpu_vram_bytes: entry.gpu_vram_bytes,
                health_check_failed: false,
            },
        )
    }

    /// The runtime build recorded at install time (independent of later manifests).
    pub fn installed_runtime(&self, runtime_id: &str) -> Option<RuntimeEntry> {
        let entry: RuntimeEntry = read_json(&self.runtime_dir(runtime_id).join("runtime.json"))?;
        (entry.runtime_id == runtime_id && self.runtime_installed(&entry)).then_some(entry)
    }

    pub fn mark_health(&self, profile_id: &str, failed: bool) -> AppResult<()> {
        if let Some(mut md) = self.model_metadata(profile_id) {
            md.health_check_failed = failed;
            write_json_atomic(&self.model_dir(profile_id).join("metadata.json"), &md)?;
        }
        Ok(())
    }
}
