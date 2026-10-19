//! Tauri adapter (ESD §5: "Tauri is an adapter, not the application architecture").
//!
//! The webview gets exactly one data command, `of_invoke(op, args)`, which is
//! checked against the application's operation registry (the IPC allow-list).
//! File opening/revealing is resolved on the Rust side from asset/project ids,
//! so the webview never holds filesystem, shell or network privileges.

mod asset_scope;
mod close_gate;
mod distribution;
mod platform;

use std::path::PathBuf;
use std::sync::Arc;

use openframe_application::events::{AppEvent, EventSink};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::AppError;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

const EVENT_CHANNEL: &str = "of://event";

struct TauriSink {
    app: AppHandle,
}

impl EventSink for TauriSink {
    fn emit(&self, event: &AppEvent) {
        if let Err(e) = self.app.emit(EVENT_CHANNEL, event) {
            tracing::warn!(error = %e, "failed to deliver event to UI");
        }
    }
}

struct CoreState(Arc<AppCore>);

/// Keeps the sleep and wake notification registered while the app runs.
struct PowerState(
    #[allow(dead_code)] parking_lot::Mutex<Option<openframe_platform::power::PowerSubscription>>,
);

/// The folders the `ofasset` protocol may read: only managed media, the open project's
/// `assets/` folder and the Global Idea Vault's `assets/` (images, audio and video previews).
/// The databases (`project.sqlite` and `vault.sqlite`, which hold every user's private notes),
/// `cache/` (the intelligence index of every member's notes and package staging copies of the
/// database), recovery data and safety backups are never readable by the webview, even if it
/// were compromised (Security review 2026-09-30, WEB-02; security baseline 2026-10-05, C4).
/// `scripts/architecture-lint.mjs` rule C5 keeps it that way.
fn asset_roots(core: &AppCore) -> asset_scope::AssetRoots {
    let mut scope = asset_scope::AssetRoots::default();
    scope.allow_directory(core.config.global_vault_dir.join("assets"), true);
    if let Some(p) = core.project_opt() {
        scope.allow_directory(p.layout.assets(), true);
    }
    scope
}

/// Replace the readable folders after every project change: a closed or switched-away project
/// is no longer readable (build plan item "Asset protocol scope").
fn sync_asset_scope(scope: &asset_scope::AssetScope, core: &AppCore) {
    scope.replace(asset_roots(core));
}

#[tauri::command]
async fn of_invoke(
    app: AppHandle,
    state: State<'_, CoreState>,
    op: String,
    args: Option<Value>,
) -> Result<Value, AppError> {
    let core = state.0.clone();
    let args = args.unwrap_or(Value::Object(Default::default()));
    let started = std::time::Instant::now();
    let op_name = op.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let actor = core.local_actor();
        let r = core.dispatch(&actor, &op, args);
        if op.starts_with("project.") {
            sync_asset_scope(&app.state::<Arc<asset_scope::AssetScope>>(), &core);
        }
        r
    })
    .await
    .map_err(|e| AppError::internal(format!("command task failed: {e}")))?;
    let ms = started.elapsed().as_millis();
    match &result {
        Ok(_) if ms > 250 => tracing::info!(op = %op_name, ms, "slow operation"),
        Ok(_) => tracing::debug!(op = %op_name, ms, "operation"),
        Err(e) => tracing::info!(op = %op_name, code = e.code_str(), ms, "operation failed"),
    }
    result
}

/// Open (or reveal in Explorer) a project asset by id. The path never comes from the UI.
#[tauri::command]
async fn of_open_asset(
    state: State<'_, CoreState>,
    asset_id: String,
    global: Option<bool>,
    reveal: Option<bool>,
) -> Result<(), AppError> {
    let core = state.0.clone();
    // The store lock (ADR-0022, ADR-0023) refuses project files here too, not only operations.
    core.entitlement.check_operation("shell.open_asset")?;
    let kind = if global.unwrap_or(false) {
        openframe_application::StoreKind::Global
    } else {
        openframe_application::StoreKind::Project
    };
    let path: PathBuf = core.with_store(kind, |s| {
        let root = s.root().to_path_buf();
        s.read(|c| openframe_application::util::asset_file_path(c, &root, &asset_id))
    })?;
    if !openframe_security::is_local_disk_path(&path) || !path.exists() {
        return Err(AppError::new(
            "not_found.file",
            "This file isn't available right now. If it's on an external drive, reconnect it or relink the file.",
        ));
    }
    // Files can arrive in received packages with attacker-chosen names: anything Windows
    // would *execute* is shown in File Explorer instead of being launched (PKG-01).
    let reveal = reveal.unwrap_or(false) || openframe_security::is_dangerous_to_open(&path);
    open_path(&path, reveal)
}

