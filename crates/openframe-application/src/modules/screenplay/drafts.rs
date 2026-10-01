//! Screenplays, named drafts, automatic history, lock and revisions
//! (FSD §15.2, §21, §24, §95; Domain "Screenplay Draft", "Automatic History Point").
//!
//! Invariants:
//! * exactly one Current draft per screenplay (`screenplay.current_draft_id`);
//!   the current draft cannot be deleted;
//! * new drafts copy content with new ids and the same `lineage_id`;
//! * restoring (a draft or an automatic history point) creates a NEW draft;
//! * a Locked draft never changes; "Start Revision" creates a Revision draft.

use openframe_domain::enums::{DraftStatus, ElementType, ProjectType};
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_persistence::rows::{opt_int, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::edit::{validate_element, validate_heading};
use super::{
    DraftRow, ScreenplayDraftDto, ScreenplayDto, draft_dto, live_scene_ids, load_draft,
    load_scenes, load_screenplay, project_type, screenplay_for_scope,
};
use crate::core::AppCore;
use crate::modules::comments;
use crate::registry::{Registry, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{optional_text, required_text};

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    use openframe_domain::Capability as Cap;
    r.command("screenplay.create", create)
        .meta(M::edit("Create a screenplay with its first draft."));
    r.command("screenplay.new_draft", new_draft)
        .meta(M::edit("Create a new draft copied from an existing draft."));
    r.command("screenplay.rename_draft", rename_draft)
        .meta(M::edit("Rename a draft or edit its note."));
    r.command("screenplay.set_current_draft", set_current_draft)
        .meta(M::edit("Make a draft the Current draft."));
    r.command("screenplay.restore_draft", restore_draft)
        .meta(M::edit("Restore an earlier draft as a new draft."));
    r.command("screenplay.delete_draft", delete_draft)
        .meta(M::soft_delete("Move a draft to Recently Deleted.").confirm());
    r.query("screenplay.lock_summary", lock_summary)
        .meta(M::compute(
            "Lock check for a draft: status, open comments, scene count.",
        ));
    r.command("screenplay.lock_draft", lock_draft)
        .meta(M::command(Cap::LockOrFinalize, "Lock a draft as the shooting draft.").confirm());
    r.command("screenplay.unlock_draft", unlock_draft)
        .meta(M::command(Cap::ManageProject, "Unlock a locked draft.").hidden(h::LOCK_OVERRIDE));
    r.command("screenplay.start_revision", start_revision)
        .meta(M::edit(
            "Start a revision of a locked draft (new Revision draft).",
        ));
    r.command("screenplay.update_revision", update_revision)
        .meta(M::edit("Edit a revision's label, colour or reason."));
    r.query("screenplay.history_points", history_points)
        .meta(M::read("Automatic history points of a draft."));
    r.command("screenplay.restore_history_point", restore_history_point)
        .meta(M::edit(
            "Restore an automatic history point as a new draft.",
        ));
    r.trash_handler(TrashHandler {
        object_type: "screenplay_draft",
        table: "screenplay_draft",
        label: "Screenplay draft",
        restore: None,
        purge: purge_draft,
    });
}

/// Automatic history: at most one interval point per draft every 5 minutes of editing.
pub const HISTORY_INTERVAL_MS: i64 = 5 * 60 * 1000;
/// Automatic history points kept per draft (oldest are pruned).
pub const HISTORY_KEEP: i64 = 60;
const MAX_DRAFT_NAME: usize = 120;
const MAX_DRAFT_NOTE: usize = 2_000;

/// Named revision colours (FSD §24.5–24.6); any `#rrggbb` is also accepted.
pub const REVISION_COLORS: &[&str] = &[
    "White",
    "Blue",
    "Pink",
    "Yellow",
    "Green",
    "Goldenrod",
    "Buff",
    "Salmon",
    "Cherry",
];

// ------------------------------------------------------- content builders

/// Content for a new scene (used by drafts, history restore, and other modules
/// such as Build Screenplay / import that create screenplay content).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewElement {
    pub element_type: ElementType,
    pub text: String,
    #[serde(default)]
    pub dual: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NewScene {
    /// Keep an existing cross-draft identity (copies); a new lineage when None.
    pub lineage_id: Option<String>,
    pub heading: String,
    pub notes: Option<String>,
    pub synopsis: Option<String>,
    pub story_day: Option<String>,
    pub time_note: Option<String>,
    pub source_scene_card_id: Option<String>,
    /// Body elements (never `scene_heading`).
    pub elements: Vec<NewElement>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    version: u32,
    draft_name: String,
    scenes: Vec<NewScene>,
}

/// Parameters of a new draft.
pub struct DraftSpec<'a> {
    pub name: &'a str,
    pub note: Option<&'a str>,
    pub status: DraftStatus,
    pub created_from: Option<&'a str>,
    /// (label, colour, reason) for Revision drafts.
    pub revision: Option<(String, Option<String>, Option<String>)>,
    pub make_current: bool,
}

