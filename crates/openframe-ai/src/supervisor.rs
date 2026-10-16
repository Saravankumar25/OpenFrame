//! Sidecar supervisor for the OpenFrame-managed `llama-server` (Local AI
//! Runtime spec §3, §9).
//!
//! ```text
//!  Unavailable ─configure─► Installed ─start─► Starting ─/health 503─► LoadingModel ─/health 200─► Ready ⇄ Busy
//!      ▲                        ▲  ▲                                                               │
//!      └────── configure(None) ─┘  └──────────── Stopping ◄── stop / idle unload / shutdown ◄──────┘
//! unexpected exit while Ready/Busy → automatic restart (bounded); otherwise → Failed
//! ```
//!
//! Security: loopback only (127.0.0.1, random free port, never 0.0.0.0), a
//! random per-launch API key passed through the environment (not the command
//! line), no web UI, offline mode, no console window. Before every launch the
//! engine files are hashed and compared with the hashes pinned in the bundle lock.
//! The engine is created suspended, put in a kill-on-close Job Object that
//! allows one process, and only then resumed, so it can never outlive OpenFrame
//! or start other programs. Every launched engine is recorded so a later start
//! can stop one left behind by a crash (`engines.rs`).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use openframe_domain::{AppError, AppResult};
use parking_lot::{Condvar, Mutex};
use serde::Serialize;

use crate::bundle::{ModelStamp, Pooling};
use crate::client::{Endpoint, unavailable};
use crate::engines::{EngineFiles, EngineRegistry};
use openframe_platform::process::{ChildGuard, SpawnError, spawn_guarded};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuntimeState {
    /// No engine is configured (the bundled AI is missing, damaged or not checked yet).
    Unavailable,
    Installed,
    Starting,
    LoadingModel,
    Ready,
    Busy,
    Stopping,
    Failed,
}

/// Exactly what to launch. The API key is supplied through `env`.
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
}

/// Builds the launch command for a given loopback port and API key.
pub trait Launcher: Send + Sync {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec;
    /// Model reference recorded with AI requests (profile id, never a file path).
    fn model_reference(&self) -> String;
    /// Check the program files before every launch. The real engine compares their hashes with
    /// the record made at install time; test launchers have nothing to check.
    fn verify(&self) -> AppResult<()> {
        Ok(())
    }
}

/// The bundled llama.cpp `llama-server`: on the processor (`-ngl 0`), or with every layer on one
/// graphics card device of the Vulkan build (ADR-0029; the launch plan in the manager is the only
/// place that chooses).
#[derive(Debug, Clone)]
pub struct LlamaServerLauncher {
    pub executable: PathBuf,
    /// The engine files pinned in the bundle lock, re-hashed before every launch.
    pub engine_files: Option<EngineFiles>,
    pub model_path: PathBuf,
    pub profile_id: String,
    pub context_tokens: u32,
    pub threads: usize,
    /// Fixed sampling seed (deterministic test runs only; `None` in the product).
    pub seed: Option<u32>,
    /// The model file as it was when the bundle check passed, compared before every launch.
    pub model_stamp: Option<Arc<ModelStamp>>,
    /// The Vulkan device (`Vulkan0`, ...) for every layer; `None` runs on the processor.
    pub gpu_device: Option<String>,
}

impl Launcher for LlamaServerLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
        let mut args: Vec<String> = vec![
            "-m".into(),
            self.model_path.to_string_lossy().into_owned(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "-c".into(),
            self.context_tokens.to_string(),
            "-ngl".into(),
            if self.gpu_device.is_some() {
                "999"
            } else {
                "0"
            }
            .into(),
            "-t".into(),
            self.threads.max(1).to_string(),
            "--parallel".into(),
            "1".into(),
            "--no-webui".into(),
            "--no-slots".into(),
            "--offline".into(),
            "--reasoning".into(),
            "off".into(),
            "--alias".into(),
            "openframe-local".into(),
        ];
        if let Some(seed) = self.seed {
            args.push("--seed".into());
            args.push(seed.to_string());
        }
        if let Some(dev) = &self.gpu_device {
            args.push("--device".into());
            args.push(dev.clone());
        }
        LaunchSpec {
            program: self.executable.clone(),
            args,
            env: vec![("LLAMA_API_KEY".into(), api_key.to_string())],
            cwd: self.executable.parent().map(|p| p.to_path_buf()),
        }
    }
    fn model_reference(&self) -> String {
        self.profile_id.clone()
    }
    fn verify(&self) -> AppResult<()> {
        verify_launch(self.engine_files.as_ref(), self.model_stamp.as_deref())
    }
}

