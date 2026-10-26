//! Story: characters, their relationships, acts and their cards, and the project's logline.
//! Reads `story_character`, `story_character_relationship`, `story_act`, `story_scene_card`.

use std::collections::HashMap;

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{clip, relation_sentence, relation_word, title_case};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_LOW, P_MID};
use crate::modules::ai::index::{Hit, Kind, Resolved, normalize};
use crate::modules::ai::queries;
use crate::modules::ai::turn::{AnswerType, Group, TurnRead};

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("story", || fetch(ctx, read, r))
}

struct Ch {
    id: String,
    name: String,
    role: String,
    description: String,
}

fn characters(ctx: &FetchCtx<'_>) -> AppResult<Vec<Ch>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, name, COALESCE(role_label, ''), COALESCE(description, '') FROM story_character
         WHERE deleted_at IS NULL AND archived = 0 ORDER BY position, name LIMIT 400",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(Ch {
                id: r.get(0)?,
                name: r.get(1)?,
                role: r.get(2)?,
                description: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Other names of a character: dialogue cues linked to it and "also called" in its description.
fn other_names(ctx: &FetchCtx<'_>, id: &str, name: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(e) = ctx.index.entity(id) {
        for a in &e.aliases {
            let a = title_case(a);
            let key = a.to_lowercase();
            if key != name.to_lowercase() && !out.iter().any(|o| o.to_lowercase() == key) {
                out.push(a);
            }
        }
    }
    out
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let t = read.answer_type;
    let mut out: Vec<Fact> = Vec::new();
    let all = characters(ctx)?;
    let by_id: HashMap<&str, &Ch> = all.iter().map(|c| (c.id.as_str(), c)).collect();

    // ---- the characters the message names
    let named: Vec<&Ch> = r
        .characters
        .iter()
        .filter_map(|h| by_id.get(h.id.as_str()).copied())
        .collect();
    let asks_names = m.any(&[
        "other names",
        "nickname",
        "nicknames",
        "also called",
        "also known",
        "alias",
        "aliases",
        "call",
        "called",
        "calls",
        "known as",
    ]);
    let wants_description = narrative
        || asks_names
        || m.any(&["who is", "what is", "about", "describe", "like", "tell"])
        || matches!(t, AnswerType::Explain);
    for ch in &named {
        let mut f = Fact::new("Character", &ch.id)
            .f("name", &ch.name)
            .f("role", &ch.role)
            .prio(P_HIGH);
        if wants_description {
            f = f.f("description", clip(&ch.description, 280));
        }
        let names = other_names(ctx, &ch.id, &ch.name);
        if narrative || asks_names {
            f = f.f("also called", names.join(", "));
        }
        if asks_names {
            for n in &names {
                f = f.require(n);
            }
        }
        if narrative || m.any(&["plays", "played", "actor", "actress", "cast"]) {
            let who: Vec<String> = ctx
                .index
                .played_by(&ch.id)
                .iter()
                .map(|e| e.name.clone())
                .collect();
            f = f.f("played by", who.join(", "));
        }
        out.push(f);
    }

    // ---- relationships
    let general_relations = r.characters.is_empty()
        && m.stem("relationship")
        && matches!(
            t,
            AnswerType::Relation | AnswerType::Explain | AnswerType::List
        );
    let asks_relations = !named.is_empty()
        && (matches!(
            t,
            AnswerType::Relation
                | AnswerType::Explain
                | AnswerType::Lookup
                | AnswerType::List
                | AnswerType::Compare
        ) || narrative
            || m.stem("relat")
            || m.stem("connect"));
    if asks_relations || general_relations {
        out.extend(relationships(ctx, read, &named, &by_id, general_relations)?);
    }

    // ---- the cast of characters as a whole
    if named.is_empty() && m.stem("character") {
        let roles = [
            "antagonist",
            "protagonist",
            "lead",
            "supporting",
            "minor",
            "villain",
            "hero",
        ];
        let asked_role = roles.iter().find(|w| m.has(w)).map(|w| match *w {
            "villain" => "antagonist",
            "hero" => "protagonist",
            other => other,
        });
        if t == AnswerType::Count {
            out.push(
                Fact::new("Count", "")
                    .f("what", "characters")
                    .f("value", all.len().to_string())
                    .require(all.len().to_string())
                    .prio(P_HIGH),
            );
        } else if matches!(
            t,
            AnswerType::List | AnswerType::Explain | AnswerType::Lookup
        ) || narrative
        {
            let few = all.len() <= 14;
            for ch in all
                .iter()
                .filter(|c| asked_role.is_none_or(|w| c.role.to_lowercase().contains(w)))
            {
                let mut f = Fact::new("Character", &ch.id)
                    .f("name", &ch.name)
                    .f("role", &ch.role)
                    .prio(P_MID);
                if few && matches!(t, AnswerType::List | AnswerType::Lookup) {
                    f = f.require(&ch.name);
                }
                out.push(f);
            }
        }
    } else if named.is_empty() && !all.is_empty() {
        // "who is the antagonist" names no character and no "characters" word.
        let roles = ["antagonist", "protagonist", "villain", "hero"];
        if let Some(w) = roles.iter().find(|w| m.has(w)) {
            let want = match *w {
                "villain" => "antagonist",
                "hero" => "protagonist",
                o => o,
            };
            for ch in all.iter().filter(|c| c.role.to_lowercase().contains(want)) {
                out.push(
                    Fact::new("Character", &ch.id)
                        .f("name", &ch.name)
                        .f("role", &ch.role)
                        .require(&ch.name)
                        .prio(P_HIGH),
                );
            }
        }
    }

    // ---- a name asked about that the project does not have
    if let Some(who) = unknown_name(&ctx.message, r)
        && named.is_empty()
    {
        out.push(Fact::missing(format!(
            "no character called {who} in the screenplays, the Story characters or Cast and Crew"
        )));
    }

    // ---- acts and cards
    out.extend(acts_and_cards(ctx, read, r, narrative)?);

    // The relationship line comes first, then the cards.
    out.sort_by_key(|f| f.kind != "Relationship");

    // ---- the project in a few words, for "explain the story"
    if narrative
        && named.is_empty()
        && r.scenes.is_empty()
        && matches!(t, AnswerType::Explain | AnswerType::Brainstorm)
    {
        out.extend(project_line(ctx)?);
    }
    Ok(out)
}

