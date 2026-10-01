//! Read tools for production planning: breakdown, catalog, locations, cast and
//! crew, the Production Source, visual planning, the schedule, call sheets,
//! sides, reports and the budget. Scene numbers refer to the Production Source
//! draft (or the current draft before production starts).

use std::collections::BTreeMap;

use openframe_domain::AppResult;
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use super::super::queries::{self, join_and};
use super::super::tools::{ToolCtx, ToolOutput, plural, scene_nav};
use super::super::types::*;
use super::reads::{a_bool, a_limit, a_str, a_u32, clip, found, item, not_found, when};
use super::resolve as rv;
use super::schema as sc;

pub fn s_catalog() -> Value {
    sc::obj(
        &[
            ("item", sc::reference()),
            ("category", sc::s(40)),
            ("query", sc::s(200)),
            ("includeArchived", sc::boolean()),
        ],
        &[],
    )
}
pub fn s_locations() -> Value {
    sc::obj(
        &[
            ("location", sc::reference()),
            (
                "status",
                sc::en(&["Idea", "Shortlisted", "Confirmed", "Rejected"]),
            ),
            ("query", sc::s(200)),
        ],
        &[],
    )
}
pub fn s_people() -> Value {
    sc::obj(
        &[
            ("which", sc::en(&["cast", "crew", "all"])),
            ("includeArchived", sc::boolean()),
        ],
        &[],
    )
}
pub fn s_moodboard() -> Value {
    sc::obj(&[("moodboard", sc::reference())], &[])
}
pub fn s_storyboard() -> Value {
    sc::obj(&[("storyboard", sc::reference())], &[])
}
pub fn s_shots() -> Value {
    sc::obj(&[("sceneNumber", sc::scene_number())], &[])
}
pub fn s_day() -> Value {
    sc::obj(
        &[(
            "day",
            sc::sd(40, "day number like \"Day 3\" or a date YYYY-MM-DD"),
        )],
        &["day"],
    )
}
pub fn s_call_sheet() -> Value {
    sc::obj(&[("callSheet", sc::reference())], &[])
}
pub fn s_report() -> Value {
    sc::obj(
        &[
            (
                "type",
                sc::en(&[
                    "scene",
                    "location",
                    "cast_scene",
                    "prop",
                    "schedule",
                    "breakdown_completeness",
                ]),
            ),
            ("filter", sc::s(120)),
        ],
        &["type"],
    )
}

