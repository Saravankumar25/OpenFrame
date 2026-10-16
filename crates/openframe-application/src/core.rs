//! The application core: owns the app database, the open project session, the
//! Global Idea Vault store, the operation registry, background tasks and
//! module services. The Tauri layer is only an adapter around this type.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Role, new_id, now_ms};
use openframe_entitlement::Channel;
use openframe_entitlement::mac::MacInputs;
use openframe_project_format::{ProjectLayout, ProjectLock, ProjectManifest};
use parking_lot::{Mutex, RwLock};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::events::{AppEvent, EventSink, StoreKind, TaskProgress};
use crate::modules::entitlement::EntitlementService;
use crate::registry::Registry;
use crate::store::{SaveTracker, Store};

/// Not `Clone`: it can carry the macOS shell's StoreKit bridge and licence server client, which are
/// handed to the entitlement service exactly once.
pub struct AppConfig {
    /// `%LOCALAPPDATA%/OpenFrame`: settings, recents, AI records, logs.
    pub app_data_dir: PathBuf,
    /// The read-only Offline AI engine (`<app resources>/ai`, ADR-0026), named by the platform
    /// adapter. `None` when the platform cannot name it and in tests: AI then reports itself
    /// missing and everything else works.
    pub ai_bundle_dir: Option<PathBuf>,
    /// The model host and manifest keys the desktop build was made with (ADR-0026 decision 8).
    /// `None` in tests and in a build without them: OpenFrame AI cannot be downloaded then.
    pub ai_model_host: Option<openframe_ai::ModelHost>,
    /// Temperature 0 and a fixed seed for the Offline AI. Only the end-to-end suite turns it on
    /// (a debug build on an isolated test data root); never in a release build.
    pub ai_deterministic: bool,
    /// Default parent folder for new projects.
    pub projects_dir: PathBuf,
    /// Global Idea Vault folder (user content).
    pub global_vault_dir: PathBuf,
    pub app_version: String,
    /// Release channel for the licence and trial check. The desktop shell sets it; anything that
    /// is not a store package is an internal team build (ADR-0023, ADR-0024).
    pub channel: Channel,
    /// The Mac App Store channel's StoreKit bridge, licence server client, nonce source and
    /// clock, supplied by the macOS desktop shell. `None` on Windows and in tests: the Mac channel
    /// then runs over the "unavailable" bridge and never grants anything. When set, the first
    /// check waits for the shell's background revalidation (see `EntitlementService::start`).
    pub mac_inputs: Option<MacInputs>,
    /// The Microsoft Store calls (`GetAppLicenseAsync`, `RequestPurchaseAsync`) supplied by the
    /// Windows shell for the Microsoft Store channel (ADR-0023). `None` until the Partner Center
    /// product identity exists: the Microsoft Store channel then reports NeedsOnlineCheck and
    /// cannot sell, a gap in the build rather than a user state. Other channels ignore it.
    pub microsoft_store: Option<Box<dyn openframe_entitlement::adapters::StoreLicenceSource>>,
    /// Development and end-to-end builds only: registers `entitlement.debug_set`, which sets the
    /// licence state and channel by hand. The desktop shell turns it on only in a debug build with
    /// an isolated test data root; release builds never do (tested in the shell).
    pub entitlement_debug_controls: bool,
    /// The internal builds' team password hash (Argon2id PHC string, ADR-0024), embedded by the
    /// desktop build script from `OPENFRAME_TEAM_PASSWORD` on the build machine. When set, nothing
    /// but the password screen runs until the password is right. `None` in store packages, which
    /// carry no gate and no hash, and in tests.
    pub team_password_hash: Option<String>,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("app_data_dir", &self.app_data_dir)
            .field("ai_bundle_dir", &self.ai_bundle_dir)
            .field(
                "ai_model_host",
                &self.ai_model_host.as_ref().map(|h| &h.base),
            )
            .field("ai_deterministic", &self.ai_deterministic)
            .field("projects_dir", &self.projects_dir)
            .field("global_vault_dir", &self.global_vault_dir)
            .field("app_version", &self.app_version)
            .field("channel", &self.channel)
            .field("mac_inputs", &self.mac_inputs.is_some())
            .field("microsoft_store", &self.microsoft_store.is_some())
            .field(
                "entitlement_debug_controls",
                &self.entitlement_debug_controls,
            )
            .field("team_password_hash", &self.team_password_hash.is_some())
            .finish()
    }
}

