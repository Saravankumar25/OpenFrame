//! Story Board: acts, sequences, beats, scene cards, parking lot, multi-select
//! operations and the board read model (FSD §7–12, §51, §89–90).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{opt_text, renumber, text, update_fields};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tree::{
    self, StoryContainerRef, StoryContainerType, StoryItemKind, StoryItemRef, children,
    container_episode, place, short,
};
use super::{
    DESC_MAX_BYTES, HEADING_MAX, NOTES_MAX_BYTES, StoryCreated, StoryIdArgs, TITLE_MAX,
    clean_color, ensure_live,
};
use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{DeleteSpec, MutationMeta, Tx, soft_delete};
use crate::util::{AssetInfo, body_text, ingest_file, load_asset, optional_text, required_text};

pub fn register(r: &mut Registry) {
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    r.query("story.board", board).meta(M::read(
        "Story Board: acts, sequences, beats, Scene Cards, Parking Lot and Unassigned.",
    ));
    r.query("story.card", card_detail).meta(M::read(
        "One Scene Card with notes, linked characters, attachments and its screenplay scene.",
    ));
    r.query("story.view_state", view_state).meta(
        M::read("The user's Story Board view state (collapsed rows, current episode).")
            .hidden(h::VIEW_STATE),
    );
    r.command("story.set_collapsed", set_collapsed).meta(
        M::command(
            openframe_domain::Capability::View,
            "Collapse or expand an act or sequence.",
        )
        .hidden(h::VIEW_STATE),
    );
    r.command("story.set_current_episode", set_current_episode)
        .meta(
            M::command(
                openframe_domain::Capability::View,
                "Choose which episode the Story Board shows.",
            )
            .hidden(h::VIEW_STATE),
        );
    r.command("story.create_act", create_act)
        .meta(M::edit("Create an act on the Story Board."));
    r.command("story.update_act", update_act)
        .meta(M::edit("Rename an act or edit its note."));
    r.command("story.move_act", move_act)
        .meta(M::edit("Move an act to another position."));
    r.command("story.delete_act", delete_act).meta(
        M::soft_delete(
            "Delete an act (move its contents out, or delete everything inside; recoverable).",
        )
        .confirm(),
    );
    r.command("story.create_sequence", create_sequence)
        .meta(M::edit("Create a sequence inside an act."));
    r.command("story.update_sequence", update_sequence)
        .meta(M::edit("Rename a sequence or edit its note."));
    r.command("story.delete_sequence", delete_sequence).meta(
        M::soft_delete(
            "Delete a sequence (move its contents out, or delete everything inside; recoverable).",
        )
        .confirm(),
    );
    r.command("story.create_beat", create_beat)
        .meta(M::edit("Create a story beat."));
    r.command("story.update_beat", update_beat)
        .meta(M::edit("Edit a beat's text, note or colour."));
    r.command("story.convert_beat", convert_beat)
        .meta(M::edit("Convert a beat into a Scene Card."));
    r.command("story.create_card", create_card)
        .meta(M::edit("Create a Scene Card."));
    r.command("story.update_card", update_card).meta(M::edit(
        "Edit a Scene Card's description, heading, notes or colour.",
    ));
    r.command("story.card_to_beat", card_to_beat)
        .meta(M::edit("Convert a Scene Card back into a beat."));
    r.command("story.move_items", move_items).meta(M::edit(
        "Move sequences, beats or Scene Cards to another act, sequence, Parking Lot or Unassigned.",
    ));
    r.command("story.duplicate_items", duplicate_items)
        .meta(M::edit("Duplicate sequences, beats or Scene Cards."));
    r.command("story.park_items", park_items)
        .meta(M::edit("Move beats or Scene Cards to the Parking Lot."));
    r.command("story.restore_from_parking", restore_from_parking)
        .meta(M::edit(
            "Return parked beats or Scene Cards to where they came from.",
        ));
    r.command("story.delete_items", delete_items).meta(
        M::soft_delete("Move sequences, beats or Scene Cards to Recently Deleted.").confirm(),
    );
    r.command("story.add_attachment", add_attachment).meta(
        M::edit("Attach a file the user picked to a Scene Card, beat or sequence.")
            .fs(Fs::ReadsUserFile)
            .hidden(h::USER_PATH),
    );
    r.command("story.remove_attachment", remove_attachment)
        .meta(M::edit(
            "Remove an attachment from a Scene Card, beat or sequence.",
        ));
}

