//! Visual: shot lists, storyboards, mood boards and the Idea Vault.
//! Reads `shot`, `storyboard`, `storyboard_panel`, `moodboard`, `moodboard_item`, `vault_item`.
//! Items a person marked private on a mood board are never read.

use std::sync::LazyLock;

use openframe_domain::AppResult;
use regex::Regex;

use crate::modules::ai::dossier::asked::{best_rows, names_row, row_words};
use crate::modules::ai::dossier::ctx::{FetchCtx, SceneRef, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID, P_PICKED};
use crate::modules::ai::index::{Kind, Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};

static PANEL_N: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bpanel\s+#?(\d{1,3})\b").expect("valid pattern"));

/// The most shots passed as rows when no scene is named.
const SHOT_ROWS: usize = 40;
/// The panel notes of a board given to the writer.
const PANEL_CHARS: usize = 700;
/// A list longer than this is not demanded row by row.
const LIST_DEMANDED: usize = 14;
/// A text longer than this is not demanded word for word.
const LONG_TEXT: usize = 90;
/// Words that only name the shot list.
const SHOT_WORDS: &[&str] = &[
    "shot", "shots", "list", "lists", "setup", "setups", "planned", "plan", "scene", "scenes",
    "there",
];
/// Words that only name the Idea Vault.
const VAULT_WORDS: &[&str] = &["idea", "vault", "project", "item", "items", "source", "who"];
/// Words that only name a mood board.
const MOOD_WORDS: &[&str] = &[
    "mood",
    "board",
    "boards",
    "moodboard",
    "moodboards",
    "item",
    "items",
];

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
            out.extend(all_shots(ctx, t)?);
        }
        for s in scenes.iter().take(4) {
            out.extend(shots(ctx, t, s)?);
        }
    }
    if m.stem("storyboard") {
        out.extend(storyboards(ctx, t, &scenes)?);
    }
    if m.stem("moodboard") || m.any(&["mood board", "mood boards"]) {
        out.extend(moodboards(ctx, t)?);
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

/// The shots of the whole project: the count, and the rows with the scene each belongs to. A word of
/// the question ("handheld", "crane") narrows them, and then the count is of the rows it picks.
fn all_shots(ctx: &FetchCtx<'_>, t: AnswerType) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(scene_id, ''), COALESCE(description, ''), COALESCE(size, ''), COALESCE(movement, ''), COALESCE(angle, ''), COALESCE(lens, '')
         FROM shot WHERE deleted_at IS NULL ORDER BY position, id",
    )?;
    type Row = (String, String, String, String, String, String, String);
    let rows: Vec<Row> = st
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Ok(vec![Fact::absent("any shots", "the shot lists")]);
    }
    let ids: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();
    let by_id = ctx.scene_numbers(&ids);
    let number_of = |id: &str| by_id.get(id).map(|(n, _)| *n);
    let words = row_words(&ctx.message, SHOT_WORDS);
    let texts: Vec<String> = rows
        .iter()
        .map(|r| format!("{} {} {} {} {}", r.2, r.3, r.4, r.5, r.6))
        .collect();
    let hits = best_rows(&words, &texts, rows.len());
    let total = Fact::new("Count", "")
        .f("what", "shots in the project")
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    let mut out = Vec::new();
    if t == AnswerType::Count && words.is_empty() {
        out.push(total.require(rows.len().to_string()));
    } else {
        out.push(total);
    }
    if !hits.is_empty() && t == AnswerType::Count {
        out.push(
            Fact::new("Count", "")
                .f("what", format!("shots with {}", words.join(" ")))
                .f("value", hits.len().to_string())
                .require(hits.len().to_string())
                .prio(P_HIGH),
        );
    }
    let mut scene_numbers: Vec<u32> = rows.iter().filter_map(|r| number_of(&r.1)).collect();
    scene_numbers.sort_unstable();
    scene_numbers.dedup();
    if words.is_empty() && !scene_numbers.is_empty() {
        let mut f = Fact::new("Count", "")
            .f("what", "scenes with shots")
            .f("value", scene_numbers.len().to_string())
            .f("scenes", numbers(scene_numbers.iter().copied()))
            .prio(P_HIGH);
        if t == AnswerType::List && scene_numbers.len() <= LIST_DEMANDED {
            for n in &scene_numbers {
                f = f.require(n.to_string());
            }
        }
        out.push(f);
    }
    for (i, (id, scene, d, size, mv, angle, lens)) in rows.iter().enumerate().take(SHOT_ROWS) {
        let hit = hits.contains(&i);
        let number = number_of(scene);
        let mut f = Fact::new("Shot", id)
            .f("scene", number.map(|n| n.to_string()).unwrap_or_default())
            .f("description", clip(d, 160))
            .f("size", size)
            .f("movement", mv)
            .f("angle", angle)
            .f("lens", lens)
            .prio(if hit { P_PICKED } else { P_MID });
        if hit && hits.len() <= 3 && t != AnswerType::Count {
            f = f.require(number.map(|n| n.to_string()).unwrap_or_default());
        }
        out.push(f);
    }
    Ok(out)
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
    // "How many shots" beside another question is still asked for.
    if t == AnswerType::Count || ctx.m.has("how many") {
        c = c.require(rows.len().to_string());
    }
    out.push(c);
    for (i, (id, d, size, mv, angle, lens)) in rows.iter().enumerate() {
        let mut shot = Fact::new("Shot", id)
            .f("scene", s.number.to_string())
            .f("number", (i + 1).to_string())
            .f("description", clip(d, 160))
            .f("size", size)
            .f("movement", mv)
            .f("angle", angle)
            .f("lens", lens)
            .prio(P_MID);
        // A short list asked for as a list is told shot by shot.
        if t == AnswerType::List && rows.len() <= 6 {
            shot = shot.require(clip(d, LONG_TEXT)).require(mv);
        }
        out.push(shot);
    }
    Ok(out)
}

