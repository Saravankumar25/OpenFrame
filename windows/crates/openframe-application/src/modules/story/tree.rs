//! Story Board structure: containers, shared sibling ordering and moves.
//!
//! Children of one container share a single position space across tables:
//! an Act's sequences, beats and cards interleave; a Sequence's beats and
//! cards interleave; the Parking Lot and the Unassigned area hold beats and
//! cards (Unassigned may also hold sequences whose act is gone). Every move
//! goes through [`place`], which keeps identities and renumbers only rows
//! whose position actually changes (minimal undo entries).

use std::collections::{BTreeSet, HashMap};

use openframe_domain::{AppError, AppResult, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryItemKind {
    Sequence,
    Beat,
    Card,
}

impl StoryItemKind {
    pub fn table(self) -> &'static str {
        match self {
            StoryItemKind::Sequence => "story_sequence",
            StoryItemKind::Beat => "story_beat",
            StoryItemKind::Card => "story_scene_card",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            StoryItemKind::Sequence => "Sequence",
            StoryItemKind::Beat => "Beat",
            StoryItemKind::Card => "Scene Card",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "sequence" => Some(StoryItemKind::Sequence),
            "beat" => Some(StoryItemKind::Beat),
            "card" => Some(StoryItemKind::Card),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryItemRef {
    pub kind: StoryItemKind,
    pub id: String,
}

impl StoryItemRef {
    pub fn new(kind: StoryItemKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryContainerType {
    Act,
    Sequence,
    Parking,
    Unassigned,
}

impl StoryContainerType {
    pub fn as_str(self) -> &'static str {
        match self {
            StoryContainerType::Act => "act",
            StoryContainerType::Sequence => "sequence",
            StoryContainerType::Parking => "parking",
            StoryContainerType::Unassigned => "unassigned",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "act" => Some(StoryContainerType::Act),
            "sequence" => Some(StoryContainerType::Sequence),
            "parking" => Some(StoryContainerType::Parking),
            "unassigned" => Some(StoryContainerType::Unassigned),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryContainerRef {
    pub parent_type: StoryContainerType,
    #[serde(default)]
    pub parent_id: Option<String>,
}

impl StoryContainerRef {
    pub fn act(id: &str) -> Self {
        Self {
            parent_type: StoryContainerType::Act,
            parent_id: Some(id.to_string()),
        }
    }
    pub fn sequence(id: &str) -> Self {
        Self {
            parent_type: StoryContainerType::Sequence,
            parent_id: Some(id.to_string()),
        }
    }
    pub fn parking() -> Self {
        Self {
            parent_type: StoryContainerType::Parking,
            parent_id: None,
        }
    }
    pub fn unassigned() -> Self {
        Self {
            parent_type: StoryContainerType::Unassigned,
            parent_id: None,
        }
    }
    pub fn is_active_story(&self) -> bool {
        matches!(
            self.parent_type,
            StoryContainerType::Act | StoryContainerType::Sequence
        )
    }
}

/// Ordered live children of a container.
pub fn children(
    c: &Connection,
    cont: &StoryContainerRef,
    episode: Option<&str>,
) -> AppResult<Vec<StoryItemRef>> {
    let pid = cont.parent_id.as_deref();
    let (sql, p1): (&str, Option<&str>) = match cont.parent_type {
        StoryContainerType::Act => (
            "SELECT 'sequence', id, position, 0 FROM story_sequence WHERE act_id = ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'beat', id, position, 1 FROM story_beat WHERE parent_type='act' AND parent_id = ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'card', id, position, 2 FROM story_scene_card WHERE parent_type='act' AND parent_id = ?1 AND deleted_at IS NULL
             ORDER BY 3, 4, 2",
            pid,
        ),
        StoryContainerType::Sequence => (
            "SELECT 'beat', id, position, 1 FROM story_beat WHERE parent_type='sequence' AND parent_id = ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'card', id, position, 2 FROM story_scene_card WHERE parent_type='sequence' AND parent_id = ?1 AND deleted_at IS NULL
             ORDER BY 3, 4, 2",
            pid,
        ),
        StoryContainerType::Parking => (
            "SELECT 'beat', id, position, 1 FROM story_beat WHERE parent_type='parking' AND episode_id IS ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'card', id, position, 2 FROM story_scene_card WHERE parent_type='parking' AND episode_id IS ?1 AND deleted_at IS NULL
             ORDER BY 3, 4, 2",
            episode,
        ),
        StoryContainerType::Unassigned => (
            "SELECT 'sequence', id, position, 0 FROM story_sequence WHERE act_id IS NULL AND episode_id IS ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'beat', id, position, 1 FROM story_beat WHERE parent_type='unassigned' AND episode_id IS ?1 AND deleted_at IS NULL
             UNION ALL SELECT 'card', id, position, 2 FROM story_scene_card WHERE parent_type='unassigned' AND episode_id IS ?1 AND deleted_at IS NULL
             ORDER BY 3, 4, 2",
            episode,
        ),
    };
    let mut stmt = c.prepare_cached(sql)?;
    let rows = stmt
        .query_map(params![p1], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(k, id)| StoryItemKind::from_str(&k).map(|kind| StoryItemRef { kind, id }))
        .collect())
}

/// Ordered live acts of a scope (episode or whole project).
pub fn act_ids(c: &Connection, episode: Option<&str>) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare_cached(
        "SELECT id FROM story_act WHERE episode_id IS ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let ids = stmt
        .query_map(params![episode], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(ids)
}

/// Rewrite positions 1..n (only rows whose position changes are touched).
pub fn renumber_items(c: &Connection, items: &[StoryItemRef]) -> AppResult<()> {
    let now = now_ms();
    for (i, it) in items.iter().enumerate() {
        c.execute(
            &format!(
                "UPDATE {} SET position = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3 AND position IS NOT ?1",
                it.kind.table()
            ),
            params![(i + 1) as i64, now, it.id],
        )?;
    }
    Ok(())
}

/// Where a live item currently sits.
#[derive(Debug, Clone)]
pub struct Located {
    pub container: StoryContainerRef,
    pub episode_id: Option<String>,
}

pub fn locate(c: &Connection, item: &StoryItemRef) -> AppResult<Located> {
    match item.kind {
        StoryItemKind::Sequence => {
            let row: Option<(Option<String>, Option<String>)> = c
                .query_row(
                    "SELECT act_id, episode_id FROM story_sequence WHERE id=?1 AND deleted_at IS NULL",
                    [&item.id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let (act, ep) = row.ok_or_else(|| AppError::not_found("sequence"))?;
            Ok(Located {
                container: match act {
                    Some(a) => StoryContainerRef::act(&a),
                    None => StoryContainerRef::unassigned(),
                },
                episode_id: ep,
            })
        }
        StoryItemKind::Beat | StoryItemKind::Card => {
            let row: Option<(String, Option<String>, Option<String>)> = c
                .query_row(
                    &format!("SELECT parent_type, parent_id, episode_id FROM {} WHERE id=?1 AND deleted_at IS NULL", item.kind.table()),
                    [&item.id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let (pt, pid, ep) = row.ok_or_else(|| {
                AppError::not_found(if item.kind == StoryItemKind::Beat {
                    "beat"
                } else {
                    "scene card"
                })
            })?;
            let parent_type = StoryContainerType::parse(&pt)
                .ok_or_else(|| AppError::internal("unknown parent type"))?;
            Ok(Located {
                container: StoryContainerRef {
                    parent_type,
                    parent_id: pid,
                },
                episode_id: ep,
            })
        }
    }
}

/// Verify an episode scope exists (None = whole project / non-episodic).
pub fn check_episode(c: &Connection, episode: Option<&str>) -> AppResult<()> {
    if let Some(e) = episode {
        let ok: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM episode WHERE id=?1 AND deleted_at IS NULL)",
            [e],
            |r| r.get(0),
        )?;
        if !ok {
            return Err(AppError::not_found("episode"));
        }
    }
    Ok(())
}

/// Validate a destination container and return its episode scope. For the
/// Parking Lot / Unassigned area the scope comes from `scope`.
pub fn container_episode(
    c: &Connection,
    cont: &StoryContainerRef,
    scope: Option<&str>,
) -> AppResult<Option<String>> {
    match cont.parent_type {
        StoryContainerType::Act => {
            let id = cont
                .parent_id
                .as_deref()
                .ok_or_else(|| AppError::not_found("act"))?;
            c.query_row(
                "SELECT episode_id FROM story_act WHERE id=?1 AND deleted_at IS NULL",
                [id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("act"))
        }
        StoryContainerType::Sequence => {
            let id = cont
                .parent_id
                .as_deref()
                .ok_or_else(|| AppError::not_found("sequence"))?;
            c.query_row(
                "SELECT episode_id FROM story_sequence WHERE id=?1 AND deleted_at IS NULL",
                [id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::not_found("sequence"))
        }
        StoryContainerType::Parking | StoryContainerType::Unassigned => {
            if cont.parent_id.is_some() {
                return Err(AppError::invalid_input(
                    "The Parking Lot has no parent container.",
                ));
            }
            check_episode(c, scope)?;
            Ok(scope.map(|s| s.to_string()))
        }
    }
}

/// Is a container still live (used by restore to decide between the old
/// place and the Unassigned area)?
pub fn container_live(c: &Connection, cont_type: &str, cont_id: Option<&str>) -> AppResult<bool> {
    Ok(match (cont_type, cont_id) {
        ("act", Some(id)) => c.query_row(
            "SELECT EXISTS(SELECT 1 FROM story_act WHERE id=?1 AND deleted_at IS NULL)",
            [id],
            |r| r.get(0),
        )?,
        ("sequence", Some(id)) => c.query_row(
            "SELECT EXISTS(SELECT 1 FROM story_sequence WHERE id=?1 AND deleted_at IS NULL)",
            [id],
            |r| r.get(0),
        )?,
        ("parking", None) | ("unassigned", None) => true,
        _ => false,
    })
}

fn set_parent(
    c: &Connection,
    item: &StoryItemRef,
    cont: &StoryContainerRef,
    episode: Option<&str>,
) -> AppResult<()> {
    let now = now_ms();
    match item.kind {
        StoryItemKind::Sequence => {
            let act = match cont.parent_type {
                StoryContainerType::Act => cont.parent_id.clone(),
                StoryContainerType::Unassigned => None,
                _ => {
                    return Err(AppError::invalid_input(
                        "A sequence can only be placed inside an Act.",
                    ));
                }
            };
            c.execute(
                "UPDATE story_sequence SET act_id=?1, episode_id=?2, rev=rev+1, updated_at=?3
                 WHERE id=?4 AND (act_id IS NOT ?1 OR episode_id IS NOT ?2)",
                params![act, episode, now, item.id],
            )?;
        }
        StoryItemKind::Beat | StoryItemKind::Card => {
            c.execute(
                &format!(
                    "UPDATE {} SET parent_type=?1, parent_id=?2, episode_id=?3, rev=rev+1, updated_at=?4
                     WHERE id=?5 AND (parent_type IS NOT ?1 OR parent_id IS NOT ?2 OR episode_id IS NOT ?3)",
                    item.kind.table()
                ),
                params![cont.parent_type.as_str(), cont.parent_id, episode, now, item.id],
            )?;
        }
    }
    Ok(())
}

/// Record/clear where a beat or card came from when it enters/leaves the Parking Lot.
fn update_parking_memory(
    c: &Connection,
    item: &StoryItemRef,
    from: &StoryContainerRef,
    from_index: usize,
    to: &StoryContainerRef,
) -> AppResult<()> {
    if item.kind == StoryItemKind::Sequence {
        return Ok(());
    }
    let t = item.kind.table();
    let now = now_ms();
    let to_parking = to.parent_type == StoryContainerType::Parking;
    let from_parking = from.parent_type == StoryContainerType::Parking;
    if to_parking && !from_parking {
        let (ft, fid, fidx) = if from.is_active_story() {
            (
                Some(from.parent_type.as_str()),
                from.parent_id.clone(),
                Some(from_index as i64),
            )
        } else {
            (None, None, None)
        };
        c.execute(
            &format!(
                "UPDATE {t} SET parked_from_type=?1, parked_from_id=?2, parked_from_index=?3, parked_at=?4,
                        rev=rev+1, updated_at=?4 WHERE id=?5"
            ),
            params![ft, fid, fidx, now, item.id],
        )?;
    } else if !to_parking {
        c.execute(
            &format!(
                "UPDATE {t} SET parked_from_type=NULL, parked_from_id=NULL, parked_from_index=NULL, parked_at=NULL,
                        rev=rev+1, updated_at=?1 WHERE id=?2 AND (parked_from_type IS NOT NULL OR parked_at IS NOT NULL)"
            ),
            params![now, item.id],
        )?;
    }
    Ok(())
}

/// Check that `kind` may live in `cont`.
pub fn check_kind_fits(kind: StoryItemKind, cont: &StoryContainerRef) -> AppResult<()> {
    match (kind, cont.parent_type) {
        (StoryItemKind::Sequence, StoryContainerType::Act) => Ok(()),
        (StoryItemKind::Sequence, StoryContainerType::Sequence) => Err(AppError::invalid_input(
            "A sequence can't be placed inside another sequence. Drop it into an Act.",
        )),
        (StoryItemKind::Sequence, StoryContainerType::Parking) => Err(AppError::invalid_input(
            "Sequences can't be parked. Park the cards inside it, or move the sequence to another Act.",
        )),
        // Unassigned holds sequences whose Act was deleted (restore / move-contents).
        _ => Ok(()),
    }
}

/// Move `items` (already in the desired relative order) into `target` at
/// `index` (clamped; None = end). Items keep their identities.
pub fn place(
    c: &Connection,
    items: &[StoryItemRef],
    target: &StoryContainerRef,
    target_episode: Option<&str>,
    index: Option<usize>,
) -> AppResult<()> {
    let moving: BTreeSet<&StoryItemRef> = items.iter().collect();
    let mut sources: Vec<(StoryContainerRef, Option<String>)> = Vec::new();
    let mut from: HashMap<&StoryItemRef, (StoryContainerRef, usize)> = HashMap::new();
    for it in items {
        check_kind_fits(it.kind, target)?;
        let loc = locate(c, it)?;
        if loc.episode_id.as_deref() != target_episode {
            return Err(AppError::invalid_input(
                "Story items can only be moved within the same episode.",
            ));
        }
        if it.kind == StoryItemKind::Sequence
            && target.parent_type == StoryContainerType::Sequence
            && target.parent_id.as_deref() == Some(it.id.as_str())
        {
            return Err(AppError::invalid_input(
                "A sequence can't be moved into itself.",
            ));
        }
        let sibs = children(c, &loc.container, loc.episode_id.as_deref())?;
        let idx = sibs.iter().position(|s| s == it).unwrap_or(0);
        if !sources.iter().any(|(s, _)| s == &loc.container) {
            sources.push((loc.container.clone(), loc.episode_id.clone()));
        }
        from.insert(it, (loc.container, idx));
    }
    let mut list: Vec<StoryItemRef> = children(c, target, target_episode)?
        .into_iter()
        .filter(|s| !moving.contains(s))
        .collect();
    let idx = index.unwrap_or(list.len()).min(list.len());
    for (k, it) in items.iter().enumerate() {
        list.insert(idx + k, it.clone());
    }
    for it in items {
        let (src, src_idx) = from
            .get(it)
            .cloned()
            .ok_or_else(|| AppError::internal("lost item"))?;
        set_parent(c, it, target, target_episode)?;
        update_parking_memory(c, it, &src, src_idx, target)?;
    }
    renumber_items(c, &list)?;
    for (src, ep) in sources {
        if &src != target {
            let rest = children(c, &src, ep.as_deref())?;
            renumber_items(c, &rest)?;
        }
    }
    Ok(())
}

/// Flattened board order for a scope: acts (and their descendants) in order,
/// then the Parking Lot, then the Unassigned area.
pub fn board_order(c: &Connection, episode: Option<&str>) -> AppResult<Vec<StoryItemRef>> {
    let mut out = Vec::new();
    fn walk(
        c: &Connection,
        cont: &StoryContainerRef,
        ep: Option<&str>,
        out: &mut Vec<StoryItemRef>,
    ) -> AppResult<()> {
        for it in children(c, cont, ep)? {
            let is_seq = it.kind == StoryItemKind::Sequence;
            let id = it.id.clone();
            out.push(it);
            if is_seq {
                walk(c, &StoryContainerRef::sequence(&id), ep, out)?;
            }
        }
        Ok(())
    }
    for act in act_ids(c, episode)? {
        walk(c, &StoryContainerRef::act(&act), episode, &mut out)?;
    }
    walk(c, &StoryContainerRef::parking(), episode, &mut out)?;
    walk(c, &StoryContainerRef::unassigned(), episode, &mut out)?;
    Ok(out)
}

/// Sort a selection into board order, dropping duplicates. Unknown items are errors.
pub fn sort_by_board_order(
    c: &Connection,
    episode: Option<&str>,
    items: &[StoryItemRef],
) -> AppResult<Vec<StoryItemRef>> {
    let order = board_order(c, episode)?;
    let rank: HashMap<&StoryItemRef, usize> =
        order.iter().enumerate().map(|(i, r)| (r, i)).collect();
    let mut uniq: Vec<StoryItemRef> = Vec::new();
    for it in items {
        if !uniq.contains(it) {
            uniq.push(it.clone());
        }
    }
    for it in &uniq {
        if !rank.contains_key(it) {
            return Err(AppError::not_found(match it.kind {
                StoryItemKind::Sequence => "sequence",
                StoryItemKind::Beat => "beat",
                StoryItemKind::Card => "scene card",
            }));
        }
    }
    uniq.sort_by_key(|r| rank[r]);
    Ok(uniq)
}

/// Short human title for messages (truncated).
pub fn item_title(c: &Connection, item: &StoryItemRef) -> String {
    let sql = match item.kind {
        StoryItemKind::Sequence => "SELECT title FROM story_sequence WHERE id=?1",
        StoryItemKind::Beat => "SELECT text FROM story_beat WHERE id=?1",
        StoryItemKind::Card => "SELECT short_description FROM story_scene_card WHERE id=?1",
    };
    let t: String = c
        .query_row(sql, [&item.id], |r| r.get(0))
        .unwrap_or_default();
    short(&t)
}

pub fn short(t: &str) -> String {
    let t = t.trim();
    let first = t.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        return "Untitled".to_string();
    }
    if first.chars().count() > 60 {
        let s: String = first.chars().take(57).collect();
        format!("{s}…")
    } else {
        first.to_string()
    }
}

/// Human label of a container for messages ("Sequence “Chase”", "the Parking Lot").
pub fn container_label(c: &Connection, cont: &StoryContainerRef) -> String {
    match cont.parent_type {
        StoryContainerType::Act => {
            let t: String = c
                .query_row(
                    "SELECT title FROM story_act WHERE id=?1",
                    [cont.parent_id.as_deref().unwrap_or("")],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            format!("Act “{}”", short(&t))
        }
        StoryContainerType::Sequence => {
            let t: String = c
                .query_row(
                    "SELECT title FROM story_sequence WHERE id=?1",
                    [cont.parent_id.as_deref().unwrap_or("")],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            format!("Sequence “{}”", short(&t))
        }
        StoryContainerType::Parking => "the Parking Lot".to_string(),
        StoryContainerType::Unassigned => "Unassigned".to_string(),
    }
}
