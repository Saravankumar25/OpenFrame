//! Offline AI façade used by the application layer (Local AI Runtime spec; agentic AI spec
//! §12, §23–§25): manifest trust, device check, the one-click install of the single profile
//! (runtime + chat model + embedding model) with one combined progress, verified resumable
//! downloads, health check before activation, atomic activation with rollback, Vulkan → CPU
//! fallback, uninstall, and the two managed sidecars (chat + embeddings).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_domain::{AppError, AppResult, now_ms};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;

use crate::client::{self, ChatMessage, ChatRequest, EMBED_BATCH};
use crate::download::{self, Control, DownloadOptions, DownloadPhase, DownloadSpec};
use crate::hardware::{self, HardwareInfo};
use crate::manifest::{self, Backend, Manifest, ModelEntry, Pooling, RuntimeEntry};
use crate::model::{ChatModel, LocalRuntimeModel};
use crate::rt::AiRuntime;
use crate::selection::{self, Fitness};
use crate::store::{ACTIVE_SCHEMA, ActiveInstall, AiPaths, write_json_atomic};
use crate::supervisor::{
    EmbeddingServerLauncher, Launcher, LlamaServerLauncher, RuntimeStatus, StateListener,
    Supervisor, SupervisorConfig, not_installed,
};

/// Upper bounds for the distribution manifest and its signature (bounded reads).
const MAX_MANIFEST_BYTES: usize = 1 << 20;
const MAX_SIGNATURE_BYTES: usize = 4 << 10;
/// Characters of one input sent to the embedding model (≈ 300–400 English tokens; the model
/// window is 512). Retrieval chunks should be shorter; longer inputs are cut, and an input the
/// runtime still refuses (dense text) is retried shorter.
pub const MAX_EMBED_INPUT_CHARS: usize = 1_500;
/// Per-request timeout for one embedding batch.
const EMBED_TIMEOUT: Duration = Duration::from_secs(120);

/// Errors of the Offline AI subsystem (the application-wide error type).
pub type AiError = AppError;

#[derive(Clone)]
pub struct ManagerConfig {
    pub app_data_dir: PathBuf,
    /// Base64 Ed25519 key manifests must be signed with.
    pub trusted_public_key: String,
    /// Manifest shipped with the build (bytes + base64 signature).
    pub embedded_manifest: (Vec<u8>, String),
    /// Optional distribution endpoint serving `manifest.json` + `manifest.json.sig`.
    pub distribution_base_url: Option<String>,
    pub download: DownloadOptions,
    /// Chat sidecar.
    pub supervisor: SupervisorConfig,
    /// Embedding sidecar.
    pub embedding_supervisor: SupervisorConfig,
    /// How long installation waits for the new components to answer their health check.
    pub health_timeout: Duration,
    /// Force the runtime backend instead of the automatic device check (diagnostics and
    /// tests; `None` in the product).
    pub backend_override: Option<Backend>,
}

impl ManagerConfig {
    /// Configuration for the running application.
    pub fn production(app_data_dir: &Path) -> Self {
        let paths = AiPaths::new(app_data_dir);
        Self {
            app_data_dir: app_data_dir.to_path_buf(),
            trusted_public_key: manifest::trusted_public_key().to_string(),
            embedded_manifest: (
                manifest::EMBEDDED_MANIFEST.to_vec(),
                manifest::EMBEDDED_MANIFEST_SIG.trim().to_string(),
            ),
            distribution_base_url: manifest::DISTRIBUTION_BASE_URL
                .map(|s| s.trim_end_matches('/').to_string()),
            download: DownloadOptions::default(),
            supervisor: SupervisorConfig {
                log_path: Some(paths.runtime_log()),
                ..SupervisorConfig::default()
            },
            embedding_supervisor: SupervisorConfig {
                log_path: Some(paths.logs_dir().join("ai-embedding.log")),
                ..SupervisorConfig::default()
            },
            health_timeout: Duration::from_secs(600),
            backend_override: None,
        }
    }
}

/// The steps the user sees (agentic AI spec §24): Checking device → Downloading → Verifying →
/// Installing → Starting → Ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InstallPhase {
    Checking,
    Downloading,
    Verifying,
    Installing,
    Starting,
    Ready,
}

/// One combined progress for the whole Offline AI package: `done`/`total` are bytes across the
/// runtime, the chat model and the embedding model (never per file).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallEvent {
    pub phase: InstallPhase,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub profile_id: String,
    pub profile_version: String,
    pub runtime_id: String,
    pub backend: Backend,
    /// Bytes of superseded components removed after the new install became active.
    pub freed_bytes: u64,
}

pub type InstallEvents<'a> = &'a (dyn Fn(InstallEvent) + Send + Sync);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    Runtime,
    ChatModel,
    EmbeddingModel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedComponent {
    pub kind: ComponentKind,
    pub id: String,
    pub bytes: u64,
    /// Bytes already on disk from a paused/interrupted download.
    pub downloaded_bytes: u64,
    pub installed: bool,
}

