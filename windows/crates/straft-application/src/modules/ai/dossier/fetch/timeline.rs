//! Story > Timeline and Story > Episodes. The Timeline groups the scenes of a screenplay by Story
//! Day (the day in the story, not the shooting day) and carries a time note on each scene; it is
//! read through the same function the Timeline screen calls. Episodes read the `episode` table.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::story::timeline::{StoryTimelineArgs, load};

/// The most screenplays one Timeline question reads.
const SCREENPLAYS: usize = 4;

/// Story > Timeline: the Story Days with their scenes and the time notes.
pub fn timeline_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("story timeline", || timeline(ctx, read, r))
}

fn timeline(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let ids: Vec<String> = if r.screenplays.is_empty() {
        ctx.index
            .screenplays
            .iter()
            .take(SCREENPLAYS)
            .map(|s| s.id.clone())
            .collect()
    } else {
        r.screenplays.iter().map(|h| h.id.clone()).collect()
    };
    let mut out = Vec::new();
    for id in ids {
        let dto = load(
            ctx.conn,
            &StoryTimelineArgs {
                episode_id: None,
                screenplay_id: Some(id),
            },
        )?;
        let title = dto
            .screenplays
            .iter()
            .find(|s| Some(&s.id) == dto.screenplay_id.as_ref())
            .map(|s| s.title.clone())
            .unwrap_or_default();
        let days: Vec<_> = dto
            .groups
            .iter()
            .filter(|g| g.story_day.is_some())
            .collect();
        if days.is_empty()
            && !dto
                .groups
                .iter()
                .any(|g| g.scenes.iter().any(|s| s.time_note.is_some()))
        {
            out.push(Fact::absent(
                format!("any Story Days in {title}"),
                "the Story Timeline",
            ));
            continue;
        }
        let mut count = Fact::new("Count", "")
            .f("what", format!("Story Days in {title}"))
            .f("value", days.len().to_string())
            .prio(P_HIGH);
        if read.answer_type == AnswerType::Count {
            count = count.require(days.len().to_string());
        }
        out.push(count);
        for g in &dto.groups {
            let label = g.story_day.clone().unwrap_or_else(|| "Unassigned".into());
            let notes: Vec<String> = g
                .scenes
                .iter()
                .filter_map(|s| {
                    s.time_note
                        .as_ref()
                        .map(|n| format!("scene {}: {}", s.number, clip(n, 80)))
                })
                .collect();
            let mut f = Fact::new(
                "Story day",
                g.scenes
                    .first()
                    .map(|s| s.scene_id.clone())
                    .unwrap_or_default(),
            )
            .f("screenplay", &title)
            .f("story day", &label)
            .f("scenes", numbers(g.scenes.iter().map(|s| s.number as u32)))
            .f("time notes", clip(&notes.join("; "), 400))
            .prio(if g.story_day.is_some() { P_HIGH } else { P_MID });
            if g.story_day.is_some() && days.len() <= 12 {
                f = f.require(&label);
            }
            out.push(f);
        }
    }
    Ok(out)
}

/// Story > Episodes: the episodes of a series with their summaries.
pub fn episodes_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("story episodes", || episodes(ctx, read))
}

fn episodes(ctx: &FetchCtx<'_>, read: &TurnRead) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, title, COALESCE(summary, ''), COALESCE(status, '') FROM episode
         WHERE deleted_at IS NULL ORDER BY position, id LIMIT 200",
    )?;
    let rows: Vec<(String, String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent("any episodes", "Story Episodes")]);
    }
    let mut count = Fact::new("Count", "")
        .f("what", "episodes")
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    if read.answer_type == AnswerType::Count {
        count = count.require(rows.len().to_string());
    }
    let mut out = vec![count];
    for (i, (id, title, summary, status)) in rows.iter().enumerate() {
        let mut f = Fact::new("Episode", id)
            .f("number", (i + 1).to_string())
            .f("title", title)
            .f("status", status)
            .f("summary", clip(summary, 240))
            .prio(P_MID);
        if rows.len() <= 14 {
            f = f.require(title);
        }
        out.push(f);
    }
    Ok(out)
}