fn format_for(pt: ProjectType) -> &'static str {
    match pt {
        ProjectType::FeatureFilm => "Feature",
        ProjectType::ShortFilm => "Short",
        ProjectType::Episodic | ProjectType::Series => "Episodic",
    }
}

/// Create the screenplay container for the project (or an episode). Fails if
/// one already exists in that scope — use a new draft instead (FSD §18.5, §19.2).
pub fn create_screenplay_tx(
    tx: &Tx<'_>,
    episode_id: Option<&str>,
    title: &str,
) -> AppResult<String> {
    let c = tx.conn();
    let pt = project_type(c)?;
    match (pt.is_episodic(), episode_id) {
        (true, None) => {
            return Err(AppError::validation(
                "episode",
                "Choose the episode this screenplay belongs to.",
            ));
        }
        (false, Some(_)) => {
            return Err(AppError::invalid_input(
                "Episodes are only used in episodic projects.",
            ));
        }
        (true, Some(e)) => {
            let ok: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM episode WHERE id=?1 AND deleted_at IS NULL)",
                [e],
                |r| r.get(0),
            )?;
            if !ok {
                return Err(AppError::not_found("episode"));
            }
        }
        (false, None) => {}
    }
    if screenplay_for_scope(c, episode_id)?.is_some() {
        return Err(AppError::conflict(
            "A screenplay already exists here. Create a new draft instead — nothing is overwritten.",
        ));
    }
    let creator: Option<String> =
        c.query_row("SELECT creator FROM project LIMIT 1", [], |r| r.get(0))?;
    let title_page = super::ScreenplayTitlePage {
        title: title.to_string(),
        written_by: creator.unwrap_or_default(),
        ..Default::default()
    };
    let id = new_id();
    let now = now_ms();
    c.execute(
        "INSERT INTO screenplay(id, episode_id, title, format, title_page_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            id,
            episode_id,
            title,
            format_for(pt),
            serde_json::to_string(&title_page).map_err(|e| AppError::internal(e.to_string()))?,
            now
        ],
    )?;
    Ok(id)
}

fn blank_scene() -> NewScene {
    NewScene {
        elements: vec![NewElement {
            element_type: ElementType::Action,
            text: String::new(),
            dual: false,
        }],
        ..Default::default()
    }
}

/// Insert a draft with content. Validates every heading and element. An empty
/// scene list produces one blank scene (the editor always has a heading).
pub fn insert_draft_tx(
    tx: &Tx<'_>,
    screenplay_id: &str,
    spec: DraftSpec<'_>,
    scenes: &[NewScene],
) -> AppResult<String> {
    let c = tx.conn();
    let name = required_text(spec.name, "Draft name", MAX_DRAFT_NAME)?;
    let note = optional_text(spec.note.map(|s| s.to_string()), "Note", MAX_DRAFT_NOTE)?;
    let id = new_id();
    let now = now_ms();
    let (label, color, reason) = match spec.revision {
        Some((l, c, r)) => (Some(l), c, r),
        None => (None, None, None),
    };
    c.execute(
        "INSERT INTO screenplay_draft(id, screenplay_id, name, note, status, created_from_draft_id, revision_label,
                                      revision_color, revision_reason, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
        params![id, screenplay_id, name, note, spec.status.as_str(), spec.created_from, label, color, reason, now],
    )?;
    let blank = [blank_scene()];
    let scenes = if scenes.is_empty() {
        &blank[..]
    } else {
        scenes
    };
    let mut scene_stmt = c.prepare(
        "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, synopsis, notes, story_day, time_note,
                                      source_scene_card_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
    )?;
    let mut el_stmt = c.prepare(
        "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, dual, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
    )?;
    for (i, s) in scenes.iter().enumerate() {
        validate_heading(&s.heading)?;
        let sid = new_id();
        let lineage = s.lineage_id.clone().unwrap_or_else(new_id);
        scene_stmt.execute(params![
            sid,
            id,
            lineage,
            (i + 1) as i64,
            s.heading,
            s.synopsis,
            s.notes,
            s.story_day,
            s.time_note,
            s.source_scene_card_id,
            now
        ])?;
        for (j, e) in s.elements.iter().enumerate() {
            validate_element(e.element_type, &e.text)?;
            el_stmt.execute(params![
                new_id(),
                sid,
                (j + 1) as i64,
                e.element_type.as_str(),
                e.text,
                e.dual,
                now
            ])?;
        }
    }
    if spec.make_current {
        set_current_tx(tx, screenplay_id, &id)?;
    }
    record_history_point(tx, &id, "draft_created")?;
    super::bump_seq(c, &id)?;
    Ok(id)
}

