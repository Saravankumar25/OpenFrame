//! OpenFrame application layer (ESD §5–§8, Rust Domain & Command Architecture).
//!
//! ```text
//! Tauri adapter ──► Registry::dispatch(op, args)
//!                     └─► module handler (validates input)
//!                           └─► Store::mutate / Store::read   ← the only DB access path
//!                                 └─► persistence (SQLite), undo capture, search index
//! ```
//!
//! Module layout: `modules/<name>` registers its operations, search indexers and
//! recoverable-delete handlers. Modules never call each other's SQL directly;
//! cross-module reads go through small `pub(crate)` query functions.

// Row tuples from SQL queries and wide insert helpers are idiomatic here.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

pub mod core;
pub mod events;
pub mod modules;
pub mod registry;
pub mod schema;
pub mod store;
pub mod util;

pub use crate::core::{AppConfig, AppCore, ProjectSession, TaskHandle};
pub use crate::events::{AppEvent, EventSink, StoreKind};
pub use crate::registry::{OpKind, Registry, SearchDoc, TrashHandler};
pub use crate::store::{DeleteSpec, MutationMeta, Store, Tx};