fn storyboards(ctx: &FetchCtx<'_>, t: AnswerType, scenes: &[SceneRef]) -> AppResult<Vec<Fact>> {
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
        let by_id = ctx.scene_numbers(&ids);
        let one_board = rows.len() <= 3;
        let panels_counted = ctx.m.stem("panel");
        let mut boards_counted = Fact::new("Count", "")
            .f("what", "storyboards")
            .f("value", rows.len().to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && !panels_counted {
            boards_counted = boards_counted.require(rows.len().to_string());
        }
        out.push(boards_counted);
        for (id, name, scene_id, _, n) in rows.iter().take(20) {
            let number = scene_id
                .as_deref()
                .and_then(|s| by_id.get(s))
                .map(|(n, _)| n.to_string())
                .unwrap_or_default();
            let named = names_row(&ctx.message, name);
            let mut f = Fact::new("Storyboard", id)
                .f("name", name)
                .f("scene", &number)
                .f("panels", n.to_string())
                .prio(if named || one_board { P_HIGH } else { P_MID });
            if named || one_board {
                let empty: i64 = ctx.conn.query_row(
                    "SELECT count(*) FROM storyboard_panel WHERE storyboard_id = ?1
                     AND deleted_at IS NULL AND visual_kind = 'placeholder'",
                    [id],
                    |r| r.get(0),
                )?;
                let panels = panel_lines(ctx, id)?;
                f = f
                    .f("panels with no drawing", format!("{empty} empty"))
                    .f("panel notes", clip(&numbered(&panels), PANEL_CHARS));
                // A panel the question names by its place is demanded.
                let m = &ctx.m;
                let asked_panel = if m.any(&["last panel", "final panel"]) {
                    panels.last()
                } else if m.any(&["first panel"]) {
                    panels.first()
                } else {
                    PANEL_N
                        .captures(&ctx.message)
                        .and_then(|c| c[1].parse::<usize>().ok())
                        .and_then(|n| panels.get(n.saturating_sub(1)))
                };
                if let Some(p) = asked_panel.filter(|p| p.chars().count() <= LONG_TEXT) {
                    f = f.require(p);
                }
                if m.any(&[
                    "drawing", "drawings", "drawn", "image", "images", "sketch", "sketches",
                ]) {
                    f = f.require(format!("{empty} empty"));
                }
                if t == AnswerType::Count && panels_counted {
                    f = f.require(n.to_string());
                }
            }
            if ctx.m.without(&["storyboards", "storyboard"]).stem("scene") {
                f = f.require(&number);
            }
            out.push(f);
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
            out.push(
                Fact::new("Storyboard", id)
                    .f("name", name)
                    .f("scene", s.number.to_string())
                    .f("panels", n.to_string())
                    .f(
                        "panel notes",
                        clip(&numbered(&panel_lines(ctx, id)?), PANEL_CHARS),
                    )
                    .prio(P_HIGH)
                    .require(name)
                    .require(n.to_string()),
            );
        }
    }
    Ok(out)
}

