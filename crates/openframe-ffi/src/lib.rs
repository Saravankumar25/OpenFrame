//! The Rust core as a Swift library (ADR-0027, Mac phase M2).
//!
//! The SwiftUI shell reaches the core through the same single door as the Windows shell
//! (ADR-0004): an operation name with JSON arguments in, a JSON result or a coded error out, plus
//! the event stream. Nothing else crosses the boundary except a few platform requests (reveal a
//! file the core resolved, open a web link, sleep and wake). The Swift side holds no rules.
//!
//! ```text
//! SwiftUI ──► CoreHandle::invoke(op, json) ──► AppCore::dispatch ──► registry ──► store
//!         ◄── EventListener::on_event(json) ◄── AppCore events (FfiSink)
//! ```
//!
//! Threading: `invoke` blocks until the operation finishes, like the Tauri adapter's
//! `spawn_blocking` body. The Swift package always calls it from a background executor, never
//! from the main thread (`OpenFrameCore.Core`). Events arrive on core threads; the Swift side hops
//! to the main actor before touching the screen.

#![deny(unsafe_op_in_unsafe_fn)]

mod logging;
mod shell;

use std::path::PathBuf;
use std::sync::Arc;

use openframe_application::events::{AppEvent, EventSink};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::AppError;
use parking_lot::Mutex;

uniffi::setup_scaffolding!();

/// A coded error from the core. `code` and `message` are the same values the Windows screens
/// receive (`AppError`); `message` is safe to show, `detail` is for diagnostics only.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CoreError {
    #[error("{message}")]
    Failed {
        code: String,
        message: String,
        detail: Option<String>,
        retryable: bool,
    },
}

impl From<AppError> for CoreError {
    fn from(e: AppError) -> Self {
        CoreError::Failed {
            code: e.code_str().to_string(),
            message: e.message,
            detail: e.detail,
            retryable: e.retryable,
        }
    }
}

/// Receives every application event as the JSON the Windows screens receive (`AppEvent`).
/// Called on core threads; implementations must return quickly.
#[uniffi::export(callback_interface)]
pub trait EventListener: Send + Sync {
    fn on_event(&self, event_json: String);
}

/// How the shell starts the core.
#[derive(Debug, Clone, uniffi::Record)]
pub struct CoreConfig {
    /// The app version shown in About and written into project files.
    pub app_version: String,
    /// An absolute folder that holds every file of this run (app data, projects, Global Idea
    /// Vault, deleted projects). Set by the automated tests so they never touch the person's own
    /// data (ADR-0031). `None`: the person's own folders.
    pub isolated_data_root: Option<String>,
    /// The folder inside the app bundle that holds the Offline AI engine (`Contents/Resources/ai`).
    pub ai_bundle_dir: Option<String>,
    /// Write the JSON log files under the app data folder.
    pub write_logs: bool,
    /// A team password hash for the automated tests of the password screen. Honoured only by a
    /// debug build on a test folder, like the other test switches (ADR-0031); a release build and
    /// a run on the person's own data always use the hash embedded at build time.
    #[uniffi(default = None)]
    pub test_team_password_hash: Option<String>,
}

/// The running core. One per app process.
#[derive(uniffi::Object)]
pub struct CoreHandle {
    core: Arc<AppCore>,
    _log_guard: Mutex<Option<tracing_appender::non_blocking::WorkerGuard>>,
}

struct FfiSink {
    listener: Box<dyn EventListener>,
}

impl EventSink for FfiSink {
    fn emit(&self, event: &AppEvent) {
        match serde_json::to_string(event) {
            Ok(json) => self.listener.on_event(json),
            Err(e) => tracing::warn!(error = %e, "an event could not be encoded for the shell"),
        }
    }
}

/// The team password hash of an internal build (ADR-0024), embedded by `build.rs`.
#[cfg(openframe_team_gate)]
const TEAM_PASSWORD_HASH: Option<&str> = Some(include_str!(concat!(
    env!("OUT_DIR"),
    "/team_gate_hash.txt"
)));
#[cfg(not(openframe_team_gate))]
const TEAM_PASSWORD_HASH: Option<&str> = None;

