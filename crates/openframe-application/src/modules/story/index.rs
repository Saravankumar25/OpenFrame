//! Search indexers and Recently Deleted handlers for the Story module.
//!
//! Restore returns an object to its previous parent/position when that
//! container still exists, otherwise to the "Unassigned" area of its scope
//! (FSD §52.5, §90). Containers deleted "with everything inside" restore and
//! purge their children as one unit (children share the container's deleted_at).

use openframe_domain::{AppError, AppResult, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::tree::{
    self, StoryContainerRef, StoryContainerType, StoryItemKind, StoryItemRef, container_live,
    place, short,
};
use crate::modules::files::purge_asset_if_unreferenced;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeletedItemRow, Tx};

pub fn register(r: &mut Registry) {
    r.indexer("story_act", index_act);
    r.indexer("story_sequence", index_sequence);
    r.indexer("story_beat", index_beat);
    r.indexer("story_scene_card", index_card);
    r.indexer("story_character", index_character);
    r.indexer("episode", index_episode);
    r.trash_handler(TrashHandler {
        object_type: "story_act",
        table: "story_act",
        label: "Act",
        restore: Some(restore_act),
        purge: purge_act,
    });
    r.trash_handler(TrashHandler {
        object_type: "story_sequence",
        table: "story_sequence",
        label: "Sequence",
        restore: Some(restore_sequence),
        purge: purge_sequence,
    });
    r.trash_handler(TrashHandler {
        object_type: "story_beat",
        table: "story_beat",
        label: "Beat",
        restore: Some(restore_item),
        purge: purge_beat_entry,
    });
    r.trash_handler(TrashHandler {
        object_type: "story_scene_card",
        table: "story_scene_card",
        label: "Scene Card",
        restore: Some(restore_item),
        purge: purge_card_entry,
    });
    r.trash_handler(TrashHandler {
        object_type: "story_character",
        table: "story_character",
        label: "Character",
        restore: None,
        purge: purge_character,
    });
    r.trash_handler(TrashHandler {
        object_type: "season",
        table: "season",
        label: "Season",
        restore: None,
        purge: purge_season,
    });
    r.trash_handler(TrashHandler {
        object_type: "episode",
        table: "episode",
        label: "Episode",
        restore: None,
        purge: purge_episode,
    });
}

// ================================================================= indexers

fn nav(base: Value, episode: Option<String>) -> Value {
    let mut v = base;
    if let (Some(e), Some(obj)) = (episode, v.as_object_mut()) {
        obj.insert("episodeId".into(), Value::String(e));
    }
    v
}

fn context_for(parent_type: &str) -> String {
    match parent_type {
        "parking" => "Story · Parking Lot".into(),
        "unassigned" => "Story · Unassigned".into(),
        _ => "Story".into(),
    }
}

