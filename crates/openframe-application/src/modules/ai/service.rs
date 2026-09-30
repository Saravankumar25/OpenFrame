//! Long-lived Offline AI service held by `AppCore` (model manager, sidecar,
//! install progress). Created lazily on the first `ai.*` operation; the rest
//! of OpenFrame never depends on it (FSD-AI-001).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use openframe_ai::{AiManager, ChatModel, InstallEvent, InstallPhase, ManagerConfig};
use openframe_domain::{AppError, AppResult};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use serde_json::json;
use ts_rs::TS;

use crate::core::{AppConfig, AppCore};
use crate::events::{AppEvent, EventSink};

/// Download/installation progress shown in the panel.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiInstallProgress")]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub task_id: String,
    pub profile_id: Option<String>,
    /// checking | downloadingRuntime | downloadingModel | verifying | installing | starting | ready | paused | cancelled | failed
    pub phase: String,
    #[ts(type = "number")]
    pub bytes_done: u64,
    #[ts(type = "number")]
    pub bytes_total: u64,
    pub message: String,
    pub error: Option<AppError>,
    /// True while the background task is running.
    pub active: bool,
}

pub struct AiService {
    manager: Option<Arc<AiManager>>,
    init_error: Option<AppError>,
    attached: RwLock<Option<Arc<dyn ChatModel>>>,
    pub(crate) install: Mutex<Option<InstallProgress>>,
    /// (task id, discard-partials flag) of the running installation.
    pub(crate) install_task: Mutex<Option<(String, Arc<AtomicBool>)>>,
    events: Arc<dyn EventSink>,
}

pub fn service(core: &AppCore) -> Arc<AiService> {
    core.service(|| AiService::new(&core.config, core.events.clone()))
}

/// Emit an AI module event (runtime state / install progress); the panel refreshes on it.
pub fn emit(events: &Arc<dyn EventSink>, name: &str, payload: serde_json::Value) {
    events.emit(&AppEvent::Module {
        module: "ai".into(),
        name: name.into(),
        payload,
    });
}

impl AiService {
    fn new(config: &AppConfig, events: Arc<dyn EventSink>) -> Self {
        match AiManager::new(ManagerConfig::production(&config.app_data_dir)) {
            Ok(m) => {
                let ev = events.clone();
                m.set_state_listener(Arc::new(move |s| {
                    emit(&ev, "runtime", json!({ "state": s }))
                }));
                Self::with_manager(Some(Arc::new(m)), None, events)
            }
            Err(e) => {
                tracing::error!(
                    code = e.code_str(),
                    "Offline AI could not initialise; the rest of OpenFrame is unaffected"
                );
                Self::with_manager(None, Some(e), events)
            }
        }
    }

    fn with_manager(
        manager: Option<Arc<AiManager>>,
        init_error: Option<AppError>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            manager,
            init_error,
            attached: RwLock::new(None),
            install: Mutex::new(None),
            install_task: Mutex::new(None),
            events,
        }
    }

    pub fn manager(&self) -> AppResult<&Arc<AiManager>> {
        self.manager.as_ref().ok_or_else(|| {
            self.init_error.clone().unwrap_or_else(|| AppError::ai("unavailable", "AI is currently unavailable. OpenFrame's core workflows continue to work normally."))
        })
    }

    /// The chat model answering requests: an attached local backend, else the
    /// OpenFrame-managed sidecar when Offline AI is installed.
    pub fn model(&self) -> Option<Arc<dyn ChatModel>> {
        if let Some(m) = self.attached.read().clone() {
            return Some(m);
        }
        self.manager.as_ref().and_then(|m| m.chat_model())
    }

    /// Attach (or detach) an in-process local model backend. The managed
    /// sidecar is used when none is attached. Swapping backends never changes
    /// data semantics or authorization (AI-AC-025).
    pub fn attach_model(&self, model: Option<Arc<dyn ChatModel>>) {
        *self.attached.write() = model;
        emit(&self.events, "runtime", json!({ "attached": true }));
    }

    pub fn has_attached_model(&self) -> bool {
        self.attached.read().is_some()
    }

    pub(crate) fn set_install(&self, p: Option<InstallProgress>) {
        *self.install.lock() = p.clone();
        emit(
            &self.events,
            "install",
            serde_json::to_value(&p).unwrap_or(serde_json::Value::Null),
        );
    }

    pub(crate) fn update_install(&self, ev: &InstallEvent) {
        let mut g = self.install.lock();
        if let Some(p) = g.as_mut() {
            let (phase, msg) = phase_text(ev.phase);
            p.phase = phase.into();
            p.message = msg.into();
            p.bytes_done = ev.done;
            p.bytes_total = ev.total;
            p.active = ev.phase != InstallPhase::Ready;
            let snapshot = p.clone();
            drop(g);
            emit(
                &self.events,
                "install",
                serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null),
            );
        }
    }
}

pub fn phase_text(p: InstallPhase) -> (&'static str, &'static str) {
    match p {
        InstallPhase::Checking => ("checking", "Checking this computer…"),
        InstallPhase::DownloadingRuntime => {
            ("downloadingRuntime", "Downloading the Offline AI engine…")
        }
        InstallPhase::DownloadingModel => ("downloadingModel", "Downloading the AI model…"),
        InstallPhase::Verifying => ("verifying", "Verifying the download…"),
        InstallPhase::Installing => ("installing", "Installing…"),
        InstallPhase::Starting => ("starting", "Starting Offline AI for a first test…"),
        InstallPhase::Ready => ("ready", "AI Ready"),
    }
}
