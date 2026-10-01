//! Operation registry: the strict IPC allow-list (Security threat model:
//! "strict IPC command allow-list"). Only operations registered here can be
//! invoked from the UI, a reviewed exchange-package import or an approved AI Change Set.
//!
//! Each module registers its queries/commands, its search indexers and its
//! recoverable-delete handlers in `modules/<name>/mod.rs::register`.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use openframe_domain::{Actor, AppError, AppResult, Capability};
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
    /// Explicit, hand-written safety metadata (spec §6, §39). Every registered
    /// operation must carry it; `tests/ai_tool_coverage.rs` fails otherwise.
    pub meta: Option<OperationMetadata>,
    /// Rust type names of the argument and result DTOs (operation inventory;
    /// shorten with `short_type_name`).
    pub arg_type: &'static str,
    pub result_type: &'static str,
}

// ------------------------------------------------------------ operation metadata

/// What an operation means to the AI tool layer (spec §6.2). Mirrors the AI
/// operation classes; `Search` is kept distinct in the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OpClass {
    Read,
    Search,
    Compute,
    Navigate,
    Suggest,
    Mutate,
}

impl OpClass {
    pub fn as_str(self) -> &'static str {
        match self {
            OpClass::Read => "Read",
            OpClass::Search => "Search",
            OpClass::Compute => "Compute",
            OpClass::Navigate => "Navigate",
            OpClass::Suggest => "Suggest",
            OpClass::Mutate => "Mutate",
        }
    }
}

/// Whether the assistant may reach an operation, decided explicitly per
/// operation — never inferred from its name (spec §39).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiExposure {
    /// Queries: answered by a bounded, permission-filtered read tool.
    /// Commands: only ever placed inside a Change Set by a proposal tool and applied
    /// after explicit human approval (spec §3, §40).
    Tool,
    /// Never offered to the model. The reason is published in the coverage matrix.
    Hidden(&'static str),
}

impl AiExposure {
    pub fn is_hidden(self) -> bool {
        matches!(self, AiExposure::Hidden(_))
    }
}

/// Effect on files outside the project database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FsEffect {
    None,
    /// Reads a file or folder the user picked (imports, adding files/photos).
    ReadsUserFile,
    /// Writes to a location the user picked (exports, packages, copies).
    WritesUserFile,
    /// Creates, opens, moves or deletes project folders / managed files on disk.
    ProjectStorage,
}

impl FsEffect {
    pub fn as_str(self) -> &'static str {
        match self {
            FsEffect::None => "none",
            FsEffect::ReadsUserFile => "reads user file",
            FsEffect::WritesUserFile => "writes user file",
            FsEffect::ProjectStorage => "project storage",
        }
    }
}

/// Explicit safety metadata for one registered operation (spec §6.1, §39).
#[derive(Debug, Clone, Copy)]
pub struct OperationMetadata {
    /// Product area, e.g. "Story" (set once per module `register` fn).
    pub module: &'static str,
    /// Plain-language meaning of the operation for a filmmaker.
    pub description: &'static str,
    pub ai_exposure: AiExposure,
    pub operation_class: OpClass,
    /// The capability the handler checks; `None` for application-level operations
    /// that need no open project (settings, model manager, recent projects).
    pub required_capability: Option<Capability>,
    /// Removes or overwrites user content (even if recoverable).
    pub destructive: bool,
    /// Cannot be undone from OpenFrame (no undo step, no Recently Deleted entry).
    pub irreversible: bool,
    /// The product UI asks for an extra destructive confirmation beyond the action
    /// itself; that confirmation still applies after Change Set approval (spec §7).
    pub confirmation: bool,
    pub filesystem_effect: FsEffect,
    /// Runs as a background task (`AppCore::spawn_task`) and returns a task id.
    pub long_running: bool,
}

