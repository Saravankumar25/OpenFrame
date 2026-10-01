//! Candidate gathering and hybrid ranking (§10, §18, §19).
//!
//! Lexical hits come from the canonical FTS5 index (`search_doc`/`search_fts`,
//! BM25) with an OR query over the question's significant terms; semantic hits
//! from vector KNN over chunk embeddings; graph hits from bounded expansion of
//! the top entities. Everything is merged by Reciprocal Rank Fusion plus
//! explainable boosts (explicit scope, exact name, graph relation, current
//! draft). Privacy is filtered here AND re-checked canonically by the assembler.

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use openframe_domain::{Actor, AppResult};
use openframe_search::graph::{self, TraversalLimits};
use openframe_search::rank::{self, Candidate, RankWeights};
use openframe_search::{SemanticIndex, SqliteVecIndex, docs};
use rusqlite::{Connection, OptionalExtension, params};

use super::documents::DocKey;
use super::indexer::{IndexSnapshot, ProjectIndex};
use crate::core::{AppCore, ProjectSession};
use crate::modules::ai::retrieval::{RetrievalRoute, RetrieveRequest};

const LEXICAL_K: usize = 40;
const VECTOR_K: usize = 48;
const GRAPH_SEEDS: usize = 5;

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "of", "to", "in", "on", "at", "for", "with", "by",
    "from", "about", "as", "is", "are", "was", "were", "be", "been", "being", "do", "does", "did",
    "doing", "have", "has", "had", "it", "its", "this", "that", "these", "those", "there", "here",
    "what", "which", "who", "whom", "whose", "where", "when", "why", "how", "i", "me", "my", "we",
    "our", "you", "your", "he", "him", "his", "she", "her", "they", "them", "their", "can",
    "could", "should", "would", "will", "shall", "may", "might", "must", "find", "show", "tell",
    "give", "list", "all", "any", "some", "every", "each", "into", "onto", "than", "then", "so",
    "if", "not", "no", "yes", "please", "project", "scenes", "scene", "moments", "moment", "begin",
    "begins", "start", "starts", "where's", "what's", "does", "just", "also", "very", "more",
    "most", "such", "like",
];

/// Significant query terms (lowercase, de-duplicated, ≤ 12).
pub fn query_terms(q: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    q.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2 && !STOPWORDS.contains(w))
        .filter(|w| seen.insert(w.to_string()))
        .take(12)
        .map(str::to_string)
        .collect()
}

