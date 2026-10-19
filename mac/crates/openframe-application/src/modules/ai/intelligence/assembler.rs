//! Context assembly (§20, §42): turn ranked candidates into the bounded,
//! labelled, de-duplicated context packet the model may read.
//!
//! Every item is rebuilt from CANONICAL rows at read time (the derived index is
//! only used to find candidates), which enforces, independently of index state:
//! - existence / not deleted (Recently Deleted content never leaks);
//! - private-note ownership (only the owner) and project boundaries;
//! - draft boundaries (a specific-draft scope never receives another draft's scenes);
//! - revision validity (the text is always the current text).
//!
//! Budgets: `CONTEXT_CHARS` in total (explicit scope included), `ITEM_CHARS`
//! per item, graph-related entities as short one-block summaries.

use std::collections::HashSet;

use openframe_domain::{Actor, AppResult};
use serde_json::json;

use super::documents::{self, Built};
use super::retriever::{Gathered, RankedHit, query_terms};
use crate::core::{AppCore, ProjectSession};
use crate::modules::ai::knowledge;
use crate::modules::ai::retrieval::{ContextSource, RetrievalRoute, RetrieveRequest};
use crate::modules::ai::scope::ContextItem;
use crate::modules::ai::types::Provenance;

/// Total characters of project text in one packet (≈1.5k tokens for Gemma 3 1B).
pub const CONTEXT_CHARS: usize = 6_000;
pub const ITEM_CHARS: usize = 1_500;
const RELATED_CHARS: usize = 320;
const MAX_RELATED: usize = 6;
/// Vector ranks from here on are too weak to count as direct evidence next to a graph relation.
const WEAK_VECTOR_RANK: usize = 8;

#[derive(Debug, Default)]
pub struct Assembled {
    pub items: Vec<ContextItem>,
    pub provenance: Vec<Provenance>,
    pub sources: Vec<ContextSource>,
    pub degraded: bool,
}

impl Assembled {
    fn used(&self) -> usize {
        self.items.iter().map(|i| i.text.chars().count()).sum()
    }
    fn room(&self) -> usize {
        CONTEXT_CHARS.saturating_sub(self.used())
    }
    fn push(
        &mut self,
        source: String,
        text: String,
        prov: Provenance,
        src: ContextSource,
        max: usize,
    ) -> bool {
        let room = self.room().min(max);
        if room < 80 || text.trim().is_empty() {
            return false;
        }
        let text = openframe_search::chunk::truncate_chars(text.trim(), room);
        self.items.push(ContextItem { source, text });
        self.provenance.push(prov);
        self.sources.push(src);
        true
    }
}

/// Pick the chunk to show: the semantic match, else the one sharing most query terms.
fn best_chunk(b: &Built, hit: &RankedHit, terms: &[String]) -> String {
    if let Some(ord) = hit.evidence.chunk
        && let Some(c) = b.doc.chunks.get(ord as usize)
    {
        return c.text.clone();
    }
    let mut best = (0usize, 0usize);
    for (i, c) in b.doc.chunks.iter().enumerate() {
        let lower = c.text.to_lowercase();
        let score = terms.iter().filter(|t| lower.contains(t.as_str())).count();
        if score > best.0 {
            best = (score, i);
        }
    }
    b.doc
        .chunks
        .get(best.1)
        .map(|c| c.text.clone())
        .unwrap_or_default()
}

fn source_label(b: &Built) -> String {
    let info = documents::kind_info(&b.doc.entity_type).map(|i| (i.label, i.module));
    match info {
        Some(("Scene", module)) => format!("{module} · {}", b.doc.title),
        Some((label, module)) => format!("{module} · {label}: {}", b.doc.title),
        None => b.doc.title.clone(),
    }
}

