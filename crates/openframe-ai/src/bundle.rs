//! The Offline AI bundled with the application (ADR-0025).
//!
//! Every package carries the complete offline AI: the llama.cpp server and the libraries it
//! loads, the chat model and the embedding model (their tokenizers are embedded in the GGUF
//! files). The committed lock `packaging/ai/ai-assets.lock.json` pins every file by path, bytes
//! and SHA-256 with its licence id. The build (`scripts/prepare-ai-assets.mjs`) refuses files that
//! do not match it, and the same lock is compiled into this crate, so the app checks the
//! installed files against hashes that cannot be changed next to them.
//!
//! ```text
//! <app resources>/ai/                     read-only, installed and replaced with the app
//! ├─ runtimes/<runtime-id>/<file>          engine for this platform (re-hashed before every launch)
//! ├─ models/<model-id>/model.gguf          chat model and embedding model
//! └─ notices/                              licence notices
//! <app data>/ai/bundle-check.json          writable: result of the last full check
//! ```
//!
//! Nothing is ever downloaded. When a file is missing or altered, AI does not start and the user
//! is told to reinstall; the project is never touched.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use openframe_domain::{AppError, AppResult, now_ms};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::engines::{EngineFiles, hash_file};

/// The lock compiled into this build (the same file the build verified the package against).
pub const LOCK_JSON: &str = include_str!("../../../packaging/ai/ai-assets.lock.json");
/// Folder of the app resources that holds the bundled AI (`scripts/prepare-ai-assets.mjs`).
pub const RESOURCE_FOLDER: &str = "ai";
/// The single production Offline AI bundle (internal id; never shown in the normal UI).
pub const BUNDLE_ID: &str = "openframe-local-ai-v1";
/// Format of the lock this build understands.
pub const LOCK_VERSION: u32 = 1;
/// Upper bound for the number of pinned files (bounded checks).
const MAX_FILES: usize = 200;

/// Shown when bundled files are missing (a damaged or partial installation).
pub fn missing() -> AppError {
    AppError::ai(
        "assets_missing",
        "Offline AI is missing from this installation, so it can't start. Reinstall OpenFrame Studio to repair it. Your projects were not changed.",
    )
}

/// Shown when bundled files were changed after installation.
pub fn damaged() -> AppError {
    AppError::ai(
        "assets_damaged",
        "Offline AI files in this installation were changed or damaged, so OpenFrame did not start them. Reinstall OpenFrame Studio to repair it. Your projects were not changed.",
    )
}

/// Shown on a platform this build carries no engine for.
pub fn unsupported() -> AppError {
    AppError::ai(
        "unsupported",
        "Offline AI isn't available for this kind of computer yet. Everything else in OpenFrame works normally.",
    )
}

fn invalid_lock(detail: impl Into<String>) -> AppError {
    AppError::internal("the Offline AI lock compiled into this build is invalid")
        .with_detail(detail)
}

/// One pinned file: a single path component (or `a/b` inside the engine folder), size and hash.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PinnedFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// Where a pinned file came from (provenance for diagnostics and notices; never fetched).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub url: String,
    #[serde(default)]
    pub bytes: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeEntry {
    pub runtime_id: String,
    pub engine: String,
    pub version: String,
    pub os: String,
    pub arch: String,
    /// Only the processor build is bundled ("cpu").
    pub backend: String,
    /// The server program inside the runtime folder.
    pub executable: String,
    pub license_id: String,
    pub source: Source,
    pub files: Vec<PinnedFile>,
}

/// What a model file is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelRole {
    /// The instruction-tuned language model that plans, answers and drafts.
    Chat,
    /// The small retrieval model that turns text into vectors for semantic search.
    Embedding,
}

/// Embedding pooling (llama.cpp `--pooling`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Pooling {
    Cls,
    Mean,
    Last,
}

