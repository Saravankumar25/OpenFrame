//! Offline AI façade used by the application layer (Local AI Runtime spec; agentic AI spec
//! §12, §23; ADR-0025): the bundled AI that ships with the app, its integrity check before the
//! first engine start, and the two managed engines (chat and embeddings).
//!
//! ```text
//! new ─► bundle present? (sizes only) ──no──► Missing / Damaged / Unsupported (AI off, plain message)
//!          │ yes
//!          ▼
//!      NotChecked ─first chat / embed / warm-up─► Checking (hashes, once per installed version)
//!                                                   ├─ ok ──► Verified ─► engines start on demand
//!                                                   └─ bad ─► Damaged (engines unconfigured)
//! ```
//!
//! Nothing is downloaded or installed at runtime, and a damaged installation is repaired by
//! reinstalling the app. The project is never touched by any of this.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;

use crate::bundle::{self, AssetLock, BundledAi, ModelEntry, ModelStamp, Pooling};
use crate::client::{self, EMBED_BATCH};
use crate::hardware::{self, HardwareInfo};
use crate::model::{ChatModel, LocalRuntimeModel};
use crate::rt::AiRuntime;
use crate::selection::{self, Fitness};
use crate::store::AiPaths;
use crate::supervisor::{
    EmbeddingServerLauncher, Launcher, LlamaServerLauncher, RuntimeStatus, StateListener,
    Supervisor, SupervisorConfig,
};

/// Characters of one input sent to the embedding model (≈ 300–400 English tokens; the model
/// window is 512). Retrieval chunks should be shorter; longer inputs are cut, and an input the
/// runtime still refuses (dense text) is retried shorter.
pub const MAX_EMBED_INPUT_CHARS: usize = 1_500;
/// Per-request timeout for one embedding batch.
const EMBED_TIMEOUT: Duration = Duration::from_secs(120);
/// Sampling seed of deterministic runs (the end-to-end suite).
pub const DETERMINISTIC_SEED: u32 = 42;

/// Errors of the Offline AI subsystem (the application-wide error type).
pub type AiError = AppError;

#[derive(Clone)]
pub struct ManagerConfig {
    /// Writable app data folder (check record, engine record, logs).
    pub app_data_dir: PathBuf,
    /// Read-only folder holding the bundled AI (`<app resources>/ai`). `None` when the platform
    /// cannot name it: AI then reports itself missing.
    pub bundle_dir: Option<PathBuf>,
    /// The lock the bundle is checked against (the one compiled into this build in the product).
    pub lock: AssetLock,
    /// Chat engine.
    pub supervisor: SupervisorConfig,
    /// Embedding engine.
    pub embedding_supervisor: SupervisorConfig,
    /// How long a request waits for an engine to get ready (model loading included).
    pub ready_timeout: Duration,
    /// Temperature 0 and a fixed seed (end-to-end tests only; `false` in the product).
    pub deterministic: bool,
}

impl ManagerConfig {
    /// Configuration for the running application.
    pub fn production(app_data_dir: &Path, bundle_dir: Option<PathBuf>) -> AppResult<Self> {
        let paths = AiPaths::new(app_data_dir);
        Ok(Self {
            app_data_dir: app_data_dir.to_path_buf(),
            bundle_dir,
            lock: AssetLock::embedded()?,
            supervisor: SupervisorConfig {
                log_path: Some(paths.runtime_log()),
                ..SupervisorConfig::default()
            },
            embedding_supervisor: SupervisorConfig {
                log_path: Some(paths.embedding_log()),
                ..SupervisorConfig::default()
            },
            ready_timeout: Duration::from_secs(600),
            deterministic: false,
        })
    }
}

/// Where the bundled AI stands on this installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BundleState {
    /// All files are present with the pinned sizes; the full check runs before the first start.
    NotChecked,
    /// The full check is running.
    Checking,
    /// Every file matches its pinned hash.
    Verified,
    /// Files are missing (a partial or broken installation).
    Missing,
    /// Files were changed after installation.
    Damaged,
    /// This build carries no engine for this kind of computer.
    Unsupported,
}

impl BundleState {
    /// AI can be used (possibly after the one-time check).
    pub fn usable(self) -> bool {
        matches!(
            self,
            BundleState::NotChecked | BundleState::Checking | BundleState::Verified
        )
    }
}

