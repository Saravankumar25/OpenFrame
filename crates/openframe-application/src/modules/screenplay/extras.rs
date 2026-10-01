//! Screenplay extras: title page (FSD §16.5), remembered layout (FSD §17.4),
//! Scene Hub (UX §3.11, mock 097), Story Board reference panel (FSD §17.2,
//! read-only), scene search indexing and recoverable scene deletion.

use openframe_domain::enums::DraftStatus;
use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use openframe_persistence::rows::{place_at, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::{ScreenplayTitlePage, bump_seq, live_scene_ids, load_screenplay, scene_number};
use crate::core::AppCore;
use crate::modules::comments;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeletedItemRow, MutationMeta, Tx};

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    use openframe_domain::Capability as Cap;
    r.command("screenplay.update_title_page", update_title_page)
        .meta(M::edit("Edit a screenplay's title page."));
    r.query("screenplay.view_state", view_state)
        .meta(M::read("The user's screenplay editor layout preferences.").hidden(h::VIEW_STATE));
    r.command("screenplay.set_view_state", set_view_state).meta(
        M::command(Cap::View, "Save screenplay editor layout preferences.").hidden(h::VIEW_STATE),
    );
    r.query("screenplay.scene_hub", scene_hub).meta(M::compute("Everything linked to a scene across modules (breakdown, shots, storyboards, schedule, comments, story)."));
    r.query("screenplay.story_reference", story_reference)
        .meta(M::read("The Scene Card a screenplay scene came from."));
    r.indexer("screenplay_scene", index_scene);
    r.trash_handler(TrashHandler {
        object_type: "screenplay_scene",
        table: "screenplay_scene",
        label: "Screenplay scene",
        restore: Some(restore_scene),
        purge: purge_scene,
    });
}

const VIEW_KEY: &str = "screenplay.layout";

// --------------------------------------------------------------- title page

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayTitlePageArgs {
    pub screenplay_id: String,
    pub title_page: ScreenplayTitlePage,
}

fn update_title_page(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayTitlePageArgs,
) -> AppResult<super::ScreenplayDto> {
    let tp = a.title_page;
    for (v, what, max) in [
        (&tp.title, "Title", 200usize),
        (&tp.written_by, "Written by", 300),
        (&tp.contact, "Contact", 1_000),
        (&tp.draft_line, "Draft / revision line", 200),
        (&tp.notes, "Notes on the title page", 2_000),
    ] {
        if v.chars().count() > max {
            return Err(AppError::invalid_input(format!("{what} is too long.")));
        }
    }
    if tp.title.trim().is_empty() {
        return Err(AppError::required("Title"));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.update_title_page",
            "Edited the title page",
            Capability::Edit,
        )
        .target("screenplay", &a.screenplay_id),
        |tx| {
            let c = tx.conn();
            load_screenplay(c, &a.screenplay_id)?;
            let json = serde_json::to_string(&tp).map_err(|e| AppError::internal(e.to_string()))?;
            update_fields(
                c,
                "screenplay",
                &a.screenplay_id,
                &[
                    ("title_page_json", text(json)),
                    ("title", text(tp.title.trim())),
                ],
                &["title_page_json", "title"],
                None,
                "screenplay",
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| load_screenplay(c, &a.screenplay_id))
}

// --------------------------------------------------------------- view state

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayViewStateArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySetViewStateArgs {
    /// Layout preferences (mode, open panels, notes visibility, open draft…).
    #[ts(type = "unknown")]
    pub state: Value,
}

/// The writer's last layout for this project (per user; never project content).
fn view_state(core: &AppCore, actor: &Actor, _: ScreenplayViewStateArgs) -> AppResult<Value> {
    actor.require(Capability::View, "view the screenplay")?;
    core.project()?.store.read(|c| {
        let v: Option<String> = c
            .query_row(
                "SELECT value_json FROM sys_view_state WHERE user_id=?1 AND key=?2",
                params![actor.user_id, VIEW_KEY],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v.and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null))
    })
}

fn set_view_state(core: &AppCore, actor: &Actor, a: ScreenplaySetViewStateArgs) -> AppResult<()> {
    actor.require(Capability::View, "use the screenplay")?;
    if !a.state.is_object() {
        return Err(AppError::invalid_input("The layout must be an object."));
    }
    let json = a.state.to_string();
    if json.len() > 8_192 {
        return Err(AppError::invalid_input("The layout is too large."));
    }
    // View state is infrastructure (not undoable, not activity) — same path as "Continue".
    core.project()?.store.with_writer(|c| {
        c.execute(
            "INSERT INTO sys_view_state(user_id, key, value_json, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(user_id, key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
            params![actor.user_id, VIEW_KEY, json, now_ms()],
        )?;
        Ok(())
    })
}