/// The same pinned `llama-server` build in embedding-only mode (second managed sidecar).
/// It always runs on the processor (`-ngl 0`): the embedding model is small, and keeping it off
/// the graphics card leaves that memory to the chat model.
#[derive(Debug, Clone)]
pub struct EmbeddingServerLauncher {
    pub executable: PathBuf,
    /// The engine files pinned in the bundle lock, re-hashed before every launch.
    pub engine_files: Option<EngineFiles>,
    pub model_path: PathBuf,
    /// Recorded model reference (the embedding model id, never a file path).
    pub model_id: String,
    pub pooling: Pooling,
    /// The model's maximum sequence length; also the batch size, so one input fits one batch.
    pub context_tokens: u32,
    pub threads: usize,
    /// The model file as it was when the bundle check passed, compared before every launch.
    pub model_stamp: Option<Arc<ModelStamp>>,
}

impl Launcher for EmbeddingServerLauncher {
    fn launch_spec(&self, port: u16, api_key: &str) -> LaunchSpec {
        let ctx = self.context_tokens.clamp(128, 8192).to_string();
        let args = vec![
            "-m".into(),
            self.model_path.to_string_lossy().into_owned(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "--embedding".into(),
            "--pooling".into(),
            self.pooling.as_arg().into(),
            "-c".into(),
            ctx.clone(),
            "-b".into(),
            ctx.clone(),
            "-ub".into(),
            ctx,
            "-ngl".into(),
            "0".into(),
            "-t".into(),
            self.threads.max(1).to_string(),
            "--parallel".into(),
            "1".into(),
            "--no-webui".into(),
            "--no-slots".into(),
            "--offline".into(),
            "--alias".into(),
            "openframe-embedding".into(),
        ];
        LaunchSpec {
            program: self.executable.clone(),
            args,
            env: vec![("LLAMA_API_KEY".into(), api_key.to_string())],
            cwd: self.executable.parent().map(|p| p.to_path_buf()),
        }
    }
    fn model_reference(&self) -> String {
        self.model_id.clone()
    }
    fn verify(&self) -> AppResult<()> {
        verify_launch(self.engine_files.as_ref(), self.model_stamp.as_deref())
    }
}

/// Before every launch: the engine files against their pinned hashes, and the model file against
/// the state it had when the bundle check passed (a model swapped later is never loaded).
fn verify_launch(files: Option<&EngineFiles>, model: Option<&ModelStamp>) -> AppResult<()> {
    match files {
        Some(f) => f.verify()?,
        None => return Err(crate::engines::unverified()),
    }
    match model {
        Some(m) => m.verify(),
        None => Ok(()),
    }
}

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    /// Maximum time from spawn to Ready (model loading included).
    pub start_timeout: Duration,
    /// Unload the model after this much inactivity (None = never).
    pub idle_timeout: Option<Duration>,
    pub poll_interval: Duration,
    /// Automatic restarts allowed within `restart_window` after crashes.
    pub max_restarts: usize,
    pub restart_window: Duration,
    /// Sanitized sidecar log (None = discard output).
    pub log_path: Option<PathBuf>,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            start_timeout: Duration::from_secs(600),
            idle_timeout: Some(Duration::from_secs(15 * 60)),
            poll_interval: Duration::from_millis(400),
            max_restarts: 2,
            restart_window: Duration::from_secs(600),
            log_path: None,
        }
    }
}

