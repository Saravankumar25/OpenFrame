//! Retrieval API for the AI agent — contract C2 (AI/RAG spec §10, §18–§21, §33, §42).
//!
//! `retrieve` turns a question (+ the user's explicit scope) into a bounded,
//! permission-filtered [`ContextPacket`] with provenance:
//!
//! ```text
//! route_for(query) ─► Structured  : explicit scope only (facts come from SQL tools)
//!                  ├► ProductHelp : product guide sections
//!                  ├► Lexical     : FTS5/BM25 over the canonical search index
//!                  ├► Semantic    : vector KNN (falls back to Lexical)
//!                  ├► Hybrid      : RRF(FTS, vector) + boosts
//!                  └► HybridGraph : Hybrid + 1–2 hop context-graph expansion
//! ```
//!
//! Fallbacks (§33): hybrid → FTS + SQL → SQL + explicit scope; `degraded` is set
//! whenever a path the route wanted was unavailable. Every item's text is
//! rebuilt from canonical rows at read time, so deleted, non-current-draft,
//! other-users'-private or out-of-scope content can never be returned, even if
//! the derived index is stale or tampered with. Retrieved text is DATA: the
//! orchestrator places it inside the untrusted-data boundary.

use openframe_domain::{Actor, AppError, AppResult, Capability};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::intelligence::{self, assembler, retriever, router};
use super::scope::{ContextItem, ResolvedScope};
use super::types::{NavTarget, Provenance};
use crate::core::AppCore;
use crate::registry::Registry;

/// Upper bound on returned items (contract: `limit ≤ 24`).
pub const MAX_ITEMS: usize = 24;
pub const DEFAULT_ITEMS: usize = 8;
/// Longest accepted question.
pub const MAX_QUERY_CHARS: usize = 2_000;
/// §36 wording while the index is not usable.
pub const DEGRADED_NOTICE: &str =
    "AI can still answer some questions while project context is being prepared.";
pub const PREPARING_NOTICE: &str = "Preparing project context…";

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.module("Offline AI");
    r.query("ai.index_status", index_status_op).meta(
        M::read("State of the derived project-context index (current, updating, rebuilding…).")
            .hidden(h::MAINTENANCE),
    );
    r.command("ai.index_rebuild", index_rebuild_op).meta(
        M::command(
            Capability::UseAi,
            "Rebuild the derived project-context index in the background.",
        )
        .long_running()
        .hidden(h::MAINTENANCE),
    );
}

