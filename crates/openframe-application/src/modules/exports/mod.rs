//! Document exports for the planning and production workspaces (FSD §20.7–20.8,
//! §31.5, §32.7, §33.6, §38.9, §58, §60, §109.6, §110–111, §122–123;
//! Import/Export spec §3 and §6).
//!
//! | op                         | document                                  | formats         |
//! |----------------------------|-------------------------------------------|-----------------|
//! | `story.export_board`       | Story Board outline / board               | PDF · TXT · CSV |
//! | `breakdown.export_report`  | Breakdown by scene + Catalog with usage   | PDF · CSV · XLSX|
//! | `moodboard.export_pdf`     | Moodboard presentation                    | PDF             |
//! | `storyboard.export_sheet`  | Storyboard sheet (panel grid)             | PDF             |
//! | `shot.export_list`         | Shot List (scene / scenes / project)      | PDF · CSV · XLSX|
//! | `schedule.export_schedule` | Shooting Schedule / stripboard            | PDF · CSV · XLSX|
//! | `callsheets.export_pdf`    | Call Sheet (paper layout)                 | PDF             |
//! | `sides.export_pdf`         | Sides (screenplay-formatted excerpt)      | PDF             |
//! | `reports.export_report`    | Production reports (live or saved)        | PDF · CSV · XLSX|
//! | `budget.export_summary`    | Budget summary                            | PDF · CSV · XLSX|
//!
//! Rules shared by every export:
//! - Capability `Export` (Owner, Editor, Export-only; never Viewer/Commenter).
//! - Exports are snapshots: other modules' tables are only *read* (one read
//!   pass), the document is rendered in memory and written atomically
//!   (temporary file + rename) to the path chosen in the Save dialog. A
//!   failure never leaves a partial file and never changes the project.
//! - Private notes are never read. Internal notes (moodboard internal notes,
//!   panel notes, story notes and comments) are included only when the user
//!   explicitly selects them. Every result says "Private notes excluded."
//! - PDFs use the standard base-14 fonts; text they can't print is a clear
//!   `export.pdf_unsupported_characters` error — never a silent "?".
//! - One activity entry per export (`sys_activity`, not undoable: an export
//!   changes nothing in the project).

mod breakdown;
mod budget;
mod callsheet;
mod reports;
mod schedule;
mod sides;
mod story;
mod visual;

use std::path::{Path, PathBuf};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_import_export::report::{Document, ReportImage, Table, tables_to_xlsx};
use rusqlite::params;
use serde::Serialize;
use ts_rs::TS;

use crate::core::AppCore;
use crate::events::{AppEvent, StoreKind};
use crate::registry::Registry;
use crate::util::AssetInfo;

pub use breakdown::ExportBreakdownArgs;
pub use budget::ExportBudgetArgs;
pub use callsheet::ExportCallSheetArgs;
pub use reports::ExportReportArgs;
pub use schedule::ExportScheduleArgs;
pub use sides::ExportSidesArgs;
pub use story::ExportStoryOutlineArgs;
pub use visual::{ExportMoodboardArgs, ExportShotListArgs, ExportStoryboardSheetArgs};

pub fn register(r: &mut Registry) {
    r.query("story.export_board", story::export);
    r.query("breakdown.export_report", breakdown::export);
    r.query("moodboard.export_pdf", visual::export_moodboard);
    r.query("storyboard.export_sheet", visual::export_storyboards);
    r.query("shot.export_list", visual::export_shots);
    r.query("schedule.export_schedule", schedule::export);
    r.query("callsheets.export_pdf", callsheet::export);
    r.query("sides.export_pdf", sides::export);
    r.query("reports.export_report", reports::export);
    r.query("budget.export_summary", budget::export);
}

/// What every export returns: where the snapshot was written and what it holds.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub file_name: String,
    /// "pdf" | "csv" | "xlsx" | "txt"
    pub format: String,
    #[ts(type = "number")]
    pub bytes_written: u64,
    /// Pages of a PDF.
    pub pages: Option<u32>,
    /// "Shooting Schedule", "Call Sheet — Day 4", …
    pub document: String,
    /// Source/version shown to the user ("Shooting Draft 6 (Locked)").
    pub source_label: String,
    /// "Entire schedule", "2 selected scenes", …
    pub scope_label: String,
    /// "12 scenes", "8 panels", …
    pub contents_label: String,
    /// Non-blocking notes (missing images, draft call sheet, …).
    pub warnings: Vec<String>,
    pub private_notes_excluded: bool,
    pub summary: String,
}

// ------------------------------------------------------------------ formats

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fmt {
    Pdf,
    Csv,
    Xlsx,
    Txt,
}