impl AppConfig {
    /// The configuration for the person signed in to this computer. Refuses (Windows phase W2)
    /// when the system cannot name Documents or the app data folder, instead of keeping projects
    /// and settings in a temp folder the system cleans up.
    pub fn for_current_user(app_version: &str) -> AppResult<Self> {
        Self::from_system_folders(
            openframe_project_format::app_data_dir(),
            openframe_project_format::default_projects_dir(),
            openframe_project_format::default_global_vault_dir(),
            app_version,
        )
    }

    /// [`AppConfig::for_current_user`] with the folders given (tests).
    pub fn from_system_folders(
        app_data_dir: Option<PathBuf>,
        projects_dir: Option<PathBuf>,
        global_vault_dir: Option<PathBuf>,
        app_version: &str,
    ) -> AppResult<Self> {
        let (Some(projects_dir), Some(global_vault_dir)) = (projects_dir, global_vault_dir) else {
            return Err(AppError::new(
                "storage.documents_unknown",
                "OpenFrame Studio can't find your Documents folder, so it has no safe place to keep your \
                 projects. Open File Explorer and check that Documents opens, then start OpenFrame Studio again.",
            ));
        };
        let Some(app_data_dir) = app_data_dir else {
            return Err(AppError::new(
                "storage.app_data_unknown",
                "OpenFrame Studio can't find its settings folder on this computer, so it can't start. \
                 Restart the computer, then start OpenFrame Studio again.",
            ));
        };
        Ok(Self {
            app_data_dir,
            ai_bundle_dir: Self::installed_ai_bundle_dir(),
            ai_model_host: None,
            ai_deterministic: false,
            projects_dir,
            global_vault_dir,
            app_version: app_version.to_string(),
            channel: Channel::DirectInstaller,
            mac_inputs: None,
            microsoft_store: None,
            entitlement_debug_controls: false,
            team_password_hash: None,
        })
    }

    /// Where this installation keeps the Offline AI engine: the `ai` folder of the app
    /// resources, named by the platform adapter (next to the executable on Windows, including
    /// MSIX; `Contents/Resources` in a Mac app bundle).
    pub fn installed_ai_bundle_dir() -> Option<PathBuf> {
        openframe_platform::dirs::app_resources_dir().map(|d| d.join(openframe_ai::RESOURCE_FOLDER))
    }

    /// The one folder to back up, shown on the data note and in the file safety tutorial: the
    /// folder that holds both the projects folder and the Global Idea Vault
    /// (`Documents/OpenFrame` by default), or the projects folder when they live apart.
    pub fn data_root(&self) -> PathBuf {
        match (self.projects_dir.parent(), self.global_vault_dir.parent()) {
            (Some(a), Some(b)) if a == b && !a.as_os_str().is_empty() => a.to_path_buf(),
            _ => self.projects_dir.clone(),
        }
    }

    /// Fully isolated configuration rooted in `root` (tests, fixtures).
    pub fn isolated(root: &std::path::Path) -> Self {
        Self {
            app_data_dir: root.join("appdata"),
            ai_bundle_dir: None,
            ai_model_host: None,
            ai_deterministic: false,
            projects_dir: root.join("projects"),
            global_vault_dir: root.join("global-vault"),
            app_version: "0.1.0-test".to_string(),
            channel: Channel::DirectInstaller,
            mac_inputs: None,
            microsoft_store: None,
            entitlement_debug_controls: false,
            team_password_hash: None,
        }
    }
}

/// Offered after an abnormal shutdown (FSD §44.3–44.4, mockup 016).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryOffer {
    #[ts(type = "number")]
    pub previous_session_started_at: i64,
    #[ts(type = "number | null")]
    pub last_autosave_at: Option<i64>,
    pub has_checkpoint: bool,
    #[ts(type = "number | null")]
    pub checkpoint_at: Option<i64>,
}

pub struct ProjectSession {
    pub layout: ProjectLayout,
    pub store: Store,
    pub manifest: Mutex<ProjectManifest>,
    pub recovery: Mutex<Option<RecoveryOffer>>,
    pub opened_at: i64,
    _lock: ProjectLock,
}

