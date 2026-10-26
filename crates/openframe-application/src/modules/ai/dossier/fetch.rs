//! The fetchers, one file per domain, and the wider round used when a plan found nothing.

pub mod breakdown;
pub mod budget;
pub mod files;
pub mod people;
pub mod product;
pub mod production;
pub mod schedule;
pub mod screenplay;
pub mod settings;
pub mod story;
pub mod visual;

use super::ctx::{FetchCtx, safe};
use super::words::clip;
use super::{Fact, P_LOW};
use crate::modules::ai::toolbox::reads::lexical_hits;

/// How many passages the wider round returns.
pub const WIDE: usize = 6;

/// Full-text search of the project for the words of the message. Private notes of other people are
/// not in reach: the search only returns what the person may read.
pub fn passages(ctx: &FetchCtx<'_>, limit: usize) -> Vec<Fact> {
    let words = ctx.m.keywords();
    if words.is_empty() {
        return Vec::new();
    }
    safe("widen", || {
        let hits = lexical_hits(ctx.conn, ctx.actor, &words.join(" "), limit)?;
        Ok(hits
            .into_iter()
            .map(|(etype, title, context, body, _)| {
                Fact::new("Found", "")
                    .f("type", etype)
                    .f("title", title)
                    .f("where", context)
                    .f("text", clip(&body, 300))
                    .prio(P_LOW)
            })
            .collect())
    })
}