/// Reveal a known location (project folder, export result, backup) in Explorer.
/// Only paths that OpenFrame itself produced or registered are accepted.
#[tauri::command]
async fn of_reveal_path(
    state: State<'_, CoreState>,
    kind: String,
    id: Option<String>,
    path: Option<String>,
) -> Result<(), AppError> {
    let core = state.0.clone();
    // The store lock (ADR-0022, ADR-0023): only the data folder and the logs, which the lock
    // screen and support need, stay reachable; project locations are refused like operations.
    // Before the team password of an internal build (ADR-0024) nothing is reachable at all.
    if core.entitlement.team_gate_locked() || !matches!(kind.as_str(), "dataRoot" | "logs") {
        core.entitlement.check_operation("shell.reveal_path")?;
    }
    let target: PathBuf = match kind.as_str() {
        "project" => match id {
            Some(pid) => core.with_app_db(|c| {
                Ok(PathBuf::from(c.query_row(
                    "SELECT path FROM sys_recent_project WHERE project_id=?1",
                    [pid],
                    |r| r.get::<_, String>(0),
                )?))
            })?,
            None => core.project()?.layout.root().to_path_buf(),
        },
        "globalVault" => core.config.global_vault_dir.clone(),
        // The one folder to back up (the data note and the file safety tutorial show it).
        // Created when it does not exist yet, so there is something to show.
        "dataRoot" => {
            let dir = core.config.data_root();
            std::fs::create_dir_all(&dir).map_err(|e| {
                AppError::new(
                    "internal.open_failed",
                    "OpenFrame couldn't open that folder.",
                )
                .with_detail(e.to_string())
            })?;
            dir
        }
        // The folder that should contain a linked file (shown when the file itself is
        // missing). Resolved from the asset id on the Rust side; local drives only.
        "assetFolder" | "globalAssetFolder" => {
            let asset_id = id.unwrap_or_default();
            let store = if kind == "globalAssetFolder" {
                openframe_application::StoreKind::Global
            } else {
                openframe_application::StoreKind::Project
            };
            let file: PathBuf = core.with_store(store, |s| {
                let root = s.root().to_path_buf();
                s.read(|c| openframe_application::util::asset_file_path(c, &root, &asset_id))
            })?;
            let dir = file
                .parent()
                .filter(|d| openframe_security::is_local_disk_path(d) && d.is_dir())
                .ok_or_else(|| AppError::not_found("folder"))?;
            return open_path(dir, false);
        }
        "logs" => core.config.app_data_dir.join("logs"),
        // Only files OpenFrame itself wrote this session to a location the user chose in a
        // save dialog (recorded server-side by the export/package/copy operations). An
        // arbitrary path from the webview (for example a UNC path that would leak credentials)
        // is refused before the filesystem is touched (PATH-01).
        "exported" => {
            let raw = path.unwrap_or_default();
            let p = openframe_security::user_path(&raw, openframe_security::NetworkPaths::Allow)
                .map_err(|_| AppError::not_found("file"))?;
            if !openframe_application::util::is_remembered_output(&core, &p) || !p.exists() {
                return Err(AppError::not_found("file"));
            }
            p
        }
        _ => return Err(AppError::invalid_input("Unknown location.")),
    };
    open_path(&target, true)
}

/// Open a web link from an Idea Vault URL item in the user's browser (explicit user action).
#[tauri::command]
async fn of_open_url(url: String) -> Result<(), AppError> {
    let url = openframe_security::web_link(&url)
        .ok_or_else(|| AppError::invalid_input("Only web links (http or https) can be opened."))?;
    openframe_platform::shell::open_url(url).map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that link.")
            .with_detail(e.to_string())
    })
}

/// One step of the window close handshake (`close_gate`): the page received the request
/// ("flushing"), the user chose to stay ("held"), or the window may close now ("close").
#[tauri::command]
async fn of_close_window(
    window: tauri::Window,
    gate: State<'_, close_gate::CloseGate>,
    step: String,
) -> Result<(), AppError> {
    tracing::info!(step = %step, "window close step from the page");
    match step.as_str() {
        "flushing" => gate.answered(),
        "held" => gate.held(),
        "close" => {
            gate.release();
            if let Err(e) = window.close() {
                // The next close request goes through the page again instead of skipping it.
                gate.unrelease();
                return Err(AppError::new(
                    "internal.close_failed",
                    "OpenFrame couldn't close the window.",
                )
                .with_detail(e.to_string()));
            }
        }
        _ => return Err(AppError::invalid_input("Unknown close step.")),
    }
    Ok(())
}