pub type StateListener = Arc<dyn Fn(RuntimeState) + Send + Sync>;

/// Launches per start when the chosen loopback port is taken before the runtime binds it.
///
/// The port is picked by binding `127.0.0.1:0` and releasing it, so another program can take
/// it in the moment before `llama-server` binds. The runtime then exits at once; the supervisor
/// launches again on a new random port with a new key, at most this many times in total.
pub const START_BIND_ATTEMPTS: u32 = 3;

/// The only variables the engine keeps from OpenFrame's environment.
const ENGINE_ENV: [&str; 8] = [
    "SystemRoot",
    "windir",
    "SystemDrive",
    "TEMP",
    "TMP",
    "TMPDIR",
    "HOME",
    "USERPROFILE",
];

struct Proc {
    child: Child,
    _job: Option<ChildGuard>,
    /// The pid recorded in the engine registry (removed again when the engine stops).
    recorded: Option<u32>,
    started: Instant,
    port: u16,
}

struct Inner {
    state: RuntimeState,
    launcher: Option<Arc<dyn Launcher>>,
    proc: Option<Proc>,
    endpoint: Option<Endpoint>,
    generation: u64,
    busy: u32,
    last_used: Instant,
    crashes: VecDeque<Instant>,
    last_error: Option<String>,
    /// Relaunches of the current start after a loopback port collision.
    bind_retries: u32,
}

struct Shared {
    inner: Mutex<Inner>,
    cond: Condvar,
    cfg: SupervisorConfig,
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    listener: Mutex<Option<StateListener>>,
    log: Mutex<()>,
    registry: Mutex<Option<Arc<EngineRegistry>>>,
}

/// Owns the sidecar. Dropping the supervisor stops the sidecar.
pub struct Supervisor {
    shared: Arc<Shared>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub state: RuntimeState,
    /// Plain-language reason for Failed.
    pub last_error: Option<String>,
    pub model_reference: Option<String>,
}

