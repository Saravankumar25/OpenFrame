//! OpenFrame Offline AI subsystem (Local AI Runtime & Model Management spec; agentic AI spec
//! §12, §23–§25).
//!
//! ```text
//! AiManager ─┬─ manifest   signed manifest: ONE profile (openframe-local-ai-v1) =
//!            │             llama.cpp runtime + chat model + embedding model (Ed25519, compiled-in key)
//!            ├─ hardware   RAM / CPU / GPU (DXGI) / disk inspection — never transmitted
//!            ├─ selection  device check: graphics card or processor, disk needed, "may be slow"
//!            ├─ download   resumable, retried, SHA-256 verified, quarantining downloads
//!            ├─ store      %LOCALAPPDATA%/OpenFrame/{models,runtimes} layout + atomic activation
//!            ├─ supervisor two `llama-server` sidecars (chat + `--embedding`), loopback only
//!            └─ model      ChatModel adapter over the OpenAI-compatible client
//! ```
//!
//! This crate has no project-database access. The chat model can only produce text or
//! schema-constrained JSON and the embedding model only vectors; the application layer
//! validates them and decides what (if anything) happens.

pub mod client;
pub mod download;
pub mod hardware;
mod job;
pub mod manager;
pub mod manifest;
pub mod model;
pub mod rt;
pub mod selection;
pub mod store;
pub mod supervisor;

pub use client::{ChatMessage, ChatRequest};
pub use manager::{
    AiError, AiManager, ComponentKind, Diagnostics, EmbeddingInfo, InstallEvent, InstallOutcome,
    InstallPhase, InstallPlan, ManagerConfig, OfflineAiState,
};
pub use manifest::{Backend, Manifest, PROFILE_ID};
pub use model::ChatModel;
pub use supervisor::{RuntimeState, RuntimeStatus};
