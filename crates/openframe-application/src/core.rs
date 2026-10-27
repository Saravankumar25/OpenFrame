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
use crate::modules::storage::library_ops::{self, LibraryPlace, LibrarySetup};
use crate::modules::storage::{self, library, library::Library};
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
    /// Where OpenFrame AI downloads its pinned model files (ADR-0032): Hugging Face, or the local
    /// test host a test build names. `None` in tests and when a test build names an invalid host:
    /// OpenFrame AI cannot be downloaded then, and an installed set still works.
    pub ai_model_source: Option<openframe_ai::ModelSource>,
    /// Temperature 0 and a fixed seed for the Offline AI. Only the end-to-end suite turns it on
    /// (a debug build on an isolated test data root); never in a release build.
    pub ai_deterministic: bool,
    /// Default parent folder for new projects.
    pub projects_dir: PathBuf,
    /// Global Idea Vault folder (user content).
    pub global_vault_dir: PathBuf,
    /// The library file (`library.json`): the person's identity and chosen folders, kept in the
    /// library folder so it outlives the app data (doc 40, ADR-0038). `None` keeps neither.
    /// `projects_dir` and `global_vault_dir` above are only the defaults; the places the person
    /// chose are read through [`AppCore::projects_dir`] and [`AppCore::vault_dir`].
    pub library_file: Option<PathBuf>,
    /// How the library folder is chosen (doc 40 section 1). `None` (tests, the end-to-end data
    /// root): the library is set up from the start in the folder that holds `projects_dir`.
    /// `Some`: the person chooses it on the first start; nothing is created before they confirm,
    /// and the choice is remembered in `app.sqlite` and in a pointer file outside the app data.
    pub library_setup: Option<LibrarySetup>,
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
            .field("ai_model_source", &self.ai_model_source)
            .field("ai_deterministic", &self.ai_deterministic)
            .field("projects_dir", &self.projects_dir)
            .field("global_vault_dir", &self.global_vault_dir)
            .field("library_file", &self.library_file)
            .field("library_setup", &self.library_setup)
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
    /// when the system cannot name the home folder or the app data folder, instead of keeping
    /// projects and settings in a temp folder the system cleans up. The library is not chosen
    /// here: the first start asks (doc 40 section 1). The folders below are the suggestion only.
    pub fn for_current_user(app_version: &str) -> AppResult<Self> {
        let app_data_dir = openframe_project_format::app_data_dir();
        let setup = app_data_dir
            .as_deref()
            .and_then(LibrarySetup::for_current_user);
        let suggested = setup.as_ref().map(|s| s.suggested_root.clone());
        let mut config = Self::from_system_folders(
            app_data_dir,
            suggested
                .as_ref()
                .map(|r| r.join(library_ops::PROJECTS_FOLDER)),
            suggested
                .as_ref()
                .map(|r| r.join(library_ops::VAULT_FOLDER)),
            app_version,
        )?;
        config.library_setup = setup;
        Ok(config)
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
        // The library folder is the one that holds the default projects folder.
        let library_file = projects_dir
            .parent()
            .map(|d| d.join(crate::modules::storage::library::LIBRARY_FILE_NAME));
        Ok(Self {
            app_data_dir,
            ai_bundle_dir: Self::installed_ai_bundle_dir(),
            ai_model_source: None,
            ai_deterministic: false,
            projects_dir,
            global_vault_dir,
            library_file,
            library_setup: None,
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

    /// Fully isolated configuration rooted in `root` (tests, fixtures).
    pub fn isolated(root: &std::path::Path) -> Self {
        Self {
            app_data_dir: root.join("appdata"),
            ai_bundle_dir: None,
            ai_model_source: None,
            ai_deterministic: false,
            projects_dir: root.join("projects"),
            global_vault_dir: root.join("global-vault"),
            library_file: Some(root.join(crate::modules::storage::library::LIBRARY_FILE_NAME)),
            library_setup: None,
            app_version: "0.1.0-test".to_string(),
            channel: Channel::DirectInstaller,
            mac_inputs: None,
            microsoft_store: None,
            entitlement_debug_controls: false,
            team_password_hash: None,
        }
    }

    /// [`AppConfig::isolated`] that starts like a new computer: the library is not set up, the
    /// suggested place is `<root>/home/OpenFrame`, an earlier library would be in
    /// `<root>/Documents/OpenFrame` and the pointer in `<root>/roaming` (tests and the end-to-end
    /// switch `OPENFRAME_LIBRARY_SETUP=1`).
    pub fn isolated_needing_setup(root: &std::path::Path) -> Self {
        let setup = LibrarySetup::under(root);
        Self {
            projects_dir: setup.suggested_root.join(library_ops::PROJECTS_FOLDER),
            global_vault_dir: setup.suggested_root.join(library_ops::VAULT_FOLDER),
            library_file: None,
            library_setup: Some(setup),
            ..Self::isolated(root)
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
    /// The assistant's list of things a user can name, built on first use (AI harness S3).
    pub ai_index: crate::modules::ai::index::IndexCache,
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
            ai_index: Default::default(),
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

/// The library in use and the places the person chose on top of its defaults. `None` is the
/// default; a `library` of `None` means the library is not set up yet (doc 40 section 1).
#[derive(Debug, Clone, Default)]
pub(crate) struct Locations {
    pub(crate) projects_dir: Option<PathBuf>,
    pub(crate) vault_dir: Option<PathBuf>,
    pub(crate) library: Option<LibraryPlace>,
}

/// What the one running storage task is doing (doc 40 section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorageOp {
    /// A task on the Global Idea Vault: backup, package, restore, move, or connecting a folder.
    Vault,
    /// Moving the open project. While it runs, no project can be opened, created or closed.
    ProjectMove,
    /// Moving the whole library. Blocks projects like a project move.
    LibraryMove,
}

/// Holds the storage-task slot until dropped, on success, error or panic alike.
pub(crate) struct StorageGuard {
    slot: Arc<Mutex<Option<StorageOp>>>,
}

impl Drop for StorageGuard {
    fn drop(&mut self) {
        *self.slot.lock() = None;
    }
}

/// How long a call that needs the vault waits while it is closed for a move or a restore.
const VAULT_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

pub struct AppCore {
    pub config: AppConfig,
    locations: RwLock<Locations>,
    /// The library file was there at start but could not be trusted. It is not rewritten until
    /// the person changes a place on purpose (doc 40 section 2).
    library_unusable: AtomicBool,
    /// The profile is in `app.sqlite`. Before the library is set up a new person's id is kept in
    /// memory only, so connecting an existing library can still take over its identity.
    profile_saved: AtomicBool,
    /// The one storage task that runs at a time (a second one is refused with `storage.busy`).
    storage_op: Arc<Mutex<Option<StorageOp>>>,
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
        // Cover photos no project row names (a crash between the file and the row) are removed.
        crate::modules::project_cover::sweep(&conn, &config.app_data_dir);
        // The library in use: the remembered one when its folder is there, else none yet (the
        // first start asks; a missing folder is never recreated empty).
        let place = library_ops::place_at_start(&conn, &config);
        // A person who comes back after an app data reset keeps their identity and chosen folders.
        let (library, library_unusable) = read_library_file(place.as_ref(), &config.app_data_dir);
        let (profile, profile_saved) = ensure_profile(&conn, library.as_ref(), place.is_some())?;
        let locations = Locations {
            projects_dir: library.as_ref().and_then(|l| l.projects_dir.clone()),
            vault_dir: library.and_then(|l| l.vault_dir),
            library: place,
        };
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
            locations: RwLock::new(locations),
            library_unusable: AtomicBool::new(library_unusable),
            profile_saved: AtomicBool::new(profile_saved),
            storage_op: Arc::new(Mutex::new(None)),
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
        core.write_library_best_effort();
        Ok(core)
    }

    // ----------------------------------------------------------- places and identity

    /// Where new projects go: the folder the person chose, else the library's `Projects`.
    pub fn projects_dir(&self) -> PathBuf {
        let chosen = self.locations.read().projects_dir.clone();
        chosen.unwrap_or_else(|| self.default_projects_dir())
    }

    /// The Global Idea Vault folder: the one the person chose or moved to, else the default.
    pub fn vault_dir(&self) -> PathBuf {
        let chosen = self.locations.read().vault_dir.clone();
        chosen.unwrap_or_else(|| self.default_vault_dir())
    }

    /// The library's own folder for new projects (`<library>/Projects`).
    pub fn default_projects_dir(&self) -> PathBuf {
        let place = self.locations.read().library.clone();
        place.map_or_else(|| self.config.projects_dir.clone(), |p| p.projects_dir)
    }

    /// The library's own vault folder (`<library>/Global Idea Vault`).
    pub fn default_vault_dir(&self) -> PathBuf {
        let place = self.locations.read().library.clone();
        place.map_or_else(|| self.config.global_vault_dir.clone(), |p| p.vault_dir)
    }

    /// The library in use; `None` until it is set up (doc 40 section 1).
    pub fn library_place(&self) -> Option<LibraryPlace> {
        self.locations.read().library.clone()
    }

    /// True once the person chose a library (always in tests and on the end-to-end data root).
    pub fn library_ready(&self) -> bool {
        self.locations.read().library.is_some()
    }

    /// The library file in use, when there is one.
    pub fn library_file(&self) -> Option<PathBuf> {
        self.locations.read().library.as_ref()?.file.clone()
    }

    /// The one folder to back up, shown on the data note and in the file safety tutorial: the
    /// folder that holds both the projects folder and the Global Idea Vault (the library folder
    /// by default), or the projects folder when they live apart.
    pub fn data_root(&self) -> PathBuf {
        let (projects, vault) = (self.projects_dir(), self.vault_dir());
        match (projects.parent(), vault.parent()) {
            (Some(a), Some(b)) if a == b && !a.as_os_str().is_empty() => a.to_path_buf(),
            _ => projects,
        }
    }

    /// Start using the library at `place` (first setup, or connecting another library). The
    /// identity: `app.sqlite` keeps its saved profile (ADR-0038); a person with no saved profile
    /// yet (a new computer, or a reset app data) takes over the identity in the library file, so
    /// their private notes stay theirs. When a saved profile meets the file of another person,
    /// that file is copied aside first, so their id is not lost. The library file is written and
    /// the choice remembered (`app.sqlite`, pointer) before anything in memory changes.
    pub(crate) fn use_library(&self, place: LibraryPlace) -> AppResult<()> {
        let (library, unusable) = read_library_file(Some(&place), &self.config.app_data_dir);
        {
            let mut profile = self.profile.write();
            if !self.profile_saved.load(Ordering::SeqCst) {
                if let Some(l) = &library {
                    profile.user_id = l.user_id.clone();
                    profile.display_name = l.display_name.clone();
                }
                self.with_app_db(|c| insert_profile(c, &profile))?;
                self.profile_saved.store(true, Ordering::SeqCst);
            } else if let (Some(l), Some(file)) = (&library, &place.file)
                && l.user_id != profile.user_id
                && library::preserve_copy(file, ".other-person-").is_none()
            {
                return Err(AppError::new(
                    "storage.not_writable",
                    "OpenFrame can't save files in that library folder, so it was not connected.",
                ));
            }
        }
        let next = Locations {
            projects_dir: library.as_ref().and_then(|l| l.projects_dir.clone()),
            vault_dir: library.and_then(|l| l.vault_dir),
            library: Some(place),
        };
        self.with_vault_closed(|| {
            let mut locations = self.locations.write();
            if !unusable {
                self.write_library(&next)?;
            }
            library_ops::remember_root(self, next.library.as_ref().map(|p| p.root.as_path()))?;
            *locations = next;
            self.library_unusable.store(unusable, Ordering::SeqCst);
            Ok(())
        })
    }

    /// A snapshot of the places, to put back after a failed library move.
    pub(crate) fn locations_snapshot(&self) -> Locations {
        self.locations.read().clone()
    }

    /// Put a snapshot back (library file and remembered root included). Used to undo a move.
    pub(crate) fn restore_locations(&self, previous: Locations) -> AppResult<()> {
        let root = previous.library.as_ref().map(|p| p.root.clone());
        library_ops::remember_root(self, root.as_deref())?;
        self.change_locations(|l| *l = previous)
    }

    /// Change the chosen places and record them in the library file. The running values change
    /// only after the file is written, so a failed write leaves everything as it was and the
    /// person hears about it. `change` may set a place to `None` to go back to its default.
    pub(crate) fn change_locations(&self, change: impl FnOnce(&mut Locations)) -> AppResult<()> {
        let mut locations = self.locations.write();
        let mut next = locations.clone();
        change(&mut next);
        let unusable = self.library_unusable.load(Ordering::SeqCst);
        if unusable && let Some(file) = locations.library.as_ref().and_then(|p| p.file.as_ref()) {
            // A copy was made at start; make sure one exists before the file is replaced.
            let _ = library::preserve_unusable(file);
        }
        self.write_library(&next)?;
        if unusable {
            self.library_unusable.store(false, Ordering::SeqCst);
        }
        *locations = next;
        Ok(())
    }

    fn write_library(&self, locations: &Locations) -> AppResult<()> {
        let Some(file) = locations.library.as_ref().and_then(|p| p.file.as_ref()) else {
            return Ok(());
        };
        let p = self.profile();
        library::write(
            file,
            &p.user_id,
            &p.display_name,
            locations.projects_dir.as_deref(),
            locations.vault_dir.as_deref(),
        )
    }

    /// Keep the library file current (start, name change). A failure is logged and never stops
    /// the app: the file is a safety net, not the source of truth while `app.sqlite` is there.
    fn write_library_best_effort(&self) {
        if self.library_unusable.load(Ordering::SeqCst) {
            return;
        }
        let locations = self.locations.read().clone();
        if let Err(e) = self.write_library(&locations) {
            tracing::warn!(code = e.code_str(), "the library file could not be written");
        }
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
        let updated = p.clone();
        drop(p);
        self.write_library_best_effort();
        Ok(updated)
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
        // A move or a restore keeps the vault closed (and this lock held) for the whole copy. A
        // query that arrives then gets a plain answer after a moment, instead of a frozen screen.
        let mut g = self.global.try_lock_for(VAULT_WAIT).ok_or_else(|| {
            storage_busy("The Global Idea Vault is being moved or restored. Try again in a moment.")
        })?;
        if let Some(s) = g.as_ref() {
            return Ok(s.clone());
        }
        // Nothing is created before the person chose where their library is (doc 40 section 1).
        if !self.library_ready() {
            return Err(library_ops::setup_required());
        }
        let dir = self.vault_dir();
        let db = dir.join("vault.sqlite");
        // Only the default folder is created on first use. A folder the person chose that has no
        // vault in it is a drive that is not connected or a folder that was moved: it is never
        // replaced by a new empty vault (doc 40 section 3).
        if !db.is_file() && !library::same_place(&dir, &self.default_vault_dir()) {
            return Err(vault_missing_error(&dir));
        }
        std::fs::create_dir_all(dir.join("assets"))?;
        let (store, _) = Store::open(
            StoreKind::Global,
            &dir,
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

    /// Let go of the cached Global Idea Vault store (moving, restoring or reconnecting the vault),
    /// after folding its write-ahead log into the database file. The next use opens whatever is
    /// at `vault_dir()` then. Holders of an earlier handle keep working on the old files until
    /// they drop it.
    pub fn close_global_store(&self) {
        if let Some(store) = self.global.lock().take()
            && let Err(e) = store.with_writer(openframe_persistence::checkpoint)
        {
            tracing::warn!(
                code = e.code_str(),
                "checkpoint of the vault before closing failed"
            );
        }
    }

    /// Take the slot for a storage task (vault backup, restore, move, locate, or a project move).
    /// Only one runs at a time: a second start is refused with `storage.busy` and nothing changes.
    /// The slot is freed when the guard drops, so an error or a panic in the task frees it too.
    pub(crate) fn begin_storage_op(&self, op: StorageOp) -> AppResult<StorageGuard> {
        let mut slot = self.storage_op.lock();
        if slot.is_some() {
            return Err(storage_busy(
                "Another storage task is still running. Wait for it to finish, then try again. Nothing was changed.",
            ));
        }
        *slot = Some(op);
        Ok(StorageGuard {
            slot: self.storage_op.clone(),
        })
    }

    /// Opening, creating or closing a project is refused while the open project is being moved:
    /// the move closes it, copies it and reopens it, and another open in between could leave the
    /// session on the old folder just before it is deleted.
    pub(crate) fn refuse_during_project_move(&self) -> AppResult<()> {
        match *self.storage_op.lock() {
            Some(StorageOp::ProjectMove) => Err(storage_busy(
                "The project is being moved. Wait for the move to finish, then try again.",
            )),
            Some(StorageOp::LibraryMove) => Err(storage_busy(
                "Your OpenFrame library is being moved. Wait for the move to finish, then try again.",
            )),
            _ => Ok(()),
        }?;
        Ok(())
    }

    /// Close the Global Idea Vault and keep it closed while `f` runs. Whatever needs the vault
    /// meanwhile waits for `f` to finish, so a move or a restore never loses a change made during
    /// the copy. `f` must not use the vault store; open it again after this returns. Fails with
    /// `storage.busy` (vault left open) when something still holds the store after a few seconds.
    pub(crate) fn with_vault_closed<R>(&self, f: impl FnOnce() -> AppResult<R>) -> AppResult<R> {
        let mut g = self.global.lock();
        if let Some(store) = g.take() {
            if let Err(e) = store.with_writer(openframe_persistence::checkpoint) {
                tracing::warn!(
                    code = e.code_str(),
                    "checkpoint of the vault before closing failed"
                );
            }
            // An operation that started before we took the lock may still hold the store. The
            // files are complete only when the last holder lets go (that closes the connections).
            let give_up = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while Arc::strong_count(&store) > 1 {
                if std::time::Instant::now() >= give_up {
                    *g = Some(store);
                    return Err(AppError::new(
                        "storage.busy",
                        "Your Global Idea Vault is in use right now. Wait a moment and try again. Nothing was changed.",
                    )
                    .retryable());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        f()
    }

    /// App start: remove what an interrupted move or restore left behind, then open the Global
    /// Idea Vault so its Recently Deleted drops items older than 30 days. A vault that does not
    /// exist yet is not created here (doc 40 section 3). The desktop shell calls this once after
    /// setup, off the main thread. Never fails: a vault that cannot open now is opened (and
    /// cleaned) on first use instead.
    pub fn startup_maintenance(&self) {
        // Internal builds: nothing opens before the team password is right. The unlock runs this
        // again once the gate opens.
        if self.entitlement.team_gate_locked() {
            return;
        }
        // Before the library is set up there is nothing to clean, and nothing may be created.
        // Setting it up runs this again.
        if !self.library_ready() {
            return;
        }
        let vault = self.vault_dir();
        storage::remove_noted_staging(&self.config.app_data_dir);
        storage::remove_staging_leftovers(&[
            &self.projects_dir(),
            vault.parent().unwrap_or(&vault),
        ]);
        if !vault.join("vault.sqlite").is_file() {
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
        // Until the library is set up only the setup screen and what draws the window run.
        if !self.library_ready() && !library_ops::allowed_before_setup(op) {
            return Err(library_ops::setup_required().with_detail(format!("{op} before setup")));
        }
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

/// A storage task cannot start or a call cannot be served right now. Retryable; nothing changed.
pub(crate) fn storage_busy(message: &str) -> AppError {
    AppError::new("storage.busy", message).retryable()
}

/// The vault folder the library file names has no vault in it: a drive that is not connected or
/// a folder that was moved. Settings > Storage offers Locate and Change.
pub(crate) fn vault_missing_error(dir: &std::path::Path) -> AppError {
    AppError::new(
        "storage.vault_missing",
        "Your Global Idea Vault can't be found where it was last used. If it is on an \
         external drive, reconnect the drive. Otherwise go to Settings > Storage and choose \
         the vault folder.",
    )
    .with_detail(dir.to_string_lossy().into_owned())
}

/// The library file of `place`: valid, or `None` with whether it is there but unusable. An
/// unusable file is never trusted and never overwritten; a copy is kept beside it.
fn read_library_file(
    place: Option<&LibraryPlace>,
    app_data_dir: &std::path::Path,
) -> (Option<Library>, bool) {
    let Some(file) = place.and_then(|p| p.file.as_deref()) else {
        return (None, false);
    };
    match library::read(file, app_data_dir) {
        library::Read::Valid(l) => (Some(l), false),
        library::Read::Unusable => {
            if library::preserve_unusable(file).is_some() {
                tracing::warn!("the unusable library file was copied aside");
            }
            (None, true)
        }
        library::Read::Absent => (None, false),
    }
}

fn insert_profile(conn: &Connection, p: &LocalProfile) -> AppResult<()> {
    let now = now_ms();
    conn.execute(
        "INSERT INTO sys_local_profile(user_id, display_name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![p.user_id, p.display_name, now],
    )?;
    Ok(())
}

/// The local profile and whether it is saved. `app.sqlite` wins when it has one; without one
/// (first start, or the app data was deleted) the identity in a valid library file is taken
/// over, so private notes that belong to it stay theirs. Only a person with neither gets a new
/// id, saved only when `save` (the library is set up): before that it lives in memory, so
/// connecting an existing library can still take over that library's identity.
fn ensure_profile(
    conn: &Connection,
    library: Option<&Library>,
    save: bool,
) -> AppResult<(LocalProfile, bool)> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT user_id, display_name FROM sys_local_profile LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((user_id, display_name)) = existing {
        return Ok((
            LocalProfile {
                user_id,
                display_name,
            },
            true,
        ));
    }
    let profile = match library {
        Some(l) => LocalProfile {
            user_id: l.user_id.clone(),
            display_name: l.display_name.clone(),
        },
        None => LocalProfile {
            user_id: new_id(),
            display_name: std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "You".to_string()),
        },
    };
    if save {
        insert_profile(conn, &profile)?;
    }
    Ok((profile, save))
}