/// Why an operation is not offered to the assistant (coverage matrix, spec §49.4).
pub mod hidden {
    pub const HUMAN_APPROVAL: &str = "Human-only approval boundary: accepting, rejecting or re-checking a Change Set is a user action in the review card (spec §7, §40).";
    pub const AI_SELF: &str = "The assistant's own entry point and conversation history; a tool calling it would recurse.";
    pub const AI_RUNTIME: &str = "Offline AI installation and runtime management is the user's decision in Settings, not something the assistant does to itself.";
    pub const APP_LIFECYCLE: &str = "Application-level project lifecycle (create, open, close, locate, duplicate, archive or delete projects on disk, recent-project list); the assistant works inside the one open project.";
    pub const USER_PATH: &str = "Reads or writes a file-system location the user picks in a file dialog; the assistant never chooses or touches file-system paths (spec §3).";
    pub const DELETE_FOREVER: &str = "Permanent deletion ('Delete forever') is never offered to the assistant (spec §28); items stay in Recently Deleted.";
    pub const VIEW_STATE: &str = "Per-user view state (collapsed rows, remembered positions, current episode); not project content.";
    pub const SESSION_CONTROL: &str = "Immediate session control (undo/redo, save, cancel a running task, display name) stays a direct user action; an applied Change Set is itself one undo step.";
    pub const MAINTENANCE: &str = "Internal maintenance with no user-visible project change.";
    pub const PACKAGE_REVIEW: &str = "Exchange and review packages bring other people's changes in from package files; opening, reviewing and applying them stays a human workflow.";
    pub const CROSS_PROJECT: &str = "Crosses the project boundary (Global Idea Vault or another project); the assistant is scoped to the open project.";
    pub const UI_FLOW: &str = "Data for an interactive dialog or editor preview; the assistant reads the same facts through its own bounded tools.";
    pub const EDITOR_PRIMITIVE: &str = "Keystroke-level screenplay editor primitive (batched typing, element moves/splits); the assistant proposes scene-level changes instead.";
    pub const LOCK_OVERRIDE: &str =
        "Owner-only override of a lock or finalization; it stays a deliberate human action.";
    pub const ASSET_PATH: &str = "Returns local file paths of managed media; the assistant never receives file-system paths.";
    pub const PRIVATE_BOUNDARY: &str = "Would move content across the private/shared boundary; the assistant only reads private notes of their owner.";
    pub const LAYOUT: &str = "Manual visual arrangement (drag position, size, stacking order) with no content meaning; the user arranges directly.";
    pub const MEDIA_INPUT: &str = "Adds media the user supplies (picked image files or pasted image data); the assistant cannot supply media.";
    pub const DUPLICATE: &str = "Alternative UI entry point for a change the assistant already prepares through an equivalent operation.";
}

impl OperationMetadata {
    const fn base(
        class: OpClass,
        cap: Option<Capability>,
        description: &'static str,
    ) -> OperationMetadata {
        OperationMetadata {
            module: "",
            description,
            ai_exposure: AiExposure::Tool,
            operation_class: class,
            required_capability: cap,
            destructive: false,
            irreversible: false,
            confirmation: false,
            filesystem_effect: FsEffect::None,
            long_running: false,
        }
    }
    /// A read of permitted project content (capability View).
    pub const fn read(description: &'static str) -> Self {
        Self::base(OpClass::Read, Some(Capability::View), description)
    }
    /// A search over permitted project content (capability View).
    pub const fn search(description: &'static str) -> Self {
        Self::base(OpClass::Search, Some(Capability::View), description)
    }
    /// A deterministic computation over permitted project content (capability View).
    pub const fn compute(description: &'static str) -> Self {
        Self::base(OpClass::Compute, Some(Capability::View), description)
    }
    /// A query needing a capability other than View.
    pub const fn query(cap: Capability, description: &'static str) -> Self {
        Self::base(OpClass::Read, Some(cap), description)
    }
    /// An ordinary content edit (capability Edit).
    pub const fn edit(description: &'static str) -> Self {
        Self::base(OpClass::Mutate, Some(Capability::Edit), description)
    }
    /// A recoverable delete to Recently Deleted (capability SoftDelete).
    pub const fn soft_delete(description: &'static str) -> Self {
        let mut m = Self::base(OpClass::Mutate, Some(Capability::SoftDelete), description);
        m.destructive = true;
        m
    }
    /// A mutation needing a specific capability.
    pub const fn command(cap: Capability, description: &'static str) -> Self {
        Self::base(OpClass::Mutate, Some(cap), description)
    }
    /// Application-level query (no open project, no project capability).
    pub const fn app_query(description: &'static str) -> Self {
        Self::base(OpClass::Read, None, description)
    }
    /// Application-level command (no open project, no project capability).
    pub const fn app_command(description: &'static str) -> Self {
        Self::base(OpClass::Mutate, None, description)
    }
    pub const fn hidden(mut self, reason: &'static str) -> Self {
        self.ai_exposure = AiExposure::Hidden(reason);
        self
    }
    pub const fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
    pub const fn irreversible(mut self) -> Self {
        self.irreversible = true;
        self
    }
    pub const fn confirm(mut self) -> Self {
        self.confirmation = true;
        self
    }
    pub const fn fs(mut self, effect: FsEffect) -> Self {
        self.filesystem_effect = effect;
        self
    }
    pub const fn long_running(mut self) -> Self {
        self.long_running = true;
        self
    }
    pub const fn class(mut self, class: OpClass) -> Self {
        self.operation_class = class;
        self
    }
}

