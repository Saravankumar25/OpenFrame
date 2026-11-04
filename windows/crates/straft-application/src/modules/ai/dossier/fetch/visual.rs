//! Visual: shot lists, storyboards and mood boards (one leaf each); the Idea Vault is `vault.rs`.
//! Reads `shot`, `storyboard`, `storyboard_panel`, `moodboard`, `moodboard_item`.
//! Items a person marked private on a mood board are never read.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, TurnRead};

/// Production > Shot Lists: the shots planned for a scene, or how many there are in all.
pub fn shot_lists_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("shot lists", || {
        let t = read.answer_type;
        let scenes = ctx.focus_scenes(r);
        let mut out = Vec::new();
        if scenes.is_empty() {
            out.extend(shot_total(ctx, t)?);
        }
        for s in scenes.iter().take(4) {
            out.extend(shots(ctx, t, s)?);
        }
        Ok(out)
    })
}

/// Production > Storyboards: the storyboards, and the panels of the ones for a scene.
pub fn storyboards_leaf(ctx: &FetchCtx<'_>, _read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("storyboards", || storyboards(ctx, &ctx.focus_scenes(r)))
}

/// Production > Moodboards: the boards, and the notes and items on the one named (or the only one).
pub fn moodboards_leaf(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("moodboards", || moodboards(ctx))
}

fn shot_total(ctx: &FetchCtx<'_>, t: AnswerType) -> AppResult<Vec<Fact>> {
    let n: i64 = ctx.conn.query_row(
        "SELECT count(*) FROM shot WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    if n == 0 {
        return Ok(vec![Fact::absent("any shots", "the shot lists")]);
    }
    let mut f = Fact::new("Count", "")
        .f("what", "shots in the project")
        .f("value", n.to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count {
        f = f.require(n.to_string());
    }
    Ok(vec![f])
}

fn shots(ctx: &FetchCtx<'_>, t: AnswerType, s: &SceneRef) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(description, ''), COALESCE(size, ''), COALESCE(movement, ''), COALESCE(angle, ''), COALESCE(lens, '')
         FROM shot WHERE (scene_id = ?1 OR scene_lineage_id = ?2) AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows: Vec<(String, String, String, String, String, String)> = st
        .query_map([&s.id, &s.lineage_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent(
            format!("any shots for scene {}", s.number),
            "the shot lists",
        )]);
    }
    let mut out = Vec::new();
    let mut c = Fact::new("Count", &s.id)
        .f("what", format!("shots planned for scene {}", s.number))
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count {
        c = c.require(rows.len().to_string());
    }
    out.push(c);
    for (i, (id, d, size, mv, angle, lens)) in rows.iter().enumerate() {
        out.push(
            Fact::new("Shot", id)
                .f("scene", s.number.to_string())
                .f("number", (i + 1).to_string())
                .f("description", clip(d, 160))
                .f("size", size)
                .f("movement", mv)
                .f("angle", angle)
                .f("lens", lens)
                .prio(P_MID),
        );
    }
    Ok(out)
}

