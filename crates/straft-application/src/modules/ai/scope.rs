//! What a request reads and what its tools default to, built from the sections the person picked
//! (ADR-0041). The picks are the only source: nothing here comes from the screen.

use straft_domain::AppResult;
use rusqlite::Connection;
use serde_json::{Value, json};

use super::queries::{self, DraftRef, SceneRow};
use super::sections::{self, Selection};

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    /// "Using: Screenplay (Draft 2, all scenes) + Cast & Crew"
    pub label: String,
    /// The draft deterministic screenplay tools default to: the picked one, else the current one.
    pub draft: Option<DraftRef>,
    /// The scene a tool means when the message names none: the picked one.
    pub scene: Option<SceneRow>,
    /// The sections the person picked. The tools offered and what the general tools read are held
    /// to them.
    pub sections: Vec<&'static str>,
}

impl ResolvedScope {
    /// The scope of a request with these picks.
    pub fn of_selection(c: &Connection, sel: &Selection) -> AppResult<Self> {
        let draft = match sel.draft() {
            Some(d) => Some(d.clone()),
            None => queries::current_draft(c)?,
        };
        Ok(Self {
            label: sel.label.clone(),
            draft,
            scene: sel.scene().cloned(),
            sections: sel.codes(),
        })
    }

    /// No request behind it: nothing is held back and no draft is assumed.
    pub fn every_section() -> Self {
        Self {
            label: sections::EVERY_SECTION_LABEL.to_string(),
            draft: None,
            scene: None,
            sections: sections::LEAVES.iter().map(|l| l.id).collect(),
        }
    }

    /// What the request read, for the sources of an answer: "Budget + Cast & Crew".
    pub fn basis(&self) -> &str {
        self.label.trim_start_matches("Using: ")
    }

    /// Every section, on the current draft: a stored proposal checked again, whichever sections it was
    /// made under.
    pub fn on_current_draft(c: &Connection) -> AppResult<Self> {
        Ok(Self {
            draft: queries::current_draft(c)?,
            ..Self::every_section()
        })
    }

    /// May a tool read a document with this navigation target? Only when a picked section holds it.
    pub fn reads(&self, nav: &Value) -> bool {
        self.sections
            .iter()
            .filter_map(|id| sections::leaf(id))
            .any(|l| l.search.iter().any(|r| r.matches(nav)))
    }

    /// Stored with the AI Request: references only, never content copies.
    pub fn refs_json(&self) -> Value {
        json!({
            "label": self.label,
            "draftId": self.draft.as_ref().map(|d| d.id.clone()),
            "sceneId": self.scene.as_ref().map(|s| s.id.clone()),
            "sections": self.sections,
        })
    }
}
