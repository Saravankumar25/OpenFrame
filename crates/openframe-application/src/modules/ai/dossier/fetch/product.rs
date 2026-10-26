//! Product help: the sections of the product guide (`modules/ai/knowledge.md`) that match the
//! words of the question. The guide is the only source of how the app works.

use crate::modules::ai::dossier::ctx::FetchCtx;
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::knowledge;
use crate::modules::ai::turn::{Group, TurnRead};

/// Sections given to a help question, and to a question of another kind that asks about the app.
const SECTIONS_HELP: usize = 2;
const SECTIONS_OTHER: usize = 1;
/// Longest section text given.
const SECTION_CHARS: usize = 1_400;

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let n = if read.group == Group::Help {
        SECTIONS_HELP
    } else {
        SECTIONS_OTHER
    };
    let found = knowledge::relevant(&ctx.message, n);
    if found.is_empty() && read.group == Group::Help {
        return vec![Fact::absent("a section on that", "the product guide")];
    }
    found
        .into_iter()
        .enumerate()
        .map(|(i, (title, body))| {
            Fact::new(
                "Guide",
                format!("guide:{}", title.to_lowercase().replace(' ', "-")),
            )
            .f("section", title)
            .f("text", clip(body, SECTION_CHARS))
            .prio(if i == 0 { P_HIGH } else { P_MID })
        })
        .collect()
}
