//! Screenplay import: preview (no mutation) → apply (one undoable transaction).

use std::path::{Path, PathBuf};

use openframe_domain::enums::DraftStatus;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_import_export::{
    ConfidenceLevel, ElementKind, ImportOutcome, ScreenplayDoc, SourceFormat, WarningLevel,
    heuristics,
};
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::InterchangeWarning;
use super::draft;
use crate::core::AppCore;
use crate::modules::files::add_file_record;
use crate::modules::screenplay::{self, drafts::DraftSpec};
use crate::store::MutationMeta;
use crate::util::{ingest_bytes, ingest_file};

/// How long an unconfirmed preview stays available.
const PREVIEW_TTL_MS: i64 = 60 * 60 * 1000;
const MAX_CACHED_PREVIEWS: usize = 6;
const MAX_PASTED_BYTES: usize = 5 << 20;

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeImportPreviewArgs {
    /// Absolute path of a file chosen in the system file dialog.
    #[serde(default)]
    pub path: Option<String>,
    /// Screenplay text pasted by the user.
    #[serde(default)]
    pub pasted_text: Option<String>,
    /// Optional hint: "fountain" or "txt" (binary formats are detected from content).
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeImportPreviewScene {
    pub index: u32,
    pub heading: String,
    /// Scene number printed in the source (display only).
    pub source_number: Option<String>,
    /// "ok" | "uncertain_heading" | "potentially_empty" | "no_heading"
    pub status: String,
    pub element_count: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeImportTargetScreenplay {
    pub id: String,
    pub title: String,
    /// Episode the screenplay belongs to (episodic projects).
    pub episode_title: Option<String>,
    pub draft_count: u32,
    pub current_draft_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeImportTargetEpisode {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeImportPreview {
    pub preview_id: String,
    pub source_name: String,
    /// "pdf" | "fdx" | "fountain" | "txt" | "docx" | "pasted"
    pub source_format: String,
    pub format_label: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub scene_count: u32,
    pub character_count: u32,
    pub characters: Vec<String>,
    pub approx_pages: u32,
    pub element_count: u32,
    pub dialogue_count: u32,
    pub scenes: Vec<InterchangeImportPreviewScene>,
    pub warnings: Vec<InterchangeWarning>,
    pub attention_count: u32,
    /// "high" | "medium" | "low"
    pub confidence: String,
    pub confidence_score: f32,
    /// True when the result must be reviewed before importing (low confidence or uncertain items).
    pub needs_review: bool,
    pub existing_screenplays: Vec<InterchangeImportTargetScreenplay>,
    /// Episodic/Series projects keep one screenplay per episode.
    pub episodic: bool,
    /// A new screenplay can be created: the project (or at least one episode)
    /// has none yet. Otherwise the script can only arrive as a new draft.
    pub can_create_screenplay: bool,
    /// Episodes without a screenplay (episodic projects; New Screenplay targets).
    pub episodes_without_screenplay: Vec<InterchangeImportTargetEpisode>,
    /// "new_screenplay" when no screenplay exists yet, else "new_draft".
    pub default_mode: String,
    pub can_keep_source: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeImportApplyArgs {
    /// A preview returned by `screenplay.import_preview` (preferred).
    #[serde(default)]
    pub preview_id: Option<String>,
    /// Re-parse arguments when no preview id is given.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub pasted_text: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    /// "new_screenplay" | "new_draft"
    pub mode: String,
    /// Target screenplay for "new_draft" (defaults to the only screenplay).
    #[serde(default)]
    pub screenplay_id: Option<String>,
    /// Episode for "new_screenplay" in episodic projects.
    #[serde(default)]
    pub episode_id: Option<String>,
    #[serde(default)]
    pub draft_name: Option<String>,
    #[serde(default)]
    pub keep_source_file: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeImportReport {
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub draft_id: String,
    pub draft_name: String,
    /// e.g. "Draft 2 — blackrain_v3 (imported)"
    pub draft_label: String,
    pub mode: String,
    pub source_name: String,
    pub source_format: String,
    pub imported_scenes: u32,
    pub imported_elements: u32,
    pub detected_characters: u32,
    pub approx_pages: u32,
    pub warnings_needing_attention: u32,
    pub warnings: Vec<InterchangeWarning>,
    pub kept_file_id: Option<String>,
    /// Human summary for the report banner.
    pub message: String,
}

// ------------------------------------------------------------------ cache

enum Source {
    File(PathBuf),
    Pasted(String),
}

struct CachedPreview {
    id: String,
    project_id: String,
    user_id: String,
    created_at: i64,
    source: Source,
    source_name: String,
    outcome: ImportOutcome,
}

#[derive(Default)]
pub(crate) struct PreviewCache {
    entries: Mutex<Vec<CachedPreview>>,
}

impl PreviewCache {
    fn put(&self, p: CachedPreview) {
        let mut e = self.entries.lock();
        let now = now_ms();
        e.retain(|x| now - x.created_at < PREVIEW_TTL_MS);
        while e.len() >= MAX_CACHED_PREVIEWS {
            e.remove(0);
        }
        e.push(p);
    }
    fn take(&self, id: &str, project_id: &str, user_id: &str) -> Option<CachedPreview> {
        let mut e = self.entries.lock();
        let now = now_ms();
        e.retain(|x| now - x.created_at < PREVIEW_TTL_MS);
        let pos = e
            .iter()
            .position(|x| x.id == id && x.project_id == project_id && x.user_id == user_id)?;
        Some(e.remove(pos))
    }
    fn put_back(&self, p: CachedPreview) {
        self.entries.lock().push(p);
    }
}

// ------------------------------------------------------------------ parsing

fn parse_source(
    args_path: Option<&str>,
    pasted: Option<&str>,
    format: Option<&str>,
) -> AppResult<(Source, String, ImportOutcome)> {
    let hint = match format.map(str::trim).filter(|f| !f.is_empty()) {
        Some(f) => Some(
            SourceFormat::parse(f)
                .ok_or_else(|| AppError::invalid_input("That import format isn't supported."))?,
        ),
        None => None,
    };
    match (args_path.map(str::trim).filter(|p| !p.is_empty()), pasted) {
        (Some(_), Some(t)) if !t.trim().is_empty() => Err(AppError::invalid_input(
            "Choose a file or paste text — not both.",
        )),
        (Some(p), _) => {
            let path = PathBuf::from(p);
            if !path.is_absolute() {
                return Err(AppError::invalid_input(
                    "Choose the file to import with the Browse… button.",
                ));
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("screenplay")
                .to_string();
            let outcome = openframe_import_export::import_file(&path, hint)?;
            Ok((Source::File(path), name, outcome))
        }
        (None, Some(t)) => {
            if t.trim().is_empty() {
                return Err(AppError::required("Screenplay text"));
            }
            if t.len() > MAX_PASTED_BYTES {
                return Err(AppError::import(
                    "too_large",
                    "That is too much text to paste at once. Save it as a file and import the file instead.",
                ));
            }
            let outcome =
                openframe_import_export::import_text(t, hint == Some(SourceFormat::Fountain))?;
            Ok((
                Source::Pasted(t.to_string()),
                "Pasted screenplay".to_string(),
                outcome,
            ))
        }
        (None, None) => Err(AppError::required("A file or pasted screenplay text")),
    }
}

fn prepare(mut outcome: ImportOutcome) -> ImportOutcome {
    let extra = draft::normalize_for_project(&mut outcome.doc);
    outcome.doc.warnings.extend(extra);
    outcome
}

fn scene_rows(doc: &ScreenplayDoc) -> Vec<InterchangeImportPreviewScene> {
    doc.scenes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let uncertain = doc
                .warnings
                .iter()
                .any(|w| w.code == "uncertain_heading" && w.scene == Some(i));
            let content = s
                .elements
                .iter()
                .filter(|e| {
                    !matches!(
                        e.kind,
                        ElementKind::Note
                            | ElementKind::Synopsis
                            | ElementKind::Section
                            | ElementKind::PageBreak
                    )
                })
                .count();
            let status = if s.heading.trim().is_empty() {
                "no_heading"
            } else if uncertain {
                "uncertain_heading"
            } else if content == 0 {
                "potentially_empty"
            } else {
                "ok"
            };
            InterchangeImportPreviewScene {
                index: i as u32,
                heading: s.heading.clone(),
                source_number: s.number.clone(),
                status: status.into(),
                element_count: s.elements.len() as u32,
            }
        })
        .collect()
}

struct ImportTargets {
    existing: Vec<InterchangeImportTargetScreenplay>,
    episodic: bool,
    episodes_without_screenplay: Vec<InterchangeImportTargetEpisode>,
    can_create_screenplay: bool,
}

fn import_targets(c: &Connection) -> AppResult<ImportTargets> {
    let mut stmt = c.prepare(
        "SELECT s.id, s.title, e.title,
                (SELECT COUNT(*) FROM screenplay_draft d WHERE d.screenplay_id = s.id AND d.deleted_at IS NULL),
                (SELECT d.name FROM screenplay_draft d WHERE d.id = s.current_draft_id AND d.deleted_at IS NULL)
         FROM screenplay s LEFT JOIN episode e ON e.id = s.episode_id
         WHERE s.deleted_at IS NULL ORDER BY s.created_at, s.id",
    )?;
    let existing = stmt
        .query_map([], |r| {
            Ok(InterchangeImportTargetScreenplay {
                id: r.get(0)?,
                title: r.get(1)?,
                episode_title: r.get(2)?,
                draft_count: r.get::<_, i64>(3)? as u32,
                current_draft_name: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let episodic = screenplay::project_type(c)?.is_episodic();
    let episodes_without_screenplay = if episodic {
        let mut e = c.prepare(
            "SELECT e.id, e.title FROM episode e LEFT JOIN season se ON se.id = e.season_id
             WHERE e.deleted_at IS NULL
               AND NOT EXISTS (SELECT 1 FROM screenplay s WHERE s.episode_id = e.id AND s.deleted_at IS NULL)
             ORDER BY se.position, e.position, e.id",
        )?;
        e.query_map([], |r| {
            Ok(InterchangeImportTargetEpisode {
                id: r.get(0)?,
                title: r.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let can_create_screenplay = if episodic {
        !episodes_without_screenplay.is_empty()
    } else {
        screenplay::screenplay_for_scope(c, None)?.is_none()
    };
    Ok(ImportTargets {
        existing,
        episodic,
        episodes_without_screenplay,
        can_create_screenplay,
    })
}

fn confidence_str(level: ConfidenceLevel) -> &'static str {
    match level {
        ConfidenceLevel::High => "high",
        ConfidenceLevel::Medium => "medium",
        ConfidenceLevel::Low => "low",
    }
}

fn build_preview(
    id: &str,
    source_name: &str,
    outcome: &ImportOutcome,
    targets: ImportTargets,
) -> InterchangeImportPreview {
    let doc = &outcome.doc;
    let characters = doc.characters();
    let scenes = scene_rows(doc);
    let attention = doc
        .warnings
        .iter()
        .filter(|w| w.level == WarningLevel::Attention)
        .count() as u32;
    let needs_review =
        outcome.confidence.level == ConfidenceLevel::Low || scenes.iter().any(|s| s.status != "ok");
    InterchangeImportPreview {
        preview_id: id.to_string(),
        source_name: source_name.to_string(),
        source_format: outcome.format.as_str().to_string(),
        format_label: outcome.format.label().to_string(),
        title: doc.title_page.title().map(|t| t.replace('\n', " ")),
        author: doc.title_page.author().map(|a| a.replace('\n', ", ")),
        scene_count: doc.headed_scene_count() as u32,
        character_count: characters.len() as u32,
        characters,
        approx_pages: heuristics::estimate_pages(doc),
        element_count: doc.element_count() as u32,
        dialogue_count: doc
            .scenes
            .iter()
            .flat_map(|s| s.elements.iter())
            .filter(|e| e.kind == ElementKind::Dialogue)
            .count() as u32,
        scenes,
        warnings: doc.warnings.iter().map(InterchangeWarning::from).collect(),
        attention_count: attention,
        confidence: confidence_str(outcome.confidence.level).into(),
        confidence_score: (outcome.confidence.score * 100.0).round() / 100.0,
        needs_review,
        default_mode: if targets.can_create_screenplay
            && (targets.existing.is_empty() || targets.episodic)
        {
            "new_screenplay".into()
        } else {
            "new_draft".into()
        },
        existing_screenplays: targets.existing,
        episodic: targets.episodic,
        can_create_screenplay: targets.can_create_screenplay,
        episodes_without_screenplay: targets.episodes_without_screenplay,
        can_keep_source: true,
    }
}

// ------------------------------------------------------------------ ops

pub(super) fn preview(
    core: &AppCore,
    actor: &Actor,
    args: InterchangeImportPreviewArgs,
) -> AppResult<InterchangeImportPreview> {
    actor.require(Capability::Import, "import into this project")?;
    let session = core.project()?;
    let (source, source_name, outcome) = parse_source(
        args.path.as_deref(),
        args.pasted_text.as_deref(),
        args.format.as_deref(),
    )?;
    let outcome = prepare(outcome);
    let targets = session.store.read(import_targets)?;
    let id = new_id();
    let preview = build_preview(&id, &source_name, &outcome, targets);
    core.service(PreviewCache::default).put(CachedPreview {
        id,
        project_id: session.project_id(),
        user_id: actor.user_id.clone(),
        created_at: now_ms(),
        source,
        source_name,
        outcome,
    });
    Ok(preview)
}

fn stem(name: &str) -> String {
    let s = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .trim()
        .to_string();
    if s.is_empty() {
        "Imported screenplay".into()
    } else {
        s
    }
}

fn unique_draft_name(c: &Connection, screenplay_id: Option<&str>, base: &str) -> AppResult<String> {
    let Some(sid) = screenplay_id else {
        return Ok(base.to_string());
    };
    let mut name = base.to_string();
    let mut n = 2;
    loop {
        let taken: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM screenplay_draft WHERE screenplay_id=?1 AND lower(name)=lower(?2) AND deleted_at IS NULL)",
            params![sid, name],
            |r| r.get(0),
        )?;
        if !taken {
            return Ok(name);
        }
        name = format!("{base} {n}");
        n += 1;
    }
}

/// Where the imported draft goes, resolved and validated before any write.
struct Destination {
    /// Existing screenplay receiving a new draft (None = create a screenplay).
    target: Option<InterchangeImportTargetScreenplay>,
    episode_id: Option<String>,
    draft_name: String,
}

fn resolve_destination(
    c: &Connection,
    new_screenplay: bool,
    args: &InterchangeImportApplyArgs,
    draft_name_arg: Option<String>,
    source_name: &str,
) -> AppResult<Destination> {
    let targets = import_targets(c)?;
    let (target, episode_id) = if new_screenplay {
        if !targets.can_create_screenplay {
            return Err(AppError::validation(
                "mode",
                "This project already has a screenplay. Import the script as a new draft instead — nothing is overwritten.",
            ));
        }
        let episode_id = if targets.episodic {
            let wanted = args.episode_id.as_deref().ok_or_else(|| {
                AppError::validation(
                    "episode_id",
                    "Choose the episode this screenplay belongs to.",
                )
            })?;
            if !targets
                .episodes_without_screenplay
                .iter()
                .any(|e| e.id == wanted)
            {
                return Err(AppError::validation(
                    "episode_id",
                    "That episode already has a screenplay. Import the script as a new draft of it instead.",
                ));
            }
            Some(wanted.to_string())
        } else {
            None
        };
        (None, episode_id)
    } else {
        let existing = &targets.existing;
        let target = match args.screenplay_id.as_deref() {
            Some(id) => existing
                .iter()
                .find(|s| s.id == id)
                .cloned()
                .ok_or_else(|| AppError::not_found("screenplay"))?,
            None => match existing.len() {
                0 => {
                    return Err(AppError::validation(
                        "mode",
                        "This project has no screenplay yet. Import it as a New Screenplay instead.",
                    ));
                }
                1 => existing[0].clone(),
                _ => {
                    return Err(AppError::validation(
                        "screenplay_id",
                        "Choose which screenplay should receive the new draft.",
                    ));
                }
            },
        };
        (Some(target), None)
    };
    let base = draft_name_arg.unwrap_or_else(|| format!("{} (imported)", stem(source_name)));
    let draft_name = unique_draft_name(c, target.as_ref().map(|t| t.id.as_str()), &base)?;
    Ok(Destination {
        target,
        episode_id,
        draft_name,
    })
}

pub(super) fn apply(
    core: &AppCore,
    actor: &Actor,
    args: InterchangeImportApplyArgs,
) -> AppResult<InterchangeImportReport> {
    actor.require(Capability::Import, "import into this project")?;
    let session = core.project()?;
    let project_id = session.project_id();
    let new_screenplay = match args.mode.as_str() {
        "new_screenplay" => true,
        "new_draft" => false,
        _ => {
            return Err(AppError::invalid_input(
                "Choose New Screenplay or New Draft.",
            ));
        }
    };
    let draft_name_arg = crate::util::optional_text(args.draft_name.clone(), "Draft name", 120)?;

    // 1. Resolve the parsed document (validation before any mutation).
    let cache = core.service(PreviewCache::default);
    let cached = match args.preview_id.as_deref() {
        Some(id) => Some(cache.take(id, &project_id, &actor.user_id).ok_or_else(|| {
            AppError::import(
                "preview_expired",
                "This import preview is no longer available. Choose the file again to preview it. Your project was not changed.",
            )
        })?),
        None => None,
    };
    let (source, source_name, outcome) = match cached {
        Some(c) => (c.source, c.source_name, c.outcome),
        None => {
            let (s, n, o) = parse_source(
                args.path.as_deref(),
                args.pasted_text.as_deref(),
                args.format.as_deref(),
            )?;
            (s, n, prepare(o))
        }
    };
    // A failed apply keeps the preview so the user can correct the choice and retry.
    let restore = |source: Source, source_name: String, outcome: ImportOutcome| {
        if let Some(id) = args.preview_id.clone() {
            cache.put_back(CachedPreview {
                id,
                project_id: project_id.clone(),
                user_id: actor.user_id.clone(),
                created_at: now_ms(),
                source,
                source_name,
                outcome,
            });
        }
    };
    if outcome
        .doc
        .scenes
        .iter()
        .all(|s| s.heading.trim().is_empty() && s.elements.is_empty())
    {
        return Err(AppError::import(
            "empty_source",
            "Nothing to import was found. Your current project was not changed.",
        ));
    }

    // 2. Resolve the destination.
    let dest = match session.store.read(|c| {
        resolve_destination(
            c,
            new_screenplay,
            &args,
            draft_name_arg.clone(),
            &source_name,
        )
    }) {
        Ok(d) => d,
        Err(e) => {
            restore(source, source_name, outcome);
            return Err(e);
        }
    };
    if args.keep_source_file
        && let Source::File(p) = &source
        && !p.is_file()
    {
        restore(source, source_name, outcome);
        return Err(AppError::import(
            "source_missing",
            "The original file is no longer available, so it can't be kept. Your project was not changed.",
        ));
    }

    // 3. One transaction: screenplay (when new) + draft with scenes/elements
    //    (through the Screenplay module, so history/search stay consistent)
    //    + the kept source file. Undo removes all of it in one step.
    let doc = &outcome.doc;
    let content = draft::to_new_scenes(doc);
    let screenplay_title: String = match &dest.target {
        Some(t) => t.title.clone(),
        None => doc
            .title_page
            .title()
            .map(|t| t.replace('\n', " ").trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| stem(&source_name)),
    }
    .chars()
    .take(200)
    .collect();
    let draft_name = dest.draft_name.clone();
    let summary = if new_screenplay {
        format!("Imported screenplay “{screenplay_title}”")
    } else {
        format!("Imported “{draft_name}” as a new draft")
    };
    let mut meta = MutationMeta::new("screenplay.import_apply", summary, Capability::Import);
    if let Some(t) = &dest.target {
        meta = meta.target("screenplay", &t.id);
    }
    let note = format!("Imported from {source_name} ({})", outcome.format.label());
    let result = session.store.mutate(actor, meta, |tx| {
        let screenplay_id = match &dest.target {
            Some(t) => t.id.clone(),
            None => {
                let id = screenplay::create_screenplay_tx(
                    tx,
                    dest.episode_id.as_deref(),
                    &screenplay_title,
                )?;
                let c = tx.conn();
                let stored: String = c.query_row(
                    "SELECT title_page_json FROM screenplay WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )?;
                c.execute(
                    "UPDATE screenplay SET title_page_json=?1 WHERE id=?2",
                    params![draft::merge_title_page(&stored, &doc.title_page), id],
                )?;
                id
            }
        };
        let draft_id = screenplay::insert_draft_tx(
            tx,
            &screenplay_id,
            DraftSpec {
                name: &draft_name,
                note: Some(&note),
                status: DraftStatus::Draft,
                created_from: None,
                revision: None,
                // A new screenplay's only draft is current; importing into an
                // existing screenplay never replaces its current draft (FSD-SCRIPT-044).
                make_current: dest.target.is_none(),
            },
            &content.scenes,
        )?;
        draft::apply_revisions(tx, &draft_id, &content.revisions)?;
        let kept = if args.keep_source_file {
            let asset = match &source {
                Source::File(p) => ingest_file(tx, p)?,
                Source::Pasted(t) => ingest_bytes(
                    tx,
                    t.as_bytes(),
                    "Pasted screenplay.txt",
                    Some("text/plain"),
                )?,
            };
            Some(add_file_record(
                tx,
                &asset,
                None,
                Some("Screenplay import"),
            )?)
        } else {
            None
        };
        let ordinal: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM screenplay_draft WHERE screenplay_id=?1 AND deleted_at IS NULL",
            [&screenplay_id],
            |r| r.get(0),
        )?;
        Ok((screenplay_id, draft_id, kept, ordinal))
    });
    let (screenplay_id, draft_id, kept, ordinal) = match result {
        Ok(v) => v,
        Err(e) => {
            restore(source, source_name, outcome);
            return Err(e);
        }
    };

    let attention = doc
        .warnings
        .iter()
        .filter(|w| w.level == WarningLevel::Attention)
        .count() as u32;
    let draft_label = format!("Draft {ordinal} — {draft_name}");
    let message = if new_screenplay {
        format!("Imported as a new screenplay, “{screenplay_title}” ({draft_label}).")
    } else {
        format!("Imported as {draft_label}. Your other drafts were not changed.")
    };
    Ok(InterchangeImportReport {
        screenplay_id,
        screenplay_title,
        draft_id,
        draft_name,
        draft_label,
        mode: args.mode.clone(),
        source_name,
        source_format: outcome.format.as_str().into(),
        imported_scenes: doc.headed_scene_count() as u32,
        imported_elements: content.element_count as u32,
        detected_characters: doc.characters().len() as u32,
        approx_pages: heuristics::estimate_pages(doc),
        warnings_needing_attention: attention,
        warnings: doc.warnings.iter().map(InterchangeWarning::from).collect(),
        kept_file_id: kept,
        message,
    })
}
