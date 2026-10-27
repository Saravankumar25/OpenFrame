//! Offline AI façade used by the application layer (Local AI Runtime spec; agentic AI spec
//! §12, §23; ADR-0026): the engine that ships with the app, the models downloaded after install,
//! their integrity check before the first engine start, and the two managed engines (chat and
//! embeddings).
//!
//! ```text
//! new / reload ─► engine and model set present? (sizes only)
//!                 ├─ no set ─► NotInstalled (AI off, "Download OpenFrame AI")
//!                 ├─ broken ─► Missing / Damaged / Unsupported (AI off, plain message)
//!                 └─ yes ───► NotChecked ─first chat / embed / warm-up─► Checking (hashes, once)
//!                                                   ├─ ok ──► Verified ─► engines start on demand
//!                                                   └─ bad ─► Damaged (engines unconfigured)
//! ```
//!
//! The download runs only from `start_download` (the person's click; `model_download.rs`). A
//! damaged engine is repaired by reinstalling the app, a damaged model set by downloading it again.
//! The project is never touched by any of this.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use openframe_domain::{AppError, AppResult};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;

use crate::bundle::{self, AssetLock, BundledAi, ModelRole, ModelStamp, Pooling};
use crate::client::{self, EMBED_BATCH};
use crate::download;
use crate::fetch::ModelSource;
use crate::gpu::{self, CpuReason, GraphicsRecord, ProbeRecord, Remembered};
use crate::hardware::{self, HardwareInfo};
use crate::model::{InferenceProvider, LocalRuntimeModel};
use crate::model_download::{self, DownloadStatus, ModelDownload};
use crate::placement::{self, GraphicsFacts, Placement};
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
/// Memory left free, beyond the chat model and its cache, for the engine to be started before the
/// first question (the app, the embedding engine and the person's other programs).
const EARLY_START_HEADROOM_MB: u64 = 1536;
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
    /// Read-only folder holding the engine (`<app resources>/ai`). `None` when the platform
    /// cannot name it: AI then reports itself missing.
    pub bundle_dir: Option<PathBuf>,
    /// Where the models come from: Hugging Face, or the test host of a test build (ADR-0032).
    /// `None` only when a test build was made with an invalid test host: it cannot download.
    pub model_source: Option<ModelSource>,
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
    /// This computer's graphics adapters and Vulkan loader (the platform layer in the product).
    pub graphics: Arc<dyn Fn() -> GraphicsFacts + Send + Sync>,
}

impl ManagerConfig {
    /// Configuration for the running application.
    pub fn production(
        app_data_dir: &Path,
        bundle_dir: Option<PathBuf>,
        model_source: Option<ModelSource>,
    ) -> AppResult<Self> {
        let paths = AiPaths::new(app_data_dir);
        Ok(Self {
            app_data_dir: app_data_dir.to_path_buf(),
            bundle_dir,
            model_source,
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
            graphics: Arc::new(placement::detect),
        })
    }
}

/// Where the Offline AI stands on this installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BundleState {
    /// No model set has been downloaded (or it was removed).
    NotInstalled,
    /// All files are present with the pinned sizes; the full check runs before the first start.
    NotChecked,
    /// The full check is running.
    Checking,
    /// Every file matches its pinned hash.
    Verified,
    /// Engine files are missing (a partial or broken installation).
    Missing,
    /// Engine or model files were changed, or a model file is missing.
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
    /// Where the chat engine runs, in one plain sentence.
    pub placement: String,
    /// Settings, Offline AI, Graphics card is Off.
    pub graphics_off: bool,
}

/// The engine and model set as last read from disk.
struct Loaded {
    bundle: Option<BundledAi>,
    /// The model files (chat, English search, multilingual search) as they were at the last
    /// successful check.
    stamps: Option<Vec<(ModelRole, Arc<ModelStamp>)>>,
}

pub struct AiManager {
    cfg: ManagerConfig,
    paths: AiPaths,
    rt: Arc<AiRuntime>,
    loaded: RwLock<Loaded>,
    download: ModelDownload,
    state: RwLock<(BundleState, Option<AppError>)>,
    check_lock: Mutex<()>,
    supervisor: Arc<Supervisor>,
    embed_supervisor: Arc<Supervisor>,
    hw_cache: Mutex<Option<HardwareInfo>>,
    /// Where the chat engine runs this session (ADR-0029), and one decision at a time.
    placement: Mutex<Placement>,
    placement_lock: Mutex<()>,
    /// Which search model the embedding engine runs: `Embedding` (English) or
    /// `MultilingualEmbedding`. Chosen by the search index for each project.
    search_model: Mutex<ModelRole>,
    /// Engine numbers of the chat calls since the last `take_usage` (the AI trace).
    usage: Mutex<client::Usage>,
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
        "ai.not_installed" => BundleState::NotInstalled,
        _ => BundleState::Damaged,
    }
}

