//! Story Board Card Display Settings (Screenplay and Production Depth Specification sections
//! 21.2, 21.3 and 52; docs/engineering/33-production-depth-contract.md section 1).
//!
//! What a minimized Scene Card shows is a per-user view setting, stored in `sys_view_state`
//! under `story.card_display`. It is never project content: changing it never touches a card,
//! a scene or the screenplay, never enters the undo history and never reaches an exchange
//! package. The assistant does not see or change it (it is view state).

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::registry::Registry;
use crate::store::{MutationMeta, Tx};

const KEY: &str = "story.card_display";

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("story.card_display", get).meta(
        M::read("What minimized Scene Cards show on this user's Story Board (view setting).")
            .hidden(h::VIEW_STATE),
    );
    r.command("story.set_card_display", set).meta(
        M::command(
            Capability::View,
            "Choose what minimized Scene Cards show (a preset or your own selection).",
        )
        .hidden(h::VIEW_STATE),
    );
}

/// The four presets (section 21.3). Custom is any combination the user ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StoryCardPreset {
    /// Short description with small indicators only (the original compact card).
    #[default]
    Minimal,
    Story,
    Production,
    Custom,
}

/// The twelve checkboxes of Story Board, Card Display Settings (section 21.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct StoryCardToggles {
    pub scene_heading: bool,
    pub short_description: bool,
    pub long_description_preview: bool,
    pub notes: bool,
    pub attachments: bool,
    pub character_count: bool,
    pub location: bool,
    pub day_night: bool,
    pub comments: bool,
    pub revision_change: bool,
    pub breakdown_status: bool,
    pub shot_status: bool,
}

impl StoryCardToggles {
    pub fn for_preset(p: StoryCardPreset) -> Option<StoryCardToggles> {
        let base = StoryCardToggles {
            short_description: true,
            comments: true,
            ..Default::default()
        };
        match p {
            StoryCardPreset::Minimal => Some(base),
            StoryCardPreset::Story => Some(StoryCardToggles {
                scene_heading: true,
                long_description_preview: true,
                notes: true,
                character_count: true,
                ..base
            }),
            StoryCardPreset::Production => Some(StoryCardToggles {
                scene_heading: true,
                attachments: true,
                location: true,
                day_night: true,
                revision_change: true,
                breakdown_status: true,
                shot_status: true,
                ..base
            }),
            StoryCardPreset::Custom => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCardDisplay {
    pub preset: StoryCardPreset,
    pub toggles: StoryCardToggles,
}

impl Default for StoryCardDisplay {
    fn default() -> Self {
        StoryCardDisplay {
            preset: StoryCardPreset::Minimal,
            toggles: StoryCardToggles::for_preset(StoryCardPreset::Minimal).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCardPresetInfo {
    pub preset: StoryCardPreset,
    pub label: String,
    pub description: String,
    pub toggles: StoryCardToggles,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct StoryCardDisplayState {
    pub current: StoryCardDisplay,
    /// Minimal, Story and Production with what each one shows (Custom has no fixed set).
    pub presets: Vec<StoryCardPresetInfo>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryCardDisplayArgs {}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorySetCardDisplayArgs {
    /// Minimal, Story or Production applies that preset; Custom needs `toggles`.
    pub preset: StoryCardPreset,
    #[serde(default)]
    pub toggles: Option<StoryCardToggles>,
}

fn presets() -> Vec<StoryCardPresetInfo> {
    [
        (
            StoryCardPreset::Minimal,
            "Minimal",
            "The short description with small markers for a heading and comments.",
        ),
        (
            StoryCardPreset::Story,
            "Story",
            "Heading, short description, the start of the long description, notes and characters.",
        ),
        (
            StoryCardPreset::Production,
            "Production",
            "Heading, location, day or night, attachments, revisions, breakdown and shots.",
        ),
    ]
    .into_iter()
    .map(|(p, label, description)| StoryCardPresetInfo {
        preset: p,
        label: label.into(),
        description: description.into(),
        toggles: StoryCardToggles::for_preset(p).unwrap_or_default(),
    })
    .collect()
}

/// Read the saved setting; anything unreadable falls back to Minimal (a view setting is never
/// worth an error).
pub(crate) fn read(c: &Connection, user_id: &str) -> AppResult<StoryCardDisplay> {
    let json: Option<String> = c
        .query_row(
            "SELECT value_json FROM sys_view_state WHERE user_id=?1 AND key=?2",
            params![user_id, KEY],
            |r| r.get(0),
        )
        .optional()?;
    let saved: StoryCardDisplay = json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default();
    // A named preset always shows that preset's set, even if an older save disagrees.
    Ok(match StoryCardToggles::for_preset(saved.preset) {
        Some(t) => StoryCardDisplay {
            preset: saved.preset,
            toggles: t,
        },
        None => saved,
    })
}

fn write(tx: &Tx<'_>, d: &StoryCardDisplay) -> AppResult<()> {
    let json = serde_json::to_string(d).map_err(|e| AppError::internal(e.to_string()))?;
    tx.conn().execute(
        "INSERT INTO sys_view_state(user_id, key, value_json, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(user_id, key) DO UPDATE SET value_json=excluded.value_json, updated_at=excluded.updated_at",
        params![tx.actor().user_id, KEY, json, now_ms()],
    )?;
    Ok(())
}

fn get(core: &AppCore, actor: &Actor, _: StoryCardDisplayArgs) -> AppResult<StoryCardDisplayState> {
    actor.require(Capability::View, "view the Story Board")?;
    let current = core.project()?.store.read(|c| read(c, &actor.user_id))?;
    Ok(StoryCardDisplayState {
        current,
        presets: presets(),
    })
}

/// Viewers and Commenters may change it too: it is their own view of the board.
fn set(
    core: &AppCore,
    actor: &Actor,
    a: StorySetCardDisplayArgs,
) -> AppResult<StoryCardDisplayState> {
    let next = match (a.preset, a.toggles) {
        (StoryCardPreset::Custom, Some(t)) => StoryCardDisplay {
            preset: StoryCardPreset::Custom,
            toggles: t,
        },
        (StoryCardPreset::Custom, None) => {
            return Err(AppError::invalid_input(
                "Choose what the cards should show for Custom.",
            ));
        }
        (p, _) => StoryCardDisplay {
            preset: p,
            toggles: StoryCardToggles::for_preset(p).unwrap_or_default(),
        },
    };
    let current = core.project()?.store.mutate(
        actor,
        MutationMeta::new(
            "story.set_card_display",
            "Changed what Story Board cards show",
            Capability::View,
        )
        .not_undoable()
        .quiet(),
        |tx| {
            write(tx, &next)?;
            Ok(next.clone())
        },
    )?;
    Ok(StoryCardDisplayState {
        current,
        presets: presets(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_differ_and_minimal_is_the_compact_card() {
        let m = StoryCardToggles::for_preset(StoryCardPreset::Minimal).unwrap();
        let s = StoryCardToggles::for_preset(StoryCardPreset::Story).unwrap();
        let p = StoryCardToggles::for_preset(StoryCardPreset::Production).unwrap();
        assert!(m.short_description && m.comments && !m.scene_heading);
        assert!(s.long_description_preview && !s.breakdown_status);
        assert!(p.breakdown_status && p.shot_status && !p.long_description_preview);
        assert_ne!(m, s);
        assert_ne!(s, p);
        assert!(StoryCardToggles::for_preset(StoryCardPreset::Custom).is_none());
    }
}
