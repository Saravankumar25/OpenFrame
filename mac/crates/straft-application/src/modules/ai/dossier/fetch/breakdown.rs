//! Breakdown: what a scene needs, by category (cast, extras, location, props, wardrobe, vehicles,
//! hair and makeup, special effects, VFX, sound, animals), and counts across the Production Source.
//! Reads `breakdown_element` and `production_source`.

use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{Msg, clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};

/// The categories and the words that name each.
const CATEGORIES: &[(&str, &[&str])] = &[
    ("Cast", &[]),
    (
        "Extras / Background",
        &["extras", "extra", "background", "crowd", "villagers"],
    ),
    ("Location / Set", &["set", "sets"]),
    ("Props", &["props", "prop"]),
    (
        "Wardrobe",
        &[
            "wardrobe", "costume", "costumes", "wear", "wears", "clothes", "clothing", "dress",
        ],
    ),
    (
        "Vehicles",
        &["vehicle", "vehicles", "boat", "boats", "truck", "trucks"],
    ),
    ("Hair / Makeup", &["makeup", "make up", "hair"]),
    (
        "Special Effects",
        &[
            "special effects",
            "special effect",
            "effects",
            "effect",
            "sfx",
        ],
    ),
    (
        "VFX",
        &[
            "vfx",
            "visual effects",
            "special effects",
            "special effect",
            "effects",
            "effect",
        ],
    ),
    ("Sound", &["sound", "sounds"]),
    ("Animals", &["animals", "animal"]),
];

