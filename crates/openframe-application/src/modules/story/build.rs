//! Build Screenplay from the Story Board (FSD §18, §91; FSD-STORY-020..025).
//!
//! The build reads active Scene Cards in exact board order, requires a valid
//! heading for every included card (supplied inline or excluded), and creates a
//! NEW screenplay or a NEW named draft — an existing script is never
//! overwritten. Afterwards the board and the script are independent objects.

use std::collections::{HashMap, HashSet};

use openframe_domain::enums::{DraftStatus, ElementType};
use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use openframe_persistence::rows::renumber;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::HEADING_MAX;
use super::board::location_label;
use super::tree::{self, StoryContainerRef, StoryContainerType, StoryItemKind, children, short};
use crate::core::AppCore;
use crate::modules::screenplay::{self, NewElement, NewScene, drafts::DraftSpec};
use crate::registry::Registry;
use crate::store::MutationMeta;
use crate::util::optional_text;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.query("story.build_preview", build_preview)
        .meta(M::compute(
            "Preview of Build Screenplay: which Scene Cards become scenes.",
        ));
    r.command("story.build_screenplay", build_screenplay)
        .meta(M::edit(
            "Build a NEW screenplay or NEW draft from the Story Board (never overwrites).",
        ));
    r.query("story.order_preview", order_preview)
        .meta(M::compute(
            "Preview of applying the Story Board order to a screenplay draft.",
        ));
    r.command("story.apply_order", apply_order).meta(M::edit("Reorder a screenplay draft's scenes to match the Story Board (confirmed when it renumbers).").confirm());
}