// ===================================================================== DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryAttachmentDto {
    pub id: String,
    pub asset: AssetInfo,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorySceneCardDto {
    pub id: String,
    pub episode_id: Option<String>,
    pub parent_type: StoryContainerType,
    pub parent_id: Option<String>,
    pub short_description: String,
    pub scene_heading: Option<String>,
    pub notes: Option<String>,
    pub color: Option<String>,
    /// "Used to create screenplay" reference — informational only (FSD §11.10).
    pub screenplay_scene_id: Option<String>,
    pub source_beat_id: Option<String>,
    pub parked_from_type: Option<String>,
    pub parked_from_id: Option<String>,
    #[ts(type = "number")]
    pub comment_count: i64,
    pub attachments: Vec<StoryAttachmentDto>,
    pub character_ids: Vec<String>,
    #[ts(type = "number")]
    pub rev: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryBeatDto {
    pub id: String,
    pub episode_id: Option<String>,
    pub parent_type: StoryContainerType,
    pub parent_id: Option<String>,
    pub text: String,
    pub note: Option<String>,
    pub color: Option<String>,
    /// "active" or "converted" (kept as a reference after Convert to Scene).
    pub state: String,
    pub converted_scene_card_id: Option<String>,
    pub parked_from_type: Option<String>,
    pub parked_from_id: Option<String>,
    #[ts(type = "number")]
    pub comment_count: i64,
    pub attachments: Vec<StoryAttachmentDto>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StorySequenceDto {
    pub id: String,
    pub act_id: Option<String>,
    pub episode_id: Option<String>,
    pub title: String,
    pub note: Option<String>,
    #[ts(type = "number")]
    pub card_count: i64,
    pub items: Vec<StoryItem>,
    pub attachments: Vec<StoryAttachmentDto>,
    #[ts(type = "number")]
    pub comment_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StoryItem {
    Sequence(StorySequenceDto),
    Beat(StoryBeatDto),
    Card(StorySceneCardDto),
}

impl StoryItem {
    fn card_count(&self) -> i64 {
        match self {
            StoryItem::Sequence(s) => s.card_count,
            StoryItem::Card(_) => 1,
            StoryItem::Beat(_) => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryActDto {
    pub id: String,
    pub episode_id: Option<String>,
    pub title: String,
    pub note: Option<String>,
    /// Scene cards inside the act (directly or in its sequences).
    #[ts(type = "number")]
    pub card_count: i64,
    pub items: Vec<StoryItem>,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryBoardState {
    pub episode_id: Option<String>,
    pub acts: Vec<StoryActDto>,
    pub parking: Vec<StoryItem>,
    /// Restored items whose Act/Sequence no longer exists.
    pub unassigned: Vec<StoryItem>,
    /// Acts/sequences this user has collapsed (per-user view state, not story data).
    pub collapsed_ids: Vec<String>,
    #[ts(type = "number")]
    pub card_count: i64,
    #[ts(type = "number")]
    pub beat_count: i64,
    #[ts(type = "number")]
    pub sequence_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryLinkedScene {
    pub scene_id: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub draft_id: String,
    pub draft_name: String,
    /// Derived from screenplay order; never stored.
    #[ts(type = "number")]
    pub number: i64,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCardDetail {
    pub card: StorySceneCardDto,
    /// e.g. "Act 1 — The Return › Railway Station Return" or "Parking Lot".
    pub location: String,
    pub linked_scene: Option<StoryLinkedScene>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryViewState {
    #[serde(default)]
    pub collapsed_ids: Vec<String>,
    /// Last episode the user worked on (episodic projects).
    #[serde(default)]
    pub episode_id: Option<String>,
    /// True once the user explicitly picked an episode (so None means "series level").
    #[serde(default)]
    pub episode_chosen: bool,
}

// ===================================================================== args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryScopeArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCollapseArgs {
    pub id: String,
    pub collapsed: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryEpisodeChoiceArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateActArgs {
    #[serde(default)]
    pub episode_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub note: Option<String>,
    /// Insert before this act; None = at the end of the Story Board (FSD §8.1).
    #[serde(default)]
    pub before_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateActArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    /// Empty string clears the note.
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryMoveActArgs {
    pub id: String,
    /// Place before this act; None = last.
    #[serde(default)]
    pub before_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryDeleteMode {
    /// Recommended: move the contents out, then delete only the empty container.
    MoveContents,
    /// Delete the container and everything inside it (recoverable together).
    DeleteAll,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryDeleteActArgs {
    pub id: String,
    #[serde(default)]
    pub mode: Option<StoryDeleteMode>,
    /// Destination Act for MoveContents; None = Unassigned area.
    #[serde(default)]
    pub move_to_act_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateSequenceArgs {
    pub act_id: String,
    pub title: String,
    /// Index among the act's children; None = end.
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateSequenceArgs {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryDeleteSequenceArgs {
    pub id: String,
    #[serde(default)]
    pub mode: Option<StoryDeleteMode>,
    /// Destination for MoveContents; None = the sequence's own Act, at its place.
    #[serde(default)]
    pub move_to: Option<StoryContainerRef>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateBeatArgs {
    /// Act, Sequence or Parking Lot; None = Parking Lot.
    #[serde(default)]
    pub parent: Option<StoryContainerRef>,
    #[serde(default)]
    pub episode_id: Option<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateBeatArgs {
    pub id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    /// Empty string clears the colour.
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCreateCardArgs {
    /// Act, Sequence or Parking Lot; None = end of the last Act (an Act is created if none exists).
    #[serde(default)]
    pub parent: Option<StoryContainerRef>,
    #[serde(default)]
    pub episode_id: Option<String>,
    /// May be blank while the user is typing the new card (FSD §11.2).
    #[serde(default)]
    pub short_description: String,
    #[serde(default)]
    pub scene_heading: Option<String>,
    /// Drop position within the container; None = end.
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryUpdateCardArgs {
    pub id: String,
    #[serde(default)]
    pub short_description: Option<String>,
    /// Empty string clears the heading.
    #[serde(default)]
    pub scene_heading: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(type = "number | null")]
    pub expected_rev: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryMoveArgs {
    pub items: Vec<StoryItemRef>,
    pub target: StoryContainerRef,
    /// Insert before this sibling; None = at the end of the target.
    #[serde(default)]
    pub before: Option<StoryItemRef>,
    /// Scope for Parking Lot / Unassigned targets.
    #[serde(default)]
    pub episode_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryItemsArgs {
    pub items: Vec<StoryItemRef>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryPlacement {
    pub item: StoryItemRef,
    pub container: StoryContainerRef,
    /// Human description of where it went, e.g. "Sequence “Chase”" or "Unassigned".
    pub label: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryAttachArgs {
    /// "scene_card", "beat" or "sequence".
    pub owner_type: String,
    pub owner_id: String,
    pub path: String,
}

// ================================================================ read model

type Key = (String, Option<String>);

fn attachments_map(
    c: &Connection,
    root: &Path,
) -> AppResult<HashMap<(String, String), Vec<StoryAttachmentDto>>> {
    let mut stmt = c.prepare_cached(
        "SELECT id, owner_type, owner_id, asset_id FROM story_attachment ORDER BY position, id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out: HashMap<(String, String), Vec<StoryAttachmentDto>> = HashMap::new();
    for (id, ot, oid, asset_id) in rows {
        if let Ok(asset) = load_asset(c, root, &asset_id) {
            out.entry((ot, oid))
                .or_default()
                .push(StoryAttachmentDto { id, asset });
        }
    }
    Ok(out)
}

fn comment_counts(c: &Connection) -> AppResult<HashMap<String, i64>> {
    let mut stmt = c.prepare_cached(
        "SELECT target_id, count(*) FROM comment
         WHERE target_type IN ('story_scene_card','scene_card','story_beat','beat','story_sequence','sequence')
           AND deleted_at IS NULL AND parent_id IS NULL AND status <> 'Resolved'
         GROUP BY target_id",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows.into_iter().collect())
}

fn character_links(c: &Connection) -> AppResult<HashMap<String, Vec<String>>> {
    let mut stmt = c.prepare_cached(
        "SELECT l.scene_card_id, l.character_id FROM story_character_card_link l
         JOIN story_character ch ON ch.id = l.character_id
         WHERE ch.deleted_at IS NULL ORDER BY ch.position, ch.name",
    )?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (card, ch) in rows {
        out.entry(card).or_default().push(ch);
    }
    Ok(out)
}

struct Extras {
    attachments: HashMap<(String, String), Vec<StoryAttachmentDto>>,
    comments: HashMap<String, i64>,
    links: HashMap<String, Vec<String>>,
}

impl Extras {
    fn load(c: &Connection, root: &Path) -> AppResult<Extras> {
        Ok(Extras {
            attachments: attachments_map(c, root)?,
            comments: comment_counts(c)?,
            links: character_links(c)?,
        })
    }
    fn att(&self, owner_type: &str, id: &str) -> Vec<StoryAttachmentDto> {
        self.attachments
            .get(&(owner_type.to_string(), id.to_string()))
            .cloned()
            .unwrap_or_default()
    }
}

const CARD_COLS: &str = "id, episode_id, parent_type, parent_id, short_description, scene_heading, notes, color,
    screenplay_scene_id, source_beat_id, parked_from_type, parked_from_id, rev, created_at, updated_at, position";

fn map_card(r: &rusqlite::Row<'_>, x: &Extras) -> rusqlite::Result<(i64, StorySceneCardDto)> {
    let id: String = r.get(0)?;
    let pt: String = r.get(2)?;
    Ok((
        r.get(15)?,
        StorySceneCardDto {
            episode_id: r.get(1)?,
            parent_type: StoryContainerType::parse(&pt).unwrap_or(StoryContainerType::Unassigned),
            parent_id: r.get(3)?,
            short_description: r.get(4)?,
            scene_heading: r.get(5)?,
            notes: r.get(6)?,
            color: r.get(7)?,
            screenplay_scene_id: r.get(8)?,
            source_beat_id: r.get(9)?,
            parked_from_type: r.get(10)?,
            parked_from_id: r.get(11)?,
            comment_count: x.comments.get(&id).copied().unwrap_or(0),
            attachments: x.att("scene_card", &id),
            character_ids: x.links.get(&id).cloned().unwrap_or_default(),
            rev: r.get(12)?,
            created_at: r.get(13)?,
            updated_at: r.get(14)?,
            id,
        },
    ))
}

const BEAT_COLS: &str = "id, episode_id, parent_type, parent_id, text, note, color, state, converted_scene_card_id, parked_from_type, parked_from_id, rev, position";

fn map_beat(r: &rusqlite::Row<'_>, x: &Extras) -> rusqlite::Result<(i64, StoryBeatDto)> {
    let id: String = r.get(0)?;
    let pt: String = r.get(2)?;
    Ok((
        r.get(12)?,
        StoryBeatDto {
            episode_id: r.get(1)?,
            parent_type: StoryContainerType::parse(&pt).unwrap_or(StoryContainerType::Unassigned),
            parent_id: r.get(3)?,
            text: r.get(4)?,
            note: r.get(5)?,
            color: r.get(6)?,
            state: r.get(7)?,
            converted_scene_card_id: r.get(8)?,
            parked_from_type: r.get(9)?,
            parked_from_id: r.get(10)?,
            comment_count: x.comments.get(&id).copied().unwrap_or(0),
            attachments: x.att("beat", &id),
            rev: r.get(11)?,
            id,
        },
    ))
}

pub(crate) fn load_card(c: &Connection, root: &Path, id: &str) -> AppResult<StorySceneCardDto> {
    let x = Extras::load(c, root)?;
    c.query_row(
        &format!("SELECT {CARD_COLS} FROM story_scene_card WHERE id=?1 AND deleted_at IS NULL"),
        [id],
        |r| map_card(r, &x),
    )
    .optional()?
    .map(|(_, d)| d)
    .ok_or_else(|| AppError::not_found("scene card"))
}

fn rank(item: &StoryItem) -> u8 {
    match item {
        StoryItem::Sequence(_) => 0,
        StoryItem::Beat(_) => 1,
        StoryItem::Card(_) => 2,
    }
}

fn item_id(item: &StoryItem) -> &str {
    match item {
        StoryItem::Sequence(s) => &s.id,
        StoryItem::Beat(b) => &b.id,
        StoryItem::Card(c) => &c.id,
    }
}

fn take_sorted(map: &mut HashMap<Key, Vec<(i64, StoryItem)>>, key: &Key) -> Vec<StoryItem> {
    let mut v = map.remove(key).unwrap_or_default();
    v.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(rank(&a.1).cmp(&rank(&b.1)))
            .then(item_id(&a.1).cmp(item_id(&b.1)))
    });
    v.into_iter().map(|(_, i)| i).collect()
}

pub(crate) fn load_board(
    c: &Connection,
    root: &Path,
    episode: Option<&str>,
) -> AppResult<StoryBoardState> {
    let x = Extras::load(c, root)?;
    let mut map: HashMap<Key, Vec<(i64, StoryItem)>> = HashMap::new();
    let mut card_count = 0;
    let mut beat_count = 0;
    {
        let mut stmt = c.prepare_cached(&format!(
            "SELECT {CARD_COLS} FROM story_scene_card WHERE episode_id IS ?1 AND deleted_at IS NULL"
        ))?;
        let rows = stmt
            .query_map([episode], |r| map_card(r, &x))?
            .collect::<Result<Vec<_>, _>>()?;
        for (pos, card) in rows {
            card_count += 1;
            let key = (
                card.parent_type.as_str().to_string(),
                card.parent_id.clone(),
            );
            map.entry(key)
                .or_default()
                .push((pos, StoryItem::Card(card)));
        }
    }
    {
        let mut stmt = c.prepare_cached(&format!(
            "SELECT {BEAT_COLS} FROM story_beat WHERE episode_id IS ?1 AND deleted_at IS NULL"
        ))?;
        let rows = stmt
            .query_map([episode], |r| map_beat(r, &x))?
            .collect::<Result<Vec<_>, _>>()?;
        for (pos, beat) in rows {
            beat_count += 1;
            let key = (
                beat.parent_type.as_str().to_string(),
                beat.parent_id.clone(),
            );
            map.entry(key)
                .or_default()
                .push((pos, StoryItem::Beat(beat)));
        }
    }
    let seqs: Vec<(
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        i64,
        i64,
    )> = {
        let mut stmt = c.prepare_cached(
            "SELECT id, act_id, episode_id, title, note, rev, position FROM story_sequence
             WHERE episode_id IS ?1 AND deleted_at IS NULL",
        )?;
        stmt.query_map([episode], |r| {
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
        .collect::<Result<_, _>>()?
    };
    let sequence_count = seqs.len() as i64;
    for (id, act_id, ep, title, note, rev, pos) in seqs {
        let items = take_sorted(&mut map, &("sequence".to_string(), Some(id.clone())));
        let cc = items.iter().map(|i| i.card_count()).sum();
        let key = match &act_id {
            Some(a) => ("act".to_string(), Some(a.clone())),
            None => ("unassigned".to_string(), None),
        };
        let dto = StorySequenceDto {
            attachments: x.att("sequence", &id),
            comment_count: x.comments.get(&id).copied().unwrap_or(0),
            id,
            act_id,
            episode_id: ep,
            title,
            note,
            card_count: cc,
            items,
            rev,
        };
        map.entry(key)
            .or_default()
            .push((pos, StoryItem::Sequence(dto)));
    }
    let acts: Vec<(String, Option<String>, String, Option<String>, i64)> = {
        let mut stmt = c.prepare_cached(
            "SELECT id, episode_id, title, note, rev FROM story_act WHERE episode_id IS ?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        stmt.query_map([episode], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?
    };
    let acts = acts
        .into_iter()
        .map(|(id, ep, title, note, rev)| {
            let items = take_sorted(&mut map, &("act".to_string(), Some(id.clone())));
            StoryActDto {
                card_count: items.iter().map(|i| i.card_count()).sum(),
                id,
                episode_id: ep,
                title,
                note,
                items,
                rev,
            }
        })
        .collect();
    let parking = take_sorted(&mut map, &("parking".to_string(), None));
    let unassigned = take_sorted(&mut map, &("unassigned".to_string(), None));
    Ok(StoryBoardState {
        episode_id: episode.map(|s| s.to_string()),
        acts,
        parking,
        unassigned,
        collapsed_ids: vec![],
        card_count,
        beat_count,
        sequence_count,
    })
}

fn read_view_state(c: &Connection, user_id: &str) -> AppResult<StoryViewState> {
    let json: Option<String> = c
        .query_row(
            "SELECT value_json FROM sys_view_state WHERE user_id=?1 AND key='story.view'",
            [user_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

fn write_view_state(tx: &Tx<'_>, state: &StoryViewState) -> AppResult<()> {
    let json = serde_json::to_string(state).map_err(|e| AppError::internal(e.to_string()))?;
    tx.conn().execute(
        "INSERT INTO sys_view_state(user_id, key, value_json, updated_at) VALUES (?1, 'story.view', ?2, ?3)
         ON CONFLICT(user_id, key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
        params![tx.actor().user_id, json, now_ms()],
    )?;
    Ok(())
}

fn board(core: &AppCore, actor: &Actor, a: StoryScopeArgs) -> AppResult<StoryBoardState> {
    actor.require(Capability::View, "view the Story Board")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let mut b = load_board(c, &root, a.episode_id.as_deref())?;
        b.collapsed_ids = read_view_state(c, &actor.user_id)?.collapsed_ids;
        Ok(b)
    })
}

pub(crate) fn linked_scene(c: &Connection, scene_id: &str) -> AppResult<Option<StoryLinkedScene>> {
    let row: Option<(String, String, String, String, String, i64)> = c
        .query_row(
            "SELECT s.draft_id, d.screenplay_id, p.title, d.name, s.heading, s.position
             FROM screenplay_scene s JOIN screenplay_draft d ON d.id = s.draft_id JOIN screenplay p ON p.id = d.screenplay_id
             WHERE s.id=?1 AND s.deleted_at IS NULL AND d.deleted_at IS NULL AND p.deleted_at IS NULL",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()?;
    let Some((draft_id, screenplay_id, screenplay_title, draft_name, heading, pos)) = row else {
        return Ok(None);
    };
    let number: i64 = c.query_row(
        "SELECT count(*) FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL AND (position < ?2 OR (position = ?2 AND id <= ?3))",
        params![draft_id, pos, scene_id],
        |r| r.get(0),
    )?;
    Ok(Some(StoryLinkedScene {
        scene_id: scene_id.to_string(),
        screenplay_id,
        screenplay_title,
        draft_id,
        draft_name,
        number,
        heading,
    }))
}

pub(crate) fn location_label(
    c: &Connection,
    parent_type: StoryContainerType,
    parent_id: Option<&str>,
) -> String {
    let title = |sql: &str, id: &str| -> String {
        c.query_row(sql, [id], |r| r.get::<_, String>(0))
            .unwrap_or_default()
    };
    match (parent_type, parent_id) {
        (StoryContainerType::Act, Some(id)) => title("SELECT title FROM story_act WHERE id=?1", id),
        (StoryContainerType::Sequence, Some(id)) => {
            let row: Option<(String, Option<String>)> = c
                .query_row(
                    "SELECT title, act_id FROM story_sequence WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok();
            match row {
                Some((seq, Some(act))) => format!(
                    "{} › {}",
                    title("SELECT title FROM story_act WHERE id=?1", &act),
                    seq
                ),
                Some((seq, None)) => format!("Unassigned › {seq}"),
                None => String::new(),
            }
        }
        (StoryContainerType::Parking, _) => "Parking Lot".to_string(),
        _ => "Unassigned".to_string(),
    }
}

fn card_detail(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<StoryCardDetail> {
    actor.require(Capability::View, "view the Story Board")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let card = load_card(c, &root, &a.id)?;
        let location = location_label(c, card.parent_type, card.parent_id.as_deref());
        let linked_scene = match &card.screenplay_scene_id {
            Some(sid) => linked_scene(c, sid)?,
            None => None,
        };
        Ok(StoryCardDetail {
            card,
            location,
            linked_scene,
        })
    })
}

fn view_state(core: &AppCore, actor: &Actor, _: StoryScopeArgs) -> AppResult<StoryViewState> {
    actor.require(Capability::View, "view the Story Board")?;
    core.project()?
        .store
        .read(|c| read_view_state(c, &actor.user_id))
}

/// Collapse/expand is per-user view state (FSD-STORY-018/019: hides descendants
/// without changing story data, and never enters the undo history).
fn set_collapsed(core: &AppCore, actor: &Actor, a: StoryCollapseArgs) -> AppResult<StoryViewState> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.set_collapsed", "Collapsed story container", Capability::View).not_undoable().quiet(),
        |tx| {
            let mut st = read_view_state(tx.conn(), &actor.user_id)?;
            st.collapsed_ids.retain(|x| x != &a.id);
            if a.collapsed {
                st.collapsed_ids.push(a.id.clone());
            }
            // Forget ids of containers that no longer exist.
            let c = tx.conn();
            st.collapsed_ids.retain(|id| {
                c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM story_act WHERE id=?1) OR EXISTS(SELECT 1 FROM story_sequence WHERE id=?1)",
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .unwrap_or(false)
            });
            write_view_state(tx, &st)?;
            Ok(st)
        },
    )
}

fn set_current_episode(
    core: &AppCore,
    actor: &Actor,
    a: StoryEpisodeChoiceArgs,
) -> AppResult<StoryViewState> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.set_current_episode",
            "Switched episode",
            Capability::View,
        )
        .not_undoable()
        .quiet(),
        |tx| {
            tree::check_episode(tx.conn(), a.episode_id.as_deref())?;
            let mut st = read_view_state(tx.conn(), &actor.user_id)?;
            st.episode_id = a.episode_id.clone();
            st.episode_chosen = true;
            write_view_state(tx, &st)?;
            Ok(st)
        },
    )
}

// ===================================================================== acts

fn act_scope(c: &Connection, id: &str) -> AppResult<(String, Option<String>)> {
    c.query_row(
        "SELECT title, episode_id FROM story_act WHERE id=?1 AND deleted_at IS NULL",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("act"))
}

fn place_act(
    c: &Connection,
    id: &str,
    episode: Option<&str>,
    before: Option<&str>,
) -> AppResult<()> {
    let mut ids = tree::act_ids(c, episode)?;
    ids.retain(|x| x != id);
    let idx = match before {
        Some(b) => ids
            .iter()
            .position(|x| x == b)
            .ok_or_else(|| AppError::not_found("act"))?,
        None => ids.len(),
    };
    ids.insert(idx, id.to_string());
    renumber(c, "story_act", &ids)
}

/// Insert a new act and return its id (used by create_card when the board is empty).
fn insert_act(
    c: &Connection,
    episode: Option<&str>,
    title: &str,
    note: Option<&str>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let pos = tree::act_ids(c, episode)?.len() as i64 + 1;
    c.execute(
        "INSERT INTO story_act(id, episode_id, title, note, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![id, episode, title, note, pos, now],
    )?;
    Ok(id)
}

fn create_act(core: &AppCore, actor: &Actor, a: StoryCreateActArgs) -> AppResult<StoryCreated> {
    let title = required_text(&a.title, "Act title", TITLE_MAX)?;
    let note = clean_note(a.note)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.create_act",
            format!("Added Act “{}”", short(&title)),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            tree::check_episode(c, a.episode_id.as_deref())?;
            let id = insert_act(c, a.episode_id.as_deref(), &title, note.as_deref())?;
            if a.before_id.is_some() {
                place_act(c, &id, a.episode_id.as_deref(), a.before_id.as_deref())?;
            }
            Ok(StoryCreated { id })
        },
    )
}

fn clean_note(note: Option<String>) -> AppResult<Option<String>> {
    match note {
        Some(n) if n.trim().is_empty() => Ok(None),
        Some(n) => Ok(Some(body_text(n, "Note", NOTES_MAX_BYTES)?)),
        None => Ok(None),
    }
}

/// Note field for updates: None = unchanged; "" = clear.
fn note_value(note: &Option<String>, what: &str) -> AppResult<Option<SqlValue>> {
    Ok(match note {
        None => None,
        Some(n) if n.trim().is_empty() => Some(SqlValue::Null),
        Some(n) => Some(text(body_text(n.clone(), what, NOTES_MAX_BYTES)?)),
    })
}

fn update_act(core: &AppCore, actor: &Actor, a: StoryUpdateActArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut summary = "Edited Act note".to_string();
    if let Some(t) = &a.title {
        let t = required_text(t, "Act title", TITLE_MAX)?;
        summary = format!("Renamed Act to “{}”", short(&t));
        fields.push(("title", text(t)));
    }
    if let Some(v) = note_value(&a.note, "Note")? {
        fields.push(("note", v));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.update_act", summary, Capability::Edit)
            .target("story_act", &a.id)
            .coalesce(format!("story.act:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "story_act", &a.id, "act")?;
            update_fields(
                tx.conn(),
                "story_act",
                &a.id,
                &fields,
                &["title", "note"],
                a.expected_rev,
                "act",
            )?;
            Ok(())
        },
    )
}

fn move_act(core: &AppCore, actor: &Actor, a: StoryMoveActArgs) -> AppResult<()> {
    let s = core.project()?;
    let (title, ep) = s.store.read(|c| act_scope(c, &a.id))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.move_act",
            format!("Moved Act “{}”", short(&title)),
            Capability::Edit,
        )
        .target("story_act", &a.id),
        |tx| place_act(tx.conn(), &a.id, ep.as_deref(), a.before_id.as_deref()),
    )
}

/// Soft-delete everything inside an act with the act's own deleted_at so the
/// act restores/purges as one unit.
fn cascade_act(c: &Connection, act_id: &str, at: i64) -> AppResult<()> {
    for t in ["story_beat", "story_scene_card"] {
        c.execute(
            &format!(
                "UPDATE {t} SET deleted_at=?2, updated_at=?2, rev=rev+1 WHERE deleted_at IS NULL AND (
                    (parent_type='act' AND parent_id=?1) OR
                    (parent_type='sequence' AND parent_id IN (SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL)))"
            ),
            params![act_id, at],
        )?;
    }
    c.execute(
        "UPDATE story_sequence SET deleted_at=?2, updated_at=?2, rev=rev+1 WHERE act_id=?1 AND deleted_at IS NULL",
        params![act_id, at],
    )?;
    Ok(())
}

fn cascade_sequence(c: &Connection, seq_id: &str, at: i64) -> AppResult<()> {
    for t in ["story_beat", "story_scene_card"] {
        c.execute(
            &format!(
                "UPDATE {t} SET deleted_at=?2, updated_at=?2, rev=rev+1
                 WHERE deleted_at IS NULL AND parent_type='sequence' AND parent_id=?1"
            ),
            params![seq_id, at],
        )?;
    }
    Ok(())
}

fn deleted_at(c: &Connection, table: &str, id: &str) -> AppResult<i64> {
    Ok(c.query_row(
        &format!("SELECT deleted_at FROM {table} WHERE id=?1"),
        [id],
        |r| r.get(0),
    )?)
}

fn delete_act(core: &AppCore, actor: &Actor, a: StoryDeleteActArgs) -> AppResult<()> {
    let s = core.project()?;
    let (title, ep) = s.store.read(|c| act_scope(c, &a.id))?;
    let summary = match a.mode {
        Some(StoryDeleteMode::DeleteAll) => {
            format!("Deleted Act “{}” and everything inside it", short(&title))
        }
        _ => format!("Deleted Act “{}”", short(&title)),
    };
    s.store.mutate(actor, MutationMeta::new("story.delete_act", summary, Capability::SoftDelete).target("story_act", &a.id), |tx| {
        let c = tx.conn();
        let kids = children(c, &StoryContainerRef::act(&a.id), ep.as_deref())?;
        let index = tree::act_ids(c, ep.as_deref())?.iter().position(|x| x == &a.id).unwrap_or(0);
        let mut cascade = false;
        if !kids.is_empty() {
            match a.mode {
                None => {
                    return Err(AppError::conflict(format!(
                        "This Act contains {} item{}. Choose whether to move them to another Act or delete them too.",
                        kids.len(),
                        if kids.len() == 1 { "" } else { "s" }
                    )));
                }
                Some(StoryDeleteMode::MoveContents) => {
                    let target = match &a.move_to_act_id {
                        Some(t) if t == &a.id => return Err(AppError::invalid_input("Choose a different Act to move the contents to.")),
                        Some(t) => StoryContainerRef::act(t),
                        None => StoryContainerRef::unassigned(),
                    };
                    let tep = container_episode(c, &target, ep.as_deref())?;
                    if tep != ep {
                        return Err(AppError::invalid_input("Contents can only be moved to an Act in the same episode."));
                    }
                    place(c, &kids, &target, ep.as_deref(), None)?;
                }
                Some(StoryDeleteMode::DeleteAll) => cascade = true,
            }
        }
        soft_delete(
            tx,
            DeleteSpec {
                object_type: "story_act",
                table: "story_act",
                id: &a.id,
                title: Some(title.clone()),
                parent_type: None,
                parent_id: None,
                position: Some(index as i64),
            },
        )?;
        if cascade {
            let at = deleted_at(c, "story_act", &a.id)?;
            cascade_act(c, &a.id, at)?;
        }
        Ok(())
    })
}

// ================================================================ sequences

fn create_sequence(
    core: &AppCore,
    actor: &Actor,
    a: StoryCreateSequenceArgs,
) -> AppResult<StoryCreated> {
    let title = required_text(&a.title, "Sequence name", TITLE_MAX)?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.create_sequence", format!("Added Sequence “{}”", short(&title)), Capability::Edit),
        |tx| {
            let c = tx.conn();
            let cont = StoryContainerRef::act(&a.act_id);
            let ep = container_episode(c, &cont, None)?;
            let id = new_id();
            let now = now_ms();
            let pos = children(c, &cont, ep.as_deref())?.len() as i64 + 1;
            c.execute(
                "INSERT INTO story_sequence(id, act_id, title, position, episode_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, a.act_id, title, pos, ep, now],
            )?;
            position_new(c, &cont, ep.as_deref(), &StoryItemRef::new(StoryItemKind::Sequence, &id), a.index)?;
            Ok(StoryCreated { id })
        },
    )
}

fn update_sequence(core: &AppCore, actor: &Actor, a: StoryUpdateSequenceArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    let mut summary = "Edited Sequence note".to_string();
    if let Some(t) = &a.title {
        let t = required_text(t, "Sequence name", TITLE_MAX)?;
        summary = format!("Renamed Sequence to “{}”", short(&t));
        fields.push(("title", text(t)));
    }
    if let Some(v) = note_value(&a.note, "Note")? {
        fields.push(("note", v));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.update_sequence", summary, Capability::Edit)
            .target("story_sequence", &a.id)
            .coalesce(format!("story.sequence:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "story_sequence", &a.id, "sequence")?;
            update_fields(
                tx.conn(),
                "story_sequence",
                &a.id,
                &fields,
                &["title", "note"],
                a.expected_rev,
                "sequence",
            )?;
            Ok(())
        },
    )
}

fn delete_sequence(core: &AppCore, actor: &Actor, a: StoryDeleteSequenceArgs) -> AppResult<()> {
    let s = core.project()?;
    let title: String = s.store.read(|c| {
        c.query_row(
            "SELECT title FROM story_sequence WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("sequence"))
    })?;
    let summary = match a.mode {
        Some(StoryDeleteMode::DeleteAll) => {
            format!("Deleted Sequence “{}” and its cards", short(&title))
        }
        _ => format!("Deleted Sequence “{}”", short(&title)),
    };
    s.store.mutate(
        actor,
        MutationMeta::new("story.delete_sequence", summary, Capability::SoftDelete).target("story_sequence", &a.id),
        |tx| {
            let c = tx.conn();
            let me = StoryItemRef::new(StoryItemKind::Sequence, &a.id);
            let loc = tree::locate(c, &me)?;
            let ep = loc.episode_id.clone();
            let index = children(c, &loc.container, ep.as_deref())?.iter().position(|x| x == &me).unwrap_or(0);
            let kids = children(c, &StoryContainerRef::sequence(&a.id), ep.as_deref())?;
            let mut cascade = false;
            if !kids.is_empty() {
                match a.mode {
                    None => {
                        return Err(AppError::conflict(format!(
                            "This Sequence contains {} item{}. Choose whether to move them out or delete them too.",
                            kids.len(),
                            if kids.len() == 1 { "" } else { "s" }
                        )));
                    }
                    Some(StoryDeleteMode::MoveContents) => match &a.move_to {
                        Some(t) => {
                            if t.parent_type == StoryContainerType::Sequence && t.parent_id.as_deref() == Some(a.id.as_str()) {
                                return Err(AppError::invalid_input("Choose a different place to move the contents to."));
                            }
                            let tep = container_episode(c, t, ep.as_deref())?;
                            if tep != ep {
                                return Err(AppError::invalid_input("Contents can only be moved within the same episode."));
                            }
                            place(c, &kids, t, ep.as_deref(), None)?;
                        }
                        // Default: the cards take the sequence's place in its Act.
                        None => place(c, &kids, &loc.container, ep.as_deref(), Some(index))?,
                    },
                    Some(StoryDeleteMode::DeleteAll) => cascade = true,
                }
            }
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "story_sequence",
                    table: "story_sequence",
                    id: &a.id,
                    title: Some(title.clone()),
                    parent_type: Some(loc.container.parent_type.as_str()),
                    parent_id: loc.container.parent_id.clone(),
                    position: Some(index as i64),
                },
            )?;
            if cascade {
                let at = deleted_at(c, "story_sequence", &a.id)?;
                cascade_sequence(c, &a.id, at)?;
            }
            Ok(())
        },
    )
}

// ============================================================ beats & cards

/// Move a freshly inserted item (currently last) to `index` in its container.
fn position_new(
    c: &Connection,
    cont: &StoryContainerRef,
    episode: Option<&str>,
    item: &StoryItemRef,
    index: Option<usize>,
) -> AppResult<()> {
    let mut list = children(c, cont, episode)?;
    list.retain(|x| x != item);
    let idx = index.unwrap_or(list.len()).min(list.len());
    list.insert(idx, item.clone());
    tree::renumber_items(c, &list)
}

fn index_of(
    c: &Connection,
    cont: &StoryContainerRef,
    episode: Option<&str>,
    item: &StoryItemRef,
) -> AppResult<usize> {
    Ok(children(c, cont, episode)?
        .iter()
        .position(|x| x == item)
        .unwrap_or(0))
}

fn resolve_target(
    c: &Connection,
    parent: Option<StoryContainerRef>,
    scope: Option<&str>,
) -> AppResult<(StoryContainerRef, Option<String>)> {
    let cont = parent.unwrap_or_else(StoryContainerRef::parking);
    let ep = container_episode(c, &cont, scope)?;
    Ok((cont, ep))
}

fn insert_beat(
    c: &Connection,
    cont: &StoryContainerRef,
    ep: Option<&str>,
    text_value: &str,
    note: Option<&str>,
    color: Option<&str>,
    index: Option<usize>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let pos = children(c, cont, ep)?.len() as i64 + 1;
    c.execute(
        "INSERT INTO story_beat(id, episode_id, parent_type, parent_id, text, note, color, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![id, ep, cont.parent_type.as_str(), cont.parent_id, text_value, note, color, pos, now],
    )?;
    position_new(
        c,
        cont,
        ep,
        &StoryItemRef::new(StoryItemKind::Beat, &id),
        index,
    )?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn insert_card(
    c: &Connection,
    cont: &StoryContainerRef,
    ep: Option<&str>,
    description: &str,
    heading: Option<&str>,
    notes: Option<&str>,
    color: Option<&str>,
    source_beat: Option<&str>,
    index: Option<usize>,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    let pos = children(c, cont, ep)?.len() as i64 + 1;
    c.execute(
        "INSERT INTO story_scene_card(id, episode_id, parent_type, parent_id, short_description, scene_heading, notes, color,
                                      source_beat_id, position, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![id, ep, cont.parent_type.as_str(), cont.parent_id, description, heading, notes, color, source_beat, pos, now],
    )?;
    position_new(
        c,
        cont,
        ep,
        &StoryItemRef::new(StoryItemKind::Card, &id),
        index,
    )?;
    Ok(id)
}

fn description_text(v: String) -> AppResult<String> {
    body_text(
        v.trim_end().to_string(),
        "Short description",
        DESC_MAX_BYTES,
    )
}

fn create_beat(core: &AppCore, actor: &Actor, a: StoryCreateBeatArgs) -> AppResult<StoryCreated> {
    let text_value = body_text(a.text.trim().to_string(), "Beat text", DESC_MAX_BYTES)?;
    let color = clean_color(a.color)?;
    let s = core.project()?;
    let summary = if text_value.is_empty() {
        "Added Beat".to_string()
    } else {
        format!("Added Beat “{}”", short(&text_value))
    };
    s.store.mutate(
        actor,
        MutationMeta::new("story.create_beat", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let (cont, ep) = resolve_target(c, a.parent.clone(), a.episode_id.as_deref())?;
            if cont.parent_type == StoryContainerType::Unassigned {
                return Err(AppError::invalid_input(
                    "Add the beat to an Act, a Sequence or the Parking Lot.",
                ));
            }
            let id = insert_beat(
                c,
                &cont,
                ep.as_deref(),
                &text_value,
                None,
                color.as_deref(),
                a.index,
            )?;
            Ok(StoryCreated { id })
        },
    )
}

fn update_beat(core: &AppCore, actor: &Actor, a: StoryUpdateBeatArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(t) = &a.text {
        fields.push((
            "text",
            text(body_text(
                t.trim_end().to_string(),
                "Beat text",
                DESC_MAX_BYTES,
            )?),
        ));
    }
    if let Some(v) = note_value(&a.note, "Note")? {
        fields.push(("note", v));
    }
    if let Some(col) = &a.color {
        fields.push(("color", opt_text(clean_color(Some(col.clone()))?)));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.update_beat", "Edited Beat", Capability::Edit)
            .target("story_beat", &a.id)
            .coalesce(format!("story.beat:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "story_beat", &a.id, "beat")?;
            update_fields(
                tx.conn(),
                "story_beat",
                &a.id,
                &fields,
                &["text", "note", "color"],
                a.expected_rev,
                "beat",
            )?;
            Ok(())
        },
    )
}

/// Convert to Scene (FSD §10.5): a NEW Scene Card from the beat text; the beat
/// stays behind in the "converted" state so no creative thought is lost.
fn convert_beat(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<StoryCreated> {
    let s = core.project()?;
    let beat_text: String = s.store.read(|c| {
        c.query_row(
            "SELECT text FROM story_beat WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("beat"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.convert_beat", format!("Converted Beat “{}” to a Scene Card", short(&beat_text)), Capability::Edit)
            .target("story_beat", &a.id),
        |tx| {
            let c = tx.conn();
            let state: String = c.query_row("SELECT state FROM story_beat WHERE id=?1", [&a.id], |r| r.get(0))?;
            if state == "converted" {
                return Err(AppError::conflict("This beat has already been converted to a Scene Card."));
            }
            let me = StoryItemRef::new(StoryItemKind::Beat, &a.id);
            let loc = tree::locate(c, &me)?;
            let idx = index_of(c, &loc.container, loc.episode_id.as_deref(), &me)?;
            let card = insert_card(c, &loc.container, loc.episode_id.as_deref(), &beat_text, None, None, None, Some(&a.id), Some(idx + 1))?;
            c.execute(
                "UPDATE story_beat SET state='converted', converted_scene_card_id=?1, rev=rev+1, updated_at=?2 WHERE id=?3",
                params![card, now_ms(), a.id],
            )?;
            Ok(StoryCreated { id: card })
        },
    )
}

fn create_card(core: &AppCore, actor: &Actor, a: StoryCreateCardArgs) -> AppResult<StoryCreated> {
    let description = description_text(a.short_description)?;
    let heading = optional_text(a.scene_heading, "Scene heading", HEADING_MAX)?;
    let s = core.project()?;
    let summary = if description.trim().is_empty() {
        "Added Scene Card".to_string()
    } else {
        format!("Added Scene Card “{}”", short(&description))
    };
    s.store.mutate(
        actor,
        MutationMeta::new("story.create_card", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let (cont, ep) = match a.parent.clone() {
                Some(p) => {
                    if p.parent_type == StoryContainerType::Unassigned {
                        return Err(AppError::invalid_input(
                            "Add the card to an Act, a Sequence or the Parking Lot.",
                        ));
                    }
                    let ep = container_episode(c, &p, a.episode_id.as_deref())?;
                    (p, ep)
                }
                None => {
                    tree::check_episode(c, a.episode_id.as_deref())?;
                    let ep = a.episode_id.clone();
                    let act = match tree::act_ids(c, ep.as_deref())?.pop() {
                        Some(act) => act,
                        None => insert_act(c, ep.as_deref(), "Act 1", None)?,
                    };
                    (StoryContainerRef::act(&act), ep)
                }
            };
            let id = insert_card(
                c,
                &cont,
                ep.as_deref(),
                &description,
                heading.as_deref(),
                None,
                None,
                None,
                a.index,
            )?;
            Ok(StoryCreated { id })
        },
    )
}

fn update_card(core: &AppCore, actor: &Actor, a: StoryUpdateCardArgs) -> AppResult<()> {
    let mut fields: Vec<(&str, SqlValue)> = Vec::new();
    if let Some(d) = a.short_description.clone() {
        fields.push(("short_description", text(description_text(d)?)));
    }
    if let Some(h) = a.scene_heading.clone() {
        fields.push((
            "scene_heading",
            opt_text(optional_text(Some(h), "Scene heading", HEADING_MAX)?),
        ));
    }
    if let Some(v) = note_value(&a.notes, "Notes")? {
        fields.push(("notes", v));
    }
    if let Some(col) = &a.color {
        fields.push(("color", opt_text(clean_color(Some(col.clone()))?)));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.update_card", "Edited Scene Card", Capability::Edit)
            .target("story_scene_card", &a.id)
            .coalesce(format!("story.card:{}", a.id)),
        |tx| {
            ensure_live(tx.conn(), "story_scene_card", &a.id, "scene card")?;
            update_fields(
                tx.conn(),
                "story_scene_card",
                &a.id,
                &fields,
                &["short_description", "scene_heading", "notes", "color"],
                a.expected_rev,
                "scene card",
            )?;
            Ok(())
        },
    )
}

/// Scene → Beat (FSD-STORY-030): a new Beat from the card's description is
/// placed right after the card; the card itself is retained unchanged.
fn card_to_beat(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<StoryCreated> {
    let s = core.project()?;
    let desc: String = s.store.read(|c| {
        c.query_row(
            "SELECT short_description FROM story_scene_card WHERE id=?1 AND deleted_at IS NULL",
            [&a.id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("scene card"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.card_to_beat",
            format!("Copied Scene Card “{}” as a Beat", short(&desc)),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let me = StoryItemRef::new(StoryItemKind::Card, &a.id);
            let loc = tree::locate(c, &me)?;
            let mut cont = loc.container.clone();
            if cont.parent_type == StoryContainerType::Unassigned {
                cont = StoryContainerRef::parking();
            }
            let idx = if cont == loc.container {
                Some(index_of(c, &cont, loc.episode_id.as_deref(), &me)? + 1)
            } else {
                None
            };
            let id = insert_beat(c, &cont, loc.episode_id.as_deref(), &desc, None, None, idx)?;
            Ok(StoryCreated { id })
        },
    )
}

// ============================================================ multi-select

fn selection_scope(c: &Connection, items: &[StoryItemRef]) -> AppResult<Option<String>> {
    let first = items
        .first()
        .ok_or_else(|| AppError::invalid_input("Select at least one item."))?;
    Ok(tree::locate(c, first)?.episode_id)
}

fn describe_items(c: &Connection, items: &[StoryItemRef]) -> String {
    if items.len() == 1 {
        format!(
            "{} “{}”",
            items[0].kind.label(),
            tree::item_title(c, &items[0])
        )
    } else if items.iter().all(|i| i.kind == StoryItemKind::Card) {
        format!("{} Scene Cards", items.len())
    } else {
        format!("{} items", items.len())
    }
}

fn check_count(items: &[StoryItemRef]) -> AppResult<()> {
    if items.is_empty() {
        return Err(AppError::invalid_input("Select at least one item."));
    }
    if items.len() > 2000 {
        return Err(AppError::invalid_input("Too many items selected."));
    }
    Ok(())
}

/// Drag & drop / Move… (FSD §11.6, §9.5, §89.6): selected items move together
/// preserving their board order; identities never change.
fn move_items(core: &AppCore, actor: &Actor, a: StoryMoveArgs) -> AppResult<()> {
    check_count(&a.items)?;
    let s = core.project()?;
    let summary = s.store.read(|c| {
        Ok(format!(
            "Moved {} to {}",
            describe_items(c, &a.items),
            tree::container_label(c, &a.target)
        ))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.move_items", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let scope = match a.target.parent_type {
                StoryContainerType::Parking | StoryContainerType::Unassigned => match &a.episode_id
                {
                    Some(e) => Some(e.clone()),
                    None => selection_scope(c, &a.items)?,
                },
                _ => None,
            };
            let ep = container_episode(c, &a.target, scope.as_deref())?;
            for it in &a.items {
                if tree::locate(c, it)?.episode_id != ep {
                    return Err(AppError::invalid_input(
                        "Story items can only be moved within the same episode.",
                    ));
                }
            }
            let items = tree::sort_by_board_order(c, ep.as_deref(), &a.items)?;
            let index = match &a.before {
                Some(b) => {
                    let list: Vec<StoryItemRef> = children(c, &a.target, ep.as_deref())?
                        .into_iter()
                        .filter(|x| !items.contains(x))
                        .collect();
                    list.iter().position(|x| x == b)
                }
                None => None,
            };
            place(c, &items, &a.target, ep.as_deref(), index)
        },
    )
}

fn copy_attachments(c: &Connection, owner_type: &str, from: &str, to: &str) -> AppResult<()> {
    let rows: Vec<(String, i64)> = {
        let mut stmt =
            c.prepare("SELECT asset_id, position FROM story_attachment WHERE owner_type=?1 AND owner_id=?2 ORDER BY position")?;
        stmt.query_map(params![owner_type, from], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    let now = now_ms();
    for (asset, pos) in rows {
        c.execute(
            "INSERT INTO story_attachment(id, owner_type, owner_id, asset_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![new_id(), owner_type, to, asset, pos, now],
        )?;
    }
    Ok(())
}

/// Duplicate (FSD §11.7): new independent identities placed right after the
/// originals; never inherits a screenplay link.
pub(crate) fn duplicate_one(c: &Connection, item: &StoryItemRef) -> AppResult<String> {
    let loc = tree::locate(c, item)?;
    let ep = loc.episode_id.as_deref();
    let idx = index_of(c, &loc.container, ep, item)?;
    match item.kind {
        StoryItemKind::Card => {
            let (d, h, n, col): (String, Option<String>, Option<String>, Option<String>) = c.query_row(
                "SELECT short_description, scene_heading, notes, color FROM story_scene_card WHERE id=?1",
                [&item.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
            let id = insert_card(
                c,
                &loc.container,
                ep,
                &d,
                h.as_deref(),
                n.as_deref(),
                col.as_deref(),
                None,
                Some(idx + 1),
            )?;
            copy_attachments(c, "scene_card", &item.id, &id)?;
            let chars: Vec<String> = {
                let mut stmt = c.prepare(
                    "SELECT character_id FROM story_character_card_link WHERE scene_card_id=?1",
                )?;
                stmt.query_map([&item.id], |r| r.get(0))?
                    .collect::<Result<_, _>>()?
            };
            let now = now_ms();
            for ch in chars {
                c.execute(
                    "INSERT INTO story_character_card_link(id, character_id, scene_card_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![new_id(), ch, id, now],
                )?;
            }
            Ok(id)
        }
        StoryItemKind::Beat => {
            let (t, n, col): (String, Option<String>, Option<String>) = c.query_row(
                "SELECT text, note, color FROM story_beat WHERE id=?1",
                [&item.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let id = insert_beat(
                c,
                &loc.container,
                ep,
                &t,
                n.as_deref(),
                col.as_deref(),
                Some(idx + 1),
            )?;
            copy_attachments(c, "beat", &item.id, &id)?;
            Ok(id)
        }
        StoryItemKind::Sequence => Err(AppError::invalid_input(
            "Select Scene Cards or Beats to duplicate.",
        )),
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCreatedMany {
    pub ids: Vec<String>,
}

fn duplicate_items(
    core: &AppCore,
    actor: &Actor,
    a: StoryItemsArgs,
) -> AppResult<StoryCreatedMany> {
    check_count(&a.items)?;
    let s = core.project()?;
    let summary = s
        .store
        .read(|c| Ok(format!("Duplicated {}", describe_items(c, &a.items))))?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.duplicate_items", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let ep = selection_scope(c, &a.items)?;
            let items = tree::sort_by_board_order(c, ep.as_deref(), &a.items)?;
            let mut ids = Vec::new();
            for it in &items {
                ids.push(duplicate_one(c, it)?);
            }
            Ok(StoryCreatedMany { ids })
        },
    )
}

fn park_items(core: &AppCore, actor: &Actor, a: StoryItemsArgs) -> AppResult<()> {
    check_count(&a.items)?;
    let s = core.project()?;
    let summary = s.store.read(|c| {
        Ok(format!(
            "Moved {} to the Parking Lot",
            describe_items(c, &a.items)
        ))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.park_items", summary, Capability::Edit),
        |tx| {
            let c = tx.conn();
            let ep = selection_scope(c, &a.items)?;
            let items: Vec<StoryItemRef> = tree::sort_by_board_order(c, ep.as_deref(), &a.items)?
                .into_iter()
                .filter(|it| {
                    tree::locate(c, it)
                        .map(|l| l.container.parent_type != StoryContainerType::Parking)
                        .unwrap_or(false)
                })
                .collect();
            if items.is_empty() {
                return Ok(());
            }
            place(
                c,
                &items,
                &StoryContainerRef::parking(),
                ep.as_deref(),
                None,
            )
        },
    )
}

/// "Restore to Story" (FSD §12.2): back to where the item was parked from when
/// that container still exists; otherwise the end of the last Act (or the
/// Unassigned area when the board has no Acts). Returns where each item went.
fn restore_from_parking(
    core: &AppCore,
    actor: &Actor,
    a: StoryItemsArgs,
) -> AppResult<Vec<StoryPlacement>> {
    check_count(&a.items)?;
    let s = core.project()?;
    let summary = s.store.read(|c| {
        Ok(format!(
            "Restored {} to the story",
            describe_items(c, &a.items)
        ))
    })?;
    s.store.mutate(actor, MutationMeta::new("story.restore_from_parking", summary, Capability::Edit), |tx| {
        let c = tx.conn();
        let ep = selection_scope(c, &a.items)?;
        let items = tree::sort_by_board_order(c, ep.as_deref(), &a.items)?;
        let mut out = Vec::new();
        for it in items {
            let loc = tree::locate(c, &it)?;
            if loc.container.parent_type != StoryContainerType::Parking {
                continue;
            }
            let (ft, fid, fidx): (Option<String>, Option<String>, Option<i64>) = c.query_row(
                &format!("SELECT parked_from_type, parked_from_id, parked_from_index FROM {} WHERE id=?1", it.kind.table()),
                [&it.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            let origin = match (ft.as_deref().and_then(StoryContainerType::parse), fid) {
                (Some(t), Some(id)) if tree::container_live(c, t.as_str(), Some(&id))? => {
                    let cont = StoryContainerRef { parent_type: t, parent_id: Some(id) };
                    (container_episode(c, &cont, ep.as_deref())? == ep).then_some(cont)
                }
                _ => None,
            };
            let (target, index) = match origin {
                Some(cont) => (cont, fidx.map(|i| i.max(0) as usize)),
                None => match tree::act_ids(c, ep.as_deref())?.pop() {
                    Some(act) => (StoryContainerRef::act(&act), None),
                    None => (StoryContainerRef::unassigned(), None),
                },
            };
            place(c, std::slice::from_ref(&it), &target, ep.as_deref(), index)?;
            out.push(StoryPlacement { label: tree::container_label(c, &target), item: it, container: target });
        }
        Ok(out)
    })
}

/// Delete (FSD §11.8): recoverable; each item gets its own Recently Deleted entry.
fn delete_items(core: &AppCore, actor: &Actor, a: StoryItemsArgs) -> AppResult<()> {
    check_count(&a.items)?;
    if a.items.iter().any(|i| i.kind == StoryItemKind::Sequence) {
        return Err(AppError::invalid_input(
            "Delete sequences one at a time so you can choose what happens to their cards.",
        ));
    }
    let s = core.project()?;
    let summary = s
        .store
        .read(|c| Ok(format!("Deleted {}", describe_items(c, &a.items))))?;
    s.store.mutate(
        actor,
        MutationMeta::new("story.delete_items", summary, Capability::SoftDelete),
        |tx| {
            let c = tx.conn();
            let ep = selection_scope(c, &a.items)?;
            let items = tree::sort_by_board_order(c, ep.as_deref(), &a.items)?;
            // Record positions before anything moves.
            let mut specs = Vec::new();
            for it in &items {
                let loc = tree::locate(c, it)?;
                let idx = index_of(c, &loc.container, ep.as_deref(), it)?;
                specs.push((it.clone(), loc.container, idx, tree::item_title(c, it)));
            }
            for (it, cont, idx, title) in specs {
                soft_delete(
                    tx,
                    DeleteSpec {
                        object_type: it.kind.table(),
                        table: it.kind.table(),
                        id: &it.id,
                        title: Some(title),
                        parent_type: Some(cont.parent_type.as_str()),
                        parent_id: cont.parent_id.clone(),
                        position: Some(idx as i64),
                    },
                )?;
            }
            Ok(())
        },
    )
}

// ============================================================ attachments

fn add_attachment(
    core: &AppCore,
    actor: &Actor,
    a: StoryAttachArgs,
) -> AppResult<StoryAttachmentDto> {
    let (table, what) = match a.owner_type.as_str() {
        "scene_card" => ("story_scene_card", "scene card"),
        "beat" => ("story_beat", "beat"),
        "sequence" => ("story_sequence", "sequence"),
        _ => {
            return Err(AppError::invalid_input(
                "Attachments can be added to Scene Cards, Beats and Sequences.",
            ));
        }
    };
    let s = core.project()?;
    let name = PathBuf::from(&a.path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let id = s.store.mutate(
        actor,
        MutationMeta::new("story.add_attachment", format!("Attached “{name}”"), Capability::Edit).target(table, &a.owner_id),
        |tx| {
            let c = tx.conn();
            ensure_live(c, table, &a.owner_id, what)?;
            let asset = ingest_file(tx, &PathBuf::from(&a.path))?;
            let pos: i64 = c.query_row(
                "SELECT COALESCE(MAX(position),0)+1 FROM story_attachment WHERE owner_type=?1 AND owner_id=?2",
                params![a.owner_type, a.owner_id],
                |r| r.get(0),
            )?;
            let id = new_id();
            let now = now_ms();
            c.execute(
                "INSERT INTO story_attachment(id, owner_type, owner_id, asset_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![id, a.owner_type, a.owner_id, asset.id, pos, now],
            )?;
            tx.reindex(table, &a.owner_id);
            Ok(id)
        },
    )?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| {
        let asset_id: String = c.query_row(
            "SELECT asset_id FROM story_attachment WHERE id=?1",
            [&id],
            |r| r.get(0),
        )?;
        Ok(StoryAttachmentDto {
            id: id.clone(),
            asset: load_asset(c, &root, &asset_id)?,
        })
    })
}

/// Removing an attachment keeps the underlying file so Undo can bring it back.
fn remove_attachment(core: &AppCore, actor: &Actor, a: StoryIdArgs) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "story.remove_attachment",
            "Removed attachment",
            Capability::Edit,
        ),
        |tx| {
            let n = tx
                .conn()
                .execute("DELETE FROM story_attachment WHERE id=?1", [&a.id])?;
            if n == 0 {
                return Err(AppError::not_found("attachment"));
            }
            Ok(())
        },
    )
}
