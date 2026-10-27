//! Reads the project database into an [`EntityIndex`]. Read-only. Every query names the real columns
//! of `migrations/project/*.sql`; a deleted row (`deleted_at`) is never indexed.

use std::collections::{BTreeSet, HashMap};

use openframe_domain::AppResult;
use rusqlite::Connection;

use super::{Entity, EntityIndex, Kind, Relation, ScreenplayInfo};
use crate::modules::ai::queries::normalize_cue;

/// Longest card or note title kept as a name.
const NAME_CHARS: usize = 80;

pub(super) fn build(c: &Connection) -> AppResult<EntityIndex> {
    let mut entities = Vec::new();
    let mut relations = Vec::new();
    let (screenplays, cues) = screenplays(c, &mut entities)?;
    characters(c, &screenplays, &cues, &mut entities, &mut relations)?;
    people(c, &mut entities, &mut relations)?;
    simple(
        c,
        "SELECT id, name, status FROM location WHERE deleted_at IS NULL ORDER BY name, id",
        Kind::Location,
        &mut entities,
    )?;
    acts_and_cards(c, &mut entities, &mut relations)?;
    simple(
        c,
        "SELECT id, title, item_type FROM vault_item
         WHERE deleted_at IS NULL AND title IS NOT NULL AND trim(title) <> '' ORDER BY created_at, id",
        Kind::VaultItem,
        &mut entities,
    )?;
    simple(
        c,
        "SELECT id, display_name, source FROM project_file WHERE deleted_at IS NULL ORDER BY position, id",
        Kind::File,
        &mut entities,
    )?;
    simple(
        c,
        "SELECT l.id, l.description, l.category FROM budget_line l
         JOIN budget_snapshot b ON b.id = l.budget_id AND b.is_current = 1 AND b.deleted_at IS NULL
         WHERE l.deleted_at IS NULL ORDER BY l.position, l.id",
        Kind::BudgetLine,
        &mut entities,
    )?;
    days_and_sheets(c, &mut entities, &mut relations)?;
    Ok(EntityIndex::new(screenplays, entities, relations))
}

fn clip(s: &str) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= NAME_CHARS {
        s
    } else {
        s.chars().take(NAME_CHARS).collect()
    }
}

/// Entities with an id, a name and one detail column, nothing else.
fn simple(c: &Connection, sql: &str, kind: Kind, out: &mut Vec<Entity>) -> AppResult<()> {
    let mut st = c.prepare(sql)?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    for row in rows {
        let (id, name, detail) = row?;
        let name = clip(&name);
        if name.is_empty() {
            continue;
        }
        let mut e = Entity::new(kind, id, name);
        e.detail = detail.unwrap_or_default();
        if kind == Kind::Location
            && let Some(open) = e.name.find('(')
        {
            let bare = e.name[..open].trim().to_string();
            if !bare.is_empty() {
                e.aliases.push(bare);
            }
        }
        out.push(e);
    }
    Ok(())
}

/// Screenplays with their scenes, and for each screenplay the dialogue cues of its draft.
type Cues = HashMap<String, BTreeSet<String>>;

