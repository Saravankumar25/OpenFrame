//! Screenplay interchange: import (preview → confirm) and export (FSD §19,
//! §20, §60, §117, §122–123; FSD-SCRIPT-038..050; Import/Export spec §4–§5).
//!
//! Operations:
//! - `screenplay.import_preview` (query, Import capability): parse a file or
//!   pasted text and return what *would* be created. Nothing is written.
//! - `screenplay.import_apply` (command): create a new screenplay or a new
//!   named draft from a preview. Never replaces an existing draft; the current
//!   draft is not changed when importing into an existing screenplay.
//!   Validation happens before the single mutation transaction, so a failure
//!   leaves the project untouched. Undoable like any other change.
//! - `screenplay.export` (query, Export capability): write PDF / FDX /
//!   Fountain / DOCX / TXT atomically to a user-chosen path. Exports are
//!   snapshots and never mutate the project.
//! - `interchange.export_preview` (query): the paginated print preview, built
//!   by the same layout engine as the PDF.
//! - `interchange.sources` / `interchange.draft_scenes` (queries): screenplays,
//!   drafts and scenes for the import/export dialogs.

mod draft;
mod export;
mod import;

use serde::Serialize;
use ts_rs::TS;

use crate::registry::Registry;

pub use export::{
    InterchangeDraftScenesArgs, InterchangeExportPreviewArgs, InterchangeExportSceneDto,
    InterchangePrintLine, InterchangePrintPage, InterchangePrintPreview,
    InterchangeScreenplayExportArgs, InterchangeScreenplayExportOptions,
    InterchangeScreenplayExportResult, InterchangeSourceDraft, InterchangeSourceScreenplay,
    InterchangeSources, InterchangeSourcesArgs,
};
pub use import::{
    InterchangeImportApplyArgs, InterchangeImportPreview, InterchangeImportPreviewArgs,
    InterchangeImportPreviewScene, InterchangeImportReport, InterchangeImportTargetScreenplay,
};

pub fn register(r: &mut Registry) {
    r.query("screenplay.import_preview", import::preview);
    r.command("screenplay.import_apply", import::apply);
    r.query("screenplay.export", export::export);
    r.query("interchange.export_preview", export::preview);
    r.query("interchange.sources", export::sources);
    r.query("interchange.draft_scenes", export::draft_scenes);
}

/// A parsing/export warning shown to the user.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct InterchangeWarning {
    pub code: String,
    pub message: String,
    /// "attention" (needs review) or "info".
    pub level: String,
    /// Index into the preview's scene list, when the warning concerns one scene.
    pub scene_index: Option<u32>,
}

impl From<&openframe_import_export::Warning> for InterchangeWarning {
    fn from(w: &openframe_import_export::Warning) -> Self {
        InterchangeWarning {
            code: w.code.clone(),
            message: w.message.clone(),
            level: match w.level {
                openframe_import_export::WarningLevel::Attention => "attention".into(),
                openframe_import_export::WarningLevel::Info => "info".into(),
            },
            scene_index: w.scene.map(|s| s as u32),
        }
    }
}