impl Supervisor {
    pub fn new(cfg: SupervisorConfig, handle: tokio::runtime::Handle) -> Self {
        let http = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("loopback http client");
        Self {
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner {
                    state: RuntimeState::Unavailable,
                    launcher: None,
                    proc: None,
                    endpoint: None,
                    generation: 0,
                    busy: 0,
                    last_used: Instant::now(),
                    crashes: VecDeque::new(),
                    last_error: None,
                    bind_retries: 0,
                }),
                cond: Condvar::new(),
                cfg,
                handle,
                http,
                listener: Mutex::new(None),
                log: Mutex::new(()),
                registry: Mutex::new(None),
            }),
        }
    }

    /// Called on every state change. Must not call back into the supervisor.
    pub fn set_listener(&self, l: StateListener) {
        *self.shared.listener.lock() = Some(l);
    }

    /// Record every engine this supervisor launches, so a later start can stop one left
    /// behind by a crash.
    pub fn set_registry(&self, registry: Arc<EngineRegistry>) {
        *self.shared.registry.lock() = Some(registry);
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.shared.http
    }

    /// Point the supervisor at a runtime + model (or none). Stops a running sidecar.
    pub fn configure(&self, launcher: Option<Arc<dyn Launcher>>) {
        let mut inner = self.shared.inner.lock();
        stop_locked(&self.shared, &mut inner);
        inner.launcher = launcher;
        inner.last_error = None;
        inner.crashes.clear();
        inner.bind_retries = 0;
        let s = if inner.launcher.is_some() {
            RuntimeState::Installed
        } else {
            RuntimeState::Unavailable
        };
        set_state(&self.shared, &mut inner, s);
    }

    pub fn status(&self) -> RuntimeStatus {
        let inner = self.shared.inner.lock();
        RuntimeStatus {
            state: inner.state,
            last_error: inner.last_error.clone(),
            model_reference: inner.launcher.as_ref().map(|l| l.model_reference()),
        }
    }

    pub fn state(&self) -> RuntimeState {
        self.shared.inner.lock().state
    }

    /// Start the sidecar if it is not running. Returns immediately (loading is
    /// asynchronous; the UI never freezes).
    pub fn start(&self) -> AppResult<()> {
        let mut inner = self.shared.inner.lock();
        match inner.state {
            RuntimeState::Unavailable => Err(no_engine()),
            RuntimeState::Starting
            | RuntimeState::LoadingModel
            | RuntimeState::Ready
            | RuntimeState::Busy => Ok(()),
            RuntimeState::Installed | RuntimeState::Failed | RuntimeState::Stopping => {
                inner.crashes.clear();
                inner.bind_retries = 0;
                spawn_locked(&self.shared, &mut inner)
            }
        }
    }

    /// Wait until Ready (or fail). Busy counts as ready: requests queue in the server.
    pub fn wait_ready(&self, timeout: Duration) -> AppResult<Endpoint> {
        let deadline = Instant::now() + timeout;
        let mut inner = self.shared.inner.lock();
        loop {
            match inner.state {
                RuntimeState::Ready | RuntimeState::Busy => {
                    return inner
                        .endpoint
                        .clone()
                        .ok_or_else(|| unavailable("no endpoint"));
                }
                RuntimeState::Failed => {
                    return Err(unavailable(
                        inner
                            .last_error
                            .clone()
                            .unwrap_or_else(|| "runtime failed".into()),
                    ));
                }
                RuntimeState::Unavailable => return Err(no_engine()),
                RuntimeState::Installed => return Err(unavailable("runtime stopped")),
                _ => {}
            }
            if self
                .shared
                .cond
                .wait_until(&mut inner, deadline)
                .timed_out()
            {
                return Err(AppError::ai(
                    "timeout",
                    "Offline AI is taking too long to get ready. Nothing was changed. Please try again in a moment.",
                )
                .retryable());
            }
        }
    }

    pub fn ensure_ready(&self, timeout: Duration) -> AppResult<Endpoint> {
        self.start()?;
        self.wait_ready(timeout)
    }

    /// Mark a request in flight (Ready → Busy); dropping the guard returns to Ready.
    pub fn begin_request(&self) -> BusyGuard {
        let mut inner = self.shared.inner.lock();
        inner.busy += 1;
        inner.last_used = Instant::now();
        if inner.state == RuntimeState::Ready {
            set_state(&self.shared, &mut inner, RuntimeState::Busy);
        }
        BusyGuard {
            shared: Arc::downgrade(&self.shared),
        }
    }

    /// Stop the sidecar (model unloaded, memory released).
    pub fn stop(&self) {
        let mut inner = self.shared.inner.lock();
        stop_locked(&self.shared, &mut inner);
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct BusyGuard {
    shared: Weak<Shared>,
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        let mut inner = shared.inner.lock();
        inner.busy = inner.busy.saturating_sub(1);
        inner.last_used = Instant::now();
        if inner.busy == 0 && inner.state == RuntimeState::Busy {
            set_state(&shared, &mut inner, RuntimeState::Ready);
        }
    }
}

/// No engine is configured: the bundled AI is missing or damaged (the manager reports which).
pub fn no_engine() -> AppError {
    crate::bundle::missing()
}

fn set_state(shared: &Shared, inner: &mut Inner, s: RuntimeState) {
    if inner.state == s {
        return;
    }
    inner.state = s;
    shared.cond.notify_all();
    if let Some(l) = shared.listener.lock().clone() {
        l(s);
    }
}

fn free_loopback_port() -> AppResult<u16> {
    let l = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(l.local_addr()?.port())
}

/// After our runtime exited during start: is another program now listening on the loopback
/// port it was given? Our process is gone, so an accepted connection means someone else holds
/// the port (the free-port race). Only a TCP handshake on 127.0.0.1; nothing is sent.
fn loopback_port_in_use(port: u16) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

fn random_key() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

