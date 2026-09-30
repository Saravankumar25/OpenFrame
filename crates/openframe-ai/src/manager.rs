//! Offline AI façade used by the application layer: manifest trust, hardware
//! check, recommendation, verified installation with atomic activation and
//! rollback, model removal, and the chat model backed by the managed sidecar.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_domain::{AppError, AppResult, now_ms};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;

use crate::client::{ChatMessage, ChatRequest};
use crate::download::{self, Control, DownloadOptions, DownloadPhase, DownloadSpec};
use crate::hardware::{self, HardwareInfo};
use crate::manifest::{self, Backend, Manifest, ModelEntry, RuntimeEntry};
use crate::model::{ChatModel, LocalRuntimeModel};
use crate::rt::AiRuntime;
use crate::selection::{self, Recommendation};
use crate::store::{ActiveSelection, AiPaths, ModelMetadata, write_json_atomic};
use crate::supervisor::{
    Launcher, LlamaServerLauncher, RuntimeStatus, StateListener, Supervisor, SupervisorConfig,
};

/// Upper bounds for the distribution manifest and its signature (bounded reads).
const MAX_MANIFEST_BYTES: usize = 1 << 20;
const MAX_SIGNATURE_BYTES: usize = 4 << 10;

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
    pub supervisor: SupervisorConfig,
    /// How long installation waits for the new model to answer its health check.
    pub health_timeout: Duration,
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
            health_timeout: Duration::from_secs(600),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InstallPhase {
    Checking,
    DownloadingRuntime,
    DownloadingModel,
    Verifying,
    Installing,
    Starting,
    Ready,
}

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
    pub runtime_id: String,
    pub backend: Backend,
}

pub type InstallEvents<'a> = &'a (dyn Fn(InstallEvent) + Send + Sync);

