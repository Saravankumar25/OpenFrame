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
use openframe_project_format::{ProjectLayout, ProjectLock, ProjectManifest};
use parking_lot::{Mutex, RwLock};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use ts_rs::TS;

use crate::events::{AppEvent, EventSink, StoreKind, TaskProgress};
use crate::registry::Registry;
use crate::store::{SaveTracker, Store};

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// `%LOCALAPPDATA%/OpenFrame` — settings, recents, models, logs.
    pub app_data_dir: PathBuf,
    /// Default parent folder for new projects.
    pub projects_dir: PathBuf,
    /// Global Idea Vault folder (user content).
    pub global_vault_dir: PathBuf,
    pub app_version: String,
}

impl AppConfig {
    pub fn for_current_user(app_version: &str) -> Self {
        Self {
            app_data_dir: openframe_project_format::app_data_dir(),
            projects_dir: openframe_project_format::default_projects_dir(),
            global_vault_dir: openframe_project_format::default_global_vault_dir(),
            app_version: app_version.to_string(),
        }
    }

    /// Fully isolated configuration rooted in `root` (tests, fixtures).
    pub fn isolated(root: &std::path::Path) -> Self {
        Self {
            app_data_dir: root.join("appdata"),
            projects_dir: root.join("projects"),
            global_vault_dir: root.join("global-vault"),
            app_version: "0.1.0-test".to_string(),
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
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    me: OnceLock<Weak<AppCore>>,
}

impl AppCore {
    pub fn new(config: AppConfig, events: Arc<dyn EventSink>) -> AppResult<Arc<AppCore>> {
        std::fs::create_dir_all(&config.app_data_dir)?;
        let mut conn =
            openframe_persistence::open_connection(&config.app_data_dir.join("app.sqlite"), true)?;
        openframe_persistence::migrate::apply(&mut conn, crate::schema::APP_MIGRATIONS)?;
        let profile = ensure_profile(&conn)?;
        let mut registry = Registry::default();
        crate::modules::register_all(&mut registry);
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
            services: Mutex::new(HashMap::new()),
            me: OnceLock::new(),
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

    /// The local filmmaker. Ordinary local use needs no account (Security §5.3);
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
        Ok(store)
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
        std::thread::Builder::new()
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
            })
            .expect("spawn task thread");
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