#[derive(Debug, Clone)]
pub struct RetrieveRequest {
    pub query: String,
    pub scope: Option<ResolvedScope>,
    /// Maximum items (clamped to 1..=24; 0 = default 8).
    pub limit: usize,
    /// Force a route; `None` = deterministic router.
    pub route: Option<RetrievalRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiRetrievalRoute")]
pub enum RetrievalRoute {
    Structured,
    Lexical,
    Semantic,
    Hybrid,
    HybridGraph,
    ProductHelp,
}

impl RetrievalRoute {
    pub fn wants_vectors(self) -> bool {
        matches!(
            self,
            RetrievalRoute::Semantic | RetrievalRoute::Hybrid | RetrievalRoute::HybridGraph
        )
    }
    pub fn wants_lexical(self) -> bool {
        matches!(
            self,
            RetrievalRoute::Lexical
                | RetrievalRoute::Semantic
                | RetrievalRoute::Hybrid
                | RetrievalRoute::HybridGraph
        )
    }
}

/// Semantic-index lifecycle state (§15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiIndexState")]
pub enum IndexState {
    Current,
    Updating,
    Stale,
    Rebuilding,
    Failed,
    Unavailable,
}

/// Where one context item came from (index-aligned with `items`/`provenance`).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiContextSource")]
#[serde(rename_all = "camelCase")]
pub struct ContextSource {
    /// Canonical entity type (table), e.g. "screenplay_scene"; "product_guide" for help.
    pub entity_type: String,
    pub entity_id: String,
    pub module: String,
    /// Navigation target (same shape as Global Search results).
    #[ts(type = "unknown")]
    pub nav: Value,
    /// "scope" | "lexical" | "semantic" | "graph" | "guide"
    pub via: String,
    /// For graph-expanded items: the relation that led here (e.g. "appears_in").
    pub relation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContextPacket {
    /// Bounded project text, most useful first. Untrusted data.
    pub items: Vec<ContextItem>,
    /// `provenance[i]` labels `items[i]` ("Scene", "Scene 12 — INT. …").
    pub provenance: Vec<Provenance>,
    pub index_state: IndexState,
    pub route: RetrievalRoute,
    /// A retrieval path the route wanted was unavailable (fallback used).
    pub degraded: bool,
    /// `sources[i]` identifies `items[i]` (entity ref + navigation).
    pub sources: Vec<ContextSource>,
    /// User-facing notice when degraded (§36 wording), else None.
    pub notice: Option<String>,
}

/// Convert an index navigation target (`{"workspace", "sub"?, "<x>Id": …}`) into
/// the AI result's [`NavTarget`] so provenance chips can open the source.
pub fn nav_target(v: &Value) -> Option<NavTarget> {
    let obj = v.as_object()?;
    let workspace = obj.get("workspace")?.as_str()?;
    let mut nav = NavTarget::to(workspace);
    for (k, val) in obj {
        match (k.as_str(), val.as_str()) {
            ("workspace", _) => {}
            ("sub", Some(sub)) => nav = nav.with_sub(sub),
            (key, Some(x)) => nav = nav.with(key, x),
            _ => {}
        }
    }
    Some(nav)
}

pub fn route_for(query: &str) -> RetrievalRoute {
    router::route_for(query)
}

/// Current index state of the open project (does not start indexing).
pub fn index_status(core: &AppCore) -> IndexState {
    if core.project_opt().is_none() {
        return IndexState::Unavailable;
    }
    match intelligence::service(core).current() {
        Some(ix) => ix.snapshot().state,
        None => IndexState::Unavailable,
    }
}

pub fn retrieve(core: &AppCore, actor: &Actor, req: &RetrieveRequest) -> AppResult<ContextPacket> {
    actor.require(Capability::UseAi, "use the assistant")?;
    actor.require(Capability::View, "view this project")?;
    let session = core.project()?;
    if req.query.chars().count() > MAX_QUERY_CHARS {
        return Err(AppError::invalid_input("That question is too long."));
    }
    let limit = match req.limit {
        0 => DEFAULT_ITEMS,
        n => n.min(MAX_ITEMS),
    };
    let route = req.route.unwrap_or_else(|| route_for(&req.query));
    let ix = intelligence::activate(core);
    let snapshot = ix.as_ref().map(|i| i.snapshot());
    let index_state = snapshot
        .as_ref()
        .map(|s| s.state)
        .unwrap_or(IndexState::Unavailable);
    let gathered = retriever::gather(
        core,
        &session,
        actor,
        ix.as_deref(),
        snapshot.as_ref(),
        req,
        route,
        limit,
    )?;
    let assembled = assembler::assemble(core, &session, actor, req, route, &gathered, limit)?;
    let degraded = gathered.degraded || assembled.degraded;
    Ok(ContextPacket {
        items: assembled.items,
        provenance: assembled.provenance,
        index_state,
        route,
        degraded,
        sources: assembled.sources,
        notice: degraded.then(|| DEGRADED_NOTICE.to_string()),
    })
}

// ------------------------------------------------------------------ ops

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiIndexArgs {}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiIndexStatus")]
#[serde(rename_all = "camelCase")]
pub struct IndexStatusDto {
    pub state: IndexState,
    /// "Preparing project context…" / degraded wording, or None when ready.
    pub message: Option<String>,
    /// Show the "Preparing project context…" indicator.
    pub preparing: bool,
    /// Meaning-based search is available (embeddings current).
    pub semantic_ready: bool,
    #[ts(type = "number | null")]
    pub progress: Option<f64>,
    #[ts(type = "number")]
    pub documents: u64,
    #[ts(type = "number")]
    pub chunks: u64,
    #[ts(type = "number")]
    pub embedded_chunks: u64,
    #[ts(type = "number")]
    pub graph_nodes: u64,
    #[ts(type = "number")]
    pub graph_edges: u64,
    #[ts(type = "number")]
    pub pending_changes: u64,
    #[ts(type = "number")]
    pub pending_embeddings: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiIndexRebuildStarted")]
#[serde(rename_all = "camelCase")]
pub struct IndexRebuildStarted {
    pub task_id: String,
}

fn status_dto(snap: Option<intelligence::IndexSnapshot>) -> IndexStatusDto {
    let Some(s) = snap else {
        return IndexStatusDto {
            state: IndexState::Unavailable,
            message: Some(DEGRADED_NOTICE.into()),
            preparing: false,
            semantic_ready: false,
            progress: None,
            documents: 0,
            chunks: 0,
            embedded_chunks: 0,
            graph_nodes: 0,
            graph_edges: 0,
            pending_changes: 0,
            pending_embeddings: 0,
        };
    };
    let preparing = matches!(s.state, IndexState::Updating | IndexState::Rebuilding);
    let message = match s.state {
        IndexState::Current => None,
        IndexState::Updating | IndexState::Rebuilding => Some(PREPARING_NOTICE.into()),
        IndexState::Stale | IndexState::Failed | IndexState::Unavailable => {
            Some(DEGRADED_NOTICE.into())
        }
    };
    IndexStatusDto {
        state: s.state,
        message,
        preparing,
        semantic_ready: s.semantic_ready,
        progress: s.progress,
        documents: s.counts.documents,
        chunks: s.counts.chunks,
        embedded_chunks: s.counts.embedded,
        graph_nodes: s.counts.nodes,
        graph_edges: s.counts.edges,
        pending_changes: s.pending_changes,
        pending_embeddings: s.pending_embeddings,
    }
}

/// `ai.index_status`: the project-context state for the AI panel. Opening the
/// AI panel starts indexing if it hasn't started yet.
fn index_status_op(core: &AppCore, actor: &Actor, _: AiIndexArgs) -> AppResult<IndexStatusDto> {
    actor.require(Capability::View, "view this project")?;
    if core.project_opt().is_none() {
        return Ok(status_dto(None));
    }
    let snap = if actor.can(Capability::UseAi) {
        intelligence::activate(core).map(|ix| ix.snapshot())
    } else {
        intelligence::service(core)
            .current()
            .map(|ix| ix.snapshot())
    };
    Ok(status_dto(snap))
}

/// `ai.index_rebuild`: discard the derived index and rebuild it in the
/// background (never touches canonical data). Returns the task id.
fn index_rebuild_op(
    core: &AppCore,
    actor: &Actor,
    _: AiIndexArgs,
) -> AppResult<IndexRebuildStarted> {
    actor.require(Capability::UseAi, "use the assistant")?;
    actor.require(Capability::View, "view this project")?;
    core.project()?;
    let ix = intelligence::activate(core).ok_or_else(AppError::no_project_open)?;
    ix.request_rebuild();
    let task_id = core.spawn_task("ai.index_rebuild", PREPARING_NOTICE, move |_core, h| {
        loop {
            h.check()?;
            if ix.wait_idle(std::time::Duration::from_millis(400)) {
                break;
            }
            let s = ix.snapshot();
            h.progress(s.progress.unwrap_or(0.0), PREPARING_NOTICE);
            if s.state == IndexState::Unavailable {
                return Err(AppError::ai(
                    "index_unavailable",
                    "Project context couldn't be prepared. AI can still answer some questions.",
                ));
            }
        }
        let s = ix.snapshot();
        Ok(json!({
            "state": s.state,
            "documents": s.counts.documents,
            "chunks": s.counts.chunks,
            "embeddedChunks": s.counts.embedded,
        }))
    });
    Ok(IndexRebuildStarted { task_id })
}
