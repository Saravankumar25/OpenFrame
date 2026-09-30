//! Generic, trigger-based undo/redo capture (ADR-0005).
//!
//! Every canonical table (TEXT `id` primary key, no BLOB columns) gets three
//! TEMP triggers on the writer connection that record before/after images of
//! each row into `temp._undo_capture`. The application command pipeline:
//!
//! 1. clears the capture table at transaction start,
//! 2. runs the command,
//! 3. drains the capture into net per-row [`RowChange`]s and stores them as
//!    one undo entry in the same transaction.
//!
//! Undo restores before-images; redo re-applies after-images. Both first verify
//! that each affected row is still exactly in the state the entry expects, so
//! undo never silently overwrites later work. Because the mechanism is
//! table-generic, every module gets correct undo for free and module code
//! cannot forget to record an inverse.

use std::collections::{BTreeMap, HashMap};

use openframe_domain::{AppError, AppResult};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Net change to one row within an undo entry. `old == None` means the row did
/// not exist before; `new == None` means it does not exist after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowChange {
    pub tbl: String,
    pub row_id: String,
    pub old: Option<Value>,
    pub new: Option<Value>,
}

/// Column metadata for tracked tables.
#[derive(Debug, Clone, Default)]
pub struct TableCatalog {
    tables: HashMap<String, Vec<String>>,
}

impl TableCatalog {
    pub fn is_tracked(&self, table: &str) -> bool {
        self.tables.contains_key(table)
    }
    pub fn columns(&self, table: &str) -> Option<&[String]> {
        self.tables.get(table).map(|v| v.as_slice())
    }
    pub fn tables(&self) -> impl Iterator<Item = &String> {
        self.tables.keys()
    }
}

/// Tables are untracked when their name starts with one of these prefixes.
pub const UNTRACKED_PREFIXES: &[&str] = &["_", "sqlite_", "sys_", "search_"];

fn is_untracked(name: &str) -> bool {
    UNTRACKED_PREFIXES.iter().any(|p| name.starts_with(p))
}

pub(crate) fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Install (or refresh) capture triggers for every tracked table. Idempotent;
/// call after migrations on each writer connection.
pub fn install_capture(conn: &Connection) -> AppResult<TableCatalog> {
    conn.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS _undo_capture(
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            tbl TEXT NOT NULL,
            row_id TEXT NOT NULL,
            op TEXT NOT NULL,
            old_json TEXT,
            new_json TEXT
        );",
    )?;
    let names: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name FROM main.sqlite_master WHERE type='table' AND sql NOT LIKE 'CREATE VIRTUAL TABLE%' ORDER BY name",
        )?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    let mut catalog = TableCatalog::default();
    for table in names.into_iter().filter(|n| !is_untracked(n)) {
        let cols: Vec<(String, String, i64)> = {
            let mut stmt =
                conn.prepare(&format!("PRAGMA main.table_info({})", quote_ident(&table)))?;
            stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(5)?,
                ))
            })?
            .collect::<Result<_, _>>()?
        };
        let has_id_pk = cols.iter().any(|(n, _, pk)| n == "id" && *pk == 1);
        if !has_id_pk {
            return Err(AppError::internal(format!(
                "table '{table}' must have an `id` primary key to be undo-tracked (or use an untracked prefix)"
            )));
        }
        if let Some((n, _, _)) = cols.iter().find(|(_, t, _)| t.eq_ignore_ascii_case("BLOB")) {
            return Err(AppError::internal(format!(
                "table '{table}' column '{n}' is BLOB; canonical tables store binary data as files"
            )));
        }
        let names: Vec<String> = cols.into_iter().map(|(n, _, _)| n).collect();
        let obj = |prefix: &str| {
            let pairs: Vec<String> = names
                .iter()
                .map(|c| format!("'{}', {}.{}", c.replace('\'', "''"), prefix, quote_ident(c)))
                .collect();
            format!("json_object({})", pairs.join(", "))
        };
        let t = quote_ident(&table);
        let lit = table.replace('\'', "''");
        // Trigger names are identifiers: quote them properly (a table name from a received
        // project could otherwise break out of `"…"` inside this multi-statement batch).
        let (ti, tu, td) = (
            quote_ident(&format!("_cap_{table}_i")),
            quote_ident(&format!("_cap_{table}_u")),
            quote_ident(&format!("_cap_{table}_d")),
        );
        conn.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS temp.{ti};
             DROP TRIGGER IF EXISTS temp.{tu};
             DROP TRIGGER IF EXISTS temp.{td};
             CREATE TEMP TRIGGER {ti} AFTER INSERT ON main.{t} BEGIN
               INSERT INTO _undo_capture(tbl,row_id,op,old_json,new_json) VALUES('{lit}', NEW.id, 'I', NULL, {new});
             END;
             CREATE TEMP TRIGGER {tu} AFTER UPDATE ON main.{t} BEGIN
               INSERT INTO _undo_capture(tbl,row_id,op,old_json,new_json) VALUES('{lit}', NEW.id, 'U', {old}, {new});
             END;
             CREATE TEMP TRIGGER {td} AFTER DELETE ON main.{t} BEGIN
               INSERT INTO _undo_capture(tbl,row_id,op,old_json,new_json) VALUES('{lit}', OLD.id, 'D', {old}, NULL);
             END;",
            new = obj("NEW"),
            old = obj("OLD"),
        ))?;
        catalog.tables.insert(table, names);
    }
    Ok(catalog)
}