/// Content of a draft as reusable scene specs (lineage preserved).
pub(crate) fn draft_content(c: &Connection, draft_id: &str) -> AppResult<Vec<NewScene>> {
    Ok(load_scenes(c, draft_id)?
        .into_iter()
        .map(|s| NewScene {
            lineage_id: Some(s.lineage_id),
            heading: s.heading,
            notes: s.notes,
            synopsis: s.synopsis,
            story_day: s.story_day,
            time_note: s.time_note,
            source_scene_card_id: s.source_scene_card_id,
            elements: s
                .elements
                .into_iter()
                .map(|e| NewElement {
                    element_type: e.element_type,
                    text: e.text,
                    dual: e.dual,
                })
                .collect(),
        })
        .collect())
}

/// Make a draft Current (the single current draft) and refresh search (only
/// the current draft's scenes are indexed).
pub(crate) fn set_current_tx(tx: &Tx<'_>, screenplay_id: &str, draft_id: &str) -> AppResult<()> {
    let c = tx.conn();
    let old: Option<String> = c.query_row(
        "SELECT current_draft_id FROM screenplay WHERE id=?1",
        [screenplay_id],
        |r| r.get(0),
    )?;
    if old.as_deref() == Some(draft_id) {
        return Ok(());
    }
    update_fields(
        c,
        "screenplay",
        screenplay_id,
        &[("current_draft_id", text(draft_id))],
        &["current_draft_id"],
        None,
        "screenplay",
    )?;
    for d in old.iter().chain(std::iter::once(&draft_id.to_string())) {
        for s in live_scene_ids(c, d)? {
            tx.reindex("screenplay_scene", &s);
        }
    }
    Ok(())
}

fn name_taken(
    c: &Connection,
    screenplay_id: &str,
    name: &str,
    except: Option<&str>,
) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL
                         AND lower(trim(name)) = lower(trim(?2)) AND id IS NOT ?3)",
        params![screenplay_id, name, except],
        |r| r.get(0),
    )?)
}

fn check_name_free(
    c: &Connection,
    screenplay_id: &str,
    name: &str,
    except: Option<&str>,
) -> AppResult<()> {
    if name_taken(c, screenplay_id, name, except)? {
        return Err(AppError::validation(
            "duplicate",
            "Another draft already has this name. Choose a different name.",
        ));
    }
    Ok(())
}

/// `base`, or `base (2)`, `base (3)`… when taken.
pub(crate) fn unique_name(c: &Connection, screenplay_id: &str, base: &str) -> AppResult<String> {
    let base: String = base.chars().take(MAX_DRAFT_NAME - 5).collect();
    if !name_taken(c, screenplay_id, &base, None)? {
        return Ok(base);
    }
    for n in 2..1000 {
        let candidate = format!("{base} ({n})");
        if !name_taken(c, screenplay_id, &candidate, None)? {
            return Ok(candidate);
        }
    }
    Ok(format!("{base} ({})", new_id()))
}

fn validate_color(color: Option<String>) -> AppResult<Option<String>> {
    match color
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
    {
        None => Ok(None),
        Some(c) => {
            let named = REVISION_COLORS.iter().find(|n| n.eq_ignore_ascii_case(&c));
            if let Some(n) = named {
                return Ok(Some(n.to_string()));
            }
            let hex =
                c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit());
            if hex {
                Ok(Some(c.to_ascii_lowercase()))
            } else {
                Err(AppError::invalid_input(
                    "Choose a revision colour or enter a colour like #3366cc.",
                ))
            }
        }
    }
}

// ----------------------------------------------------------- history points

