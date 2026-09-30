//! Explicit copies out of a vault (never live sync):
//! * Global ↔ Project copies (FSD §5.8, §88.9, FSD-IDEA-003/014) — new identities,
//!   independent files.
//! * "Send to Story" (FSD §5.10, FSD-IDEA-004/015..017) — a new Story object;
//!   the Vault original stays exactly as it was.

use openframe_domain::enums::VaultItemType;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{next_position, opt_text, text};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::items::{NewItem, VaultFailedFile, check_collection, check_folder, insert_item};
use super::model::{VaultItemDto, load_item, type_label};
use super::{mutate_vault, read_vault, store_label, vault_actor};
use crate::core::AppCore;
use crate::store::MutationMeta;
use crate::util::{StoreSel, copy_asset_between, with_store};

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultCopyArgs {
    pub from: StoreSel,
    pub to: StoreSel,
    pub ids: Vec<String>,
    /// Optional destination collection (mock 041 "Collection (optional)").
    #[serde(default)]
    pub collection_id: Option<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultCopyResult {
    /// Ids of the new, independent items in the destination vault.
    pub copied: Vec<String>,
    pub failed: Vec<VaultFailedFile>,
}

/// What a Vault item becomes in Story (mock 039). "Story note" needs a Story-owned
/// note table and is not offered until that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum VaultStoryTarget {
    Beat,
    SceneCard,
    Sequence,
    Character,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultSendToStoryArgs {
    /// Which vault holds the item (Global items can go straight into the open project's Story).
    #[serde(default)]
    pub store: StoreSel,
    pub id: String,
    pub target: VaultStoryTarget,
    /// "act" | "sequence" | "parking" | "unassigned" (default for beats/cards: "unassigned").
    #[serde(default)]
    pub parent_type: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultSendToStoryResult {
    pub target: VaultStoryTarget,
    /// Id of the new Story object (story_beat / story_scene_card / story_sequence / story_character).
    pub story_id: String,
    pub table: String,
    pub parent_type: Option<String>,
    pub parent_id: Option<String>,
    /// Human confirmation, e.g. "Created a Scene Card copy in the Parking Lot."
    pub message: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultStoryTargetsArgs {}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultStorySequence {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultStoryAct {
    pub id: String,
    pub title: String,
    pub episode_id: Option<String>,
    pub sequences: Vec<VaultStorySequence>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VaultStoryTargets {
    pub acts: Vec<VaultStoryAct>,
}

// ------------------------------------------------------------- store copies

pub(crate) fn copy_to_store(
    core: &AppCore,
    actor: &Actor,
    args: VaultCopyArgs,
) -> AppResult<VaultCopyResult> {
    if args.from == args.to {
        return Err(AppError::invalid_input(
            "Choose the other Idea Vault to copy into.",
        ));
    }
    if args.ids.is_empty() {
        return Err(AppError::invalid_input("Select at least one item."));
    }
    if args.ids.len() > super::items::MAX_BATCH {
        return Err(AppError::invalid_input(
            "Copy at most 1000 items at a time.",
        ));
    }
    // Snapshot the source items (requires View on the source vault).
    let sources: Vec<VaultItemDto> = read_vault(core, actor, args.from, |c, root| {
        args.ids
            .iter()
            .map(|id| load_item(c, root, args.from, id))
            .collect()
    })?;
    let summary = if sources.len() == 1 {
        format!(
            "Copied “{}” from the {}",
            sources[0].display_name,
            store_label(args.from)
        )
    } else {
        format!(
            "Copied {} items from the {}",
            sources.len(),
            store_label(args.from)
        )
    };
    let src_actor = vault_actor(actor, args.from)?;
    src_actor.require(Capability::View, "view the Idea Vault")?;
    let (copied, failed) = with_store(core, args.from, |src| {
        let src_root = src.root().to_path_buf();
        src.read(|src_conn| {
            mutate_vault(
                core,
                actor,
                args.to,
                MutationMeta::new("vault.copy_to_store", summary.clone(), Capability::Edit),
                |tx| {
                    check_folder(tx.conn(), args.folder_id.as_deref())?;
                    if let Some(c) = &args.collection_id {
                        check_collection(tx.conn(), c)?;
                    }
                    let mut copied = Vec::new();
                    let mut failed = Vec::new();
                    for item in &sources {
                        tx.conn().execute_batch("SAVEPOINT vault_copy_one")?;
                        let one = (|| -> AppResult<String> {
                            let asset_id = match &item.asset {
                                Some(a) => {
                                    Some(copy_asset_between(src_conn, &src_root, &a.id, tx)?.id)
                                }
                                None => None,
                            };
                            if let (Some(new_asset), Some(a)) = (&asset_id, &item.asset) {
                                // Keep the recording length / media facts on the copy.
                                tx.conn().execute(
                                    "UPDATE asset SET duration_ms=?1 WHERE id=?2",
                                    params![a.duration_ms, new_asset],
                                )?;
                            }
                            let new = NewItem {
                                item_type: Some(item.item_type),
                                title: item.title.clone(),
                                body: item.body.clone(),
                                caption: item.caption.clone(),
                                url: item.url.clone(),
                                source_text: item.source_text.clone(),
                                asset_id,
                                folder_id: args.folder_id.clone(),
                                // Informational provenance only; no sync (FSD §5.8).
                                source_global_item_id: match args.from {
                                    StoreSel::Global => Some(item.id.clone()),
                                    StoreSel::Project => None,
                                },
                            };
                            insert_item(tx, &new, args.collection_id.as_deref(), &item.tags)
                        })();
                        match one {
                            Ok(id) => {
                                tx.conn().execute_batch("RELEASE vault_copy_one")?;
                                copied.push(id);
                            }
                            Err(e)
                                if matches!(
                                    e.code_str(),
                                    "storage.disk_full" | "storage.write_failed" | "storage.busy"
                                ) =>
                            {
                                return Err(e);
                            }
                            Err(e) => {
                                tx.conn().execute_batch(
                                    "ROLLBACK TO vault_copy_one; RELEASE vault_copy_one",
                                )?;
                                failed.push(VaultFailedFile {
                                    path: item.id.clone(),
                                    name: item.display_name.clone(),
                                    reason: e.message,
                                });
                            }
                        }
                    }
                    Ok((copied, failed))
                },
            )
        })
    })?;
    Ok(VaultCopyResult { copied, failed })
}

// -------------------------------------------------------------- send to story

pub(crate) fn story_targets(
    core: &AppCore,
    actor: &Actor,
    _: VaultStoryTargetsArgs,
) -> AppResult<VaultStoryTargets> {
    actor.require(Capability::View, "view the Story Board")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut acts: Vec<VaultStoryAct> = {
            let mut stmt = c.prepare(
                "SELECT id, title, episode_id FROM story_act WHERE deleted_at IS NULL ORDER BY episode_id, position, id",
            )?;
            stmt.query_map([], |r| {
                Ok(VaultStoryAct { id: r.get(0)?, title: r.get(1)?, episode_id: r.get(2)?, sequences: vec![] })
            })?
            .collect::<Result<_, _>>()?
        };
        let mut stmt = c.prepare(
            "SELECT id, title FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        for act in &mut acts {
            act.sequences = stmt
                .query_map([&act.id], |r| Ok(VaultStorySequence { id: r.get(0)?, title: r.get(1)? }))?
                .collect::<Result<_, _>>()?;
        }
        Ok(VaultStoryTargets { acts })
    })
}

fn clip(text: &str, max_chars: usize) -> String {
    let t = text.trim();
    if t.chars().count() <= max_chars {
        return t.to_string();
    }
    let mut out: String = t
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim_end()
        .to_string();
    out.push('…');
    out
}

/// The short text a Story object is built from, and the longer remainder kept as its note.
fn story_texts(item: &VaultItemDto) -> (String, Option<String>) {
    let body = item
        .body
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty());
    let mut extra: Vec<String> = Vec::new();
    let short = match (
        item.title
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty()),
        body,
    ) {
        (Some(t), Some(b)) => {
            extra.push(b.to_string());
            t.to_string()
        }
        (Some(t), None) => t.to_string(),
        (None, Some(b)) => {
            if b.chars().count() > 300 || b.lines().count() > 3 {
                extra.push(b.to_string());
            }
            b.to_string()
        }
        (None, None) => item.display_name.clone(),
    };
    if item.item_type == VaultItemType::Quote
        && let Some(src) = item.source_text.as_deref().filter(|s| !s.trim().is_empty())
    {
        extra.push(format!("— {src}"));
    }
    if let Some(c) = item.caption.as_deref().filter(|c| !c.trim().is_empty()) {
        extra.push(c.to_string());
    }
    if let Some(u) = item.url.as_deref() {
        extra.push(u.to_string());
    }
    if let Some(a) = &item.asset
        && (item.title.is_some() || body.is_some())
    {
        extra.push(format!("File: {}", a.original_name));
    }
    let note = if extra.is_empty() {
        None
    } else {
        Some(extra.join("\n\n"))
    };
    (short, note)
}

/// Resolve and validate the placement; returns (parent_type, parent_id, episode_id).
fn placement(
    c: &Connection,
    target: VaultStoryTarget,
    parent_type: Option<&str>,
    parent_id: Option<&str>,
) -> AppResult<(Option<String>, Option<String>, Option<String>)> {
    let act_episode = |id: &str| -> AppResult<Option<String>> {
        c.query_row(
            "SELECT episode_id FROM story_act WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("act"))
    };
    match target {
        VaultStoryTarget::Character => Ok((None, None, None)),
        VaultStoryTarget::Sequence => {
            if parent_type.unwrap_or("act") != "act" {
                return Err(AppError::invalid_input(
                    "A sequence idea must be placed in an act.",
                ));
            }
            let act = parent_id.ok_or_else(|| AppError::required("Act"))?;
            let ep = act_episode(act)?;
            Ok((Some("act".into()), Some(act.to_string()), ep))
        }
        VaultStoryTarget::Beat | VaultStoryTarget::SceneCard => {
            match parent_type.unwrap_or("unassigned") {
                "parking" | "unassigned" => Ok((
                    Some(parent_type.unwrap_or("unassigned").to_string()),
                    None,
                    None,
                )),
                "act" => {
                    let act = parent_id.ok_or_else(|| AppError::required("Act"))?;
                    let ep = act_episode(act)?;
                    Ok((Some("act".into()), Some(act.to_string()), ep))
                }
                "sequence" => {
                    let seq = parent_id.ok_or_else(|| AppError::required("Sequence"))?;
                    let act_id: Option<String> = c
                        .query_row(
                            "SELECT act_id FROM story_sequence WHERE id=?1 AND deleted_at IS NULL",
                            [seq],
                            |r| r.get(0),
                        )
                        .optional()?
                        .ok_or_else(|| AppError::not_found("sequence"))?;
                    let ep = match act_id {
                        Some(a) => c
                            .query_row("SELECT episode_id FROM story_act WHERE id=?1", [a], |r| {
                                r.get::<_, Option<String>>(0)
                            })
                            .optional()?
                            .flatten(),
                        None => None,
                    };
                    Ok((Some("sequence".into()), Some(seq.to_string()), ep))
                }
                _ => Err(AppError::invalid_input("Choose where to place the copy.")),
            }
        }
    }
}

fn place_label(c: &Connection, parent_type: Option<&str>, parent_id: Option<&str>) -> String {
    let title = |sql: &str, id: &str| {
        c.query_row(sql, [id], |r| r.get::<_, String>(0))
            .unwrap_or_default()
    };
    match (parent_type, parent_id) {
        (Some("parking"), _) => "in the Parking Lot".into(),
        (Some("unassigned"), _) => "on the Story Board (not yet placed)".into(),
        (Some("act"), Some(id)) => format!(
            "in {}",
            title("SELECT title FROM story_act WHERE id=?1", id)
        ),
        (Some("sequence"), Some(id)) => format!(
            "in {}",
            title("SELECT title FROM story_sequence WHERE id=?1", id)
        ),
        _ => "in Story".into(),
    }
}

pub(crate) fn send_to_story(
    core: &AppCore,
    actor: &Actor,
    args: VaultSendToStoryArgs,
) -> AppResult<VaultSendToStoryResult> {
    let item = read_vault(core, actor, args.store, |c, root| {
        load_item(c, root, args.store, &args.id)
    })?;
    let (short, note) = story_texts(&item);
    let session = core.project()?;
    let label = match args.target {
        VaultStoryTarget::Beat => "Beat",
        VaultStoryTarget::SceneCard => "Scene Card",
        VaultStoryTarget::Sequence => "Sequence",
        VaultStoryTarget::Character => "Character",
    };
    let summary = format!("Sent “{}” to Story as a {label}", item.display_name);
    // Only project items keep an informational "Source: Idea Vault" link; a Global
    // item id means nothing inside the project.
    let source_id = match args.store {
        StoreSel::Project => Some(item.id.clone()),
        StoreSel::Global => None,
    };
    let src_root = match args.store {
        StoreSel::Global => Some(core.global_store()?.root().to_path_buf()),
        StoreSel::Project => None,
    };
    let global = match args.store {
        StoreSel::Global => Some(core.global_store()?),
        StoreSel::Project => None,
    };
    session.store.mutate(actor, MutationMeta::new("vault.send_to_story", summary, Capability::Edit), |tx| {
        let c = tx.conn();
        let (ptype, pid, episode) = placement(c, args.target, args.parent_type.as_deref(), args.parent_id.as_deref())?;
        let now = now_ms();
        let id = new_id();
        let table = match args.target {
            VaultStoryTarget::Beat => {
                let pos = next_position(c, "story_beat", "parent_type = ?1 AND parent_id IS ?2", &[
                    text(ptype.clone().unwrap_or_default()),
                    opt_text(pid.clone()),
                ])?;
                c.execute(
                    "INSERT INTO story_beat(id, episode_id, parent_type, parent_id, text, note, source_vault_item_id,
                                            position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![id, episode, ptype, pid, clip(&short, 2_000), note, source_id, pos, now],
                )?;
                "story_beat"
            }
            VaultStoryTarget::SceneCard => {
                let pos = next_position(c, "story_scene_card", "parent_type = ?1 AND parent_id IS ?2", &[
                    text(ptype.clone().unwrap_or_default()),
                    opt_text(pid.clone()),
                ])?;
                c.execute(
                    "INSERT INTO story_scene_card(id, episode_id, parent_type, parent_id, short_description, notes,
                                                  source_vault_item_id, position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![id, episode, ptype, pid, clip(&short, 500), note, source_id, pos, now],
                )?;
                "story_scene_card"
            }
            VaultStoryTarget::Sequence => {
                let pos = next_position(c, "story_sequence", "act_id IS ?1", &[opt_text(pid.clone())])?;
                c.execute(
                    "INSERT INTO story_sequence(id, act_id, title, note, position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![id, pid, clip(&short, 200), note, pos, now],
                )?;
                "story_sequence"
            }
            VaultStoryTarget::Character => {
                let pos = next_position(c, "story_character", "1=1", &[] as &[SqlValue])?;
                // An image item becomes the character's picture (independent asset row).
                let image = match (&item.asset, item.item_type) {
                    (Some(a), VaultItemType::Image | VaultItemType::Screenshot | VaultItemType::Sketch) => {
                        match (&global, &src_root) {
                            (Some(g), Some(root)) => Some(g.read(|gc| copy_asset_between(gc, root, &a.id, tx))?.id),
                            _ => Some(a.id.clone()),
                        }
                    }
                    _ => None,
                };
                let (name, notes) = match item.title.as_deref().filter(|t| !t.trim().is_empty()) {
                    Some(t) => (clip(t, 120), note.clone()),
                    None => (clip(&short, 120), if short.chars().count() > 120 { Some(short.clone()) } else { note.clone() }),
                };
                c.execute(
                    "INSERT INTO story_character(id, name, notes, image_asset_id, position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![id, name, notes, image, pos, now],
                )?;
                "story_character"
            }
        };
        let place = place_label(c, ptype.as_deref(), pid.as_deref());
        let kind = type_label(item.item_type).to_lowercase();
        let message = match args.target {
            VaultStoryTarget::Character => format!("Created a Character from this {kind}. The original stays in your Idea Vault."),
            _ => format!("Created a {label} copy {place}. The original stays in your Idea Vault."),
        };
        Ok(VaultSendToStoryResult {
            target: args.target,
            story_id: id,
            table: table.to_string(),
            parent_type: ptype,
            parent_id: pid,
            message,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_keeps_short_text() {
        assert_eq!(clip("  hello ", 10), "hello");
        assert_eq!(clip("abcdefghijk", 5), "abcde…");
    }
}