impl Fmt {
    pub fn parse(s: &str, allowed: &[Fmt]) -> AppResult<Fmt> {
        let f = match s.trim().to_ascii_lowercase().as_str() {
            "pdf" => Some(Fmt::Pdf),
            "csv" => Some(Fmt::Csv),
            "xlsx" | "excel" => Some(Fmt::Xlsx),
            "txt" | "text" => Some(Fmt::Txt),
            _ => None,
        };
        match f {
            Some(f) if allowed.contains(&f) => Ok(f),
            _ => {
                let names: Vec<&str> = allowed.iter().map(|f| f.label()).collect();
                Err(AppError::invalid_input(format!(
                    "Choose one of the available formats: {}.",
                    names.join(", ")
                )))
            }
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Fmt::Pdf => "pdf",
            Fmt::Csv => "csv",
            Fmt::Xlsx => "xlsx",
            Fmt::Txt => "txt",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Fmt::Pdf => "PDF",
            Fmt::Csv => "CSV",
            Fmt::Xlsx => "Excel (XLSX)",
            Fmt::Txt => "Plain text",
        }
    }
}

/// A document ready to be written in any of its formats.
pub(crate) struct Output {
    /// PDF rendering.
    pub doc: Document,
    /// CSV writes the first table; XLSX writes all (one sheet each).
    pub tables: Vec<Table>,
    /// Plain-text rendering, when the export offers TXT.
    pub text: Option<String>,
    /// What to suggest when the PDF font can't print some characters.
    pub pdf_hint: &'static str,
    /// A PDF rendered by another engine (Sides use the screenplay layout):
    /// bytes and page count. Takes precedence over `doc`.
    pub pdf: Option<(Vec<u8>, u32)>,
    pub warnings: Vec<String>,
}

impl Output {
    pub fn new(doc: Document, pdf_hint: &'static str) -> Output {
        Output {
            doc,
            tables: Vec::new(),
            text: None,
            pdf_hint,
            pdf: None,
            warnings: Vec::new(),
        }
    }
}

/// Human description of the export, for the result and the activity entry.
pub(crate) struct Job {
    pub action: &'static str,
    pub document: String,
    pub source_label: String,
    pub scope_label: String,
    pub contents_label: String,
    pub target: Option<(&'static str, String)>,
}

pub(crate) const HINT_TABLES: &str = "Export it as CSV or Excel (XLSX) to keep them, or replace those characters and export the PDF again.";
pub(crate) const HINT_TEXT: &str = "Export it as plain text or CSV to keep them, or replace those characters and export the PDF again.";
pub(crate) const HINT_EDIT: &str =
    "Replace those characters in the project (or remove them) and export the PDF again.";

// ------------------------------------------------------------------ pipeline

/// Permission gate for every export (before anything is read).
pub(crate) fn require_export(actor: &Actor) -> AppResult<()> {
    actor.require(Capability::Export, "export from this project")
}

/// Validate the destination chosen in the Save dialog: absolute, existing
/// folder, a file name Windows accepts, the format's extension, and never
/// inside the open project's own folder.
pub(crate) fn destination(core: &AppCore, path: &str, fmt: Fmt) -> AppResult<PathBuf> {
    let raw = path.trim();
    let p = PathBuf::from(raw);
    if raw.is_empty() || !p.is_absolute() {
        return Err(AppError::invalid_input(
            "Choose where to save the export with the Save dialog.",
        ));
    }
    let mut p = p;
    let ext_ok = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(fmt.ext()))
        .unwrap_or(false);
    if !ext_ok {
        let mut s = p.into_os_string();
        s.push(".");
        s.push(fmt.ext());
        p = PathBuf::from(s);
    }
    let parent = p
        .parent()
        .filter(|x| !x.as_os_str().is_empty())
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
    if let Ok(session) = core.project()
        && openframe_security::is_within(session.layout.root(), parent)
    {
        return Err(AppError::export(
            "inside_project",
            "Exports can't be saved inside the project's own folder. Choose another location, such as Documents.",
        ));
    }
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.is_empty() || openframe_security::sanitize_file_name(name) != name {
        return Err(AppError::invalid_input(
            "That file name contains characters Windows doesn't allow.",
        ));
    }
    // Device/ADS/reserved paths, app data and the Global Idea Vault (PATH-03/PATH-04).
    crate::util::check_output_file(core, &p)?;
    Ok(p)
}

/// Keep specific, actionable errors; anything else becomes a plain
/// "could not be written" (the project is never changed by an export).
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