pub(crate) fn record_history_point(tx: &Tx<'_>, draft_id: &str, reason: &str) -> AppResult<()> {
    let c = tx.conn();
    let d = load_draft(c, draft_id)?;
    let scenes = draft_content(c, draft_id)?;
    let element_count: usize = scenes.iter().map(|s| s.elements.len()).sum();
    let snap = Snapshot {
        version: 1,
        draft_name: d.name,
        scenes,
    };
    let json = serde_json::to_string(&snap).map_err(|e| AppError::internal(e.to_string()))?;
    let now = now_ms();
    c.execute(
        "INSERT INTO screenplay_history_point(id, draft_id, reason, scene_count, element_count, snapshot_json, created_by,
                                              created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![new_id(), draft_id, reason, snap.scenes.len() as i64, element_count as i64, json, tx.actor().user_id, now],
    )?;
    c.execute(
        "DELETE FROM screenplay_history_point WHERE draft_id=?1 AND id NOT IN
            (SELECT id FROM screenplay_history_point WHERE draft_id=?1 ORDER BY created_at DESC, id DESC LIMIT ?2)",
        params![draft_id, HISTORY_KEEP],
    )?;
    Ok(())
}

/// Whether an interval history point is due.
pub fn history_due(last_point_at: Option<i64>, now: i64) -> bool {
    last_point_at
        .map(|l| now - l >= HISTORY_INTERVAL_MS)
        .unwrap_or(true)
}

/// Record an interval history point after edits when due (FSD §21.1: automatic,
/// recoverable history; not undoable and not an activity entry). Failures are
/// logged and never fail the user's edit.
pub(crate) fn maybe_record_history(core: &AppCore, actor: &Actor, draft_id: &str) {
    let Ok(s) = core.project() else { return };
    let due = s
        .store
        .read(|c| {
            let last: Option<i64> = c.query_row(
                "SELECT max(created_at) FROM screenplay_history_point WHERE draft_id=?1",
                [draft_id],
                |r| r.get(0),
            )?;
            Ok(history_due(last, now_ms()))
        })
        .unwrap_or(false);
    if !due {
        return;
    }
    let meta = MutationMeta::new(
        "screenplay.history_point",
        "Automatic history point",
        Capability::Edit,
    )
    .not_undoable()
    .quiet();
    if let Err(e) = s.store.mutate(actor, meta, |tx| {
        // A locked draft never changes, so it needs no further points.
        if load_draft(tx.conn(), draft_id)?.status == DraftStatus::Locked {
            return Ok(());
        }
        record_history_point(tx, draft_id, "interval")
    }) {
        tracing::warn!(
            code = e.code_str(),
            "could not record an automatic history point"
        );
    }
}