/// The panels of a board in order.
fn panel_lines(ctx: &FetchCtx<'_>, board: &str) -> AppResult<Vec<String>> {
    let mut st = ctx.conn.prepare(
        "SELECT COALESCE(description, ''), COALESCE(framing, '') FROM storyboard_panel
         WHERE storyboard_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let panels: Vec<String> = st
        .query_map([board], |r| {
            Ok(format!(
                "{} {}",
                r.get::<_, String>(1)?,
                r.get::<_, String>(0)?
            ))
        })?
        .collect::<Result<_, _>>()?;
    Ok(panels.iter().map(|p| p.trim().to_string()).collect())
}

/// The panels numbered: "1: Wide Lamps on the water; 2: Close the meter climbs".
fn numbered(panels: &[String]) -> String {
    panels
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}: {p}", i + 1))
        .collect::<Vec<_>>()
        .join("; ")
}

fn moodboards(ctx: &FetchCtx<'_>, t: AnswerType) -> AppResult<Vec<Fact>> {
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
    let asks_items = ctx.m.stem("note") || ctx.m.stem("item");
    let mut out = Vec::new();
    let mut boards_counted = Fact::new("Count", "")
        .f("what", "mood boards")
        .f("value", boards.len().to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count && !asks_items {
        boards_counted = boards_counted.require(boards.len().to_string());
    }
    out.push(boards_counted);
    let words = row_words(&ctx.message, MOOD_WORDS);
    for (id, name, notes) in &boards {
        let is_named = named.iter().any(|(i, _, _)| i == id);
        let mut f = Fact::new("Mood board", id)
            .f("name", name)
            .prio(if is_named { P_HIGH } else { P_LOW });
        if !is_named && boards.len() != 1 {
            out.push(f);
            continue;
        }
        f = f.f("notes", clip(notes, 200));
        let mut st = ctx.conn.prepare(
            "SELECT kind, COALESCE(body, ''), COALESCE(caption, ''), COALESCE(link_title, '') FROM moodboard_item
             WHERE moodboard_id = ?1 AND deleted_at IS NULL AND is_private = 0 ORDER BY z, id LIMIT 40",
        )?;
        let items: Vec<(String, String)> = st
            .query_map([id], |r| {
                let (kind, body, caption, title): (String, String, String, String) =
                    (r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?);
                Ok((kind, [body, caption, title].join(" ").trim().to_string()))
            })?
            .collect::<Result<_, _>>()?;
        let with_kind =
            |kind: &str, plural: &str| match items.iter().filter(|(k, _)| k == kind).count() {
                0 => format!("no {plural}"),
                n => format!("{n} {plural}"),
            };
        f = f.f(
            "holds",
            format!(
                "{}, {}, {}",
                with_kind("note", "notes"),
                with_kind("image", "images"),
                with_kind("link", "links")
            ),
        );
        out.push(f);
        let mut counted = Fact::new("Count", id)
            .f("what", format!("notes on the board {name}"))
            .f("value", items.len().to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && asks_items {
            counted = counted.require(items.len().to_string());
        }
        out.push(counted);
        let texts: Vec<String> = items.iter().map(|(_, x)| x.clone()).collect();
        let hits = best_rows(&words, &texts, 3);
        for (i, (kind, text)) in items.iter().enumerate() {
            let mut row = Fact::new("Mood board", id)
                .f("board", name)
                .f("kind", kind)
                .f("text", clip(text, 240))
                .prio(if hits.contains(&i) { P_PICKED } else { P_MID });
            if hits.contains(&i) && text.chars().count() <= LONG_TEXT {
                row = row.require(text);
            }
            out.push(row);
        }
    }
    Ok(out)
}

/// The type of a vault item as the screen calls it.
fn vault_type(kind: &str) -> &str {
    if kind == "url" { "link" } else { kind }
}

fn vault(ctx: &FetchCtx<'_>, read: &TurnRead, named: &[String]) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, COALESCE(title, ''), item_type, COALESCE(body, ''), COALESCE(caption, ''), COALESCE(source_text, '') FROM vault_item
         WHERE deleted_at IS NULL ORDER BY created_at, id LIMIT 200",
    )?;
    type Row = (String, String, String, String, String, String);
    let rows: Vec<Row> = st
        .query_map([], |r| {
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
        return Ok(vec![Fact::absent("any items", "the Idea Vault")]);
    }
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let t = read.answer_type;
    let list = t == AnswerType::List;
    let mut count = Fact::new("Count", "")
        .f("what", "items in the Idea Vault")
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count {
        count = count.require(rows.len().to_string());
    }
    let mut out = vec![count];
    let words = row_words(&ctx.message, VAULT_WORDS);
    let texts: Vec<String> = rows
        .iter()
        .map(|r| format!("{} {} {} {} {}", r.1, vault_type(&r.2), r.3, r.4, r.5))
        .collect();
    let hits = best_rows(&words, &texts, 3);
    let asks_source = ctx.m.any(&["source", "who", "whose", "from"]);
    let asks_kinds = ctx
        .m
        .any(&["kind", "kinds", "type", "types", "sort", "sorts"]);
    let whole = list && words.is_empty();
    for (i, (id, title, kind, body, caption, source)) in rows.iter().enumerate() {
        let is_named = named.contains(id);
        let hit = hits.contains(&i);
        if !is_named && !list && !ctx.m.has("vault") {
            continue;
        }
        let mut f = Fact::new("Idea Vault item", id)
            .f("title", title)
            .f("type", vault_type(kind))
            .f("source", source)
            .prio(if hit {
                P_PICKED
            } else if is_named {
                P_HIGH
            } else {
                P_MID
            });
        if is_named || hit || narrative {
            f = f.f("text", clip(&format!("{body} {caption}"), 360));
        }
        if t != AnswerType::Count {
            if asks_kinds {
                f = f.require(vault_type(kind));
            } else if hit || (whole && rows.len() <= LIST_DEMANDED) {
                f = f.require(title);
            }
            if asks_source && hit && !source.trim().is_empty() {
                f = f.require(source);
            }
        }
        out.push(f);
    }
    Ok(out)
}

/// The Global Idea Vault, read only when the person picks its section (ADR-0041). It lives outside
/// the project, so a failure to reach it says nothing about the project.
pub fn global_vault(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("global idea vault", || {
        let Some(core) = ctx.core else {
            return Ok(Vec::new());
        };
        let rows: Vec<(String, String, String, String, String)> =
            core.global_store()?.read(|c| {
                let mut st = c.prepare(
                    "SELECT id, COALESCE(title, ''), item_type, COALESCE(body, ''), COALESCE(caption, '') FROM vault_item
                     WHERE deleted_at IS NULL ORDER BY created_at, id LIMIT 200",
                )?;
                let rows = st
                    .query_map([], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
                    })?
                    .collect::<Result<_, _>>()?;
                Ok(rows)
            })?;
        if rows.is_empty() {
            return Ok(vec![Fact::absent("any items", "the Global Idea Vault")]);
        }
        let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
        let words = ctx.m.keywords();
        let mut out = Vec::new();
        for (id, title, kind, body, caption) in &rows {
            let text = format!("{title} {body} {caption}").to_lowercase();
            let hit = words.iter().any(|w| text.contains(w.as_str()));
            let mut f = Fact::new("Global Idea Vault item", id)
                .f("title", title)
                .f("type", kind)
                .prio(if hit { P_HIGH } else { P_MID });
            if hit || narrative {
                f = f.f("text", clip(&format!("{body} {caption}"), 360));
            }
            out.push(f);
        }
        Ok(out)
    })
}