/// Open a file with its default program, or show it in the file manager, through the platform
/// boundary (`openframe_platform::shell`).
fn open_path(path: &std::path::Path, reveal: bool) -> Result<(), AppError> {
    let r = if reveal {
        openframe_platform::shell::reveal_path(path)
    } else {
        openframe_platform::shell::open_path(path)
    };
    r.map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that file.")
            .with_detail(e.to_string())
    })
}

/// The JSON log subscriber used by the app (and by the log-redaction test).
fn log_subscriber<W>(directives: &str, writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'w> tracing_subscriber::fmt::MakeWriter<'w> + Send + Sync + 'static,
{
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_new(directives).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .json()
        .finish()
}

/// Log filter from `OPENFRAME_LOG` (Security baseline E7, log redaction test / P13).
///
/// Development builds use the variable as written. Release builds accept only plain
/// `level` and `target=level` directives: OpenFrame's own targets (`openframe*`, which never
/// log content) may go up to `trace`, every other crate is capped at `info`, because
/// third-party debug output can quote paths, request bodies or prompts. Span and field
/// filters are dropped. The asset protocol's refusal message (it contains the requested
/// path) is always off in release builds.
fn log_filter_directives(requested: Option<&str>, release: bool) -> String {
    const ASSET_PROTOCOL_TARGET: &str = "tauri::protocol::asset";
    const LEVELS: [&str; 6] = ["off", "error", "warn", "info", "debug", "trace"];
    const INFO: usize = 3;
    let requested = requested.map(str::trim).filter(|r| !r.is_empty());
    if !release {
        return requested.unwrap_or("info").to_string();
    }
    let mut out: Vec<String> = Vec::new();
    for directive in requested.unwrap_or("info").split(',').map(str::trim) {
        let (target, level) = match directive.rsplit_once('=') {
            Some((t, l)) => (Some(t.trim()), l.trim()),
            None => (None, directive),
        };
        let Some(rank) = LEVELS.iter().position(|l| l.eq_ignore_ascii_case(level)) else {
            continue; // Not a plain level (span or field filter, or a typo).
        };
        let capped = LEVELS[rank.min(INFO)];
        match target {
            None => {
                out.push(capped.to_string());
                if rank > INFO {
                    out.push(format!("openframe={}", LEVELS[rank]));
                }
            }
            Some(t)
                if t.is_empty()
                    || t.starts_with(ASSET_PROTOCOL_TARGET)
                    || !t
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') =>
            {
                continue;
            }
            Some(t) if t.starts_with("openframe") => out.push(format!("{t}={}", LEVELS[rank])),
            Some(t) => out.push(format!("{t}={capped}")),
        }
    }
    if out.is_empty() {
        out.push("info".to_string());
    }
    out.push(format!("{ASSET_PROTOCOL_TARGET}=off"));
    out.join(",")
}

fn init_logging(config: &AppConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::util::SubscriberInitExt;
    let dir = config.app_data_dir.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("openframe")
        .filename_suffix("log")
        .max_log_files(14)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let requested = std::env::var("OPENFRAME_LOG").ok();
    let directives = log_filter_directives(requested.as_deref(), !cfg!(debug_assertions));
    let _ = log_subscriber(&directives, writer).try_init();
    // Panics are logged (without project content) before the default hook runs.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        tracing::error!("{}", openframe_security::panic_summary(&message, &location));
        default_hook(info);
    }));
    Some(guard)
}

/// `OPENFRAME_DATA_ROOT=<dir>` runs the app fully isolated in `<dir>` (app data, projects, Global
/// Idea Vault, deleted projects). Used by the desktop end-to-end suite (`tests/e2e`) so it never
/// touches the user's data. The release build honours it too, for file locations only (ADR-0031),
/// so the suite can drive the artifact; every other test switch below stays debug-only.
fn isolated_data_root() -> Option<PathBuf> {
    let root = std::env::var_os("OPENFRAME_DATA_ROOT")?;
    let root = PathBuf::from(root);
    if root.is_absolute() && std::fs::create_dir_all(&root).is_ok() {
        return Some(root);
    }
    // A test folder was asked for but cannot be used: never fall back to the person's real data.
    eprintln!(
        "OPENFRAME_DATA_ROOT is set but is not an absolute folder that can be created; OpenFrame Studio stops."
    );
    std::process::exit(2);
}

