//! Global search (FSD §41; docs/engineering/16-search.md).
//! Plain local full-text matching over indexed projections of canonical rows.
//! Private documents are only returned to their owner (Security §10.2).

use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::core::AppCore;
use crate::events::StoreKind;
use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    r.query("search.query", query);
    r.command("search.rebuild", rebuild);
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchArgs {
    pub text: String,
    /// FSD §41.1: current project by default; optionally include the Global Idea Vault.
    #[serde(default)]
    pub include_global: bool,
    /// Restrict to entity types (e.g. ["vault_item"]) — used by workspace-local search boxes.
    #[serde(default)]
    pub entity_types: Option<Vec<String>>,
    #[serde(default)]
    pub limit: Option<u32>,
    /// Search only the Global Idea Vault (no project needed).
    #[serde(default)]
    pub global_only: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub store: StoreKind,
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    /// Matched excerpt; matches are wrapped in \u0001 … \u0002 markers.
    pub snippet: String,
    pub context: String,
    #[ts(type = "unknown")]
    pub nav: Value,
    pub rank: f64,
}

/// Turn user text into a safe FTS5 query: every term is quoted (so operators and
/// punctuation in user input are literal) and prefix-matched; terms are ANDed.
pub fn fts_query(text: &str) -> Option<String> {
    let terms: Vec<String> = text
        .split(|c: char| c.is_whitespace())
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .take(12)
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn run(
    c: &Connection,
    store: StoreKind,
    q: &str,
    user_id: &str,
    types: Option<&[String]>,
    limit: u32,
) -> AppResult<Vec<SearchHit>> {
    let mut stmt = c.prepare(
        "SELECT d.entity_type, d.entity_id, d.title,
                snippet(search_fts, 1, char(1), char(2), '…', 14),
                d.context, d.nav_json, bm25(search_fts, 4.0, 1.0)
         FROM search_fts JOIN search_doc d ON d.rowid = search_fts.rowid
         WHERE search_fts MATCH ?1 AND (d.owner_user_id IS NULL OR d.owner_user_id = ?2)
         ORDER BY bm25(search_fts, 4.0, 1.0) LIMIT ?3",
    )?;
    let rows = stmt
        .query_map(params![q, user_id, limit * 3], |r| {
            let nav: String = r.get(5)?;
            Ok(SearchHit {
                store,
                entity_type: r.get(0)?,
                entity_id: r.get(1)?,
                title: r.get(2)?,
                snippet: r.get(3)?,
                context: r.get(4)?,
                nav: serde_json::from_str(&nav).unwrap_or(Value::Null),
                rank: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|h| {
            types
                .map(|t| t.iter().any(|x| x == &h.entity_type))
                .unwrap_or(true)
        })
        .take(limit as usize)
        .collect())
}

fn query(core: &AppCore, actor: &Actor, args: SearchArgs) -> AppResult<Vec<SearchHit>> {
    actor.require(Capability::View, "search this project")?;
    if args.text.chars().count() > 500 {
        return Err(AppError::invalid_input("That search is too long."));
    }
    let Some(q) = fts_query(&args.text) else {
        return Ok(vec![]);
    };
    let limit = args.limit.unwrap_or(50).clamp(1, 500);
    let types = args.entity_types.as_deref();
    let mut hits = Vec::new();
    if !args.global_only {
        let s = core.project()?;
        hits.extend(
            s.store
                .read(|c| run(c, StoreKind::Project, &q, &actor.user_id, types, limit))?,
        );
    }
    if args.include_global || args.global_only {
        let g = core.global_store()?;
        hits.extend(g.read(|c| run(c, StoreKind::Global, &q, &actor.user_id, types, limit))?);
    }
    hits.sort_by(|a, b| {
        a.rank
            .partial_cmp(&b.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit as usize);
    Ok(hits)
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RebuildArgs {}

fn rebuild(core: &AppCore, actor: &Actor, _: RebuildArgs) -> AppResult<usize> {
    actor.require(Capability::View, "rebuild search")?;
    let s = core.project()?;
    s.store.rebuild_search_index()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_text_cannot_inject_fts_syntax() {
        assert_eq!(
            fts_query("red motorcycle").unwrap(),
            "\"red\"* AND \"motorcycle\"*"
        );
        assert_eq!(
            fts_query("NEAR( \"x\" OR y)").unwrap(),
            "\"NEAR\"* AND \"x\"* AND \"OR\"* AND \"y\"*"
        );
        assert!(fts_query("  ** ").is_none());
    }
}
