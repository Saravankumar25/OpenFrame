//! Product help: the sections of the product guide (`modules/ai/knowledge.md`) that match the
//! words of the question. The guide is the only source of how the app works.

use crate::modules::ai::dossier::ctx::FetchCtx;
use crate::modules::ai::dossier::words::{Msg, clip};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::knowledge;
use crate::modules::ai::turn::{Group, TurnRead};

/// Sections given to a help question, and to a question of another kind that asks about the app.
const SECTIONS_HELP: usize = 2;
const SECTIONS_OTHER: usize = 1;
/// Longest section text given.
const SECTION_CHARS: usize = 1_400;

/// What the guide promises, and the words of a question it answers. An answer about where a project
/// goes says what the guide says, in the guide's words, or code adds them: a model must not guess this.
const PROMISES: &[(&[&str], &str)] = &[
    (
        &[
            "sent", "send", "sends", "upload", "uploaded", "uploads", "cloud", "leave", "leaves",
            "anywhere", "share", "shared",
        ],
        "never sent",
    ),
    (
        &[
            "internet",
            "offline",
            "online",
            "connection",
            "wifi",
            "network",
        ],
        "without the internet",
    ),
];

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let help_section = ctx.picked.is_some_and(|p| p.leaf.id.starts_with("help/"));
    let n = if read.group == Group::Help || help_section {
        SECTIONS_HELP
    } else {
        SECTIONS_OTHER
    };
    let found = knowledge::relevant(&ctx.message, n);
    if found.is_empty() && read.group == Group::Help {
        return vec![Fact::absent("a section on that", "the product guide")];
    }
    let typed = Msg::new(&ctx.message);
    let mut promised: Vec<&str> = PROMISES
        .iter()
        .filter(|(words, _)| typed.any(words))
        .map(|(_, phrase)| *phrase)
        .collect();
    let mut out = Vec::with_capacity(found.len());
    for (i, (title, body)) in found.into_iter().enumerate() {
        let mut f = Fact::new(
            "Guide",
            format!("guide:{}", title.to_lowercase().replace(' ', "-")),
        )
        .f("section", title)
        .f("text", clip(body, SECTION_CHARS))
        .prio(if i == 0 { P_HIGH } else { P_MID });
        // Each promise this section states is demanded once.
        let lower = body.to_lowercase();
        let (stated, rest): (Vec<&str>, Vec<&str>) =
            promised.iter().partition(|p| lower.contains(**p));
        promised = rest;
        for phrase in stated {
            f = f.require(phrase);
        }
        out.push(f);
    }
    out
}
