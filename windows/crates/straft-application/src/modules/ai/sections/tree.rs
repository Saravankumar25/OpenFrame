//! `ai.sections`: the tree the assistant panel shows for the open project. Only the leaves the
//! person may read are listed, Episodes only on an episodic project, and for the instance pickers
//! the screenplays, drafts, scenes, shooting days and call sheets to choose from.
//!
//! The tree is a read of the leaf table and of the project; it changes nothing and runs no model.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use straft_domain::{Actor, AppResult, Capability};
use ts_rs::TS;

use super::{BRANCHES, LEAVES, Picker};
use crate::core::AppCore;
use crate::modules::ai::queries;
use crate::modules::schedule::board;

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiSectionsArgs {
    /// List the scenes of this draft (the screenplay it belongs to); otherwise each screenplay
    /// lists the scenes of its current draft.
    #[serde(default)]
    #[ts(optional)]
    pub draft_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiLeafDto {
    /// `production/cast-crew`
    pub id: String,
    pub label: String,
    /// `Production > Cast & Crew`
    pub path_label: String,
    /// none, screenplay, scene, shootingDay, dayAndScene or callSheet.
    pub picker: String,
    /// Why the leaf cannot answer, when it cannot.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiBranchDto {
    pub id: String,
    pub label: String,
    pub leaves: Vec<AiLeafDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiPickDraftDto {
    pub id: String,
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiPickSceneDto {
    pub id: String,
    #[ts(type = "number")]
    pub number: i64,
    pub heading: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiPickScreenplayDto {
    pub id: String,
    pub title: String,
    pub drafts: Vec<AiPickDraftDto>,
    /// The draft the scenes below belong to.
    pub scenes_draft_id: Option<String>,
    pub scenes: Vec<AiPickSceneDto>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiPickDayDto {
    pub id: String,
    /// "Shoot Day 4"
    pub label: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiPickCallSheetDto {
    pub id: String,
    pub title: String,
    pub day_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiSectionTreeDto {
    pub branches: Vec<AiBranchDto>,
    pub screenplays: Vec<AiPickScreenplayDto>,
    pub days: Vec<AiPickDayDto>,
    pub call_sheets: Vec<AiPickCallSheetDto>,
}

/// The `ai.sections` query.
pub fn sections(
    core: &AppCore,
    actor: &Actor,
    args: AiSectionsArgs,
) -> AppResult<AiSectionTreeDto> {
    actor.require(Capability::UseAi, "use Straft AI")?;
    let s = core.project()?;
    s.store.read(|c| tree(c, actor, args.draft_id.as_deref()))
}

pub fn tree(c: &Connection, actor: &Actor, draft_id: Option<&str>) -> AppResult<AiSectionTreeDto> {
    let episodic: bool = c
        .query_row("SELECT project_type FROM project LIMIT 1", [], |r| {
            r.get::<_, String>(0)
        })
        .map(|t| matches!(t.as_str(), "Episodic" | "Series"))
        .unwrap_or(false);
    let mut branches = Vec::new();
    for b in BRANCHES {
        let leaves: Vec<AiLeafDto> = LEAVES
            .iter()
            .filter(|l| l.branch == b.id)
            .filter(|l| actor.can(l.min_capability))
            .filter(|l| l.id != "story/episodes" || episodic)
            .map(|l| AiLeafDto {
                id: l.id.into(),
                label: l.label.into(),
                path_label: l.path_label(),
                picker: l.picker.code().into(),
                note: l.note.map(Into::into),
            })
            .collect();
        if !leaves.is_empty() {
            branches.push(AiBranchDto {
                id: b.id.into(),
                label: b.label.into(),
                leaves,
            });
        }
    }
    let wants = |p: Picker| {
        branches
            .iter()
            .flat_map(|b| &b.leaves)
            .any(|l| l.picker == p.code())
    };
    let screenplays = if wants(Picker::Screenplay) {
        screenplays(c, draft_id)?
    } else {
        Vec::new()
    };
    let (days, call_sheets) =
        if wants(Picker::ShootingDay) || wants(Picker::DayAndScene) || wants(Picker::CallSheet) {
            (days(c)?, call_sheets(c)?)
        } else {
            (Vec::new(), Vec::new())
        };
    Ok(AiSectionTreeDto {
        branches,
        screenplays,
        days,
        call_sheets,
    })
}

fn screenplays(c: &Connection, draft_id: Option<&str>) -> AppResult<Vec<AiPickScreenplayDto>> {
    let drafts = queries::list_drafts(c)?;
    let mut out: Vec<AiPickScreenplayDto> = Vec::new();
    for d in &drafts {
        let at = match out.iter().position(|s| s.id == d.screenplay_id) {
            Some(i) => i,
            None => {
                out.push(AiPickScreenplayDto {
                    id: d.screenplay_id.clone(),
                    title: d.screenplay_title.clone(),
                    drafts: Vec::new(),
                    scenes_draft_id: None,
                    scenes: Vec::new(),
                });
                out.len() - 1
            }
        };
        out[at].drafts.push(AiPickDraftDto {
            id: d.id.clone(),
            name: d.name.clone(),
            current: d.is_current,
        });
    }
    for sp in &mut out {
        // The scenes of the draft asked for when it is this screenplay's, else of the current draft
        // (the latest when none is current).
        let pick = draft_id
            .and_then(|id| sp.drafts.iter().find(|d| d.id == id))
            .or_else(|| sp.drafts.iter().find(|d| d.current))
            .or_else(|| sp.drafts.last())
            .map(|d| d.id.clone());
        if let Some(id) = pick {
            sp.scenes = queries::scenes(c, &id)?
                .into_iter()
                .filter(|s| !s.omitted)
                .map(|s| AiPickSceneDto {
                    id: s.id,
                    number: s.number as i64,
                    heading: s.heading,
                })
                .collect();
            sp.scenes_draft_id = Some(id);
        }
    }
    Ok(out)
}

fn days(c: &Connection) -> AppResult<Vec<AiPickDayDto>> {
    let Some(sched) = board::current_schedule(c)? else {
        return Ok(Vec::new());
    };
    Ok(board::load_days(c, &sched.id)?
        .into_iter()
        .filter(|d| !d.off)
        .map(|d| AiPickDayDto {
            label: d.label(),
            date: d.date.clone(),
            id: d.id,
        })
        .collect())
}

fn call_sheets(c: &Connection) -> AppResult<Vec<AiPickCallSheetDto>> {
    let mut st = c.prepare(
        "SELECT id, title, shoot_day_id FROM call_sheet
         WHERE deleted_at IS NULL AND superseded_by IS NULL ORDER BY created_at, revision LIMIT 200",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(AiPickCallSheetDto {
                id: r.get(0)?,
                title: r.get(1)?,
                day_id: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}