// ---------------------------------------------------------------- scene hub

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySceneArgs {
    pub scene_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneHubRow {
    /// screenplay | breakdown | shots | storyboard | schedule | comments | story
    pub key: String,
    pub label: String,
    #[ts(type = "number | null")]
    pub count: Option<i64>,
    pub detail: String,
    /// Workspace to open, focused on this scene.
    #[ts(type = "unknown")]
    pub nav: Value,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneHub {
    pub scene_id: String,
    pub number: Option<u32>,
    pub heading: String,
    pub draft_name: String,
    pub rows: Vec<ScreenplaySceneHubRow>,
    /// Production planning already refers to this scene (reorder warning, mock 099).
    pub used_in_production: bool,
    /// e.g. "Breakdown, 4 shots and Shoot Day 7".
    pub production_summary: String,
}

fn table_exists(c: &Connection, t: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [t],
        |r| r.get(0),
    )?)
}

fn columns(c: &Connection, t: &str) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare(&format!("PRAGMA table_info(\"{t}\")"))?;
    let cols = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    Ok(cols)
}

/// Count rows of another module's table that refer to this scene, by whichever
/// scene reference column the table has (tables owned by other modules may not
/// exist yet in this build).
fn count_for_scene(
    c: &Connection,
    table: &str,
    scene_id: &str,
    lineage: &str,
    extra: &str,
) -> AppResult<Option<i64>> {
    if !table_exists(c, table)? {
        return Ok(None);
    }
    let cols = columns(c, table)?;
    let live = if cols.iter().any(|x| x == "deleted_at") {
        " AND deleted_at IS NULL"
    } else {
        ""
    };
    let (col, val) = if cols.iter().any(|x| x == "scene_id") {
        ("scene_id", scene_id)
    } else if cols.iter().any(|x| x == "screenplay_scene_id") {
        ("screenplay_scene_id", scene_id)
    } else if cols.iter().any(|x| x == "scene_lineage_id") {
        ("scene_lineage_id", lineage)
    } else {
        return Ok(None);
    };
    Ok(Some(c.query_row(
        &format!("SELECT count(*) FROM \"{table}\" WHERE \"{col}\"=?1{live}{extra}"),
        [val],
        |r| r.get(0),
    )?))
}

