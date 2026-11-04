//! What a request reads (AI spec §7, FSD §42.4, ADR-0040): the sections the person picked, the draft
//! and scene the agent's tools default to, and the "Using: ..." line.
//!
//! The scope selector of the panel is gone (round R7): the person picks one to three sections
//! (`sections::LEAVES`) and the dossier reads those itself, so a scope carries no project text and
//! no character budget. What is left is the small struct the agent's tools and the Change Set
//! rebuild need: the draft and scene they default to, and the picks.

use rusqlite::Connection;
use serde_json::{Value, json};
use straft_domain::{AppError, AppResult};

use super::index;
use super::queries::{self, DraftRef, SceneRow};
use super::sections::{self, SectionPick};

/// What is written to the `scope_kind` column of a request and of its trace row. Rows written
/// before ADR-0040 hold the old scope kinds (`CurrentScene`, `WholeProject` and so on); nothing
/// reads the column back, so those rows stay as they are.
pub const SCOPE_KIND: &str = "Sections";

/// One piece of project text given to the model or to a retrieval packet, always marked as
/// untrusted data.
#[derive(Debug, Clone)]
pub struct ContextItem {
    pub source: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    /// "Using: Draft 7"
    pub label: String,
    /// Draft deterministic screenplay tools default to.
    pub draft: Option<DraftRef>,
    pub scene: Option<SceneRow>,
    pub selection: Vec<String>,
    /// The sections the person picked for this question (empty for a scope built without picks).
    pub sections: Vec<SectionPick>,
}

impl ResolvedScope {
    /// Stored with the AI Request: references only, never content copies.
    pub fn refs_json(&self) -> Value {
        json!({
            "kind": SCOPE_KIND,
            "label": self.label,
            "draftId": self.draft.as_ref().map(|d| d.id.clone()),
            "sceneId": self.scene.as_ref().map(|s| s.id.clone()),
            "selection": self.selection,
            "sections": self.sections,
        })
    }
}

/// A scope with no picks: the current draft only. For rebuilding a Change Set and for calls made
/// without a question (tests, internal reads). Every tool is open under it.
pub fn whole(c: &Connection) -> AppResult<ResolvedScope> {
    Ok(ResolvedScope {
        label: "Using: Whole Project".into(),
        draft: queries::current_draft(c)?,
        scene: None,
        selection: Vec::new(),
        sections: Vec::new(),
    })
}

/// The scope of a question from the sections the person picked. The picks are checked first
/// (`sections::validate`); this reads what they name: the draft the agent's screenplay tools default
/// to (the one a Screenplay pick names, else the project's current draft) and the scene a pick narrows
/// to. No project text is carried: the dossier reads the picked sections itself.
pub fn from_picks(c: &Connection, picks: &[SectionPick]) -> AppResult<ResolvedScope> {
    let mut draft = queries::current_draft(c)?;
    let mut scene = None;
    let mut named = false;
    // A Screenplay pick names the draft; any pick may narrow to a scene.
    for p in picks.iter().filter(|p| p.leaf == "screenplay/scenes") {
        if let Some(id) = &p.draft_id {
            draft = Some(queries::draft_by_id(c, id)?.ok_or_else(|| {
                AppError::ai("scope_missing", "That draft is no longer available.")
            })?);
            named = true;
        } else if let Some(id) = &p.screenplay_id
            && let Some(d) = index::draft_for_screenplay(c, id)?
        {
            draft = Some(d);
            named = true;
        }
    }
    if let Some(id) = picks.iter().find_map(|p| p.scene_id.as_deref()) {
        let (d, s) = queries::scene_by_id(c, id)?
            .ok_or_else(|| AppError::ai("scope_missing", "That scene is no longer available."))?;
        // The scene's draft stands only when the person named no draft in the Screenplay pick.
        if !named {
            draft = Some(d);
        }
        scene = Some(s);
    }
    Ok(ResolvedScope {
        label: sections::using_label(c, picks)?,
        draft,
        scene,
        selection: Vec::new(),
        sections: picks.to_vec(),
    })
}
