//! Visual: shot lists, storyboards, mood boards and the Idea Vault.
//! Reads `shot`, `storyboard`, `storyboard_panel`, `moodboard`, `moodboard_item`, `vault_item`.
//! Items a person marked private on a mood board are never read.

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Kind, Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("visual", || fetch(ctx, read, r))
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let scenes = ctx.focus_scenes(r);
    let mut out = Vec::new();
    if m.any(&["shots", "shot list", "shot lists", "setups"]) || (m.has("shot") && m.has("many")) {
        if scenes.is_empty() {
            out.extend(shot_total(ctx, t)?);
        }
        for s in scenes.iter().take(4) {
            out.extend(shots(ctx, t, s)?);
        }
    }
    if m.stem("storyboard") {
        out.extend(storyboards(ctx, &scenes)?);
    }
    if m.stem("moodboard") || m.any(&["mood board", "mood boards"]) {
        out.extend(moodboards(ctx)?);
    }
    let vault_items: Vec<_> = r
        .others
        .iter()
        .filter(|(k, _)| *k == Kind::VaultItem)
        .map(|(_, h)| h)
        .collect();
    if m.has("vault") || m.has("idea vault") || !vault_items.is_empty() {
        out.extend(vault(
            ctx,
            read,
            &vault_items.iter().map(|h| h.id.clone()).collect::<Vec<_>>(),
        )?);
    }
    Ok(out)
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
        for (id, name, _, _, n) in rows.iter().take(20) {
            out.push(
                Fact::new("Storyboard", id)
                    .f("name", name)
                    .f("panels", n.to_string())
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
                    .f("panel notes", clip(&panels.join("; "), 360))
                    .prio(P_HIGH)
                    .require(name)
                    .require(n.to_string()),
            );
        }
    }
    Ok(out)
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

fn vault(ctx: &FetchCtx<'_>, read: &TurnRead, named: &[String]) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(title, ''), item_type, COALESCE(body, ''), COALESCE(caption, '') FROM vault_item
         WHERE deleted_at IS NULL ORDER BY created_at, id LIMIT 200",
    )?;
    let rows: Vec<(String, String, String, String, String)> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent("any items", "the Idea Vault")]);
    }
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let list = read.answer_type == AnswerType::List;
    let mut out = Vec::new();
    for (id, title, kind, body, caption) in &rows {
        let is_named = named.contains(id);
        if !is_named && !list && !ctx.m.has("vault") {
            continue;
        }
        let mut f = Fact::new("Idea Vault item", id)
            .f("title", title)
            .f("type", kind)
            .prio(if is_named { P_HIGH } else { P_MID });
        if is_named || narrative {
            f = f.f("text", clip(&format!("{body} {caption}"), 360));
        }
        if list && rows.len() <= 14 {
            f = f.require(title);
        }
        out.push(f);
    }
    Ok(out)
}
