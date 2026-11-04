//! What the message is about, once the resolver's answer is completed with what the app knows
//! besides the words: the scene the person has selected, the conversation focus for a correction,
//! and the screenplay that has the production data when a scene number fits several.
//!
//! The resolver (`index::resolve`) stays a pure function of the message and the focus. These three
//! completions need the app's state, so they live in the fetch stage. Each one that guesses says so
//! with an `Assumed` fact, so the writer can tell the person which screenplay it used.

use std::sync::LazyLock;

use regex::Regex;

use super::ctx::FetchCtx;
use super::{Elsewhere, Fact, Input, P_HIGH};
use crate::modules::ai::index::{Hit, How, Kind, Resolved, SceneHit};
use crate::modules::ai::queries;
use crate::modules::ai::sections::{self, Leaf, SectionPick};
use crate::modules::ai::turn::{Domain, ReadHow};

static SCENE_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bscenes?\s+#?(\d{1,4})\b").expect("valid pattern"));

/// Words that point at the scene on screen.
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

/// True when the message names nothing the project holds.
pub fn names_nothing(r: &Resolved) -> bool {
    r.screenplays.is_empty()
        && r.scenes.is_empty()
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
pub fn effective(ctx: &FetchCtx<'_>, input: &Input<'_>) -> (Resolved, Vec<Fact>) {
    let mut r = input.resolved.clone();
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
        r.ambiguous = None;
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

    // The scene on screen, for "it", "this" and "they" when nothing else is named.
    if r.scenes.is_empty()
        && let Some(id) = input.selected_scene
        && let Some(e) = ix.entity(id)
        && let Some(sp) = e.screenplay_id.as_deref()
        && let Some(n) = e.ordinal
        && (names_nothing(&r) || ctx.m.any(POINTERS))
    {
        r.scenes.push(SceneHit {
            screenplay_id: sp.to_string(),
            ordinal: n,
            scene_id: Some(e.id.clone()),
            heading: Some(e.name.clone()),
            how: How::Focus,
        });
        if r.screenplays.is_empty()
            && let Some(info) = ix.screenplay(sp)
        {
            r.screenplays.push(Hit {
                id: info.id.clone(),
                name: info.title.clone(),
                how: How::Focus,
            });
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
        && !input.read.domains.contains(&Domain::Budget)
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

/// The draft a Screenplay pick chose is the one that screenplay is read from.
pub fn pin_drafts(ctx: &FetchCtx<'_>, picks: &[SectionPick]) {
    for p in picks {
        if let Some(id) = &p.draft_id
            && let Ok(Some(d)) = queries::draft_by_id(ctx.conn, id)
        {
            ctx.pin_draft(d);
        }
    }
}

/// Keep only the names homed in a picked leaf (spec 12.1). A name homed in another leaf is not
/// fetched; it is returned aside, for the pointer to name the leaf. A scene, a shooting day and a
/// screenplay are addresses that many leaves read by (breakdown, sides, the schedule), not content
/// of one leaf, so they stay.
pub fn restrict(mut r: Resolved, picked: &[&'static Leaf]) -> (Resolved, Vec<Elsewhere>) {
    let mut aside: Vec<Elsewhere> = Vec::new();
    let held = |k: Kind| sections::name_served(picked, k);
    let mut drop = |list: &mut Vec<Hit>, kind: Kind| {
        if held(kind) {
            return;
        }
        if let Some(home) = sections::home_of_name_kind(kind) {
            for h in list.drain(..) {
                aside.push(Elsewhere {
                    leaf: home.id,
                    kind,
                    name: h.name,
                });
            }
        }
    };
    drop(&mut r.characters, Kind::Character);
    drop(&mut r.persons, Kind::Person);
    drop(&mut r.locations, Kind::Location);
    drop(&mut r.acts, Kind::Act);
    drop(&mut r.files, Kind::File);
    drop(&mut r.budget_lines, Kind::BudgetLine);
    let mut kept = Vec::new();
    for (kind, hit) in std::mem::take(&mut r.others) {
        if held(kind) {
            kept.push((kind, hit));
        } else if let Some(home) = sections::home_of_name_kind(kind) {
            aside.push(Elsewhere {
                leaf: home.id,
                kind,
                name: hit.name,
            });
        }
    }
    r.others = kept;
    (r, aside)
}

/// Apply the instance picks: a screenplay, a scene, a shooting day or a call sheet the person chose
/// stands for what the message did not name. What the message names wins, because it is more
/// specific than the picker.
pub fn narrow(ctx: &FetchCtx<'_>, mut r: Resolved, picks: &[SectionPick]) -> Resolved {
    let ix = ctx.index;
    for p in picks {
        let screenplay = p.screenplay_id.clone().or_else(|| {
            p.draft_id
                .as_deref()
                .and_then(|id| queries::draft_by_id(ctx.conn, id).ok().flatten())
                .map(|d| d.screenplay_id)
        });
        if r.screenplays.is_empty()
            && let Some(sp) = screenplay.as_deref().and_then(|id| ix.screenplay(id))
        {
            r.screenplays.push(Hit {
                id: sp.id.clone(),
                name: sp.title.clone(),
                how: How::Focus,
            });
        }
        if r.scenes.is_empty()
            && let Some(e) = p.scene_id.as_deref().and_then(|id| ix.entity(id))
            && let Some(sp) = e.screenplay_id.as_deref()
            && let Some(n) = e.ordinal
        {
            r.scenes.push(SceneHit {
                screenplay_id: sp.to_string(),
                ordinal: n,
                scene_id: Some(e.id.clone()),
                heading: Some(e.name.clone()),
                how: How::Focus,
            });
            if r.screenplays.is_empty()
                && let Some(info) = ix.screenplay(sp)
            {
                r.screenplays.push(Hit {
                    id: info.id.clone(),
                    name: info.title.clone(),
                    how: How::Focus,
                });
            }
        }
        // A call sheet stands for its shooting day.
        let day = p.shooting_day_id.clone().or_else(|| {
            p.call_sheet_id.as_deref().and_then(|id| {
                ctx.conn
                    .query_row(
                        "SELECT shoot_day_id FROM call_sheet WHERE id = ?1 AND deleted_at IS NULL",
                        [id],
                        |row| row.get::<_, String>(0),
                    )
                    .ok()
            })
        });
        if r.days.is_empty()
            && let Some(e) = day.as_deref().and_then(|id| ix.entity(id))
        {
            r.days.push(Hit {
                id: e.id.clone(),
                name: e.name.clone(),
                how: How::Focus,
            });
        }
    }
    r
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