/// Everything the UI must show BEFORE the download starts (exact size, disk preflight, how it
/// will run on this computer).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub profile_version: String,
    pub backend: Backend,
    pub runtime: RuntimeEntry,
    pub chat: ModelEntry,
    pub embedding: ModelEntry,
    pub components: Vec<PlannedComponent>,
    /// Exact bytes still to transfer.
    pub download_bytes: u64,
    /// Full size of the components that are not installed yet.
    pub total_bytes: u64,
    /// Bytes of those components already downloaded (resumable).
    pub downloaded_bytes: u64,
    pub required_free_bytes: u64,
    pub free_disk_bytes: Option<u64>,
    pub enough_disk: bool,
    pub fitness: Fitness,
    /// The active install already is this exact profile version.
    pub up_to_date: bool,
}

/// What `ai.status` needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineAiState {
    pub installed: bool,
    pub active: Option<ActiveInstall>,
    /// A newer verified profile version than the active one is available.
    pub update_available: bool,
    pub installed_bytes: u64,
}

/// C1 contract (agentic AI spec §12): identity of the installed embedding model. Part of the
/// semantic index metadata — a different model/version/hash means the index is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingInfo {
    pub model_id: String,
    pub version: String,
    pub sha256: String,
    pub dim: u32,
}

/// Technical details for Settings → Offline AI → diagnostics / About (never the normal UI).
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
    pub sha256: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub profile_id: String,
    pub profile_version: Option<String>,
    pub available_version: Option<String>,
    pub manifest_channel: Option<String>,
    pub manifest_sequence: Option<u64>,
    /// This build still trusts the development manifest key (never a public release).
    pub development_key: bool,
    pub backend: Option<Backend>,
    pub components: Vec<ComponentDiagnostics>,
    pub chat_runtime: RuntimeStatus,
    pub embedding_runtime: RuntimeStatus,
    pub hardware: HardwareInfo,
    pub store_dir: String,
    pub log_file: String,
    pub installed_bytes: u64,
}

pub struct AiManager {
    cfg: ManagerConfig,
    paths: AiPaths,
    rt: Arc<AiRuntime>,
    http: reqwest::Client,
    supervisor: Arc<Supervisor>,
    embed_supervisor: Arc<Supervisor>,
    embedding: RwLock<Option<EmbeddingInfo>>,
    manifest: RwLock<Option<Manifest>>,
    hw_cache: Mutex<Option<(Instant, HardwareInfo)>>,
    install_lock: Mutex<()>,
}

fn unsupported() -> AppError {
    AppError::ai(
        "unsupported",
        "Offline AI isn't available for this kind of computer yet.",
    )
}