pub(crate) fn hub_for(c: &Connection, scene_id: &str) -> AppResult<ScreenplaySceneHub> {
    let (draft_id, lineage, heading, source_card): (String, String, String, Option<String>) = c
        .query_row(
            "SELECT draft_id, lineage_id, heading, source_scene_card_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("scene"))?;
    let draft_name: String = c.query_row(
        "SELECT name FROM screenplay_draft WHERE id=?1",
        [&draft_id],
        |r| r.get(0),
    )?;
    let nav = |ws: &str| json!({ "workspace": ws, "sceneId": scene_id, "sceneLineageId": lineage });
    let mut rows = vec![ScreenplaySceneHubRow {
        key: "screenplay".into(),
        label: "Screenplay".into(),
        count: None,
        detail: draft_name.clone(),
        nav: nav("screenplay"),
    }];
    let mut used: Vec<String> = Vec::new();
    // Breakdown (hub table, FSD §26): confirmed/manual elements are production truth.
    if let Some(n) = count_for_scene(
        c,
        "breakdown_element",
        scene_id,
        &lineage,
        " AND confirmation_state IN ('Confirmed','Manual')",
    )? {
        let suggested = count_for_scene(
            c,
            "breakdown_element",
            scene_id,
            &lineage,
            " AND confirmation_state = 'Suggested'",
        )?
        .unwrap_or(0);
        let detail = match (n, suggested) {
            (0, 0) => "Nothing created yet".to_string(),
            (n, 0) => format!("{n} element{}", if n == 1 { "" } else { "s" }),
            (n, s) => format!("{n} confirmed · {s} suggested"),
        };
        if n + suggested > 0 {
            used.push("Breakdown".into());
        }
        rows.push(ScreenplaySceneHubRow {
            key: "breakdown".into(),
            label: "Breakdown".into(),
            count: Some(n),
            detail,
            nav: nav("breakdown"),
        });
    }
    for (key, label, table, ws) in [
        ("shots", "Shots", "shot", "production"),
        ("storyboard", "Storyboard", "storyboard_panel", "production"),
        ("schedule", "Schedule", "schedule_strip", "production"),
    ] {
        if let Some(n) = count_for_scene(c, table, scene_id, &lineage, "")? {
            if n > 0 {
                used.push(match key {
                    "shots" => format!("{n} shot{}", if n == 1 { "" } else { "s" }),
                    "storyboard" => {
                        format!("{n} storyboard panel{}", if n == 1 { "" } else { "s" })
                    }
                    _ => "the shooting schedule".to_string(),
                });
            }
            let mut v = nav(ws);
            v["sub"] = json!(key);
            rows.push(ScreenplaySceneHubRow {
                key: key.into(),
                label: format!("{label} ({n})"),
                count: Some(n),
                detail: if n == 0 {
                    "Nothing created yet".into()
                } else {
                    format!("{n}")
                },
                nav: v,
            });
        }
    }
    let (total, open): (i64, i64) = c.query_row(
        "SELECT count(*), coalesce(sum(status IN ('Open','In Discussion')), 0) FROM comment
         WHERE scene_id=?1 AND parent_id IS NULL AND deleted_at IS NULL",
        [scene_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    rows.push(ScreenplaySceneHubRow {
        key: "comments".into(),
        label: format!("Comments ({total})"),
        count: Some(total),
        detail: if total == 0 {
            "No comments yet.".into()
        } else {
            format!("{open} open")
        },
        nav: {
            let mut v = super::scene_nav(c, scene_id)?;
            v["panel"] = json!("comments");
            v
        },
    });
    let card: Option<String> = c
        .query_row(
            "SELECT id FROM story_scene_card WHERE deleted_at IS NULL AND (screenplay_scene_id=?1 OR id IS ?2) LIMIT 1",
            params![scene_id, source_card],
            |r| r.get(0),
        )
        .optional()?;
    rows.push(ScreenplaySceneHubRow {
        key: "story".into(),
        label: "Story Board".into(),
        count: None,
        detail: if card.is_some() {
            "Card found".into()
        } else {
            "No linked card".into()
        },
        nav: match &card {
            Some(id) => json!({ "workspace": "story", "cardId": id }),
            None => json!({ "workspace": "story" }),
        },
    });
    let production_summary = match used.len() {
        0 => String::new(),
        1 => used[0].clone(),
        n => format!("{} and {}", used[..n - 1].join(", "), used[n - 1]),
    };
    Ok(ScreenplaySceneHub {
        scene_id: scene_id.to_string(),
        number: scene_number(c, scene_id)?,
        heading,
        draft_name,
        rows,
        used_in_production: !used.is_empty(),
        production_summary,
    })
}

fn scene_hub(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySceneArgs,
) -> AppResult<ScreenplaySceneHub> {
    actor.require(Capability::View, "view the screenplay")?;
    core.project()?.store.read(|c| hub_for(c, &a.scene_id))
}

// ----------------------------------------------------------- story reference

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCardRef {
    pub id: String,
    pub short_description: String,
    pub scene_heading: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayStoryReference {
    /// The Story Board card this scene was built from / linked to, if any.
    pub linked: Option<ScreenplayCardRef>,
    /// Active Story Board cards in board order (reference only).
    pub cards: Vec<ScreenplayCardRef>,
}

fn card_ref(r: &rusqlite::Row<'_>) -> rusqlite::Result<ScreenplayCardRef> {
    Ok(ScreenplayCardRef {
        id: r.get(0)?,
        short_description: r.get(1)?,
        scene_heading: r.get(2)?,
        notes: r.get(3)?,
        color: r.get(4)?,
    })
}

/// Read-only Story Board reference for the Writing Room (FSD §17.5: viewing never edits or locks).
fn story_reference(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySceneArgs,
) -> AppResult<ScreenplayStoryReference> {
    actor.require(Capability::View, "view the Story Board")?;
    core.project()?.store.read(|c| {
        let (source, episode): (Option<String>, Option<String>) = c
            .query_row(
                "SELECT s.source_scene_card_id, sp.episode_id FROM screenplay_scene s
                 JOIN screenplay_draft d ON d.id = s.draft_id JOIN screenplay sp ON sp.id = d.screenplay_id WHERE s.id=?1",
                [&a.scene_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("scene"))?;
        let linked = c
            .query_row(
                "SELECT id, short_description, scene_heading, notes, color FROM story_scene_card
                 WHERE deleted_at IS NULL AND (screenplay_scene_id=?1 OR id IS ?2) LIMIT 1",
                params![a.scene_id, source],
                card_ref,
            )
            .optional()?;
        let mut stmt = c.prepare(
            "SELECT c.id, c.short_description, c.scene_heading, c.notes, c.color FROM story_scene_card c
             LEFT JOIN story_sequence q ON c.parent_type='sequence' AND q.id = c.parent_id
             LEFT JOIN story_act a ON a.id = CASE WHEN c.parent_type='act' THEN c.parent_id ELSE q.act_id END
             WHERE c.deleted_at IS NULL AND c.parent_type IN ('act','sequence') AND c.episode_id IS ?1
             ORDER BY coalesce(a.position, 0), coalesce(q.position, 0), c.position, c.id LIMIT 1000",
        )?;
        let cards = stmt.query_map([episode], card_ref)?.collect::<Result<Vec<_>, _>>()?;
        Ok(ScreenplayStoryReference { linked, cards })
    })
}

// ------------------------------------------------------------------ search

/// Scenes of each screenplay's CURRENT draft are searchable (older drafts would
/// only duplicate hits). Title carries the derived number and heading; body is
/// the scene's written text and notes.
fn index_scene(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, bool, Option<String>)> = c
        .query_row(
            "SELECT draft_id, heading, deleted_at IS NOT NULL, notes FROM screenplay_scene WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((draft_id, heading, deleted, notes)) = row else {
        return Ok(None);
    };
    if deleted {
        return Ok(None);
    }
    let draft: Option<(String, bool, Option<String>)> = c
        .query_row(
            "SELECT d.name, d.deleted_at IS NOT NULL OR sp.deleted_at IS NOT NULL, sp.current_draft_id
             FROM screenplay_draft d JOIN screenplay sp ON sp.id = d.screenplay_id WHERE d.id=?1",
            [&draft_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((draft_name, gone, current)) = draft else {
        return Ok(None);
    };
    if gone || current.as_deref() != Some(draft_id.as_str()) {
        return Ok(None);
    }
    let number = scene_number(c, id)?.unwrap_or(0);
    let mut stmt =
        c.prepare("SELECT text FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id")?;
    let mut body: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if let Some(n) = notes {
        body.push(n);
    }
    let heading_label = if heading.trim().is_empty() {
        "Untitled scene".to_string()
    } else {
        heading.trim().to_string()
    };
    Ok(Some(SearchDoc {
        entity_type: "screenplay_scene".into(),
        title: format!("Scene {number} — {heading_label}"),
        body: body.join("\n"),
        context: format!("Screenplay · {draft_name}"),
        nav: super::scene_nav(c, id)?,
        owner_user_id: None,
    }))
}

// ------------------------------------------------------------------- trash

/// Restore a deleted scene to its former position in its draft (FSD §52.5).
fn restore_scene(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let draft_id: String = c
        .query_row(
            "SELECT draft_id FROM screenplay_scene WHERE id=?1",
            [&row.object_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("scene"))?;
    let status: Option<(String, bool)> = c
        .query_row(
            "SELECT status, deleted_at IS NOT NULL FROM screenplay_draft WHERE id=?1",
            [&draft_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match status {
        Some((_, true)) | None => {
            return Err(AppError::conflict(
                "This scene's draft was deleted. Restore the draft first.",
            ));
        }
        Some((s, false)) if s == DraftStatus::Locked.as_str() => {
            return Err(AppError::locked_draft());
        }
        _ => {}
    }
    let siblings = live_scene_ids(c, &draft_id)?;
    c.execute(
        "UPDATE screenplay_scene SET deleted_at=NULL, updated_at=?1, rev=rev+1 WHERE id=?2",
        params![now_ms(), row.object_id],
    )?;
    place_at(
        c,
        "screenplay_scene",
        siblings,
        &row.object_id,
        row.position.map(|p| p.max(0) as usize),
    )?;
    bump_seq(c, &draft_id)?;
    for s in live_scene_ids(c, &draft_id)? {
        tx.reindex("screenplay_scene", &s);
    }
    Ok(())
}

fn purge_scene(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = &row.object_id;
    let used: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM breakdown_element WHERE scene_id=?1)",
        [id],
        |r| r.get(0),
    )?;
    if used {
        return Err(AppError::conflict(
            "This scene is referenced by the breakdown, so it can't be permanently deleted. Restore it instead.",
        ));
    }
    let mut stmt = c.prepare("SELECT id FROM screenplay_element WHERE scene_id=?1")?;
    let elements: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    comments::purge_for_targets(tx, "screenplay_element", &elements)?;
    comments::purge_for_targets(tx, "screenplay_scene", std::slice::from_ref(id))?;
    comments::purge_private_notes_for_targets(tx, "screenplay_element", &elements)?;
    comments::purge_private_notes_for_targets(tx, "screenplay_scene", std::slice::from_ref(id))?;
    c.execute("DELETE FROM screenplay_element WHERE scene_id=?1", [id])?;
    c.execute("DELETE FROM screenplay_scene WHERE id=?1", [id])?;
    Ok(())
}