pub fn assemble(
    core: &AppCore,
    session: &ProjectSession,
    actor: &Actor,
    req: &RetrieveRequest,
    route: RetrievalRoute,
    gathered: &Gathered,
    limit: usize,
) -> AppResult<Assembled> {
    let mut out = Assembled::default();
    let mut seen: HashSet<String> = HashSet::new();
    let mut seen_text: HashSet<String> = HashSet::new();

    // 1. The user's explicit scope (already resolved and budgeted by scope.rs).
    if let Some(scope) = &req.scope {
        for item in &scope.context {
            let src = ContextSource {
                entity_type: "scope".into(),
                entity_id: String::new(),
                module: scope.kind.label().into(),
                nav: serde_json::Value::Null,
                via: "scope".into(),
                relation: None,
            };
            if out.push(
                item.source.clone(),
                item.text.clone(),
                Provenance::new("Scope", item.source.clone()),
                src,
                ITEM_CHARS * 2,
            ) {
                seen_text.insert(item.text.trim().to_string());
            }
        }
        if let Some(s) = &scope.scene {
            seen.insert(format!("screenplay_scene:{}", s.id));
        }
        for id in &scope.selection {
            seen.insert(format!("*:{id}"));
        }
    }

    // 2. Product help: curated guide sections (deterministic, versioned).
    if route == RetrievalRoute::ProductHelp {
        for (title, body) in knowledge::relevant(&req.query, 3) {
            out.push(
                format!("{}: {title}", knowledge::GUIDE_LABEL),
                body.to_string(),
                Provenance::new("Product guide", knowledge::GUIDE_LABEL),
                ContextSource {
                    entity_type: "product_guide".into(),
                    entity_id: title.to_string(),
                    module: "Help".into(),
                    nav: json!({ "workspace": "help" }),
                    via: "guide".into(),
                    relation: None,
                },
                ITEM_CHARS,
            );
        }
        return Ok(out);
    }

    // 3. Retrieved project context, re-validated against canonical data.
    let terms = query_terms(&req.query);
    let scope_draft = req
        .scope
        .as_ref()
        .and_then(|s| s.draft.as_ref().map(|d| d.id.clone()));
    let project_id = session.project_id();
    let registry = core.registry.clone();
    let mut related = 0usize;
    let mut dropped = 0usize;
    // Graph-related entities get reserved room (up to a third of the items) so
    // strong direct hits cannot crowd out the cross-module context (§18).
    // "Graph-led": reached through the graph with no keyword hit and at most a weak
    // semantic one (KNN ranks almost everything in a small project).
    let is_graph_only = |h: &RankedHit| {
        h.evidence.graph_score > 0.0
            && h.evidence.lexical_rank.is_none()
            && !h.evidence.scope
            && h.evidence.vector_rank.is_none_or(|r| r >= WEAK_VECTOR_RANK)
    };
    let graph_only_count = gathered.ranked.iter().filter(|h| is_graph_only(h)).count();
    let reserved = graph_only_count.min(MAX_RELATED).min(limit / 3);
    let direct_limit = limit - reserved;
    let mut direct = 0usize;
    let ordered = gathered
        .ranked
        .iter()
        .filter(|h| !is_graph_only(h))
        .chain(gathered.ranked.iter().filter(|h| is_graph_only(h)));
    for hit in ordered {
        if out.items.len() >= limit || out.room() < 80 {
            break;
        }
        if !is_graph_only(hit) && direct >= direct_limit {
            continue;
        }
        let doc_id = hit.key.doc_id();
        if seen.contains(&doc_id) || seen.contains(&format!("*:{}", hit.key.id)) {
            continue;
        }
        let graph_only = is_graph_only(hit);
        if graph_only && related >= MAX_RELATED {
            continue;
        }
        let built = session
            .store
            .read(|c| documents::build(c, &registry, &project_id, &hit.key))?;
        let Some(b) = built else {
            dropped += 1;
            continue;
        };
        if b.doc
            .owner_user_id
            .as_deref()
            .is_some_and(|o| o != actor.user_id)
        {
            dropped += 1;
            continue;
        }
        if let (Some(d), Some(k)) = (&scope_draft, &b.doc.scope_key)
            && d != k
        {
            continue;
        }
        let text = if graph_only {
            // Related entity: its header (what it is + key facts), not its full text.
            b.doc
                .chunks
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_default()
        } else {
            best_chunk(&b, hit, &terms)
        };
        if !seen_text.insert(text.trim().to_string()) {
            continue;
        }
        let info = documents::kind_info(&b.doc.entity_type);
        let via = if hit.evidence.scope {
            "scope"
        } else if graph_only {
            "graph"
        } else if hit.evidence.vector_rank.is_some() && hit.evidence.lexical_rank.is_none() {
            "semantic"
        } else if hit.evidence.lexical_rank.is_some() {
            "lexical"
        } else {
            "graph"
        };
        let src = ContextSource {
            entity_type: b.doc.entity_type.clone(),
            entity_id: b.doc.entity_id.clone(),
            module: b.doc.module.clone(),
            nav: b.nav.clone(),
            via: via.into(),
            relation: if graph_only {
                hit.evidence.graph_relation.clone()
            } else {
                None
            },
        };
        let kind = info.map(|i| i.label).unwrap_or("Project");
        let prov = match crate::modules::ai::retrieval::nav_target(&b.nav) {
            Some(nav) => Provenance::linked(kind, b.doc.title.clone(), nav),
            None => Provenance::new(kind, b.doc.title.clone()),
        };
        let max = if graph_only {
            RELATED_CHARS
        } else {
            ITEM_CHARS
        };
        if out.push(source_label(&b), text, prov, src, max) {
            seen.insert(doc_id);
            if graph_only {
                related += 1;
            } else {
                direct += 1;
            }
        }
    }
    if dropped > 0 {
        tracing::debug!(
            dropped,
            "retrieval candidates no longer valid (deleted/private/stale index)"
        );
    }
    Ok(out)
}
