//! Breakdown: what a scene needs, by category (cast, extras, location, props, wardrobe, vehicles,
//! hair and makeup, special effects, VFX, sound, animals), and counts across the Production Source.
//! Reads `breakdown_element` and `production_source`.

use std::collections::BTreeMap;

use rusqlite::OptionalExtension;
use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{Msg, clip};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::Resolved;
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

/// Breakdown: what a scene needs, or, with no scene named, how many elements each category holds.
pub fn breakdown_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("breakdown", || {
        let mut out = fetch(ctx, read, r)?;
        if out.is_empty() && ctx.focus_scenes(r).is_empty() {
            let all: Vec<&str> = CATEGORIES.iter().map(|(c, _)| *c).collect();
            out = project_counts(ctx, AnswerType::Lookup, &all)?;
            // A category the project holds none of is not worth a line when nothing asked for it.
            out.retain(|f| !f.is_missing());
        }
        Ok(out)
    })
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
            if rows.is_empty() && !t.narrates() {
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

/// The scenes of the Production Source that have, and that lack, elements of a category: "which
/// scenes have a VFX element", "which scenes have no wardrobe". The numbers are the scene numbers.
fn scenes_with(ctx: &FetchCtx<'_>, cat: &str, source: Option<&str>) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT DISTINCT scene_id FROM breakdown_element
         WHERE category = ?1 AND deleted_at IS NULL AND archived = 0 AND confirmation_state <> 'Rejected'
           AND (?2 IS NULL OR source_id = ?2)",
    )?;
    let with: Vec<String> = st
        .query_map(rusqlite::params![cat, source], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let numbers = ctx.scene_numbers(&with);
    let mut have: Vec<u32> = with
        .iter()
        .filter_map(|id| numbers.get(id).map(|n| n.0))
        .collect();
    have.sort_unstable();
    have.dedup();
    let draft =
        crate::modules::ai::queries::production_source_draft(ctx.conn)?.and_then(|(_, d)| {
            crate::modules::ai::queries::draft_by_id(ctx.conn, &d)
                .ok()
                .flatten()
        });
    let all: Vec<u32> = match draft {
        Some(d) => crate::modules::ai::queries::scenes(ctx.conn, &d.id)?
            .into_iter()
            .filter(|s| !s.omitted)
            .map(|s| s.number as u32)
            .collect(),
        None => Vec::new(),
    };
    let without: Vec<u32> = all.iter().copied().filter(|n| !have.contains(n)).collect();
    let asks_without = ctx
        .m
        .any(&["no", "without", "lack", "lacks", "missing", "none"]);
    let mut f = Fact::new("Breakdown", "")
        .f("category", cat)
        .f(
            "scenes with it",
            crate::modules::ai::dossier::words::numbers(have.iter().copied()),
        )
        .prio(P_HIGH);
    if !without.is_empty() {
        f = f.f(
            "scenes without it",
            crate::modules::ai::dossier::words::numbers(without.iter().copied()),
        );
    }
    let asked = if asks_without { &without } else { &have };
    if asked.len() <= 16 {
        for n in asked {
            f = f.require(n.to_string());
        }
    }
    Ok(vec![f])
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
            f = f.require(elements.to_string());
        }
        out.push(f);
        out.extend(scenes_with(ctx, cat, source.as_deref())?);
    }
    Ok(out)
}