/// Draft production tools use, and a scene of it.
fn prod_draft(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<queries::DraftRef> {
    rv::production_draft(ctx.conn, ctx.scope, a_str(a, "draft").as_deref())
}

// ------------------------------------------------------------------ breakdown

pub fn breakdown_scene(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = prod_draft(ctx, a)?;
    let s = rv::scene_or_scope(c, ctx.scope, &d, a_u32(a, "sceneNumber"))?;
    let mut stmt = c.prepare(
        "SELECT category, display_name, confirmation_state, archived, COALESCE(notes,'') FROM breakdown_element
         WHERE scene_id=?1 AND deleted_at IS NULL AND confirmation_state <> 'Rejected' ORDER BY category, display_name",
    )?;
    let rows: Vec<(String, String, String, bool, String)> = stmt
        .query_map([&s.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let state: Option<(bool, bool, Option<i64>)> = c
        .query_row(
            "SELECT complete, needs_breakdown, reviewed_at FROM production_scene_state WHERE scene_lineage_id=?1",
            [&s.lineage_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let confirmed = rows.iter().filter(|r| r.2 != "Suggested").count();
    let suggested = rows.len() - confirmed;
    let mut o = ToolOutput::exact(format!(
        "{} ({}): {} and {} awaiting review.",
        s.label(),
        d.label(),
        plural(confirmed, "confirmed element", "confirmed elements"),
        plural(suggested, "suggestion", "suggestions")
    ));
    if let Some((complete, needs, reviewed)) = state {
        o = o.detail(format!(
            "Breakdown {}{}{}.",
            if complete {
                "marked complete"
            } else {
                "not marked complete"
            },
            if needs {
                " · changed in the script, needs review"
            } else {
                ""
            },
            reviewed
                .map(|r| format!(" · reviewed {}", when(r)))
                .unwrap_or_default()
        ));
    }
    for (cat, name, st, archived, notes) in rows {
        o.items.push(item(
            format!("{cat}: {name}"),
            Some(format!(
                "{st}{}{}",
                if archived { " · archived" } else { "" },
                if notes.trim().is_empty() {
                    String::new()
                } else {
                    format!(" · {}", clip(&notes, 120))
                }
            )),
            Some(
                NavTarget::to("breakdown")
                    .with("sceneId", &s.id)
                    .with("draftId", &d.id),
            ),
        ));
    }
    Ok(o.prov("Production Source", d.label())
        .prov("Scene", s.label()))
}

// ------------------------------------------------------------------ catalog & locations

pub fn production_catalog(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "item") {
        let it = found(ctx, &rv::CATALOG_ITEM, &r)?;
        let (cat, status, desc, notes, contact, archived): (String, String, Option<String>, Option<String>, Option<String>, bool) = c.query_row(
            "SELECT category, status, description, notes, contact, archived FROM catalog_item WHERE id=?1",
            [&it.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )?;
        let mut o = ToolOutput::exact(format!(
            "{} — {cat}, {status}{}.",
            it.label,
            if archived { ", archived" } else { "" }
        ));
        for (label, v) in [
            ("Description", desc),
            ("Notes", notes),
            ("Contact", contact),
        ] {
            if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
                o = o.detail(format!("{label}: {}", clip(&v, 500)));
            }
        }
        let aliases: Vec<String> = c
            .prepare("SELECT alias FROM catalog_alias WHERE catalog_item_id=?1 ORDER BY alias")?
            .query_map([&it.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        if !aliases.is_empty() {
            o = o.detail(format!("Also called: {}", aliases.join(", ")));
        }
        if let Ok(d) = rv::production_draft(c, ctx.scope, None) {
            let used: Vec<String> = c
                .prepare("SELECT DISTINCT scene_id FROM breakdown_element WHERE catalog_item_id=?1 AND deleted_at IS NULL AND confirmation_state IN ('Confirmed','Manual')")?
                .query_map([&it.id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            let nums: Vec<String> = queries::scenes(c, &d.id)?
                .iter()
                .filter(|s| used.contains(&s.id))
                .map(|s| s.number.to_string())
                .collect();
            o = o.detail(if nums.is_empty() {
                "Not used in any scene yet.".to_string()
            } else {
                format!("Used in scenes {} ({}).", join_and(&nums), d.label())
            });
        }
        o.nav = Some(
            NavTarget::to("production")
                .with_sub("catalog")
                .with("itemId", &it.id),
        );
        return Ok(o.prov("Catalog item", it.label));
    }
    let q = a_str(a, "query").map(|q| format!("%{}%", q.replace('%', "")));
    let mut stmt = c.prepare(
        "SELECT i.id, i.category, i.name, i.status, i.archived,
                (SELECT count(DISTINCT scene_id) FROM breakdown_element b WHERE b.catalog_item_id = i.id AND b.deleted_at IS NULL)
         FROM catalog_item i WHERE i.deleted_at IS NULL AND (?1 = 1 OR i.archived = 0) AND (?2 IS NULL OR i.category = ?2)
           AND (?3 IS NULL OR i.name LIKE ?3 OR i.id IN (SELECT catalog_item_id FROM catalog_alias WHERE alias LIKE ?3))
         ORDER BY i.category, i.name LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, String, bool, i64)> = stmt
        .query_map(
            params![a_bool(a, "includeArchived") as i64, a_str(a, "category"), q],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{} in the Production Catalog.",
        plural(rows.len(), "item", "items")
    ));
    for (id, cat, name, status, archived, scenes) in rows {
        o.items.push(item(
            name,
            Some(format!(
                "{cat} · {status} · {}{}",
                plural(scenes as usize, "scene", "scenes"),
                if archived { " · archived" } else { "" }
            )),
            Some(
                NavTarget::to("production")
                    .with_sub("catalog")
                    .with("itemId", &id),
            ),
        ));
    }
    Ok(o.prov("Scope", "Production Catalog"))
}

pub fn production_locations(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "location") {
        let loc = found(ctx, &rv::LOCATION, &r)?;
        let (address, contact, status, notes, archived, replacement): (Option<String>, Option<String>, String, String, bool, Option<String>) = c.query_row(
            "SELECT address, contact, status, notes_json, archived, replacement_location_id FROM location WHERE id=?1",
            [&loc.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )?;
        let mut o = ToolOutput::exact(format!(
            "{} — {status}{}.",
            loc.label,
            if archived { ", archived" } else { "" }
        ));
        for (label, v) in [("Address", address), ("Contact", contact)] {
            if let Some(v) = v.filter(|v| !v.trim().is_empty()) {
                o = o.detail(format!("{label}: {}", clip(&v, 300)));
            }
        }
        if let Ok(Value::Object(n)) = serde_json::from_str::<Value>(&notes) {
            for (k, v) in n {
                if let Some(t) = v.as_str().filter(|t| !t.trim().is_empty()) {
                    o.items
                        .push(item(format!("{k} notes"), Some(clip(t, 400)), None));
                }
            }
        }
        let photos = queries::count(
            c,
            "SELECT count(*) FROM location_photo WHERE location_id=?1 AND deleted_at IS NULL",
            [&loc.id],
        )?;
        o = o.detail(format!("{}.", plural(photos as usize, "photo", "photos")));
        if let Some(r) = replacement {
            o = o.detail(format!(
                "Replaced by {}.",
                rv::column(c, "location", "name", &r)?
            ));
        }
        let items: Vec<String> = c
            .prepare("SELECT id FROM catalog_item WHERE location_id=?1 AND deleted_at IS NULL")?
            .query_map([&loc.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        if let Ok(d) = rv::production_draft(c, ctx.scope, None)
            && !items.is_empty()
        {
            let n = queries::count_in(
                c,
                "SELECT count(DISTINCT scene_id) FROM breakdown_element WHERE deleted_at IS NULL AND catalog_item_id IN",
                &items,
            )?;
            o = o.detail(format!(
                "Used by {} in the breakdown of {}.",
                plural(n as usize, "scene", "scenes"),
                d.label()
            ));
        }
        o.nav = Some(
            NavTarget::to("production")
                .with_sub("locations")
                .with("locationId", &loc.id),
        );
        return Ok(o.prov("Location", loc.label));
    }
    let q = a_str(a, "query").map(|q| format!("%{}%", q.replace('%', "")));
    let mut stmt = c.prepare(
        "SELECT id, name, status, COALESCE(address,'') FROM location WHERE deleted_at IS NULL AND archived=0
           AND (?1 IS NULL OR status=?1) AND (?2 IS NULL OR name LIKE ?2 OR address LIKE ?2) ORDER BY name LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, String)> = stmt
        .query_map(params![a_str(a, "status"), q], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{}.",
        plural(rows.len(), "production location", "production locations")
    ));
    for (id, name, status, address) in rows {
        o.items.push(item(
            name,
            Some(if address.is_empty() {
                status
            } else {
                format!("{status} · {}", clip(&address, 120))
            }),
            Some(
                NavTarget::to("production")
                    .with_sub("locations")
                    .with("locationId", &id),
            ),
        ));
    }
    Ok(o.prov("Scope", "Production locations"))
}

pub fn cast_and_crew(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let which = a_str(a, "which").unwrap_or_else(|| "all".into());
    let archived = a_bool(a, "includeArchived") as i64;
    let mut o = ToolOutput::exact(String::new());
    let mut cast_n = 0usize;
    let mut crew_n = 0usize;
    if which != "crew" {
        let mut st = c.prepare(
            "SELECT m.person_name, COALESCE(ch.name, m.character_name, ''), m.is_primary, COALESCE(m.availability_notes,''), m.archived
             FROM cast_member m LEFT JOIN story_character ch ON ch.id = m.character_id AND ch.deleted_at IS NULL
             WHERE m.deleted_at IS NULL AND (?1 = 1 OR m.archived = 0) ORDER BY 2, m.person_name LIMIT 150",
        )?;
        for row in st.query_map([archived], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, bool>(4)?,
            ))
        })? {
            let (p, ch, primary, avail, arch) = row?;
            cast_n += 1;
            o.items.push(item(
                format!("Cast: {p}"),
                Some(format!(
                    "{}{}{}{}",
                    if ch.is_empty() {
                        "no character".to_string()
                    } else {
                        format!("plays {ch}")
                    },
                    if primary { "" } else { " (alternate)" },
                    if avail.trim().is_empty() {
                        String::new()
                    } else {
                        format!(" · {}", clip(&avail, 100))
                    },
                    if arch { " · archived" } else { "" }
                )),
                Some(NavTarget::to("production").with_sub("cast-crew")),
            ));
        }
    }
    if which != "cast" {
        let mut st = c.prepare(
            "SELECT person_name, role, COALESCE(department,''), archived FROM crew_member
             WHERE deleted_at IS NULL AND (?1 = 1 OR archived = 0) ORDER BY department, role, person_name LIMIT 150",
        )?;
        for row in st.query_map([archived], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, bool>(3)?,
            ))
        })? {
            let (p, role, dept, arch) = row?;
            crew_n += 1;
            o.items.push(item(
                format!("Crew: {p}"),
                Some(format!(
                    "{role}{}{}",
                    if dept.is_empty() {
                        String::new()
                    } else {
                        format!(" · {dept}")
                    },
                    if arch { " · archived" } else { "" }
                )),
                Some(NavTarget::to("production").with_sub("cast-crew")),
            ));
        }
    }
    o.content = match which.as_str() {
        "cast" => format!("{}.", plural(cast_n, "cast member", "cast members")),
        "crew" => format!("{}.", plural(crew_n, "crew member", "crew members")),
        _ => format!(
            "{} and {}.",
            plural(cast_n, "cast member", "cast members"),
            plural(crew_n, "crew member", "crew members")
        ),
    };
    Ok(o.prov("Scope", "Cast & Crew"))
}

pub fn production_source(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let drafts = queries::list_drafts(c)?;
    let Some((src_id, draft_id)) = queries::production_source_draft(c)? else {
        let mut o = ToolOutput::exact("No Production Source is selected yet.");
        for d in drafts {
            o.items.push(item(
                d.label(),
                Some(format!(
                    "{}{}",
                    d.status,
                    if d.is_current { " · current" } else { "" }
                )),
                None,
            ));
        }
        return Ok(o.prov("Scope", "Production"));
    };
    let src = queries::draft_by_id(c, &draft_id)?
        .ok_or_else(|| not_found("The Production Source draft is gone."))?;
    let (reason, at): (Option<String>, i64) = c.query_row(
        "SELECT selection_reason, selected_at FROM production_source WHERE id=?1",
        [&src_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let mut o = ToolOutput::exact(format!(
        "Production follows {} (chosen {}).",
        src.label(),
        when(at)
    ));
    if let Some(r) = reason.filter(|r| !r.trim().is_empty()) {
        o = o.detail(format!("Reason: {}", clip(&r, 200)));
    }
    if let Some(cur) = queries::current_draft(c)?
        && cur.id != src.id
    {
        let a = queries::scenes(c, &src.id)?;
        let b = queries::scenes(c, &cur.id)?;
        let la: Vec<&String> = a.iter().map(|s| &s.lineage_id).collect();
        let lb: Vec<&String> = b.iter().map(|s| &s.lineage_id).collect();
        let added = b.iter().filter(|s| !la.contains(&&s.lineage_id)).count();
        let removed = a.iter().filter(|s| !lb.contains(&&s.lineage_id)).count();
        let mut changed = 0usize;
        for s in &b {
            if let Some(old) = a.iter().find(|x| x.lineage_id == s.lineage_id)
                && queries::scene_fingerprint(c, &old.id)? != queries::scene_fingerprint(c, &s.id)?
            {
                changed += 1;
            }
        }
        o = o.detail(format!(
            "The current draft is {}: updating Production would add {}, remove {} and flag {} for review.",
            cur.label(),
            plural(added, "scene", "scenes"),
            plural(removed, "scene", "scenes"),
            plural(changed, "changed scene", "changed scenes")
        ));
    }
    for d in drafts {
        o.items.push(item(
            d.label(),
            Some(format!(
                "{}{}{}",
                d.status,
                if d.is_current { " · current" } else { "" },
                if d.id == src.id {
                    " · Production Source"
                } else {
                    ""
                }
            )),
            None,
        ));
    }
    Ok(o.prov("Production Source", src.label()))
}

// ------------------------------------------------------------------ visual planning

pub fn visual_planning(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::production_draft(c, ctx.scope, None)?;
    let scenes = queries::scenes(c, &d.id)?;
    let mut o = ToolOutput::exact(String::new());
    let (mut with_shots, mut review) = (0usize, 0usize);
    for s in scenes.iter().take(200) {
        let shots = queries::count(
            c,
            "SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
            [&s.lineage_id],
        )?;
        let boards = queries::count(
            c,
            "SELECT count(*) FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
            [&s.lineage_id],
        )?;
        let moods = queries::count(
            c,
            "SELECT count(*) FROM moodboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
            [&s.lineage_id],
        )?;
        let flagged = queries::count(
            c,
            "SELECT (SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL AND needs_review=1)
                  + (SELECT count(*) FROM storyboard WHERE scene_lineage_id=?1 AND deleted_at IS NULL AND needs_review=1)",
            [&s.lineage_id],
        )?;
        if shots > 0 {
            with_shots += 1;
        }
        if flagged > 0 {
            review += 1;
        }
        if shots + boards + moods + flagged > 0 {
            o.items.push(item(
                s.label(),
                Some(format!(
                    "{} · {} · {}{}",
                    plural(shots as usize, "shot", "shots"),
                    plural(boards as usize, "storyboard", "storyboards"),
                    plural(moods as usize, "moodboard", "moodboards"),
                    if flagged > 0 {
                        " · needs review after a script change"
                    } else {
                        ""
                    }
                )),
                Some(
                    NavTarget::to("production")
                        .with_sub("shots")
                        .with("sceneId", &s.id),
                ),
            ));
        }
    }
    o.content = format!(
        "{} of {} in {} have shots; {} need planning review.",
        with_shots,
        plural(scenes.len(), "scene", "scenes"),
        d.label(),
        plural(review, "scene", "scenes")
    );
    Ok(o.prov("Draft", d.label()))
}

pub fn moodboards(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "moodboard") {
        let b = found(ctx, &rv::MOODBOARD, &r)?;
        let notes = rv::column(c, "moodboard", "notes", &b.id)?;
        let mut o = ToolOutput::exact(format!("Moodboard “{}”.", b.label));
        if !notes.trim().is_empty() {
            o = o.detail(format!("Notes: {}", clip(&notes, 600)));
        }
        let mut st = c.prepare(
            "SELECT kind, COALESCE(caption,''), COALESCE(body,''), COALESCE(url,''), COALESCE(link_title,''), is_private
             FROM moodboard_item WHERE moodboard_id=?1 AND deleted_at IS NULL ORDER BY z, id LIMIT 80",
        )?;
        for row in st.query_map([&b.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })? {
            let (kind, caption, body, url, title, private) = row?;
            let label = match kind.as_str() {
                "note" => format!("Note: {}", clip(&body, 200)),
                "link" => format!(
                    "Link: {}",
                    if title.is_empty() {
                        clip(&url, 120)
                    } else {
                        title
                    }
                ),
                _ => format!(
                    "Image{}",
                    if caption.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", clip(&caption, 120))
                    }
                ),
            };
            o.items
                .push(item(label, private.then(|| "internal".to_string()), None));
        }
        o.nav = Some(
            NavTarget::to("production")
                .with_sub("moodboards")
                .with("moodboardId", &b.id),
        );
        return Ok(o.prov("Moodboard", b.label));
    }
    let mut st = c.prepare(
        "SELECT m.id, m.name, COALESCE(s.heading,''), (SELECT count(*) FROM moodboard_item i WHERE i.moodboard_id=m.id AND i.deleted_at IS NULL)
         FROM moodboard m LEFT JOIN screenplay_scene s ON s.id = m.scene_id WHERE m.deleted_at IS NULL ORDER BY m.position LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, i64)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{}.",
        plural(rows.len(), "moodboard", "moodboards")
    ));
    for (id, name, heading, n) in rows {
        o.items.push(item(
            name,
            Some(format!(
                "{}{}",
                plural(n as usize, "item", "items"),
                if heading.is_empty() {
                    String::new()
                } else {
                    format!(" · for {heading}")
                }
            )),
            Some(
                NavTarget::to("production")
                    .with_sub("moodboards")
                    .with("moodboardId", &id),
            ),
        ));
    }
    Ok(o.prov("Scope", "Moodboards"))
}