/// Deterministic Offline AI answers (temperature 0, fixed seed) for the end-to-end suite: only a
/// debug build on an isolated test data root with `OPENFRAME_AI_DETERMINISTIC=1`.
fn ai_deterministic(release: bool, isolated: bool) -> bool {
    !release && isolated && std::env::var_os("OPENFRAME_AI_DETERMINISTIC").is_some_and(|v| v == "1")
}

/// Whether the hand-set licence state for tests (`entitlement.debug_set`) is available: only in a
/// debug build that runs on an isolated test data root (the end-to-end suite). A release build
/// never registers it, whatever the environment says.
fn entitlement_debug_controls(release: bool, isolated: bool) -> bool {
    !release && isolated
}

/// The release channel for the licence and trial check: a process running from an MSIX package is
/// the Microsoft Store build, a Mac store build (`OPENFRAME_DISTRIBUTION=store`) is the Mac App
/// Store build, and anything else is an internal team build: the Windows installer or the Mac
/// disk image (ADR-0023, ADR-0024).
fn entitlement_channel() -> openframe_entitlement::Channel {
    channel_for(
        openframe_platform::identity::package_family_name().is_some(),
        openframe_platform::current_os() == openframe_platform::Os::MacOs,
        distribution::Channel::current() == distribution::Channel::Store,
    )
}

fn channel_for(packaged: bool, mac: bool, store_build: bool) -> openframe_entitlement::Channel {
    use openframe_entitlement::Channel;
    if packaged {
        Channel::MicrosoftStore
    } else if mac && store_build {
        Channel::MacAppStore
    } else {
        Channel::DirectInstaller
    }
}