/// What `ai.status` needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineAiState {
    pub available: bool,
    pub bundle: BundleState,
    /// The plain-language reason when AI is not available.
    pub problem: Option<AppError>,
}

/// C1 contract (agentic AI spec §12): identity of the bundled embedding model. Part of the
/// semantic index metadata: a different model, version or hash means the index is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingInfo {
    pub model_id: String,
    pub version: String,
    pub sha256: String,
    pub dim: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    Runtime,
    ChatModel,
    EmbeddingModel,
}

/// Technical details for Settings > Offline AI (never the normal UI).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentDiagnostics {
    pub kind: ComponentKind,
    pub id: String,
    pub name: String,
    pub version: String,
    pub license_id: String,
    pub license_url: Option<String>,
    pub bytes: u64,
    /// SHA-256 of the model file, or of the server program for the engine.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub bundle_id: String,
    pub bundle_version: String,
    pub bundle: BundleState,
    pub problem: Option<String>,
    pub components: Vec<ComponentDiagnostics>,
    pub chat_runtime: RuntimeStatus,
    pub embedding_runtime: RuntimeStatus,
    pub hardware: HardwareInfo,
    pub fitness: Fitness,
    /// Read-only folder of the bundled AI.
    pub bundle_dir: String,
    pub bundle_bytes: u64,
    pub log_file: String,
}

pub struct AiManager {
    cfg: ManagerConfig,
    paths: AiPaths,
    rt: Arc<AiRuntime>,
    bundle: Option<BundledAi>,
    /// Chat and embedding model files as they were at the last successful check.
    stamps: Option<[Arc<ModelStamp>; 2]>,
    state: RwLock<(BundleState, Option<AppError>)>,
    check_lock: Mutex<()>,
    supervisor: Arc<Supervisor>,
    embed_supervisor: Arc<Supervisor>,
    hw_cache: Mutex<Option<HardwareInfo>>,
}

fn embedding_info(e: &ModelEntry) -> Option<EmbeddingInfo> {
    Some(EmbeddingInfo {
        model_id: e.model_id.clone(),
        version: e.version.clone(),
        sha256: e.file.sha256.clone(),
        dim: e.embedding_dim?,
    })
}

/// Cut an embedding input to `max_chars` on a character boundary; an empty input becomes a
/// single space (the server refuses empty strings).
pub fn prepare_embedding_input(s: &str, max_chars: usize) -> String {
    let t = s.trim();
    if t.is_empty() {
        return " ".into();
    }
    match t.char_indices().nth(max_chars) {
        Some((cut, _)) => t[..cut].to_string(),
        None => t.to_string(),
    }
}

fn state_of(e: &AppError) -> BundleState {
    match e.code_str() {
        "ai.unsupported" => BundleState::Unsupported,
        "ai.assets_missing" => BundleState::Missing,
        _ => BundleState::Damaged,
    }
}