// --------------------------------------------------------------------- args

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateScreenplayArgs {
    #[serde(default)]
    #[ts(optional)]
    pub title: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub episode_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub draft_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct CreatedScreenplay {
    pub screenplay: ScreenplayDto,
    pub draft: ScreenplayDraftDto,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayNewDraftArgs {
    pub source_draft_id: String,
    pub name: String,
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
    /// Defaults to true: the new draft becomes the Current draft.
    #[serde(default)]
    #[ts(optional)]
    pub make_current: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayRenameDraftArgs {
    pub draft_id: String,
    pub name: String,
    /// None keeps the note; an empty string clears it.
    #[serde(default)]
    #[ts(optional)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayDraftArgs {
    pub draft_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayLockSummary {
    pub draft_id: String,
    pub draft_name: String,
    pub status: DraftStatus,
    pub is_current: bool,
    #[ts(type = "number")]
    pub open_comments: i64,
    /// Up to five unresolved notes, e.g. "Scene 12: Should we establish…".
    pub open_comment_samples: Vec<String>,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub last_modified: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayStartRevisionArgs {
    /// The locked source draft.
    pub draft_id: String,
    pub label: String,
    #[serde(default)]
    #[ts(optional)]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayUpdateRevisionArgs {
    pub draft_id: String,
    #[serde(default)]
    #[ts(optional)]
    pub label: Option<String>,
    /// Empty string clears the colour.
    #[serde(default)]
    #[ts(optional)]
    pub color: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayHistoryPoint {
    pub id: String,
    pub draft_id: String,
    /// "interval" (automatic while editing) or "draft_created".
    pub reason: String,
    #[ts(type = "number")]
    pub scene_count: i64,
    #[ts(type = "number")]
    pub element_count: i64,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayHistoryPointArgs {
    pub history_point_id: String,
}

fn draft_result(core: &AppCore, id: &str) -> AppResult<ScreenplayDraftDto> {
    core.project()?
        .store
        .read(|c| draft_dto(c, &load_draft(c, id)?))
}

// ----------------------------------------------------------------- commands

/// "New Screenplay" (FSD §15.2, FSD-SCRIPT-001..004): an empty screenplay with
/// Draft 1, no Story Board needed. Format follows the project type.
fn create(core: &AppCore, actor: &Actor, a: CreateScreenplayArgs) -> AppResult<CreatedScreenplay> {
    let s = core.project()?;
    let draft_name = a.draft_name.unwrap_or_else(|| "Draft 1".into());
    let (sp, draft) = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.create",
            "Created the screenplay",
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let default_title: String = match &a.episode_id {
                Some(e) => c
                    .query_row("SELECT title FROM episode WHERE id=?1", [e], |r| r.get(0))
                    .optional()?
                    .unwrap_or_else(|| "Screenplay".into()),
                None => c.query_row("SELECT title FROM project LIMIT 1", [], |r| r.get(0))?,
            };
            let title = match &a.title {
                Some(t) => required_text(t, "Title", 200)?,
                None => default_title,
            };
            let sp = create_screenplay_tx(tx, a.episode_id.as_deref(), &title)?;
            let spec = DraftSpec {
                name: &draft_name,
                note: None,
                status: DraftStatus::Draft,
                created_from: None,
                revision: None,
                make_current: true,
            };
            let draft = insert_draft_tx(tx, &sp, spec, &[])?;
            Ok((sp, draft))
        },
    )?;
    s.store.read(|c| {
        Ok(CreatedScreenplay {
            screenplay: load_screenplay(c, &sp)?,
            draft: draft_dto(c, &load_draft(c, &draft)?)?,
        })
    })
}

/// "New Draft" (FSD §21.3): copy the source into a new named version; the source is unchanged.
fn new_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayNewDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let name = required_text(&a.name, "Draft name", MAX_DRAFT_NAME)?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.new_draft",
            format!("Created draft “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let src = load_draft(c, &a.source_draft_id)?;
            check_name_free(c, &src.screenplay_id, &name, None)?;
            let content = draft_content(c, &src.id)?;
            let spec = DraftSpec {
                name: &name,
                note: a.note.as_deref(),
                status: DraftStatus::Draft,
                created_from: Some(&src.id),
                revision: None,
                make_current: a.make_current.unwrap_or(true),
            };
            insert_draft_tx(tx, &src.screenplay_id, spec, &content)
        },
    )?;
    draft_result(core, &id)
}

/// Rename changes only the label, never the text (FSD §21.8).
fn rename_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayRenameDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let name = required_text(&a.name, "Draft name", MAX_DRAFT_NAME)?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.rename_draft",
            format!("Renamed a draft to “{name}”"),
            Capability::Edit,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            check_name_free(c, &d.screenplay_id, &name, Some(&d.id))?;
            let mut fields = vec![("name", text(name.clone()))];
            if let Some(n) = &a.note {
                fields.push((
                    "note",
                    opt_text(optional_text(Some(n.clone()), "Note", MAX_DRAFT_NOTE)?),
                ));
            }
            update_fields(
                c,
                "screenplay_draft",
                &d.id,
                &fields,
                &["name", "note"],
                None,
                "draft",
            )?;
            for sid in live_scene_ids(c, &d.id)? {
                tx.reindex("screenplay_scene", &sid);
            }
            Ok(())
        },
    )?;
    draft_result(core, &a.draft_id)
}

/// Exactly one draft is Current (FSD §21.5).
fn set_current_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let name = s.store.read(|c| Ok(load_draft(c, &a.draft_id)?.name))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.set_current_draft",
            format!("Made “{name}” the current draft"),
            Capability::Edit,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let d = load_draft(tx.conn(), &a.draft_id)?;
            set_current_tx(tx, &d.screenplay_id, &d.id)
        },
    )?;
    draft_result(core, &a.draft_id)
}

/// Restoring an old draft creates a new draft "Restored from …" (FSD §21.7).
fn restore_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.restore_draft",
            "Restored a draft as a new draft",
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let src = load_draft(c, &a.draft_id)?;
            let name = unique_name(
                c,
                &src.screenplay_id,
                &format!("Restored from {}", src.name),
            )?;
            let note = format!("Restored from {}", src.name);
            let content = draft_content(c, &src.id)?;
            let spec = DraftSpec {
                name: &name,
                note: Some(&note),
                status: DraftStatus::Draft,
                created_from: Some(&src.id),
                revision: None,
                make_current: true,
            };
            insert_draft_tx(tx, &src.screenplay_id, spec, &content)
        },
    )?;
    draft_result(core, &id)
}

