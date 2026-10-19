//! Small validated row helpers shared by application modules.
//!
//! Conventions for canonical tables (docs/engineering/11-physical-database.md):
//! `id TEXT PRIMARY KEY`, `created_at INTEGER`, `updated_at INTEGER`,
//! `rev INTEGER NOT NULL DEFAULT 1` (optimistic concurrency), optional
//! `deleted_at INTEGER` (recoverable delete) and `position INTEGER` for
//! user-controlled sibling order. Display numbers are never stored.

use openframe_domain::{AppError, AppResult, now_ms};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params_from_iter};

use crate::undo::quote_ident;

/// Update allow-listed columns of one row, bumping `rev` and `updated_at`.
/// When `expected_rev` is given and differs, returns `conflict.stale` (stale-base safety for packages and AI Change Sets).
pub fn update_fields(
    conn: &Connection,
    table: &str,
    id: &str,
    fields: &[(&str, SqlValue)],
    allowed: &[&str],
    expected_rev: Option<i64>,
    what: &str,
) -> AppResult<i64> {
    for (col, _) in fields {
        if !allowed.contains(col) {
            return Err(AppError::invalid_input(format!(
                "'{col}' cannot be changed here."
            )));
        }
    }
    let t = quote_ident(table);
    let rev: Option<i64> = conn
        .query_row(&format!("SELECT rev FROM {t} WHERE id = ?1"), [id], |r| {
            r.get(0)
        })
        .optional()?;
    let Some(rev) = rev else {
        return Err(AppError::not_found(what));
    };
    if let Some(expected) = expected_rev
        && expected != rev
    {
        return Err(AppError::stale(what));
    }
    if fields.is_empty() {
        return Ok(rev);
    }
    let mut sets: Vec<String> = fields
        .iter()
        .enumerate()
        .map(|(i, (c, _))| format!("{} = ?{}", quote_ident(c), i + 1))
        .collect();
    let n = fields.len();
    sets.push(format!("rev = rev + 1, updated_at = ?{}", n + 1));
    let mut params: Vec<SqlValue> = fields.iter().map(|(_, v)| v.clone()).collect();
    params.push(SqlValue::Integer(now_ms()));
    params.push(SqlValue::Text(id.to_string()));
    conn.execute(
        &format!("UPDATE {t} SET {} WHERE id = ?{}", sets.join(", "), n + 2),
        params_from_iter(params),
    )?;
    Ok(rev + 1)
}

/// Ordered sibling ids within a scope (`where_sql` uses `?1..` placeholders).
pub fn sibling_ids(
    conn: &Connection,
    table: &str,
    where_sql: &str,
    params: &[SqlValue],
) -> AppResult<Vec<String>> {
    let sql = format!(
        "SELECT id FROM {} WHERE {where_sql} ORDER BY position, id",
        quote_ident(table)
    );
    let mut stmt = conn.prepare(&sql)?;
    let ids = stmt
        .query_map(params_from_iter(params.iter()), |r| r.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(ids)
}

/// Position for appending a new row at the end of a scope.
pub fn next_position(
    conn: &Connection,
    table: &str,
    where_sql: &str,
    params: &[SqlValue],
) -> AppResult<i64> {
    let sql = format!(
        "SELECT COALESCE(MAX(position), 0) + 1 FROM {} WHERE {where_sql}",
        quote_ident(table)
    );
    Ok(conn.query_row(&sql, params_from_iter(params.iter()), |r| r.get(0))?)
}

/// Rewrite `position` = 1..n for the given ordered ids. Only rows whose position
/// changes are touched, keeping undo entries and revision bumps minimal.
pub fn renumber(conn: &Connection, table: &str, ordered_ids: &[String]) -> AppResult<()> {
    let t = quote_ident(table);
    let now = now_ms();
    for (i, id) in ordered_ids.iter().enumerate() {
        conn.execute(
            &format!(
                "UPDATE {t} SET position = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3 AND position IS NOT ?1"
            ),
            rusqlite::params![(i + 1) as i64, now, id],
        )?;
    }
    Ok(())
}

/// Insert `id` into an ordered sibling list at `index` (clamped) and renumber.
pub fn place_at(
    conn: &Connection,
    table: &str,
    mut siblings: Vec<String>,
    id: &str,
    index: Option<usize>,
) -> AppResult<()> {
    siblings.retain(|s| s != id);
    let idx = index.unwrap_or(siblings.len()).min(siblings.len());
    siblings.insert(idx, id.to_string());
    renumber(conn, table, &siblings)
}

pub fn text(v: impl Into<String>) -> SqlValue {
    SqlValue::Text(v.into())
}

pub fn opt_text(v: Option<impl Into<String>>) -> SqlValue {
    v.map(|s| SqlValue::Text(s.into()))
        .unwrap_or(SqlValue::Null)
}

pub fn int(v: i64) -> SqlValue {
    SqlValue::Integer(v)
}

pub fn opt_int(v: Option<i64>) -> SqlValue {
    v.map(SqlValue::Integer).unwrap_or(SqlValue::Null)
}

/// Normalize user text: trim, collapse to None when empty.
pub fn clean(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = crate::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE cards(id TEXT PRIMARY KEY, parent TEXT, title TEXT, position INTEGER NOT NULL,
                                rev INTEGER NOT NULL DEFAULT 1, updated_at INTEGER);
             INSERT INTO cards(id,parent,title,position) VALUES ('a','p','A',1),('b','p','B',2),('c','p','C',3);",
        )
        .unwrap();
        c
    }

    #[test]
    fn update_respects_allow_list_and_revision() {
        let c = db();
        let rev = update_fields(
            &c,
            "cards",
            "a",
            &[("title", text("A2"))],
            &["title"],
            Some(1),
            "card",
        )
        .unwrap();
        assert_eq!(rev, 2);
        let err = update_fields(
            &c,
            "cards",
            "a",
            &[("title", text("A3"))],
            &["title"],
            Some(1),
            "card",
        )
        .unwrap_err();
        assert_eq!(err.code_str(), "conflict.stale");
        let err = update_fields(
            &c,
            "cards",
            "a",
            &[("parent", text("x"))],
            &["title"],
            None,
            "card",
        )
        .unwrap_err();
        assert!(err.is("validation"));
        let err = update_fields(
            &c,
            "cards",
            "zz",
            &[("title", text("x"))],
            &["title"],
            None,
            "card",
        )
        .unwrap_err();
        assert!(err.is("not_found"));
    }

    #[test]
    fn place_at_reorders_without_changing_identity() {
        let c = db();
        let sibs = sibling_ids(&c, "cards", "parent = ?1", &[text("p")]).unwrap();
        place_at(&c, "cards", sibs, "c", Some(0)).unwrap();
        let order = sibling_ids(&c, "cards", "parent = ?1", &[text("p")]).unwrap();
        assert_eq!(order, vec!["c", "a", "b"]);
        assert_eq!(
            next_position(&c, "cards", "parent = ?1", &[text("p")]).unwrap(),
            4
        );
    }
}