fn screenplays(c: &Connection, out: &mut Vec<Entity>) -> AppResult<(Vec<ScreenplayInfo>, Cues)> {
    let mut st = c.prepare(
        "SELECT s.id, s.title, s.format,
                (SELECT d.id FROM screenplay_draft d WHERE d.id = s.current_draft_id AND d.deleted_at IS NULL),
                (SELECT d.id FROM screenplay_draft d WHERE d.screenplay_id = s.id AND d.deleted_at IS NULL
                   ORDER BY d.created_at DESC, d.id DESC LIMIT 1)
         FROM screenplay s WHERE s.deleted_at IS NULL ORDER BY s.created_at, s.id",
    )?;
    type Row = (String, String, String, Option<String>, Option<String>);
    let rows: Vec<Row> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut scenes = c.prepare(
        "SELECT id, heading, production_number FROM screenplay_scene
         WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let mut cue_st = c.prepare(
        "SELECT DISTINCT e.text FROM screenplay_element e
         JOIN screenplay_scene sc ON sc.id = e.scene_id AND sc.deleted_at IS NULL
         WHERE sc.draft_id = ?1 AND e.element_type = 'character'",
    )?;
    let mut infos = Vec::new();
    let mut cues = Cues::new();
    for (id, title, format, current, latest) in rows {
        let has_current_draft = current.is_some();
        let mut count = 0u32;
        if let Some(draft) = current.or(latest) {
            let list = scenes
                .query_map([&draft], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for (scene_id, heading, number) in list {
                count += 1;
                let mut e = Entity::new(Kind::Scene, scene_id, heading);
                e.screenplay_id = Some(id.clone());
                e.ordinal = Some(count);
                if let Some(n) = number.filter(|n| !n.trim().is_empty()) {
                    e.aliases.push(n.trim().to_string());
                }
                out.push(e);
            }
            let set: BTreeSet<String> = cue_st
                .query_map([&draft], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .map(|t| normalize_cue(t))
                .filter(|t| !t.is_empty())
                .collect();
            cues.insert(id.clone(), set);
        }
        infos.push(ScreenplayInfo {
            id,
            title,
            format,
            scene_count: count,
            has_current_draft,
            aliases: Vec::new(),
        });
    }
    Ok((infos, cues))
}

/// "Also called Anju." in a description: the name after the words, up to the next full stop.
fn also_called(description: &str) -> Vec<String> {
    let lower = description.to_lowercase();
    let mut out = Vec::new();
    for marker in ["also called ", "also known as "] {
        let mut from = 0;
        while let Some(at) = lower[from..].find(marker) {
            let start = from + at + marker.len();
            let tail = &description[start..];
            let end = tail.find(['.', ',', ';', '\n', '(']).unwrap_or(tail.len());
            let name = tail[..end].trim();
            if !name.is_empty() && name.chars().count() <= 40 {
                out.push(name.to_string());
            }
            from = start;
        }
    }
    out
}

const TITLES: [&str; 6] = ["dr", "fr", "mr", "mrs", "ms", "sri"];

fn characters(
    c: &Connection,
    screenplays: &[ScreenplayInfo],
    cues: &Cues,
    out: &mut Vec<Entity>,
    relations: &mut Vec<Relation>,
) -> AppResult<()> {
    // Cue to character, as the user set it with screenplay.set_character_link.
    let mut links: HashMap<String, Vec<String>> = HashMap::new();
    {
        let mut st = c.prepare(
            "SELECT character_id, cue_name FROM screenplay_character_link
             WHERE ignored = 0 AND character_id IS NOT NULL",
        )?;
        for row in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (character, cue) = row?;
            links.entry(character).or_default().push(cue);
        }
    }
    let mut st = c.prepare(
        "SELECT id, name, role_label, description FROM story_character
         WHERE deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, role, description) in rows {
        let mut e = Entity::new(Kind::Character, id.clone(), name.clone());
        e.detail = role.unwrap_or_default();
        let mut cue_names = vec![normalize_cue(&name)];
        for cue in links.get(&id).into_iter().flatten() {
            e.aliases.push(cue.clone());
            cue_names.push(normalize_cue(cue));
        }
        e.aliases
            .extend(also_called(&description.unwrap_or_default()));
        // "Dr. Leela" is also "Leela".
        let parts: Vec<&str> = name.split_whitespace().collect();
        if parts.len() > 1
            && TITLES.contains(&parts[0].trim_end_matches('.').to_lowercase().as_str())
        {
            e.aliases.push(parts[1..].join(" "));
        }
        for sp in screenplays {
            if cues
                .get(&sp.id)
                .is_some_and(|set| cue_names.iter().any(|n| set.contains(n)))
            {
                relations.push(Relation {
                    from: id.clone(),
                    to: sp.id.clone(),
                    kind: "appears_in".into(),
                });
            }
        }
        out.push(e);
    }
    let mut st = c.prepare(
        "SELECT from_character_id, to_character_id, relationship_type FROM story_character_relationship
         ORDER BY position, id",
    )?;
    for row in st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))? {
        let (from, to, kind) = row?;
        relations.push(Relation { from, to, kind });
    }
    Ok(())
}

fn name_tokens(name: &str) -> Vec<String> {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() < 2 {
        return Vec::new();
    }
    let mut out = vec![parts[0].to_string(), parts[parts.len() - 1].to_string()];
    out.retain(|t| t.chars().count() >= 3);
    out.dedup();
    out
}

