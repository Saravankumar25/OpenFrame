//! Recently Deleted keeps items for 30 days (owner decision 2026-10-07, ADR-0012 amendment;
//! build plan area 9 box "Recently Deleted 30-day purge").
//!
//! Every recoverable delete writes a `deleted_item` row with the time of the delete. An item is
//! removed for good once 30 days have passed since then. The removal runs, with no network, when
//! a project opens (the project's own Recently Deleted, which also holds the project's Idea Vault
//! items) and when the Global Idea Vault opens for the first time in a session, which the desktop
//! shell does at app start. The Owner can still delete an item permanently before that, as before
//! (`trash.purge`, `vault.purge`).
//!
//! Rules that keep the removal safe:
//!
//! - **One item per transaction.** Each removal is its own `Store::mutate`, so a crash, a full
//!   drive or a kill in the middle leaves every item either fully there or fully gone. The next
//!   open carries on where the last one stopped.
//! - **Live items are never touched.** Inside the transaction the row is read again; it is removed
//!   only when its registered table is the one the delete record names and the object there still
//!   carries `deleted_at`. A restored item, a record that points at a live row, or a table without
//!   `deleted_at` is skipped and left as it is.
//! - **The clock cannot remove an item early by going back.** A delete time later than now (the
//!   clock was turned back after the delete, or the item came from a computer whose clock ran
//!   ahead) counts as a delete made just now: the screens show 30 days and nothing is removed.
//!   A clock turned forward removes what is more than 30 days old by that clock; OpenFrame has no
//!   other time source without the network, and the items were already deleted by the user.
//! - **Bounded work.** At most [`MAX_PER_RUN`] items are removed per open, so opening a project
//!   never waits long; anything left is removed at the next open.

use openframe_domain::{Actor, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};

use crate::store::{MutationMeta, Store, load_deleted, purge_deleted};

/// How long Recently Deleted keeps an item.
pub const RETENTION_DAYS: u32 = 30;
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const RETENTION_MS: i64 = RETENTION_DAYS as i64 * DAY_MS;
/// Upper bound on removals per run.
pub const MAX_PER_RUN: usize = 500;

/// Whole days an item deleted at `deleted_at` still stays in Recently Deleted at `now`: 1 to 30
/// while it stays, 0 once it is due for removal. A delete time in the future counts as now.
pub fn days_left(deleted_at: i64, now: i64) -> u32 {
    if now <= deleted_at {
        return RETENTION_DAYS;
    }
    let remaining = deleted_at.saturating_add(RETENTION_MS).saturating_sub(now);
    if remaining <= 0 {
        return 0;
    }
    // Round up: 29 days and one hour left reads "30 days left", one hour left reads "1 day left".
    u32::try_from((remaining + DAY_MS - 1) / DAY_MS)
        .unwrap_or(RETENTION_DAYS)
        .clamp(1, RETENTION_DAYS)
}

/// True when the item is due for removal at `now`: exactly 30 days after the delete or later.
pub fn is_expired(deleted_at: i64, now: i64) -> bool {
    days_left(deleted_at, now) == 0
}

/// What one run did, for the log and the tests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PurgeReport {
    /// Items removed for good.
    pub removed: usize,
    /// Items due for removal that were left in place: a live object, an unknown type, a record
    /// another member owns (a private note), or an error. Never counted as removed.
    pub skipped: usize,
    /// Due items not looked at because the run reached its limit.
    pub deferred: usize,
}