/// Recoverable delete with safety rules (FSD §21.9, UX §3.15).
fn delete_draft(core: &AppCore, actor: &Actor, a: ScreenplayDraftArgs) -> AppResult<()> {
    let s = core.project()?;
    let name = s.store.read(|c| Ok(load_draft(c, &a.draft_id)?.name))?;
    s.store.mutate(
        actor,
        MutationMeta::new("screenplay.delete_draft", format!("Deleted draft “{name}”"), Capability::SoftDelete)
            .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            let current: Option<String> =
                c.query_row("SELECT current_draft_id FROM screenplay WHERE id=?1", [&d.screenplay_id], |r| r.get(0))?;
            if current.as_deref() == Some(d.id.as_str()) {
                return Err(AppError::conflict(
                    "This is the current draft. Make another draft current before deleting it.",
                ));
            }
            if d.status == DraftStatus::Locked {
                let others: i64 = c.query_row(
                    "SELECT count(*) FROM screenplay_draft WHERE screenplay_id=?1 AND status='Locked' AND deleted_at IS NULL AND id<>?2",
                    params![d.screenplay_id, d.id],
                    |r| r.get(0),
                )?;
                if others == 0 {
                    return Err(AppError::conflict(
                        "This locked draft is the only production baseline, so it can't be deleted.",
                    ));
                }
            }
            let used: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM production_source WHERE draft_id=?1 AND active=1)",
                [&d.id],
                |r| r.get(0),
            )?;
            if used {
                return Err(AppError::conflict(
                    "This draft is the production source. Choose a different production source before deleting it.",
                ));
            }
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "screenplay_draft",
                    table: "screenplay_draft",
                    id: &d.id,
                    title: Some(format!("Draft “{}”", d.name)),
                    parent_type: Some("screenplay"),
                    parent_id: Some(d.screenplay_id.clone()),
                    position: None,
                },
            )
        },
    )
}

