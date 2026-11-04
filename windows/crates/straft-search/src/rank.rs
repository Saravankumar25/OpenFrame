//! Hybrid ranking (§19): Reciprocal Rank Fusion of the lexical (FTS5/BM25) and
//! vector lists, plus small, explainable boosts. No LLM reranker.
//!
//! Scale: one list's top hit contributes 1/(k+1) ≈ 0.0164 with k = 60, so the
//! boosts below are "worth" roughly one to two top-rank positions each.

use std::collections::HashMap;

/// Fuse ranked lists of keys: score(key) = Σ 1 / (k + rank + 1).
pub fn rrf_fuse(lists: &[&[String]], k: f32) -> HashMap<String, f32> {
    let mut out: HashMap<String, f32> = HashMap::new();
    for list in lists {
        for (rank, key) in list.iter().enumerate() {
            *out.entry(key.clone()).or_default() += 1.0 / (k + rank as f32 + 1.0);
        }
    }
    out
}

/// One candidate entity with the evidence gathered for it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Candidate {
    pub key: String,
    /// 0-based rank in the lexical list.
    pub lexical_rank: Option<usize>,
    /// 0-based rank in the vector list.
    pub vector_rank: Option<usize>,
    /// Graph-expansion score in [0, 1] (0 = not reached through the graph).
    pub graph_score: f32,
    /// Inside the user's explicit scope (selected object, open scene, chosen draft…).
    pub scope_match: bool,
    /// The query names this entity exactly (e.g. a character or location name).
    pub exact_name: bool,
    /// Belongs to the draft the request is about (current draft by default).
    pub current_draft: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RankWeights {
    pub k: f32,
    pub lexical: f32,
    pub vector: f32,
    pub graph: f32,
    pub scope_boost: f32,
    pub exact_boost: f32,
    pub draft_boost: f32,
}

impl Default for RankWeights {
    fn default() -> Self {
        Self {
            k: 60.0,
            lexical: 1.0,
            vector: 1.0,
            graph: 0.012,
            scope_boost: 0.02,
            exact_boost: 0.016,
            draft_boost: 0.004,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub key: String,
    pub score: f32,
    /// Why it ranked ("lexical", "semantic", "graph", "scope", "exact", "draft").
    pub reasons: Vec<&'static str>,
}

pub fn score(c: &Candidate, w: &RankWeights) -> Ranked {
    let mut s = 0.0;
    let mut reasons = Vec::new();
    if let Some(r) = c.lexical_rank {
        s += w.lexical / (w.k + r as f32 + 1.0);
        reasons.push("lexical");
    }
    if let Some(r) = c.vector_rank {
        s += w.vector / (w.k + r as f32 + 1.0);
        reasons.push("semantic");
    }
    if c.graph_score > 0.0 {
        s += w.graph * c.graph_score.min(1.0);
        reasons.push("graph");
    }
    if c.scope_match {
        s += w.scope_boost;
        reasons.push("scope");
    }
    if c.exact_name {
        s += w.exact_boost;
        reasons.push("exact");
    }
    if c.current_draft && s > 0.0 {
        s += w.draft_boost;
        reasons.push("draft");
    }
    Ranked {
        key: c.key.clone(),
        score: s,
        reasons,
    }
}

/// Rank candidates, best first. Deterministic: ties break on the key.
pub fn rank(candidates: &[Candidate], w: &RankWeights) -> Vec<Ranked> {
    let mut out: Vec<Ranked> = candidates
        .iter()
        .map(|c| score(c, w))
        .filter(|r| r.score > 0.0)
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.key.cmp(&b.key))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rrf_rewards_agreement_between_lists() {
        let lex = keys(&["a", "b", "c"]);
        let vec = keys(&["c", "d", "a"]);
        let fused = rrf_fuse(&[&lex, &vec], 60.0);
        let mut order: Vec<_> = fused.into_iter().collect();
        order.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap().then(x.0.cmp(&y.0)));
        assert_eq!(order[0].0, "a");
        assert_eq!(order[1].0, "c");
        assert!(order.iter().any(|(k, _)| k == "d"));
    }

    #[test]
    fn boosts_are_explainable_and_bounded() {
        let w = RankWeights::default();
        let base = Candidate {
            key: "x".into(),
            lexical_rank: Some(0),
            ..Default::default()
        };
        let scoped = Candidate {
            key: "y".into(),
            lexical_rank: Some(3),
            scope_match: true,
            ..Default::default()
        };
        let ranked = rank(&[base.clone(), scoped], &w);
        assert_eq!(ranked[0].key, "y");
        assert_eq!(ranked[0].reasons, vec!["lexical", "scope"]);
        // Draft freshness never creates relevance on its own.
        let only_draft = Candidate {
            key: "z".into(),
            current_draft: true,
            ..Default::default()
        };
        assert!(rank(&[only_draft], &w).is_empty());
        // Graph-only candidates rank below direct top hits.
        let graph_only = Candidate {
            key: "g".into(),
            graph_score: 1.0,
            ..Default::default()
        };
        let r = rank(&[graph_only, base], &w);
        assert_eq!(r[0].key, "x");
    }

    #[test]
    fn ties_are_deterministic() {
        let w = RankWeights::default();
        let a = Candidate {
            key: "b".into(),
            vector_rank: Some(0),
            ..Default::default()
        };
        let b = Candidate {
            key: "a".into(),
            lexical_rank: Some(0),
            ..Default::default()
        };
        let r = rank(&[a, b], &w);
        assert_eq!(r[0].key, "a");
    }
}