/// Safe FTS5 OR-query (every term quoted, prefix-matched): ranks by how many
/// terms match (BM25), unlike Global Search's AND query which is right for
/// typed keywords but returns nothing for natural-language questions.
fn fts_or_query(terms: &[String]) -> Option<String> {
    if terms.is_empty() {
        return None;
    }
    Some(
        terms
            .iter()
            .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

pub fn norm(s: &str) -> String {
    s.to_uppercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Does the question name this entity exactly? ("Ravi", "Railway Station", "Scene 12")
pub fn names_exactly(query_norm: &str, title: &str) -> bool {
    let padded = format!(" {query_norm} ");
    let has = |phrase: &str| {
        let p = norm(phrase);
        p.chars().count() >= 3 && padded.contains(&format!(" {p} "))
    };
    // "Scene 12 — INT. STATION - NIGHT": both the number and the heading count.
    let (head, rest) = match title.split_once(" — ") {
        Some((h, r)) => (h, Some(r)),
        None => (title, None),
    };
    has(head) || rest.is_some_and(has)
}

/// Why a candidate was found.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    pub lexical_rank: Option<usize>,
    pub vector_rank: Option<usize>,
    /// Chunk ordinal that matched semantically.
    pub chunk: Option<i64>,
    pub graph_score: f32,
    pub graph_relation: Option<String>,
    pub scope: bool,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct RankedHit {
    pub key: DocKey,
    pub score: f32,
    pub reasons: Vec<&'static str>,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, Default)]
pub struct Gathered {
    pub ranked: Vec<RankedHit>,
    pub degraded: bool,
    /// Timings in microseconds per stage (diagnostics / measurement tests).
    pub timings: BTreeMap<&'static str, u128>,
}

/// Keys of the explicit scope (open scene, selected objects).
fn scope_keys(c: &Connection, actor: &Actor, req: &RetrieveRequest) -> AppResult<Vec<DocKey>> {
    let Some(scope) = &req.scope else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    if let Some(s) = &scope.scene {
        out.push(DocKey::new("screenplay_scene", s.id.clone()));
    }
    for id in &scope.selection {
        let table: Option<String> = c
            .query_row(
                "SELECT source_table FROM search_doc WHERE entity_id=?1 AND (owner_user_id IS NULL OR owner_user_id=?2) LIMIT 1",
                params![id, actor.user_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(k) = table.and_then(|t| DocKey::from_table(&t, id)) {
            out.push(k);
        }
    }
    Ok(out)
}

fn lexical(c: &Connection, actor: &Actor, terms: &[String]) -> AppResult<Vec<(DocKey, String)>> {
    let Some(q) = fts_or_query(terms) else {
        return Ok(Vec::new());
    };
    let mut stmt = c.prepare_cached(
        "SELECT d.source_table, d.entity_id, d.title FROM search_fts JOIN search_doc d ON d.rowid = search_fts.rowid
         WHERE search_fts MATCH ?1 AND (d.owner_user_id IS NULL OR d.owner_user_id = ?2)
         ORDER BY bm25(search_fts, 4.0, 1.0) LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![q, actor.user_id, LEXICAL_K as i64 * 2], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(t, id, title)| DocKey::from_table(&t, &id).map(|k| (k, title)))
        .take(LEXICAL_K)
        .collect())
}

/// Relation priorities for graph expansion, from what the question is about.
fn relation_weights(q: &str) -> Vec<(String, f32)> {
    let q = q.to_lowercase();
    let mut w: Vec<(&str, f32)> = vec![
        ("appears_in", 0.8),
        ("represented_by", 0.7),
        ("located_at", 0.7),
        ("belongs_to", 0.5),
        ("related_to", 0.6),
        ("requires", 0.6),
        ("scheduled_on", 0.6),
        ("plays", 0.6),
    ];
    let mut bump = |rels: &[&str], words: &[&str]| {
        if words.iter().any(|x| q.contains(x)) {
            for (r, v) in w.iter_mut() {
                if rels.contains(r) {
                    *v = 1.0;
                }
            }
        }
    };
    bump(
        &["scheduled_on", "for_day"],
        &["schedule", "shoot", "day", "tomorrow", "call sheet"],
    );
    bump(
        &["requires", "associated_with"],
        &["prop", "costume", "vehicle", "need", "breakdown", "require"],
    );
    bump(&["located_at"], &["location", "where", "place", "set"]);
    bump(
        &["appears_in", "related_to", "plays"],
        &["character", "relationship", "who", "cast", "actor"],
    );
    bump(
        &["represented_by", "belongs_to", "derived_from"],
        &["story", "card", "act", "sequence", "beat"],
    );
    w.into_iter().map(|(r, v)| (r.to_string(), v)).collect()
}

fn cross_module(q: &str) -> bool {
    let q = q.to_lowercase();
    let groups: [&[&str]; 4] = [
        &["scene", "screenplay", "draft"],
        &["story", "card", "character", "act", "sequence"],
        &["prop", "costume", "location", "cast", "crew", "breakdown"],
        &["schedule", "shoot", "day", "call sheet"],
    ];
    groups
        .iter()
        .filter(|g| g.iter().any(|x| q.contains(x)))
        .count()
        >= 2
}

#[allow(clippy::too_many_arguments)]
pub fn gather(
    core: &AppCore,
    session: &ProjectSession,
    actor: &Actor,
    ix: Option<&ProjectIndex>,
    snapshot: Option<&IndexSnapshot>,
    req: &RetrieveRequest,
    route: RetrievalRoute,
    limit: usize,
) -> AppResult<Gathered> {
    let mut out = Gathered::default();
    let mut ev: BTreeMap<DocKey, Evidence> = BTreeMap::new();
    let terms = query_terms(&req.query);
    let query_norm = norm(&req.query);
    let scope_draft = req
        .scope
        .as_ref()
        .and_then(|s| s.draft.as_ref().map(|d| d.id.clone()));

    // Explicit scope first: always relevant, whatever the route.
    let t = Instant::now();
    let scoped = session.store.read(|c| scope_keys(c, actor, req))?;
    for k in scoped {
        ev.entry(k).or_default().scope = true;
    }
    out.timings.insert("scope", t.elapsed().as_micros());

    if matches!(
        route,
        RetrievalRoute::Structured | RetrievalRoute::ProductHelp
    ) {
        out.ranked = rank_all(ev, &query_norm, scope_draft.as_deref(), limit);
        return Ok(out);
    }

    // 1. Lexical (FTS5/BM25) — canonical; available whenever the project is open.
    if route.wants_lexical() {
        let t = Instant::now();
        match session.store.read(|c| lexical(c, actor, &terms)) {
            Ok(hits) => {
                for (rank, (k, title)) in hits.into_iter().enumerate() {
                    let e = ev.entry(k).or_default();
                    e.lexical_rank.get_or_insert(rank);
                    if e.title.is_empty() {
                        e.title = title;
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    code = e.code_str(),
                    "lexical retrieval failed; using scope and tools only"
                );
                out.degraded = true;
            }
        }
        out.timings.insert("lexical", t.elapsed().as_micros());
    }

    // 2. Semantic (vector KNN).
    if route.wants_vectors() {
        let t = Instant::now();
        let svc = super::service(core);
        let usable = snapshot.is_some_and(|s| s.embedder_present && s.counts.embedded > 0);
        let embedder = if usable { svc.embedder(core) } else { None };
        let mut ok = false;
        if let (Some(ix), Some(e)) = (ix, embedder)
            && !req.query.trim().is_empty()
        {
            let qv = svc.query_cache().get_or_embed(e.as_ref(), &req.query);
            out.timings.insert("embed_query", t.elapsed().as_micros());
            let t2 = Instant::now();
            match qv.and_then(|qv| {
                ix.with_reader(|c| {
                    let hits = SqliteVecIndex.search(c, &qv, VECTOR_K)?;
                    let rows = docs::chunks_by_vec_rowids(
                        c,
                        &hits.iter().map(|h| h.vec_rowid).collect::<Vec<_>>(),
                    )?;
                    Ok(rows)
                })
            }) {
                Ok(rows) => {
                    ok = true;
                    let mut rank = 0usize;
                    let mut seen = HashSet::new();
                    for (chunk, doc) in rows {
                        // Privacy + draft boundary on the derived copy (re-checked canonically later).
                        if doc
                            .owner_user_id
                            .as_deref()
                            .is_some_and(|o| o != actor.user_id)
                        {
                            continue;
                        }
                        if let (Some(d), Some(k)) = (&scope_draft, &doc.scope_key)
                            && d != k
                        {
                            continue;
                        }
                        let Some(key) = DocKey::parse(&doc.doc_id) else {
                            continue;
                        };
                        if !seen.insert(key.clone()) {
                            continue;
                        }
                        let e = ev.entry(key).or_default();
                        e.vector_rank = Some(rank);
                        e.chunk = Some(chunk.ordinal);
                        if e.title.is_empty() {
                            e.title = doc.title.clone();
                        }
                        rank += 1;
                    }
                }
                Err(err) => tracing::warn!(error = %err, "semantic retrieval failed; using search"),
            }
            out.timings.insert("vector", t2.elapsed().as_micros());
        }
        if !ok {
            out.degraded = true;
        }
    }

    // 3. Graph expansion from the best entities (bounded, permission-filtered per node).
    if route == RetrievalRoute::HybridGraph {
        let t = Instant::now();
        let prelim = rank_all(ev.clone(), &query_norm, scope_draft.as_deref(), GRAPH_SEEDS);
        let mut seeds: Vec<String> = prelim
            .iter()
            .map(|h| graph::node_id(h.key.kind, &h.key.id))
            .collect();
        // Breakdown documents hang off their scene's node.
        for h in &prelim {
            if h.key.kind == "scene_breakdown" {
                seeds.push(graph::node_id("screenplay_scene", &h.key.id));
            }
        }
        match ix {
            Some(ix) if !seeds.is_empty() => {
                let limits = TraversalLimits {
                    max_hops: if cross_module(&req.query) { 2 } else { 1 },
                    max_nodes: 16,
                    max_edges_per_node: 12,
                    relation_weights: relation_weights(&req.query),
                    default_weight: 0.4,
                    include_inferred: false,
                };
                let user = actor.user_id.clone();
                let allow = move |n: &graph::NodeRecord| {
                    n.owner_user_id.as_deref().is_none_or(|o| o == user)
                };
                match ix.with_reader(|c| graph::expand(c, &seeds, &limits, &allow)) {
                    Ok(hits) => {
                        for h in hits {
                            let Some(key) = DocKey::parse(&h.node.node_id) else {
                                continue;
                            };
                            let e = ev.entry(key).or_default();
                            if h.score > e.graph_score {
                                e.graph_score = h.score;
                                e.graph_relation = Some(h.relation.clone());
                            }
                            if e.title.is_empty() {
                                e.title = h.node.label.clone();
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "graph expansion unavailable");
                        out.degraded = true;
                    }
                }
            }
            Some(_) => {}
            None => out.degraded = true,
        }
        out.timings.insert("graph", t.elapsed().as_micros());
    }

    let t = Instant::now();
    out.ranked = rank_all(
        ev,
        &query_norm,
        scope_draft.as_deref(),
        limit.max(1) * 3 + 16,
    );
    out.timings.insert("rank", t.elapsed().as_micros());
    Ok(out)
}

fn rank_all(
    ev: BTreeMap<DocKey, Evidence>,
    query_norm: &str,
    _scope_draft: Option<&str>,
    take: usize,
) -> Vec<RankedHit> {
    let w = RankWeights::default();
    let keys: Vec<(String, DocKey, Evidence)> =
        ev.into_iter().map(|(k, e)| (k.doc_id(), k, e)).collect();
    let cands: Vec<Candidate> = keys
        .iter()
        .map(|(id, k, e)| Candidate {
            key: id.clone(),
            lexical_rank: e.lexical_rank,
            vector_rank: e.vector_rank,
            graph_score: e.graph_score,
            scope_match: e.scope,
            exact_name: !e.title.is_empty() && names_exactly(query_norm, &e.title),
            // The index only holds the current draft's scenes, and the vector path
            // already dropped other-draft documents for a specific-draft scope.
            current_draft: matches!(k.kind, "screenplay_scene" | "scene_breakdown"),
        })
        .collect();
    let by_id: std::collections::HashMap<&str, (&DocKey, &Evidence)> = keys
        .iter()
        .map(|(id, k, e)| (id.as_str(), (k, e)))
        .collect();
    rank::rank(&cands, &w)
        .into_iter()
        .take(take)
        .filter_map(|r| {
            let (k, e) = by_id.get(r.key.as_str())?;
            Some(RankedHit {
                key: (*k).clone(),
                score: r.score,
                reasons: r.reasons,
                evidence: (*e).clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_drop_question_words_and_fts_query_is_injection_safe() {
        let t = query_terms("Where does Ravi begin losing trust in Anjali?");
        assert_eq!(t, vec!["ravi", "losing", "trust", "anjali"]);
        let q = fts_or_query(&query_terms("NEAR(\"x\" OR y) ravi")).unwrap();
        assert_eq!(q, "\"near\"* OR \"ravi\"*");
        assert!(fts_or_query(&query_terms("the of and")).is_none());
    }

    #[test]
    fn exact_name_matching_is_whole_word() {
        let q = norm("What happens at the Railway Station in scene 12?");
        assert!(names_exactly(&q, "Railway Station"));
        assert!(names_exactly(&q, "Scene 12 — INT. HOSPITAL - DAY"));
        assert!(!names_exactly(&q, "Scene 120 — INT. HOSPITAL - DAY"));
        assert!(!names_exactly(&norm("ravine"), "Ravi"));
        assert!(names_exactly(&norm("where is ravi"), "Ravi"));
    }

    #[test]
    fn relation_priorities_follow_the_question() {
        let w = relation_weights("what props does the shoot need");
        let get = |r: &str| w.iter().find(|(x, _)| x == r).map(|(_, v)| *v).unwrap();
        assert_eq!(get("requires"), 1.0);
        assert_eq!(get("scheduled_on"), 1.0);
        assert!(cross_module("props for the railway scene"));
        assert!(!cross_module("ravi"));
    }
}
