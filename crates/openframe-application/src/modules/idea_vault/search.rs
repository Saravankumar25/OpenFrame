//! Vault search (FSD §5.9, FSD-IDEA-009): the vault_item search projection and
//! vault-scoped ranking over the shared local full-text index.

use openframe_domain::AppResult;
use openframe_domain::enums::VaultItemType;
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;

use super::model::{VaultItemDto, display_name};
use crate::modules::search::fts_query;
use crate::registry::SearchDoc;
use crate::util::load_asset_opt;

/// Search projection of one vault item: title (user title or generated display
/// name) plus note text, caption, URL, quote source, filename and tags.
pub(crate) fn index_item(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    #[allow(clippy::type_complexity)]
    let row: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT item_type, title, body, caption, url, source_text, asset_id, deleted_at FROM vault_item WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .optional()?;
    let Some((ty, title, body, caption, url, source, asset_id, None)) = row else {
        return Ok(None);
    };
    let item_type = VaultItemType::parse(&ty).unwrap_or(VaultItemType::File);
    // The asset path is irrelevant for indexing; only its name/duration are used.
    let asset = load_asset_opt(c, std::path::Path::new(""), asset_id.as_deref())?;
    let tags: Vec<String> = {
        let mut stmt = c.prepare("SELECT tag FROM vault_item_tag WHERE item_id=?1 ORDER BY tag")?;
        stmt.query_map([id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    let name = display_name(
        item_type,
        title.as_deref(),
        body.as_deref(),
        caption.as_deref(),
        url.as_deref(),
        asset.as_ref(),
    );
    let mut parts: Vec<String> = Vec::new();
    for v in [body, caption, url, source] {
        if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
            parts.push(v);
        }
    }
    if let Some(a) = &asset {
        parts.push(a.original_name.clone());
    }
    if !tags.is_empty() {
        parts.push(
            tags.iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    Ok(Some(SearchDoc {
        entity_type: "vault_item".into(),
        title: name,
        body: parts.join("\n"),
        context: "Idea Vault".into(),
        nav: json!({ "workspace": "vault", "itemId": id }),
        owner_user_id: None,
    }))
}

/// Vault item ids matching `text`, best match first.
pub(crate) fn ranked_ids(c: &Connection, text: &str) -> AppResult<Vec<String>> {
    let Some(q) = fts_query(text) else {
        return Ok(vec![]);
    };
    let mut stmt = c.prepare(
        "SELECT d.entity_id FROM search_fts JOIN search_doc d ON d.rowid = search_fts.rowid
         WHERE search_fts MATCH ?1 AND d.source_table = 'vault_item'
         ORDER BY bm25(search_fts, 4.0, 1.0) LIMIT 2000",
    )?;
    let ids = stmt
        .query_map([q], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

fn terms(text: &str) -> Vec<String> {
    text.split(char::is_whitespace)
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// True when any term prefix-matches a word of `field` (mirrors FTS prefix matching).
fn field_matches(field: &str, terms: &[String]) -> bool {
    let lower = field.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    terms
        .iter()
        .any(|t| words.iter().any(|w| w.starts_with(t.as_str())))
}

/// "Matched in caption & tag" (mock 042). None when no single field explains the hit.
pub(crate) fn match_reason(item: &VaultItemDto, text: &str) -> Option<String> {
    let ts = terms(text);
    if ts.is_empty() {
        return None;
    }
    let mut hits: Vec<&str> = Vec::new();
    let mut check = |label: &'static str, v: Option<&str>| {
        if v.map(|v| field_matches(v, &ts)).unwrap_or(false) && !hits.contains(&label) {
            hits.push(label);
        }
    };
    check("title", item.title.as_deref());
    check("text", item.body.as_deref());
    check("caption", item.caption.as_deref());
    check("link", item.url.as_deref());
    check("source", item.source_text.as_deref());
    check(
        "filename",
        item.asset.as_ref().map(|a| a.original_name.as_str()),
    );
    let tag_hit = item.tags.iter().any(|t| field_matches(t, &ts));
    if tag_hit {
        hits.push("tag");
    }
    if hits.is_empty() {
        // Untitled items are found through their generated name's source text.
        if field_matches(&item.display_name, &ts) {
            return Some("Matched in name".into());
        }
        return None;
    }
    let joined = match hits.len() {
        1 => hits[0].to_string(),
        n => format!("{} & {}", hits[..n - 1].join(", "), hits[n - 1]),
    };
    Some(format!("Matched in {joined}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_word_matching() {
        let t = terms("stat");
        assert!(field_matches("old-station platform.jpg", &t));
        assert!(!field_matches("constant", &t));
    }
}
