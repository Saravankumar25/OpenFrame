//! OpenFrame domain vocabulary.
//!
//! This crate is intentionally free of I/O, SQLite, Tauri and AI-runtime types
//! (ESD §5). It defines stable identities, the error taxonomy (ESD §12),
//! the security role/capability model (Security spec §6) and canonical
//! enumerations shared by every other crate.

pub mod auth;
pub mod enums;
pub mod error;
pub mod ids;

pub use auth::{Actor, Capability, Role};
pub use error::{AppError, AppResult, ErrorCode};
pub use ids::{Id, new_id, now_ms};

/// Current project format version written into `openframe.json`.
pub const PROJECT_FORMAT_VERSION: u32 = 1;
/// Package (project/backup/exchange) container format version.
pub const PACKAGE_FORMAT_VERSION: u32 = 1;
/// AI tool/Change Set contract version (bumped when tool schemas change incompatibly).
pub const AI_COMPATIBILITY_VERSION: u32 = 1;