/// Render `out` in `fmt`, write it atomically to `dest`, record the activity
/// entry and describe the result.
pub(crate) fn deliver(
    core: &AppCore,
    actor: &Actor,
    dest: &Path,
    fmt: Fmt,
    job: Job,
    out: Output,
) -> AppResult<ExportResult> {
    let (bytes, pages) = match fmt {
        Fmt::Pdf => match out.pdf {
            Some((bytes, pages)) => (bytes, Some(pages)),
            None => {
                let pdf = out.doc.to_pdf_strict(out.pdf_hint)?;
                (pdf.bytes, Some(pdf.pages as u32))
            }
        },
        Fmt::Csv => {
            let t = out.tables.first().ok_or_else(|| {
                AppError::invalid_input("This document can't be exported as CSV.")
            })?;
            (t.to_csv()?, None)
        }
        Fmt::Xlsx => {
            if out.tables.is_empty() {
                return Err(AppError::invalid_input(
                    "This document can't be exported as a spreadsheet.",
                ));
            }
            let refs: Vec<&Table> = out.tables.iter().collect();
            (tables_to_xlsx(&refs)?, None)
        }
        Fmt::Txt => {
            let t = out.text.as_ref().ok_or_else(|| {
                AppError::invalid_input("This document can't be exported as plain text.")
            })?;
            let mut b = Vec::with_capacity(t.len() + 3);
            b.extend_from_slice(b"\xEF\xBB\xBF");
            b.extend_from_slice(t.replace('\n', "\r\n").as_bytes());
            (b, None)
        }
    };
    openframe_import_export::write_atomic(dest, &bytes).map_err(export_error)?;
    let file_name = dest
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let summary = format!(
        "Exported {} ({}) as {}. Private notes excluded. Export creates a snapshot; your project does not change.",
        job.document,
        job.scope_label.to_lowercase(),
        fmt.label()
    );
    record_activity(core, actor, &job, fmt);
    Ok(ExportResult {
        path: dest.to_string_lossy().into_owned(),
        file_name,
        format: fmt.ext().into(),
        bytes_written: bytes.len() as u64,
        pages,
        document: job.document,
        source_label: job.source_label,
        scope_label: job.scope_label,
        contents_label: job.contents_label,
        warnings: out.warnings,
        private_notes_excluded: true,
        summary,
    })
}

/// "Exported Shooting Schedule as PDF" in Activity. Infrastructure row (not
/// undoable); a failure to log never fails an export that was written.
fn record_activity(core: &AppCore, actor: &Actor, job: &Job, fmt: Fmt) {
    let Ok(session) = core.project() else { return };
    let origin = match &actor.origin {
        openframe_domain::auth::ActorOrigin::Local => "local",
        openframe_domain::auth::ActorOrigin::Exchange { .. } => "exchange",
        openframe_domain::auth::ActorOrigin::Ai { .. } => "ai",
    };
    let (tt, tid) = match &job.target {
        Some((t, id)) => (Some(t.to_string()), Some(id.clone())),
        None => (None, None),
    };
    let summary = format!("Exported {} as {}", job.document, fmt.label());
    let res = session.store.with_writer(|c| {
        c.execute(
            "INSERT INTO sys_activity(id, at, actor_id, actor_name, action, summary, target_type, target_id, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![new_id(), now_ms(), actor.user_id, actor.display_name, job.action, summary, tt, tid, origin],
        )?;
        Ok(())
    });
    match res {
        Ok(()) => core.emit(AppEvent::DataChanged {
            store: StoreKind::Project,
            tables: vec!["sys_activity".into()],
            ids: Vec::new(),
            origin: "local".into(),
        }),
        Err(e) => tracing::warn!(code = e.code_str(), "could not record export activity"),
    }
}

// ------------------------------------------------------------------ helpers

/// "30 Sep 2026" (local date; UTC when the local offset is unavailable).
pub(crate) fn today_label() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {} {}",
        now.day(),
        months[(now.month() as usize).saturating_sub(1).min(11)],
        now.year()
    )
}

/// "30 Sep 2026" for a stored timestamp (milliseconds, local date).
pub(crate) fn date_from_ms(ms: i64) -> String {
    let utc = time::OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    let local = time::UtcOffset::current_local_offset()
        .map(|o| utc.to_offset(o))
        .unwrap_or(utc);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{} {} {}",
        local.day(),
        months[(local.month() as usize).saturating_sub(1).min(11)],
        local.year()
    )
}