impl ProjectSession {
    pub fn new(
        layout: ProjectLayout,
        store: Store,
        manifest: ProjectManifest,
        lock: ProjectLock,
        recovery: Option<RecoveryOffer>,
    ) -> Self {
        Self {
            layout,
            store,
            manifest: Mutex::new(manifest),
            recovery: Mutex::new(recovery),
            opened_at: now_ms(),
            _lock: lock,
        }
    }
    pub fn project_id(&self) -> String {
        self.manifest.lock().project_id.clone()
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct LocalProfile {
    pub user_id: String,
    pub display_name: String,
}

/// Handle for a background task: progress events, cancellation, completion.
#[derive(Clone)]
pub struct TaskHandle {
    pub id: String,
    kind: String,
    label: String,
    cancelled: Arc<AtomicBool>,
    events: Arc<dyn EventSink>,
}

impl TaskHandle {
    fn emit(
        &self,
        state: &str,
        progress: Option<f64>,
        message: Option<String>,
        error: Option<AppError>,
        result: Option<serde_json::Value>,
    ) {
        self.events.emit(&AppEvent::Task(TaskProgress {
            task_id: self.id.clone(),
            kind: self.kind.clone(),
            label: self.label.clone(),
            state: state.to_string(),
            progress,
            message,
            error,
            result,
        }));
    }
    pub fn progress(&self, fraction: f64, message: impl Into<String>) {
        self.emit(
            "running",
            Some(fraction.clamp(0.0, 1.0)),
            Some(message.into()),
            None,
            None,
        );
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
    /// Return `internal.cancelled` if the user cancelled.
    pub fn check(&self) -> AppResult<()> {
        if self.is_cancelled() {
            Err(AppError::cancelled())
        } else {
            Ok(())
        }
    }
    pub fn complete(&self, result: serde_json::Value) {
        self.emit("completed", Some(1.0), None, None, Some(result));
    }
    pub fn fail(&self, err: &AppError) {
        if err.code_str() == "internal.cancelled" {
            self.emit("cancelled", None, None, None, None);
        } else {
            self.emit("failed", None, None, Some(err.clone()), None);
        }
    }
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}

#[derive(Default)]
pub struct TaskManager {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl TaskManager {
    pub fn cancel(&self, task_id: &str) -> bool {
        match self.running.lock().get(task_id) {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }
}

pub struct AppCore {
    pub config: AppConfig,
    app_db: Mutex<Connection>,
    project: RwLock<Option<Arc<ProjectSession>>>,
    global: Mutex<Option<Arc<Store>>>,
    pub registry: Arc<Registry>,
    pub events: Arc<dyn EventSink>,
    /// Save state for the open project (status bar indicator).
    pub save: Arc<SaveTracker>,
    global_save: Arc<SaveTracker>,
    profile: RwLock<LocalProfile>,
    pub tasks: TaskManager,
    /// Licence and free-trial state (read-only in this release; gates nothing yet).
    pub entitlement: EntitlementService,
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    me: OnceLock<Weak<AppCore>>,
    /// Held for the whole of a project close, so a shutdown that starts meanwhile waits for the
    /// close to finish (checkpoint, crash marker) instead of finding no project and exiting.
    pub(crate) closing: Mutex<()>,
}

impl AppCore {
    pub fn new(mut config: AppConfig, events: Arc<dyn EventSink>) -> AppResult<Arc<AppCore>> {
        std::fs::create_dir_all(&config.app_data_dir)?;
        // Staging files left here by a write a crash interrupted (older than the platform's
        // stale age) are removed. Best effort.
        let stale = openframe_platform::atomic::remove_stale_temps(&config.app_data_dir);
        if stale > 0 {
            tracing::debug!(
                removed = stale,
                "removed stale staging files from the app data folder"
            );
        }
        let mut conn =
            openframe_persistence::open_connection(&config.app_data_dir.join("app.sqlite"), true)?;
        openframe_persistence::migrate::apply(&mut conn, crate::schema::APP_MIGRATIONS)?;
        let profile = ensure_profile(&conn)?;
        // Never fails and never blocks startup: problems are logged and the state is Unknown.
        let mac_inputs = config.mac_inputs.take();
        let microsoft_store = config.microsoft_store.take();
        let entitlement = EntitlementService::start(&config, mac_inputs, microsoft_store);
        let mut registry = Registry::default();
        crate::modules::register_all(&mut registry);
        if config.entitlement_debug_controls {
            crate::modules::entitlement::register_debug(&mut registry);
        }
        let core = Arc::new(AppCore {
            save: Arc::new(SaveTracker::new(events.clone())),
            global_save: Arc::new(SaveTracker::new(Arc::new(crate::events::NullSink))),
            config,
            app_db: Mutex::new(conn),
            project: RwLock::new(None),
            global: Mutex::new(None),
            registry: Arc::new(registry),
            events,
            profile: RwLock::new(profile),
            tasks: TaskManager::default(),
            entitlement,
            services: Mutex::new(HashMap::new()),
            me: OnceLock::new(),
            closing: Mutex::new(()),
        });
        let _ = core.me.set(Arc::downgrade(&core));
        Ok(core)
    }

    /// Owned handle for background threads.
    pub fn arc(&self) -> Arc<AppCore> {
        self.me
            .get()
            .and_then(|w| w.upgrade())
            .expect("AppCore is always held in an Arc")
    }

    pub fn profile(&self) -> LocalProfile {
        self.profile.read().clone()
    }

    pub fn set_display_name(&self, name: &str) -> AppResult<LocalProfile> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::required("Your name"));
        }
        let mut p = self.profile.write();
        self.with_app_db(|c| {
            c.execute(
                "UPDATE sys_local_profile SET display_name=?1, updated_at=?2 WHERE user_id=?3",
                params![name, now_ms(), p.user_id],
            )?;
            Ok(())
        })?;
        p.display_name = name.to_string();
        Ok(p.clone())
    }

    /// The local user. Ordinary local use needs no account (Security §5.3);
    /// the local user is the project's Owner unless the project says otherwise.
    pub fn local_actor(&self) -> Actor {
        let p = self.profile();
        let role = self
            .project_opt()
            .and_then(|s| {
                s.store
                    .read(|c| {
                        Ok(c.query_row(
                            "SELECT role FROM project_member WHERE user_id=?1 AND deleted_at IS NULL",
                            [&p.user_id],
                            |r| r.get::<_, String>(0),
                        )
                        .optional()?)
                    })
                    .ok()
                    .flatten()
            })
            .and_then(|r| Role::parse(&r))
            .unwrap_or(Role::Owner);
        Actor {
            user_id: p.user_id,
            display_name: p.display_name,
            role,
            origin: ActorOrigin::Local,
        }
    }

    pub fn with_app_db<R>(&self, f: impl FnOnce(&Connection) -> AppResult<R>) -> AppResult<R> {
        let conn = self.app_db.lock();
        f(&conn)
    }

    // ------------------------------------------------------------- project session

    pub fn project(&self) -> AppResult<Arc<ProjectSession>> {
        self.project_opt().ok_or_else(AppError::no_project_open)
    }

    pub fn project_opt(&self) -> Option<Arc<ProjectSession>> {
        self.project.read().clone()
    }

    pub fn set_project(&self, session: Option<Arc<ProjectSession>>) -> Option<Arc<ProjectSession>> {
        let prev = std::mem::replace(&mut *self.project.write(), session.clone());
        // The derived AI intelligence index follows the open project (detach waits for its
        // background worker to release the project files; attach never blocks).
        crate::modules::ai::intelligence::session_changed(self, prev.as_ref(), session.as_ref());
        prev
    }

    /// The Global Idea Vault store, opened lazily.
    pub fn global_store(&self) -> AppResult<Arc<Store>> {
        let mut g = self.global.lock();
        if let Some(s) = g.as_ref() {
            return Ok(s.clone());
        }
        let dir = &self.config.global_vault_dir;
        std::fs::create_dir_all(dir.join("assets"))?;
        let db = dir.join("vault.sqlite");
        let (store, _) = Store::open(
            StoreKind::Global,
            dir,
            &db,
            crate::schema::GLOBAL_MIGRATIONS,
            true,
            &dir.join("backups"),
            self.registry.clone(),
            self.events.clone(),
            self.global_save.clone(),
        )?;
        let store = Arc::new(store);
        *g = Some(store.clone());
        drop(g);
        // Recently Deleted keeps items 30 days; the first open in a session removes older ones.
        let p = self.profile();
        let owner = Actor::local_owner(p.user_id, p.display_name);
        crate::modules::retention::purge_expired_now(&store, &owner, "global");
        Ok(store)
    }

    /// App start: open the Global Idea Vault so its Recently Deleted drops items older than 30
    /// days. The desktop shell calls this once after setup, off the main thread. Never fails:
    /// a vault that cannot open now is opened (and cleaned) on first use instead.
    pub fn startup_maintenance(&self) {
        // Internal builds: nothing opens before the team password is right. The unlock runs this
        // again once the gate opens.
        if self.entitlement.team_gate_locked() {
            return;
        }
        if let Err(e) = self.global_store() {
            tracing::warn!(
                code = e.code_str(),
                "the Global Idea Vault could not be opened at start"
            );
        }
    }

    /// Run `f` against the requested store.
    pub fn with_store<R>(
        &self,
        kind: StoreKind,
        f: impl FnOnce(&Store) -> AppResult<R>,
    ) -> AppResult<R> {
        match kind {
            StoreKind::Project => {
                let s = self.project()?;
                f(&s.store)
            }
            StoreKind::Global => {
                let s = self.global_store()?;
                f(&s)
            }
        }
    }

    pub fn dispatch(
        &self,
        actor: &Actor,
        op: &str,
        args: serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        self.registry.dispatch(self, actor, op, args)
    }

    // ------------------------------------------------------------------- tasks

    /// Start a tracked background task on its own thread. `work` receives the
    /// handle for progress/cancellation; its result is delivered as an event.
    pub fn spawn_task<F>(&self, kind: &str, label: &str, work: F) -> String
    where
        F: FnOnce(Arc<AppCore>, TaskHandle) -> AppResult<serde_json::Value> + Send + 'static,
    {
        let handle = TaskHandle {
            id: new_id(),
            kind: kind.to_string(),
            label: label.to_string(),
            cancelled: Arc::new(AtomicBool::new(false)),
            events: self.events.clone(),
        };
        self.tasks
            .running
            .lock()
            .insert(handle.id.clone(), handle.cancelled.clone());
        handle.emit("queued", Some(0.0), None, None, None);
        let core = self.arc();
        let id = handle.id.clone();
        let on_spawn_failure = handle.clone();
        let spawned = std::thread::Builder::new()
            .name(format!("of-task-{kind}"))
            .spawn(move || {
                let h = handle.clone();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(core.clone(), handle)))
                    .unwrap_or_else(|_| Err(AppError::internal("background task panicked")));
                match result {
                    Ok(v) => h.complete(v),
                    Err(e) => {
                        tracing::warn!(code = e.code_str(), kind = %h.kind, "background task failed");
                        h.fail(&e)
                    }
                }
                core.tasks.running.lock().remove(&h.id);
            });
        // The system refused a new thread (out of resources): the task fails with a message instead
        // of taking the app down.
        if let Err(e) = spawned {
            tracing::error!(error = %e, kind, "a background task could not start");
            self.tasks.running.lock().remove(&id);
            on_spawn_failure.fail(
                &AppError::new(
                    "internal.task_start_failed",
                    "OpenFrame could not start this task. Close other programs and try again.",
                )
                .with_detail(e.to_string()),
            );
        }
        id
    }