pub fn reset_capture(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM temp._undo_capture", [])?;
    Ok(())
}

/// Drain raw captures and fold them into net per-row changes (first-seen order).
pub fn drain_capture(conn: &Connection) -> AppResult<Vec<RowChange>> {
    let raw: Vec<(String, String, Option<String>, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT tbl, row_id, old_json, new_json FROM temp._undo_capture ORDER BY seq",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<_, _>>()?
    };
    reset_capture(conn)?;
    let parsed = raw
        .into_iter()
        .map(|(tbl, row_id, old, new)| {
            let parse = |s: Option<String>| -> AppResult<Option<Value>> {
                s.map(|s| serde_json::from_str(&s).map_err(|e| AppError::internal(e.to_string())))
                    .transpose()
            };
            Ok(RowChange {
                tbl,
                row_id,
                old: parse(old)?,
                new: parse(new)?,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(merge(Vec::new(), parsed))
}

/// Fold `next` into `prev`: per row keep the earliest before-image and the
/// latest after-image. Rows whose net effect is nothing are dropped.
pub fn merge(prev: Vec<RowChange>, next: Vec<RowChange>) -> Vec<RowChange> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut map: BTreeMap<(String, String), RowChange> = BTreeMap::new();
    for c in prev.into_iter().chain(next) {
        let key = (c.tbl.clone(), c.row_id.clone());
        match map.get_mut(&key) {
            Some(existing) => existing.new = c.new,
            None => {
                order.push(key.clone());
                map.insert(key, c);
            }
        }
    }
    order
        .into_iter()
        .filter_map(|k| map.remove(&k))
        .filter(|c| c.old != c.new)
        .collect()
}

/// Distinct tables touched by a change list (drives UI invalidation and search reindex).
pub fn touched_tables(changes: &[RowChange]) -> Vec<String> {
    let mut t: Vec<String> = changes.iter().map(|c| c.tbl.clone()).collect();
    t.sort();
    t.dedup();
    t
}

/// Current row image as JSON (same shape the capture triggers record).
pub fn current_row(
    conn: &Connection,
    catalog: &TableCatalog,
    table: &str,
    id: &str,
) -> AppResult<Option<Value>> {
    let cols = catalog
        .columns(table)
        .ok_or_else(|| AppError::internal(format!("untracked table {table}")))?;
    let pairs: Vec<String> = cols
        .iter()
        .map(|c| format!("'{}', {}", c.replace('\'', "''"), quote_ident(c)))
        .collect();
    let sql = format!(
        "SELECT json_object({}) FROM main.{} WHERE id = ?1",
        pairs.join(", "),
        quote_ident(table)
    );
    let json: Option<String> = conn.query_row(&sql, [id], |r| r.get(0)).optional()?;
    json.map(|s| serde_json::from_str(&s).map_err(|e| AppError::internal(e.to_string())))
        .transpose()
}

pub fn json_to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(*b as i64),
        Value::Number(n) => n
            .as_i64()
            .map(SqlValue::Integer)
            .unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// Write a full row image (or delete when `target` is None). Used by undo/redo
/// and by package import to materialize validated rows.
pub fn write_row(
    conn: &Connection,
    catalog: &TableCatalog,
    table: &str,
    id: &str,
    target: Option<&Value>,
) -> AppResult<()> {
    let t = quote_ident(table);
    let Some(target) = target else {
        conn.execute(&format!("DELETE FROM main.{t} WHERE id = ?1"), [id])?;
        return Ok(());
    };
    let cols = catalog
        .columns(table)
        .ok_or_else(|| AppError::internal(format!("untracked table {table}")))?;
    let obj = target
        .as_object()
        .ok_or_else(|| AppError::internal("row image is not an object"))?;
    let values: Vec<SqlValue> = cols
        .iter()
        .map(|c| obj.get(c).map(json_to_sql).unwrap_or(SqlValue::Null))
        .collect();
    let exists: bool = conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM main.{t} WHERE id = ?1)"),
        [id],
        |r| r.get(0),
    )?;
    if exists {
        let sets: Vec<String> = cols
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{} = ?{}", quote_ident(c), i + 1))
            .collect();
        let mut params = values;
        params.push(SqlValue::Text(id.to_string()));
        conn.execute(
            &format!(
                "UPDATE main.{t} SET {} WHERE id = ?{}",
                sets.join(", "),
                cols.len() + 1
            ),
            params_from_iter(params),
        )?;
    } else {
        let names: Vec<String> = cols.iter().map(|c| quote_ident(c)).collect();
        let marks: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
        conn.execute(
            &format!(
                "INSERT INTO main.{t} ({}) VALUES ({})",
                names.join(", "),
                marks.join(", ")
            ),
            params_from_iter(values),
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Undo,
    Redo,
}

/// Verify every row is in the state this entry left it (undo) or found it (redo).
pub fn check_applicable(
    conn: &Connection,
    catalog: &TableCatalog,
    changes: &[RowChange],
    dir: Direction,
) -> AppResult<()> {
    for c in changes {
        if !catalog.is_tracked(&c.tbl) {
            return Err(
                AppError::undo_conflict().with_detail(format!("table {} no longer tracked", c.tbl))
            );
        }
        let expected = match dir {
            Direction::Undo => c.new.as_ref(),
            Direction::Redo => c.old.as_ref(),
        };
        let current = current_row(conn, catalog, &c.tbl, &c.row_id)?;
        if current.as_ref() != expected {
            return Err(
                AppError::undo_conflict().with_detail(format!("{}:{} changed", c.tbl, c.row_id))
            );
        }
    }
    Ok(())
}

/// Apply an entry in the given direction. Must run inside a transaction; the
/// caller sets `PRAGMA defer_foreign_keys=ON` so intermediate FK states are allowed.
pub fn apply(
    conn: &Connection,
    catalog: &TableCatalog,
    changes: &[RowChange],
    dir: Direction,
) -> AppResult<()> {
    check_applicable(conn, catalog, changes, dir)?;
    match dir {
        Direction::Undo => {
            for c in changes.iter().rev() {
                write_row(conn, catalog, &c.tbl, &c.row_id, c.old.as_ref())?;
            }
        }
        Direction::Redo => {
            for c in changes {
                write_row(conn, catalog, &c.tbl, &c.row_id, c.new.as_ref())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Connection, TableCatalog) {
        let c = crate::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE acts(id TEXT PRIMARY KEY, title TEXT NOT NULL, position INTEGER NOT NULL, weight REAL);
             CREATE TABLE sequences(id TEXT PRIMARY KEY, act_id TEXT NOT NULL REFERENCES acts(id), title TEXT);
             CREATE TABLE sys_activity(id TEXT PRIMARY KEY, summary TEXT);",
        )
        .unwrap();
        let cat = install_capture(&c).unwrap();
        (c, cat)
    }

    fn run(c: &Connection, sql: &str) -> Vec<RowChange> {
        reset_capture(c).unwrap();
        c.execute_batch(sql).unwrap();
        drain_capture(c).unwrap()
    }

    fn undo(
        c: &mut Connection,
        cat: &TableCatalog,
        ch: &[RowChange],
        dir: Direction,
    ) -> AppResult<()> {
        let tx = c.transaction().unwrap();
        tx.execute_batch("PRAGMA defer_foreign_keys=ON").unwrap();
        apply(&tx, cat, ch, dir)?;
        tx.commit().unwrap();
        Ok(())
    }

    #[test]
    fn untracked_tables_are_skipped() {
        let (_c, cat) = setup();
        assert!(cat.is_tracked("acts"));
        assert!(!cat.is_tracked("sys_activity"));
    }

    #[test]
    fn insert_update_delete_round_trip() {
        let (mut c, cat) = setup();
        let ch = run(
            &c,
            "INSERT INTO acts VALUES('a1','Act One',1,1.5); INSERT INTO sequences VALUES('s1','a1','Chase');",
        );
        assert_eq!(ch.len(), 2);
        undo(&mut c, &cat, &ch, Direction::Undo).unwrap();
        let n: i64 = c
            .query_row("SELECT count(*) FROM acts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
        undo(&mut c, &cat, &ch, Direction::Redo).unwrap();
        let t: String = c
            .query_row("SELECT title FROM sequences WHERE id='s1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(t, "Chase");

        let upd = run(
            &c,
            "UPDATE acts SET title='Setup', weight=2.25 WHERE id='a1';",
        );
        undo(&mut c, &cat, &upd, Direction::Undo).unwrap();
        let (t, w): (String, f64) = c
            .query_row("SELECT title, weight FROM acts", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((t.as_str(), w), ("Act One", 1.5));

        let del = run(&c, "DELETE FROM sequences WHERE id='s1';");
        undo(&mut c, &cat, &del, Direction::Undo).unwrap();
        let n: i64 = c
            .query_row("SELECT count(*) FROM sequences", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn undo_refuses_to_overwrite_later_changes() {
        let (mut c, cat) = setup();
        run(&c, "INSERT INTO acts VALUES('a1','Act One',1,NULL);");
        let first = run(&c, "UPDATE acts SET title='B' WHERE id='a1';");
        run(&c, "UPDATE acts SET title='C' WHERE id='a1';");
        let err = undo(&mut c, &cat, &first, Direction::Undo).unwrap_err();
        assert_eq!(err.code_str(), "conflict.undo");
        let t: String = c
            .query_row("SELECT title FROM acts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(t, "C", "later work preserved");
    }

    #[test]
    fn merge_coalesces_and_drops_no_ops() {
        let (c, _cat) = setup();
        let a = run(&c, "INSERT INTO acts VALUES('a1','x',1,NULL);");
        let b = run(&c, "UPDATE acts SET title='y' WHERE id='a1';");
        let m = merge(a, b);
        assert_eq!(m.len(), 1);
        assert!(m[0].old.is_none());
        assert_eq!(m[0].new.as_ref().unwrap()["title"], "y");
        let back = run(&c, "UPDATE acts SET title='x' WHERE id='a1';");
        let again = run(&c, "UPDATE acts SET title='y' WHERE id='a1';");
        assert!(merge(back, again).is_empty());
    }

    #[test]
    fn untracked_writes_are_not_captured() {
        let (c, _cat) = setup();
        let ch = run(&c, "INSERT INTO sys_activity VALUES('x','did a thing');");
        assert!(ch.is_empty());
    }
}