/// "BLACK RAIN · Exported 30 Sep 2026 · Source: …".
pub(crate) fn subtitle(project: &str, source: Option<&str>) -> String {
    let mut parts = Vec::new();
    if !project.trim().is_empty() {
        parts.push(project.trim().to_string());
    }
    parts.push(format!("Exported {}", today_label()));
    if let Some(s) = source.filter(|s| !s.trim().is_empty()) {
        parts.push(format!("Source: {s}"));
    }
    parts.join(" · ")
}

/// Footer text: "BLACK RAIN · Shooting Schedule".
pub(crate) fn footer(project: &str, document: &str) -> String {
    if project.trim().is_empty() {
        document.to_string()
    } else {
        format!("{} · {document}", project.trim())
    }
}

pub(crate) fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Trimmed text, or None when blank.
pub(crate) fn nonblank(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Decode an asset's image for a PDF. Returns None (with a reason) when the
/// file is missing, too large or not a readable picture — the caller shows a
/// placeholder and reports it; the reference itself is never changed.
pub(crate) fn load_image(asset: &AssetInfo) -> Result<ReportImage, &'static str> {
    if !asset.available {
        return Err("missing");
    }
    let Some(path) = asset.path.as_deref() else {
        return Err("missing");
    };
    let meta = std::fs::metadata(path).map_err(|_| "missing")?;
    if meta.len() as usize > ReportImage::MAX_SOURCE_BYTES {
        return Err("too large");
    }
    let bytes = std::fs::read(path).map_err(|_| "missing")?;
    ReportImage::decode(&bytes).map_err(|_| "unreadable")
}

/// "2 images could not be included (file missing)" style warnings.
pub(crate) fn image_warnings(missing: usize, unreadable: usize, what: &str) -> Vec<String> {
    let mut w = Vec::new();
    if missing > 0 {
        w.push(format!(
            "{} could not be found and {} shown as a placeholder. The references in your project were not changed.",
            plural(missing, what, &format!("{what}s")),
            if missing == 1 { "is" } else { "are" }
        ));
    }
    if unreadable > 0 {
        w.push(format!(
            "{} could not be read as a picture and {} shown as a placeholder.",
            plural(unreadable, what, &format!("{what}s")),
            if unreadable == 1 { "is" } else { "are" }
        ));
    }
    w
}

/// Money in minor units → "INR 12,00,000" / "USD 1,500.50".
pub(crate) fn money(minor: i64, currency: &str) -> String {
    format!("{currency} {}", amount_text(minor, currency, true))
}

/// Decimal digits of a currency's minor unit.
pub(crate) fn currency_digits(currency: &str) -> u32 {
    match currency {
        "JPY" | "KRW" => 0,
        _ => 2,
    }
}

/// Amount without currency: grouped for display, plain for spreadsheets.
pub(crate) fn amount_text(minor: i64, currency: &str, grouped: bool) -> String {
    let digits = currency_digits(currency);
    let neg = minor < 0;
    let abs = minor.unsigned_abs();
    let scale = 10u64.pow(digits);
    let whole = abs / scale;
    let frac = abs % scale;
    let whole_s = whole.to_string();
    let whole_s = if grouped {
        group(&whole_s, currency == "INR")
    } else {
        whole_s
    };
    let mut s = if neg { format!("-{whole_s}") } else { whole_s };
    if digits > 0 && (frac > 0 || !grouped) {
        s.push_str(&format!(".{:0width$}", frac, width = digits as usize));
    }
    s
}

fn group(digits: &str, indian: bool) -> String {
    let b = digits.as_bytes();
    if b.len() <= 3 {
        return digits.to_string();
    }
    let (head, tail) = digits.split_at(b.len() - 3);
    let mut groups: Vec<&str> = Vec::new();
    let size = if indian { 2 } else { 3 };
    let mut end = head.len();
    while end > 0 {
        let start = end.saturating_sub(size);
        groups.push(&head[start..end]);
        end = start;
    }
    groups.reverse();
    format!("{},{tail}", groups.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_groups_like_the_budget_view() {
        assert_eq!(money(120_000_000, "INR"), "INR 12,00,000");
        assert_eq!(money(150_050, "USD"), "USD 1,500.50");
        assert_eq!(money(1500, "JPY"), "JPY 1,500");
        assert_eq!(amount_text(150_050, "USD", false), "1500.50");
        assert_eq!(amount_text(-5, "EUR", true), "-0.05");
    }

    #[test]
    fn formats_are_limited_to_the_offered_ones() {
        assert_eq!(Fmt::parse("PDF", &[Fmt::Pdf]).unwrap(), Fmt::Pdf);
        assert!(Fmt::parse("csv", &[Fmt::Pdf]).is_err());
        assert!(Fmt::parse("docx", &[Fmt::Pdf, Fmt::Csv]).is_err());
    }
}
