//! Operation registry: the strict IPC allow-list (Security threat model:
//! "strict IPC command allow-list"). Only operations registered here can be
//! invoked from the UI, a reviewed exchange-package import or an approved AI Change Set.
//!
//! Each module registers its queries/commands, its search indexers and its
//! recoverable-delete handlers in `modules/<name>/mod.rs::register`.

use std::collections::HashMap;
use std::sync::Arc;

use openframe_domain::{Actor, AppError, AppResult};
use rusqlite::Connection;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::core::AppCore;
use crate::store::{DeletedItemRow, Tx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Reads only. Never mutates canonical state.
    Query,
    /// Mutates through the store pipeline (transaction, undo, activity, events).
    Command,
}

pub type OpFn = Arc<dyn Fn(&AppCore, &Actor, Value) -> AppResult<Value> + Send + Sync>;

pub struct OpEntry {
    pub kind: OpKind,
    pub handler: OpFn,
}

/// A searchable projection of one canonical row.
#[derive(Debug, Clone, Default)]
pub struct SearchDoc {
    pub entity_type: String,
    pub title: String,
    pub body: String,
    /// Where the hit lives, e.g. "Screenplay", "Production", "Idea Vault".
    pub context: String,
    /// Opaque navigation target the UI understands (workspace + ids).
    pub nav: Value,
    /// Private documents (Private Notes) are only visible to this user.
    pub owner_user_id: Option<String>,
}

/// Returns the search document for a row, or None to remove it from the index
/// (deleted, empty, or not searchable).
pub type IndexerFn = fn(&Connection, &str) -> AppResult<Option<SearchDoc>>;

pub type TrashFn = fn(&Tx<'_>, &DeletedItemRow) -> AppResult<()>;

#[derive(Clone, Copy)]
pub struct TrashHandler {
    pub object_type: &'static str,
    pub table: &'static str,
    /// Human label, e.g. "Scene Card".
    pub label: &'static str,
    /// Custom restore (e.g. re-parent to Unassigned when the parent is gone).
    /// When None, restore clears `deleted_at` only.
    pub restore: Option<TrashFn>,
    /// Permanently remove the object and anything it exclusively owns.
    pub purge: TrashFn,
}

#[derive(Default)]
pub struct Registry {
    ops: HashMap<&'static str, OpEntry>,
    indexers: HashMap<&'static str, IndexerFn>,
    trash: HashMap<&'static str, TrashHandler>,
}

impl Registry {
    fn insert<A, R>(
        &mut self,
        name: &'static str,
        kind: OpKind,
        f: fn(&AppCore, &Actor, A) -> AppResult<R>,
    ) where
        A: DeserializeOwned + 'static,
        R: Serialize + 'static,
    {
        assert!(
            !self.ops.contains_key(name),
            "operation {name} registered twice"
        );
        assert!(
            name.contains('.'),
            "operation names are namespaced: module.action ({name})"
        );
        let handler: OpFn = Arc::new(move |core, actor, args| {
            let args: A = serde_json::from_value(args).map_err(|e| {
                AppError::invalid_input("The request was not in the expected format.")
                    .with_detail(format!("{name}: {e}"))
            })?;
            let out = f(core, actor, args)?;
            serde_json::to_value(out).map_err(|e| AppError::internal(e.to_string()))
        });
        self.ops.insert(name, OpEntry { kind, handler });
    }

    pub fn query<A, R>(&mut self, name: &'static str, f: fn(&AppCore, &Actor, A) -> AppResult<R>)
    where
        A: DeserializeOwned + 'static,
        R: Serialize + 'static,
    {
        self.insert(name, OpKind::Query, f)
    }

    pub fn command<A, R>(&mut self, name: &'static str, f: fn(&AppCore, &Actor, A) -> AppResult<R>)
    where
        A: DeserializeOwned + 'static,
        R: Serialize + 'static,
    {
        self.insert(name, OpKind::Command, f)
    }

    pub fn indexer(&mut self, table: &'static str, f: IndexerFn) {
        assert!(
            !self.indexers.contains_key(table),
            "indexer for {table} registered twice"
        );
        self.indexers.insert(table, f);
    }

    pub fn trash_handler(&mut self, h: TrashHandler) {
        assert!(
            !self.trash.contains_key(h.object_type),
            "trash handler {} registered twice",
            h.object_type
        );
        self.trash.insert(h.object_type, h);
    }

    pub fn get(&self, name: &str) -> Option<&OpEntry> {
        self.ops.get(name)
    }

    pub fn indexer_for(&self, table: &str) -> Option<IndexerFn> {
        self.indexers.get(table).copied()
    }

    pub fn indexed_tables(&self) -> Vec<&'static str> {
        let mut t: Vec<_> = self.indexers.keys().copied().collect();
        t.sort();
        t
    }

    pub fn trash_for(&self, object_type: &str) -> Option<TrashHandler> {
        self.trash.get(object_type).copied()
    }

    pub fn op_names(&self) -> Vec<&'static str> {
        let mut n: Vec<_> = self.ops.keys().copied().collect();
        n.sort();
        n
    }

    pub fn dispatch(
        &self,
        core: &AppCore,
        actor: &Actor,
        name: &str,
        args: Value,
    ) -> AppResult<Value> {
        let entry = self.get(name).ok_or_else(|| {
            AppError::security("unknown_operation", "That action isn't available.")
                .with_detail(format!("unknown op {name}"))
        })?;
        check_arg_shape(&args, 0)?;
        (entry.handler)(core, actor, args)
    }
}

/// Upper bounds for any operation's arguments, enforced before deserialization: no op
/// needs more than this many list items or this much nesting. Individual ops keep their
/// own, tighter limits (Security review 2026-09-30, IPC-01).
pub const MAX_ARG_ARRAY_LEN: usize = 50_000;
pub const MAX_ARG_DEPTH: usize = 32;

fn check_arg_shape(v: &Value, depth: usize) -> AppResult<()> {
    let too_big = || {
        AppError::invalid_input("That request is too large.").with_detail("argument shape limit")
    };
    if depth > MAX_ARG_DEPTH {
        return Err(too_big());
    }
    match v {
        Value::Array(items) => {
            if items.len() > MAX_ARG_ARRAY_LEN {
                return Err(too_big());
            }
            items.iter().try_for_each(|i| check_arg_shape(i, depth + 1))
        }
        Value::Object(map) => {
            if map.len() > 1_000 {
                return Err(too_big());
            }
            map.values().try_for_each(|i| check_arg_shape(i, depth + 1))
        }
        _ => Ok(()),
    }
}