/// `llama-server --list-devices` of a pinned engine (its files checked first), with the same
/// scrubbed environment and kill-on-close guard as a launch. Returns what it printed.
pub fn list_devices(program: &std::path::Path, files: Option<&EngineFiles>) -> AppResult<String> {
    if let Some(f) = files {
        f.verify()?;
    }
    let mut cmd = Command::new(program);
    cmd.arg("--list-devices")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd.env_clear();
    for k in ENGINE_ENV {
        if let Some(v) = std::env::var_os(k) {
            cmd.env(k, v);
        }
    }
    if let Some(dir) = program.parent() {
        cmd.current_dir(dir);
    }
    let (mut child, _guard) = spawn_guarded(&mut cmd)
        .map_err(|e| unavailable(format!("the engine could not list devices: {e:?}")))?;
    let mut out = child.stdout.take();
    let reader = std::thread::spawn(move || {
        // Adapter names can be in the system code page, so bytes that are not UTF-8 are kept.
        let mut b = Vec::new();
        if let Some(o) = out.as_mut() {
            use std::io::Read;
            let _ = o.take(64 * 1024).read_to_end(&mut b);
        }
        String::from_utf8_lossy(&b).into_owned()
    });
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(unavailable("listing devices took too long"));
            }
        }
    }
    reader
        .join()
        .map_err(|_| unavailable("device listing failed"))
}

fn spawn_locked(shared: &Arc<Shared>, inner: &mut Inner) -> AppResult<()> {
    let launcher = inner.launcher.clone().ok_or_else(no_engine)?;
    if let Err(e) = launcher.verify() {
        inner.last_error = Some(e.message.clone());
        set_state(shared, inner, RuntimeState::Failed);
        return Err(e);
    }
    let port = free_loopback_port()?;
    let key = random_key();
    let spec = launcher.launch_spec(port, &key);
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The engine starts with an empty environment plus the few variables a process needs to
    // start. Nothing that configures llama.cpp, ggml or OpenMP (`LLAMA_*`, `GGML_*` such as a
    // backend search path, `OMP_*`, `KMP_*`) can reach it, and there is no PATH: it finds its own
    // libraries next to itself and the system ones in the system folders.
    cmd.env_clear();
    for k in ENGINE_ENV {
        if let Some(v) = std::env::var_os(k) {
            cmd.env(k, v);
        }
    }
    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    let (mut child, job) = match spawn_guarded(&mut cmd) {
        Ok(c) => c,
        // Masterplan process safety: the sidecar must die with the app and may not start other
        // programs. Where the operating system enforces that (Windows Job Object), the child was
        // stopped before it ran and OpenFrame refuses to run it.
        Err(SpawnError::Unguarded | SpawnError::Resume) => {
            inner.last_error = Some("Offline AI could not start safely on this computer.".into());
            set_state(shared, inner, RuntimeState::Failed);
            return Err(unavailable("kill-on-close job unavailable"));
        }
        Err(SpawnError::Spawn(e)) => {
            inner.last_error = Some("Offline AI could not start on this computer.".into());
            set_state(shared, inner, RuntimeState::Failed);
            return Err(unavailable(format!("spawn failed: {e}")));
        }
    };
    if job.is_none() {
        // macOS: the guard is best effort; the explicit stop and the startup sweep still run.
        tracing::warn!("offline AI runtime could not be placed in a kill-on-close guard");
    }
    let registry = shared.registry.lock().clone();
    let recorded = registry.and_then(|r| r.record(&child, &spec.program));
    if let Some(out) = child.stdout.take() {
        pipe_log(shared, out);
    }
    if let Some(err) = child.stderr.take() {
        pipe_log(shared, err);
    }
    inner.generation += 1;
    inner.proc = Some(Proc {
        child,
        _job: job,
        recorded,
        started: Instant::now(),
        port,
    });
    inner.endpoint = Some(Endpoint {
        base_url: format!("http://127.0.0.1:{port}"),
        api_key: key,
    });
    inner.last_error = None;
    set_state(shared, inner, RuntimeState::Starting);
    let generation = inner.generation;
    let weak = Arc::downgrade(shared);
    std::thread::Builder::new()
        .name("of-ai-supervisor".into())
        .spawn(move || monitor(weak, generation))
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(())
}

