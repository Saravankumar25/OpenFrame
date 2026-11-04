//! Idea Vault: the items of this project's vault and of the Global Idea Vault of this computer.
//! Reads `vault_item` in either store.

use rusqlite::Connection;
use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::{Kind, Resolved};
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};
use crate::modules::idea_vault::read_vault;
use crate::util::StoreSel;

/// Idea Vault > This Project.
pub fn project_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    let named: Vec<String> = r
        .others
        .iter()
        .filter(|(k, _)| *k == Kind::VaultItem)
        .map(|(_, h)| h.id.clone())
        .collect();
    safe("idea vault", || {
        vault(ctx.conn, read, &named, "Idea Vault item", "the Idea Vault")
    })
}

/// Idea Vault > Global: the person's own vault on this computer, shared by all their projects.
pub fn global_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let Some(core) = ctx.core else {
        return Vec::new();
    };
    safe("global idea vault", || {
        read_vault(core, ctx.actor, StoreSel::Global, |c, _| {
            vault(c, read, &[], "Global vault item", "the Global Idea Vault")
        })
    })
}

fn vault(
    conn: &Connection,
    read: &TurnRead,
    named: &[String],
    kind: &'static str,
    place: &str,
) -> AppResult<Vec<Fact>> {
    let mut st = conn.prepare(
        "SELECT id, COALESCE(title, ''), item_type, COALESCE(body, ''), COALESCE(caption, ''),
                COALESCE(url, ''), COALESCE(source_text, '') FROM vault_item
         WHERE deleted_at IS NULL ORDER BY created_at, id LIMIT 200",
    )?;
    let rows: Vec<(String, String, String, String, String, String, String)> = st
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent("any items", place)]);
    }
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let list = read.answer_type == AnswerType::List;
    let mut out = Vec::new();
    let mut count = Fact::new("Count", "")
        .f("what", format!("items in {place}"))
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    if read.answer_type == AnswerType::Count {
        count = count.require(rows.len().to_string());
    }
    out.push(count);
    for (id, title, item_type, body, caption, url, source) in &rows {
        let is_named = named.contains(id);
        let mut f = Fact::new(kind, id)
            .f("title", title)
            .f("type", item_type)
            .prio(if is_named { P_HIGH } else { P_MID });
        if is_named || narrative || rows.len() <= 8 {
            f = f
                .f("text", clip(&format!("{body} {caption}"), 360))
                .f("link", clip(url, 200))
                .f("source", clip(source, 160));
        }
        if list && rows.len() <= 14 {
            f = f.require(title);
        }
        out.push(f);
    }
    Ok(out)
}