/// The categories a message names.
pub fn categories_in(m: &Msg) -> Vec<&'static str> {
    CATEGORIES
        .iter()
        .filter(|(_, words)| m.any(words))
        .map(|(c, _)| *c)
        .collect()
}

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("breakdown", || fetch(ctx, read, r))
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let t = read.answer_type;
    let asked = categories_in(&ctx.m);
    let scenes = ctx.focus_scenes(r);
    let mut out = Vec::new();
    if scenes.is_empty() {
        if asked.is_empty() {
            return Ok(out);
        }
        out.extend(project_counts(ctx, t, &asked)?);
        return Ok(out);
    }
    for s in scenes.iter().take(4) {
        let mut st = ctx.conn.prepare(
            "SELECT category, display_name FROM breakdown_element
             WHERE scene_id = ?1 AND deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'
             ORDER BY category, display_name",
        )?;
        let rows: Vec<(String, String)> = st
            .query_map([&s.id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut by_cat: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for (cat, name) in &rows {
            if let Some((c, _)) = CATEGORIES.iter().find(|(c, _)| c == cat) {
                by_cat.entry(c).or_default().push(name.clone());
            }
        }
        let one = |cat: &str, items: &[String]| {
            let mut f = Fact::new("Breakdown", &s.id)
                .f("scene", s.number.to_string())
                .f("category", cat)
                .f("items", clip(&items.join(", "), 420))
                .prio(if asked.contains(&cat) { P_HIGH } else { P_MID });
            if asked.contains(&cat) && items.len() <= 10 {
                for i in items {
                    f = f.require(i);
                }
            }
            f
        };
        if asked.is_empty() {
            // Nothing named: everything the scene needs, but no category a person asks about cast for.
            for (cat, _) in CATEGORIES.iter().filter(|(c, _)| *c != "Cast") {
                if let Some(items) = by_cat.get(cat) {
                    let mut f = one(cat, items);
                    f.priority = if matches!(read.group, Group::Narrative) {
                        P_MID
                    } else {
                        P_LOW + 10
                    };
                    f.requires.clear();
                    out.push(f);
                }
            }
            if rows.is_empty() && !t.tells() {
                out.push(Fact::absent(
                    format!("any breakdown elements for scene {}", s.number),
                    "the Breakdown",
                ));
            }
        } else {
            let mut any = false;
            for cat in &asked {
                if let Some(items) = by_cat.get(cat) {
                    out.push(one(cat, items));
                    any = true;
                }
            }
            if !any {
                out.push(Fact::absent(
                    format!(
                        "any {} for scene {}",
                        asked
                            .iter()
                            .map(|c| c.to_lowercase())
                            .collect::<Vec<_>>()
                            .join(" or "),
                        s.number
                    ),
                    "the Breakdown",
                ));
            }
        }
    }
    Ok(out)
}

/// "How many wardrobe elements are there": the elements of the active Production Source.
fn project_counts(ctx: &FetchCtx<'_>, t: AnswerType, asked: &[&str]) -> AppResult<Vec<Fact>> {
    let source: Option<String> = ctx
        .conn
        .query_row(
            "SELECT id FROM production_source WHERE active = 1 ORDER BY selected_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let mut out = Vec::new();
    for cat in asked {
        let (elements, distinct): (i64, i64) = ctx.conn.query_row(
            "SELECT count(*), count(DISTINCT lower(display_name)) FROM breakdown_element
             WHERE category = ?1 AND deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'
               AND (?2 IS NULL OR source_id = ?2)",
            rusqlite::params![cat, source],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if elements == 0 {
            out.push(Fact::absent(
                format!("any {} in this project", cat.to_lowercase()),
                "the Breakdown",
            ));
            continue;
        }
        let mut f = Fact::new("Count", "")
            .f(
                "what",
                format!("{} elements in the breakdown", cat.to_lowercase()),
            )
            .f("value", elements.to_string())
            .f("different items", distinct.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count {
            let different = ctx.m.any(&["distinct", "different", "unique"]);
            f = f.require(if different { distinct } else { elements }.to_string());
        }
        out.push(f);
        // "Which animals appear": the names of the category, when they are few enough to state.
        if t == AnswerType::List && !ctx.m.any(&["scene", "scenes"]) {
            let mut st = ctx.conn.prepare(
                "SELECT DISTINCT display_name FROM breakdown_element
                 WHERE category = ?1 AND deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'
                   AND (?2 IS NULL OR source_id = ?2) ORDER BY display_name LIMIT 40",
            )?;
            let names: Vec<String> = st
                .query_map(rusqlite::params![cat, source], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            let mut f = Fact::new("Breakdown", "")
                .f("category", cat)
                .f("items", clip(&names.join(", "), 420))
                .prio(P_HIGH);
            if names.len() <= 10 {
                for n in &names {
                    f = f.require(n);
                }
            }
            out.push(f);
        }
        // "Which scenes have no wardrobe": the scenes of the source with nothing in the category.
        if t == AnswerType::List && ctx.m.any(&["no", "without"]) && ctx.m.any(&["scene", "scenes"])
        {
            let without = scenes_without(ctx, cat)?;
            let mut f = Fact::new("Breakdown", "")
                .f("category", cat)
                .f("scenes with none", numbers(without.iter().copied()))
                .prio(P_HIGH);
            if without.len() <= 12 {
                for n in &without {
                    f = f.require(n.to_string());
                }
            }
            out.push(f);
        }
    }
    Ok(out)
}

/// The numbers of the scenes of the current draft that have no element of a category.
fn scenes_without(ctx: &FetchCtx<'_>, category: &str) -> AppResult<Vec<u32>> {
    let Some((_, draft)) = queries::production_source_draft(ctx.conn)? else {
        return Ok(Vec::new());
    };
    let mut st = ctx.conn.prepare(
        "SELECT DISTINCT scene_id FROM breakdown_element
         WHERE category = ?1 AND deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'",
    )?;
    let with: Vec<String> = st
        .query_map([category], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(queries::scenes(ctx.conn, &draft)?
        .into_iter()
        .filter(|s| !with.contains(&s.id))
        .map(|s| s.number as u32)
        .collect())
}

/// How much the breakdown holds, for a question that names no category and no scene.
pub fn summary(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    if !categories_in(&ctx.m).is_empty() || !ctx.focus_scenes(r).is_empty() {
        return Vec::new();
    }
    safe("breakdown summary", || {
        let mut st = ctx.conn.prepare(
            "SELECT category, count(*) FROM breakdown_element
             WHERE deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'
             GROUP BY category ORDER BY category",
        )?;
        let rows: Vec<(String, i64)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        if rows.is_empty() {
            return Ok(vec![Fact::absent(
                "any breakdown elements",
                "the Breakdown",
            )]);
        }
        let total: i64 = rows.iter().map(|(_, n)| n).sum();
        // A category the words of the question name ("cast elements") is what is counted.
        let named_row = |category: &str| {
            category
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .any(|w| ctx.m.has(&w.to_lowercase()))
        };
        let counting = read.answer_type == AnswerType::Count;
        let any_named = rows.iter().any(|(c, _)| named_row(c));
        let mut whole = Fact::new("Count", "")
            .f("what", "elements in the breakdown")
            .f("value", total.to_string())
            .prio(P_HIGH);
        if counting && !any_named {
            whole = whole.require(total.to_string());
        }
        let mut out = vec![whole];
        for (category, n) in rows {
            let named = named_row(&category);
            let mut f = Fact::new("Count", "")
                .f(
                    "what",
                    format!("{} elements in the breakdown", category.to_lowercase()),
                )
                .f("value", n.to_string())
                .prio(if named { P_HIGH } else { P_MID });
            if counting && named {
                f = f.require(n.to_string());
            }
            out.push(f);
        }
        Ok(out)
    })
}
