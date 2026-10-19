//! Application events pushed to the UI (ESD §6 step 8–9).
//!
//! The UI never polls for correctness: every committed mutation emits
//! `DataChanged` listing the tables touched, and each React query declares the
//! tables it reads, so invalidation is exact and module-agnostic.

use serde::Serialize;
use ts_rs::TS;

use openframe_domain::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoreKind {
    Project,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum SaveStatus {
    Saved,
    Saving,
    /// Saved locally; a package/export/external operation is still running.
    SavedPendingExternal,
    Error,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SaveState {
    pub status: SaveStatus,
    #[ts(type = "number | null")]
    pub last_saved_at: Option<i64>,
    pub error: Option<AppError>,
    pub pending_operations: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub task_id: String,
    pub kind: String,
    pub label: String,
    /// queued | running | completed | failed | cancelled
    pub state: String,
    /// 0..=1 when measurable.
    pub progress: Option<f64>,
    pub message: Option<String>,
    pub error: Option<AppError>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppEvent {
    #[serde(rename_all = "camelCase")]
    DataChanged {
        store: StoreKind,
        tables: Vec<String>,
        ids: Vec<String>,
        origin: String,
    },
    SaveState(SaveState),
    #[serde(rename_all = "camelCase")]
    ProjectOpened {
        project_id: String,
    },
    #[serde(rename_all = "camelCase")]
    ProjectClosed {
        project_id: String,
    },
    Task(TaskProgress),
    /// Module-specific notification (AI runtime state, download progress, …).
    #[serde(rename_all = "camelCase")]
    Module {
        module: String,
        name: String,
        payload: serde_json::Value,
    },
}

/// Implemented by the Tauri adapter (and by tests).
pub trait EventSink: Send + Sync {
    fn emit(&self, event: &AppEvent);
}

/// Sink that drops events (headless tools, some tests).
pub struct NullSink;
impl EventSink for NullSink {
    fn emit(&self, _event: &AppEvent) {}
}

/// Sink that records events (tests).
#[derive(Default)]
pub struct RecordingSink {
    pub events: parking_lot::Mutex<Vec<AppEvent>>,
}
impl EventSink for RecordingSink {
    fn emit(&self, event: &AppEvent) {
        self.events.lock().push(event.clone());
    }
}