/// Permanent purge of a draft and everything it exclusively owns.
fn purge_draft(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let id = &row.object_id;
    let used: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM production_source WHERE draft_id=?1)
             OR EXISTS(SELECT 1 FROM breakdown_element WHERE scene_id IN (SELECT id FROM screenplay_scene WHERE draft_id=?1))",
        [id],
        |r| r.get(0),
    )?;
    if used {
        return Err(AppError::conflict(
            "This draft is referenced by production planning, so it can't be permanently deleted. Restore it instead.",
        ));
    }
    let parent: Option<String> = c.query_row(
        "SELECT created_from_draft_id FROM screenplay_draft WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    // Keep later drafts' lineage readable: they now descend from this draft's parent.
    c.execute(
        "UPDATE screenplay_draft SET created_from_draft_id=?1, updated_at=?2, rev=rev+1 WHERE created_from_draft_id=?3",
        params![parent, now_ms(), id],
    )?;
    let mut stmt = c.prepare("SELECT id FROM screenplay_scene WHERE draft_id=?1")?;
    let scenes: Vec<String> = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut elements: Vec<String> = Vec::new();
    for sid in &scenes {
        let mut st = c.prepare("SELECT id FROM screenplay_element WHERE scene_id=?1")?;
        elements.extend(
            st.query_map([sid], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    comments::purge_for_targets(tx, "screenplay_element", &elements)?;
    comments::purge_for_targets(tx, "screenplay_scene", &scenes)?;
    comments::purge_for_targets(tx, "screenplay_draft", std::slice::from_ref(id))?;
    comments::purge_private_notes_for_targets(tx, "screenplay_element", &elements)?;
    comments::purge_private_notes_for_targets(tx, "screenplay_scene", &scenes)?;
    comments::purge_private_notes_for_targets(tx, "screenplay_draft", std::slice::from_ref(id))?;
    c.execute(
        "UPDATE comment SET review_round_id=NULL WHERE review_round_id IN (SELECT id FROM review_round WHERE draft_id=?1)",
        [id],
    )?;
    c.execute("DELETE FROM review_round WHERE draft_id=?1", [id])?;
    c.execute(
        "DELETE FROM screenplay_history_point WHERE draft_id=?1",
        [id],
    )?;
    for sid in &scenes {
        c.execute("DELETE FROM screenplay_element WHERE scene_id=?1", [sid])?;
        c.execute(
            "DELETE FROM deleted_item WHERE table_name='screenplay_scene' AND object_id=?1",
            [sid],
        )?;
    }
    c.execute("DELETE FROM screenplay_scene WHERE draft_id=?1", [id])?;
    c.execute("DELETE FROM screenplay_draft WHERE id=?1", [id])?;
    c.execute("DELETE FROM sys_screenplay_sync WHERE draft_id=?1", [id])?;
    Ok(())
}

/// Facts shown in the lock confirmation (FSD §24.2, UX §3.17).
fn lock_summary(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<ScreenplayLockSummary> {
    actor.require(Capability::View, "view the screenplay")?;
    core.project()?.store.read(|c| {
        let d = load_draft(c, &a.draft_id)?;
        let dto = draft_dto(c, &d)?;
        let mut stmt = c.prepare(
            "SELECT body, scene_id FROM comment WHERE parent_id IS NULL AND deleted_at IS NULL AND status IN ('Open','In Discussion')
               AND ((target_type='screenplay_draft' AND target_id=?1) OR scene_id IN (SELECT id FROM screenplay_scene WHERE draft_id=?1))
             ORDER BY created_at LIMIT 5",
        )?;
        let rows: Vec<(String, Option<String>)> = stmt.query_map([&d.id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        let mut samples = Vec::new();
        for (body, scene) in rows {
            let body: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
            let body = if body.chars().count() > 80 { format!("{}…", body.chars().take(80).collect::<String>()) } else { body };
            let n = match scene {
                Some(s) => super::scene_number(c, &s)?,
                None => None,
            };
            samples.push(match n {
                Some(n) => format!("Scene {n}: {body}"),
                None => body,
            });
        }
        Ok(ScreenplayLockSummary {
            draft_id: d.id.clone(),
            draft_name: d.name.clone(),
            status: d.status,
            is_current: dto.is_current,
            open_comments: dto.open_comments,
            open_comment_samples: samples,
            scene_count: dto.scene_count,
            last_modified: dto.last_modified,
        })
    })
}

/// "Lock as Shooting Draft" (FSD §24.1, §95): records time and user identity.
/// Resolving comments is not required.
fn lock_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let name = s.store.read(|c| Ok(load_draft(c, &a.draft_id)?.name))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.lock_draft",
            format!("Locked “{name}” as the Shooting Draft"),
            Capability::LockOrFinalize,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            if d.status == DraftStatus::Locked {
                return Err(AppError::conflict("This draft is already locked."));
            }
            update_fields(
                c,
                "screenplay_draft",
                &d.id,
                &[
                    ("status", text(DraftStatus::Locked.as_str())),
                    ("locked_at", opt_int(Some(now_ms()))),
                    ("locked_by", text(actor.user_id.clone())),
                    ("locked_by_name", text(actor.display_name.clone())),
                ],
                &["status", "locked_at", "locked_by", "locked_by_name"],
                None,
                "draft",
            )?;
            super::bump_seq(c, &d.id)?;
            Ok(())
        },
    )?;
    draft_result(core, &a.draft_id)
}

/// Owner-only: return a locked draft to normal editing (PRD: lock can be undone by authorized users).
fn unlock_draft(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let name = s.store.read(|c| Ok(load_draft(c, &a.draft_id)?.name))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.unlock_draft",
            format!("Unlocked “{name}”"),
            Capability::ManageProject,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            if d.status != DraftStatus::Locked {
                return Err(AppError::conflict("This draft is not locked."));
            }
            let back = if d.revision_label.is_some() {
                DraftStatus::Revision
            } else {
                DraftStatus::Draft
            };
            update_fields(
                c,
                "screenplay_draft",
                &d.id,
                &[
                    ("status", text(back.as_str())),
                    ("locked_at", opt_int(None)),
                    ("locked_by", opt_text(None::<String>)),
                    ("locked_by_name", opt_text(None::<String>)),
                ],
                &["status", "locked_at", "locked_by", "locked_by_name"],
                None,
                "draft",
            )?;
            super::bump_seq(c, &d.id)?;
            Ok(())
        },
    )?;
    draft_result(core, &a.draft_id)
}

/// "Start Revision" (FSD §24.4, §95): a new writable Revision draft derived from
/// the locked source; the locked source never changes.
fn start_revision(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayStartRevisionArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let label = required_text(&a.label, "Revision label", 80)?;
    let color = validate_color(a.color)?;
    let reason = optional_text(a.reason, "Reason", MAX_DRAFT_NOTE)?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.start_revision",
            format!("Started {label}"),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let src = load_draft(c, &a.draft_id)?;
            if src.status != DraftStatus::Locked {
                return Err(AppError::validation(
                    "status",
                    "Start Revision is used for a locked shooting draft.",
                ));
            }
            let name = unique_name(c, &src.screenplay_id, &label)?;
            let content = draft_content(c, &src.id)?;
            let spec = DraftSpec {
                name: &name,
                note: reason.as_deref(),
                status: DraftStatus::Revision,
                created_from: Some(&src.id),
                revision: Some((label.clone(), color.clone(), reason.clone())),
                make_current: true,
            };
            insert_draft_tx(tx, &src.screenplay_id, spec, &content)
        },
    )?;
    draft_result(core, &id)
}

