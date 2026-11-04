//! The Story tabs the Story fetcher does not read (ADR-0041): the Story page, the Timeline and the
//! Episodes. The Story page stays out of search and out of every tool: its text reaches the
//! assistant only when the person picks the Story Page section for that question.

use rusqlite::OptionalExtension;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::story::episodes::load_series;
use crate::modules::story::timeline::{self, StoryTimelineArgs};

/// How much of the Story page one answer may read.
const PAGE_CHARS: usize = 1_800;
/// The most paragraphs of the Story page given as separate lines.
const PAGE_PARAGRAPHS: usize = 6;

pub fn page(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("story page", || {
        let body: Option<String> = ctx
            .conn
            .query_row("SELECT body FROM story_page LIMIT 1", [], |r| r.get(0))
            .optional()?;
        let body = body.unwrap_or_default();
        if body.trim().is_empty() {
            return Ok(vec![Fact::absent("any text", "the Story page")]);
        }
        let words = ctx.m.keywords();
        let paragraphs: Vec<&str> = body
            .split('\n')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        // Paragraphs that share a word with the question come first, then the page from the top.
        let mut order: Vec<usize> = (0..paragraphs.len()).collect();
        order.sort_by_key(|i| {
            let lower = paragraphs[*i].to_lowercase();
            (!words.iter().any(|w| lower.contains(w.as_str())), *i)
        });
        let mut room = PAGE_CHARS;
        let mut out = Vec::new();
        for i in order.into_iter().take(PAGE_PARAGRAPHS) {
            if room < 40 {
                break;
            }
            let text = clip(paragraphs[i], room);
            room = room.saturating_sub(text.chars().count());
            out.push(
                Fact::new("Story page", "")
                    .f("paragraph", (i + 1).to_string())
                    .f("text", text)
                    .prio(P_MID),
            );
        }
        Ok(out)
    })
}

pub fn timeline(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("story timeline", || {
        let args = StoryTimelineArgs {
            episode_id: None,
            screenplay_id: r.screenplay_id().map(str::to_string),
        };
        let t = timeline::load(ctx.conn, &args)?;
        if t.groups.is_empty() {
            return Ok(vec![Fact::absent("any scenes", "the Story Timeline")]);
        }
        let mut out = Vec::new();
        for g in &t.groups {
            let scenes: Vec<String> = g
                .scenes
                .iter()
                .map(
                    |s| match s.time_note.as_deref().filter(|n| !n.trim().is_empty()) {
                        Some(note) => format!("{} ({note})", s.number),
                        None => s.number.to_string(),
                    },
                )
                .collect();
            let named = g.scenes.iter().any(|s| {
                r.scenes
                    .iter()
                    .any(|h| h.scene_id.as_deref() == Some(s.scene_id.as_str()))
            });
            out.push(
                Fact::new("Story day", "")
                    .f("story day", g.story_day.as_deref().unwrap_or("Unassigned"))
                    .f("scenes", clip(&scenes.join(", "), 400))
                    .prio(if named { P_HIGH } else { P_MID }),
            );
        }
        if read.answer_type == AnswerType::Count {
            let days = t.groups.iter().filter(|g| g.story_day.is_some()).count();
            out.insert(
                0,
                Fact::new("Count", "")
                    .f("what", "story days on the timeline")
                    .f("value", days.to_string())
                    .require(days.to_string())
                    .prio(P_HIGH),
            );
        }
        Ok(out)
    })
}

pub fn episodes(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("episodes", || {
        let series = load_series(ctx.conn)?;
        if !series.is_episodic {
            return Ok(vec![Fact::missing("This project has no episodes.")]);
        }
        let mut out = Vec::new();
        for season in &series.seasons {
            for e in &season.episodes {
                out.push(
                    Fact::new("Episode", &e.id)
                        .f("season", &season.title)
                        .f("number", e.number.to_string())
                        .f("title", &e.title)
                        .f("status", e.status.as_deref().unwrap_or_default())
                        .f(
                            "summary",
                            clip(e.summary.as_deref().unwrap_or_default(), 240),
                        )
                        .f("cards", e.card_count.to_string())
                        .prio(P_MID),
                );
            }
        }
        for e in &series.unseasoned {
            out.push(
                Fact::new("Episode", &e.id)
                    .f("number", e.number.to_string())
                    .f("title", &e.title)
                    .f("status", e.status.as_deref().unwrap_or_default())
                    .f(
                        "summary",
                        clip(e.summary.as_deref().unwrap_or_default(), 240),
                    )
                    .f("cards", e.card_count.to_string())
                    .prio(P_MID),
            );
        }
        if out.is_empty() {
            return Ok(vec![Fact::absent("any episodes", "Episodes")]);
        }
        if read.answer_type == AnswerType::Count {
            let n = out.len();
            out.insert(
                0,
                Fact::new("Count", "")
                    .f("what", "episodes")
                    .f("value", n.to_string())
                    .require(n.to_string())
                    .prio(P_HIGH),
            );
        }
        Ok(out)
    })
}