impl AiManager {
    pub fn new(cfg: ManagerConfig) -> AppResult<Self> {
        let rt = Arc::new(AiRuntime::new()?);
        let paths = AiPaths::new(&cfg.app_data_dir);
        // Builds before ADR-0025 downloaded the runtime and models into app data; that copy is
        // never used again.
        let freed = paths.remove_retired_downloads();
        if freed > 0 {
            tracing::info!(
                freed,
                "removed Offline AI files left by the retired download flow"
            );
        }
        // Engines left running by a crashed session are stopped before any engine starts.
        let registry = Arc::new(crate::engines::EngineRegistry::new(paths.engine_registry()));
        registry.sweep();
        let supervisor = Arc::new(Supervisor::new(cfg.supervisor.clone(), rt.handle()));
        let embed_supervisor = Arc::new(Supervisor::new(
            cfg.embedding_supervisor.clone(),
            rt.handle(),
        ));
        supervisor.set_registry(registry.clone());
        embed_supervisor.set_registry(registry);
        let bundle = match &cfg.bundle_dir {
            Some(dir) => BundledAi::new(dir, cfg.lock.clone()),
            None => Err(bundle::missing().with_detail("the app resources folder is unknown")),
        };
        let (bundle, state) = match bundle.and_then(|b| b.check_present().map(|()| b)) {
            Ok(b) => (Some(b), (BundleState::NotChecked, None)),
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    detail = e.detail.as_deref().unwrap_or(""),
                    "the bundled Offline AI is not usable"
                );
                (None, (state_of(&e), Some(e)))
            }
        };
        let stamps = bundle.as_ref().map(|b| {
            let (chat, emb) = (cfg.lock.chat_model(), cfg.lock.embedding_model());
            [
                ModelStamp::new(b.model_file(chat), chat.file.bytes),
                ModelStamp::new(b.model_file(emb), emb.file.bytes),
            ]
        });
        let m = Self {
            cfg,
            paths,
            rt,
            bundle,
            stamps,
            state: RwLock::new(state),
            check_lock: Mutex::new(()),
            supervisor,
            embed_supervisor,
            hw_cache: Mutex::new(None),
        };
        m.configure();
        Ok(m)
    }

    pub fn paths(&self) -> &AiPaths {
        &self.paths
    }

    /// Chat engine state changes (the AI panel refreshes on them).
    pub fn set_state_listener(&self, l: StateListener) {
        self.supervisor.set_listener(l);
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        self.supervisor.status()
    }

    pub fn embedding_status(&self) -> RuntimeStatus {
        self.embed_supervisor.status()
    }

    /// Stop both engines (memory released; they start again on the next request).
    pub fn stop_runtime(&self) {
        self.supervisor.stop();
        self.embed_supervisor.stop();
    }

    pub fn hardware(&self) -> HardwareInfo {
        self.hw_cache
            .lock()
            .get_or_insert_with(hardware::inspect)
            .clone()
    }

    // ------------------------------------------------------------- bundle state

    pub fn state(&self) -> OfflineAiState {
        let (bundle, problem) = self.state.read().clone();
        OfflineAiState {
            available: bundle.usable(),
            bundle,
            problem,
        }
    }

    /// AI can be used (the bundled files are present and not known to be damaged).
    pub fn available(&self) -> bool {
        self.state.read().0.usable()
    }

    /// Why AI cannot be used right now, in plain language.
    pub fn unavailable_reason(&self) -> AppError {
        self.state.read().1.clone().unwrap_or_else(bundle::missing)
    }

    /// Check every bundled file against its pinned hash before the first engine start (once per
    /// installed version; later starts compare size and modification time with the recorded
    /// check). On a mismatch AI is switched off with a plain message and nothing else changes.
    pub fn ensure_verified(&self) -> AppResult<()> {
        {
            let s = self.state.read();
            match s.0 {
                BundleState::Verified => return Ok(()),
                BundleState::NotChecked | BundleState::Checking => {}
                _ => return Err(s.1.clone().unwrap_or_else(bundle::missing)),
            }
        }
        let _one_check = self.check_lock.lock();
        {
            let s = self.state.read();
            match s.0 {
                BundleState::Verified => return Ok(()),
                BundleState::NotChecked | BundleState::Checking => {}
                _ => return Err(s.1.clone().unwrap_or_else(bundle::missing)),
            }
        }
        let Some(b) = &self.bundle else {
            return Err(self.unavailable_reason());
        };
        *self.state.write() = (BundleState::Checking, None);
        let checked = b.verify(&self.paths.bundle_check()).and_then(|()| {
            self.stamps
                .iter()
                .flatten()
                .try_for_each(|s| s.mark_checked())
        });
        match checked {
            Ok(()) => {
                *self.state.write() = (BundleState::Verified, None);
                Ok(())
            }
            Err(e) => Err(self.switch_off(e)),
        }
    }

    /// A bundled file is missing or altered: AI stays off for this session with a plain message
    /// (the engines are unconfigured, so nothing can start them again until a reinstall).
    fn switch_off(&self, e: AppError) -> AppError {
        tracing::error!(
            code = e.code_str(),
            detail = e.detail.as_deref().unwrap_or(""),
            "the bundled Offline AI failed its check; AI stays off"
        );
        *self.state.write() = (state_of(&e), Some(e.clone()));
        self.configure();
        e
    }

    /// Engine start refused because its pinned files changed (checked before every launch).
    fn guard<T>(&self, r: AppResult<T>) -> AppResult<T> {
        match r {
            Err(e) if matches!(e.code_str(), "ai.assets_damaged" | "ai.assets_missing") => {
                Err(self.switch_off(e))
            }
            other => other,
        }
    }

    /// Check the bundle, then begin loading the chat model in the background.
    pub fn warm_up(&self) -> AppResult<()> {
        self.ensure_verified()?;
        let r = self.supervisor.start();
        self.guard(r)
    }

    /// The chat model when AI is available.
    pub fn chat_model(self: &Arc<Self>) -> Option<Arc<dyn ChatModel>> {
        if !self.available() {
            return None;
        }
        let reference = self.supervisor.status().model_reference?;
        Some(Arc::new(LocalRuntimeModel {
            manager: self.clone(),
            reference,
        }))
    }

    /// One chat completion on the chat engine (after the bundle check). Deterministic runs use
    /// temperature 0; the engine was started with a fixed seed.
    pub(crate) fn chat(&self, req: &client::ChatRequest) -> AppResult<String> {
        self.ensure_verified()?;
        let ep = self.guard(self.supervisor.ensure_ready(self.cfg.ready_timeout))?;
        let _busy = self.supervisor.begin_request();
        let http = self.supervisor.http().clone();
        let mut req = req.clone();
        if self.cfg.deterministic {
            req.temperature = 0.0;
        }
        self.rt
            .run(async move { client::chat(&http, &ep, &req).await })
    }

    // ------------------------------------------------------------ embeddings (C1)

    /// The bundled embedding model (None when AI is not available).
    pub fn embedding_info(&self) -> Option<EmbeddingInfo> {
        if !self.available() {
            return None;
        }
        embedding_info(self.cfg.lock.embedding_model())
    }

    /// Embed texts with the local embedding model. Blocking (call from a background thread);
    /// batches internally; one L2-normalised vector of `embedding_info().dim` values per input,
    /// in input order. Inputs are trimmed and bounded to [`MAX_EMBED_INPUT_CHARS`] characters.
    pub fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let info = self
            .embedding_info()
            .ok_or_else(|| self.unavailable_reason())?;
        self.ensure_verified()?;
        let dim = info.dim as usize;
        let ep = self.guard(self.embed_supervisor.ensure_ready(self.cfg.ready_timeout))?;
        let _busy = self.embed_supervisor.begin_request();
        let mut out = Vec::with_capacity(inputs.len());
        for chunk in inputs.chunks(EMBED_BATCH) {
            let batch: Vec<String> = chunk
                .iter()
                .map(|s| prepare_embedding_input(s, MAX_EMBED_INPUT_CHARS))
                .collect();
            match self.embed_batch(&ep, batch, dim) {
                Ok(vectors) => out.extend(vectors),
                // An input can still exceed the model window (dense or unusual text): retry the
                // batch one by one, shortening only the input that doesn't fit.
                Err(e) if e.code_str() == "ai.embedding_rejected" => {
                    for s in chunk {
                        out.push(self.embed_shrinking(&ep, s, dim)?);
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    fn embed_batch(
        &self,
        ep: &client::Endpoint,
        batch: Vec<String>,
        dim: usize,
    ) -> AppResult<Vec<Vec<f32>>> {
        let http = self.embed_supervisor.http().clone();
        let ep = ep.clone();
        self.rt
            .run(async move { client::embed(&http, &ep, &batch, dim, EMBED_TIMEOUT).await })
    }

    fn embed_shrinking(&self, ep: &client::Endpoint, s: &str, dim: usize) -> AppResult<Vec<f32>> {
        let mut limit = MAX_EMBED_INPUT_CHARS;
        loop {
            match self.embed_batch(ep, vec![prepare_embedding_input(s, limit)], dim) {
                Ok(mut v) => return Ok(v.remove(0)),
                Err(e) if e.code_str() == "ai.embedding_rejected" && limit > 100 => limit /= 2,
                Err(e) => return Err(e),
            }
        }
    }

    // ------------------------------------------------------------ launchers

    fn threads(hw: &HardwareInfo) -> usize {
        hw.physical_cpus.unwrap_or(hw.logical_cpus / 2).clamp(1, 16)
    }

    /// Point both engines at the bundle, or at nothing when AI is not usable.
    fn configure(&self) {
        let usable = self.available();
        match self.bundle.as_ref().filter(|_| usable) {
            Some(b) => {
                let hw = self.hardware();
                let chat = self.cfg.lock.chat_model();
                let emb = self.cfg.lock.embedding_model();
                let chat_launcher: Arc<dyn Launcher> = Arc::new(LlamaServerLauncher {
                    executable: b.executable(),
                    engine_files: Some(b.engine_files()),
                    model_path: b.model_file(chat),
                    profile_id: format!("{}:{}", bundle::BUNDLE_ID, chat.model_id),
                    context_tokens: chat.context_tokens.clamp(2048, 32768),
                    threads: Self::threads(&hw),
                    seed: self.cfg.deterministic.then_some(DETERMINISTIC_SEED),
                    model_stamp: self.stamps.as_ref().map(|s| s[0].clone()),
                });
                let embedding_launcher: Arc<dyn Launcher> = Arc::new(EmbeddingServerLauncher {
                    executable: b.executable(),
                    engine_files: Some(b.engine_files()),
                    model_path: b.model_file(emb),
                    model_id: emb.model_id.clone(),
                    pooling: emb.pooling.unwrap_or(Pooling::Cls),
                    context_tokens: emb.context_tokens,
                    // Leave most cores to the chat model when both run.
                    threads: (Self::threads(&hw) / 2).clamp(1, 4),
                    model_stamp: self.stamps.as_ref().map(|s| s[1].clone()),
                });
                self.supervisor.configure(Some(chat_launcher));
                self.embed_supervisor.configure(Some(embedding_launcher));
            }
            None => {
                self.supervisor.configure(None);
                self.embed_supervisor.configure(None);
            }
        }
    }

    // ----------------------------------------------------------- diagnostics

    /// Technical details for Settings > Offline AI.
    pub fn diagnostics(&self) -> Diagnostics {
        let lock = &self.cfg.lock;
        let st = self.state();
        let mut components = Vec::new();
        if let Some(r) = lock.runtime_for_this_computer() {
            components.push(ComponentDiagnostics {
                kind: ComponentKind::Runtime,
                id: r.runtime_id.clone(),
                name: format!("{} server (processor)", r.engine),
                version: r.version.clone(),
                license_id: r.license_id.clone(),
                license_url: None,
                bytes: r.files.iter().map(|f| f.bytes).sum(),
                sha256: r
                    .files
                    .iter()
                    .find(|f| f.path == r.executable)
                    .map(|f| f.sha256.clone())
                    .unwrap_or_default(),
            });
        }
        for (kind, m) in [
            (ComponentKind::ChatModel, lock.chat_model()),
            (ComponentKind::EmbeddingModel, lock.embedding_model()),
        ] {
            components.push(ComponentDiagnostics {
                kind,
                id: m.model_id.clone(),
                name: format!("{} ({})", m.display_name, m.quantization),
                version: m.version.clone(),
                license_id: m.license_id.clone(),
                license_url: Some(m.license_url.clone()),
                bytes: m.file.bytes,
                sha256: m.file.sha256.clone(),
            });
        }
        let hw = self.hardware();
        Diagnostics {
            bundle_id: lock.bundle_id.clone(),
            bundle_version: lock.bundle_version.clone(),
            bundle: st.bundle,
            problem: st.problem.map(|e| e.message),
            components,
            chat_runtime: self.supervisor.status(),
            embedding_runtime: self.embed_supervisor.status(),
            fitness: selection::assess(&hw, lock.chat_model()),
            hardware: hw,
            bundle_dir: self
                .cfg
                .bundle_dir
                .as_ref()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default(),
            bundle_bytes: self
                .bundle
                .as_ref()
                .map(BundledAi::total_bytes)
                .unwrap_or(0),
            log_file: self.paths.runtime_log().to_string_lossy().into_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_inputs_are_bounded_and_never_empty() {
        assert_eq!(prepare_embedding_input("  ", 10), " ");
        assert_eq!(prepare_embedding_input(" Ravi ", 10), "Ravi");
        let long = "é".repeat(MAX_EMBED_INPUT_CHARS + 50);
        assert_eq!(
            prepare_embedding_input(&long, MAX_EMBED_INPUT_CHARS)
                .chars()
                .count(),
            MAX_EMBED_INPUT_CHARS
        );
    }
}
