//! Long-lived Offline AI service held by `AppCore` (the bundled AI and its two engines,
//! ADR-0025). Created lazily on the first `ai.*` operation; the rest of OpenFrame never depends
//! on it (FSD-AI-001).

use std::sync::Arc;

use openframe_ai::{AiManager, InferenceProvider, ManagerConfig};
use openframe_domain::{AppError, AppResult};
use parking_lot::RwLock;
use serde_json::json;

use crate::core::{AppConfig, AppCore};
use crate::events::{AppEvent, EventSink};

pub struct AiService {
    manager: Option<Arc<AiManager>>,
    init_error: Option<AppError>,
    attached: RwLock<Option<Arc<dyn InferenceProvider>>>,
    events: Arc<dyn EventSink>,
}

pub fn service(core: &AppCore) -> Arc<AiService> {
    core.service(|| AiService::new(&core.config, core.events.clone()))
}

/// Emit an AI module event (engine state); the panel refreshes on it.
pub fn emit(events: &Arc<dyn EventSink>, name: &str, payload: serde_json::Value) {
    events.emit(&AppEvent::Module {
        module: "ai".into(),
        name: name.into(),
        payload,
    });
}

fn unavailable() -> AppError {
    AppError::ai(
        "unavailable",
        "AI is currently unavailable. OpenFrame's core workflows continue to work normally.",
    )
}

impl AiService {
    fn new(config: &AppConfig, events: Arc<dyn EventSink>) -> Self {
        let made = ManagerConfig::production(
            &config.app_data_dir,
            config.ai_bundle_dir.clone(),
            config.ai_model_source.clone(),
        )
        .map(|mut c| {
            c.deterministic = config.ai_deterministic;
            c
        })
        .and_then(AiManager::new);
        match made {
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
            events,
        }
    }

    pub fn manager(&self) -> AppResult<&Arc<AiManager>> {
        self.manager
            .as_ref()
            .ok_or_else(|| self.init_error.clone().unwrap_or_else(unavailable))
    }

    /// The chat model answering requests: an attached local backend, else the bundled engine
    /// when the bundled AI is available.
    pub fn model(&self) -> Option<Arc<dyn InferenceProvider>> {
        if let Some(m) = self.attached.read().clone() {
            return Some(m);
        }
        self.manager.as_ref().and_then(|m| m.chat_model())
    }

    /// Why no model can answer: the bundled AI is missing, damaged or unsupported here.
    pub fn unavailable_reason(&self) -> AppError {
        match &self.manager {
            Some(m) => m.unavailable_reason(),
            None => self.init_error.clone().unwrap_or_else(unavailable),
        }
    }

    /// Attach (or detach) an in-process local model backend. The bundled engine is used when
    /// none is attached. Swapping backends never changes data semantics or authorization
    /// (AI-AC-025).
    pub fn attach_model(&self, model: Option<Arc<dyn InferenceProvider>>) {
        *self.attached.write() = model;
        emit(&self.events, "runtime", json!({ "attached": true }));
    }

    pub fn has_attached_model(&self) -> bool {
        self.attached.read().is_some()
    }
}