fn people(c: &Connection, out: &mut Vec<Entity>, relations: &mut Vec<Relation>) -> AppResult<()> {
    let characters: HashMap<String, String> = out
        .iter()
        .filter(|e| e.kind == Kind::Character)
        .map(|e| (e.name.to_lowercase(), e.id.clone()))
        .collect();
    let mut st = c.prepare(
        "SELECT id, person_name, character_id, character_name FROM cast_member
         WHERE deleted_at IS NULL ORDER BY created_at, id",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, character_id, character_name) in rows {
        let mut e = Entity::new(Kind::Person, id.clone(), name.clone());
        e.tokens = name_tokens(&name);
        let character = character_id.or_else(|| {
            character_name
                .as_ref()
                .and_then(|n| characters.get(&n.trim().to_lowercase()).cloned())
        });
        e.detail = match &character_name {
            Some(n) if !n.trim().is_empty() => format!("cast, plays {n}"),
            _ => "cast".into(),
        };
        if let Some(to) = character {
            relations.push(Relation {
                from: id,
                to,
                kind: "plays".into(),
            });
        }
        out.push(e);
    }
    let mut st = c.prepare(
        "SELECT id, person_name, role FROM crew_member WHERE deleted_at IS NULL ORDER BY created_at, id",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, name, role) in rows {
        // The same person can be cast and crew (one entry each): both stay, with their own detail.
        let mut e = Entity::new(Kind::Person, id, name.clone());
        e.tokens = name_tokens(&name);
        e.detail = format!("crew, {role}");
        out.push(e);
    }
    Ok(())
}

fn acts_and_cards(
    c: &Connection,
    out: &mut Vec<Entity>,
    relations: &mut Vec<Relation>,
) -> AppResult<()> {
    let mut st = c.prepare(
        "SELECT id, title FROM story_act WHERE deleted_at IS NULL ORDER BY position, id",
    )?;
    let acts = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (i, (id, title)) in acts.into_iter().enumerate() {
        let n = i as u32 + 1;
        let mut e = Entity::new(Kind::Act, id, title);
        e.ordinal = Some(n);
        e.aliases.push(format!("act {n}"));
        out.push(e);
    }
    let mut st = c.prepare(
        "SELECT id, short_description, scene_heading, parent_type, parent_id FROM story_scene_card
         WHERE deleted_at IS NULL AND trim(short_description) <> '' ORDER BY position, id",
    )?;
    let cards = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, text, heading, parent_type, parent_id) in cards {
        let mut e = Entity::new(Kind::Card, id.clone(), clip(&text));
        if let Some(h) = heading.filter(|h| !h.trim().is_empty()) {
            e.aliases.push(clip(&h));
        }
        if let (true, Some(to)) = (parent_type == "act", parent_id) {
            relations.push(Relation {
                from: id,
                to,
                kind: "in_act".into(),
            });
        }
        out.push(e);
    }
    Ok(())
}

fn days_and_sheets(
    c: &Connection,
    out: &mut Vec<Entity>,
    relations: &mut Vec<Relation>,
) -> AppResult<()> {
    // The schedule of the active Production Source, else the oldest one.
    let schedule: Option<String> = c
        .query_row(
            "SELECT s.id FROM shooting_schedule s WHERE s.deleted_at IS NULL
             ORDER BY (s.source_id IN (SELECT id FROM production_source WHERE active = 1)) DESC,
                      s.created_at, s.id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    if let Some(schedule) = schedule {
        let mut st = c.prepare(
            "SELECT id, is_off_day, shoot_date FROM shooting_day
             WHERE schedule_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        let days = st
            .query_map([&schedule], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, bool>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut number = 0u32;
        for (id, off, date) in days {
            // Off days are not numbered.
            if off {
                continue;
            }
            number += 1;
            let mut e = Entity::new(Kind::Day, id, format!("Shoot Day {number}"));
            e.ordinal = Some(number);
            e.aliases.push(format!("day {number}"));
            if let Some(d) = date {
                e.detail = d.clone();
                e.aliases.push(d);
            }
            out.push(e);
        }
    }
    let mut st = c.prepare(
        "SELECT id, title, shoot_day_id FROM call_sheet
         WHERE deleted_at IS NULL AND superseded_by IS NULL ORDER BY created_at, id",
    )?;
    let sheets = st
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (id, title, day) in sheets {
        out.push(Entity::new(Kind::CallSheet, id.clone(), title));
        relations.push(Relation {
            from: id,
            to: day,
            kind: "for_day".into(),
        });
    }
    Ok(())
}