pub fn storyboards(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "storyboard") {
        let b = found(ctx, &rv::STORYBOARD, &r)?;
        let heading = rv::column(c, "storyboard", "scene_heading", &b.id)?;
        let mut o = ToolOutput::exact(format!(
            "Storyboard “{}”{}.",
            b.label,
            if heading.is_empty() {
                " (standalone)".to_string()
            } else {
                format!(" for {heading}")
            }
        ));
        let mut st = c.prepare(
            "SELECT description, COALESCE(framing,''), COALESCE(movement,''), COALESCE(angle,''), COALESCE(sound_note,''), duration_ms, shot_id, visual_kind
             FROM storyboard_panel WHERE storyboard_id=?1 AND deleted_at IS NULL ORDER BY position, id LIMIT 80",
        )?;
        let mut i = 0;
        for row in st.query_map([&b.id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, String>(7)?,
            ))
        })? {
            let (desc, framing, movement, angle, sound, dur, shot, visual) = row?;
            i += 1;
            let mut det: Vec<String> = [framing, movement, angle]
                .into_iter()
                .filter(|x| !x.is_empty())
                .collect();
            if !sound.is_empty() {
                det.push(format!("sound: {}", clip(&sound, 60)));
            }
            if let Some(ms) = dur {
                det.push(format!("{} s", ms / 1000));
            }
            if shot.is_some() {
                det.push("linked to a shot".into());
            }
            if visual == "placeholder" {
                det.push("no image yet".into());
            }
            o.items.push(item(
                format!("Panel {i}: {}", clip(&desc, 160)),
                Some(det.join(" · ")),
                None,
            ));
        }
        o.nav = Some(
            NavTarget::to("production")
                .with_sub("storyboards")
                .with("storyboardId", &b.id),
        );
        return Ok(o.prov("Storyboard", b.label));
    }
    let mut st = c.prepare(
        "SELECT b.id, b.name, COALESCE(b.scene_heading,''), b.needs_review, (SELECT count(*) FROM storyboard_panel p WHERE p.storyboard_id=b.id AND p.deleted_at IS NULL)
         FROM storyboard b WHERE b.deleted_at IS NULL ORDER BY b.position LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, bool, i64)> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{}.",
        plural(rows.len(), "storyboard", "storyboards")
    ));
    for (id, name, heading, review, n) in rows {
        o.items.push(item(
            name,
            Some(format!(
                "{}{}{}",
                plural(n as usize, "panel", "panels"),
                if heading.is_empty() {
                    " · standalone".to_string()
                } else {
                    format!(" · {heading}")
                },
                if review { " · needs review" } else { "" }
            )),
            Some(
                NavTarget::to("production")
                    .with_sub("storyboards")
                    .with("storyboardId", &id),
            ),
        ));
    }
    Ok(o.prov("Scope", "Storyboards"))
}