/// Remove every item that has been in `store`'s Recently Deleted for 30 days or more at `now`,
/// oldest first, at most `limit` of them. Each item is removed in its own transaction. A failing
/// item is logged and skipped; the run never fails because of one item. Returns an error only
/// when the list of due items cannot be read at all.
pub fn purge_expired(
    store: &Store,
    actor: &Actor,
    now: i64,
    limit: usize,
) -> AppResult<PurgeReport> {
    let cutoff = now.saturating_sub(RETENTION_MS);
    let due: Vec<String> = store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id FROM deleted_item WHERE deleted_at <= ?1 ORDER BY deleted_at, id",
        )?;
        let ids = stmt
            .query_map([cutoff], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    })?;
    let mut report = PurgeReport {
        deferred: due.len().saturating_sub(limit),
        ..PurgeReport::default()
    };
    for id in due.into_iter().take(limit) {
        let meta = MutationMeta::new(
            "trash.expire",
            format!("Removed an item deleted more than {RETENTION_DAYS} days ago"),
            Capability::PermanentDelete,
        )
        .not_undoable()
        .quiet();
        let outcome = store.mutate(actor, meta, |tx| {
            // Read again inside the transaction: the item may have been restored or removed since.
            let row = match load_deleted(tx.conn(), &id) {
                Ok(row) => row,
                Err(e) if e.is("not_found") => return Ok(false),
                Err(e) => return Err(e),
            };
            if !is_expired(row.deleted_at, now) {
                return Ok(false);
            }
            let Some(handler) = tx
                .registry()
                .trash_for(&row.object_type)
                .filter(|h| h.table == row.table_name)
            else {
                return Ok(false);
            };
            if !object_still_deleted(tx.conn(), handler.table, &row.object_id)? {
                return Ok(false);
            }
            purge_deleted(tx, &id)?;
            Ok(true)
        });
        match outcome {
            Ok(true) => report.removed += 1,
            Ok(false) => report.skipped += 1,
            Err(e) => {
                report.skipped += 1;
                tracing::warn!(
                    code = e.code_str(),
                    "an item due for removal from Recently Deleted was left in place"
                );
            }
        }
    }
    Ok(report)
}

/// [`purge_expired`] at the current time, never failing: problems are logged. Used when a project
/// or the Global Idea Vault opens, where a failed clean-up must never stop the user.
pub fn purge_expired_now(store: &Store, actor: &Actor, which: &'static str) {
    match purge_expired(store, actor, now_ms(), MAX_PER_RUN) {
        Ok(r) if r.removed + r.skipped + r.deferred > 0 => tracing::info!(
            store = which,
            removed = r.removed,
            skipped = r.skipped,
            deferred = r.deferred,
            "removed items deleted more than 30 days ago"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(
            store = which,
            code = e.code_str(),
            "could not check Recently Deleted for items older than 30 days"
        ),
    }
}

/// True when the object still exists in `table` with `deleted_at` set. A missing row counts as
/// deleted (only its delete record is left to remove). A live row, or a table without a
/// `deleted_at` column, is never removed.
fn object_still_deleted(c: &Connection, table: &str, object_id: &str) -> AppResult<bool> {
    let has_column: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name='deleted_at')",
        [table],
        |r| r.get(0),
    )?;
    if !has_column {
        return Ok(false);
    }
    let deleted_at: Option<Option<i64>> = c
        .query_row(
            &format!(
                "SELECT deleted_at FROM \"{}\" WHERE id=?1",
                table.replace('"', "\"\"")
            ),
            params![object_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match deleted_at {
        None => true,
        Some(at) => at.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_791_244_800_000;

    #[test]
    fn days_left_counts_down_and_reaches_zero_exactly_after_30_days() {
        assert_eq!(days_left(T0, T0), 30);
        assert_eq!(days_left(T0, T0 + 1), 30);
        assert_eq!(days_left(T0, T0 + DAY_MS), 29);
        assert_eq!(days_left(T0, T0 + DAY_MS + 1), 29);
        assert_eq!(days_left(T0, T0 + 29 * DAY_MS), 1);
        assert_eq!(
            days_left(T0, T0 + 30 * DAY_MS - 1),
            1,
            "the last millisecond"
        );
        assert_eq!(days_left(T0, T0 + 30 * DAY_MS), 0, "the boundary");
        assert_eq!(days_left(T0, T0 + 400 * DAY_MS), 0);
        assert!(!is_expired(T0, T0 + 30 * DAY_MS - 1));
        assert!(is_expired(T0, T0 + 30 * DAY_MS));
    }

    #[test]
    fn a_delete_time_in_the_future_counts_as_now() {
        // The clock was turned back after the delete.
        assert_eq!(days_left(T0, T0 - 5 * DAY_MS), 30);
        assert!(!is_expired(T0, T0 - 400 * DAY_MS));
        assert!(!is_expired(i64::MAX, 0));
        assert_eq!(days_left(i64::MIN, i64::MAX), 0);
    }
}