/// Remove a stopped or exited engine from the registry.
fn forget(shared: &Shared, p: &Proc) {
    if let (Some(pid), Some(r)) = (p.recorded, shared.registry.lock().clone()) {
        r.forget(pid);
    }
}

/// Launch again after an exit; a launch that cannot even begin ends in Failed (never a state
/// that claims to be starting or serving without a process).
fn relaunch(shared: &Arc<Shared>, inner: &mut Inner) {
    if spawn_locked(shared, inner).is_err() {
        tracing::error!("offline AI runtime could not be restarted");
        if inner.state != RuntimeState::Failed {
            inner.last_error = Some("Offline AI could not start on this computer.".into());
            set_state(shared, inner, RuntimeState::Failed);
        }
    }
}

fn stop_locked(shared: &Shared, inner: &mut Inner) {
    if let Some(mut p) = inner.proc.take() {
        set_state(shared, inner, RuntimeState::Stopping);
        inner.generation += 1;
        let _ = p.child.kill();
        let _ = p.child.wait();
        forget(shared, &p);
    }
    inner.endpoint = None;
    inner.busy = 0;
    let s = if inner.launcher.is_some() {
        RuntimeState::Installed
    } else {
        RuntimeState::Unavailable
    };
    // A Failed runtime stays Failed until explicitly restarted/reconfigured.
    if inner.state != RuntimeState::Failed || inner.launcher.is_none() {
        set_state(shared, inner, s);
    }
}

enum Health {
    Ok,
    Loading,
    Down,
}

async fn probe(http: reqwest::Client, url: String) -> AppResult<Health> {
    let r = http.get(url).timeout(Duration::from_secs(2)).send().await;
    Ok(match r {
        Ok(resp) if resp.status().is_success() => Health::Ok,
        Ok(resp) if resp.status().as_u16() == 503 => Health::Loading,
        _ => Health::Down,
    })
}

