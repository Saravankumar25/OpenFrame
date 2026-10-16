//! OpenFrame Offline AI subsystem (Local AI Runtime & Model Management spec; agentic AI spec
//! §12, §23; ADR-0025).
//!
//! ```text
//! AiManager ─┬─ bundle     the AI shipped inside the app: pinned lock (compiled in), read-only
//!            │             resources (engine + chat model + embedding model), hash check before
//!            │             the first start, cached per installed version in app data
//!            ├─ hardware   memory and processor (diagnostics, "may be slow"), never transmitted
//!            ├─ selection  plain-language fitness hint
//!            ├─ store      writable app data: check record, engine record, logs
//!            ├─ engines    pinned engine files re-hashed before every launch; stray engine sweep
//!            ├─ supervisor two `llama-server` engines (chat + `--embedding`), loopback only
//!            └─ model      InferenceProvider over the OpenAI-compatible loopback client
//! ```
//!
//! Nothing here downloads anything. This crate has no project-database access. The chat model
//! can only produce text or schema-constrained JSON and the embedding model only vectors; the
//! application layer validates them and decides what (if anything) happens.

pub mod bundle;
pub mod client;
pub mod download;
pub mod engines;
pub mod fetch;
pub mod gpu;
pub mod hardware;
pub mod install;
pub mod manager;
pub mod model;
pub mod rt;
pub mod selection;
pub mod store;
pub mod supervisor;

pub use bundle::{AssetLock, BUNDLE_ID, BundledAi, RESOURCE_FOLDER};
pub use client::{ChatMessage, ChatRequest, Usage};
pub use manager::{
    AiError, AiManager, BundleState, ComponentKind, Diagnostics, EmbeddingInfo, ManagerConfig,
    OfflineAiState,
};
pub use model::{InferenceProvider, ProviderCapabilities};
pub use supervisor::{RuntimeState, RuntimeStatus};
