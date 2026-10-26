//! Screenplay: the scene a message names (heading, who speaks, summary, text), the neighbours of a
//! scene to brainstorm over, the scenes a character is in, and the size of a screenplay.
//! Reads `screenplay`, `screenplay_scene` and `screenplay_element` through `queries`.

use std::sync::LazyLock;

use openframe_domain::AppResult;
use regex::Regex;

use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, ScreenplayInfo};
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};

/// Characters of scene text given for a paragraph answer and for a brainstorm.
const TEXT_EXPLAIN: usize = 1_400;
const TEXT_BRAINSTORM: usize = 1_200;
const TEXT_COMPARE: usize = 700;

static SCENE_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bscenes?\s+#?(\d{1,4})\b").expect("valid pattern"));
static BETWEEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbetween\s+(\d{1,4})\s+and\s+(\d{1,4})\b").expect("valid pattern")
});
static YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(1[5-9]\d\d|20\d\d)\b").expect("valid pattern"));

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("screenplay", || fetch(ctx, read, r))
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let mut out: Vec<Fact> = Vec::new();
    let scenes = ctx.focus_scenes(r);

    for s in scenes.iter().take(4) {
        out.extend(scene_facts(ctx, read, s, scenes.len() > 1)?);
    }
    if read.group == Group::Brainstorm
        && let Some(s) = scenes.first()
    {
        for n in [
            s.number.checked_sub(1).filter(|n| *n > 0),
            Some(s.number + 1),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(nb) = ctx.scene_of(&s.screenplay_id, n) {
                let (synopsis, _) = scene_notes(ctx, &nb.id)?;
                out.push(
                    Fact::new("Neighbour scene", &nb.id)
                        .f("scene", nb.number.to_string())
                        .f("heading", &nb.heading)
                        .f("summary", clip(&synopsis, 220))
                        .prio(P_LOW + 10),
                );
            }
        }
    }

    // The screenplays the message is about.
    let sps: Vec<&ScreenplayInfo> = if r.screenplays.is_empty() {
        ctx.screenplay(r).into_iter().collect()
    } else {
        r.screenplays
            .iter()
            .filter_map(|h| ctx.index.screenplay(&h.id))
            .collect()
    };

    // A message that opens or names a place: how long its screenplay is.
    if read.group == Group::Navigate {
        for sp in &sps {
            out.push(
                Fact::new("Screenplay", &sp.id)
                    .f("title", &sp.title)
                    .f("scenes", sp.scene_count.to_string())
                    .f("format", &sp.format)
                    .prio(P_MID),
            );
        }
    }

    // The people who speak in a screenplay, when no one is named.
    if scenes.is_empty()
        && r.characters.is_empty()
        && m.stem("character")
        && m.any(&["speaking", "speak", "speaks", "dialogue", "lines"])
    {
        for sp in &sps {
            let Some(d) = ctx.draft(&sp.id) else { continue };
            let names: Vec<String> = ctx.presence(&d.id).by_name.keys().cloned().collect();
            if names.is_empty() {
                continue;
            }
            let mut f = Fact::new("Speaking characters", &sp.id)
                .f("screenplay", &sp.title)
                .f("count", names.len().to_string())
                .f("names", clip(&names.join(", "), 600))
                .prio(P_HIGH);
            if names.len() <= 12 {
                for n in &names {
                    f = f.require(n);
                }
            }
            out.push(f);
        }
    }

    // How big a screenplay is.
    if scenes.is_empty()
        && r.characters.is_empty()
        && m.stem("scene")
        && (t == AnswerType::Count || m.has("how many"))
    {
        let list: Vec<&ScreenplayInfo> = if r.screenplays.is_empty() {
            ctx.index.screenplays.iter().collect()
        } else {
            sps.clone()
        };
        for sp in list {
            out.push(
                Fact::new("Count", &sp.id)
                    .f("what", format!("scenes in {}", sp.title))
                    .f("value", sp.scene_count.to_string())
                    .require(sp.scene_count.to_string())
                    .prio(P_HIGH),
            );
        }
    }

    // A scene number past the end of the screenplay.
    for cap in SCENE_N.captures_iter(&ctx.message) {
        let n: u32 = cap[1].parse().unwrap_or(0);
        for sp in &sps {
            if (n == 0 || n > sp.scene_count) && !scenes.iter().any(|s| s.number == n) {
                out.push(Fact::missing(crate::modules::ai::index::scene_not_found(
                    n,
                    &sp.title,
                    sp.scene_count,
                )));
            }
        }
    }

    // Where a character speaks.
    let asks_scenes = m.stem("scene")
        || m.any(&[
            "appear",
            "appears",
            "appearing",
            "both",
            "together",
            "there",
            "present",
        ]);
    if !r.characters.is_empty() && asks_scenes && (scenes.is_empty() || t == AnswerType::List) {
        for sp in &sps {
            if let Some(d) = ctx.draft(&sp.id) {
                out.extend(appearances(ctx, read, r, sp, &d.id));
            }
        }
    }

    // Scenes by a word of their heading: "scenes set in 1999".
    if scenes.is_empty()
        && matches!(t, AnswerType::List | AnswerType::Count)
        && let Some(y) = YEAR.captures(&ctx.message)
    {
        for sp in &sps {
            let Some(d) = ctx.draft(&sp.id) else { continue };
            let hits: Vec<u32> = queries::scenes(ctx.conn, &d.id)?
                .into_iter()
                .filter(|s| s.heading.contains(&y[1]))
                .map(|s| s.number as u32)
                .collect();
            if !hits.is_empty() {
                let mut f = Fact::new("Scenes", &sp.id)
                    .f("heading has", &y[1])
                    .f("screenplay", &sp.title)
                    .f("scenes", numbers(hits.iter().copied()))
                    .f("count", hits.len().to_string())
                    .prio(P_HIGH);
                for n in &hits {
                    f = f.require(n.to_string());
                }
                out.push(f);
            }
        }
    }
    // Scenes that mention a word, in every screenplay the message does not narrow: "which scenes
    // mention the cassette". Each screenplay is one fact, so the hits say where they are.
    if scenes.is_empty()
        && r.characters.is_empty()
        && let Some(term) = mention_term(&ctx.message)
    {
        let list: Vec<&ScreenplayInfo> = if r.screenplays.is_empty() {
            ctx.index.screenplays.iter().collect()
        } else {
            sps.clone()
        };
        let mut any = false;
        for sp in &list {
            let Some(d) = ctx.draft(&sp.id) else { continue };
            let hits = scenes_mentioning(ctx, &d.id, &term)?;
            if hits.is_empty() {
                continue;
            }
            any = true;
            let mut f = Fact::new("Scenes", &sp.id)
                .f("mention", &term)
                .f("screenplay", &sp.title)
                .f("scenes", numbers(hits.iter().copied()))
                .f("count", hits.len().to_string())
                .prio(P_HIGH);
            for n in &hits {
                f = f.require(n.to_string());
            }
            out.push(f);
        }
        if !any && !list.is_empty() {
            let places: Vec<String> = list
                .iter()
                .map(|s| format!("\u{201C}{}\u{201D}", s.title))
                .collect();
            out.push(Fact::absent(
                format!("a scene that mentions \u{201C}{term}\u{201D}"),
                queries::join_and(&places),
            ));
        }
    }
    Ok(out)
}

