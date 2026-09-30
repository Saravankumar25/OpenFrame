//! Context resolution (AI spec §7, FSD §42.4): turn the scope the user picked
//! into (a) a human "Using: …" label, (b) references for deterministic tools
//! and (c) the MINIMAL project text the model may read.
//!
//! Private-note boundary: selection context is read from the search projection
//! with the same owner filter Global Search uses, so another user's Private
//! Notes can never reach the model (Security §10.2).

use openframe_domain::{Actor, AppError, AppResult};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::queries::{self, DraftRef, SceneRow};

/// Scope selector values (FSD §42.4; UX §3.40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AiScopeKind {
    CurrentSelection,
    CurrentScene,
    CurrentScreenplay,
    SpecificDraft,
    StoryBoard,
    IdeaVaultSelection,
    Production,
    ShootingDay,
    CallSheet,
    WholeProject,
}

impl AiScopeKind {
    pub fn label(self) -> &'static str {
        match self {
            AiScopeKind::CurrentSelection => "Current Selection",
            AiScopeKind::CurrentScene => "Current Scene",
            AiScopeKind::CurrentScreenplay => "Current Screenplay",
            AiScopeKind::SpecificDraft => "Specific Draft",
            AiScopeKind::StoryBoard => "Story Board",
            AiScopeKind::IdeaVaultSelection => "Selected Idea Vault Items",
            AiScopeKind::Production => "Production",
            AiScopeKind::ShootingDay => "Shooting Day",
            AiScopeKind::CallSheet => "Call Sheet",
            AiScopeKind::WholeProject => "Whole Project",
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            AiScopeKind::CurrentSelection => "CurrentSelection",
            AiScopeKind::CurrentScene => "CurrentScene",
            AiScopeKind::CurrentScreenplay => "CurrentScreenplay",
            AiScopeKind::SpecificDraft => "SpecificDraft",
            AiScopeKind::StoryBoard => "StoryBoard",
            AiScopeKind::IdeaVaultSelection => "IdeaVaultSelection",
            AiScopeKind::Production => "Production",
            AiScopeKind::ShootingDay => "ShootingDay",
            AiScopeKind::CallSheet => "CallSheet",
            AiScopeKind::WholeProject => "WholeProject",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiSelectionRef")]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionRef {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiScopeArgs {
    pub kind: AiScopeKind,
    #[serde(default)]
    pub draft_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
    /// Selected objects (Scene Cards, Idea Vault items, files, …) by id.
    #[serde(default)]
    pub selection: Vec<SelectionRef>,
    #[serde(default)]
    pub shooting_day_id: Option<String>,
    #[serde(default)]
    pub call_sheet_id: Option<String>,
}

/// One piece of project text given to the model — always marked as untrusted data.
#[derive(Debug, Clone)]
pub struct ContextItem {
    pub source: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedScope {
    pub kind: AiScopeKind,
    /// "Using: Draft 7 — Director Rewrite"
    pub label: String,
    /// Draft deterministic screenplay tools default to.
    pub draft: Option<DraftRef>,
    pub scene: Option<SceneRow>,
    pub selection: Vec<String>,
    pub context: Vec<ContextItem>,
}

impl ResolvedScope {
    /// Stored with the AI Request: references only, never content copies.
    pub fn refs_json(&self) -> Value {
        json!({
            "kind": self.kind.as_str(),
            "label": self.label,
            "draftId": self.draft.as_ref().map(|d| d.id.clone()),
            "sceneId": self.scene.as_ref().map(|s| s.id.clone()),
            "selection": self.selection,
        })
    }
}

/// Total characters of project text a request may carry to the model.
pub const CONTEXT_BUDGET: usize = 6_000;
const ITEM_BUDGET: usize = 2_500;

fn search_doc(
    c: &Connection,
    actor: &Actor,
    id: &str,
) -> AppResult<Option<(String, String, String, String)>> {
    Ok(c.query_row(
        "SELECT entity_type, title, body, context FROM search_doc
         WHERE entity_id=?1 AND (owner_user_id IS NULL OR owner_user_id=?2) LIMIT 1",
        params![id, actor.user_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .optional()?)
}

fn push(ctx: &mut Vec<ContextItem>, used: &mut usize, source: String, text: String) {
    if *used >= CONTEXT_BUDGET || text.trim().is_empty() {
        return;
    }
    let room = (CONTEXT_BUDGET - *used).min(ITEM_BUDGET);
    let text = queries::truncate_chars(text.trim(), room);
    *used += text.len();
    ctx.push(ContextItem { source, text });
}

pub fn resolve(c: &Connection, actor: &Actor, args: &AiScopeArgs) -> AppResult<ResolvedScope> {
    let mut ctx = Vec::new();
    let mut used = 0usize;
    let current = queries::current_draft(c)?;
    let mut out = ResolvedScope {
        kind: args.kind,
        label: format!("Using: {}", args.kind.label()),
        draft: current.clone(),
        scene: None,
        selection: Vec::new(),
        context: Vec::new(),
    };
    match args.kind {
        AiScopeKind::CurrentScene => {
            let id = args.scene_id.as_deref().ok_or_else(|| {
                AppError::ai(
                    "scope_missing",
                    "Open a scene in the Screenplay first, or choose a different scope.",
                )
            })?;
            let (draft, scene) = queries::scene_by_id(c, id)?.ok_or_else(|| {
                AppError::ai("scope_missing", "That scene is no longer available.")
            })?;
            out.label = format!("Using: {} ({})", scene.label(), draft.label());
            push(
                &mut ctx,
                &mut used,
                scene.label(),
                queries::scene_text(c, &scene.id, ITEM_BUDGET)?,
            );
            out.draft = Some(draft);
            out.scene = Some(scene);
        }
        AiScopeKind::CurrentScreenplay | AiScopeKind::SpecificDraft => {
            let draft = match (args.kind, args.draft_id.as_deref()) {
                (AiScopeKind::SpecificDraft, Some(id))
                | (AiScopeKind::CurrentScreenplay, Some(id)) => queries::draft_by_id(c, id)?
                    .ok_or_else(|| {
                        AppError::ai("scope_missing", "That draft is no longer available.")
                    })?,
                (AiScopeKind::SpecificDraft, None) => {
                    return Err(AppError::ai("scope_missing", "Choose which draft to use."));
                }
                _ => current.clone().ok_or_else(|| {
                    AppError::ai(
                        "no_screenplay",
                        "This project doesn't have a screenplay draft yet.",
                    )
                })?,
            };
            out.label = format!("Using: {}", draft.label());
            let scenes = queries::scenes(c, &draft.id)?;
            let outline: Vec<String> = scenes.iter().take(80).map(|s| s.label()).collect();
            push(
                &mut ctx,
                &mut used,
                format!("{} — scene list", draft.label()),
                outline.join("\n"),
            );
            out.draft = Some(draft);
        }
        AiScopeKind::StoryBoard => {
            out.label = "Using: Story Board".into();
            push(
                &mut ctx,
                &mut used,
                "Story Board outline".into(),
                story_outline(c)?,
            );
        }
        AiScopeKind::IdeaVaultSelection | AiScopeKind::CurrentSelection => {
            if args.selection.is_empty() {
                return Err(AppError::ai(
                    "scope_missing",
                    "Select one or more items first, or choose a different scope.",
                ));
            }
            let mut names = Vec::new();
            for s in args.selection.iter().take(20) {
                // Missing or not-visible-to-you items are skipped silently: their existence is not revealed.
                if let Some((etype, title, body, context)) = search_doc(c, actor, &s.id)? {
                    names.push(if title.is_empty() {
                        etype.clone()
                    } else {
                        title.clone()
                    });
                    push(&mut ctx, &mut used, format!("{context}: {title}"), body);
                    out.selection.push(s.id.clone());
                }
            }
            let what = if args.kind == AiScopeKind::IdeaVaultSelection {
                "Selected Idea Vault items"
            } else {
                "Selection"
            };
            out.label = if names.is_empty() {
                format!("Using: {what} (nothing available)")
            } else {
                format!(
                    "Using: {what} — {}",
                    queries::truncate_chars(&queries::join_and(&names), 80)
                )
            };
        }
        AiScopeKind::Production => {
            let src = queries::production_source_draft(c)?;
            let draft = match &src {
                Some((_, d)) => queries::draft_by_id(c, d)?,
                None => None,
            };
            out.label = match &draft {
                Some(d) => format!("Using: Production (Production Source: {})", d.label()),
                None => "Using: Production".into(),
            };
            if draft.is_some() {
                out.draft = draft;
            }
        }
        AiScopeKind::ShootingDay | AiScopeKind::CallSheet => {
            let id = if args.kind == AiScopeKind::ShootingDay {
                args.shooting_day_id.as_deref()
            } else {
                args.call_sheet_id.as_deref()
            };
            let what = args.kind.label();
            let id = id.ok_or_else(|| {
                AppError::ai(
                    "scope_missing",
                    format!("Open a {what} first, or choose a different scope."),
                )
            })?;
            match search_doc(c, actor, id)? {
                Some((_, title, body, _)) => {
                    out.label = format!("Using: {what} — {title}");
                    push(&mut ctx, &mut used, format!("{what}: {title}"), body);
                    out.selection.push(id.to_string());
                }
                None => {
                    return Err(AppError::ai(
                        "scope_missing",
                        format!("That {what} is no longer available."),
                    ));
                }
            }
        }
        AiScopeKind::WholeProject => {
            out.label = "Using: Whole Project".into();
        }
    }
    out.context = ctx;
    Ok(out)
}

/// Titles-only outline of the Story Board (acts → sequences → cards), budgeted.
pub fn story_outline(c: &Connection) -> AppResult<String> {
    let mut lines = Vec::new();
    let mut acts =
        c.prepare("SELECT id, title FROM story_act WHERE deleted_at IS NULL ORDER BY position")?;
    let acts: Vec<(String, String)> = acts
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (act_id, title) in acts {
        lines.push(format!("Act: {title}"));
        let mut cards = c.prepare(
            "SELECT short_description FROM story_scene_card WHERE deleted_at IS NULL AND
               ((parent_type='act' AND parent_id=?1) OR (parent_type='sequence' AND parent_id IN
                 (SELECT id FROM story_sequence WHERE act_id=?1 AND deleted_at IS NULL)))
             ORDER BY position LIMIT 40",
        )?;
        let cards: Vec<String> = cards
            .query_map([&act_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for card in cards {
            lines.push(format!("  - {}", queries::truncate_chars(&card, 140)));
        }
        if lines.len() > 120 {
            break;
        }
    }
    Ok(lines.join("\n"))
}