/// "who is Dr. Menon", "who plays Marcus": a capitalised name after the question words that no
/// index entry answers to.
pub fn unknown_name(message: &str, r: &Resolved) -> Option<String> {
    let any_named = !(r.characters.is_empty()
        && r.persons.is_empty()
        && r.locations.is_empty()
        && r.acts.is_empty()
        && r.others.is_empty());
    if any_named || !r.scenes.is_empty() {
        return None;
    }
    let msg = message;
    let lower = msg.to_lowercase();
    // The positions found in the lower case text are used on the original: they must agree.
    if lower.len() != msg.len() {
        return None;
    }
    for lead in [
        "who is ",
        "who plays ",
        "who played ",
        "tell me about ",
        "who's ",
    ] {
        if let Some(at) = lower.find(lead) {
            let tail = &msg[at + lead.len()..];
            let words: Vec<&str> = tail
                .split_whitespace()
                .take_while(|w| {
                    w.chars().next().is_some_and(char::is_uppercase)
                        && !["In", "The", "My", "Our", "This"]
                            .contains(&w.trim_matches(|c: char| !c.is_alphanumeric()))
                })
                .collect();
            if !words.is_empty() {
                return Some(
                    words
                        .join(" ")
                        .trim_matches(|c: char| c == '?' || c == ',' || c == '.')
                        .to_string(),
                );
            }
        }
    }
    None
}