pub fn shot_list(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let d = rv::production_draft(c, ctx.scope, None)?;
    let scene = match a_u32(a, "sceneNumber") {
        Some(n) => Some(rv::scene(c, &d, n)?),
        None => ctx
            .scope
            .scene
            .clone()
            .filter(|_| ctx.scope.draft.as_ref().is_some_and(|x| x.id == d.id)),
    };
    let Some(s) = scene else {
        let scenes = queries::scenes(c, &d.id)?;
        let mut o = ToolOutput::exact(String::new());
        let mut total = 0;
        for s in &scenes {
            let n = queries::count(
                c,
                "SELECT count(*) FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL",
                [&s.lineage_id],
            )?;
            if n > 0 {
                total += n;
                o.items.push(item(
                    s.label(),
                    Some(plural(n as usize, "shot", "shots")),
                    Some(scene_nav(&d, s)),
                ));
            }
        }
        o.content = format!(
            "{} across {} of {}.",
            plural(total as usize, "shot", "shots"),
            plural(o.items.len(), "scene", "scenes"),
            d.label()
        );
        return Ok(o.prov("Draft", d.label()));
    };
    let mut st = c.prepare(
        "SELECT description, COALESCE(size,''), COALESCE(movement,''), COALESCE(angle,''), COALESCE(lens,''), characters_json, COALESCE(camera_notes,''), needs_review
         FROM shot WHERE scene_lineage_id=?1 AND deleted_at IS NULL ORDER BY position, id LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, String, String, String, String, bool)> = st
        .query_map([&s.lineage_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{} has {}.",
        s.label(),
        plural(rows.len(), "shot", "shots")
    ));
    for (i, (desc, size, mv, angle, lens, chars, notes, review)) in rows.into_iter().enumerate() {
        let mut det: Vec<String> = [size, mv, angle, lens]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect();
        let chars: Vec<String> = serde_json::from_str(&chars).unwrap_or_default();
        if !chars.is_empty() {
            det.push(chars.join(", "));
        }
        if !notes.trim().is_empty() {
            det.push(clip(&notes, 80));
        }
        if review {
            det.push("needs review".into());
        }
        o.items.push(item(
            format!("Shot {}: {}", i + 1, clip(&desc, 160)),
            Some(det.join(" · ")),
            None,
        ));
    }
    o.nav = Some(
        NavTarget::to("production")
            .with_sub("shots")
            .with("sceneId", &s.id),
    );
    Ok(o.prov("Draft", d.label()).prov("Scene", s.label()))
}

