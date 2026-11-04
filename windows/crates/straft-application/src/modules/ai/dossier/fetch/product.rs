//! Product help: the sections of the product guide (`modules/ai/knowledge.md`) that match the
//! words of the question. The guide is the only source of how the app works.

use crate::modules::ai::dossier::ctx::FetchCtx;
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::knowledge;
use crate::modules::ai::turn::TurnRead;

/// Sections given to a question asked under the Product Guide.
const SECTIONS: usize = 3;
/// Longest section text given.
const SECTION_CHARS: usize = 1_400;

/// App Help & Settings > Product Guide: the sections of the guide that match the question.
pub fn guide_leaf(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let found = knowledge::relevant(&ctx.message, SECTIONS);
    if found.is_empty() {
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