fn relationships(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    named: &[&Ch],
    by_id: &HashMap<&str, &Ch>,
    general: bool,
) -> AppResult<Vec<Fact>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, from_character_id, to_character_id, relationship_type, COALESCE(note, '')
         FROM story_character_relationship ORDER BY position, id LIMIT 400",
    )?;
    let rows: Vec<(String, String, String, String, String)> = st
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    let ids: Vec<&str> = named.iter().map(|c| c.id.as_str()).collect();
    let t = read.answer_type;
    let mut out = Vec::new();
    // The relationship between the named characters comes first.
    let mut pair: Vec<Fact> = Vec::new();
    let mut rest: Vec<Fact> = Vec::new();
    for (id, from, to, kind, note) in rows {
        let (Some(a), Some(b)) = (by_id.get(from.as_str()), by_id.get(to.as_str())) else {
            continue;
        };
        let both = ids.contains(&from.as_str()) && ids.contains(&to.as_str());
        let touches = ids.contains(&from.as_str()) || ids.contains(&to.as_str());
        if !general && !touches {
            continue;
        }
        let mut f = Fact::new("Relationship", id)
            .said(relation_sentence(&a.name, &kind, &b.name))
            .f("note", &note)
            .prio(if both { P_HIGH } else { P_MID });
        if both {
            f = f.require(relation_word(&kind));
        } else if !general && matches!(t, AnswerType::Relation | AnswerType::Lookup) {
            // The other end is what a one-name question asks for ("who is Babu's rival").
            let other = if ids.contains(&from.as_str()) {
                &b.name
            } else {
                &a.name
            };
            let word = relation_word(&kind).to_lowercase();
            let asked = ctx
                .m
                .words()
                .any(|w| w.len() >= 4 && word.split(' ').any(|x| x == w))
                || t == AnswerType::Relation;
            if asked {
                f = f.require(other);
            }
        }
        if general {
            f = f.require(relation_word(&kind));
        }
        if both { pair.push(f) } else { rest.push(f) }
    }
    out.extend(pair);
    out.extend(rest);
    Ok(out)
}

