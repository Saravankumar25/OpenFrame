//! OpenFrame Offline AI subsystem (Local AI Runtime & Model Management spec).
//!
//! ```text
//! AiManager ─┬─ manifest   signed model/runtime manifest (Ed25519, compiled-in key)
//!            ├─ hardware   RAM / CPU / GPU (DXGI) / disk inspection — never transmitted
//!            ├─ selection  Lightweight / Recommended / High Quality recommendation
//!            ├─ download   resumable, retried, SHA-256 verified, quarantining downloads
//!            ├─ store      %LOCALAPPDATA%/OpenFrame/{models,runtimes} layout + atomic activation
//!            ├─ supervisor llama.cpp `llama-server` sidecar lifecycle (loopback only)
//!            └─ model      ChatModel adapter over the OpenAI-compatible client
//! ```
//!
//! This crate has no project-database access. The model can only produce text
//! or schema-constrained JSON; the application layer validates it and decides
//! what (if anything) happens.

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
pub use manager::{AiManager, InstallEvent, InstallOutcome, InstallPhase, ManagerConfig};
pub use manifest::{Backend, Manifest, Tier};
pub use model::ChatModel;
pub use supervisor::{RuntimeState, RuntimeStatus};
