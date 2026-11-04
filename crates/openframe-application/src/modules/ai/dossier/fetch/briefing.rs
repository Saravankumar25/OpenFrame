//! The briefing of a long answer (ADR-0041 decision 6; design section 3.7): what the writer is given
//! to describe a character or a place, to tell how two people stand or to explain the story. Every line
//! carries its scene number, so an answer can say where a thing happens and the checker can hold it to
//! that. Scene lines come from the screenplay section; character cards and relationship records stay
//! with the Story fetcher, and a brief of a named scene is the screenplay fetcher's own reading.

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::{Fact, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, TurnRead};

/// Scenes excerpted for one named character or place: the first ones and the last.
const SCENES_PER_NAME: usize = 4;
/// Characters of one scene's excerpt.
const EXCERPT_CHARS: usize = 500;
/// What all the one-line scene summaries of a story explanation may take, in characters.
const STORY_LINES_CHARS: usize = 6_000;

pub fn scenes(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("briefing", || fetch(ctx, read, r))
}

fn excerpt(ctx: &FetchCtx<'_>, s: &SceneRef, about: &str) -> AppResult<Option<Fact>> {
    let text = queries::scene_text(ctx.conn, &s.id, EXCERPT_CHARS)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(
        Fact::new("Scene text", &s.id)
            .f("scene", s.number.to_string())
            .f("heading", &s.heading)
            .f("about", about)
            .f("text", text)
            .prio(P_MID - 10),
    ))
}

/// The first scenes of a list and its last, without repeats.
fn spread(numbers: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = numbers.iter().take(SCENES_PER_NAME).copied().collect();
    if let Some(last) = numbers.last()
        && !out.contains(last)
    {
        out.push(*last);
    }
    out
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let t = read.answer_type;
    if !t.conversational() {
        return Ok(Vec::new());
    }
    // The person chose one scene, or the message names scenes: the screenplay fetcher reads them.
    if ctx.picked.is_some_and(|p| p.narrowed()) || !ctx.focus_scenes(r).is_empty() {
        return Ok(Vec::new());
    }
    let Some(sp) = ctx.screenplay(r) else {
        return Ok(Vec::new());
    };
    let Some(draft) = ctx.draft(&sp.id) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let presence = ctx.presence(&draft.id);
    let names: Vec<String> = r
        .characters
        .iter()
        .map(|h| {
            ctx.index
                .entity(&h.id)
                .map_or(h.name.clone(), |e| e.name.clone())
        })
        .collect();
    // How two people stand: the scenes they share first.
    if t == AnswerType::Relation && names.len() >= 2 {
        let empty = Default::default();
        let first = presence.by_name.get(&names[0]).unwrap_or(&empty);
        let second = presence.by_name.get(&names[1]).unwrap_or(&empty);
        let both: Vec<u32> = first.intersection(second).copied().collect();
        for n in spread(&both) {
            if let Some(s) = ctx.scene_of(&sp.id, n) {
                out.extend(excerpt(ctx, &s, &names.join(" and "))?);
            }
        }
        return Ok(out);
    }
    for name in names.iter().take(2) {
        let numbers: Vec<u32> = presence
            .by_name
            .get(name)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        for n in spread(&numbers) {
            if let Some(s) = ctx.scene_of(&sp.id, n) {
                out.extend(excerpt(ctx, &s, name)?);
            }
        }
    }
    let all = queries::scenes(ctx.conn, &draft.id)?;
    for place in r.locations.iter().take(2) {
        let wanted = normalize(&place.name);
        let numbers: Vec<u32> = all
            .iter()
            .filter(|s| !s.omitted && normalize(&s.heading).contains(&wanted))
            .map(|s| s.number as u32)
            .collect();
        for n in spread(&numbers) {
            if let Some(s) = ctx.scene_of(&sp.id, n) {
                out.extend(excerpt(ctx, &s, &place.name)?);
            }
        }
    }
    // The story as a whole: one line per scene, so the answer can follow it in order.
    if t == AnswerType::Story && r.characters.is_empty() {
        let live: Vec<_> = all.iter().filter(|s| !s.omitted).collect();
        let each = (STORY_LINES_CHARS / live.len().max(1)).clamp(60, 200);
        for s in live {
            let synopsis: String = ctx.conn.query_row(
                "SELECT COALESCE(synopsis, '') FROM screenplay_scene WHERE id = ?1",
                [&s.id],
                |r| r.get(0),
            )?;
            let line = if synopsis.trim().is_empty() {
                queries::scene_text(ctx.conn, &s.id, each)?
            } else {
                queries::truncate_chars(&synopsis, each)
            };
            out.push(
                Fact::new("Scene", &s.id)
                    .f("scene", s.number.to_string())
                    .f("heading", &s.heading)
                    .f("summary", line)
                    .prio(P_LOW + 10),
            );
        }
    }
    Ok(out)
}