impl AiManager {
    pub fn new(cfg: ManagerConfig) -> AppResult<Self> {
        let rt = Arc::new(AiRuntime::new()?);
        let paths = AiPaths::new(&cfg.app_data_dir);
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
        let (loaded, state) = Self::load(&cfg);
        let m = Self {
            cfg,
            paths,
            rt,
            loaded: RwLock::new(loaded),
            download: ModelDownload::default(),
            state: RwLock::new(state),
            check_lock: Mutex::new(()),
            supervisor,
            embed_supervisor,
            hw_cache: Mutex::new(None),
            placement: Mutex::new(Placement::Undecided),
            placement_lock: Mutex::new(()),
            search_model: Mutex::new(ModelRole::Embedding),
            usage: Mutex::new(client::Usage::default()),
        };
        m.configure();
        Ok(m)
    }

    /// Read the engine and the downloaded model set from disk (sizes only).
    fn load(cfg: &ManagerConfig) -> (Loaded, (BundleState, Option<AppError>)) {
        // The sizes and hashes the set is checked against: the pins compiled into this app.
        let (version, tier) = download::pins(&cfg.lock);
        let bundle = match &cfg.bundle_dir {
            Some(dir) => model_download::models_dir(&cfg.app_data_dir)
                .installed(&version, &tier)
                .and_then(|models| BundledAi::new(dir, cfg.lock.clone(), models)),
            None => Err(bundle::missing().with_detail("the app resources folder is unknown")),
        };
        match bundle.and_then(|b| b.check_present().map(|()| b)) {
            Ok(b) => {
                let stamps = b.models().map(|m| {
                    let mut roles = vec![ModelRole::Chat, ModelRole::Embedding];
                    if m.multilingual.is_some() {
                        roles.push(ModelRole::MultilingualEmbedding);
                    }
                    roles
                        .into_iter()
                        .map(|r| (r, ModelStamp::new(m.path(r), m.file(r).bytes)))
                        .collect()
                });
                let loaded = Loaded {
                    bundle: Some(b),
                    stamps,
                };
                (loaded, (BundleState::NotChecked, None))
            }
            Err(e) => {
                if !e.is("ai.not_installed") {
                    tracing::warn!(
                        code = e.code_str(),
                        detail = e.detail.as_deref().unwrap_or(""),
                        "the Offline AI is not usable"
                    );
                }
                let loaded = Loaded {
                    bundle: None,
                    stamps: None,
                };
                (loaded, (state_of(&e), Some(e)))
            }
        }
    }

    /// Stop the engines and let go of the model files (before they are removed or set aside).
    fn unload(&self) {
        self.supervisor.configure(None);
        self.embed_supervisor.configure(None);
        *self.loaded.write() = Loaded {
            bundle: None,
            stamps: None,
        };
    }