fn index_act(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT title, note, episode_id, deleted_at FROM story_act WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, note, ep, None)) => Some(SearchDoc {
            entity_type: "story_act".into(),
            title,
            body: note.unwrap_or_default(),
            context: "Story · Act".into(),
            nav: nav(json!({ "workspace": "story", "actId": id }), ep),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn index_sequence(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT title, note, episode_id, deleted_at FROM story_sequence WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, note, ep, None)) => Some(SearchDoc {
            entity_type: "story_sequence".into(),
            title,
            body: note.unwrap_or_default(),
            context: "Story · Sequence".into(),
            nav: nav(json!({ "workspace": "story", "sequenceId": id }), ep),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn index_beat(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT text, note, parent_type, episode_id, deleted_at FROM story_beat WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    Ok(match row {
        Some((text, note, pt, ep, None))
            if !(text.trim().is_empty() && note.as_deref().unwrap_or("").trim().is_empty()) =>
        {
            Some(SearchDoc {
                entity_type: "story_beat".into(),
                title: short(&text),
                body: format!("{} {}", text, note.unwrap_or_default()),
                context: context_for(&pt),
                nav: nav(json!({ "workspace": "story", "beatId": id }), ep),
                owner_user_id: None,
            })
        }
        _ => None,
    })
}

fn index_card(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<String>, String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT short_description, scene_heading, notes, parent_type, episode_id, deleted_at FROM story_scene_card WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    Ok(match row {
        Some((desc, heading, notes, pt, ep, None)) => {
            let heading = heading.unwrap_or_default();
            let notes = notes.unwrap_or_default();
            if desc.trim().is_empty() && heading.trim().is_empty() && notes.trim().is_empty() {
                return Ok(None);
            }
            let title = if desc.trim().is_empty() {
                short(&heading)
            } else {
                short(&desc)
            };
            Some(SearchDoc {
                entity_type: "story_scene_card".into(),
                title,
                body: format!("{desc}\n{heading}\n{notes}"),
                context: context_for(&pt),
                nav: nav(json!({ "workspace": "story", "cardId": id }), ep),
                owner_user_id: None,
            })
        }
        _ => None,
    })
}

fn index_character(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    type Row = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
    );
    let row: Option<Row> = c
        .query_row(
            "SELECT name, role_label, description, notes, episode_id, deleted_at FROM story_character WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    Ok(match row {
        Some((name, role, desc, notes, ep, None)) => Some(SearchDoc {
            entity_type: "story_character".into(),
            title: name,
            body: format!(
                "{} {} {}",
                role.unwrap_or_default(),
                desc.unwrap_or_default(),
                notes.unwrap_or_default()
            ),
            context: "Story · Characters".into(),
            nav: nav(
                json!({ "workspace": "story", "sub": "characters", "characterId": id }),
                ep,
            ),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn index_episode(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, Option<String>, Option<i64>)> = c
        .query_row(
            "SELECT title, summary, deleted_at FROM episode WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, summary, None)) => Some(SearchDoc {
            entity_type: "episode".into(),
            title,
            body: summary.unwrap_or_default(),
            context: "Series · Episodes".into(),
            nav: json!({ "workspace": "story", "sub": "episodes", "episodeId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

// ================================================================== restore

fn undelete(c: &Connection, table: &str, id: &str) -> AppResult<()> {
    c.execute(
        &format!("UPDATE {table} SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2"),
        params![now_ms(), id],
    )?;
    Ok(())
}

fn restore_children_of_sequence(c: &Connection, seq: &str, at: i64) -> AppResult<()> {
    let now = now_ms();
    for t in ["story_beat", "story_scene_card"] {
        c.execute(
            &format!(
                "UPDATE {t} SET deleted_at=NULL, updated_at=?3, rev=rev+1 WHERE deleted_at=?2 AND parent_type='sequence' AND parent_id=?1"
            ),
            params![seq, at, now],
        )?;
    }
    Ok(())
}

fn restore_act(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let (ep, at): (Option<String>, Option<i64>) = c.query_row(
        "SELECT episode_id, deleted_at FROM story_act WHERE id=?1",
        [&row.object_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    undelete(c, "story_act", &row.object_id)?;
    if let Some(at) = at {
        let now = now_ms();
        let seqs: Vec<String> = {
            let mut stmt =
                c.prepare("SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at=?2")?;
            stmt.query_map(params![row.object_id, at], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        for s in &seqs {
            undelete(c, "story_sequence", s)?;
            restore_children_of_sequence(c, s, at)?;
        }
        for t in ["story_beat", "story_scene_card"] {
            c.execute(
                &format!(
                    "UPDATE {t} SET deleted_at=NULL, updated_at=?3, rev=rev+1 WHERE deleted_at=?2 AND parent_type='act' AND parent_id=?1"
                ),
                params![row.object_id, at, now],
            )?;
        }
    }
    let mut ids = tree::act_ids(c, ep.as_deref())?;
    ids.retain(|x| x != &row.object_id);
    let idx = row
        .position
        .unwrap_or(ids.len() as i64)
        .clamp(0, ids.len() as i64) as usize;
    ids.insert(idx, row.object_id.clone());
    openframe_persistence::rows::renumber(c, "story_act", &ids)
}

fn restore_sequence(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let (act, ep, at): (Option<String>, Option<String>, Option<i64>) = c.query_row(
        "SELECT act_id, episode_id, deleted_at FROM story_sequence WHERE id=?1",
        [&row.object_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    undelete(c, "story_sequence", &row.object_id)?;
    if let Some(at) = at {
        restore_children_of_sequence(c, &row.object_id, at)?;
    }
    let me = StoryItemRef::new(StoryItemKind::Sequence, &row.object_id);
    match act {
        Some(a) if container_live(c, "act", Some(&a))? => place(
            c,
            std::slice::from_ref(&me),
            &StoryContainerRef::act(&a),
            ep.as_deref(),
            row.position.map(|p| p.max(0) as usize),
        ),
        // The Act is gone: the sequence (with its cards) goes to Unassigned.
        _ => place(
            c,
            std::slice::from_ref(&me),
            &StoryContainerRef::unassigned(),
            ep.as_deref(),
            None,
        ),
    }
}

fn restore_item(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let kind = if row.table_name == "story_beat" {
        StoryItemKind::Beat
    } else {
        StoryItemKind::Card
    };
    let (pt, pid): (String, Option<String>) = c.query_row(
        &format!(
            "SELECT parent_type, parent_id FROM {} WHERE id=?1",
            kind.table()
        ),
        [&row.object_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let ep: Option<String> = c.query_row(
        &format!("SELECT episode_id FROM {} WHERE id=?1", kind.table()),
        [&row.object_id],
        |r| r.get(0),
    )?;
    undelete(c, kind.table(), &row.object_id)?;
    let me = StoryItemRef::new(kind, &row.object_id);
    let parent_type = StoryContainerType::parse(&pt).unwrap_or(StoryContainerType::Unassigned);
    if container_live(c, &pt, pid.as_deref())? {
        let cont = StoryContainerRef {
            parent_type,
            parent_id: pid,
        };
        place(
            c,
            std::slice::from_ref(&me),
            &cont,
            ep.as_deref(),
            row.position.map(|p| p.max(0) as usize),
        )
    } else {
        place(
            c,
            std::slice::from_ref(&me),
            &StoryContainerRef::unassigned(),
            ep.as_deref(),
            None,
        )
    }
}

// ==================================================================== purge

fn purge_attachments(tx: &Tx<'_>, owner_type: &str, owner_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let assets: Vec<String> = {
        let mut stmt =
            c.prepare("SELECT asset_id FROM story_attachment WHERE owner_type=?1 AND owner_id=?2")?;
        stmt.query_map(params![owner_type, owner_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    c.execute(
        "DELETE FROM story_attachment WHERE owner_type=?1 AND owner_id=?2",
        params![owner_type, owner_id],
    )?;
    for a in assets {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

fn purge_card(tx: &Tx<'_>, id: &str) -> AppResult<()> {
    let c = tx.conn();
    let now = now_ms();
    c.execute(
        "DELETE FROM story_character_card_link WHERE scene_card_id=?1",
        [id],
    )?;
    c.execute(
        "UPDATE story_beat SET converted_scene_card_id=NULL, updated_at=?2, rev=rev+1 WHERE converted_scene_card_id=?1",
        params![id, now],
    )?;
    purge_attachments(tx, "scene_card", id)?;
    c.execute("DELETE FROM story_scene_card WHERE id=?1", [id])?;
    Ok(())
}

fn purge_beat(tx: &Tx<'_>, id: &str) -> AppResult<()> {
    let c = tx.conn();
    c.execute(
        "UPDATE story_scene_card SET source_beat_id=NULL, updated_at=?2, rev=rev+1 WHERE source_beat_id=?1",
        params![id, now_ms()],
    )?;
    purge_attachments(tx, "beat", id)?;
    c.execute("DELETE FROM story_beat WHERE id=?1", [id])?;
    Ok(())
}

fn purge_card_entry(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    purge_card(tx, &row.object_id)
}

fn purge_beat_entry(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    purge_beat(tx, &row.object_id)
}

fn ids(c: &Connection, sql: &str, p: (&str, i64)) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare(sql)?;
    let v = stmt
        .query_map(params![p.0, p.1], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

fn purge_sequence_with(tx: &Tx<'_>, id: &str, at: Option<i64>) -> AppResult<()> {
    let c = tx.conn();
    if let Some(at) = at {
        for card in ids(
            c,
            "SELECT id FROM story_scene_card WHERE parent_type='sequence' AND parent_id=?1 AND deleted_at=?2",
            (id, at),
        )? {
            purge_card(tx, &card)?;
        }
        for beat in ids(
            c,
            "SELECT id FROM story_beat WHERE parent_type='sequence' AND parent_id=?1 AND deleted_at=?2",
            (id, at),
        )? {
            purge_beat(tx, &beat)?;
        }
    }
    purge_attachments(tx, "sequence", id)?;
    c.execute("DELETE FROM story_sequence WHERE id=?1", [id])?;
    Ok(())
}

fn purge_sequence(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let at: Option<i64> = tx.conn().query_row(
        "SELECT deleted_at FROM story_sequence WHERE id=?1",
        [&row.object_id],
        |r| r.get(0),
    )?;
    purge_sequence_with(tx, &row.object_id, at)
}

fn purge_act(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = row.object_id.as_str();
    let at: Option<i64> =
        c.query_row("SELECT deleted_at FROM story_act WHERE id=?1", [id], |r| {
            r.get(0)
        })?;
    if let Some(at) = at {
        for s in ids(
            c,
            "SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at=?2",
            (id, at),
        )? {
            purge_sequence_with(tx, &s, Some(at))?;
        }
        for card in ids(
            c,
            "SELECT id FROM story_scene_card WHERE parent_type='act' AND parent_id=?1 AND deleted_at=?2",
            (id, at),
        )? {
            purge_card(tx, &card)?;
        }
        for beat in ids(
            c,
            "SELECT id FROM story_beat WHERE parent_type='act' AND parent_id=?1 AND deleted_at=?2",
            (id, at),
        )? {
            purge_beat(tx, &beat)?;
        }
    }
    // Sequences deleted separately earlier keep their own Recently Deleted entry;
    // they will restore to Unassigned.
    c.execute(
        "UPDATE story_sequence SET act_id=NULL, updated_at=?2, rev=rev+1 WHERE act_id=?1",
        params![id, now_ms()],
    )?;
    c.execute("DELETE FROM story_act WHERE id=?1", [id])?;
    Ok(())
}

fn purge_character(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = row.object_id.as_str();
    let linked: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM cast_member WHERE character_id=?1) OR EXISTS(SELECT 1 FROM catalog_item WHERE character_id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if linked {
        return Err(AppError::conflict(
            "This character is still linked to Cast & Crew or the Production Catalog, so it can't be permanently deleted. Restore it and archive it instead, or remove those links first.",
        ));
    }
    let image: Option<String> = c.query_row(
        "SELECT image_asset_id FROM story_character WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    c.execute(
        "DELETE FROM story_character_relationship WHERE from_character_id=?1 OR to_character_id=?1",
        [id],
    )?;
    c.execute(
        "DELETE FROM story_character_card_link WHERE character_id=?1",
        [id],
    )?;
    c.execute("DELETE FROM story_character WHERE id=?1", [id])?;
    if let Some(a) = image {
        purge_asset_if_unreferenced(tx, &a)?;
    }
    Ok(())
}

fn purge_season(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    c.execute(
        "UPDATE episode SET season_id=NULL, updated_at=?2, rev=rev+1 WHERE season_id=?1",
        params![row.object_id, now_ms()],
    )?;
    c.execute("DELETE FROM season WHERE id=?1", [&row.object_id])?;
    Ok(())
}

/// An episode can only be permanently deleted once nothing refers to it.
fn purge_episode(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let tables: Vec<String> = {
        let mut stmt = c.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'search_%'
               AND name NOT LIKE 'sys_%' AND name NOT LIKE '\\_%' ESCAPE '\\' AND name <> 'episode'",
        )?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for t in tables {
        let has_col: bool = c.query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('{}') WHERE name='episode_id')",
                t.replace('\'', "''")
            ),
            [],
            |r| r.get(0),
        )?;
        if has_col {
            let used: bool = c.query_row(
                &format!(
                    "SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE episode_id=?1)",
                    t.replace('"', "")
                ),
                [&row.object_id],
                |r| r.get(0),
            )?;
            if used {
                return Err(AppError::conflict(
                    "This episode still has story, screenplay or production material. Restore it and move or delete that material first.",
                ));
            }
        }
    }
    c.execute("DELETE FROM episode WHERE id=?1", [&row.object_id])?;
    Ok(())
}