// ------------------------------------------------------------------ schedule

pub fn schedule_overview(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let (id, name) = rv::schedule(c)?;
    let (status, strict, minutes): (String, bool, i64) = c.query_row(
        "SELECT status, strict_validation, day_duration_minutes FROM shooting_schedule WHERE id=?1",
        [&id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut st = c.prepare(
        "SELECT d.id, d.shoot_date, d.is_off_day,
                (SELECT count(*) FROM schedule_strip s WHERE s.day_id=d.id AND s.deleted_at IS NULL AND s.archived=0),
                (SELECT COALESCE(sum(COALESCE(s.page_eighths_override, s.source_page_eighths)),0) FROM schedule_strip s WHERE s.day_id=d.id AND s.deleted_at IS NULL AND s.archived=0),
                (SELECT COALESCE(sum(s.estimated_minutes),0) FROM schedule_strip s WHERE s.day_id=d.id AND s.deleted_at IS NULL AND s.archived=0)
         FROM shooting_day d WHERE d.schedule_id=?1 AND d.deleted_at IS NULL ORDER BY d.position, d.id",
    )?;
    let days: Vec<(String, Option<String>, bool, i64, i64, i64)> = st
        .query_map([&id], |r| {
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
    let unscheduled: Vec<(String, String)> = c
        .prepare("SELECT source_heading, source_state FROM schedule_strip WHERE schedule_id=?1 AND day_id IS NULL AND deleted_at IS NULL AND archived=0 ORDER BY position")?
        .query_map([&id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let changed = queries::count(
        c,
        "SELECT count(*) FROM schedule_strip WHERE schedule_id=?1 AND deleted_at IS NULL AND archived=0 AND source_state IN ('Changed','Removed')",
        [&id],
    )?;
    let mut o = ToolOutput::exact(format!(
        "{name} ({status}): {}, {} unscheduled.",
        plural(days.len(), "day", "days"),
        plural(unscheduled.len(), "scene", "scenes")
    ));
    o = o.detail(format!(
        "Strict validation {}; default day {minutes} min.",
        if strict { "on" } else { "off" }
    ));
    if changed > 0 {
        o = o.detail(format!(
            "{} changed or cut in the script and need attention.",
            plural(changed as usize, "scheduled scene", "scheduled scenes")
        ));
    }
    for (i, (did, date, off, scenes, eighths, est)) in days.iter().enumerate() {
        o.items.push(item(
            format!(
                "Day {}{}",
                i + 1,
                date.as_ref().map(|d| format!(" ({d})")).unwrap_or_default()
            ),
            Some(if *off {
                "Off day".to_string()
            } else {
                format!(
                    "{} · {} {}/8 pages · {} min estimated",
                    plural(*scenes as usize, "scene", "scenes"),
                    eighths / 8,
                    eighths % 8,
                    est
                )
            }),
            Some(
                NavTarget::to("production")
                    .with_sub("schedule")
                    .with("dayId", did),
            ),
        ));
    }
    // Grouping hint: unscheduled scenes sharing a location (same parsed heading location).
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();
    for (h, _) in &unscheduled {
        let p = queries::parse_heading(h);
        if !p.location.is_empty() {
            *groups
                .entry(
                    format!("{} {}", p.location, p.time.unwrap_or_default())
                        .trim()
                        .to_string(),
                )
                .or_default() += 1;
        }
    }
    let hints: Vec<String> = groups
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(k, n)| format!("{k} ({n})"))
        .collect();
    if !hints.is_empty() {
        o = o.detail(format!(
            "Unscheduled scenes that could shoot together: {}.",
            hints.join(", ")
        ));
    }
    Ok(o.prov("Schedule", name))
}

pub fn schedule_day(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let day = rv::day(c, &a_str(a, "day").unwrap_or_default())?;
    let (notes, planned, off): (Option<String>, Option<i64>, bool) = c.query_row(
        "SELECT notes, planned_minutes, is_off_day FROM shooting_day WHERE id=?1",
        [&day.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let mut o = ToolOutput::exact(format!(
        "{}{}.",
        day.label,
        if off { " is an off day" } else { "" }
    ));
    if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
        o = o.detail(format!("Notes: {}", clip(&n, 400)));
    }
    if let Some(p) = planned {
        o = o.detail(format!("Target length: {p} min."));
    }
    // Scenes and breaks in day order.
    let mut rows: Vec<(i64, String, Option<String>)> = Vec::new();
    let mut st = c.prepare(
        "SELECT position, source_heading, scene_lineage_id, COALESCE(page_eighths_override, source_page_eighths), estimated_minutes, source_state
         FROM schedule_strip WHERE day_id=?1 AND deleted_at IS NULL AND archived=0",
    )?;
    let mut lineages = Vec::new();
    let src = rv::production_draft(c, ctx.scope, None).ok();
    let numbers: BTreeMap<String, usize> = match &src {
        Some(d) => queries::scenes(c, &d.id)?
            .into_iter()
            .map(|s| (s.lineage_id, s.number))
            .collect(),
        None => BTreeMap::new(),
    };
    for row in st.query_map([&day.id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, String>(5)?,
        ))
    })? {
        let (pos, heading, lineage, eighths, est, state) = row?;
        let n = numbers
            .get(&lineage)
            .map(|n| format!("Scene {n}: "))
            .unwrap_or_default();
        lineages.push(lineage);
        rows.push((
            pos,
            format!("{n}{heading}"),
            Some(format!(
                "{} {}/8 pages{}{}",
                eighths / 8,
                eighths % 8,
                est.map(|m| format!(" · {m} min")).unwrap_or_default(),
                if state == "Current" {
                    String::new()
                } else {
                    format!(" · {state} in script")
                }
            )),
        ));
    }
    let mut st = c.prepare(
        "SELECT position, marker_type, label, at_time, duration_minutes FROM schedule_marker WHERE day_id=?1 AND deleted_at IS NULL",
    )?;
    for row in st.query_map([&day.id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, Option<i64>>(4)?,
        ))
    })? {
        let (pos, kind, label, at, dur) = row?;
        rows.push((
            pos,
            format!("Break: {label}"),
            Some(format!(
                "{kind}{}{}",
                at.map(|t| format!(" at {t}")).unwrap_or_default(),
                dur.map(|d| format!(" · {d} min")).unwrap_or_default()
            )),
        ));
    }
    rows.sort_by_key(|r| r.0);
    let scene_count = lineages.len();
    for (_, label, det) in rows {
        o.items.push(item(
            label,
            det,
            Some(
                NavTarget::to("production")
                    .with_sub("schedule")
                    .with("dayId", &day.id),
            ),
        ));
    }
    if !lineages.is_empty() {
        let cast = queries::count_in(
            c,
            "SELECT count(DISTINCT display_name) FROM breakdown_element WHERE deleted_at IS NULL AND category='Cast' AND confirmation_state IN ('Confirmed','Manual') AND scene_lineage_id IN",
            &lineages,
        )?;
        let mut names: Vec<String> = Vec::new();
        let marks = vec!["?"; lineages.len()].join(",");
        let sql = format!(
            "SELECT DISTINCT display_name FROM breakdown_element WHERE deleted_at IS NULL AND category='Cast' AND confirmation_state IN ('Confirmed','Manual') AND scene_lineage_id IN ({marks}) ORDER BY display_name LIMIT 30"
        );
        let mut st = c.prepare(&sql)?;
        for r in st.query_map(rusqlite::params_from_iter(lineages.iter()), |r| {
            r.get::<_, String>(0)
        })? {
            names.push(r?);
        }
        o = o.detail(format!(
            "Cast needed ({}): {}.",
            cast,
            if names.is_empty() {
                "none confirmed in the breakdown".into()
            } else {
                join_and(&names)
            }
        ));
    }
    o.content = format!(
        "{}: {}.",
        day.label,
        if off {
            "off day".to_string()
        } else {
            plural(scene_count, "scene", "scenes")
        }
    );
    Ok(o.prov("Shooting day", day.label))
}

pub fn call_sheets(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    if let Some(r) = a_str(a, "callSheet") {
        let cs = found(ctx, &rv::CALL_SHEET, &r)?;
        let (status, revision, needs_refresh, doc): (String, i64, bool, String) = c.query_row(
            "SELECT status, revision, needs_refresh, document_json FROM call_sheet WHERE id=?1",
            [&cs.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let doc: Value = serde_json::from_str(&doc).unwrap_or(Value::Null);
        let text = |p: &str| {
            doc.pointer(p)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        let mut o = ToolOutput::exact(format!(
            "{} — {} (revision {revision}, {status}){}.",
            cs.label,
            text("/dayLabel"),
            if needs_refresh {
                "; the schedule changed since it was prepared"
            } else {
                ""
            }
        ));
        for (label, p) in [
            ("Date", "/date"),
            ("Crew call", "/crewCall"),
            ("Day notes", "/dayNotes"),
            ("Parking", "/practical/parking"),
            ("Meeting point", "/practical/meetingPoint"),
            ("Travel notes", "/practical/travelNotes"),
            ("Meal break", "/practical/mealBreak"),
            ("Emergency contact", "/practical/emergencyContact"),
            ("Production notes", "/practical/productionNotes"),
            ("Weather", "/optional/weather"),
            ("Special notes", "/optional/specialNotes"),
        ] {
            let v = text(p);
            if !v.trim().is_empty() {
                o.items.push(item(label, Some(clip(&v, 300)), None));
            }
        }
        for key in ["scenes", "cast", "locations"] {
            if let Some(arr) = doc.get(key).and_then(|v| v.as_array()) {
                o = o.detail(format!("{}: {}", key, arr.len()));
            }
        }
        o.nav = Some(NavTarget::to("callsheets").with("callSheetId", &cs.id));
        return Ok(o.prov("Call sheet", cs.label));
    }
    let mut st = c.prepare(
        "SELECT id, title, status, revision, needs_refresh, shoot_day_id FROM call_sheet WHERE deleted_at IS NULL ORDER BY created_at, revision LIMIT 100",
    )?;
    let rows: Vec<(String, String, String, i64, bool, String)> = st
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
    let mut o = ToolOutput::exact(format!(
        "{}.",
        plural(rows.len(), "call sheet", "call sheets")
    ));
    for (id, title, status, rev, refresh, day) in rows {
        let date = rv::column(c, "shooting_day", "shoot_date", &day)?;
        o.items.push(item(
            title,
            Some(format!(
                "{status} · revision {rev}{}{}",
                if date.is_empty() {
                    String::new()
                } else {
                    format!(" · {date}")
                },
                if refresh { " · needs refresh" } else { "" }
            )),
            Some(NavTarget::to("callsheets").with("callSheetId", &id)),
        ));
    }
    Ok(o.prov("Scope", "Call sheets"))
}

pub fn sides_and_reports(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let sides: Vec<(String, String, i64)> = c
        .prepare("SELECT title, source_label, created_at FROM side WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT 40")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let reports: Vec<(String, String, i64)> = c
        .prepare("SELECT title, report_type, generated_at FROM production_report WHERE deleted_at IS NULL ORDER BY generated_at DESC LIMIT 40")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut o = ToolOutput::exact(format!(
        "{} and {}.",
        plural(sides.len(), "saved sides document", "saved sides documents"),
        plural(reports.len(), "saved report", "saved reports")
    ));
    for (t, src, at) in sides {
        o.items.push(item(
            format!("Sides: {t}"),
            Some(format!("{src} · {}", when(at))),
            Some(NavTarget::to("production").with_sub("sides")),
        ));
    }
    for (t, kind, at) in reports {
        o.items.push(item(
            format!("Report: {t}"),
            Some(format!("{} · {}", kind.replace('_', " "), when(at))),
            None,
        ));
    }
    Ok(o.prov("Scope", "Sides & reports"))
}

pub fn production_report(ctx: &ToolCtx<'_>, a: &Value) -> AppResult<ToolOutput> {
    let t = a_str(a, "type").unwrap_or_default();
    let data =
        crate::modules::schedule::docs::build_report(ctx.conn, &t, a_str(a, "filter").as_deref())?;
    let mut o = ToolOutput::exact(format!(
        "{} ({}): {}.",
        data.title,
        data.source_label,
        plural(data.rows.len(), "row", "rows")
    ));
    if let Some(n) = &data.note {
        o = o.detail(n.clone());
    }
    o = o.detail(format!("Columns: {}", data.columns.join(" | ")));
    for row in data.rows.iter().take(a_limit(a, 60)) {
        o.items.push(item(clip(&row.join(" | "), 280), None, None));
    }
    Ok(o.prov("Report", data.title))
}

pub fn budget_summary(ctx: &ToolCtx<'_>, _: &Value) -> AppResult<ToolOutput> {
    let c = ctx.conn;
    let b = rv::budget(c)?;
    let (planned, mode, value, notes): (Option<i64>, String, i64, Option<String>) = c.query_row(
        "SELECT planned_total, contingency_mode, contingency_value, notes FROM budget_snapshot WHERE id=?1",
        [&b.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let money = |minor: i64| format!("{} {}.{:02}", b.label, minor / 100, minor % 100);
    let lines: Vec<(String, String, i64)> = c
        .prepare("SELECT category, description, amount FROM budget_line WHERE budget_id=?1 AND deleted_at IS NULL ORDER BY category, position")?
        .query_map([&b.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let total: i64 = lines.iter().map(|l| l.2).sum();
    let mut by_cat: BTreeMap<String, i64> = BTreeMap::new();
    for (cat, _, amt) in &lines {
        *by_cat.entry(cat.clone()).or_default() += amt;
    }
    let mut o = ToolOutput::exact(format!(
        "Budget ({}): {} entered across {}{}.",
        b.label,
        money(total),
        plural(lines.len(), "line", "lines"),
        planned
            .map(|p| format!("; planned total {}", money(p)))
            .unwrap_or_default()
    ));
    o = o.detail(if mode == "percent" {
        format!(
            "Contingency {}.{:02}% of the planned total.",
            value / 100,
            value % 100
        )
    } else {
        format!("Contingency {}.", money(value))
    });
    if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
        o = o.detail(format!("Notes: {}", clip(&n, 300)));
    }
    for (cat, amt) in &by_cat {
        o = o.detail(format!("{cat}: {}", money(*amt)));
    }
    for (cat, desc, amt) in lines.iter().take(80) {
        o.items.push(item(
            desc.clone(),
            Some(format!("{cat} · {}", money(*amt))),
            None,
        ));
    }
    let snaps = queries::count(
        c,
        "SELECT count(*) FROM budget_snapshot WHERE is_current=0 AND deleted_at IS NULL",
        [],
    )?;
    o = o.detail(format!(
        "{}.",
        plural(snaps as usize, "saved snapshot", "saved snapshots")
    ));
    Ok(o.prov("Scope", "Budget"))
}