fn embedding_info(e: &ModelEntry) -> Option<EmbeddingInfo> {
    Some(EmbeddingInfo {
        model_id: e.model_id.clone(),
        version: e.version.clone(),
        sha256: e.sha256.clone(),
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

impl AiManager {
    pub fn new(cfg: ManagerConfig) -> AppResult<Self> {
        let rt = Arc::new(AiRuntime::new()?);
        let http = reqwest::Client::builder()
            .user_agent(concat!("OpenFrame-Studio/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(30))
            // Every redirect hop must stay HTTPS (or loopback), see `manifest::redirect_policy`.
            .redirect(manifest::redirect_policy())
            .build()
            .map_err(|e| AppError::internal(e.to_string()))?;
        let supervisor = Arc::new(Supervisor::new(cfg.supervisor.clone(), rt.handle()));
        let embed_supervisor = Arc::new(Supervisor::new(
            cfg.embedding_supervisor.clone(),
            rt.handle(),
        ));
        let m = Self {
            paths: AiPaths::new(&cfg.app_data_dir),
            cfg,
            rt,
            http,
            supervisor,
            embed_supervisor,
            embedding: RwLock::new(None),
            manifest: RwLock::new(None),
            hw_cache: Mutex::new(None),
            install_lock: Mutex::new(()),
        };
        m.configure_from_active();
        Ok(m)
    }

    pub fn paths(&self) -> &AiPaths {
        &self.paths
    }

    /// Chat runtime state changes (the AI panel refreshes on them).
    pub fn set_state_listener(&self, l: StateListener) {
        self.supervisor.set_listener(l);
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        self.supervisor.status()
    }

    pub fn embedding_status(&self) -> RuntimeStatus {
        self.embed_supervisor.status()
    }

    /// Begin loading the chat model in the background (no-op error when not installed).
    pub fn warm_up(&self) -> AppResult<()> {
        self.supervisor.start()
    }

    /// Stop both sidecars (memory released; they start again on the next request).
    pub fn stop_runtime(&self) {
        self.supervisor.stop();
        self.embed_supervisor.stop();
    }

    // ------------------------------------------------------------- manifest

    /// The newest verified manifest (embedded or cached from the distribution endpoint).
    pub fn manifest(&self) -> AppResult<Manifest> {
        if let Some(m) = self.manifest.read().clone() {
            return Ok(m);
        }
        let (bytes, sig) = &self.cfg.embedded_manifest;
        // The embedded manifest is only one candidate: a release build that trusts its own key
        // but still embeds a development-signed manifest must fall back to a verified cached
        // (or later, fetched) manifest — and must never trust the embedded one.
        let embedded = manifest::verify_and_parse(bytes, sig, &self.cfg.trusted_public_key);
        let mut best: Option<Manifest> = embedded.as_ref().ok().cloned();
        if let (Ok(cbytes), Ok(csig)) = (
            std::fs::read(self.paths.manifest_cache()),
            std::fs::read_to_string(self.paths.manifest_cache_sig()),
        ) {
            match manifest::verify_and_parse(&cbytes, &csig, &self.cfg.trusted_public_key) {
                Ok(cached) if best.as_ref().is_none_or(|b| cached.sequence > b.sequence) => {
                    best = Some(cached)
                }
                Ok(_) => {}
                Err(e) => {
                    // A tampered (or retired-format) cache is never used; remove it.
                    tracing::warn!(
                        code = e.code_str(),
                        "cached AI manifest failed verification; discarding"
                    );
                    let _ = std::fs::remove_file(self.paths.manifest_cache());
                    let _ = std::fs::remove_file(self.paths.manifest_cache_sig());
                }
            }
        }
        let best = match (best, embedded) {
            (Some(b), _) => b,
            (None, Err(e)) => return Err(e),
            (None, Ok(m)) => m,
        };
        *self.manifest.write() = Some(best.clone());
        Ok(best)
    }

    /// Fetch, verify and cache the manifest from the distribution endpoint.
    /// Keeps the current manifest if the endpoint is unreachable or offers an older one.
    pub fn refresh_manifest(&self) -> AppResult<Manifest> {
        let current = self.manifest();
        let Some(base) = self.cfg.distribution_base_url.clone() else {
            return current;
        };
        if !manifest::valid_url(&base) {
            return current;
        }
        let http = self.http.clone();
        let fetched = self.rt.run(async move {
            let get = |u: String, cap: usize| {
                let http = http.clone();
                async move {
                    let mut r = http
                        .get(u)
                        .timeout(Duration::from_secs(30))
                        .send()
                        .await
                        .map_err(|e| AppError::network(e.to_string()))?;
                    if !r.status().is_success() {
                        return Err(AppError::network(format!("HTTP {}", r.status().as_u16())));
                    }
                    // Bounded read: a hostile or broken endpoint can't make us buffer gigabytes.
                    let mut body = Vec::new();
                    while let Some(chunk) = r
                        .chunk()
                        .await
                        .map_err(|e| AppError::network(e.to_string()))?
                    {
                        if body.len() + chunk.len() > cap {
                            return Err(AppError::network("manifest response too large"));
                        }
                        body.extend_from_slice(&chunk);
                    }
                    Ok(body)
                }
            };
            let bytes = get(format!("{base}/manifest.json"), MAX_MANIFEST_BYTES).await?;
            let sig = get(format!("{base}/manifest.json.sig"), MAX_SIGNATURE_BYTES).await?;
            Ok((bytes, String::from_utf8_lossy(&sig).trim().to_string()))
        });
        let (bytes, sig) = match fetched {
            Ok(v) => v,
            Err(e) => {
                tracing::info!(code = e.code_str(), "AI manifest refresh skipped");
                return current;
            }
        };
        let remote = manifest::verify_and_parse(&bytes, &sig, &self.cfg.trusted_public_key)?;
        if let Ok(current) = current
            && remote.sequence < current.sequence
        {
            return Ok(current);
        }
        std::fs::create_dir_all(self.paths.models_dir())?;
        let tmp = self.paths.manifest_cache().with_extension("json.tmp");
        std::fs::write(&tmp, &bytes)?;
        std::fs::write(self.paths.manifest_cache_sig(), sig.as_bytes())?;
        std::fs::rename(&tmp, self.paths.manifest_cache())?;
        *self.manifest.write() = Some(remote.clone());
        Ok(remote)
    }

    // ------------------------------------------------------------- hardware

    pub fn hardware(&self, refresh: bool) -> HardwareInfo {
        let mut cache = self.hw_cache.lock();
        if !refresh
            && let Some((at, hw)) = cache.as_ref()
            && at.elapsed() < Duration::from_secs(30)
        {
            return hw.clone();
        }
        let _ = std::fs::create_dir_all(self.paths.models_dir());
        let hw = hardware::inspect(&self.paths.models_dir());
        *cache = Some((Instant::now(), hw.clone()));
        hw
    }

    // --------------------------------------------------------------- status

    pub fn active(&self) -> Option<ActiveInstall> {
        self.paths.active()
    }

    /// Installed = an active install whose components are all present and verified.
    pub fn state(&self) -> OfflineAiState {
        let active = self.paths.active();
        let installed =
            self.supervisor.status().model_reference.is_some() && self.embedding.read().is_some();
        let update_available = match (&active, self.manifest()) {
            (Some(a), Ok(m)) if installed => {
                a.profile_version != m.profile.version
                    || a.chat_model_id != m.profile.chat_model_id
                    || a.embedding_model_id != m.profile.embedding_model_id
            }
            _ => false,
        };
        OfflineAiState {
            installed,
            active: active.filter(|_| installed),
            update_available,
            installed_bytes: self.paths.installed_bytes(),
        }
    }

    /// Exact download size, disk preflight and device fit — shown before anything is downloaded.
    pub fn plan(&self) -> AppResult<InstallPlan> {
        let m = self.manifest()?;
        self.plan_with(&m, &self.hardware(false))
    }

    fn plan_with(&self, m: &Manifest, hw: &HardwareInfo) -> AppResult<InstallPlan> {
        let chat = m.chat_model()?.clone();
        let embedding = m.embedding_model()?.clone();
        let active = self.paths.active();
        let up_to_date = active.as_ref().is_some_and(|a| {
            a.profile_version == m.profile.version
                && a.chat_model_id == chat.model_id
                && a.embedding_model_id == embedding.model_id
                && self.paths.installed_runtime(&a.runtime_id).is_some()
                && self.paths.model_installed(&chat)
                && self.paths.model_installed(&embedding)
        });
        // An up-to-date install keeps the runtime it proved healthy with (e.g. after a
        // graphics-card → processor fallback); otherwise pick one for this computer.
        let runtime = match active
            .as_ref()
            .filter(|_| up_to_date)
            .and_then(|a| self.paths.installed_runtime(&a.runtime_id))
        {
            Some(r) => r,
            None => {
                let backend = self
                    .cfg
                    .backend_override
                    .unwrap_or_else(|| selection::choose_backend(hw, &chat));
                m.runtime_for(backend)
                    .or_else(|| m.runtime_for(Backend::Cpu))
                    .ok_or_else(unsupported)?
                    .clone()
            }
        };
        let fitness = selection::assess(hw, &chat, runtime.backend);
        let rt_spec = self.runtime_spec(&runtime);
        let mut components = vec![PlannedComponent {
            kind: ComponentKind::Runtime,
            id: runtime.runtime_id.clone(),
            bytes: runtime.bytes,
            downloaded_bytes: download::partial_len(&rt_spec),
            installed: self.paths.runtime_installed(&runtime),
        }];
        for (kind, e) in [
            (ComponentKind::ChatModel, &chat),
            (ComponentKind::EmbeddingModel, &embedding),
        ] {
            components.push(PlannedComponent {
                kind,
                id: e.model_id.clone(),
                bytes: e.bytes,
                downloaded_bytes: download::partial_len(&self.model_spec(e)),
                installed: self.paths.model_installed(e),
            });
        }
        let pending = || components.iter().filter(|c| !c.installed);
        let total_bytes: u64 = pending().map(|c| c.bytes).sum();
        let downloaded_bytes: u64 = pending().map(|c| c.downloaded_bytes.min(c.bytes)).sum();
        let remaining = |kind_is_runtime: bool| -> u64 {
            pending()
                .filter(|c| (c.kind == ComponentKind::Runtime) == kind_is_runtime)
                .map(|c| c.bytes.saturating_sub(c.downloaded_bytes))
                .sum()
        };
        let required_free_bytes = selection::required_free_bytes(remaining(false), remaining(true));
        let free_disk_bytes = hw.free_disk_bytes;
        Ok(InstallPlan {
            profile_version: m.profile.version.clone(),
            backend: runtime.backend,
            runtime,
            chat,
            embedding,
            download_bytes: total_bytes - downloaded_bytes,
            total_bytes,
            downloaded_bytes,
            required_free_bytes,
            free_disk_bytes,
            enough_disk: free_disk_bytes.is_none_or(|f| f >= required_free_bytes),
            fitness,
            up_to_date,
            components,
        })
    }

    /// The chat model when Offline AI is installed.
    pub fn chat_model(&self) -> Option<Arc<dyn ChatModel>> {
        let status = self.supervisor.status();
        let reference = status.model_reference?;
        Some(Arc::new(LocalRuntimeModel {
            supervisor: self.supervisor.clone(),
            rt: self.rt.clone(),
            reference,
            ready_timeout: self.cfg.health_timeout,
        }))
    }

    // ------------------------------------------------------------ embeddings (C1)

    /// The installed embedding model (None when Offline AI is not installed).
    pub fn embedding_info(&self) -> Option<EmbeddingInfo> {
        self.embedding.read().clone()
    }

    /// Embed texts with the local embedding model. Blocking (call from a background thread);
    /// batches internally; one L2-normalised vector of `embedding_info().dim` values per input,
    /// in input order. Inputs are trimmed and bounded to [`MAX_EMBED_INPUT_CHARS`] characters.
    pub fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let info = self.embedding_info().ok_or_else(not_installed)?;
        let dim = info.dim as usize;
        let ep = self
            .embed_supervisor
            .ensure_ready(self.cfg.health_timeout)?;
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

    fn chat_launcher(
        &self,
        runtime: &RuntimeEntry,
        chat: &ModelEntry,
        hw: &HardwareInfo,
    ) -> Arc<dyn Launcher> {
        Arc::new(LlamaServerLauncher {
            executable: self.paths.runtime_executable(runtime),
            model_path: self.paths.model_file(&chat.model_id),
            profile_id: format!("{}:{}", manifest::PROFILE_ID, chat.model_id),
            context_tokens: chat.context_tokens.clamp(2048, 32768),
            gpu_layers: selection::gpu_layers(hw, chat, runtime.backend),
            threads: Self::threads(hw),
        })
    }

    fn embedding_launcher(
        &self,
        runtime: &RuntimeEntry,
        emb: &ModelEntry,
        hw: &HardwareInfo,
    ) -> Arc<dyn Launcher> {
        Arc::new(EmbeddingServerLauncher {
            executable: self.paths.runtime_executable(runtime),
            model_path: self.paths.model_file(&emb.model_id),
            model_id: emb.model_id.clone(),
            pooling: emb.pooling.unwrap_or(Pooling::Cls),
            context_tokens: emb.context_tokens,
            // Leave most cores to the chat model when both run.
            threads: (Self::threads(hw) / 2).clamp(1, 4),
        })
    }

    /// Point both sidecars at the active install recorded on disk (or at nothing).
    fn configure_from_active(&self) {
        let found = self.paths.active().and_then(|a| {
            let rt = self.paths.installed_runtime(&a.runtime_id)?;
            let chat = self.paths.installed_model(&a.chat_model_id)?;
            let emb = self.paths.installed_model(&a.embedding_model_id)?;
            Some((rt, chat, emb))
        });
        match found {
            Some((rt, chat, emb)) => {
                let hw = self.hardware(false);
                self.supervisor
                    .configure(Some(self.chat_launcher(&rt, &chat, &hw)));
                self.embed_supervisor
                    .configure(Some(self.embedding_launcher(&rt, &emb, &hw)));
                *self.embedding.write() = embedding_info(&emb);
            }
            None => {
                self.supervisor.configure(None);
                self.embed_supervisor.configure(None);
                *self.embedding.write() = None;
            }
        }
    }

    // --------------------------------------------------------------- install

    fn model_spec(&self, e: &ModelEntry) -> DownloadSpec {
        DownloadSpec {
            url: e.url.clone(),
            bytes: e.bytes,
            sha256: e.sha256.clone(),
            partial: self.paths.model_partial(&e.model_id),
            dest: self.paths.model_file(&e.model_id),
            quarantine_dir: self.paths.quarantine_dir(),
            label: e.model_id.clone(),
        }
    }

    fn runtime_spec(&self, r: &RuntimeEntry) -> DownloadSpec {
        let partial = self.paths.runtime_partial(&r.runtime_id);
        DownloadSpec {
            url: r.url.clone(),
            bytes: r.bytes,
            sha256: r.sha256.clone(),
            dest: partial.with_extension("verified"),
            partial,
            quarantine_dir: self.paths.quarantine_dir(),
            label: r.runtime_id.clone(),
        }
    }

    /// Download, verify and activate the Offline AI profile (runtime + chat model + embedding
    /// model). Blocking; call from a background task. The previously active install stays
    /// active until the new one passes its health check; OpenFrame is usable immediately after
    /// (no restart).
    pub fn install(
        &self,
        control: &Control,
        events: InstallEvents<'_>,
    ) -> AppResult<InstallOutcome> {
        let _guard = self.install_lock.try_lock().ok_or_else(|| {
            AppError::ai("install_running", "Offline AI is already being installed.")
        })?;
        events(InstallEvent {
            phase: InstallPhase::Checking,
            done: 0,
            total: 0,
        });
        let manifest = self.refresh_manifest()?;
        let hw = self.hardware(true);
        let plan = self.plan_with(&manifest, &hw)?;

        // Disk-space preflight: nothing is downloaded when space is short.
        std::fs::create_dir_all(self.paths.models_dir())?;
        download::check_disk_space(&self.paths.models_dir(), plan.required_free_bytes)?;

        let progress = Progress::new(plan.total_bytes, events);
        if !self.paths.runtime_installed(&plan.runtime) {
            self.install_runtime(&plan.runtime, control, &progress)?;
        }
        for e in [&plan.chat, &plan.embedding] {
            if !self.paths.model_installed(e) {
                self.download_model(e, control, &progress)?;
            }
        }

        // Health check both sidecars, then atomic activation (active.json switch).
        control.check()?;
        progress.phase(InstallPhase::Starting);
        let used_runtime = match self.health_check(&plan.runtime, &plan.chat, &plan.embedding, &hw)
        {
            Ok(()) => plan.runtime.clone(),
            Err(e) if plan.runtime.backend != Backend::Cpu => {
                // Graphics driver trouble: fall back to the processor build.
                tracing::warn!(
                    code = e.code_str(),
                    "graphics-card runtime failed its health check; trying the processor runtime"
                );
                let cpu = manifest
                    .runtime_for(Backend::Cpu)
                    .cloned()
                    .ok_or_else(|| self.rollback(&plan.chat.model_id, e))?;
                if !self.paths.runtime_installed(&cpu) {
                    progress.grow(cpu.bytes);
                    self.install_runtime(&cpu, control, &progress)?;
                    progress.phase(InstallPhase::Starting);
                }
                self.health_check(&cpu, &plan.chat, &plan.embedding, &hw)
                    .map_err(|e| self.rollback(&plan.chat.model_id, e))?;
                cpu
            }
            Err(e) => return Err(self.rollback(&plan.chat.model_id, e)),
        };
        let active = ActiveInstall {
            schema: ACTIVE_SCHEMA,
            profile_id: manifest.profile.profile_id.clone(),
            profile_version: manifest.profile.version.clone(),
            runtime_id: used_runtime.runtime_id.clone(),
            backend: used_runtime.backend,
            chat_model_id: plan.chat.model_id.clone(),
            embedding_model_id: plan.embedding.model_id.clone(),
            activated_at: now_ms(),
        };
        self.paths.set_active(Some(&active))?;
        self.paths.mark_health(&plan.chat.model_id, false)?;
        *self.embedding.write() = embedding_info(&plan.embedding);
        // Only now is anything older removed (superseded versions, the unused runtime build,
        // leftovers of the retired multi-profile system).
        let freed_bytes = self.paths.remove_unreferenced(&active);
        progress.phase(InstallPhase::Ready);
        Ok(InstallOutcome {
            profile_id: active.profile_id,
            profile_version: active.profile_version,
            runtime_id: active.runtime_id,
            backend: active.backend,
            freed_bytes,
        })
    }

    /// Start both sidecars with the candidate components and require one real answer from each:
    /// a chat completion and an embedding of the expected dimension.
    fn health_check(
        &self,
        runtime: &RuntimeEntry,
        chat: &ModelEntry,
        emb: &ModelEntry,
        hw: &HardwareInfo,
    ) -> AppResult<()> {
        self.supervisor
            .configure(Some(self.chat_launcher(runtime, chat, hw)));
        self.embed_supervisor
            .configure(Some(self.embedding_launcher(runtime, emb, hw)));
        {
            let ep = self.supervisor.ensure_ready(self.cfg.health_timeout)?;
            let _busy = self.supervisor.begin_request();
            let http = self.supervisor.http().clone();
            let mut req =
                ChatRequest::new(vec![ChatMessage::user("Reply with the single word: ready")])
                    .max_tokens(8);
            req.timeout = Duration::from_secs(120);
            self.rt
                .run(async move { client::chat(&http, &ep, &req).await })?;
        }
        let dim = emb.embedding_dim.unwrap_or(0) as usize;
        let ep = self
            .embed_supervisor
            .ensure_ready(self.cfg.health_timeout)?;
        let _busy = self.embed_supervisor.begin_request();
        let http = self.embed_supervisor.http().clone();
        let v = self.rt.run(async move {
            client::embed(&http, &ep, &["ready".to_string()], dim, EMBED_TIMEOUT).await
        })?;
        if v.len() != 1 || v[0].iter().all(|x| *x == 0.0) {
            return Err(client::malformed(
                "embedding health check returned no vector",
            ));
        }
        Ok(())
    }

    /// The candidate failed: keep the previous setup active and explain.
    fn rollback(&self, chat_model_id: &str, cause: AppError) -> AppError {
        let _ = self.paths.mark_health(chat_model_id, true);
        self.configure_from_active();
        AppError::ai(
            "start_failed",
            "Offline AI was downloaded and verified, but it could not start on this computer. Anything you had before was kept. You can try again.",
        )
        .with_detail(format!("{}: {}", cause.code_str(), cause.detail.clone().unwrap_or_default()))
        .retryable()
    }

    fn fetch(
        &self,
        spec: &DownloadSpec,
        control: &Control,
        progress: &Progress<'_>,
    ) -> AppResult<PathBuf> {
        let http = self.http.clone();
        let opts = self.cfg.download.clone();
        let control2 = control.clone();
        let spec2 = spec.clone();
        let cb = |phase: DownloadPhase, done: u64, _total: u64| progress.file(phase, done);
        let path = run_with_progress(
            &self.rt,
            move |p| async move {
                download::download_verified(&http, &spec2, &opts, &control2, &*p).await
            },
            &cb,
        )?;
        progress.finish_file(spec.bytes);
        Ok(path)
    }

    fn download_model(
        &self,
        model: &ModelEntry,
        control: &Control,
        progress: &Progress<'_>,
    ) -> AppResult<()> {
        // A leftover folder without valid integrity is replaced. It is never the active install
        // (the active install is always verified); stop the sidecars anyway if it is referenced.
        let dir = self.paths.model_dir(&model.model_id);
        if dir.exists() && !self.paths.model_installed(model) {
            if self.paths.active().is_some_and(|a| {
                a.chat_model_id == model.model_id || a.embedding_model_id == model.model_id
            }) {
                self.supervisor.configure(None);
                self.embed_supervisor.configure(None);
            }
            std::fs::remove_dir_all(&dir)?;
        }
        self.fetch(&self.model_spec(model), control, progress)?;
        progress.phase(InstallPhase::Installing);
        self.paths.record_model(model)?;
        Ok(())
    }

    fn install_runtime(
        &self,
        runtime: &RuntimeEntry,
        control: &Control,
        progress: &Progress<'_>,
    ) -> AppResult<()> {
        let zip = self.fetch(&self.runtime_spec(runtime), control, progress)?;
        progress.phase(InstallPhase::Installing);
        let final_dir = self.paths.runtime_dir(&runtime.runtime_id);
        let tmp = self
            .paths
            .runtimes_dir()
            .join(format!(".{}.extracting", runtime.runtime_id));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
        let limits = openframe_security::archive::ArchiveLimits {
            max_entries: 4_000,
            max_total_bytes: 3 << 30,
            max_entry_bytes: 1536 << 20,
            max_ratio: 200,
        };
        openframe_security::archive::extract_all(&zip, &tmp, limits).map_err(|e| {
            AppError::ai(
                "runtime_invalid",
                "The Offline AI engine package couldn't be installed. Nothing was changed.",
            )
            .with_detail(format!(
                "{}: {}",
                e.code_str(),
                e.detail.clone().unwrap_or_default()
            ))
        })?;
        let mut exe = tmp.clone();
        for part in runtime.executable.split(['/', '\\']) {
            exe.push(part);
        }
        if !exe.is_file() {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(AppError::ai(
                "runtime_invalid",
                "The Offline AI engine package couldn't be installed. Nothing was changed.",
            )
            .with_detail("server executable missing from archive"));
        }
        write_json_atomic(
            &tmp.join("integrity.json"),
            &crate::store::IntegrityRecord {
                algorithm: "sha256".into(),
                sha256: runtime.sha256.clone(),
                bytes: runtime.bytes,
                verified_at: now_ms(),
            },
        )?;
        write_json_atomic(&tmp.join("runtime.json"), runtime)?;
        if final_dir.exists() {
            // Only reached when the old directory failed its integrity check.
            if self
                .paths
                .active()
                .is_some_and(|a| a.runtime_id == runtime.runtime_id)
            {
                self.supervisor.configure(None);
                self.embed_supervisor.configure(None);
            }
            std::fs::remove_dir_all(&final_dir)?;
        }
        std::fs::rename(&tmp, &final_dir)?;
        let _ = std::fs::remove_file(&zip);
        Ok(())
    }

    /// Remove Offline AI from this computer: both sidecars stop and every downloaded component
    /// (runtime, models, partial downloads) is deleted. Projects and assistant history are not
    /// touched; the rest of OpenFrame is unaffected. Returns the bytes freed.
    pub fn uninstall(&self) -> AppResult<u64> {
        let _guard = self.install_lock.try_lock().ok_or_else(|| {
            AppError::ai(
                "install_running",
                "Please wait until the current Offline AI download finishes or is cancelled.",
            )
        })?;
        self.supervisor.configure(None);
        self.embed_supervisor.configure(None);
        *self.embedding.write() = None;
        self.paths.remove_all_components()
    }

    /// Throw away partially downloaded files (the "Cancel" choice; "Pause" keeps them).
    pub fn discard_partials(&self) {
        let Ok(m) = self.manifest() else { return };
        for e in &m.models {
            download::discard_partial(&self.paths.model_partial(&e.model_id));
        }
        for r in &m.runtimes {
            download::discard_partial(&self.paths.runtime_partial(&r.runtime_id));
        }
    }

    // ----------------------------------------------------------- diagnostics

    /// Technical details for Settings → Offline AI → diagnostics / About.
    pub fn diagnostics(&self) -> Diagnostics {
        let manifest = self.manifest().ok();
        let active = self.state().active;
        let mut components = Vec::new();
        let model_diag = |kind, e: &ModelEntry, installed: bool| ComponentDiagnostics {
            kind,
            id: e.model_id.clone(),
            name: format!("{} ({})", e.display_name, e.quantization),
            version: e.version.clone(),
            license_id: e.license_id.clone(),
            license_url: Some(e.license_url.clone()),
            bytes: e.bytes,
            sha256: e.sha256.clone(),
            installed,
        };
        let runtime_diag = |r: &RuntimeEntry, installed: bool| ComponentDiagnostics {
            kind: ComponentKind::Runtime,
            id: r.runtime_id.clone(),
            name: format!("{} server ({})", r.engine, r.backend.label()),
            version: r.version.clone(),
            license_id: r.license_id.clone(),
            license_url: None,
            bytes: r.bytes,
            sha256: r.sha256.clone(),
            installed,
        };
        match &active {
            Some(a) => {
                if let Some(r) = self.paths.installed_runtime(&a.runtime_id) {
                    components.push(runtime_diag(&r, true));
                }
                if let Some(e) = self.paths.installed_model(&a.chat_model_id) {
                    components.push(model_diag(ComponentKind::ChatModel, &e, true));
                }
                if let Some(e) = self.paths.installed_model(&a.embedding_model_id) {
                    components.push(model_diag(ComponentKind::EmbeddingModel, &e, true));
                }
            }
            None => {
                if let Ok(plan) = self.plan() {
                    components.push(runtime_diag(&plan.runtime, false));
                    components.push(model_diag(ComponentKind::ChatModel, &plan.chat, false));
                    components.push(model_diag(
                        ComponentKind::EmbeddingModel,
                        &plan.embedding,
                        false,
                    ));
                }
            }
        }
        Diagnostics {
            profile_id: manifest::PROFILE_ID.to_string(),
            profile_version: active.as_ref().map(|a| a.profile_version.clone()),
            available_version: manifest.as_ref().map(|m| m.profile.version.clone()),
            manifest_channel: manifest.as_ref().map(|m| m.channel.clone()),
            manifest_sequence: manifest.as_ref().map(|m| m.sequence),
            development_key: manifest::using_development_key(),
            backend: active.as_ref().map(|a| a.backend),
            components,
            chat_runtime: self.supervisor.status(),
            embedding_runtime: self.embed_supervisor.status(),
            hardware: self.hardware(false),
            store_dir: self.paths.root().to_string_lossy().into_owned(),
            log_file: self.paths.runtime_log().to_string_lossy().into_owned(),
            installed_bytes: self.paths.installed_bytes(),
        }
    }
}

/// Folds per-file download callbacks into ONE progress for the whole package.
struct Progress<'a> {
    events: InstallEvents<'a>,
    state: Mutex<ProgressState>,
}

struct ProgressState {
    /// Bytes of files finished earlier in this install.
    finished: u64,
    /// Bytes of the file currently transferring (including a resumed partial).
    current: u64,
    total: u64,
    phase: InstallPhase,
}

impl<'a> Progress<'a> {
    fn new(total: u64, events: InstallEvents<'a>) -> Self {
        Self {
            events,
            state: Mutex::new(ProgressState {
                finished: 0,
                current: 0,
                total,
                phase: InstallPhase::Checking,
            }),
        }
    }
    fn emit(&self, s: &ProgressState) {
        (self.events)(InstallEvent {
            phase: s.phase,
            done: (s.finished + s.current).min(s.total),
            total: s.total,
        });
    }
    fn file(&self, phase: DownloadPhase, done: u64) {
        let mut s = self.state.lock();
        match phase {
            DownloadPhase::Transferring => {
                s.phase = InstallPhase::Downloading;
                s.current = done;
            }
            DownloadPhase::Verifying => s.phase = InstallPhase::Verifying,
        }
        self.emit(&s);
    }
    fn finish_file(&self, bytes: u64) {
        let mut s = self.state.lock();
        s.finished += bytes;
        s.current = 0;
        self.emit(&s);
    }
    fn phase(&self, phase: InstallPhase) {
        let mut s = self.state.lock();
        s.phase = phase;
        if phase == InstallPhase::Ready {
            s.finished = s.total;
            s.current = 0;
        }
        self.emit(&s);
    }
    /// A component was added mid-install (processor runtime after a graphics-card failure).
    fn grow(&self, bytes: u64) {
        self.state.lock().total += bytes;
    }
}

/// Run an async download on the AI runtime while forwarding progress callbacks
/// to the (synchronous) caller's closure on the calling thread.
fn run_with_progress<T, F, Fut>(
    rt: &AiRuntime,
    make: F,
    progress: &(dyn Fn(DownloadPhase, u64, u64) + Sync),
) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(Arc<dyn Fn(DownloadPhase, u64, u64) + Send + Sync>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = AppResult<T>> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<(DownloadPhase, u64, u64)>();
    let tx = Mutex::new(tx);
    let last = Mutex::new(Instant::now() - Duration::from_secs(1));
    let cb: Arc<dyn Fn(DownloadPhase, u64, u64) + Send + Sync> = Arc::new(move |ph, d, t| {
        // Throttle to ~5 updates/second (always pass completion/phase edges).
        let mut l = last.lock();
        if d == 0 || d == t || l.elapsed() >= Duration::from_millis(200) {
            *l = Instant::now();
            let _ = tx.lock().send((ph, d, t));
        }
    });
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let fut = make(cb);
    rt.handle().spawn(async move {
        let _ = done_tx.send(fut.await);
    });
    loop {
        while let Ok((ph, d, t)) = rx.try_recv() {
            progress(ph, d, t);
        }
        match done_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => {
                while let Ok((ph, d, t)) = rx.try_recv() {
                    progress(ph, d, t);
                }
                return r;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => return Err(AppError::internal("download task stopped unexpectedly")),
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

    #[test]
    fn progress_is_one_combined_figure_across_files() {
        let seen = Mutex::new(Vec::new());
        let events = |e: InstallEvent| seen.lock().push((e.phase, e.done, e.total));
        let p = Progress::new(1_000, &events);
        p.file(DownloadPhase::Transferring, 100); // runtime
        p.file(DownloadPhase::Verifying, 100);
        p.finish_file(100);
        p.file(DownloadPhase::Transferring, 400); // chat model, part way
        p.finish_file(850);
        p.file(DownloadPhase::Transferring, 50); // embedding model
        p.finish_file(50);
        p.phase(InstallPhase::Ready);
        let seen = seen.lock().clone();
        assert!(seen.iter().all(|(_, _, t)| *t == 1_000));
        let dones: Vec<u64> = seen.iter().map(|(_, d, _)| *d).collect();
        assert!(
            dones.windows(2).all(|w| w[0] <= w[1]),
            "never goes backwards: {dones:?}"
        );
        assert_eq!(seen[3], (InstallPhase::Downloading, 500, 1_000));
        assert_eq!(*seen.last().unwrap(), (InstallPhase::Ready, 1_000, 1_000));
    }
}