/// The word or phrase of "which scenes mention the cassette": what follows mention, contain or say.
fn mention_term(message: &str) -> Option<String> {
    static MENTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r#"(?i)\b(?:mention(?:s|ed|ing)?|contain(?:s|ed|ing)?|includ(?:e|es|ing)|say(?:s|ing)?)\s+(?:the\s+|a\s+|an\s+|word\s+|words\s+)*["\u{201C}'\u{2018}]?([^"\u{201D}'\u{2019}?.!,;]+?)["\u{201D}'\u{2019}]?\s*(?:[?.!,;]|\b(?:in|across|of|anywhere)\b|$)"#,
        )
        .expect("valid pattern")
    });
    let term = MENTION
        .captures(message)?
        .get(1)?
        .as_str()
        .trim()
        .to_string();
    (term.chars().count() >= 2 && term.chars().count() <= 40).then_some(term)
}

/// The scene numbers of a draft whose heading or text (not its notes) holds `term`.
fn scenes_mentioning(ctx: &FetchCtx<'_>, draft_id: &str, term: &str) -> AppResult<Vec<u32>> {
    let scenes = queries::scenes(ctx.conn, draft_id)?;
    let lower = term.to_lowercase();
    let mut stmt = ctx.conn.prepare(
        "SELECT DISTINCT e.scene_id FROM screenplay_element e JOIN screenplay_scene s ON s.id=e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type <> 'note' AND e.text LIKE ?2 ESCAPE '\\'",
    )?;
    let in_text: std::collections::BTreeSet<String> = stmt
        .query_map(
            rusqlite::params![draft_id, crate::modules::ai::tools::like_pattern(term)],
            |r| r.get::<_, String>(0),
        )?
        .collect::<Result<_, _>>()?;
    Ok(scenes
        .iter()
        .filter(|s| in_text.contains(&s.id) || s.heading.to_lowercase().contains(&lower))
        .map(|s| s.number as u32)
        .collect())
}

