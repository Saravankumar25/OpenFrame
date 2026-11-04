//! Production > Overview and Production > Catalog.
//! The overview is the read the Overview screen calls (`production::source::overview_read`); the
//! catalog reads `catalog_item`. Contact data of a catalog item is never read.

use straft_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, numbers};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::{AnswerType, TurnRead};
use crate::modules::production::source::overview_read;

/// Production > Overview: how far the breakdown is, and how many locations, characters and catalog
/// items there are.
pub fn overview_leaf(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    let Some(core) = ctx.core else {
        return Vec::new();
    };
    safe("production overview", || {
        let root = core.project()?.layout.root().to_path_buf();
        let o = overview_read(ctx.conn, &root)?;
        let first = |v: &[crate::modules::production::ProductionSceneRef]| {
            numbers(
                v.iter()
                    .filter_map(|s| s.number.as_ref().and_then(|n| n.parse::<u32>().ok()))
                    .take(12),
            )
        };
        let mut f = Fact::new("Production overview", "")
            .f(
                "source draft",
                o.source
                    .as_ref()
                    .map(|s| format!("{} ({})", s.draft.screenplay_title, s.draft.name))
                    .unwrap_or_default(),
            )
            .f("scenes", o.scene_count.to_string())
            .f("breakdown complete", o.complete_count.to_string())
            .f("with confirmed elements", o.planned_count.to_string())
            .f("suggestions to review", o.pending_suggestions.to_string())
            .f("scenes needing review", first(&o.needs_review))
            .f("scenes needing a breakdown", first(&o.needs_breakdown))
            .f("locations", o.location_count.to_string())
            .f(
                "locations not confirmed",
                o.unresolved_locations.to_string(),
            )
            .f("characters", o.character_count.to_string())
            .f(
                "characters without an actor",
                o.unresolved_characters.join(", "),
            )
            .f("catalog items", o.catalog_count.to_string())
            .prio(P_HIGH);
        for n in [
            o.scene_count,
            o.complete_count,
            o.location_count,
            o.character_count,
            o.catalog_count,
        ] {
            f = f.require(n.to_string());
        }
        Ok(vec![f])
    })
}

/// Production > Catalog: the items of the production by category (props, wardrobe, vehicles and
/// the rest) with their status.
pub fn catalog_leaf(ctx: &FetchCtx<'_>, read: &TurnRead, _r: &Resolved) -> Vec<Fact> {
    safe("production catalog", || catalog(ctx, read))
}

fn catalog(ctx: &FetchCtx<'_>, read: &TurnRead) -> AppResult<Vec<Fact>> {
    // How many scenes use each item is counted from the confirmed breakdown elements.
    let mut st = ctx.conn.prepare(
        "SELECT c.id, c.category, c.name, c.status, COALESCE(c.description, ''),
                (SELECT count(DISTINCT b.scene_id) FROM breakdown_element b
                 WHERE b.catalog_item_id = c.id AND b.deleted_at IS NULL AND b.archived = 0
                   AND b.confirmation_state <> 'Rejected')
         FROM catalog_item c WHERE c.deleted_at IS NULL AND c.archived = 0 ORDER BY c.category, c.name LIMIT 400",
    )?;
    let rows: Vec<(String, String, String, String, String, i64)> = st
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
        return Ok(vec![Fact::absent("any catalog items", "the Catalog")]);
    }
    let t = read.answer_type;
    let mut out = Vec::new();
    let mut total = Fact::new("Count", "")
        .f("what", "catalog items")
        .f("value", rows.len().to_string())
        .prio(P_HIGH);
    if t == AnswerType::Count {
        total = total.require(rows.len().to_string());
    }
    out.push(total);
    let mut categories: Vec<(String, usize)> = Vec::new();
    for (_, c, ..) in &rows {
        match categories.iter_mut().find(|(k, _)| k == c) {
            Some(e) => e.1 += 1,
            None => categories.push((c.clone(), 1)),
        }
    }
    let named_category =
        |c: &str| ctx.m.has(&c.to_lowercase()) || ctx.m.has(c.trim_end_matches('s'));
    for (c, n) in &categories {
        let mut f = Fact::new("Count", "")
            .f("what", format!("{} in the catalog", c.to_lowercase()))
            .f("value", n.to_string())
            .prio(P_HIGH);
        if t == AnswerType::Count && named_category(c) {
            f = f.require(n.to_string());
        }
        out.push(f);
    }
    // "used in the most scenes", "used in more than two scenes": the items are ranked by their scenes.
    let by_scenes = ctx.m.stem("scene") && ctx.m.any(&["most", "more", "least", "fewest"]);
    let any_named = categories.iter().any(|(c, _)| named_category(c));
    let pool: Vec<&(String, String, String, String, String, i64)> = rows
        .iter()
        .filter(|r| !any_named || named_category(&r.1))
        .collect();
    let top = pool.iter().map(|r| r.5).max().unwrap_or(0);
    let floor = threshold(&ctx.m);
    let few = pool.len() <= 14;
    let mut shown: Vec<&(String, String, String, String, String, i64)> = pool.clone();
    if by_scenes {
        shown.sort_by_key(|r| std::cmp::Reverse(r.5));
    }
    for r in shown.iter().take(60) {
        let (id, category, name, status, description, scenes) = *r;
        let wanted_by_scenes = by_scenes && (*scenes == top || floor.is_some_and(|n| *scenes > n));
        let mut f = Fact::new("Catalog item", id)
            .f("category", category)
            .f("name", name)
            .f("status", status)
            .f("used in scenes", scenes.to_string())
            .f("description", clip(description, 100))
            .prio(if any_named || wanted_by_scenes {
                P_HIGH
            } else {
                P_LOW + 20
            });
        if matches!(
            t,
            AnswerType::List | AnswerType::Lookup | AnswerType::Compare
        ) && (wanted_by_scenes || (!by_scenes && (few || any_named)))
        {
            f = f.require(name);
        }
        out.push(f);
    }
    Ok(out)
}

/// The number in "more than two scenes", read as a digit or a word up to ten.
fn threshold(m: &crate::modules::ai::dossier::words::Msg) -> Option<i64> {
    const WORDS: [&str; 10] = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    if !m.has("more than") {
        return None;
    }
    m.words().find_map(|w| {
        w.parse::<i64>()
            .ok()
            .or_else(|| WORDS.iter().position(|x| x == &w).map(|i| i as i64 + 1))
    })
}