fn acts_and_cards(
    ctx: &FetchCtx<'_>,
    read: &TurnRead,
    r: &Resolved,
    narrative: bool,
) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let named_cards: Vec<&Hit> = r
        .others
        .iter()
        .filter(|(k, _)| *k == Kind::Card)
        .map(|(_, h)| h)
        .collect();
    let board_words = m.any(&[
        "act",
        "acts",
        "story board",
        "beat board",
        "board",
        "outline",
    ]) || m.stem("card");
    if r.acts.is_empty() && named_cards.is_empty() && !board_words {
        // Narrative questions about the whole story read the acts too.
        if !(narrative && r.characters.is_empty() && r.scenes.is_empty()) {
            return Ok(Vec::new());
        }
    }
    let mut st = ctx.conn.prepare(
        "SELECT id, title, COALESCE(note, '') FROM story_act WHERE deleted_at IS NULL ORDER BY position, id",
    )?;
    let acts: Vec<(String, String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut st = ctx.conn.prepare(
        "SELECT id, parent_id, short_description, COALESCE(scene_heading, ''), screenplay_scene_id,
                COALESCE(long_description, ''), COALESCE(notes, '')
         FROM story_scene_card WHERE deleted_at IS NULL AND parent_type = 'act' ORDER BY position, id",
    )?;
    type Card = (
        String,
        Option<String>,
        String,
        String,
        Option<String>,
        String,
        String,
    );
    let cards: Vec<Card> = st
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
    let mut out = Vec::new();
    if acts.is_empty() && !board_words {
        return Ok(out);
    }
    if acts.is_empty() {
        out.push(Fact::missing(
            "no acts on the Story Board: no act has been entered in this project",
        ));
        return Ok(out);
    }
    let want: Vec<&str> = r.acts.iter().map(|h| h.id.as_str()).collect();
    let whole = want.is_empty();
    let show_cards = !whole
        || !named_cards.is_empty()
        || m.stem("card")
        || m.any(&["scene", "scenes", "point", "points", "what is in"])
        || narrative && acts.len() <= 6;
    if t == AnswerType::Count {
        out.push(
            Fact::new("Count", "")
                .f("what", "acts")
                .f("value", acts.len().to_string())
                .require(acts.len().to_string()),
        );
        if m.stem("card") {
            let n = if whole {
                cards.len()
            } else {
                cards
                    .iter()
                    .filter(|c| c.1.as_deref().is_some_and(|p| want.contains(&p)))
                    .count()
            };
            let what = if whole {
                "cards on the Story Board".to_string()
            } else {
                format!(
                    "cards in {}",
                    r.acts
                        .iter()
                        .map(|h| h.name.clone())
                        .collect::<Vec<_>>()
                        .join(" and ")
                )
            };
            out.push(
                Fact::new("Count", "")
                    .f("what", what)
                    .f("value", n.to_string())
                    .require(n.to_string()),
            );
        }
    }
    // Scene numbers of the cards that point to a scene.
    let linked: Vec<String> = cards.iter().filter_map(|c| c.4.clone()).collect();
    let numbers = ctx.scene_numbers(&linked);
    // A card with no link to a scene points where its heading does.
    let mut by_heading: HashMap<String, Vec<u32>> = HashMap::new();
    if cards
        .iter()
        .any(|c| c.4.is_none() && !c.3.trim().is_empty())
        && let Some(sp) = ctx.screenplay(r)
        && let Some(d) = ctx.draft(&sp.id)
    {
        for s in queries::scenes(ctx.conn, &d.id)? {
            by_heading
                .entry(normalize(&s.heading))
                .or_default()
                .push(s.number as u32);
        }
    }
    let heading_scenes = |heading: &str| -> String {
        by_heading
            .get(&normalize(heading))
            .map(|v| {
                v.iter()
                    .take(6)
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    for (i, (id, title, note)) in acts.iter().enumerate() {
        if !whole && !want.contains(&id.as_str()) {
            continue;
        }
        let n = i + 1;
        let count = cards
            .iter()
            .filter(|c| c.1.as_deref() == Some(id.as_str()))
            .count();
        let mut f = Fact::new("Act", id)
            .f("number", n.to_string())
            .f("title", title)
            .f("note", clip(note, 220))
            .f("cards", count.to_string())
            .prio(P_HIGH);
        if whole && narrative {
            f = f.require(title);
        }
        out.push(f);
        if !show_cards {
            continue;
        }
        for c in cards.iter().filter(|c| c.1.as_deref() == Some(id.as_str())) {
            if !named_cards.is_empty() && !named_cards.iter().any(|h| h.id == c.0) && !whole {
                continue;
            }
            let scene = c.4.as_ref().and_then(|s| numbers.get(s));
            out.push(
                Fact::new("Card", &c.0)
                    .f("act", n.to_string())
                    .f("what", clip(&c.2, 200))
                    .f("detail", clip(&c.5, 200))
                    .f("heading", &c.3)
                    .f("scene", scene.map(|s| s.0.to_string()).unwrap_or_default())
                    .f(
                        "scene heading",
                        scene.map(|s| s.1.clone()).unwrap_or_default(),
                    )
                    .f(
                        "scenes with this heading",
                        if scene.is_none() {
                            heading_scenes(&c.3)
                        } else {
                            String::new()
                        },
                    )
                    .prio(P_MID),
            );
        }
    }
    // A card found by a word of the message ("the card about the batteries").
    if m.stem("card") && whole && named_cards.is_empty() {
        let words = m.keywords();
        for c in &cards {
            let hay = format!("{} {} {}", c.2, c.5, c.6).to_lowercase();
            if words
                .iter()
                .filter(|w| w.as_str() != "card" && w.as_str() != "cards")
                .any(|w| hay.contains(w.trim_end_matches('s')))
            {
                let scene = c.4.as_ref().and_then(|s| numbers.get(s));
                out.push(
                    Fact::new("Card", &c.0)
                        .f("what", clip(&c.2, 200))
                        .f("heading", &c.3)
                        .f("scene", scene.map(|s| s.0.to_string()).unwrap_or_default())
                        .f(
                            "scene heading",
                            scene.map(|s| s.1.clone()).unwrap_or_default(),
                        )
                        .prio(P_HIGH),
                );
            }
        }
    }
    Ok(out)
}

fn project_line(ctx: &FetchCtx<'_>) -> AppResult<Vec<Fact>> {
    let row: Option<(String, String, String)> = ctx
        .conn
        .query_row(
            "SELECT title, COALESCE(logline, ''), COALESCE(genre, '') FROM project LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    Ok(row
        .into_iter()
        .map(|(title, logline, genre)| {
            Fact::new("Project", "")
                .f("title", title)
                .f("logline", clip(&logline, 400))
                .f("genre", genre)
                .prio(P_LOW + 40)
        })
        .collect())
}