    /// Read the engine and model set again (after a download or a removal) and point the engines
    /// at it. Running engines are stopped first.
    fn reload(&self) {
        let _one_check = self.check_lock.lock();
        let (loaded, state) = Self::load(&self.cfg);
        *self.loaded.write() = loaded;
        *self.state.write() = state;
        // Another model set may need another place: decide again before the next start.
        *self.placement.lock() = Placement::Undecided;
        self.configure();
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
        let (Some(b), stamps) = ({
            let l = self.loaded.read();
            (l.bundle.clone(), l.stamps.clone())
        }) else {
            return Err(self.unavailable_reason());
        };
        *self.state.write() = (BundleState::Checking, None);
        stamps.iter().flatten().for_each(|(_, s)| s.hold());
        let checked = b.verify(&self.paths.bundle_check()).and_then(|()| {
            stamps
                .iter()
                .flatten()
                .try_for_each(|(_, s)| s.mark_checked())
        });
        match checked {
            Ok(()) => {
                *self.state.write() = (BundleState::Verified, None);
                Ok(())
            }
            // A busy file is not damage: AI stays usable and the check runs again on the next request.
            Err(e) if e.is("ai.file_busy") => {
                *self.state.write() = (BundleState::NotChecked, None);
                Err(e)
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
            Err(e)
                if matches!(
                    e.code_str(),
                    "ai.assets_damaged" | "ai.assets_missing" | "ai.models_damaged"
                ) =>
            {
                Err(self.switch_off(e))
            }
            other => other,
        }
    }

    /// Check the bundle, then begin loading the chat model in the background.
    pub fn warm_up(&self) -> AppResult<()> {
        self.ensure_verified()?;
        self.ensure_placement();
        let r = self.supervisor.start();
        self.guard(r)
    }

    /// The chat model when AI is available.
    pub fn chat_model(self: &Arc<Self>) -> Option<Arc<dyn InferenceProvider>> {
        if !self.available() {
            return None;
        }
        let reference = self.supervisor.status().model_reference?;
        Some(Arc::new(LocalRuntimeModel {
            manager: self.clone(),
            reference,
        }))
    }

    /// The chat engine's context window: the pinned model's, kept between 2,048 and 32,768 tokens.
    pub(crate) fn chat_context_tokens(&self) -> u32 {
        self.cfg.lock.chat_model().context_tokens.clamp(2048, 32768)
    }

    /// One chat completion on the chat engine (after the bundle check). Deterministic runs use
    /// temperature 0; the engine was started with a fixed seed.
    pub(crate) fn chat(&self, req: &client::ChatRequest) -> AppResult<String> {
        self.ensure_verified()?;
        self.ensure_placement();
        match self.chat_once(req, true) {
            // The engine on the graphics card stopped: remember it and go on on the processor
            // (ADR-0029 decision 5).
            Err(e) if self.on_card() && e.is("ai.unavailable") => {
                tracing::warn!(
                    code = e.code_str(),
                    "the AI engine on the graphics card stopped; going on on the processor"
                );
                self.card_failed();
                self.chat_once(req, true)
            }
            r => r,
        }
    }

    /// Get the chat engine ready before the first question: start it if memory allows, then read
    /// each fixed prompt start once, so the engine keeps its work and the first question reads
    /// only what differs. Blocking: call from a background thread. Answers are thrown away and the
    /// calls are not counted in the trace numbers. Does nothing when AI is off, when starting the
    /// engine would leave too little memory, once `stop` says so, or once a real request is in
    /// flight (it then finds the engine as the warm-up left it).
    pub fn prime(&self, prompts: &[client::ChatRequest], stop: &dyn Fn() -> bool) -> AppResult<()> {
        if !self.available() || stop() {
            return Ok(());
        }
        self.ensure_verified()?;
        self.ensure_placement();
        if !self.supervisor.is_running() && !self.room_to_start() {
            tracing::info!("the AI engine was not started early: little memory is free");
            return Ok(());
        }
        for req in prompts {
            if stop() || self.supervisor.in_flight() > 0 {
                break;
            }
            self.chat_once(req, false)?;
        }
        Ok(())
    }

    /// Free memory covers the chat model, its cache and the headroom the rest of the app needs.
    fn room_to_start(&self) -> bool {
        let need = self.loaded.read().bundle.as_ref().and_then(|b| {
            b.models()
                .map(|m| placement::model_mb(m.chat.bytes, self.chat_context_tokens()))
        });
        let free = hardware::inspect().available_ram_bytes / (1024 * 1024);
        need.is_some_and(|n| free >= n + EARLY_START_HEADROOM_MB)
    }

    fn chat_once(&self, req: &client::ChatRequest, count: bool) -> AppResult<String> {
        let ep = self.guard(self.supervisor.ensure_ready(self.cfg.ready_timeout))?;
        let _busy = self.supervisor.begin_request();
        let http = self.supervisor.http().clone();
        let mut req = req.clone();
        if self.cfg.deterministic {
            req.temperature = 0.0;
        }
        let (text, usage) = self
            .rt
            .run(async move { client::chat(&http, &ep, &req).await })?;
        if count {
            self.usage.lock().add(usage);
        }
        Ok(text)
    }

    /// The engine numbers of the chat calls since the last call, then zero again.
    pub(crate) fn take_usage(&self) -> client::Usage {
        std::mem::take(&mut *self.usage.lock())
    }

    // ------------------------------------------------------------ embeddings (C1)

    /// The search model the embedding engine runs now and its lock entry (the English model when
    /// the lock pins no multilingual one).
    fn search_entry(&self) -> (ModelRole, &crate::bundle::ModelEntry) {
        let role = *self.search_model.lock();
        match (role, self.cfg.lock.multilingual_embedding_model()) {
            (ModelRole::MultilingualEmbedding, Some(m)) => (role, m),
            _ => (ModelRole::Embedding, self.cfg.lock.embedding_model()),
        }
    }

    /// Whether this app pins a multilingual search model.
    pub fn has_multilingual_search(&self) -> bool {
        self.cfg.lock.multilingual_embedding_model().is_some()
    }

    /// The search model in use: `Embedding` (English) or `MultilingualEmbedding`.
    pub fn search_model(&self) -> ModelRole {
        self.search_entry().0
    }

    /// Run the English (`Embedding`) or the multilingual search model. Only the embedding engine
    /// restarts, and only when the choice changes; the chat engine is never touched.
    pub fn set_search_model(&self, role: ModelRole) {
        if role == ModelRole::Chat {
            return;
        }
        {
            let mut cur = self.search_model.lock();
            if *cur == role {
                return;
            }
            *cur = role;
        }
        tracing::info!(model = role.as_str(), "the search model was chosen");
        let usable = self.available();
        let l = self.loaded.read();
        let launcher = l
            .bundle
            .as_ref()
            .filter(|_| usable)
            .and_then(|b| b.models().map(|m| self.embedding_launcher(&l, b, m)));
        self.embed_supervisor.configure(launcher);
    }

    /// The downloaded search model in use (None when AI is not available).
    pub fn embedding_info(&self) -> Option<EmbeddingInfo> {
        if !self.available() {
            return None;
        }
        let (role, e) = self.search_entry();
        let l = self.loaded.read();
        let models = l.bundle.as_ref()?.models()?;
        Some(EmbeddingInfo {
            model_id: e.model_id.clone(),
            version: e.version.clone(),
            sha256: models.file(role).sha256.clone(),
            dim: e.embedding_dim?,
        })
    }

    /// Embed passages (indexed project text) with the search model in use. Blocking (call from a
    /// background thread); batches internally; one L2-normalised vector of
    /// `embedding_info().dim` values per input, in input order. Inputs are trimmed and bounded to
    /// [`MAX_EMBED_INPUT_CHARS`] characters; a model that needs a passage prefix gets it.
    pub fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        let prefix = self.search_entry().1.passage_prefix.clone();
        self.embed_prefixed(inputs, prefix.as_deref())
    }

