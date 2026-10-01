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
    use crate::registry::{FsEffect as Fs, OperationMetadata as M, hidden as h};
    use openframe_domain::Capability as Cap;
    r.module("Import & Export");
    r.query("screenplay.import_preview", import::preview).meta(
        M::query(
            Cap::Import,
            "Preview importing a screenplay file (FDX/Fountain/…) or pasted text.",
        )
        .fs(Fs::ReadsUserFile)
        .hidden(h::USER_PATH),
    );
    r.command("screenplay.import_apply", import::apply).meta(
        M::command(
            Cap::Import,
            "Import a screenplay as a new screenplay or draft.",
        )
        .fs(Fs::ReadsUserFile)
        .hidden(h::USER_PATH),
    );
    r.query("screenplay.export", export::export).meta(
        M::query(
            Cap::Export,
            "Export a screenplay draft (PDF/FDX/Fountain/…) to a location the user picked.",
        )
        .fs(Fs::WritesUserFile)
        .hidden(h::USER_PATH),
    );
    r.query("interchange.export_preview", export::preview)
        .meta(M::compute("Print preview of a screenplay export.").hidden(h::UI_FLOW));
    r.query("interchange.sources", export::sources).meta(
        M::read("Screenplays and drafts that can be exported (dialog helper).").hidden(h::UI_FLOW),
    );
    r.query("interchange.draft_scenes", export::draft_scenes)
        .meta(
            M::read("Scenes of a draft for export selection (dialog helper).").hidden(h::UI_FLOW),
        );
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
