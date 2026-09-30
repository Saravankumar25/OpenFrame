//! Screenplay export and print preview. Exports read the selected draft and
//! write a snapshot file; the project is never mutated (Import/Export §3.1).

use std::path::PathBuf;

use openframe_domain::{Actor, AppError, AppResult, Capability};
use openframe_import_export::layout::{self, LineStyle};
use openframe_import_export::{ExportFormat, ExportOptions, ScreenplayDoc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::InterchangeWarning;
use super::draft::{self, DraftInfo};
use crate::core::AppCore;

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeScreenplayExportOptions {
    #[serde(default = "yes")]
    pub title_page: bool,
    #[serde(default)]
    pub scene_numbers: bool,
    /// Screenplay notes (never private notes).
    #[serde(default)]
    pub include_notes: bool,
    /// Revision-marked version: asterisks beside revised lines.
    #[serde(default)]
    pub revision_marks: bool,
    /// Revision line on the title page.
    #[serde(default = "yes")]
    pub revision_info: bool,
    /// Selected scenes only (None = entire draft). Numbers still follow the full draft order.
    #[serde(default)]
    pub scene_ids: Option<Vec<String>>,
}

fn yes() -> bool {
    true
}

impl Default for InterchangeScreenplayExportOptions {
    fn default() -> Self {
        Self {
            title_page: true,
            scene_numbers: false,
            include_notes: false,
            revision_marks: false,
            revision_info: true,
            scene_ids: None,
        }
    }
}

impl InterchangeScreenplayExportOptions {
    fn engine(&self) -> ExportOptions {
        ExportOptions {
            title_page: self.title_page,
            scene_numbers: self.scene_numbers,
            include_notes: self.include_notes,
            revision_marks: self.revision_marks,
            revision_info: self.revision_info,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeScreenplayExportArgs {
    pub draft_id: String,
    /// "pdf" | "fdx" | "fountain" | "docx" | "txt"
    pub format: String,
    /// Absolute destination chosen in the system save dialog.
    pub path: String,
    #[serde(default)]
    pub options: Option<InterchangeScreenplayExportOptions>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeScreenplayExportResult {
    pub path: String,
    pub file_name: String,
    pub format: String,
    #[ts(type = "number")]
    pub bytes_written: u64,
    pub pages: Option<u32>,
    pub scene_count: u32,
    pub source_label: String,
    pub warnings: Vec<InterchangeWarning>,
    pub private_notes_excluded: bool,
    pub summary: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeExportPreviewArgs {
    pub draft_id: String,
    #[serde(default)]
    pub options: Option<InterchangeScreenplayExportOptions>,
    /// Page numbers in the preview (the PDF always numbers from page 2).
    #[serde(default = "yes")]
    pub page_numbers: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangePrintLine {
    pub row: i32,
    /// Inches from the page's left edge.
    pub x_in: f32,
    /// Baseline inches from the page's top edge.
    pub y_in: f32,
    pub text: String,
    /// heading | action | character | parenthetical | dialogue | transition | centered | shot | lyric | note |
    /// more | page_number | scene_number | revision_mark | title_main | title_text
    pub style: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangePrintPage {
    pub title_page: bool,
    pub number: Option<u32>,
    pub lines: Vec<InterchangePrintLine>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangePrintPreview {
    pub source_label: String,
    pub page_width_in: f32,
    pub page_height_in: f32,
    pub pages: Vec<InterchangePrintPage>,
    pub script_page_count: u32,
    pub warnings: Vec<InterchangeWarning>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeSourcesArgs {}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeSourceDraft {
    pub id: String,
    pub name: String,
    pub status: String,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    pub scene_count: u32,
    pub is_current: bool,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeSourceScreenplay {
    pub id: String,
    pub title: String,
    pub current_draft_id: Option<String>,
    pub drafts: Vec<InterchangeSourceDraft>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeSources {
    pub screenplays: Vec<InterchangeSourceScreenplay>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterchangeDraftScenesArgs {
    pub draft_id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeExportSceneDto {
    pub id: String,
    /// Display number derived from order (as in the Screenplay workspace).
    pub number: u32,
    pub heading: String,
    pub omitted: bool,
}

// ------------------------------------------------------------------ helpers

fn source_label(info: &DraftInfo) -> String {
    format!("{} — {}", info.screenplay_title, info.draft_name)
}

fn load(
    c: &Connection,
    draft_id: &str,
    opts: &InterchangeScreenplayExportOptions,
) -> AppResult<(DraftInfo, ScreenplayDoc)> {
    crate::util::require_id(draft_id, "draft")?;
    let info = draft::load_draft_info(c, draft_id)?;
    let scenes = draft::load_scenes(c, draft_id)?;
    if let Some(ids) = &opts.scene_ids {
        if ids.is_empty() {
            return Err(AppError::validation(
                "scene_ids",
                "Select at least one scene to export.",
            ));
        }
        if let Some(missing) = ids.iter().find(|id| !scenes.iter().any(|s| &s.id == *id)) {
            return Err(
                AppError::not_found("scene").with_detail(format!("scene {missing} not in draft"))
            );
        }
    }
    let doc = draft::build_doc(
        &info,
        &scenes,
        opts.scene_ids.as_deref(),
        opts.scene_numbers,
    );
    Ok((info, doc))
}

fn validate_destination(core: &AppCore, path: &str, format: ExportFormat) -> AppResult<PathBuf> {
    let p = PathBuf::from(path.trim());
    if path.trim().is_empty() || !p.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose where to save the export with the Save dialog.",
        ));
    }
    let mut p = p;
    let ext_ok = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(format.extension()))
        .unwrap_or(false);
    if !ext_ok {
        let mut s = p.into_os_string();
        s.push(".");
        s.push(format.extension());
        p = PathBuf::from(s);
    }
    let parent = p
        .parent()
        .ok_or_else(|| AppError::invalid_input("Choose a folder to save the export in."))?;
    if !parent.is_dir() {
        return Err(AppError::export(
            "folder_missing",
            "The folder you chose no longer exists. Choose another location.",
        ));
    }
    if p.is_dir() {
        return Err(AppError::invalid_input("Choose a file name, not a folder."));
    }
    // Never write inside the open project's own folder (it could damage the project).
    if let Ok(session) = core.project() {
        let root = session.layout.root();
        if openframe_security::is_within(root, parent) {
            return Err(AppError::export(
                "inside_project",
                "Exports can't be saved inside the project's own folder. Choose another location, such as Documents.",
            ));
        }
    }
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if openframe_security::sanitize_file_name(name) != name {
        return Err(AppError::invalid_input(
            "That file name contains characters Windows doesn't allow.",
        ));
    }
    Ok(p)
}

fn style_str(s: LineStyle) -> &'static str {
    match s {
        LineStyle::Heading => "heading",
        LineStyle::Action => "action",
        LineStyle::Character => "character",
        LineStyle::Parenthetical => "parenthetical",
        LineStyle::Dialogue => "dialogue",
        LineStyle::Transition => "transition",
        LineStyle::Centered => "centered",
        LineStyle::Shot => "shot",
        LineStyle::Lyric => "lyric",
        LineStyle::Note => "note",
        LineStyle::More => "more",
        LineStyle::PageNumber => "page_number",
        LineStyle::SceneNumber => "scene_number",
        LineStyle::RevisionMark => "revision_mark",
        LineStyle::TitleMain => "title_main",
        LineStyle::TitleText => "title_text",
    }
}

/// Keep specific, actionable errors (export.*, full drive, no permission,
/// drive gone); anything else becomes a plain "could not be written".
fn export_error(e: AppError) -> AppError {
    if e.is("export")
        || e.is("storage.disk_full")
        || e.is("storage.access_denied")
        || e.is("storage.drive_unavailable")
    {
        return e;
    }
    let detail = e.detail.clone().unwrap_or_else(|| e.code_str().to_string());
    AppError::export(
        "write_failed",
        "The export file could not be written. Nothing in your project was changed.",
    )
    .with_detail(detail)
}

// ------------------------------------------------------------------ ops

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    args: InterchangeScreenplayExportArgs,
) -> AppResult<InterchangeScreenplayExportResult> {
    actor.require(Capability::Export, "export from this project")?;
    let format = ExportFormat::parse(&args.format)
        .ok_or_else(|| AppError::invalid_input("Choose PDF, Final Draft, Fountain or DOCX."))?;
    let opts = args.options.unwrap_or_default();
    let dest = validate_destination(core, &args.path, format)?;
    let session = core.project()?;
    // Read a consistent snapshot of the draft, then render outside the database lock.
    let (info, doc) = session.store.read(|c| load(c, &args.draft_id, &opts))?;
    if doc.scenes.is_empty() {
        return Err(AppError::export(
            "empty_draft",
            "This draft has no scenes to export yet.",
        ));
    }
    let out = openframe_import_export::export_to_path(&doc, format, &opts.engine(), &dest)
        .map_err(export_error)?;
    let file_name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let scene_count = doc.headed_scene_count() as u32;
    let summary = format!(
        "Exported {} of {} as {}. Private notes excluded. Export creates a snapshot; your project does not change.",
        if opts.scene_ids.is_some() {
            format!("{scene_count} selected scene(s)")
        } else {
            "the entire draft".to_string()
        },
        source_label(&info),
        format.label()
    );
    Ok(InterchangeScreenplayExportResult {
        path: dest.to_string_lossy().into_owned(),
        file_name,
        format: format.extension().into(),
        bytes_written: out.bytes_written,
        pages: out.pages.map(|p| p as u32),
        scene_count,
        source_label: source_label(&info),
        warnings: out.warnings.iter().map(InterchangeWarning::from).collect(),
        private_notes_excluded: true,
        summary,
    })
}

pub(super) fn preview(
    core: &AppCore,
    actor: &Actor,
    args: InterchangeExportPreviewArgs,
) -> AppResult<InterchangePrintPreview> {
    actor.require(Capability::View, "view this project")?;
    let opts = args.options.unwrap_or_default();
    let session = core.project()?;
    let (info, doc) = session.store.read(|c| load(c, &args.draft_id, &opts))?;
    let mut lopts = opts.engine().layout();
    lopts.page_numbers = args.page_numbers;
    let lay = layout::layout(&doc, &lopts);
    let mut warnings = Vec::new();
    let unsupported = openframe_import_export::unsupported_pdf_characters(&lay);
    if !unsupported.is_empty() {
        let sample: Vec<String> = unsupported
            .iter()
            .take(6)
            .map(|(c, _)| format!("“{c}”"))
            .collect();
        warnings.push(InterchangeWarning {
            code: "pdf_unsupported_characters".into(),
            message: format!(
                "This draft contains characters the standard screenplay PDF font (Courier) can't print, such as {}. PDF export is unavailable for it; DOCX, Final Draft and Fountain keep them.",
                sample.join(" ")
            ),
            level: "attention".into(),
            scene_index: None,
        });
    }
    Ok(InterchangePrintPreview {
        source_label: source_label(&info),
        page_width_in: layout::PAGE_WIDTH_IN,
        page_height_in: layout::PAGE_HEIGHT_IN,
        script_page_count: lay.script_page_count() as u32,
        pages: lay
            .pages
            .iter()
            .map(|p| InterchangePrintPage {
                title_page: p.title_page,
                number: p.number,
                lines: p
                    .lines
                    .iter()
                    .map(|l| InterchangePrintLine {
                        row: l.row,
                        x_in: l.x_in,
                        y_in: l.y_in(),
                        text: l.text.clone(),
                        style: style_str(l.style).into(),
                    })
                    .collect(),
            })
            .collect(),
        warnings,
    })
}

pub(super) fn sources(
    core: &AppCore,
    actor: &Actor,
    _: InterchangeSourcesArgs,
) -> AppResult<InterchangeSources> {
    actor.require(Capability::View, "view this project")?;
    let session = core.project()?;
    session.store.read(|c| {
        let mut s = c.prepare("SELECT id, title, current_draft_id FROM screenplay WHERE deleted_at IS NULL ORDER BY created_at, id")?;
        let screenplays: Vec<(String, String, Option<String>)> =
            s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        let mut d = c.prepare(
            "SELECT d.id, d.name, d.status, d.revision_label, d.revision_color, d.created_at,
                    (SELECT COUNT(*) FROM screenplay_scene sc WHERE sc.draft_id = d.id AND sc.deleted_at IS NULL)
             FROM screenplay_draft d WHERE d.screenplay_id = ?1 AND d.deleted_at IS NULL ORDER BY d.created_at, d.id",
        )?;
        let mut out = Vec::new();
        for (id, title, current) in screenplays {
            let drafts = d
                .query_map([&id], |r| {
                    let draft_id: String = r.get(0)?;
                    Ok(InterchangeSourceDraft {
                        is_current: current.as_deref() == Some(draft_id.as_str()),
                        id: draft_id,
                        name: r.get(1)?,
                        status: r.get(2)?,
                        revision_label: r.get(3)?,
                        revision_color: r.get(4)?,
                        created_at: r.get(5)?,
                        scene_count: r.get::<_, i64>(6)? as u32,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            out.push(InterchangeSourceScreenplay { id, title, current_draft_id: current, drafts });
        }
        Ok(InterchangeSources { screenplays: out })
    })
}

pub(super) fn draft_scenes(
    core: &AppCore,
    actor: &Actor,
    args: InterchangeDraftScenesArgs,
) -> AppResult<Vec<InterchangeExportSceneDto>> {
    actor.require(Capability::View, "view this project")?;
    crate::util::require_id(&args.draft_id, "draft")?;
    let session = core.project()?;
    session.store.read(|c| {
        draft::load_draft_info(c, &args.draft_id)?;
        let scenes = draft::load_scenes(c, &args.draft_id)?;
        Ok(scenes
            .into_iter()
            .map(|s| InterchangeExportSceneDto {
                id: s.id,
                number: s.number as u32,
                heading: s.heading,
                omitted: s.omitted,
            })
            .collect())
    })
}