    /// Embed search questions: like [`Self::embed`] with the model's query prefix.
    pub fn embed_queries(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        let prefix = self.search_entry().1.query_prefix.clone();
        self.embed_prefixed(inputs, prefix.as_deref())
    }

    fn embed_prefixed(
        &self,
        inputs: &[String],
        prefix: Option<&str>,
    ) -> Result<Vec<Vec<f32>>, AiError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let owned: Vec<String>;
        let inputs = match prefix {
            Some(p) => {
                owned = inputs.iter().map(|s| format!("{p}{}", s.trim())).collect();
                &owned[..]
            }
            None => inputs,
        };
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
        let cores = hw.physical_cpus.unwrap_or(hw.logical_cpus / 2);
        // Large hybrid chips put low-power cores in the count, and every engine thread waits for
        // the slowest. Measured 2026-10-16 on an Intel Core Ultra 5 125H (14 cores) with the
        // assistant's real prompts: 12 threads answered in 21.7 s against 25.7 s for 14, with no
        // overlap in 9 runs (docs/release/evidence/w5/). Smaller chips keep one thread per core.
        let threads = if cores >= 12 { cores - 2 } else { cores };
        threads.clamp(1, 16)
    }

    /// Point both engines at the bundle, or at nothing when AI is not usable.
    fn configure(&self) {
        let p = self.placement.lock().clone();
        self.configure_with(&p);
    }

    /// Point both engines at the bundle with the chat engine in place `p`, or at nothing when AI
    /// is not usable.
    fn configure_with(&self, p: &Placement) {
        let usable = self.available();
        let l = self.loaded.read();
        let ready = l.bundle.as_ref().filter(|_| usable);
        match ready.zip(ready.and_then(|b| b.models())) {
            Some((b, models)) => {
                let hw = self.hardware();
                let chat = self.cfg.lock.chat_model();
                // The launch plan is the only place that puts the chat engine on a card.
                let card = match p {
                    Placement::Card { device, .. } => b
                        .gpu_engine()
                        .map(|(exe, files)| (exe, files, device.clone())),
                    _ => None,
                };
                let (chat_exe, chat_files, gpu_device) = match card {
                    Some((exe, files, dev)) => (exe, files, Some(dev)),
                    None => (b.executable(), b.engine_files(), None),
                };
                let chat_launcher: Arc<dyn Launcher> = Arc::new(LlamaServerLauncher {
                    executable: chat_exe,
                    engine_files: Some(chat_files),
                    gpu_device,
                    model_path: models.path(ModelRole::Chat),
                    profile_id: format!("{}:{}", bundle::BUNDLE_ID, chat.model_id),
                    context_tokens: self.chat_context_tokens(),
                    threads: Self::threads(&hw),
                    seed: self.cfg.deterministic.then_some(DETERMINISTIC_SEED),
                    model_stamp: stamp(&l, ModelRole::Chat),
                });
                let embedding_launcher = self.embedding_launcher(&l, b, models);
                self.supervisor.configure(Some(chat_launcher));
                self.embed_supervisor.configure(Some(embedding_launcher));
            }
            None => {
                self.supervisor.configure(None);
                self.embed_supervisor.configure(None);
            }
        }
    }

    /// The embedding engine for the search model in use.
    fn embedding_launcher(
        &self,
        l: &Loaded,
        b: &BundledAi,
        models: &bundle::InstalledModels,
    ) -> Arc<dyn Launcher> {
        let (role, emb) = self.search_entry();
        Arc::new(EmbeddingServerLauncher {
            executable: b.executable(),
            engine_files: Some(b.engine_files()),
            model_path: models.path(role),
            model_id: emb.model_id.clone(),
            pooling: emb.pooling.unwrap_or(Pooling::Cls),
            context_tokens: emb.context_tokens,
            // Leave most cores to the chat model when both run.
            threads: (Self::threads(&self.hardware()) / 2).clamp(1, 4),
            model_stamp: stamp(l, role),
        })
    }

    // ------------------------------------------------------------- graphics card

    fn on_card(&self) -> bool {
        matches!(*self.placement.lock(), Placement::Card { .. })
    }

    /// Where the chat engine runs, in one plain sentence (Settings and the health card).
    pub fn placement_text(&self) -> String {
        placement::describe(&self.placement.lock())
    }

    /// Settings, Offline AI, Graphics card: `off` keeps the AI on the processor. The engine
    /// restarts in its new place on the next request.
    pub fn set_graphics_off(&self, off: bool) -> AppResult<()> {
        let _one = self.placement_lock.lock();
        let mut rec = self.graphics_record();
        rec.off = off;
        crate::store::write_json_atomic(&self.paths.graphics(), &rec)?;
        *self.placement.lock() = Placement::Undecided;
        self.configure();
        Ok(())
    }

    pub fn graphics_off(&self) -> bool {
        self.graphics_record().off
    }

    fn graphics_record(&self) -> GraphicsRecord {
        let path = self.paths.graphics();
        crate::store::read_json(&path).unwrap_or_else(|| {
            if path.exists() {
                tracing::warn!("the graphics card record could not be read; it starts again");
            }
            GraphicsRecord::default()
        })
    }

    /// Decide once per session, before the chat engine first starts, where it runs; the probe
    /// runs here the first time an adapter, driver and engine are seen.
    fn ensure_placement(&self) {
        if *self.placement.lock() != Placement::Undecided {
            return;
        }
        let _one = self.placement_lock.lock();
        if *self.placement.lock() != Placement::Undecided {
            return;
        }
        let p = self.choose_placement();
        tracing::info!(
            placement = placement::describe(&p),
            "the AI engine place was chosen"
        );
        *self.placement.lock() = p;
        self.configure();
    }

    fn choose_placement(&self) -> Placement {
        let Some(b) = self.loaded.read().bundle.clone() else {
            return Placement::Processor(CpuReason::NoAdapter);
        };
        let Some(models) = b.models().cloned() else {
            return Placement::Processor(CpuReason::NoAdapter);
        };
        let rec = self.graphics_record();
        let facts = (self.cfg.graphics)();
        // A result is remembered per engine and per way of measuring (a new probe checks again).
        let runtime = b.gpu_runtime().map(|r| placement::probe_key(&r.version));
        let need = placement::model_mb(models.chat.bytes, self.chat_context_tokens());
        // Free memory now, not when the hardware was first read.
        let free = hardware::inspect().available_ram_bytes / (1024 * 1024);
        match placement::plan(&facts, &rec, runtime.as_deref(), need, free) {
            gpu::Plan::Cpu(r) => Placement::Processor(r),
            gpu::Plan::Gpu { adapter } => {
                let (a, driver) = &facts.adapters[adapter];
                let tps = rec
                    .probes
                    .iter()
                    .rev()
                    .find(|p| p.adapter == a.name && &p.driver == driver)
                    .and_then(|p| p.gpu_tokens_per_second);
                match self.device_for(&b, &a.name) {
                    Some(device) => Placement::Card {
                        adapter: a.name.clone(),
                        vendor_id: a.vendor_id,
                        driver: driver.clone(),
                        device,
                        tokens_per_second: tps,
                    },
                    None => Placement::Processor(CpuReason::ProbeFailed),
                }
            }
            gpu::Plan::Probe { adapter } => {
                let (a, driver) = facts.adapters[adapter].clone();
                let runtime = runtime.unwrap_or_default();
                self.probe(&b, &a, &driver, &runtime)
            }
        }
    }

    /// The engine device for an adapter, from the Vulkan engine's own list.
    fn device_for(&self, b: &BundledAi, adapter: &str) -> Option<String> {
        let (exe, files) = b.gpu_engine()?;
        match crate::supervisor::list_devices(&exe, Some(&files)) {
            Ok(listing) => gpu::device_for(adapter, &gpu::parse_devices(&listing)),
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    "the graphics card engine could not list devices"
                );
                None
            }
        }
    }

    /// ADR-0029 decision 4: the fixed request on the processor, then on the card. The card is used
    /// only when it gave valid output faster; either way the result is remembered.
    fn probe(&self, b: &BundledAi, a: &gpu::Adapter, driver: &str, runtime: &str) -> Placement {
        let device = self.device_for(b, &a.name);
        // The processor figure. The session's placement stays undecided meanwhile, so no other
        // request runs on an engine that is still being checked.
        self.configure_with(&Placement::Undecided);
        let cpu = self.probe_once();
        let card = device.clone().map(|device| Placement::Card {
            adapter: a.name.clone(),
            vendor_id: a.vendor_id,
            driver: driver.to_string(),
            device,
            tokens_per_second: None,
        });
        let gpu = match &card {
            Some(c) => {
                self.configure_with(c);
                self.probe_once()
            }
            None => (false, None),
        };
        let passed = gpu::probe_passed(gpu.0, gpu.1, cpu.1);
        // A check that could not run fully (no processor figure, the card not listed, an unknown
        // driver) is not remembered: the processor is used this session and the card is checked
        // again next time.
        if cpu.1.is_none() || card.is_none() || driver.is_empty() {
            tracing::warn!("the graphics card check could not run fully; the processor is used");
            return Placement::Processor(CpuReason::ProbeFailed);
        }
        // Read again: the setting may have changed while the check ran.
        let mut rec = self.graphics_record();
        rec.remember(ProbeRecord {
            adapter: a.name.clone(),
            vendor_id: a.vendor_id,
            driver: driver.to_string(),
            runtime: runtime.to_string(),
            result: if passed {
                Remembered::Passed
            } else {
                Remembered::Failed
            },
            gpu_tokens_per_second: gpu.1,
            cpu_tokens_per_second: cpu.1,
            at: openframe_domain::now_ms(),
        });
        if let Err(e) = crate::store::write_json_atomic(&self.paths.graphics(), &rec) {
            tracing::warn!(
                code = e.code_str(),
                "the graphics card check could not be recorded"
            );
        }
        tracing::info!(
            passed,
            gpu_tps = gpu.1.unwrap_or(0.0),
            cpu_tps = cpu.1.unwrap_or(0.0),
            "the graphics card check ran"
        );
        match card {
            Some(Placement::Card {
                adapter,
                vendor_id,
                driver,
                device,
                ..
            }) if passed => Placement::Card {
                adapter,
                vendor_id,
                driver,
                device,
                tokens_per_second: gpu.1,
            },
            _ => Placement::Processor(CpuReason::ProbeFailed),
        }
    }

    /// One probe request on the engine as configured: (valid output, tokens a second).
    fn probe_once(&self) -> (bool, Option<f64>) {
        let ep = match self.guard(self.supervisor.ensure_ready(Duration::from_secs(180))) {
            Ok(ep) => ep,
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    detail = e.detail.as_deref().unwrap_or(""),
                    "the engine did not start for the graphics card check"
                );
                return (false, None);
            }
        };
        let http = self.supervisor.http().clone();
        let req = placement::probe_request();
        match self
            .rt
            .run(async move { client::chat(&http, &ep, &req).await })
        {
            Ok((text, usage)) => (
                placement::probe_valid(&text),
                placement::tokens_per_second(&usage),
            ),
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    detail = e.detail.as_deref().unwrap_or(""),
                    "the graphics card check request failed"
                );
                (false, None)
            }
        }
    }

    /// The engine on the card stopped while in use: remember the failure for this adapter,
    /// driver and engine, and go on on the processor.
    fn card_failed(&self) {
        let _one = self.placement_lock.lock();
        let Placement::Card {
            adapter,
            vendor_id,
            driver,
            ..
        } = self.placement.lock().clone()
        else {
            return;
        };
        let runtime = self
            .loaded
            .read()
            .bundle
            .as_ref()
            .and_then(|b| b.gpu_runtime().map(|r| placement::probe_key(&r.version)))
            .unwrap_or_default();
        let mut rec = self.graphics_record();
        rec.remember(ProbeRecord {
            adapter,
            vendor_id,
            driver,
            runtime,
            result: Remembered::Failed,
            gpu_tokens_per_second: None,
            cpu_tokens_per_second: None,
            at: openframe_domain::now_ms(),
        });
        if let Err(e) = crate::store::write_json_atomic(&self.paths.graphics(), &rec) {
            tracing::warn!(
                code = e.code_str(),
                "the graphics card check could not be recorded"
            );
        }
        *self.placement.lock() = Placement::Processor(CpuReason::ProbeFailed);
        self.configure();
    }

    // ------------------------------------------------------------- download

    /// The download screen's state.
    pub fn download_status(&self) -> DownloadStatus {
        self.download.status()
    }

    /// Bytes of the model set this build expects (shown before the click, without any network).
    pub fn download_size(&self) -> u64 {
        self.cfg.lock.models.iter().map(|m| m.file.bytes).sum()
    }

    /// This build can download (always, unless its test source value was invalid).
    pub fn can_download(&self) -> bool {
        self.cfg.model_source.is_some()
    }

    /// Start (or resume) the download on the person's click. Returns at once; the work runs on a
    /// background thread and `download_status` reports it.
    pub fn start_download(self: &Arc<Self>) -> AppResult<()> {
        let Some(host) = self.cfg.model_source.clone() else {
            return Err(AppError::ai(
                "download_unavailable",
                "This copy of OpenFrame Studio cannot download OpenFrame AI.",
            ));
        };
        if !self.download.begin() {
            return Ok(());
        }
        let me = self.clone();
        let spawned = std::thread::Builder::new()
            .name("of-ai-download".into())
            .spawn(move || {
                // Even a panic ends the run, so the slot is never left taken.
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    me.run_download(host)
                }))
                .unwrap_or_else(|_| Err(AppError::internal("the download stopped unexpectedly")));
                if let Err(e) = &r {
                    tracing::warn!(
                        code = e.code_str(),
                        detail = e.detail.as_deref().unwrap_or(""),
                        "the OpenFrame AI download stopped"
                    );
                }
                me.download.end(&r);
            });
        if let Err(e) = spawned {
            let r = Err(AppError::internal(format!("download thread: {e}")));
            self.download.end(&r);
            return r;
        }
        Ok(())
    }

    /// Pause a running download (the downloaded part stays).
    pub fn pause_download(&self) {
        self.download.pause();
    }

    /// Remove the downloaded models (the Remove button). AI is off afterwards until the next
    /// download; the projects are not touched.
    pub fn remove_models(&self) -> AppResult<()> {
        if !self.download.claim() {
            return Err(AppError::ai(
                "download_running",
                "Pause the OpenFrame AI download before removing it.",
            ));
        }
        self.unload();
        let r = model_download::models_dir(&self.cfg.app_data_dir).remove_all();
        self.download.release();
        self.download.reset();
        self.reload();
        r
    }

    fn run_download(self: &Arc<Self>, source: ModelSource) -> AppResult<()> {
        // What to fetch and trust: the pins compiled into this app (ADR-0032).
        let (version, tier) = download::pins(&self.cfg.lock);
        let dir = model_download::models_dir(&self.cfg.app_data_dir);
        if dir.active().is_some_and(|a| a.version == version) {
            if self.available() {
                return Ok(()); // this set is already in use
            }
            return Err(AppError::ai(
                "remove_first",
                "Remove OpenFrame AI in Settings, Offline AI, then download it again.",
            )
            .with_detail("the active set of this version is damaged"));
        }
        let me = self.clone();
        let (v, t) = (version.clone(), tier.clone());
        self.rt.run(async move {
            let dir = model_download::models_dir(&me.cfg.app_data_dir);
            me.download.fetch_files(&source, &dir, &v, &t).await
        })?;
        self.health_check(&dir)
    }

    /// Manual import (ADR-0032 decision 5): use a model file the person picked when it matches a
    /// pin. When both files are there (imported or downloaded before), the set is activated and
    /// checked like a download. Returns whether OpenFrame AI is now complete.
    pub fn import_model_file(&self, path: &std::path::Path) -> AppResult<bool> {
        if !self.download.claim() {
            return Err(AppError::ai(
                "download_running",
                "Pause the OpenFrame AI download before importing a file.",
            ));
        }
        let r = self.import_claimed(path);
        self.download.release();
        r
    }

    fn import_claimed(&self, path: &std::path::Path) -> AppResult<bool> {
        let (version, tier) = download::pins(&self.cfg.lock);
        let dir = model_download::models_dir(&self.cfg.app_data_dir);
        let f = dir.import_file(&version, &tier, path)?;
        tracing::info!(
            role = f.role.as_str(),
            "an OpenFrame AI model file was imported"
        );
        let same_version_active = dir.active().is_some_and(|a| a.version == version);
        if same_version_active && self.ensure_verified().is_ok() {
            // The set in use is whole: the copy is not needed, and nothing working is replaced.
            dir.sweep_staging("");
            return Ok(true);
        }
        if !tier
            .files
            .iter()
            .all(|f| dir.checked(&version, f).is_file())
        {
            return Ok(false);
        }
        if same_version_active {
            // The same set is active but damaged: replace it with the one just completed. On any
            // failure the engines come back on whatever is on disk, never left unloaded.
            self.unload();
            if let Err(e) = dir
                .remove_active_set()
                .and_then(|()| dir.activate(&version, &tier).map(|_| ()))
            {
                self.reload();
                return Err(e);
            }
        } else {
            dir.activate(&version, &tier)?;
        }
        self.health_check(&dir).map(|()| true)
    }

    /// After activation: the new set passes its full check and the engine starts on it, or the
    /// previous set comes back.
    fn health_check(&self, dir: &crate::install::ModelsDir) -> AppResult<()> {
        self.reload();
        // Same order as a chat: the files, then the engine's place (a first graphics card check
        // restarts the engine), then the engine. Deciding the place first means a warm-up running
        // at the same time can no longer restart the engine under the check.
        let healthy = self.ensure_verified().and_then(|()| {
            self.ensure_placement();
            self.guard(self.supervisor.ensure_ready(self.cfg.ready_timeout))
                .map(|_| ())
        });
        match healthy {
            Ok(()) => dir.confirm_healthy(),
            // A busy file or a slow first start is not a broken set: keep it (the previous set
            // stays until a later check passes) instead of throwing gigabytes away.
            Err(e) if e.is("ai.file_busy") || e.is("ai.timeout") => Err(e),
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    "the new OpenFrame AI set failed its health check; going back to the previous set"
                );
                self.unload();
                let rolled = dir.roll_back();
                self.reload();
                if let Err(r) = rolled {
                    tracing::error!(
                        code = r.code_str(),
                        "the previous OpenFrame AI set could not be restored"
                    );
                }
                // The health check's own reason is what the person needs to see.
                Err(e)
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
        let mut models = vec![
            (ComponentKind::ChatModel, lock.chat_model()),
            (ComponentKind::EmbeddingModel, lock.embedding_model()),
        ];
        if let Some(m) = lock.multilingual_embedding_model() {
            models.push((ComponentKind::EmbeddingModel, m));
        }
        for (kind, m) in models {
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
        if let Some(m) = self.loaded.read().bundle.as_ref().and_then(|b| b.models()) {
            for (c, f) in components.iter_mut().skip(1).zip(m.files()) {
                c.bytes = f.bytes;
                c.sha256 = f.sha256.clone();
            }
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
                .loaded
                .read()
                .bundle
                .as_ref()
                .map(BundledAi::total_bytes)
                .unwrap_or(0),
            log_file: self.paths.runtime_log().to_string_lossy().into_owned(),
            placement: self.placement_text(),
            graphics_off: self.graphics_off(),
        }
    }
}

/// The stamp of one model file, as it was at the last successful check.
fn stamp(l: &Loaded, role: ModelRole) -> Option<Arc<ModelStamp>> {
    l.stamps
        .as_ref()?
        .iter()
        .find(|(r, _)| *r == role)
        .map(|(_, s)| s.clone())
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