fn monitor(weak: Weak<Shared>, generation: u64) {
    loop {
        let interval = match weak.upgrade() {
            Some(s) => s.cfg.poll_interval,
            None => return,
        };
        std::thread::sleep(interval);
        let Some(shared) = weak.upgrade() else { return };
        let mut inner = shared.inner.lock();
        if inner.generation != generation {
            return;
        }
        // 1. Did the process exit?
        let exited = match inner.proc.as_mut() {
            Some(p) => matches!(p.child.try_wait(), Ok(Some(_)) | Err(_)),
            None => return,
        };
        if exited {
            let was_serving = matches!(inner.state, RuntimeState::Ready | RuntimeState::Busy);
            // Free-port race: the runtime exited before it ever served because another
            // program took its loopback port. Launch again on a new port (bounded).
            let start_port = if !was_serving && inner.bind_retries + 1 < START_BIND_ATTEMPTS {
                inner.proc.as_ref().map(|p| p.port)
            } else {
                None
            };
            if let Some(port) = start_port {
                drop(inner);
                let collided = loopback_port_in_use(port);
                inner = shared.inner.lock();
                if inner.generation != generation {
                    return;
                }
                if collided {
                    if let Some(p) = inner.proc.take() {
                        forget(&shared, &p);
                    }
                    inner.endpoint = None;
                    inner.bind_retries += 1;
                    tracing::warn!(
                        "offline AI runtime could not use its loopback port; starting again on a new port"
                    );
                    relaunch(&shared, &mut inner);
                    return;
                }
            }
            if let Some(p) = inner.proc.take() {
                forget(&shared, &p);
            }
            inner.endpoint = None;
            inner.busy = 0;
            let now = Instant::now();
            let window = shared.cfg.restart_window;
            inner.crashes.push_back(now);
            while inner
                .crashes
                .front()
                .is_some_and(|t| now.duration_since(*t) > window)
            {
                inner.crashes.pop_front();
            }
            if was_serving && inner.crashes.len() <= shared.cfg.max_restarts {
                tracing::warn!("offline AI runtime exited unexpectedly; restarting");
                relaunch(&shared, &mut inner);
            } else {
                tracing::error!(
                    "offline AI runtime exited and will not be restarted automatically"
                );
                inner.last_error = Some(if was_serving {
                    "Offline AI stopped unexpectedly. Your project was not changed.".into()
                } else {
                    "Offline AI could not start on this computer.".into()
                });
                set_state(&shared, &mut inner, RuntimeState::Failed);
            }
            return;
        }
        match inner.state {
            RuntimeState::Starting | RuntimeState::LoadingModel => {
                let started = inner
                    .proc
                    .as_ref()
                    .map(|p| p.started)
                    .unwrap_or_else(Instant::now);
                if started.elapsed() > shared.cfg.start_timeout {
                    if let Some(mut p) = inner.proc.take() {
                        let _ = p.child.kill();
                        let _ = p.child.wait();
                        forget(&shared, &p);
                    }
                    inner.endpoint = None;
                    inner.last_error =
                        Some("Offline AI took too long to start on this computer.".into());
                    set_state(&shared, &mut inner, RuntimeState::Failed);
                    return;
                }
                let Some(ep) = inner.endpoint.clone() else {
                    return;
                };
                drop(inner);
                let health = {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let http = shared.http.clone();
                    shared.handle.spawn(async move {
                        let _ = tx.send(probe(http, format!("{}/health", ep.base_url)).await);
                    });
                    rx.recv_timeout(Duration::from_secs(5))
                        .ok()
                        .and_then(|r| r.ok())
                        .unwrap_or(Health::Down)
                };
                let mut inner = shared.inner.lock();
                if inner.generation != generation {
                    return;
                }
                // Only our own live process may make the runtime Ready: if it exited while the
                // probe ran, whatever answered is not ours. The next round handles the exit.
                let alive = inner
                    .proc
                    .as_mut()
                    .is_some_and(|p| matches!(p.child.try_wait(), Ok(None)));
                if !alive {
                    continue;
                }
                match health {
                    Health::Ok => {
                        inner.last_used = Instant::now();
                        inner.bind_retries = 0;
                        set_state(&shared, &mut inner, RuntimeState::Ready);
                    }
                    Health::Loading => set_state(&shared, &mut inner, RuntimeState::LoadingModel),
                    Health::Down => {}
                }
            }
            RuntimeState::Ready => {
                if let Some(idle) = shared.cfg.idle_timeout
                    && inner.busy == 0
                    && inner.last_used.elapsed() > idle
                {
                    tracing::info!("unloading offline AI model after inactivity");
                    stop_locked(&shared, &mut inner);
                    return;
                }
            }
            _ => {}
        }
    }
}

/// Copy sidecar output into the sanitized log (never the UI). Lines are redacted (home
/// paths, e-mail addresses, keys, prompt and content text, absolute paths outside the app's
/// folders) and truncated; the log rotates at 4 MB.
fn pipe_log<R: std::io::Read + Send + 'static>(shared: &Arc<Shared>, stream: R) {
    let weak = Arc::downgrade(shared);
    let _ = std::thread::Builder::new()
        .name("of-ai-log".into())
        .spawn(move || {
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let Some(shared) = weak.upgrade() else { break };
                let Some(path) = shared.cfg.log_path.clone() else {
                    continue;
                };
                let lower = line.to_ascii_lowercase();
                if lower.contains("api_key")
                    || lower.contains("api-key")
                    || lower.contains("authorization")
                {
                    continue;
                }
                let mut clean = openframe_security::redact_process_output(&line);
                if clean.len() > 400 {
                    let mut cut = 400;
                    while !clean.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    clean.truncate(cut);
                }
                let _g = shared.log.lock();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::metadata(&path)
                    .map(|m| m.len() > 4 * 1024 * 1024)
                    .unwrap_or(false)
                {
                    let _ = std::fs::rename(&path, path.with_extension("log.1"));
                }
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                {
                    let _ = writeln!(f, "{clean}");
                }
            }
        });
}