/// Returned by `Registry::query`/`command` so the registration can be annotated:
/// `r.command("story.create_card", create_card).meta(OperationMetadata::edit("…"));`
pub struct Registration<'r> {
    entry: &'r mut OpEntry,
    module: &'static str,
}

impl Registration<'_> {
    pub fn meta(self, m: OperationMetadata) {
        self.entry.meta = Some(OperationMetadata {
            module: self.module,
            ..m
        });
    }
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
    /// Product area recorded on the metadata of operations registered next.
    module: &'static str,
}

/// A registry with every module registered, for metadata lookups that have no
/// `AppCore` at hand (Change Set allow-list, coverage inventory). Handlers are
/// never invoked through it.
pub fn catalog() -> &'static Registry {
    static CATALOG: OnceLock<Registry> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut r = Registry::default();
        crate::modules::register_all(&mut r);
        r
    })
}

impl Registry {
    /// Set the product area for the operations registered after this call.
    pub fn module(&mut self, name: &'static str) {
        self.module = name;
    }

    fn insert<A, R>(
        &mut self,
        name: &'static str,
        kind: OpKind,
        f: fn(&AppCore, &Actor, A) -> AppResult<R>,
    ) -> Registration<'_>
    where
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
        let module = self.module;
        let entry = self.ops.entry(name).or_insert(OpEntry {
            kind,
            handler,
            meta: None,
            arg_type: std::any::type_name::<A>(),
            result_type: std::any::type_name::<R>(),
        });
        Registration { entry, module }
    }

    pub fn query<A, R>(
        &mut self,
        name: &'static str,
        f: fn(&AppCore, &Actor, A) -> AppResult<R>,
    ) -> Registration<'_>
    where
        A: DeserializeOwned + 'static,
        R: Serialize + 'static,
    {
        self.insert(name, OpKind::Query, f)
    }

    pub fn command<A, R>(
        &mut self,
        name: &'static str,
        f: fn(&AppCore, &Actor, A) -> AppResult<R>,
    ) -> Registration<'_>
    where
        A: DeserializeOwned + 'static,
        R: Serialize + 'static,
    {
        self.insert(name, OpKind::Command, f)
    }

    /// Safety metadata of a registered operation.
    pub fn metadata(&self, name: &str) -> Option<&OperationMetadata> {
        self.ops.get(name).and_then(|e| e.meta.as_ref())
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

/// `alloc::vec::Vec<crate::modules::x::Dto>` → `Vec<Dto>` (inventory display only).
pub fn short_type_name(full: &str) -> String {
    let mut out = String::with_capacity(full.len());
    let mut segment = String::new();
    for ch in full.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == ':' {
            segment.push(ch);
        } else {
            out.push_str(segment.rsplit("::").next().unwrap_or(""));
            segment.clear();
            out.push(ch);
        }
    }
    out.push_str(segment.rsplit("::").next().unwrap_or(""));
    out
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
