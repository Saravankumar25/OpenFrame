//! Derived context graph inside SQLite (§16–§18). No graph database.
//!
//! Edges are grouped into *fragments* owned by one canonical entity (the
//! `origin_key`): rebuilding that entity replaces exactly its fragment, so the
//! graph stays consistent under incremental updates. Edge provenance records
//! whether a relation is a canonical foreign key / link row (`Canonical`), a
//! deterministic parse or rule (`Derived`) or model output (`Inferred` —
//! not produced in v1 and excluded from traversal unless asked for).

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::{SearchResult, content_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeProvenance {
    Canonical,
    Derived,
    Inferred,
}

impl EdgeProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeProvenance::Canonical => "Canonical",
            EdgeProvenance::Derived => "Derived",
            EdgeProvenance::Inferred => "Inferred",
        }
    }
    pub fn parse(s: &str) -> EdgeProvenance {
        match s {
            "Canonical" => EdgeProvenance::Canonical,
            "Derived" => EdgeProvenance::Derived,
            _ => EdgeProvenance::Inferred,
        }
    }
}

pub fn node_id(entity_type: &str, entity_id: &str) -> String {
    format!("{entity_type}:{entity_id}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeInput {
    pub entity_type: String,
    pub entity_id: String,
    pub label: String,
    pub module: String,
    pub owner_user_id: Option<String>,
    pub source_rev: Option<i64>,
    pub metadata: Value,
}

impl NodeInput {
    pub fn id(&self) -> String {
        node_id(&self.entity_type, &self.entity_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeInput {
    /// (entity_type, entity_id)
    pub from: (String, String),
    pub relation: String,
    pub to: (String, String),
    pub provenance: EdgeProvenance,
    pub weight: f32,
    pub source_rev: Option<i64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeRecord {
    pub node_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub label: String,
    pub module: String,
    pub owner_user_id: Option<String>,
}

fn upsert_node(conn: &Connection, n: &NodeInput, overwrite: bool) -> SearchResult<()> {
    let sql = if overwrite {
        "INSERT INTO context_node(node_id, entity_type, entity_id, label, module, owner_user_id, source_rev, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(node_id) DO UPDATE SET label=excluded.label, module=excluded.module,
             owner_user_id=excluded.owner_user_id, source_rev=excluded.source_rev, metadata_json=excluded.metadata_json"
    } else {
        "INSERT OR IGNORE INTO context_node(node_id, entity_type, entity_id, label, module, owner_user_id, source_rev, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    };
    conn.prepare_cached(sql)?.execute(params![
        n.id(),
        n.entity_type,
        n.entity_id,
        n.label,
        n.module,
        n.owner_user_id,
        n.source_rev,
        n.metadata.to_string()
    ])?;
    Ok(())
}

/// Replace the fragment owned by `origin_key`: its own node(s) are upserted,
/// referenced nodes are created only when missing (their own fragment owns
/// their label), and the fragment's edges are replaced.
pub fn replace_fragment(
    conn: &Connection,
    origin_key: &str,
    own: &[NodeInput],
    referenced: &[NodeInput],
    edges: &[EdgeInput],
) -> SearchResult<()> {
    conn.prepare_cached("DELETE FROM context_edge WHERE origin_key=?1")?
        .execute([origin_key])?;
    for n in own {
        upsert_node(conn, n, true)?;
    }
    for n in referenced {
        upsert_node(conn, n, false)?;
    }
    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO context_edge(edge_id, from_node_id, relation, to_node_id, provenance, weight, source_rev, origin_key, metadata_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    for e in edges {
        let from = node_id(&e.from.0, &e.from.1);
        let to = node_id(&e.to.0, &e.to.1);
        if from == to {
            continue;
        }
        let edge_id = content_hash(&[origin_key, &from, &e.relation, &to]);
        stmt.execute(params![
            edge_id,
            from,
            e.relation,
            to,
            e.provenance.as_str(),
            e.weight as f64,
            e.source_rev,
            origin_key,
            e.metadata.to_string()
        ])?;
    }
    Ok(())
}

/// Remove a fragment and the given own nodes (the entity no longer exists or
/// is no longer indexable). Edges other fragments hold towards these nodes stay
/// dormant: traversal skips edges whose node is missing, and they come alive
/// again if the entity is restored.
pub fn remove_fragment(
    conn: &Connection,
    origin_key: &str,
    own_node_ids: &[String],
) -> SearchResult<()> {
    conn.prepare_cached("DELETE FROM context_edge WHERE origin_key=?1")?
        .execute([origin_key])?;
    for id in own_node_ids {
        conn.prepare_cached("DELETE FROM context_node WHERE node_id=?1")?
            .execute([id])?;
    }
    Ok(())
}

/// Drop nodes nothing refers to any more (no edges, no document).
pub fn gc_nodes(conn: &Connection) -> SearchResult<usize> {
    Ok(conn.execute(
        "DELETE FROM context_node WHERE
            NOT EXISTS (SELECT 1 FROM context_edge e WHERE e.from_node_id = context_node.node_id)
        AND NOT EXISTS (SELECT 1 FROM context_edge e WHERE e.to_node_id = context_node.node_id)
        AND NOT EXISTS (SELECT 1 FROM semantic_document d
                        WHERE d.entity_type = context_node.entity_type AND d.entity_id = context_node.entity_id)",
        [],
    )?)
}

/// Bounds for graph expansion (§18): 1 hop normally, 2 when useful, capped.
#[derive(Debug, Clone)]
pub struct TraversalLimits {
    /// Clamped to 1..=2.
    pub max_hops: u8,
    pub max_nodes: usize,
    pub max_edges_per_node: usize,
    /// Relation priorities (higher = expanded first, scores more). Unlisted relations use `default_weight`.
    pub relation_weights: Vec<(String, f32)>,
    pub default_weight: f32,
    pub include_inferred: bool,
}

impl Default for TraversalLimits {
    fn default() -> Self {
        Self {
            max_hops: 1,
            max_nodes: 24,
            max_edges_per_node: 16,
            relation_weights: Vec::new(),
            default_weight: 0.5,
            include_inferred: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphHit {
    pub node: NodeRecord,
    pub hops: u8,
    pub relation: String,
    /// The node this one was reached from.
    pub via: String,
    pub provenance: EdgeProvenance,
    pub score: f32,
}

struct Neighbor {
    node: NodeRecord,
    relation: String,
    provenance: EdgeProvenance,
    weight: f32,
}

fn neighbors(conn: &Connection, node: &str, cap: usize) -> SearchResult<Vec<Neighbor>> {
    let mut stmt = conn.prepare_cached(
        "SELECT e.relation, e.provenance, e.weight, n.node_id, n.entity_type, n.entity_id, n.label, n.module, n.owner_user_id
         FROM context_edge e JOIN context_node n ON n.node_id = e.to_node_id
         WHERE e.from_node_id = ?1
         UNION ALL
         SELECT e.relation, e.provenance, e.weight, n.node_id, n.entity_type, n.entity_id, n.label, n.module, n.owner_user_id
         FROM context_edge e JOIN context_node n ON n.node_id = e.from_node_id
         WHERE e.to_node_id = ?1
         LIMIT ?2",
    )?;
    let out = stmt
        .query_map(params![node, cap as i64], |r| {
            let prov: String = r.get(1)?;
            let w: f64 = r.get(2)?;
            Ok(Neighbor {
                relation: r.get(0)?,
                provenance: EdgeProvenance::parse(&prov),
                weight: w as f32,
                node: NodeRecord {
                    node_id: r.get(3)?,
                    entity_type: r.get(4)?,
                    entity_id: r.get(5)?,
                    label: r.get(6)?,
                    module: r.get(7)?,
                    owner_user_id: r.get(8)?,
                },
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(out)
}

pub fn get_node(conn: &Connection, node: &str) -> SearchResult<Option<NodeRecord>> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row(
            "SELECT node_id, entity_type, entity_id, label, module, owner_user_id FROM context_node WHERE node_id=?1",
            [node],
            |r| {
                Ok(NodeRecord {
                    node_id: r.get(0)?,
                    entity_type: r.get(1)?,
                    entity_id: r.get(2)?,
                    label: r.get(3)?,
                    module: r.get(4)?,
                    owner_user_id: r.get(5)?,
                })
            },
        )
        .optional()?)
}

/// Bounded breadth-first expansion from `seeds` (node ids). `allow` is called
/// for EVERY node (seeds included): a node that fails it is neither returned
/// nor traversed through, so private or out-of-scope entities can never be
/// reached indirectly. Seeds are not part of the result.
pub fn expand(
    conn: &Connection,
    seeds: &[String],
    limits: &TraversalLimits,
    allow: &dyn Fn(&NodeRecord) -> bool,
) -> SearchResult<Vec<GraphHit>> {
    let hops = limits.max_hops.clamp(1, 2);
    let weights: HashMap<&str, f32> = limits
        .relation_weights
        .iter()
        .map(|(r, w)| (r.as_str(), *w))
        .collect();
    let seed_set: HashSet<&str> = seeds.iter().map(|s| s.as_str()).collect();
    let mut best: HashMap<String, GraphHit> = HashMap::new();
    let mut frontier: Vec<(String, f32)> = Vec::new();
    for s in seeds {
        if let Some(n) = get_node(conn, s)?
            && allow(&n)
        {
            frontier.push((n.node_id, 1.0));
        }
    }
    for hop in 1..=hops {
        let mut next: Vec<(String, f32)> = Vec::new();
        for (from, from_score) in &frontier {
            let mut ns = neighbors(conn, from, limits.max_edges_per_node * 2)?;
            ns.retain(|n| limits.include_inferred || n.provenance != EdgeProvenance::Inferred);
            let w = |rel: &str| *weights.get(rel).unwrap_or(&limits.default_weight);
            ns.sort_by(|a, b| {
                w(&b.relation)
                    .partial_cmp(&w(&a.relation))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.node.node_id.cmp(&b.node.node_id))
            });
            for n in ns.into_iter().take(limits.max_edges_per_node) {
                if seed_set.contains(n.node.node_id.as_str()) || !allow(&n.node) {
                    continue;
                }
                let score = from_score * w(&n.relation) * n.weight / hop as f32;
                let id = n.node.node_id.clone();
                let improves = best.get(&id).is_none_or(|h| score > h.score);
                if improves {
                    if !best.contains_key(&id) {
                        next.push((id.clone(), score));
                    }
                    best.insert(
                        id,
                        GraphHit {
                            node: n.node,
                            hops: hop,
                            relation: n.relation,
                            via: from.clone(),
                            provenance: n.provenance,
                            score,
                        },
                    );
                }
            }
        }
        if best.len() >= limits.max_nodes {
            break;
        }
        frontier = next;
    }
    let mut out: Vec<GraphHit> = best.into_values().collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.node.node_id.cmp(&b.node.node_id))
    });
    out.truncate(limits.max_nodes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{IndexIdentity, IntelligenceDb};
    use serde_json::json;

    fn node(t: &str, id: &str, owner: Option<&str>) -> NodeInput {
        NodeInput {
            entity_type: t.into(),
            entity_id: id.into(),
            label: format!("{t} {id}"),
            module: "M".into(),
            owner_user_id: owner.map(str::to_string),
            source_rev: None,
            metadata: json!({}),
        }
    }

    fn edge(from: (&str, &str), rel: &str, to: (&str, &str)) -> EdgeInput {
        EdgeInput {
            from: (from.0.into(), from.1.into()),
            relation: rel.into(),
            to: (to.0.into(), to.1.into()),
            provenance: EdgeProvenance::Canonical,
            weight: 1.0,
            source_rev: None,
            metadata: json!({}),
        }
    }

    fn setup() -> (tempfile::TempDir, IntelligenceDb) {
        let dir = tempfile::tempdir().unwrap();
        let (db, _) = IntelligenceDb::open(
            &dir.path().join("g.sqlite"),
            &IndexIdentity {
                project_id: "p".into(),
                canonical_schema_version: 1,
                builder_version: 1,
            },
        )
        .unwrap();
        let c = db.conn();
        // Ravi --appears_in--> Scene 34 <--appears_in-- Anjali; Scene 34 --represented_by--> Card 19 --belongs_to--> Seq 6
        replace_fragment(
            c,
            "scene:34",
            &[node("scene", "34", None)],
            &[
                node("character", "ravi", None),
                node("character", "anjali", None),
                node("card", "19", None),
            ],
            &[
                edge(("character", "ravi"), "appears_in", ("scene", "34")),
                edge(("character", "anjali"), "appears_in", ("scene", "34")),
                edge(("scene", "34"), "represented_by", ("card", "19")),
            ],
        )
        .unwrap();
        replace_fragment(
            c,
            "card:19",
            &[node("card", "19", None)],
            &[node("sequence", "6", None)],
            &[edge(("card", "19"), "belongs_to", ("sequence", "6"))],
        )
        .unwrap();
        replace_fragment(
            c,
            "note:n1",
            &[node("private_note", "n1", Some("owner"))],
            &[],
            &[edge(("private_note", "n1"), "about", ("scene", "34"))],
        )
        .unwrap();
        (dir, db)
    }

    #[test]
    fn one_hop_then_two_hops_with_caps() {
        let (_d, db) = setup();
        let c = db.conn();
        let all = |_: &NodeRecord| true;
        let one = expand(c, &["scene:34".into()], &TraversalLimits::default(), &all).unwrap();
        let ids: HashSet<_> = one.iter().map(|h| h.node.node_id.as_str()).collect();
        assert!(ids.contains("character:ravi") && ids.contains("card:19"));
        assert!(!ids.contains("sequence:6"));
        let two = expand(
            c,
            &["scene:34".into()],
            &TraversalLimits {
                max_hops: 2,
                ..Default::default()
            },
            &all,
        )
        .unwrap();
        let seq = two.iter().find(|h| h.node.node_id == "sequence:6").unwrap();
        assert_eq!((seq.hops, seq.via.as_str()), (2, "card:19"));
        let capped = expand(
            c,
            &["scene:34".into()],
            &TraversalLimits {
                max_hops: 2,
                max_nodes: 2,
                ..Default::default()
            },
            &all,
        )
        .unwrap();
        assert_eq!(capped.len(), 2);
    }

    #[test]
    fn relation_priority_orders_results() {
        let (_d, db) = setup();
        let limits = TraversalLimits {
            relation_weights: vec![("represented_by".into(), 1.0), ("appears_in".into(), 0.2)],
            ..Default::default()
        };
        let hits = expand(db.conn(), &["scene:34".into()], &limits, &|_| true).unwrap();
        assert_eq!(hits[0].node.node_id, "card:19");
    }

    #[test]
    fn permission_filter_applies_to_every_node() {
        let (_d, db) = setup();
        let c = db.conn();
        let not_private = |n: &NodeRecord| n.owner_user_id.is_none();
        let hits = expand(
            c,
            &["scene:34".into()],
            &TraversalLimits::default(),
            &not_private,
        )
        .unwrap();
        assert!(hits.iter().all(|h| h.node.entity_type != "private_note"));
        // A private seed is not traversed through at all.
        let hits = expand(
            c,
            &["private_note:n1".into()],
            &TraversalLimits::default(),
            &not_private,
        )
        .unwrap();
        assert!(hits.is_empty());
        let owner = |n: &NodeRecord| n.owner_user_id.as_deref().is_none_or(|o| o == "owner");
        let hits = expand(c, &["scene:34".into()], &TraversalLimits::default(), &owner).unwrap();
        assert!(hits.iter().any(|h| h.node.entity_type == "private_note"));
    }

    #[test]
    fn fragments_replace_and_removed_nodes_go_dormant() {
        let (_d, db) = setup();
        let c = db.conn();
        // Rebuild scene 34 without Anjali.
        replace_fragment(
            c,
            "scene:34",
            &[node("scene", "34", None)],
            &[node("character", "ravi", None)],
            &[edge(("character", "ravi"), "appears_in", ("scene", "34"))],
        )
        .unwrap();
        let hits = expand(
            c,
            &["scene:34".into()],
            &TraversalLimits::default(),
            &|_| true,
        )
        .unwrap();
        assert!(!hits.iter().any(|h| h.node.node_id == "character:anjali"));
        // Card 19 deleted: its node disappears, the scene's edge to it goes dormant.
        remove_fragment(c, "card:19", &["card:19".into()]).unwrap();
        let hits = expand(
            c,
            &["scene:34".into()],
            &TraversalLimits::default(),
            &|_| true,
        )
        .unwrap();
        assert!(!hits.iter().any(|h| h.node.node_id == "card:19"));
        let removed = gc_nodes(c).unwrap();
        assert!(removed >= 1, "anjali + sequence 6 are orphaned");
        assert!(get_node(c, "character:anjali").unwrap().is_none());
        assert!(get_node(c, "scene:34").unwrap().is_some());
    }
}