// ===================================================================== DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryBuildRow {
    pub card_id: String,
    /// 1-based order in the Story Board at this moment.
    #[ts(type = "number")]
    pub order: i64,
    pub short_description: String,
    pub scene_heading: Option<String>,
    /// False → the preview shows "Heading needed".
    pub heading_valid: bool,
    pub location: String,
    /// The card was already used to create a screenplay scene.
    pub already_built: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryScreenplayOption {
    pub id: String,
    pub title: String,
    #[ts(type = "number")]
    pub draft_count: i64,
    pub current_draft_id: Option<String>,
    pub current_draft_name: Option<String>,
    /// The current draft already contains written text beyond headings.
    pub has_written_scenes: bool,
    /// Name the next "from Story Board" draft would get.
    pub next_draft_name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryBuildPreview {
    pub rows: Vec<StoryBuildRow>,
    #[ts(type = "number")]
    pub parked_count: i64,
    #[ts(type = "number")]
    pub unassigned_count: i64,
    pub screenplays: Vec<StoryScreenplayOption>,
    pub default_title: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryBuildPreviewArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryBuildInclude {
    pub card_id: String,
    /// Heading supplied inline in the preview (saved onto the card).
    #[serde(default)]
    pub heading: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryDescriptionMode {
    /// Default (FSD §91.2): description becomes a scene planning note.
    #[default]
    PlanningNote,
    /// Description becomes editable temporary action text.
    ActionText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryBuildDestination {
    NewScreenplay,
    NewDraft,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryBuildArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
    pub include: Vec<StoryBuildInclude>,
    #[serde(default)]
    pub description_mode: StoryDescriptionMode,
    pub destination: StoryBuildDestination,
    /// Required for NewDraft.
    #[serde(default)]
    pub screenplay_id: Option<String>,
    /// Title for a new screenplay document.
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub draft_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryBuildResult {
    pub screenplay_id: String,
    pub draft_id: String,
    pub draft_name: String,
    pub first_scene_id: String,
    #[ts(type = "number")]
    pub scene_count: i64,
    pub created_screenplay: bool,
}

// ================================================================= helpers

/// Marks description text inserted as action (FSD §91.2: editable draft text,
/// not final prose). Notes never print.
pub const TEMPORARY_ACTION_NOTE: &str =
    "Temporary action text from the Story Board card — rewrite it as screenplay prose.";

fn element(element_type: ElementType, text: &str) -> NewElement {
    NewElement {
        element_type,
        text: text.to_string(),
        dual: false,
    }
}

/// A screenplay heading must start with a standard location prefix and name a place.
pub fn heading_is_valid(h: &str) -> bool {
    let u = h.trim().to_uppercase();
    const PREFIXES: &[&str] = &[
        "INT./EXT.",
        "EXT./INT.",
        "INT/EXT",
        "EXT/INT",
        "I/E",
        "INT.",
        "EXT.",
        "EST.",
        "INT ",
        "EXT ",
        "EST ",
    ];
    PREFIXES.iter().any(|p| {
        u.starts_with(p)
            && u[p.len()..]
                .trim_start_matches(['.', ' '])
                .chars()
                .any(|c| c.is_alphanumeric())
    })
}

/// Active Scene Cards (inside Acts/Sequences) in exact board order.
pub(crate) fn active_cards_in_order(
    c: &Connection,
    episode: Option<&str>,
) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    for act in tree::act_ids(c, episode)? {
        for it in children(c, &StoryContainerRef::act(&act), episode)? {
            match it.kind {
                StoryItemKind::Card => out.push(it.id),
                StoryItemKind::Sequence => {
                    for g in children(c, &StoryContainerRef::sequence(&it.id), episode)? {
                        if g.kind == StoryItemKind::Card {
                            out.push(g.id);
                        }
                    }
                }
                StoryItemKind::Beat => {}
            }
        }
    }
    Ok(out)
}

fn project_info(c: &Connection) -> AppResult<(String, String)> {
    Ok(
        c.query_row("SELECT title, project_type FROM project LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?,
    )
}

fn draft_has_written(c: &Connection, draft_id: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
          WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type NOT IN ('scene_heading','note')
            AND length(trim(e.text)) > 0)",
        [draft_id],
        |r| r.get(0),
    )?)
}

fn next_draft_name(c: &Connection, screenplay_id: &str) -> AppResult<String> {
    let n: i64 = c.query_row(
        "SELECT count(*) FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL",
        [screenplay_id],
        |r| r.get(0),
    )?;
    Ok(format!("Draft {} — from Story Board", n + 1))
}

pub(crate) fn screenplays_in_scope(
    c: &Connection,
    episode: Option<&str>,
) -> AppResult<Vec<StoryScreenplayOption>> {
    let rows: Vec<(String, String, Option<String>)> = {
        let mut stmt = c.prepare(
            "SELECT id, title, current_draft_id FROM screenplay WHERE episode_id IS ?1 AND deleted_at IS NULL
             ORDER BY updated_at DESC, id",
        )?;
        stmt.query_map([episode], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?
    };
    let mut out = Vec::new();
    for (id, title, current) in rows {
        let draft_count: i64 = c.query_row(
            "SELECT count(*) FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL",
            [&id],
            |r| r.get(0),
        )?;
        let current_name: Option<String> = match &current {
            Some(d) => c
                .query_row(
                    "SELECT name FROM screenplay_draft WHERE id=?1 AND deleted_at IS NULL",
                    [d],
                    |r| r.get(0),
                )
                .optional()?,
            None => None,
        };
        let has_written = match &current {
            Some(d) => draft_has_written(c, d)?,
            None => false,
        };
        out.push(StoryScreenplayOption {
            next_draft_name: next_draft_name(c, &id)?,
            id,
            title,
            draft_count,
            current_draft_id: current,
            current_draft_name: current_name,
            has_written_scenes: has_written,
        });
    }
    Ok(out)
}

fn default_title(c: &Connection, episode: Option<&str>) -> AppResult<String> {
    if let Some(e) = episode {
        let t: Option<String> = c
            .query_row("SELECT title FROM episode WHERE id=?1", [e], |r| r.get(0))
            .optional()?;
        if let Some(t) = t {
            return Ok(t);
        }
    }
    Ok(project_info(c)?.0)
}

// ================================================================== preview

fn build_preview(
    core: &AppCore,
    actor: &Actor,
    a: StoryBuildPreviewArgs,
) -> AppResult<StoryBuildPreview> {
    actor.require(Capability::View, "view the Story Board")?;
    core.project()?.store.read(|c| {
        let ep = a.episode_id.as_deref();
        tree::check_episode(c, ep)?;
        let ids = active_cards_in_order(c, ep)?;
        let mut rows = Vec::with_capacity(ids.len());
        for (i, id) in ids.iter().enumerate() {
            let (desc, heading, pt, pid, sid): (String, Option<String>, String, Option<String>, Option<String>) = c.query_row(
                "SELECT short_description, scene_heading, parent_type, parent_id, screenplay_scene_id FROM story_scene_card WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )?;
            let parent_type = StoryContainerType::parse(&pt).unwrap_or(StoryContainerType::Act);
            rows.push(StoryBuildRow {
                card_id: id.clone(),
                order: i as i64 + 1,
                short_description: desc,
                heading_valid: heading.as_deref().map(heading_is_valid).unwrap_or(false),
                scene_heading: heading,
                location: location_label(c, parent_type, pid.as_deref()),
                already_built: sid.is_some(),
            });
        }
        let parked_count: i64 = c.query_row(
            "SELECT count(*) FROM story_scene_card WHERE parent_type='parking' AND episode_id IS ?1 AND deleted_at IS NULL",
            [ep],
            |r| r.get(0),
        )?;
        let unassigned_count: i64 = c.query_row(
            "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND episode_id IS ?1 AND (parent_type='unassigned'
               OR (parent_type='sequence' AND parent_id IN (SELECT id FROM story_sequence WHERE act_id IS NULL)))",
            [ep],
            |r| r.get(0),
        )?;
        Ok(StoryBuildPreview {
            rows,
            parked_count,
            unassigned_count,
            screenplays: screenplays_in_scope(c, ep)?,
            default_title: default_title(c, ep)?,
        })
    })
}

// ==================================================================== build

fn build_screenplay(
    core: &AppCore,
    actor: &Actor,
    a: StoryBuildArgs,
) -> AppResult<StoryBuildResult> {
    if a.include.is_empty() {
        return Err(AppError::invalid_input(
            "Choose at least one Scene Card to build.",
        ));
    }
    if a.include.len() > 5000 {
        return Err(AppError::invalid_input("Too many cards selected."));
    }
    let title_arg = optional_text(a.title.clone(), "Screenplay title", 200)?;
    let draft_arg = optional_text(a.draft_name.clone(), "Draft name", 120)?;
    let s = core.project()?;
    let summary = format!(
        "Built screenplay from Story Board ({} scene{})",
        a.include.len(),
        if a.include.len() == 1 { "" } else { "s" }
    );
    s.store.mutate(actor, MutationMeta::new("story.build_screenplay", summary, Capability::Edit), |tx| {
        let c = tx.conn();
        let ep = a.episode_id.as_deref();
        tree::check_episode(c, ep)?;
        let order = active_cards_in_order(c, ep)?;
        let rank: HashMap<&str, usize> = order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
        let mut overrides: HashMap<String, Option<String>> = HashMap::new();
        for inc in &a.include {
            if !rank.contains_key(inc.card_id.as_str()) {
                return Err(AppError::invalid_input(
                    "Only Scene Cards in the active Story Board can be built. Parking Lot and deleted cards are excluded.",
                ));
            }
            let h = optional_text(inc.heading.clone(), "Scene heading", HEADING_MAX)?;
            overrides.insert(inc.card_id.clone(), h);
        }
        // Source order is exactly the Story Board order at the moment of build (FSD §91.3).
        let mut chosen: Vec<&str> = overrides.keys().map(|k| k.as_str()).collect();
        chosen.sort_by_key(|id| rank[id]);

        struct Planned {
            card_id: String,
            description: String,
            heading: String,
            save_heading: bool,
        }
        let mut planned = Vec::with_capacity(chosen.len());
        let mut missing = 0usize;
        for id in &chosen {
            let (desc, heading): (String, Option<String>) = c.query_row(
                "SELECT short_description, scene_heading FROM story_scene_card WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let supplied = overrides.get(*id).cloned().flatten();
            let (h, save) = match supplied {
                Some(s) => {
                    let changed = heading.as_deref() != Some(s.as_str());
                    (s, changed)
                }
                None => (heading.unwrap_or_default(), false),
            };
            if !heading_is_valid(&h) {
                missing += 1;
            }
            planned.push(Planned { card_id: id.to_string(), description: desc, heading: h, save_heading: save });
        }
        if missing > 0 {
            return Err(AppError::validation(
                "heading_needed",
                format!(
                    "{missing} Scene Card{} need{} a scene heading (for example INT. POLICE STATION — NIGHT) before {} can become screenplay scenes. Add a heading or exclude {}.",
                    if missing == 1 { "" } else { "s" },
                    if missing == 1 { "s" } else { "" },
                    if missing == 1 { "it" } else { "they" },
                    if missing == 1 { "it" } else { "them" },
                ),
            ));
        }

        let now = now_ms();
        let (screenplay_id, created) = match a.destination {
            StoryBuildDestination::NewDraft => {
                let sid = a
                    .screenplay_id
                    .clone()
                    .ok_or_else(|| AppError::invalid_input("Choose the screenplay that should receive the new draft."))?;
                let sep: Option<Option<String>> = c
                    .query_row("SELECT episode_id FROM screenplay WHERE id=?1 AND deleted_at IS NULL", [&sid], |r| r.get(0))
                    .optional()?;
                match sep {
                    None => return Err(AppError::not_found("screenplay")),
                    Some(e) if e.as_deref() != ep => {
                        return Err(AppError::invalid_input("That screenplay belongs to a different episode."));
                    }
                    Some(_) => {}
                }
                (sid, false)
            }
            StoryBuildDestination::NewScreenplay => {
                // One screenplay per project (or episode) — the Screenplay
                // workspace shows exactly that one. A repeat build therefore
                // becomes a new draft, never a hidden second document.
                if !screenplays_in_scope(c, ep)?.is_empty() {
                    return Err(AppError::validation(
                        "destination",
                        "This project already has a screenplay. Build a new draft of it instead — nothing is overwritten.",
                    ));
                }
                let title = title_arg.clone().unwrap_or(default_title(c, ep)?);
                (screenplay::create_screenplay_tx(tx, ep, &title)?, true)
            }
        };
        let draft_name = match draft_arg.clone() {
            Some(n) => n,
            None if created => "Draft 1 — from Story Board".to_string(),
            None => next_draft_name(c, &screenplay_id)?,
        };
        // Scene content (FSD §91.2): the heading lives on the scene row (the
        // Screenplay module never stores a `scene_heading` body element); the
        // card description is planning material — a scene note by default, or
        // clearly marked temporary action text.
        let scenes: Vec<NewScene> = planned
            .iter()
            .map(|p| {
                let desc = p.description.trim();
                let mut elements = Vec::with_capacity(3);
                match a.description_mode {
                    StoryDescriptionMode::PlanningNote => {
                        if !desc.is_empty() {
                            elements.push(element(ElementType::Note, desc));
                        }
                        elements.push(element(ElementType::Action, ""));
                    }
                    StoryDescriptionMode::ActionText => {
                        if !desc.is_empty() {
                            elements.push(element(ElementType::Note, TEMPORARY_ACTION_NOTE));
                        }
                        elements.push(element(ElementType::Action, desc));
                    }
                }
                NewScene {
                    heading: p.heading.trim().to_uppercase(),
                    synopsis: (!desc.is_empty()).then(|| desc.to_string()),
                    source_scene_card_id: Some(p.card_id.clone()),
                    elements,
                    ..Default::default()
                }
            })
            .collect();
        let draft_id = screenplay::insert_draft_tx(
            tx,
            &screenplay_id,
            DraftSpec {
                name: &draft_name,
                note: Some("Built from the Story Board."),
                status: DraftStatus::Draft,
                created_from: None,
                revision: None,
                // A new screenplay's only draft is current; a new draft of an
                // existing screenplay never replaces its current draft.
                make_current: created,
            },
            &scenes,
        )?;
        let built: Vec<(String, String)> = {
            let mut stmt = c.prepare(
                "SELECT id, source_scene_card_id FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
            )?;
            stmt.query_map([&draft_id], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?
        };
        let first_scene = built.first().map(|(id, _)| id.clone()).unwrap_or_default();
        let by_card: HashMap<&str, &str> = built.iter().map(|(s, card)| (card.as_str(), s.as_str())).collect();
        for p in &planned {
            if p.save_heading {
                c.execute(
                    "UPDATE story_scene_card SET scene_heading=?1, rev=rev+1, updated_at=?2 WHERE id=?3",
                    params![p.heading.trim(), now, p.card_id],
                )?;
            }
            if let Some(scene_id) = by_card.get(p.card_id.as_str()) {
                c.execute(
                    "UPDATE story_scene_card SET screenplay_scene_id=?1, rev=rev+1, updated_at=?2 WHERE id=?3",
                    params![scene_id, now, p.card_id],
                )?;
            }
        }
        c.execute("UPDATE screenplay SET updated_at=?1 WHERE id=?2", params![now, screenplay_id])?;
        Ok(StoryBuildResult {
            screenplay_id,
            draft_id,
            draft_name,
            first_scene_id: first_scene,
            scene_count: planned.len() as i64,
            created_screenplay: created,
        })
    })
}

// ======================================================= apply board order

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryOrderArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
    /// Target draft; None = current draft of the most recently edited screenplay in scope.
    #[serde(default)]
    pub draft_id: Option<String>,
    /// Must be true when the change would renumber written scenes (FSD-STORY-025).
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneMove {
    pub scene_id: String,
    pub heading: String,
    #[ts(type = "number")]
    pub from_number: i64,
    #[ts(type = "number")]
    pub to_number: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryOrderPreview {
    pub draft_id: Option<String>,
    pub draft_name: Option<String>,
    pub screenplay_title: Option<String>,
    pub locked: bool,
    pub has_written_scenes: bool,
    #[ts(type = "number")]
    pub linked_count: i64,
    pub moves: Vec<StorySceneMove>,
}

fn resolve_draft(
    c: &Connection,
    episode: Option<&str>,
    draft: Option<&str>,
) -> AppResult<Option<(String, String, String, String)>> {
    let draft_id = match draft {
        Some(d) => Some(d.to_string()),
        None => screenplays_in_scope(c, episode)?
            .into_iter()
            .find_map(|s| s.current_draft_id),
    };
    let Some(d) = draft_id else { return Ok(None) };
    let row: Option<(String, String, String)> = c
        .query_row(
            "SELECT d.name, d.status, p.title FROM screenplay_draft d JOIN screenplay p ON p.id=d.screenplay_id
             WHERE d.id=?1 AND d.deleted_at IS NULL AND p.deleted_at IS NULL AND p.episode_id IS ?2",
            params![d, episode],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        Some((name, status, title)) => Ok(Some((d, name, status, title))),
        None if draft.is_some() => Err(AppError::not_found("draft")),
        None => Ok(None),
    }
}

/// Current draft of the most recently edited screenplay in a scope (used by
/// character scene lists and the Story Timeline).
pub(crate) fn scope_current_draft(
    c: &Connection,
    episode: Option<&str>,
) -> AppResult<Option<String>> {
    Ok(screenplays_in_scope(c, episode)?
        .into_iter()
        .find_map(|s| s.current_draft_id))
}

/// Compute the new scene order: scenes linked to active cards take the board
/// order within the slots they already occupy; unlinked scenes keep their place.
fn planned_order(
    c: &Connection,
    episode: Option<&str>,
    draft_id: &str,
) -> AppResult<(Vec<(String, String)>, Vec<String>, i64)> {
    let scenes: Vec<(String, String, String)> = {
        let mut stmt = c.prepare(
            "SELECT id, lineage_id, heading FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        stmt.query_map([draft_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?
    };
    let by_lineage: HashMap<&str, &str> = scenes
        .iter()
        .map(|(id, l, _)| (l.as_str(), id.as_str()))
        .collect();
    let mut board_scenes: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for card in active_cards_in_order(c, episode)? {
        let sid: Option<String> = c.query_row(
            "SELECT screenplay_scene_id FROM story_scene_card WHERE id=?1",
            [&card],
            |r| r.get(0),
        )?;
        let Some(sid) = sid else { continue };
        let lineage: Option<String> = c
            .query_row(
                "SELECT lineage_id FROM screenplay_scene WHERE id=?1",
                [&sid],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(target) = lineage.as_deref().and_then(|l| by_lineage.get(l))
            && seen.insert(target.to_string())
        {
            board_scenes.push(target.to_string());
        }
    }
    let linked: HashSet<&str> = board_scenes.iter().map(|s| s.as_str()).collect();
    let mut next = board_scenes.iter();
    let new_order: Vec<String> = scenes
        .iter()
        .map(|(id, _, _)| {
            if linked.contains(id.as_str()) {
                next.next().cloned().unwrap_or_else(|| id.clone())
            } else {
                id.clone()
            }
        })
        .collect();
    let current: Vec<(String, String)> = scenes.into_iter().map(|(id, _, h)| (id, h)).collect();
    Ok((current, new_order, linked.len() as i64))
}

fn moves_for(current: &[(String, String)], new_order: &[String]) -> Vec<StorySceneMove> {
    let old_num: HashMap<&str, (i64, &str)> = current
        .iter()
        .enumerate()
        .map(|(i, (id, h))| (id.as_str(), (i as i64 + 1, h.as_str())))
        .collect();
    new_order
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            let (from, heading) = old_num[id.as_str()];
            let to = i as i64 + 1;
            (from != to).then(|| StorySceneMove {
                scene_id: id.clone(),
                heading: heading.to_string(),
                from_number: from,
                to_number: to,
            })
        })
        .collect()
}

fn order_preview(core: &AppCore, actor: &Actor, a: StoryOrderArgs) -> AppResult<StoryOrderPreview> {
    actor.require(Capability::View, "view the Story Board")?;
    core.project()?.store.read(|c| {
        let ep = a.episode_id.as_deref();
        let Some((draft_id, name, status, title)) = resolve_draft(c, ep, a.draft_id.as_deref())?
        else {
            return Ok(StoryOrderPreview {
                draft_id: None,
                draft_name: None,
                screenplay_title: None,
                locked: false,
                has_written_scenes: false,
                linked_count: 0,
                moves: vec![],
            });
        };
        let (current, new_order, linked) = planned_order(c, ep, &draft_id)?;
        Ok(StoryOrderPreview {
            has_written_scenes: draft_has_written(c, &draft_id)?,
            locked: status == "Locked",
            moves: moves_for(&current, &new_order),
            draft_id: Some(draft_id),
            draft_name: Some(name),
            screenplay_title: Some(title),
            linked_count: linked,
        })
    })
}

/// Explicitly apply the board order to a screenplay draft (FSD §18.6, HR-UX-003).
/// Requires confirmation whenever scenes would be renumbered.
fn apply_order(core: &AppCore, actor: &Actor, a: StoryOrderArgs) -> AppResult<StoryOrderPreview> {
    let s = core.project()?;
    let ep = a.episode_id.clone();
    let (draft_id, name) = s.store.read(|c| {
        resolve_draft(c, ep.as_deref(), a.draft_id.as_deref())?
            .map(|(d, n, _, _)| (d, n))
            .ok_or_else(|| AppError::not_found("screenplay"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.apply_order", format!("Applied Story Board order to “{}”", short(&name)), Capability::Edit)
            .target("screenplay_draft", &draft_id),
        |tx| {
            let c = tx.conn();
            let status: String = c.query_row("SELECT status FROM screenplay_draft WHERE id=?1", [&draft_id], |r| r.get(0))?;
            if status == "Locked" {
                return Err(AppError::locked_draft());
            }
            let (current, new_order, linked) = planned_order(c, ep.as_deref(), &draft_id)?;
            let moves = moves_for(&current, &new_order);
            if !moves.is_empty() && !a.confirmed {
                return Err(AppError::new(
                    "conflict.confirm_required",
                    "Reordering these cards will change the screenplay scene order. Confirm to apply it.",
                ));
            }
            renumber(c, "screenplay_scene", &new_order)?;
            Ok(StoryOrderPreview {
                draft_id: Some(draft_id.clone()),
                draft_name: Some(name.clone()),
                screenplay_title: None,
                locked: false,
                has_written_scenes: draft_has_written(c, &draft_id)?,
                linked_count: linked,
                moves,
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::heading_is_valid;

    #[test]
    fn headings() {
        assert!(heading_is_valid("INT. POLICE STATION — NIGHT"));
        assert!(heading_is_valid("ext. street - day"));
        assert!(heading_is_valid("INT./EXT. CAR - MOVING"));
        assert!(heading_is_valid("I/E TRAIN"));
        assert!(!heading_is_valid("INT."));
        assert!(!heading_is_valid(""));
        assert!(!heading_is_valid("Bus station at night"));
        assert!(!heading_is_valid("INTERIOR"));
    }
}