/// The team password hash of an internal build (ADR-0024), embedded by `build.rs` from
/// `OPENFRAME_TEAM_PASSWORD` on the build machine. Store packages, and development builds made
/// without the variable, carry none.
#[cfg(openframe_team_gate)]
const TEAM_PASSWORD_HASH: Option<&str> = Some(include_str!(concat!(
    env!("OUT_DIR"),
    "/team_gate_hash.txt"
)));
#[cfg(not(openframe_team_gate))]
const TEAM_PASSWORD_HASH: Option<&str> = None;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let distribution = distribution::Distribution::detect();
    let isolated_root = isolated_data_root();
    // An isolated (automated-test) root keeps deleted projects in its own folder, never the
    // computer's Recycle Bin. A file location, so the release build honours it too (ADR-0031).
    if let Some(root) = &isolated_root {
        openframe_application::modules::project::fault::trash_into(root.join("recycle-bin"));
    }
    let mut config = match &isolated_root {
        Some(root) => AppConfig {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            ..AppConfig::isolated(root)
        },
        None => match AppConfig::for_current_user(env!("CARGO_PKG_VERSION")) {
            Ok(config) => config,
            // No Documents or app data folder: say so plainly and stop, never use a temp folder (W2).
            Err(e) => {
                distribution::show_startup_error(&e.message);
                return;
            }
        },
    };
    if isolated_root.is_none() {
        // MSIX: keep app-private data in the package's own (real, non-virtualized) data folder.
        config.app_data_dir = distribution.app_data_dir(config.app_data_dir.clone());
    }
    config.channel = entitlement_channel();
    config.ai_bundle_dir = AppConfig::installed_ai_bundle_dir();
    // Fixed at build time (build.rs; ADR-0032): Hugging Face at the pinned revisions, or the local
    // test host a test build names. Nothing at run time can change it.
    config.ai_model_source =
        openframe_application::ModelSource::from_build(option_env!("OPENFRAME_TEST_MODEL_SOURCE"));
    config.ai_deterministic = ai_deterministic(!cfg!(debug_assertions), isolated_root.is_some());
    // The Mac App Store build hands over the StoreKit bridge and the licence server client; the
    // first check runs on a background thread after setup. Windows: none.
    config.mac_inputs = platform::mac_inputs();
    // The Microsoft Store build's Store calls (GetAppLicenseAsync, RequestPurchaseAsync with the
    // window handle, and a refresh on OfflineLicensesChanged) need the Partner Center product
    // identity, which does not exist yet (ADR-0023). Until they are wired here as
    // `config.microsoft_store`, that build reports NeedsOnlineCheck: a gap in the build, not a
    // user state.
    config.entitlement_debug_controls =
        entitlement_debug_controls(!cfg!(debug_assertions), isolated_root.is_some());
    // Internal builds ask the team password before anything opens; the core enforces it.
    config.team_password_hash = TEAM_PASSWORD_HASH.map(str::to_string);
    let log_guard = init_logging(&config);
    if config.ai_model_source.is_none() {
        // The value is a loopback test address, never a secret: it says what was wrong.
        tracing::error!(
            value = option_env!("OPENFRAME_TEST_MODEL_SOURCE").unwrap_or(""),
            "the test model source this build was made with is not valid; OpenFrame AI cannot be downloaded"
        );
    }
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        build = distribution::CHANNEL_MARKER,
        channel = distribution.channel.as_str(),
        packaged = distribution.is_packaged(),
        in_app_updates = distribution.in_app_updates_allowed(),
        "OpenFrame Studio starting"
    );
    if !distribution::ensure_webview2() {
        drop(log_guard);
        return;
    }
    // No in-app updater is integrated yet (docs/engineering/24 §3). When it is, compile it only
    // under `#[cfg(not(openframe_store))]` and register it only if
    // `distribution.in_app_updates_allowed()`: Store builds are updated by the Microsoft Store.

    let mut builder = tauri::Builder::default();
    // An isolated (automated-test) instance must not hand over to, or be
    // blocked by, the user's own running OpenFrame Studio.
    if isolated_root.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }
    let assets = Arc::new(asset_scope::AssetScope::default());
    let protocol_scope = assets.clone();
    // A test run keeps the web view's profile (cache, local storage) in its own folder too (ADR-0031).
    let webview_dir = isolated_root.as_ref().map(|root| root.join("webview"));
    builder
        .register_asynchronous_uri_scheme_protocol(asset_scope::SCHEME, move |_ctx, request, responder| {
            let scope = protocol_scope.clone();
            // File reads stay off the webview's thread.
            std::thread::spawn(move || responder.respond(scope.respond(&request)));
        })
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let sink = Arc::new(TauriSink { app: app.handle().clone() });
            let core = AppCore::new(config, sink).map_err(|e| {
                tracing::error!(code = e.code_str(), detail = %openframe_security::redact(e.detail.as_deref().unwrap_or("")), "failed to start application core");
                Box::<dyn std::error::Error>::from(e.message.clone())
            })?;
            sync_asset_scope(&assets, &core);
            app.manage(assets.clone());
            // Sleep and wake: the open project's data is folded into its file before sleep.
            let power_core = Arc::downgrade(&core);
            let power = openframe_platform::power::subscribe(Arc::new(move |event| {
                if let Some(core) = power_core.upgrade() {
                    core.on_power_event(event);
                }
            }));
            if power.is_none() {
                tracing::info!("sleep and wake notifications are not available");
            }
            app.manage(PowerState(parking_lot::Mutex::new(power)));
            platform::start_entitlement_checks(&core);
            // App start: Recently Deleted in the Global Idea Vault drops items older than 30 days.
            // Off the main thread; an internal build does it after the team password instead.
            let start_core = core.clone();
            std::thread::spawn(move || start_core.startup_maintenance());
            app.manage(CoreState(core));
            // The main window is built here from its tauri.conf.json entry ("create": false), after
            // the core is managed, so its first command always finds the core, and a test run can
            // give its web view a profile folder of its own.
            let main = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .cloned()
                .ok_or("tauri.conf.json has no main window")?;
            let mut window = tauri::WebviewWindowBuilder::from_config(app.handle(), &main)?;
            if let Some(dir) = webview_dir {
                window = window.data_directory(dir);
            }
            window.build()?;
            Ok(())
        })
        .manage(close_gate::CloseGate::default())
        .on_window_event(|window, event| {
            platform::on_window_event(event);
            // The close button, Alt+F4 and the system: hold the window until the page has stored
            // the text its editors still hold, or until it fails to answer (close_gate).
            if let tauri::WindowEvent::CloseRequested { api, .. } = event
                && let Some(gate) = window.try_state::<close_gate::CloseGate>()
                && let close_gate::Request::Hold { ask, round } = gate.on_request()
            {
                tracing::info!(ask, round, "window close requested; held for the page");
                api.prevent_close();
                if ask {
                    if let Err(e) = window.emit(close_gate::EVENT, ()) {
                        tracing::warn!(error = %e, "the close request could not reach the page; closing");
                        gate.release();
                        let _ = window.destroy();
                        return;
                    }
                    let w = window.clone();
                    std::thread::spawn(move || {
                        // First: did the page answer at all? Then: did it finish storing?
                        for (wait, waited_for_answer) in [
                            (close_gate::ANSWER_WAIT, true),
                            (close_gate::FLUSH_WAIT, false),
                        ] {
                            std::thread::sleep(wait);
                            if w.state::<close_gate::CloseGate>().try_force(round, waited_for_answer) {
                                tracing::warn!(waited_for_answer, "the page stopped answering the close request; closing");
                                if let Err(e) = w.close() {
                                    w.state::<close_gate::CloseGate>().unrelease();
                                    tracing::error!(error = %e, "closing the window after the wait failed");
                                }
                                return;
                            }
                        }
                    });
                }
            }
            if let tauri::WindowEvent::Destroyed = event
                && let Some(state) = window.try_state::<CoreState>()
            {
                state.0.shutdown();
            }
        })
        .invoke_handler(tauri::generate_handler![of_invoke, of_open_asset, of_reveal_path, of_open_url, of_close_window])
        .build(tauri::generate_context!())
        .expect("error while running OpenFrame Studio")
        .run(|app, event| platform::on_run_event(app, &event));
    drop(log_guard);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic AI settings are test-only: a release build, or a debug build on the user's
    /// real data, always samples normally. The engine is always the installed one.
    #[test]
    fn ai_test_settings_never_reach_a_release_or_a_real_data_root() {
        assert!(!ai_deterministic(true, true));
        assert!(!ai_deterministic(false, false));
        assert!(
            AppConfig::installed_ai_bundle_dir()
                .is_some_and(|d| d.ends_with("ai") && d.is_absolute())
        );
    }

    #[test]
    fn store_packages_are_store_channels_and_everything_else_is_an_internal_build() {
        use openframe_entitlement::Channel;
        assert_eq!(channel_for(true, false, true), Channel::MicrosoftStore);
        assert_eq!(channel_for(true, false, false), Channel::MicrosoftStore);
        assert_eq!(channel_for(false, true, true), Channel::MacAppStore);
        // The internal Mac disk image and the internal Windows installer.
        assert_eq!(channel_for(false, true, false), Channel::DirectInstaller);
        assert_eq!(channel_for(false, false, false), Channel::DirectInstaller);
    }

    /// A store package never carries the team password hash (ADR-0024 decision 6).
    #[test]
    fn a_store_build_embeds_no_team_password_hash() {
        if cfg!(openframe_store) {
            assert!(TEAM_PASSWORD_HASH.is_none());
        }
        if let Some(hash) = TEAM_PASSWORD_HASH {
            assert!(
                hash.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"),
                "{hash}"
            );
        }
    }

    #[test]
    fn release_builds_never_expose_the_hand_set_licence_state() {
        assert!(!entitlement_debug_controls(true, true));
        assert!(!entitlement_debug_controls(true, false));
        assert!(!entitlement_debug_controls(false, false));
        assert!(entitlement_debug_controls(false, true));
    }

    #[test]
    fn release_builds_on_a_test_folder_keep_every_other_test_switch_off() {
        // ADR-0031: the release build honours the test data folder, and nothing else.
        assert!(!ai_deterministic(true, true));
        assert!(!entitlement_debug_controls(true, true));
    }

    // ---------------------------------------------------------------- log redaction test log filter (P13)

    #[test]
    fn release_builds_cap_third_party_targets_at_info() {
        let f = |r: &str| log_filter_directives(Some(r), true);
        assert_eq!(
            f("trace"),
            "info,openframe=trace,tauri::protocol::asset=off"
        );
        assert_eq!(
            f("debug"),
            "info,openframe=debug,tauri::protocol::asset=off"
        );
        assert_eq!(
            f("warn,reqwest=trace,hyper=debug,openframe_ai=trace"),
            "warn,reqwest=info,hyper=info,openframe_ai=trace,tauri::protocol::asset=off"
        );
        // Span/field filters and junk are dropped; the asset protocol cannot be re-enabled.
        assert_eq!(
            f(
                "tauri[cmd{path}]=trace,=debug,x y=info,tauri::protocol::asset=trace,openframe=bogus"
            ),
            "info,tauri::protocol::asset=off"
        );
        assert_eq!(
            log_filter_directives(None, true),
            "info,tauri::protocol::asset=off"
        );
        assert_eq!(f("  "), "info,tauri::protocol::asset=off");
        // Lowering is always allowed.
        assert_eq!(f("ERROR"), "error,tauri::protocol::asset=off");
    }

    #[test]
    fn development_builds_use_openframe_log_as_written() {
        assert_eq!(
            log_filter_directives(Some("hyper=trace"), false),
            "hyper=trace"
        );
        assert_eq!(log_filter_directives(None, false), "info");
    }

    #[test]
    fn capped_directives_parse_and_filter_as_intended() {
        use tracing_subscriber::layer::SubscriberExt;
        let filter =
            tracing_subscriber::EnvFilter::try_new(log_filter_directives(Some("trace"), true))
                .expect("valid directives");
        let sub = tracing_subscriber::registry().with(filter);
        tracing::subscriber::with_default(sub, || {
            assert!(tracing::enabled!(target: "openframe_application", tracing::Level::TRACE));
            assert!(!tracing::enabled!(target: "reqwest", tracing::Level::DEBUG));
            assert!(tracing::enabled!(target: "reqwest", tracing::Level::INFO));
            assert!(!tracing::enabled!(target: "tauri::protocol::asset", tracing::Level::ERROR));
        });
    }

    // ---------------------------------------------------------------- asset protocol scope

    /// Opening, switching and closing projects through the real dispatcher replaces the folders
    /// the `ofasset` protocol may read: the project that was left is refused at once.
    #[test]
    fn asset_scope_follows_project_open_switch_and_close() {
        use openframe_application::events::RecordingSink;
        use serde_json::json;
        let base = tempfile::tempdir().unwrap();
        let core = AppCore::new(
            AppConfig::isolated(&base.path().join("env")),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let actor = core.local_actor();
        let scope = asset_scope::AssetScope::default();
        let create = |title: &str| {
            core.dispatch(
                &actor,
                "project.create",
                json!({ "title": title, "projectType": "Feature Film",
                        "parentDir": base.path().to_string_lossy() }),
            )
            .unwrap();
            sync_asset_scope(&scope, &core);
            let assets = core.project().unwrap().layout.assets();
            std::fs::create_dir_all(&assets).unwrap();
            std::fs::write(assets.join("still.png"), b"png").unwrap();
            (
                core.project().unwrap().layout.root().to_path_buf(),
                assets.join("still.png"),
            )
        };
        let url = |p: &std::path::Path| {
            let mut s = String::from("/");
            for b in p.to_string_lossy().as_bytes() {
                if b.is_ascii_alphanumeric() {
                    s.push(*b as char);
                } else {
                    s.push_str(&format!("%{b:02X}"));
                }
            }
            s
        };
        let (root_a, still_a) = create("Railway");
        assert!(
            scope.resolve(&url(&still_a)).is_some(),
            "open project readable"
        );
        assert!(
            scope
                .resolve(&url(&root_a.join("project.sqlite")))
                .is_none()
        );
        // Switching to another project revokes the first one.
        let (_root_b, still_b) = create("Harbour");
        assert!(
            scope.resolve(&url(&still_a)).is_none(),
            "switched-away project refused"
        );
        assert!(scope.resolve(&url(&still_b)).is_some());
        // Closing revokes the open one; only the Global Idea Vault stays.
        core.dispatch(&actor, "project.close", json!({})).unwrap();
        sync_asset_scope(&scope, &core);
        assert!(
            scope.resolve(&url(&still_b)).is_none(),
            "closed project refused"
        );
        assert_eq!(
            scope.roots().dirs(),
            [core.config.global_vault_dir.join("assets")]
        );
        // Reopening the first project makes it readable again.
        core.dispatch(
            &actor,
            "project.open",
            json!({ "path": root_a.to_string_lossy() }),
        )
        .unwrap();
        sync_asset_scope(&scope, &core);
        assert!(
            scope.resolve(&url(&still_a)).is_some(),
            "reopened project readable"
        );
        core.shutdown();
    }

    // ---------------------------------------------------------------- log redaction test no content in logs

    #[derive(Clone, Default)]
    struct Capture(Arc<parking_lot::Mutex<Vec<u8>>>);

    impl std::io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'w> tracing_subscriber::fmt::MakeWriter<'w> for Capture {
        type Writer = Capture;
        fn make_writer(&'w self) -> Self::Writer {
            self.clone()
        }
    }

    /// Real operations with distinctive content, logged through the production subscriber with
    /// OpenFrame's own targets at `trace` (the most a release build allows): no marker may
    /// reach the log. Covers screenplay text, notes, contacts, AI prompts, project paths and
    /// key-like values (CORE-014, log redaction test).
    #[test]
    fn real_operations_never_write_content_to_the_log() {
        use openframe_application::events::RecordingSink;
        use serde_json::json;
        use tracing_subscriber::util::SubscriberInitExt;

        const MARKERS: [&str; 9] = [
            "OFMARKERFOLDER7731",
            "OFMARKERTITLE7731",
            "OFMARKERSCENE7731",
            "OFMARKERSCREEN7731",
            "OFMARKERNOTE7731",
            "OFMARKERPERSON7731",
            "ofmarker.contact7731@example.com",
            "OFMARKERPROMPT7731",
            "OFMARKERKEY7731",
        ];

        let capture = Capture::default();
        let directives = log_filter_directives(Some("trace"), true);
        log_subscriber(&directives, capture.clone())
            .try_init()
            .expect("this test owns the global subscriber");

        let base = std::env::temp_dir().join(format!(
            "of-log-redaction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let parent = base.join("Projects").join(MARKERS[0]);
        std::fs::create_dir_all(&parent).unwrap();
        let core = AppCore::new(
            AppConfig::isolated(&base.join("env")),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        let actor = core.local_actor();
        let call = |op: &str, args: Value| core.dispatch(&actor, op, args);
        let ok = |op: &str, args: Value| {
            call(op, args).unwrap_or_else(|e| panic!("{op} failed: {}", e.code_str()))
        };

        ok(
            "project.create",
            json!({ "title": MARKERS[1], "projectType": "Feature Film",
                    "parentDir": parent.to_string_lossy() }),
        );
        let root = core.project().unwrap().layout.root().to_path_buf();
        let draft = ok("screenplay.create", json!({}))["draft"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let doc = ok("screenplay.document", json!({ "draftId": draft }));
        let scene = doc["scenes"][0]["id"].as_str().unwrap().to_string();
        let element = doc["scenes"][0]["elements"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        ok(
            "screenplay.apply_edits",
            json!({ "draftId": draft, "ops": [
                { "op": "updateScene", "id": scene, "heading": format!("INT. {} - NIGHT", MARKERS[2]) },
                { "op": "updateElement", "id": element, "text": format!("{} api_key={}", MARKERS[3], MARKERS[8]) },
            ] }),
        );
        ok(
            "notes.create",
            json!({ "title": MARKERS[4], "body": format!("{} token={}", MARKERS[4], MARKERS[8]) }),
        );
        let crew = ok(
            "crew.create",
            json!({ "personName": MARKERS[5], "role": "Sound Recordist", "department": "Sound" }),
        );
        ok(
            "crew.update",
            json!({ "id": crew["id"], "contact": MARKERS[6] }),
        );
        let _ = call("search.query", json!({ "text": MARKERS[3] }));
        let _ = call(
            "ai.ask",
            json!({ "text": format!("{} {}", MARKERS[7], MARKERS[3]), "scope": { "kind": "WholeProject" } }),
        );
        // Failures carry the markers in their arguments too.
        let _ = call(
            "project.open",
            json!({ "path": parent.join(format!("{}.openframe", MARKERS[1])).join("missing") }),
        );
        let _ = call(
            "notes.create",
            json!({ "title": MARKERS[4], "bogus": MARKERS[8] }),
        );
        // Close and reopen: the project lock path (with the project name) used to be logged.
        ok("project.close", json!({}));
        ok("project.open", json!({ "path": root.to_string_lossy() }));
        ok("project.close", json!({}));
        core.shutdown();
        drop(core);
        std::thread::sleep(std::time::Duration::from_millis(300));

        let log = String::from_utf8_lossy(&capture.0.lock()).into_owned();
        let _ = std::fs::remove_dir_all(&base);
        // Positive control: OpenFrame's own debug output was captured.
        assert!(
            log.contains("project lock released"),
            "the capture saw OpenFrame's debug output:\n{log}"
        );
        let lower = log.to_lowercase();
        for marker in MARKERS {
            assert!(
                !lower.contains(&marker.to_lowercase()),
                "log contains {marker}:\n{log}"
            );
        }
        let temp = base.to_string_lossy().to_lowercase();
        let json_temp = temp.replace('\\', "\\\\");
        assert!(
            !lower.contains(&temp) && !lower.contains(&json_temp),
            "log contains the project path"
        );
    }
}
