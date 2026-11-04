//! What the message is about, once the resolver's answer is completed with what the app knows
//! besides the words: the conversation focus for a correction, and the screenplay that has the
//! production data when a scene number fits several. The scene the person chose is a pick and reaches
//! the fetchers of that section by itself.
//!
//! The resolver (`index::resolve`) stays a pure function of the message and the focus. These
//! completions need the app's state, so they live in the fetch stage. One that guesses says so with an
//! `Assumed` fact, so the writer can tell the person which screenplay it used.

use std::sync::LazyLock;

use regex::Regex;

use super::ctx::FetchCtx;
use super::{Fact, Input, P_HIGH};
use crate::modules::ai::index::{EntityIndex, Hit, How, Resolved, SceneHit};
use crate::modules::ai::queries;
use crate::modules::ai::turn::{ReadHow, leaf_hints};

static SCENE_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bscenes?\s+#?(\d{1,4})\b").expect("valid pattern"));

/// Words that lean on what the conversation was about.
const POINTERS: &[&str] = &[
    "it", "this", "that", "here", "they", "them", "their", "there",
];

/// True when the message names nothing but, at most, a screenplay.
pub fn no_subject(r: &Resolved) -> bool {
    r.scenes.is_empty()
        && r.persons.is_empty()
        && r.characters.is_empty()
        && r.locations.is_empty()
        && r.acts.is_empty()
        && r.days.is_empty()
        && r.files.is_empty()
        && r.budget_lines.is_empty()
        && r.others.is_empty()
}

/// The resolver's answer, completed. The facts are notes about what was assumed.
pub fn effective(
    ctx: &FetchCtx<'_>,
    input: &Input<'_>,
    resolved: &Resolved,
) -> (Resolved, Vec<Fact>) {
    let mut r = resolved.clone();
    let mut notes = Vec::new();
    let ix = input.index;

    // A scene number that fits more than one screenplay: the one with the production data.
    if r.scenes.is_empty()
        && r.ambiguous
            .as_ref()
            .is_some_and(|q| q.text.starts_with("Which screenplay"))
        && let Some(sp) = default_screenplay(ctx)
    {
        let mut any = false;
        for c in SCENE_N.captures_iter(input.message) {
            let n: u32 = c[1].parse().unwrap_or(0);
            if let Some(e) = ix.scene(&sp.id, n) {
                r.scenes.push(SceneHit {
                    screenplay_id: sp.id.clone(),
                    ordinal: n,
                    scene_id: Some(e.id.clone()),
                    heading: Some(e.name.clone()),
                    how: How::CurrentDraft,
                });
                any = true;
            }
        }
        r.screenplays = vec![Hit {
            id: sp.id.clone(),
            name: sp.title.clone(),
            how: How::CurrentDraft,
        }];
        if any {
            r.ambiguous = None;
        }
        if any || input.message.to_lowercase().contains("scene") {
            notes.push(
                Fact::new("Assumed", &sp.id)
                    .f("screenplay", &sp.title)
                    .f(
                        "reason",
                        "the message named no screenplay and the scene number fits more than one; this is the one with the production data",
                    )
                    .prio(P_HIGH),
            );
        }
    }

    // A correction, or a message with a pointer ("that", "they") or a contact question, that names
    // no subject is about what the conversation was about. A money question is not: "that is not
    // what I asked" says nothing of the last scene.
    let continuing = input.read.how == ReadHow::Repair
        || ctx.m.any(POINTERS)
        || ctx
            .m
            .any(&["contact", "phone", "email", "number", "details"]);
    if continuing
        && !asks_money(input.message, &r)
        && no_subject(&r)
        && let Some(f) = input.focus
    {
        if r.characters.is_empty() && r.persons.is_empty() {
            for id in &f.characters {
                if let Some(e) = ix.entity(id) {
                    r.characters.push(Hit {
                        id: e.id.clone(),
                        name: e.name.clone(),
                        how: How::Focus,
                    });
                }
            }
            for id in &f.persons {
                if let Some(e) = ix.entity(id) {
                    r.persons.push(Hit {
                        id: e.id.clone(),
                        name: e.name.clone(),
                        how: How::Focus,
                    });
                }
            }
        }
        let same_screenplay = r
            .screenplays
            .first()
            .is_none_or(|h| Some(h.id.as_str()) == f.screenplay.as_deref());
        if same_screenplay
            && let Some(sp) = f.screenplay.as_deref().and_then(|id| ix.screenplay(id))
        {
            if r.screenplays.is_empty() {
                r.screenplays.push(Hit {
                    id: sp.id.clone(),
                    name: sp.title.clone(),
                    how: How::Focus,
                });
            }
            if r.scenes.is_empty()
                && let Some(n) = f.scene
                && let Some(e) = ix.scene(&sp.id, n)
            {
                r.scenes.push(SceneHit {
                    screenplay_id: sp.id.clone(),
                    ordinal: n,
                    scene_id: Some(e.id.clone()),
                    heading: Some(e.name.clone()),
                    how: How::Focus,
                });
            }
        }
    }
    (r, notes)
}

/// A question about money speaks of the budget, whatever "that" or "it" said before.
fn asks_money(message: &str, r: &Resolved) -> bool {
    !r.budget_lines.is_empty()
        || leaf_hints(message)
            .iter()
            .any(|h| h.leaves.contains(&"production/budget"))
}

/// The scene numbers of a message, read in one screenplay.
pub fn scenes_in_message(ix: &EntityIndex, message: &str, screenplay_id: &str) -> Vec<SceneHit> {
    let mut out: Vec<SceneHit> = Vec::new();
    for c in SCENE_N.captures_iter(message) {
        let n: u32 = c[1].parse().unwrap_or(0);
        if out.iter().any(|s| s.ordinal == n) {
            continue;
        }
        if let Some(e) = ix.scene(screenplay_id, n) {
            out.push(SceneHit {
                screenplay_id: screenplay_id.to_string(),
                ordinal: n,
                scene_id: Some(e.id.clone()),
                heading: Some(e.name.clone()),
                how: How::Exact,
            });
        }
    }
    out
}

/// The screenplay of the active Production Source, else the longest one with a current draft.
fn default_screenplay<'a>(
    ctx: &'a FetchCtx<'_>,
) -> Option<&'a crate::modules::ai::index::ScreenplayInfo> {
    let from_source = queries::production_source_draft(ctx.conn)
        .ok()
        .flatten()
        .and_then(|(_, draft)| queries::draft_by_id(ctx.conn, &draft).ok().flatten())
        .and_then(|d| ctx.index.screenplay(&d.screenplay_id));
    from_source.or_else(|| {
        ctx.index
            .screenplays
            .iter()
            .filter(|s| s.has_current_draft)
            .max_by_key(|s| s.scene_count)
    })
}