fn build_config(config: &CoreConfig) -> Result<(AppConfig, bool), CoreError> {
    let isolated = match config.isolated_data_root.as_deref() {
        Some(raw) => {
            let root = PathBuf::from(raw);
            // A test folder was asked for but cannot be used: never fall back to the real data.
            if !root.is_absolute() || std::fs::create_dir_all(&root).is_err() {
                return Err(AppError::new(
                    "storage.test_root_invalid",
                    "The test data folder is not an absolute folder that can be created.",
                )
                .into());
            }
            Some(root)
        }
        None => None,
    };
    let mut app = match &isolated {
        Some(root) => {
            // Deleted projects of a test run stay in its own folder, never the Trash.
            openframe_application::modules::project::fault::trash_into(root.join("recycle-bin"));
            AppConfig {
                app_version: config.app_version.clone(),
                ..AppConfig::isolated(root)
            }
        }
        None => AppConfig::for_current_user(&config.app_version)?,
    };
    app.channel = channel_for(cfg!(openframe_store));
    app.ai_bundle_dir = config
        .ai_bundle_dir
        .as_deref()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute());
    app.ai_model_source =
        openframe_application::ModelSource::from_build(option_env!("OPENFRAME_TEST_MODEL_SOURCE"));
    // Hand-set licence states exist only in a debug build on a test folder (as on Windows).
    app.entitlement_debug_controls = cfg!(debug_assertions) && isolated.is_some();
    app.team_password_hash = TEAM_PASSWORD_HASH.map(str::to_string);
    if let Some(hash) = test_gate(config, cfg!(debug_assertions), isolated.is_some()) {
        app.team_password_hash = Some(hash);
    }
    Ok((app, isolated.is_some()))
}

/// The test hash of the password screen tests: only a debug build on a test folder, and only a
/// well-formed Argon2id string (the gate check refuses anything else anyway).
fn test_gate(config: &CoreConfig, debug: bool, isolated: bool) -> Option<String> {
    let hash = config.test_team_password_hash.as_deref()?;
    (debug && isolated && hash.starts_with("$argon2id$")).then(|| hash.to_string())
}

/// The disk image is an internal team build; the Mac App Store package is the store channel.
fn channel_for(store_build: bool) -> openframe_entitlement::Channel {
    if store_build {
        openframe_entitlement::Channel::MacAppStore
    } else {
        openframe_entitlement::Channel::DirectInstaller
    }
}

#[uniffi::export]
impl CoreHandle {
    /// Start the core: open the app database, the entitlement check and the event stream.
    #[uniffi::constructor]
    pub fn start(
        config: CoreConfig,
        listener: Box<dyn EventListener>,
    ) -> Result<Arc<Self>, CoreError> {
        let (app_config, _isolated) = build_config(&config)?;
        let guard = if config.write_logs {
            logging::init(&app_config.app_data_dir)
        } else {
            None
        };
        tracing::info!(
            version = %config.app_version,
            shell = "swiftui",
            "OpenFrame Studio starting"
        );
        let sink = Arc::new(FfiSink { listener });
        let core = AppCore::new(app_config, sink).inspect_err(|e| {
            tracing::error!(
                code = e.code_str(),
                detail = %openframe_security::redact(e.detail.as_deref().unwrap_or("")),
                "failed to start application core"
            );
        })?;
        // App start: Recently Deleted in the Global Idea Vault drops items older than 30 days.
        let start_core = core.clone();
        std::thread::spawn(move || start_core.startup_maintenance());
        Ok(Arc::new(CoreHandle {
            core,
            _log_guard: Mutex::new(guard),
        }))
    }

    /// Run one registered operation. `args_json` is a JSON object (an empty string means `{}`).
    /// Blocks until the operation ends; call it off the main thread.
    pub fn invoke(&self, op: String, args_json: String) -> Result<String, CoreError> {
        let args = if args_json.trim().is_empty() {
            serde_json::Value::Object(Default::default())
        } else {
            serde_json::from_str(&args_json).map_err(|e| {
                AppError::invalid_input("The request could not be read.")
                    .with_detail(format!("arguments for {op}: {e}"))
            })?
        };
        let started = std::time::Instant::now();
        let actor = self.core.local_actor();
        let result = self.core.dispatch(&actor, &op, args);
        let ms = started.elapsed().as_millis();
        match &result {
            Ok(_) if ms > 250 => tracing::info!(op = %op, ms, "slow operation"),
            Ok(_) => tracing::debug!(op = %op, ms, "operation"),
            Err(e) => tracing::info!(op = %op, code = e.code_str(), ms, "operation failed"),
        }
        let value = result?;
        serde_json::to_string(&value).map_err(|e| {
            AppError::internal(format!("result of {op} could not be encoded: {e}")).into()
        })
    }