pub struct AiManager {
    cfg: ManagerConfig,
    paths: AiPaths,
    rt: Arc<AiRuntime>,
    http: reqwest::Client,
    supervisor: Arc<Supervisor>,
    manifest: RwLock<Option<Manifest>>,
    hw_cache: Mutex<Option<(Instant, HardwareInfo)>>,
    install_lock: Mutex<()>,
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
        let m = Self {
            paths: AiPaths::new(&cfg.app_data_dir),
            cfg,
            rt,
            http,
            supervisor,
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

    pub fn set_state_listener(&self, l: StateListener) {
        self.supervisor.set_listener(l);
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        self.supervisor.status()
    }

    /// Begin loading the model in the background (no-op when not installed).
    pub fn warm_up(&self) -> AppResult<()> {
        self.supervisor.start()
    }

    pub fn stop_runtime(&self) {
        self.supervisor.stop();
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
                    // A tampered cache is never used; remove it so it can't be retried.
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

    pub fn recommend(&self) -> AppResult<Recommendation> {
        let m = self.manifest()?;
        selection::recommend(&self.hardware(false), &m).ok_or_else(|| {
            AppError::ai(
                "unsupported",
                "Offline AI isn't available for this kind of computer yet.",
            )
        })
    }

    // --------------------------------------------------------------- status

    pub fn active(&self) -> Option<ActiveSelection> {
        self.paths.active()
    }

    pub fn installed_models(&self) -> Vec<(ModelMetadata, u64)> {
        self.paths.installed_profiles()
    }

    /// Bytes already downloaded for a profile (resumable).
    pub fn partial_bytes(&self, profile_id: &str) -> u64 {
        let Ok(m) = self.manifest() else { return 0 };
        match m.model(profile_id) {
            Some(e) => download::partial_len(&self.model_spec(e)),
            None => 0,
        }
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

    fn launcher(
        &self,
        runtime: &RuntimeEntry,
        model: &ModelMetadata,
        hw: &HardwareInfo,
    ) -> Arc<dyn Launcher> {
        let entry_like = ModelEntry {
            profile_id: model.profile_id.clone(),
            tier: model.tier,
            display_name: model.display_name.clone(),
            engine: "llama.cpp".into(),
            architecture: String::new(),
            bytes: model.bytes,
            sha256: String::new(),
            url: String::new(),
            min_ram_bytes: 0,
            recommended_ram_bytes: 0,
            gpu_vram_bytes: model.gpu_vram_bytes.max(1),
            context_tokens: model.context_tokens,
            license_id: model.license_id.clone(),
        };
        let threads = hw.physical_cpus.unwrap_or(hw.logical_cpus / 2).clamp(1, 16);
        Arc::new(LlamaServerLauncher {
            executable: self.paths.runtime_executable(runtime),
            model_path: self.paths.model_file(&model.profile_id),
            profile_id: model.profile_id.clone(),
            context_tokens: model.context_tokens.clamp(2048, 32768),
            gpu_layers: selection::gpu_layers(hw, &entry_like, runtime.backend),
            threads,
        })
    }

    /// Point the supervisor at the active model/runtime recorded on disk.
    fn configure_from_active(&self) {
        let launcher = self.paths.active().and_then(|a| {
            let rt = self.paths.installed_runtime(&a.runtime_id)?;
            let md = self.paths.model_metadata(&a.profile_id)?;
            let size = std::fs::metadata(self.paths.model_file(&a.profile_id))
                .ok()?
                .len();
            (size == md.bytes).then(|| self.launcher(&rt, &md, &self.hardware(false)))
        });
        self.supervisor.configure(launcher);
    }

    // --------------------------------------------------------------- install

    fn model_spec(&self, e: &ModelEntry) -> DownloadSpec {
        DownloadSpec {
            url: e.url.clone(),
            bytes: e.bytes,
            sha256: e.sha256.clone(),
            partial: self.paths.model_partial(&e.profile_id),
            dest: self.paths.model_file(&e.profile_id),
            quarantine_dir: self.paths.quarantine_dir(),
            label: e.profile_id.clone(),
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

    /// Download, verify and activate a profile (the recommended one when None).
    /// Blocking; call from a background task. The previously active model stays
    /// active until the new one passes its health check.
    pub fn install(
        &self,
        profile_id: Option<&str>,
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
        let rec = selection::recommend(&hw, &manifest).ok_or_else(|| {
            AppError::ai(
                "unsupported",
                "Offline AI isn't available for this kind of computer yet.",
            )
        })?;
        let model = match profile_id {
            Some(p) => manifest
                .model(p)
                .ok_or_else(|| AppError::not_found("AI profile"))?,
            None => manifest
                .model(&rec.profile_id)
                .ok_or_else(|| AppError::not_found("AI profile"))?,
        }
        .clone();
        let runtime = manifest
            .runtime_for(rec.backend)
            .or_else(|| manifest.runtime_for(Backend::Cpu))
            .ok_or_else(|| {
                AppError::ai(
                    "unsupported",
                    "Offline AI isn't available for this kind of computer yet.",
                )
            })?
            .clone();

        // Disk-space preflight (nothing is downloaded when space is short).
        let model_spec = self.model_spec(&model);
        let model_needed = if self.paths.model_installed(&model) {
            0
        } else {
            model.bytes - download::partial_len(&model_spec)
        };
        let runtime_needed = if self.paths.runtime_installed(&runtime) {
            0
        } else {
            runtime.bytes - download::partial_len(&self.runtime_spec(&runtime))
        };
        std::fs::create_dir_all(self.paths.models_dir())?;
        download::check_disk_space(
            &self.paths.models_dir(),
            selection::required_free_bytes(model_needed, runtime_needed),
        )?;

        if !self.paths.runtime_installed(&runtime) {
            self.install_runtime(&runtime, control, events)?;
        }
        if !self.paths.model_installed(&model) {
            self.download_model(&model, control, events)?;
        }

        // Health check, then atomic activation (active.json switch).
        control.check()?;
        events(InstallEvent {
            phase: InstallPhase::Starting,
            done: 0,
            total: 0,
        });
        let md = self
            .paths
            .model_metadata(&model.profile_id)
            .ok_or_else(|| AppError::internal("model metadata missing"))?;
        let first = self.try_activate(&runtime, &md, &hw);
        let used_runtime = match first {
            Ok(()) => runtime.clone(),
            Err(e) if runtime.backend != Backend::Cpu => {
                // Graphics driver trouble: fall back to the processor build.
                tracing::warn!(
                    code = e.code_str(),
                    "graphics-card runtime failed its health check; trying the processor runtime"
                );
                let cpu = manifest.runtime_for(Backend::Cpu).cloned().ok_or(e)?;
                if !self.paths.runtime_installed(&cpu) {
                    self.install_runtime(&cpu, control, events)?;
                }
                self.try_activate(&cpu, &md, &hw)
                    .map_err(|e| self.rollback(&model.profile_id, e))?;
                cpu
            }
            Err(e) => return Err(self.rollback(&model.profile_id, e)),
        };
        let previous = self
            .paths
            .active()
            .map(|a| a.profile_id)
            .filter(|p| p != &model.profile_id);
        self.paths.set_active(Some(&ActiveSelection {
            profile_id: model.profile_id.clone(),
            runtime_id: used_runtime.runtime_id.clone(),
            activated_at: now_ms(),
            previous_profile_id: previous,
        }))?;
        self.paths.mark_health(&model.profile_id, false)?;
        events(InstallEvent {
            phase: InstallPhase::Ready,
            done: 1,
            total: 1,
        });
        Ok(InstallOutcome {
            profile_id: model.profile_id,
            runtime_id: used_runtime.runtime_id,
            backend: used_runtime.backend,
        })
    }

    /// Start the runtime with the candidate model and require one real answer.
    fn try_activate(
        &self,
        runtime: &RuntimeEntry,
        md: &ModelMetadata,
        hw: &HardwareInfo,
    ) -> AppResult<()> {
        self.supervisor
            .configure(Some(self.launcher(runtime, md, hw)));
        let ep = self.supervisor.ensure_ready(self.cfg.health_timeout)?;
        let _busy = self.supervisor.begin_request();
        let http = self.supervisor.http().clone();
        let mut req =
            ChatRequest::new(vec![ChatMessage::user("Reply with the single word: ready")])
                .max_tokens(8);
        req.timeout = Duration::from_secs(120);
        self.rt
            .run(async move { crate::client::chat(&http, &ep, &req).await })
            .map(|_| ())
    }

    /// The candidate failed: keep the previous setup active and explain.
    fn rollback(&self, profile_id: &str, cause: AppError) -> AppError {
        let _ = self.paths.mark_health(profile_id, true);
        self.configure_from_active();
        AppError::ai(
            "start_failed",
            "Offline AI was downloaded and verified, but it could not start on this computer. Your previous setup was kept. You can retry or choose a lighter profile.",
        )
        .with_detail(format!("{}: {}", cause.code_str(), cause.detail.clone().unwrap_or_default()))
        .retryable()
    }

    fn download_model(
        &self,
        model: &ModelEntry,
        control: &Control,
        events: InstallEvents<'_>,
    ) -> AppResult<()> {
        let spec = self.model_spec(model);
        // A leftover directory without valid integrity is replaced (never the active one mid-run:
        // the active model is always a verified, installed one).
        let dir = self.paths.model_dir(&model.profile_id);
        if dir.exists() && !self.paths.model_installed(model) {
            if self
                .paths
                .active()
                .is_some_and(|a| a.profile_id == model.profile_id)
            {
                self.supervisor.configure(None);
            }
            std::fs::remove_dir_all(&dir)?;
        }
        let progress = |phase: DownloadPhase, done: u64, total: u64| {
            let phase = match phase {
                DownloadPhase::Transferring => InstallPhase::DownloadingModel,
                DownloadPhase::Verifying => InstallPhase::Verifying,
            };
            events(InstallEvent { phase, done, total });
        };
        let http = self.http.clone();
        let opts = self.cfg.download.clone();
        let control2 = control.clone();
        let spec2 = spec.clone();
        run_with_progress(
            &self.rt,
            move |p| async move {
                download::download_verified(&http, &spec2, &opts, &control2, &*p).await
            },
            &progress,
        )?;
        self.paths.record_model(model)?;
        Ok(())
    }

    fn install_runtime(
        &self,
        runtime: &RuntimeEntry,
        control: &Control,
        events: InstallEvents<'_>,
    ) -> AppResult<()> {
        let spec = self.runtime_spec(runtime);
        let progress = |phase: DownloadPhase, done: u64, total: u64| {
            let phase = match phase {
                DownloadPhase::Transferring => InstallPhase::DownloadingRuntime,
                DownloadPhase::Verifying => InstallPhase::Verifying,
            };
            events(InstallEvent { phase, done, total });
        };
        let http = self.http.clone();
        let opts = self.cfg.download.clone();
        let control2 = control.clone();
        let spec2 = spec.clone();
        let zip = run_with_progress(
            &self.rt,
            move |p| async move {
                download::download_verified(&http, &spec2, &opts, &control2, &*p).await
            },
            &progress,
        )?;
        events(InstallEvent {
            phase: InstallPhase::Installing,
            done: 0,
            total: 0,
        });
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
            }
            std::fs::remove_dir_all(&final_dir)?;
        }
        std::fs::rename(&tmp, &final_dir)?;
        let _ = std::fs::remove_file(&zip);
        Ok(())
    }

    /// Delete an installed model. Removing the active model turns Offline AI off
    /// (the rest of OpenFrame is unaffected).
    pub fn remove_model(&self, profile_id: &str) -> AppResult<()> {
        let _guard = self.install_lock.try_lock().ok_or_else(|| {
            AppError::ai(
                "install_running",
                "Please wait until the current Offline AI download finishes or is cancelled.",
            )
        })?;
        // The id becomes a directory that is deleted recursively: it must be a single safe
        // component (on Windows `models_dir.join("C:")` would be the current directory of C:).
        if !manifest::safe_id(profile_id) || profile_id.contains(':') {
            return Err(AppError::invalid_input("That AI profile isn't valid."));
        }
        let dir = self.paths.model_dir(profile_id);
        if dir.parent() != Some(self.paths.models_dir().as_path()) {
            return Err(AppError::invalid_input("That AI profile isn't valid."));
        }
        if !dir.is_dir() {
            return Err(AppError::not_found("AI model"));
        }
        if self
            .paths
            .active()
            .is_some_and(|a| a.profile_id == profile_id)
        {
            self.supervisor.configure(None);
            self.paths.set_active(None)?;
        }
        std::fs::remove_dir_all(&dir)?;
        download::discard_partial(&self.paths.model_partial(profile_id));
        self.configure_from_active();
        Ok(())
    }

    /// Throw away partially downloaded files (the "Cancel" choice; "Pause" keeps them).
    pub fn discard_partials(&self) {
        let Ok(m) = self.manifest() else { return };
        for e in &m.models {
            download::discard_partial(&self.paths.model_partial(&e.profile_id));
        }
        for r in &m.runtimes {
            download::discard_partial(&self.paths.runtime_partial(&r.runtime_id));
        }
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