fn update_revision(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayUpdateRevisionArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let label = a
        .label
        .map(|l| required_text(&l, "Revision label", 80))
        .transpose()?;
    let color = match a.color {
        Some(c) => Some(validate_color(Some(c))?),
        None => None,
    };
    let reason = match a.reason {
        Some(r) => Some(optional_text(Some(r), "Reason", MAX_DRAFT_NOTE)?),
        None => None,
    };
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.update_revision",
            "Changed revision details",
            Capability::Edit,
        )
        .target("screenplay_draft", &a.draft_id),
        |tx| {
            let c = tx.conn();
            let d = load_draft(c, &a.draft_id)?;
            if d.revision_label.is_none() && d.status != DraftStatus::Revision {
                return Err(AppError::validation(
                    "status",
                    "Only revisions have a revision label and colour.",
                ));
            }
            let mut fields = Vec::new();
            if let Some(l) = label.clone() {
                fields.push(("revision_label", text(l)));
            }
            if let Some(cl) = color.clone() {
                fields.push(("revision_color", opt_text(cl)));
            }
            if let Some(r) = reason.clone() {
                fields.push(("revision_reason", opt_text(r)));
            }
            update_fields(
                c,
                "screenplay_draft",
                &d.id,
                &fields,
                &["revision_label", "revision_color", "revision_reason"],
                None,
                "draft",
            )?;
            Ok(())
        },
    )?;
    draft_result(core, &a.draft_id)
}

fn history_points(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayDraftArgs,
) -> AppResult<Vec<ScreenplayHistoryPoint>> {
    actor.require(Capability::View, "view screenplay history")?;
    core.project()?.store.read(|c| {
        load_draft(c, &a.draft_id)?;
        let mut stmt = c.prepare(
            "SELECT id, draft_id, reason, scene_count, element_count, created_at FROM screenplay_history_point
             WHERE draft_id=?1 ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt
            .query_map([&a.draft_id], |r| {
                Ok(ScreenplayHistoryPoint {
                    id: r.get(0)?,
                    draft_id: r.get(1)?,
                    reason: r.get(2)?,
                    scene_count: r.get(3)?,
                    element_count: r.get(4)?,
                    created_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

/// Restoring an automatic point creates a new draft; nothing is replaced (FSD §21.2, §21.7).
fn restore_history_point(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayHistoryPointArgs,
) -> AppResult<ScreenplayDraftDto> {
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.restore_history_point",
            "Restored an automatic history point as a new draft",
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let (draft_id, json): (String, String) = c
                .query_row(
                    "SELECT draft_id, snapshot_json FROM screenplay_history_point WHERE id=?1",
                    [&a.history_point_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?
                .ok_or_else(|| AppError::not_found("history point"))?;
            let d: DraftRow = load_draft(c, &draft_id)?;
            let snap: Snapshot = serde_json::from_str(&json).map_err(|e| {
                AppError::new(
                    "project_format.invalid",
                    "This history point can't be read.",
                )
                .with_detail(e.to_string())
            })?;
            let name = unique_name(
                c,
                &d.screenplay_id,
                &format!("Restored from {} (automatic point)", snap.draft_name),
            )?;
            let note = format!(
                "Restored from an automatic history point of {}",
                snap.draft_name
            );
            let spec = DraftSpec {
                name: &name,
                note: Some(&note),
                status: DraftStatus::Draft,
                created_from: Some(&d.id),
                revision: None,
                make_current: true,
            };
            insert_draft_tx(tx, &d.screenplay_id, spec, &snap.scenes)
        },
    )?;
    draft_result(core, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_interval() {
        assert!(history_due(None, 1_000));
        assert!(!history_due(Some(1_000), 1_000 + HISTORY_INTERVAL_MS - 1));
        assert!(history_due(Some(1_000), 1_000 + HISTORY_INTERVAL_MS));
    }

    #[test]
    fn revision_colours() {
        assert_eq!(
            validate_color(Some("blue".into())).unwrap().as_deref(),
            Some("Blue")
        );
        assert_eq!(
            validate_color(Some("#A0b1C2".into())).unwrap().as_deref(),
            Some("#a0b1c2")
        );
        assert_eq!(validate_color(Some("  ".into())).unwrap(), None);
        assert!(validate_color(Some("sparkly".into())).is_err());
    }
}
