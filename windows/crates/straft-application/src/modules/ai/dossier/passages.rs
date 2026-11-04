//! Text passages for a dossier, found inside the picked leaves only (spec 3.5, round R3).
//!
//! Structured facts (counts, cast, budget, dates, status) come from the leaves' own fetchers and are
//! authoritative. Text (scene text, notes, the story, files) is searched with the hybrid retriever,
//! pre-filtered to the search entity types of the picked leaves and to what the person may read.
//! When the project index is not ready the same filter runs over the words alone, and the dossier
//! says so (`Retrieval::degraded`; the panel already shows "Preparing project context"). Passages
//! fill leftover budget only: they never displace a fact.

use super::words::clip;
use super::{Fact, Input, P_LOW, Retrieval};
use crate::modules::ai::intelligence::{self, retriever};
use crate::modules::ai::sections::{self, Leaf};

/// The most passages one question gets, and the characters of each.
const MAX_PASSAGES: usize = 5;
const PASSAGE_CHARS: usize = 320;

/// Passages that fit `room` tokens, found for the question inside `picked`. `have` are the record
/// ids the dossier already holds, so a record is not told twice.
pub fn find(
    conn: &rusqlite::Connection,
    input: &Input<'_>,
    picked: &[&'static Leaf],
    message: &str,
    room: usize,
    have: &[String],
) -> (Vec<Fact>, Option<Retrieval>) {
    let Some(core) = input.core else {
        return (Vec::new(), None);
    };
    if room < 40 {
        return (Vec::new(), None);
    }
    let mut types: Vec<&str> = picked
        .iter()
        .flat_map(|l| l.entity_types.iter().copied())
        .collect();
    types.sort_unstable();
    types.dedup();
    if types.is_empty() {
        return (Vec::new(), None);
    }
    let ix = intelligence::service(core).current();
    let snapshot = ix.as_ref().map(|i| i.snapshot());
    let found = retriever::gather_in(
        conn,
        core,
        input.actor,
        ix.as_deref(),
        snapshot.as_ref(),
        message,
        &types,
        MAX_PASSAGES * 3,
    );
    let how = Retrieval {
        hybrid: found.hybrid,
        degraded: found.degraded,
    };
    let mut out: Vec<Fact> = Vec::new();
    let mut used = 0usize;
    for hit in found.hits {
        if out.len() >= MAX_PASSAGES || have.contains(&hit.key.id) {
            continue;
        }
        // Defence in depth: the retriever filtered by type, and the gate would drop a stray kind, but
        // a passage whose type has no picked home never reaches the writer.
        let Some(home) = sections::home_of_entity_type(hit.key.kind) else {
            continue;
        };
        if !picked.iter().any(|l| l.id == home.id) {
            continue;
        }
        let Some((etype, title, context, body)) = read_doc(conn, input, &hit.key.id) else {
            continue;
        };
        let text = hit.chunk.as_deref().unwrap_or(&body);
        let mut f = Fact::new("Found", hit.key.id.clone())
            .f("type", etype)
            .f(
                "title",
                if title.is_empty() {
                    hit.title.clone()
                } else {
                    title
                },
            )
            .f("where", context)
            .f("text", clip(text, PASSAGE_CHARS))
            .prio(P_LOW);
        f.home = Some(home.id);
        if etype_is_private(&f) {
            f = f.private_to(&input.actor.user_id);
        }
        if used + f.tokens() > room {
            break;
        }
        used += f.tokens();
        out.push(f);
    }
    (out, Some(how))
}

/// A person's private note: only the person it belongs to may be given it.
fn etype_is_private(f: &Fact) -> bool {
    f.field("type") == Some("private_note")
}

/// The search document of a record, only when the person may read it.
fn read_doc(
    conn: &rusqlite::Connection,
    input: &Input<'_>,
    id: &str,
) -> Option<(String, String, String, String)> {
    conn.query_row(
        "SELECT entity_type, title, context, body FROM search_doc
         WHERE entity_id = ?1 AND (owner_user_id IS NULL OR owner_user_id = ?2) LIMIT 1",
        rusqlite::params![id, input.actor.user_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .ok()
}
