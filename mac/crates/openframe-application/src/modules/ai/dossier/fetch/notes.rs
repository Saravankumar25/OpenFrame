//! The Notes section (ADR-0041): the latest project notes, and the person's own private notes for
//! them only. The Files fetcher reads the notes a question names. A private note always carries
//! `private_to`, so the guard removes it for anyone else even if a read here were wrong.

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_LOW};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::TurnRead;

/// The most private notes one answer reads.
const PRIVATE_NOTES: i64 = 12;
/// The most project notes read when the question names none.
const LATEST_NOTES: i64 = 12;

/// The latest project notes, the way the Files fetcher writes them, so a note it read too is one line.
pub fn latest_notes(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("project notes", || {
        let mut st = ctx.conn.prepare(
            "SELECT id, COALESCE(title, ''), body FROM project_note WHERE deleted_at IS NULL
             ORDER BY pinned DESC, updated_at DESC LIMIT ?1",
        )?;
        let rows: Vec<(String, String, String)> = st
            .query_map([LATEST_NOTES], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows
            .into_iter()
            .map(|(id, title, body)| {
                Fact::new("Project note", id)
                    .f("title", title)
                    .f("text", clip(&body, 360))
                    .prio(P_LOW)
            })
            .collect())
    })
}

pub fn private_notes(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("private notes", || {
        let mut st = ctx.conn.prepare(
            "SELECT id, body FROM private_note
             WHERE owner_user_id = ?1 AND deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let rows: Vec<(String, String)> = st
            .query_map(rusqlite::params![ctx.actor.user_id, PRIVATE_NOTES], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows
            .into_iter()
            .map(|(id, body)| {
                Fact::new("Private note", id)
                    .f("text", clip(&body, 300))
                    .prio(P_LOW)
                    .private_to(&ctx.actor.user_id)
            })
            .collect())
    })
}