fn storyboards(ctx: &FetchCtx<'_>, scenes: &[SceneRef]) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT b.id, b.name, b.scene_id, b.scene_lineage_id,
                (SELECT count(*) FROM storyboard_panel p WHERE p.storyboard_id = b.id AND p.deleted_at IS NULL)
         FROM storyboard b WHERE b.deleted_at IS NULL ORDER BY b.position, b.id LIMIT 100",
    )?;
    let rows: Vec<(String, String, Option<String>, Option<String>, i64)> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    if scenes.is_empty() {
        if rows.is_empty() {
            return Ok(vec![Fact::absent("any storyboards", "Storyboards")]);
        }
        let ids: Vec<String> = rows.iter().filter_map(|r| r.2.clone()).collect();
        let numbers = ctx.scene_numbers(&ids);
        for (id, name, scene_id, _, n) in rows.iter().take(20) {
            let scene = scene_id
                .as_ref()
                .and_then(|s| numbers.get(s))
                .map(|(n, _)| n.to_string())
                .unwrap_or_default();
            out.push(
                Fact::new("Storyboard", id)
                    .f("name", name)
                    .f("scene", scene)
                    .f("panels", n.to_string())
                    .f("panels without a picture", empty_panels(ctx, id)?)
                    .prio(P_MID),
            );
        }
        return Ok(out);
    }
    for s in scenes.iter().take(3) {
        let mine: Vec<_> = rows
            .iter()
            .filter(|(_, _, sid, lid, _)| {
                sid.as_deref() == Some(s.id.as_str())
                    || lid.as_deref() == Some(s.lineage_id.as_str())
            })
            .collect();
        if mine.is_empty() {
            out.push(Fact::absent(
                format!("a storyboard for scene {}", s.number),
                "Storyboards",
            ));
        }
        for (id, name, _, _, n) in mine {
            let mut st = ctx.conn.prepare(
                "SELECT COALESCE(description, ''), COALESCE(framing, '') FROM storyboard_panel
                 WHERE storyboard_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
            )?;
            let panels: Vec<String> = st
                .query_map([id], |r| {
                    Ok(format!(
                        "{} {}",
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(0)?
                    ))
                })?
                .collect::<Result<_, _>>()?;
            out.push(
                Fact::new("Storyboard", id)
                    .f("name", name)
                    .f("scene", s.number.to_string())
                    .f("panels", n.to_string())
                    .f("panels without a picture", empty_panels(ctx, id)?)
                    .f("panel notes", clip(&panels.join("; "), 360))
                    .prio(P_HIGH)
                    .require(name)
                    .require(n.to_string()),
            );
        }
    }
    Ok(out)
}

/// How many panels of a storyboard are still the Empty Panel the screen adds (no image, no sketch).
fn empty_panels(ctx: &FetchCtx<'_>, board_id: &str) -> AppResult<String> {
    let n: i64 = ctx.conn.query_row(
        "SELECT count(*) FROM storyboard_panel WHERE storyboard_id = ?1 AND deleted_at IS NULL AND visual_kind = 'placeholder'",
        [board_id],
        |r| r.get(0),
    )?;
    Ok(format!("{n} (empty panels)"))
}

fn moodboards(ctx: &FetchCtx<'_>) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, name, COALESCE(notes, '') FROM moodboard WHERE deleted_at IS NULL ORDER BY position, id LIMIT 60",
    )?;
    let boards: Vec<(String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    if boards.is_empty() {
        return Ok(vec![Fact::absent("any moodboards", "Moodboards")]);
    }
    let named: Vec<_> = boards
        .iter()
        .filter(|(_, n, _)| {
            let n = normalize(n);
            !n.is_empty() && ctx.m.has(&n)
        })
        .collect();
    let mut out = Vec::new();
    for (id, name, notes) in &boards {
        let is_named = named.iter().any(|(i, _, _)| i == id);
        let mut f = Fact::new("Mood board", id)
            .f("name", name)
            .prio(if is_named { P_HIGH } else { P_LOW });
        if is_named || boards.len() == 1 {
            f = f.f("notes", clip(notes, 200));
            let mut st = ctx.conn.prepare(
                "SELECT kind, COALESCE(body, ''), COALESCE(caption, ''), COALESCE(link_title, '') FROM moodboard_item
                 WHERE moodboard_id = ?1 AND deleted_at IS NULL AND is_private = 0 ORDER BY z, id LIMIT 40",
            )?;
            let items: Vec<String> = st
                .query_map([id], |r| {
                    let (kind, body, caption, title): (String, String, String, String) =
                        (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
                    Ok(format!(
                        "{kind}: {}",
                        [body, caption, title].join(" ").trim()
                    ))
                })?
                .collect::<Result<_, _>>()?;
            f = f.f("items", clip(&items.join("; "), 520));
        }
        out.push(f);
    }
    Ok(out)
}