    // ---------------------------------------------------------------- services

    /// Get or lazily create a long-lived module service (AI runtime, model downloads, …).
    pub fn service<T: Any + Send + Sync>(&self, init: impl FnOnce() -> T) -> Arc<T> {
        let mut map = self.services.lock();
        let entry = map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(init()) as Arc<dyn Any + Send + Sync>);
        entry
            .clone()
            .downcast::<T>()
            .expect("service type matches its TypeId")
    }

    pub fn existing_service<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.services
            .lock()
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|s| s.downcast::<T>().ok())
    }

    pub fn emit(&self, event: AppEvent) {
        self.events.emit(&event);
    }

    /// Sleep and wake from the platform boundary (`openframe_platform::power`). Before the
    /// computer sleeps, the write-ahead logs of the open project and of the Global Idea Vault are
    /// folded into their database files, so a laptop that runs out of battery while asleep
    /// reopens with every committed change in the main file. Errors are logged, never raised:
    /// the system is going to sleep either way.
    pub fn on_power_event(&self, event: openframe_platform::power::PowerEvent) {
        use openframe_platform::power::PowerEvent;
        match event {
            PowerEvent::Suspending => {
                let project = self.project_opt();
                let global = self.global.lock().clone();
                let stores = project
                    .as_ref()
                    .map(|p| &p.store)
                    .into_iter()
                    .chain(global.as_deref());
                for store in stores {
                    if let Err(e) = store.with_writer(openframe_persistence::checkpoint) {
                        tracing::warn!(code = e.code_str(), "checkpoint before sleep failed");
                    }
                }
                tracing::info!("data folded into the database files before sleep");
            }
            PowerEvent::Resumed => tracing::info!("computer woke from sleep"),
        }
    }

    /// Clean shutdown: close the project (checkpoint + clear crash marker).
    pub fn shutdown(&self) {
        if let Err(e) = crate::modules::project::close_current(self) {
            tracing::error!(
                code = e.code_str(),
                "error while closing project at shutdown"
            );
        }
        for flag in self.tasks.running.lock().values() {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

fn ensure_profile(conn: &Connection) -> AppResult<LocalProfile> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT user_id, display_name FROM sys_local_profile LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((user_id, display_name)) = existing {
        return Ok(LocalProfile {
            user_id,
            display_name,
        });
    }
    let name = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "You".to_string());
    let profile = LocalProfile {
        user_id: new_id(),
        display_name: name,
    };
    let now = now_ms();
    conn.execute(
        "INSERT INTO sys_local_profile(user_id, display_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![profile.user_id, profile.display_name, now],
    )?;
    Ok(profile)
}
