//! Tauri adapter (ESD §5: "Tauri is an adapter, not the application architecture").
//!
//! The webview gets exactly one data command, `of_invoke(op, args)`, which is
//! checked against the application's operation registry (the IPC allow-list).
//! File opening/revealing is resolved on the Rust side from asset/project ids,
//! so the webview never holds filesystem, shell or network privileges.

mod distribution;

use std::path::PathBuf;
use std::sync::Arc;

use openframe_application::events::{AppEvent, EventSink};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::AppError;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

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

/// Allow the restricted asset protocol to read only managed media: the open project's
/// `assets/` and `cache/` folders and the Global Idea Vault's `assets/` (images, audio and
/// video previews). The databases (`project.sqlite`, `vault.sqlite` — which hold every
/// user's private notes), recovery data and safety backups are never readable by the
/// webview, even if it were compromised (Security review 2026-09-30, WEB-02).
fn sync_asset_scope(app: &AppHandle, core: &AppCore) {
    let scope = app.asset_protocol_scope();
    let _ = scope.allow_directory(core.config.global_vault_dir.join("assets"), true);
    if let Some(p) = core.project_opt() {
        let _ = scope.allow_directory(p.layout.assets(), true);
        let _ = scope.allow_directory(p.layout.cache(), true);
    }
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
            sync_asset_scope(&app, &core);
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
    app: AppHandle,
    state: State<'_, CoreState>,
    asset_id: String,
    global: Option<bool>,
    reveal: Option<bool>,
) -> Result<(), AppError> {
    let core = state.0.clone();
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
    open_path(&app, &path, reveal)
}

/// Reveal a known location (project folder, export result, backup) in Explorer.
/// Only paths that OpenFrame itself produced or registered are accepted.
#[tauri::command]
async fn of_reveal_path(
    app: AppHandle,
    state: State<'_, CoreState>,
    kind: String,
    id: Option<String>,
    path: Option<String>,
) -> Result<(), AppError> {
    let core = state.0.clone();
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
            return open_path(&app, dir, false);
        }
        "logs" => core.config.app_data_dir.join("logs"),
        // Only files OpenFrame itself wrote this session to a location the user chose in a
        // save dialog (recorded server-side by the export/package/copy operations). An
        // arbitrary path from the webview — e.g. a UNC path that would leak credentials —
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
    open_path(&app, &target, true)
}

/// Open a web link from an Idea Vault URL item in the user's browser (explicit user action).
#[tauri::command]
async fn of_open_url(app: AppHandle, url: String) -> Result<(), AppError> {
    let url = openframe_security::web_link(&url)
        .ok_or_else(|| AppError::invalid_input("Only web links (http or https) can be opened."))?;
    app.opener().open_url(url, None::<&str>).map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that link.")
            .with_detail(e.to_string())
    })
}

fn open_path(app: &AppHandle, path: &std::path::Path, reveal: bool) -> Result<(), AppError> {
    let r = if reveal {
        app.opener().reveal_item_in_dir(path)
    } else {
        app.opener().open_path(path.to_string_lossy(), None::<&str>)
    };
    r.map_err(|e| {
        AppError::new("internal.open_failed", "OpenFrame couldn't open that file.")
            .with_detail(e.to_string())
    })
}

fn init_logging(config: &AppConfig) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::EnvFilter;
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
    let filter =
        EnvFilter::try_from_env("OPENFRAME_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .json()
        .try_init();
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

/// Development/test builds only: `OPENFRAME_DATA_ROOT=<dir>` runs the app fully
/// isolated in `<dir>` (app data, projects, Global Idea Vault). Used by the
/// desktop end-to-end suite (`tests/e2e`) so it never touches the user's data.
/// Release builds ignore it.
fn isolated_data_root() -> Option<PathBuf> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let root = std::env::var_os("OPENFRAME_DATA_ROOT")?;
    let root = PathBuf::from(root);
    (root.is_absolute() && std::fs::create_dir_all(&root).is_ok()).then_some(root)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let distribution = distribution::Distribution::detect();
    let isolated_root = isolated_data_root();
    let mut config = match &isolated_root {
        Some(root) => AppConfig {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            ..AppConfig::isolated(root)
        },
        None => AppConfig::for_current_user(env!("CARGO_PKG_VERSION")),
    };
    if isolated_root.is_none() {
        // MSIX: keep app-private data in the package's own (real, non-virtualized) data folder.
        config.app_data_dir = distribution.app_data_dir(config.app_data_dir.clone());
    }
    let log_guard = init_logging(&config);
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
    // An isolated (automated-test) instance must not hand over to — or be
    // blocked by — the user's own running OpenFrame Studio.
    if isolated_root.is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let sink = Arc::new(TauriSink { app: app.handle().clone() });
            let core = AppCore::new(config.clone(), sink).map_err(|e| {
                tracing::error!(code = e.code_str(), detail = %openframe_security::redact(e.detail.as_deref().unwrap_or("")), "failed to start application core");
                Box::<dyn std::error::Error>::from(e.message.clone())
            })?;
            sync_asset_scope(app.handle(), &core);
            app.manage(CoreState(core));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event
                && let Some(state) = window.try_state::<CoreState>()
            {
                state.0.shutdown();
            }
        })
        .invoke_handler(tauri::generate_handler![of_invoke, of_open_asset, of_reveal_path, of_open_url])
        .run(tauri::generate_context!())
        .expect("error while running OpenFrame Studio");
    drop(log_guard);
}