impl Pooling {
    pub fn as_arg(self) -> &'static str {
        match self {
            Pooling::Cls => "cls",
            Pooling::Mean => "mean",
            Pooling::Last => "last",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelEntry {
    /// Versioned, unique id; also the folder name (`models/<model_id>`).
    pub model_id: String,
    pub role: ModelRole,
    /// Technical name for diagnostics, About and licence notices only (never the normal UI).
    pub display_name: String,
    pub family: String,
    pub quantization: String,
    pub version: String,
    pub license_id: String,
    pub license_url: String,
    /// Context window the runtime is started with.
    pub context_tokens: u32,
    /// Below this much memory Offline AI is not expected to work.
    pub min_ram_bytes: u64,
    /// Memory for comfortable processor inference.
    pub recommended_ram_bytes: u64,
    pub layers: u32,
    /// Vector size (embedding models only).
    #[serde(default)]
    pub embedding_dim: Option<u32>,
    /// Pooling (embedding models only).
    #[serde(default)]
    pub pooling: Option<Pooling>,
    /// "embedded": the tokenizer is part of the GGUF file.
    pub tokenizer: String,
    pub source: Source,
    pub file: PinnedFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetLock {
    pub lock_version: u32,
    pub bundle_id: String,
    /// Raised whenever any bundled file changes.
    pub bundle_version: String,
    pub runtimes: Vec<RuntimeEntry>,
    pub models: Vec<ModelEntry>,
}

/// Ids become folder names, so they must be one portable, non-reserved path component.
pub fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
        && openframe_security::sanitize_file_name(id) == id
}

/// A pinned file name: one path component made of letters, digits, `.`, `-` and `_`.
fn safe_file(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !name.starts_with('.')
        && !name.contains("..")
        && openframe_security::sanitize_file_name(name) == name
}

fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn valid_text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

fn check_file(f: &PinnedFile, owner: &str) -> AppResult<()> {
    if !safe_file(&f.path) || f.bytes == 0 || !valid_sha(&f.sha256) {
        return Err(invalid_lock(format!(
            "{owner}: file entry {} is invalid",
            f.path
        )));
    }
    Ok(())
}

impl AssetLock {
    /// The lock compiled into this build.
    pub fn embedded() -> AppResult<AssetLock> {
        Self::parse(LOCK_JSON)
    }

    pub fn parse(json: &str) -> AppResult<AssetLock> {
        let lock: AssetLock =
            serde_json::from_str(json).map_err(|e| invalid_lock(format!("parse: {e}")))?;
        lock.validate()?;
        Ok(lock)
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.lock_version != LOCK_VERSION {
            return Err(invalid_lock(format!("lock version {}", self.lock_version)));
        }
        if self.bundle_id != BUNDLE_ID || !safe_id(&self.bundle_version) {
            return Err(invalid_lock("unknown bundle"));
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.runtimes {
            if !safe_id(&r.runtime_id) || !ids.insert(r.runtime_id.clone()) {
                return Err(invalid_lock(format!("runtime id {}", r.runtime_id)));
            }
            if !matches!(r.backend.as_str(), "cpu" | "vulkan")
                || !safe_id(&r.version)
                || !valid_text(&r.license_id, 80)
                || r.files.is_empty()
                || r.files.len() > MAX_FILES
            {
                return Err(invalid_lock(format!("runtime {} is invalid", r.runtime_id)));
            }
            let mut names = std::collections::HashSet::new();
            for f in &r.files {
                check_file(f, &r.runtime_id)?;
                if !names.insert(f.path.to_ascii_lowercase()) {
                    return Err(invalid_lock(format!("duplicate file {}", f.path)));
                }
            }
            if !names.contains(&r.executable.to_ascii_lowercase()) {
                return Err(invalid_lock(format!(
                    "runtime {}: the server program is not pinned",
                    r.runtime_id
                )));
            }
        }
        // Each platform has exactly one processor build; a Vulkan build beside it is optional
        // (ADR-0029 decision 1). The processor build stays the fallback.
        for r in &self.runtimes {
            let same = |b: &str| {
                self.runtimes
                    .iter()
                    .filter(|o| o.os == r.os && o.arch == r.arch && o.backend == b)
                    .count()
            };
            if same("cpu") != 1 || same("vulkan") > 1 {
                return Err(invalid_lock(format!(
                    "{}-{} needs one processor build and at most one Vulkan build",
                    r.os, r.arch
                )));
            }
        }
        let (mut chat, mut emb) = (0, 0);
        for m in &self.models {
            if !safe_id(&m.model_id) || !ids.insert(m.model_id.clone()) {
                return Err(invalid_lock(format!("model id {}", m.model_id)));
            }
            check_file(&m.file, &m.model_id)?;
            if m.tokenizer != "embedded"
                || !safe_id(&m.version)
                || !valid_text(&m.display_name, 120)
                || !valid_text(&m.license_id, 80)
                || !m.license_url.starts_with("https://")
            {
                return Err(invalid_lock(format!("model {} is invalid", m.model_id)));
            }
            match m.role {
                ModelRole::Chat => {
                    chat += 1;
                    if m.context_tokens < 2048 || m.embedding_dim.is_some() {
                        return Err(invalid_lock(format!("chat model {}", m.model_id)));
                    }
                }
                ModelRole::Embedding => {
                    emb += 1;
                    let dim_ok = m.embedding_dim.is_some_and(|d| (8..=4096).contains(&d));
                    if !dim_ok || m.pooling.is_none() || m.context_tokens < 128 {
                        return Err(invalid_lock(format!("embedding model {}", m.model_id)));
                    }
                }
            }
        }
        if chat != 1 || emb != 1 {
            return Err(invalid_lock(
                "exactly one chat model and one embedding model",
            ));
        }
        Ok(())
    }

    pub fn chat_model(&self) -> &ModelEntry {
        self.models
            .iter()
            .find(|m| m.role == ModelRole::Chat)
            .expect("validated: one chat model")
    }

    pub fn embedding_model(&self) -> &ModelEntry {
        self.models
            .iter()
            .find(|m| m.role == ModelRole::Embedding)
            .expect("validated: one embedding model")
    }

    /// The engine pinned for this computer's operating system and processor.
    pub fn runtime_for_this_computer(&self) -> Option<&RuntimeEntry> {
        let os = openframe_platform::current_os().name();
        let arch = std::env::consts::ARCH;
        self.runtimes
            .iter()
            .find(|r| r.os == os && r.arch == arch && r.backend == "cpu")
    }

    /// The Vulkan build pinned for this computer, when there is one (ADR-0029).
    pub fn gpu_runtime_for_this_computer(&self) -> Option<&RuntimeEntry> {
        let os = openframe_platform::current_os().name();
        let arch = std::env::consts::ARCH;
        self.runtimes
            .iter()
            .find(|r| r.os == os && r.arch == arch && r.backend == "vulkan")
    }

    /// Identifies this exact lock (a different app version with other files gets a new one).
    pub fn fingerprint(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }
}

/// A model file as it was when the bundle check passed: compared (size, modification time, still a
/// regular file) before every engine launch, so a model replaced after the check, for example
/// while the engine was unloaded after inactivity, is never loaded without a new full check.
#[derive(Debug)]
pub struct ModelStamp {
    path: PathBuf,
    bytes: u64,
    checked: parking_lot::Mutex<Option<u128>>,
}

impl ModelStamp {
    pub fn new(path: PathBuf, bytes: u64) -> std::sync::Arc<ModelStamp> {
        std::sync::Arc::new(ModelStamp {
            path,
            bytes,
            checked: parking_lot::Mutex::new(None),
        })
    }

    fn current(&self) -> AppResult<u128> {
        let m = std::fs::symlink_metadata(&self.path)
            .map_err(|_| missing().with_detail(format!("{} not found", self.path.display())))?;
        if !m.file_type().is_file() || m.len() != self.bytes {
            return Err(damaged().with_detail(format!("{} changed", self.path.display())));
        }
        Ok(modified_ms(&m))
    }

    /// Record the file's state right after a successful full check.
    pub fn mark_checked(&self) -> AppResult<()> {
        let now = self.current()?;
        *self.checked.lock() = Some(now);
        Ok(())
    }

    /// The file is still the one that was checked.
    pub fn verify(&self) -> AppResult<()> {
        let Some(at_check) = *self.checked.lock() else {
            return Err(damaged().with_detail("model not checked before launch"));
        };
        if self.current()? != at_check {
            return Err(
                damaged().with_detail(format!("{} changed after the check", self.path.display()))
            );
        }
        Ok(())
    }
}

/// One file of the bundle to check: where it is and what the lock says about it.
#[derive(Debug, Clone)]
struct Pinned {
    rel: String,
    path: PathBuf,
    bytes: u64,
    sha256: String,
}

/// The bundled AI of this installation: the read-only resource folder plus the lock.
#[derive(Debug, Clone)]
pub struct BundledAi {
    root: PathBuf,
    lock: AssetLock,
    runtime: RuntimeEntry,
}

/// `bundle-check.json` in the writable app data folder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CheckRecord {
    version: u32,
    lock_fingerprint: String,
    root: String,
    files: Vec<CheckedFile>,
    verified_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct CheckedFile {
    rel: String,
    bytes: u64,
    modified_ms: u128,
    sha256: String,
}

fn modified_ms(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

impl BundledAi {
    /// The bundle at `root` for this computer. `Err(unsupported)` when the lock pins no engine
    /// for this platform.
    pub fn new(root: &Path, lock: AssetLock) -> AppResult<BundledAi> {
        let runtime = lock
            .runtime_for_this_computer()
            .cloned()
            .ok_or_else(unsupported)?;
        Ok(BundledAi {
            root: root.to_path_buf(),
            lock,
            runtime,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn lock(&self) -> &AssetLock {
        &self.lock
    }
    pub fn runtime(&self) -> &RuntimeEntry {
        &self.runtime
    }
    pub fn runtime_dir(&self) -> PathBuf {
        self.root.join("runtimes").join(&self.runtime.runtime_id)
    }
    pub fn executable(&self) -> PathBuf {
        self.runtime_dir().join(&self.runtime.executable)
    }
    pub fn model_dir(&self, m: &ModelEntry) -> PathBuf {
        self.root.join("models").join(&m.model_id)
    }
    pub fn model_file(&self, m: &ModelEntry) -> PathBuf {
        self.model_dir(m).join(&m.file.path)
    }
    /// The engine files with their pinned hashes (checked again before every launch).
    pub fn engine_files(&self) -> EngineFiles {
        EngineFiles::pinned(&self.runtime_dir(), &self.runtime.files)
    }
    /// Bytes of every bundled file for this platform.
    pub fn total_bytes(&self) -> u64 {
        self.pinned().iter().map(|p| p.bytes).sum()
    }

    fn pinned(&self) -> Vec<Pinned> {
        let mut out: Vec<Pinned> = self
            .runtime
            .files
            .iter()
            .map(|f| Pinned {
                rel: format!("runtimes/{}/{}", self.runtime.runtime_id, f.path),
                path: self.runtime_dir().join(&f.path),
                bytes: f.bytes,
                sha256: f.sha256.clone(),
            })
            .collect();
        for m in &self.lock.models {
            out.push(Pinned {
                rel: format!("models/{}/{}", m.model_id, m.file.path),
                path: self.model_file(m),
                bytes: m.file.bytes,
                sha256: m.file.sha256.clone(),
            });
        }
        out
    }

    /// Folders between the root and the pinned files. None of them may be a link or junction,
    /// so a pinned path can never lead out of the installed resources.
    fn folders(&self) -> Vec<PathBuf> {
        let mut v = vec![
            self.root.clone(),
            self.root.join("runtimes"),
            self.runtime_dir(),
            self.root.join("models"),
        ];
        v.extend(self.lock.models.iter().map(|m| self.model_dir(m)));
        v
    }

    /// Cheap check (no hashing): every folder is a real folder and every pinned file is a
    /// regular file of the pinned size. `Err(missing)` otherwise.
    pub fn check_present(&self) -> AppResult<()> {
        for dir in self.folders() {
            match std::fs::symlink_metadata(&dir) {
                Ok(m) if m.file_type().is_dir() => {}
                Ok(_) => {
                    return Err(damaged().with_detail(format!("{} is not a folder", dir.display())));
                }
                Err(_) => return Err(missing().with_detail(format!("{} not found", dir.display()))),
            }
        }
        for p in self.pinned() {
            match std::fs::symlink_metadata(&p.path) {
                Ok(m) if m.file_type().is_file() => {
                    if m.len() != p.bytes {
                        return Err(damaged().with_detail(format!("{} has the wrong size", p.rel)));
                    }
                }
                Ok(_) => {
                    return Err(damaged().with_detail(format!("{} is not a regular file", p.rel)));
                }
                Err(_) => return Err(missing().with_detail(format!("{} not found", p.rel))),
            }
        }
        Ok(())
    }

    /// Full check against the pinned hashes. A file whose size and modification time match the
    /// last successful check recorded in `record` (same lock, same folder) is not hashed again,
    /// so the full read happens once per installed version. The engine files are re-hashed
    /// before every launch regardless ([`EngineFiles::verify`]).
    pub fn verify(&self, record: &Path) -> AppResult<()> {
        self.check_present()?;
        let fingerprint = self.lock.fingerprint();
        let root = self.root.to_string_lossy().into_owned();
        let previous: Option<CheckRecord> =
            crate::store::read_json(record).filter(|r: &CheckRecord| {
                r.version == 1 && r.lock_fingerprint == fingerprint && r.root == root
            });
        let mut checked = Vec::new();
        for p in self.pinned() {
            let meta = std::fs::symlink_metadata(&p.path).map_err(|_| missing())?;
            let modified = modified_ms(&meta);
            let cached = previous.as_ref().is_some_and(|r| {
                r.files.iter().any(|c| {
                    c.rel == p.rel
                        && c.bytes == meta.len()
                        && c.modified_ms == modified
                        && c.sha256 == p.sha256
                })
            });
            if !cached {
                let (bytes, sha) = hash_file(&p.path)
                    .map_err(|_| missing().with_detail(format!("{} unreadable", p.rel)))?;
                if bytes != p.bytes || sha != p.sha256 {
                    let _ = std::fs::remove_file(record);
                    return Err(
                        damaged().with_detail(format!("{} does not match its pinned hash", p.rel))
                    );
                }
            }
            checked.push(CheckedFile {
                rel: p.rel,
                bytes: meta.len(),
                modified_ms: modified,
                sha256: p.sha256,
            });
        }
        let rec = CheckRecord {
            version: 1,
            lock_fingerprint: fingerprint,
            root,
            files: checked,
            verified_at: now_ms(),
        };
        if previous.as_ref().map(|p| &p.files) != Some(&rec.files)
            && let Err(e) = crate::store::write_json_atomic(record, &rec)
        {
            // The check passed; only the shortcut for the next start is lost.
            tracing::warn!(code = e.code_str(), "could not record the Offline AI check");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiled_lock_describes_one_complete_bundle() {
        let lock = AssetLock::embedded().expect("the committed lock is valid");
        assert_eq!(lock.bundle_id, BUNDLE_ID);
        assert_eq!(lock.chat_model().family, "gemma3");
        assert_eq!(lock.embedding_model().embedding_dim, Some(384));
        assert!(lock.models.iter().all(|m| m.tokenizer == "embedded"));
        // Windows x64 is pinned today; every pinned engine names its server program.
        assert!(
            lock.runtimes
                .iter()
                .any(|r| r.os == "windows" && r.arch == "x86_64")
        );
        if cfg!(all(windows, target_arch = "x86_64")) {
            let rt = lock.runtime_for_this_computer().unwrap();
            assert!(rt.files.iter().any(|f| f.path == "llama-server.exe"));
            // Only the server and what it loads: no other llama.cpp programs, no RPC backend.
            for f in &rt.files {
                assert!(
                    !f.path.ends_with(".exe") || f.path == "llama-server.exe",
                    "{} must not ship",
                    f.path
                );
                assert!(!f.path.contains("rpc"), "{} must not ship", f.path);
            }
        }
    }

    #[test]
    fn a_lock_with_unsafe_entries_is_refused() {
        let good = AssetLock::embedded().unwrap();
        type Mutation = Box<dyn Fn(&mut AssetLock)>;
        let cases: Vec<(&str, Mutation)> = vec![
            (
                "path traversal",
                Box::new(|l| l.runtimes[0].files[0].path = "../evil.dll".into()),
            ),
            (
                "nested path",
                Box::new(|l| l.models[0].file.path = "a/b.gguf".into()),
            ),
            (
                "bad hash",
                Box::new(|l| l.models[0].file.sha256 = "AB".into()),
            ),
            ("empty file", Box::new(|l| l.models[0].file.bytes = 0)),
            (
                "device name",
                Box::new(|l| l.models[0].model_id = "con".into()),
            ),
            (
                "unpinned server",
                Box::new(|l| l.runtimes[0].executable = "other.exe".into()),
            ),
            (
                "graphics build",
                Box::new(|l| l.runtimes[0].backend = "vulkan".into()),
            ),
            (
                "separate tokenizer",
                Box::new(|l| l.models[0].tokenizer = "tokenizer.json".into()),
            ),
            (
                "two chat models",
                Box::new(|l| {
                    let m = l.models[0].clone();
                    l.models.push(ModelEntry {
                        model_id: "second".into(),
                        ..m
                    });
                }),
            ),
            (
                "other bundle",
                Box::new(|l| l.bundle_id = "qwen3-recommended".into()),
            ),
        ];
        for (name, mutate) in cases {
            let mut l = good.clone();
            mutate(&mut l);
            assert!(l.validate().is_err(), "{name} must be refused");
        }
    }

    #[test]
    fn ids_are_single_safe_path_components() {
        for bad in [
            "con", "nul", "com1", "aux.gguf", "a..b", "x.", ".x", "", "C:", "a/b",
        ] {
            assert!(!safe_id(bad), "{bad} must be rejected");
        }
        assert!(safe_id("gemma-3-1b-it-q8-0-v1"));
    }
}