    /// The names of every registered operation, sorted (the IPC allow-list).
    pub fn operation_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .core
            .registry
            .op_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        names.sort();
        names
    }

    /// Cancel a background task started by a long-running operation.
    pub fn cancel_task(&self, task_id: String) -> bool {
        self.core.tasks.cancel(&task_id)
    }

    /// Sleep and wake from `NSWorkspace` notifications: before sleep the open project's data is
    /// folded into its database file.
    pub fn power_event(&self, sleeping: bool) {
        use openframe_platform::power::PowerEvent;
        self.core.on_power_event(if sleeping {
            PowerEvent::Suspending
        } else {
            PowerEvent::Resumed
        });
    }

    /// Open a project asset with its default app, or show it in Finder. The path is resolved by
    /// the core from the id; the shell never passes a path.
    pub fn open_asset(&self, asset_id: String, global: bool, reveal: bool) -> Result<(), CoreError> {
        shell::open_asset(&self.core, &asset_id, global, reveal).map_err(Into::into)
    }

    /// The file of a project or Global Idea Vault asset, for previews in the shell (the Mac
    /// counterpart of the Windows `ofasset` protocol). Only managed asset folders are answered,
    /// never the databases, `cache/` or recovery data.
    pub fn asset_file(&self, asset_id: String, global: bool) -> Result<String, CoreError> {
        shell::asset_file(&self.core, &asset_id, global)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(Into::into)
    }

    /// Show a location OpenFrame knows in Finder (`project`, `globalVault`, `dataRoot`, `logs`,
    /// `assetFolder`, `globalAssetFolder`, `exported`).
    pub fn reveal_location(
        &self,
        kind: String,
        id: Option<String>,
        path: Option<String>,
    ) -> Result<(), CoreError> {
        shell::reveal_location(&self.core, &kind, id, path).map_err(Into::into)
    }

    /// Open a web link (http or https) in the default browser, after an explicit user action.
    pub fn open_url(&self, url: String) -> Result<(), CoreError> {
        shell::open_url(&url).map_err(Into::into)
    }

    /// Clean shutdown: close the project (checkpoint, clear the crash marker), stop tasks.
    pub fn stop(&self) {
        self.core.shutdown();
    }
}

/// Whether this library was built with the internal team password gate (diagnostics and tests).
#[uniffi::export]
pub fn team_gate_built_in() -> bool {
    TEAM_PASSWORD_HASH.is_some()
}

/// Every registered operation with its kind and argument and result type names, as JSON. The
/// Swift type generator (`scripts/gen-swift-types.mjs`) reads it so the Swift operation list
/// comes from the same registry as the TypeScript bindings (ADR-0027 decision 4).
#[uniffi::export]
pub fn operation_inventory_json() -> String {
    use openframe_application::registry::{OpKind, catalog, short_type_name};
    let registry = catalog();
    let ops: Vec<serde_json::Value> = registry
        .op_names()
        .into_iter()
        .filter_map(|name| registry.get(name).map(|e| (name, e)))
        .map(|(name, e)| {
            serde_json::json!({
                "name": name,
                "kind": match e.kind { OpKind::Query => "query", OpKind::Command => "command" },
                "argType": short_type_name(e.arg_type),
                "resultType": short_type_name(e.result_type),
                "longRunning": e.meta.as_ref().is_some_and(|m| m.long_running),
                "module": e.meta.as_ref().map(|m| m.module).unwrap_or(""),
            })
        })
        .collect();
    serde_json::to_string_pretty(&ops).unwrap_or_else(|_| "[]".to_string())
}

/// The version of this library, for the About window and diagnostics.
#[uniffi::export]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disk_image_is_an_internal_build_and_the_store_package_is_the_store_channel() {
        assert_eq!(channel_for(false), openframe_entitlement::Channel::DirectInstaller);
        assert_eq!(channel_for(true), openframe_entitlement::Channel::MacAppStore);
    }

    #[test]
    fn a_relative_test_folder_is_refused_instead_of_falling_back_to_real_data() {
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: Some("relative/folder".into()),
            ai_bundle_dir: None,
            write_logs: false,
            test_team_password_hash: None,
        };
        let err = build_config(&config).expect_err("refused");
        let CoreError::Failed { code, .. } = err;
        assert_eq!(code, "storage.test_root_invalid");
    }

    #[test]
    fn a_relative_ai_folder_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: Some(dir.path().to_string_lossy().into_owned()),
            ai_bundle_dir: Some("Resources/ai".into()),
            write_logs: false,
            test_team_password_hash: None,
        };
        let (app, isolated) = build_config(&config).unwrap();
        assert!(isolated);
        assert!(app.ai_bundle_dir.is_none());
        assert!(app.projects_dir.starts_with(dir.path()));
    }

    #[test]
    fn the_test_gate_hash_is_honoured_only_by_a_debug_build_on_a_test_folder() {
        let config = CoreConfig {
            app_version: "0.1.0".into(),
            isolated_data_root: None,
            ai_bundle_dir: None,
            write_logs: false,
            test_team_password_hash: Some("$argon2id$v=19$m=65536,t=3,p=1$abc$def".into()),
        };
        assert!(test_gate(&config, true, true).is_some());
        assert!(test_gate(&config, false, true).is_none(), "release build");
        assert!(test_gate(&config, true, false).is_none(), "the person's own data");
        let junk = CoreConfig { test_team_password_hash: Some("plain".into()), ..config };
        assert!(test_gate(&junk, true, true).is_none());
    }

    #[test]
    fn a_store_build_embeds_no_team_password_hash() {
        if cfg!(openframe_store) {
            assert!(TEAM_PASSWORD_HASH.is_none());
        }
        if let Some(hash) = TEAM_PASSWORD_HASH {
            assert!(hash.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"), "{hash}");
        }
    }
}
