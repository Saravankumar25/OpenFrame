//! Local project intelligence for the AI agent (AI/RAG spec §8–§22, §31–§34).
//!
//! ```text
//! project.sqlite ──(post-commit observer)──► indexer ──► cache/intelligence.sqlite
//!        │                                               (openframe-search: docs, chunks,
//!        │                                                vectors, context graph)
//!        └── search_fts (canonical FTS5) ─┐                        │
//!                                         ▼                        ▼
//!                        retriever: structured / lexical / semantic / hybrid / hybrid+graph
//!                                         ▼
//!                        assembler: canonical re-validation, privacy, budgets, provenance
//!                                         ▼
//!                               retrieval::ContextPacket (contract C2)
//! ```
//!
//! Everything here is derived and disposable. OpenFrame works without it; when
//! it is missing, stale or failing, retrieval degrades to FTS + SQL tools.
//!
//! Activation is lazy: the per-project worker starts when Offline AI is
//! installed at project open, or on the first retrieval / index request — a
//! user who never uses AI pays nothing but a cheap post-commit bookkeeping step.

pub mod assembler;
pub mod documents;
pub mod indexer;
pub mod retriever;
pub mod router;

/// Adapter from the `openframe-ai` embedding runtime (contract C1:
/// `AiManager::{embedding_info, embed}`) to [`openframe_search::Embedder`]. When
/// Offline AI is not installed there is no embedder and retrieval uses FTS + SQL.
mod ai_embedder {
    use std::sync::Arc;

    use openframe_ai::AiManager;
    use openframe_search::{Embedder, SearchError};

    pub struct AiManagerEmbedder {
        manager: Arc<AiManager>,
        model_id: String,
        dim: usize,
    }

    impl AiManagerEmbedder {
        pub fn from_manager(manager: Arc<AiManager>) -> Option<Self> {
            let info = manager.embedding_info()?;
            let sha: String = info.sha256.chars().take(12).collect();
            Some(Self {
                model_id: format!("{}@{}#{}", info.model_id, info.version, sha),
                dim: info.dim as usize,
                manager,
            })
        }
    }

    impl Embedder for AiManagerEmbedder {
        fn model_id(&self) -> String {
            self.model_id.clone()
        }
        fn dim(&self) -> usize {
            self.dim
        }
        fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, SearchError> {
            self.manager
                .embed(inputs)
                .map_err(|e| SearchError::Embedding(e.to_string()))
        }
    }
}

use std::sync::Arc;
use std::time::Duration;

use openframe_search::{Embedder, QueryEmbeddingCache};
use parking_lot::{Mutex, RwLock};

pub use indexer::{IndexSettings, IndexSnapshot, ProjectIndex};

use crate::core::{AppCore, ProjectSession};

/// Long-lived service (one per `AppCore`).
pub struct IntelligenceService {
    current: Mutex<Option<Arc<ProjectIndex>>>,
    /// Explicitly attached embedder (tests; or a host wiring its own runtime).
    embedder: RwLock<Option<Arc<dyn Embedder>>>,
    settings: RwLock<IndexSettings>,
    query_cache: QueryEmbeddingCache,
}

impl IntelligenceService {
    fn new() -> Self {
        Self {
            current: Mutex::new(None),
            embedder: RwLock::new(None),
            settings: RwLock::new(IndexSettings::default()),
            query_cache: QueryEmbeddingCache::new(64),
        }
    }

    /// Attach (or detach) an embedder. The next indexing cycle picks it up; a
    /// different model id re-embeds everything.
    pub fn set_embedder(&self, embedder: Option<Arc<dyn Embedder>>) {
        *self.embedder.write() = embedder;
        self.query_cache.clear();
        if let Some(ix) = self.current() {
            ix.request_full();
        }
    }

    /// The embedder to use now: an attached one, else the Offline AI embedding runtime.
    pub fn embedder(&self, core: &AppCore) -> Option<Arc<dyn Embedder>> {
        if let Some(e) = self.embedder.read().clone() {
            return Some(e);
        }
        production_embedder(core)
    }

    pub fn settings(&self) -> IndexSettings {
        self.settings.read().clone()
    }

    pub fn set_settings(&self, s: IndexSettings) {
        *self.settings.write() = s;
    }

    pub fn current(&self) -> Option<Arc<ProjectIndex>> {
        self.current.lock().clone()
    }

    pub fn query_cache(&self) -> &QueryEmbeddingCache {
        &self.query_cache
    }
}

/// The installed Offline AI embedding model (None when Offline AI is not installed).
fn production_embedder(core: &AppCore) -> Option<Arc<dyn Embedder>> {
    let svc = crate::modules::ai::service::service(core);
    let m = svc.manager().ok()?.clone();
    ai_embedder::AiManagerEmbedder::from_manager(m).map(|e| Arc::new(e) as Arc<dyn Embedder>)
}

pub fn service(core: &AppCore) -> Arc<IntelligenceService> {
    core.service(IntelligenceService::new)
}

/// Offline AI is installed (or a local model is attached): index eagerly at open.
fn ai_installed(core: &AppCore) -> bool {
    core.existing_service::<crate::modules::ai::AiService>()
        .is_some_and(|s| s.has_attached_model() || s.manager().is_ok_and(|m| m.active().is_some()))
}

/// Called by `AppCore::set_project` whenever the open project changes.
pub fn session_changed(
    core: &AppCore,
    prev: Option<&Arc<ProjectSession>>,
    next: Option<&Arc<ProjectSession>>,
) {
    if let (Some(p), Some(n)) = (prev, next)
        && Arc::ptr_eq(p, n)
    {
        return;
    }
    let svc = service(core);
    if let Some(prev) = prev {
        prev.store.set_commit_observer(None);
        let old = svc.current.lock().take();
        if let Some(old) = old {
            old.shutdown();
        }
    }
    if let Some(next) = next {
        let ix = Arc::new(ProjectIndex::new(
            next.project_id(),
            next.store.db_path().to_path_buf(),
            next.layout.cache().join("intelligence.sqlite"),
        ));
        next.store
            .set_commit_observer(Some(Arc::new(indexer::Observer(Arc::downgrade(&ix)))));
        *svc.current.lock() = Some(ix.clone());
        if ai_installed(core) {
            ix.activate(core);
        }
    }
}

/// The open project's index, started if it wasn't yet.
pub fn activate(core: &AppCore) -> Option<Arc<ProjectIndex>> {
    let ix = service(core).current()?;
    ix.activate(core);
    Some(ix)
}

/// Wait until the open project's index has caught up (tests, rebuild task).
pub fn wait_idle(core: &AppCore, timeout: Duration) -> bool {
    match service(core).current() {
        Some(ix) => ix.wait_idle(timeout),
        None => true,
    }
}