fn scene_notes(ctx: &FetchCtx<'_>, scene_id: &str) -> AppResult<(String, String)> {
    Ok(ctx.conn.query_row(
        "SELECT COALESCE(synopsis, ''), COALESCE(notes, '') FROM screenplay_scene WHERE id = ?1",
        [scene_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

fn scene_facts(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    s: &SceneRef,
    several: bool,
) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let asks_who = m.stem("character")
        || read.answer_type == AnswerType::Compare
        || m.any(&[
            "who", "in it", "cast", "present", "speaks", "appears", "involved",
        ]);
    let several_screenplays = ctx.index.screenplays.len() > 1;
    let presence = ctx.presence(&s.draft_id);
    let names = presence.by_scene.get(&s.id).cloned().unwrap_or_default();
    let (synopsis, note) = scene_notes(ctx, &s.id)?;
    let mut f = Fact::new("Scene", &s.id)
        .f("scene", s.number.to_string())
        .f("heading", &s.heading)
        .prio(P_HIGH);
    if several_screenplays {
        f = f.f("screenplay", &s.screenplay_title);
    }
    if narrative || asks_who {
        f = f.f("characters", names.join(", "));
        for n in &names {
            f = f.require(n);
        }
    }
    if narrative {
        f = f.f("summary", clip(&synopsis, 320));
        f = f.f("writer's note", clip(&note, 240));
    }
    let mut out = vec![f];
    if read.group == Group::Brainstorm {
        out.extend(scene_notes_for_brainstorm(ctx, s)?);
    }
    // Two scenes to compare are read, a little of each.
    let compare = read.answer_type == AnswerType::Compare && several;
    if narrative || compare {
        let limit = if compare {
            TEXT_COMPARE
        } else if read.group == Group::Brainstorm {
            TEXT_BRAINSTORM
        } else {
            TEXT_EXPLAIN
        };
        let text = queries::scene_text(ctx.conn, &s.id, limit)?;
        if !text.trim().is_empty() {
            out.push(
                Fact::new("Scene text", &s.id)
                    .f("scene", s.number.to_string())
                    .f("text", text)
                    .prio(P_MID - 10),
            );
        }
    }
    Ok(out)
}

/// What people wrote about a scene: the open comments, and the notes the asking person made for
/// themselves. A private note carries its owner, so no guard or later step can hand it to anyone else.
fn scene_notes_for_brainstorm(ctx: &FetchCtx<'_>, s: &SceneRef) -> AppResult<Vec<Fact>> {
    let mut out = Vec::new();
    let mut st = ctx.conn.prepare(
        "SELECT id, body FROM private_note WHERE owner_user_id = ?1 AND target_id = ?2 AND deleted_at IS NULL
         ORDER BY updated_at DESC LIMIT 3",
    )?;
    let mine: Vec<(String, String)> = st
        .query_map([&ctx.actor.user_id, &s.id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (id, body) in mine {
        out.push(
            Fact::new("Your private note", id)
                .f("scene", s.number.to_string())
                .f("text", clip(&body, 300))
                .private_to(&ctx.actor.user_id)
                .prio(P_LOW + 20),
        );
    }
    let mut st = ctx.conn.prepare(
        "SELECT id, body, author_name FROM comment WHERE scene_id = ?1 AND deleted_at IS NULL
         AND status <> 'Resolved' ORDER BY created_at DESC LIMIT 4",
    )?;
    let comments: Vec<(String, String, String)> = st
        .query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (id, body, author) in comments {
        out.push(
            Fact::new("Comment", id)
                .f("scene", s.number.to_string())
                .f("by", author)
                .f("text", clip(&body, 300))
                .prio(P_LOW + 20),
        );
    }
    Ok(out)
}

/// The scenes in which each named character speaks, and the scenes they share.
fn appearances(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    r: &Resolved,
    sp: &ScreenplayInfo,
    draft_id: &str,
) -> Vec<Fact> {
    let t = read.answer_type;
    let presence = ctx.presence(draft_id);
    let range = BETWEEN.captures(&ctx.message).and_then(|c| {
        let (a, b): (u32, u32) = (c[1].parse().ok()?, c[2].parse().ok()?);
        Some((a.min(b), a.max(b)))
    });
    let in_range = |n: &u32| range.is_none_or(|(a, b)| *n >= a && *n <= b);
    let several = ctx.index.screenplays.len() > 1;
    let mut out = Vec::new();
    let mut sets: Vec<(String, Vec<u32>)> = Vec::new();
    for h in &r.characters {
        let name = ctx
            .index
            .entity(&h.id)
            .map_or(h.name.clone(), |e| e.name.clone());
        let list: Vec<u32> = presence
            .by_name
            .get(&name)
            .map(|s| s.iter().copied().filter(|n| in_range(n)).collect())
            .unwrap_or_default();
        let mut f = Fact::new("Appearances", &h.id)
            .f("character", &name)
            .f(
                "speaks in scenes",
                if list.is_empty() {
                    "none".to_string()
                } else {
                    numbers(list.iter().copied())
                },
            )
            .f("count", list.len().to_string())
            .prio(P_HIGH);
        if several {
            f = f.f("screenplay", &sp.title);
        }
        if let Some((a, b)) = range {
            f = f.f("between", format!("{a} and {b}"));
        }
        if r.characters.len() == 1 {
            if t == AnswerType::Count {
                f = f.require(list.len().to_string());
            } else if list.len() <= 12 {
                for n in &list {
                    f = f.require(n.to_string());
                }
            }
        }
        out.push(f);
        sets.push((name, list));
    }
    if sets.len() >= 2 {
        let first = sets[0].1.clone();
        let both: Vec<u32> = first
            .into_iter()
            .filter(|n| sets.iter().all(|(_, l)| l.contains(n)))
            .collect();
        let names = sets
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(" and ");
        let mut f = Fact::new("Together", "")
            .f("characters", names)
            .f(
                "speak in the same scenes",
                if both.is_empty() {
                    "none".to_string()
                } else {
                    numbers(both.iter().copied())
                },
            )
            .f("count", both.len().to_string())
            .prio(P_HIGH + 10);
        if both.len() <= 12 {
            for n in &both {
                f = f.require(n.to_string());
            }
        }
        out.push(f);
    }
    out
}
