//! Sides PDF (FSD §58.2–58.4, §111; UX §3.32 mock 148): only the scenes of a
//! shooting day (or a selection), laid out by the screenplay PDF engine
//! (Courier 12, industry margins, scene numbers) with a header on every page
//! naming the production, the day and the script draft (FSD §111.4), and an
//! optional call-sheet cover page (§111.3).
//!
//! Source: a saved sides snapshot (`side.content_json`, immutable) or the live
//! selection built by `sides.preview`. Sides never change the screenplay.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::layout::{
    self, CHAR_WIDTH_IN, LEFT_MARGIN_IN, LINES_PER_PAGE, LaidOutPage, LayoutOptions, LineStyle,
    PAGE_WIDTH_IN, PageLine,
};
use openframe_import_export::report::Document;
use openframe_import_export::{Element, ElementKind, Scene, ScreenplayDoc, pdf_export};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use serde_json::json;
use ts_rs::TS;

use super::{Fmt, HINT_EDIT, Job, Output, deliver, destination, plural, require_export};
use crate::core::AppCore;
use crate::modules::schedule::docs::ScheduleSidesContent;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportSidesArgs {
    /// "pdf"
    pub format: String,
    pub path: String,
    /// A saved sides snapshot. When absent the sides are built from the
    /// selection below (same rules as the Sides preview).
    #[serde(default)]
    #[ts(optional)]
    pub side_id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub day_id: Option<String>,
    /// Selected scene strips (empty = every scene of the day).
    #[serde(default)]
    #[ts(optional)]
    pub strip_ids: Option<Vec<String>>,
    /// "Selected cast": only scenes with one of these cast keys.
    #[serde(default)]
    #[ts(optional)]
    pub cast_keys: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub include_cover: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub show_draft_name: Option<bool>,
}

fn kind(t: &str) -> Option<ElementKind> {
    match t {
        "scene_heading" | "note" => None,
        other => Some(ElementKind::from_stored(other).unwrap_or(ElementKind::Action)),
    }
}

/// Centered title-page line.
fn centered(row: i32, text: &str, style: LineStyle) -> PageLine {
    let len = text.chars().count() as f32;
    PageLine {
        row,
        x_in: ((PAGE_WIDTH_IN - len * CHAR_WIDTH_IN) / 2.0).max(1.0),
        text: text.to_string(),
        style,
    }
}

fn left(row: i32, x: f32, text: &str) -> PageLine {
    PageLine {
        row,
        x_in: x,
        text: text.to_string(),
        style: LineStyle::TitleText,
    }
}

fn fit(s: &str, width: usize) -> String {
    let n = s.chars().count();
    if n <= width {
        format!("{s:<width$}")
    } else {
        let mut t: String = s.chars().take(width.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// Cover page with the call-sheet information (FSD §111.3).
fn cover_page(c: &ScheduleSidesContent) -> Option<LaidOutPage> {
    let cover = c.cover.as_ref()?;
    let mut lines = Vec::new();
    let mut row = 4;
    lines.push(centered(
        row,
        &c.production_title.to_uppercase(),
        LineStyle::TitleMain,
    ));
    row += 2;
    lines.push(centered(row, "SIDES", LineStyle::TitleText));
    row += 1;
    let day = [c.day_label.clone(), c.date_label.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" — ");
    if !day.is_empty() {
        lines.push(centered(row, &day, LineStyle::TitleText));
        row += 1;
    }
    if c.show_draft_name {
        lines.push(centered(row, &c.source_label, LineStyle::TitleText));
        row += 1;
    }
    row += 3;
    let x = LEFT_MARGIN_IN - 0.5;
    if let Some(call) = cover.crew_call.as_deref().filter(|s| !s.trim().is_empty()) {
        lines.push(left(row, x, &format!("CREW CALL: {call}")));
        row += 2;
    }
    if !cover.locations.is_empty() {
        lines.push(left(row, x, "LOCATION"));
        row += 1;
        for l in &cover.locations {
            for w in layout::wrap(l, 65) {
                lines.push(left(row, x + 0.3, &w));
                row += 1;
            }
        }
        row += 1;
    }
    if !cover.cast.is_empty() {
        lines.push(left(row, x, "CAST"));
        row += 1;
        lines.push(left(
            row,
            x + 0.3,
            &format!("{}{}CALL", fit("CHARACTER", 24), fit("ACTOR", 24)),
        ));
        row += 1;
        for m in &cover.cast {
            let line = format!(
                "{}{}{}",
                fit(&m.character, 24),
                fit(m.actor.as_deref().unwrap_or("—"), 24),
                m.call_time.as_deref().unwrap_or("—")
            );
            lines.push(left(row, x + 0.3, line.trim_end()));
            row += 1;
        }
        row += 1;
    }
    if let Some(n) = cover.notes.as_deref().filter(|s| !s.trim().is_empty()) {
        lines.push(left(row, x, "NOTES"));
        row += 1;
        for w in layout::wrap(n, 65) {
            lines.push(left(row, x + 0.3, &w));
            row += 1;
        }
    }
    lines.retain(|l| l.row < LINES_PER_PAGE as i32);
    Some(LaidOutPage {
        title_page: true,
        number: None,
        lines,
    })
}

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportSidesArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf])?;
    let dest = destination(core, &a.path, fmt)?;
    let (content, saved_title) = match &a.side_id {
        Some(id) => {
            crate::util::require_id(id, "sides")?;
            let row: Option<(String, String)> = core.project()?.store.read(|c| {
                Ok(c.query_row(
                    "SELECT title, content_json FROM side WHERE id = ?1 AND deleted_at IS NULL",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?)
            })?;
            let (title, json) = row.ok_or_else(|| AppError::not_found("sides"))?;
            let content: ScheduleSidesContent = serde_json::from_str(&json)
                .map_err(|e| AppError::internal(format!("saved sides: {e}")))?;
            (content, Some(title))
        }
        None => {
            if a.day_id.is_none() && a.strip_ids.as_ref().is_none_or(|v| v.is_empty()) {
                return Err(AppError::required(
                    "A shooting day or a selection of scenes",
                ));
            }
            let v = core.dispatch(
                actor,
                "sides.preview",
                json!({
                    "dayId": a.day_id,
                    "stripIds": a.strip_ids.clone().unwrap_or_default(),
                    "castKeys": a.cast_keys,
                    "includeCover": a.include_cover.unwrap_or(false),
                    "showDraftName": a.show_draft_name.unwrap_or(true),
                }),
            )?;
            let content: ScheduleSidesContent = serde_json::from_value(v)
                .map_err(|e| AppError::internal(format!("sides preview: {e}")))?;
            (content, None)
        }
    };
    if content.scenes.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "There are no scenes in these sides.",
        ));
    }

    // Screenplay-formatted pages for the selected scenes.
    let mut doc = ScreenplayDoc::default();
    for s in &content.scenes {
        let mut scene = Scene::new(s.heading.clone());
        scene.number = Some(s.number.clone());
        scene.elements = s
            .elements
            .iter()
            .filter_map(|e| kind(&e.element_type).map(|k| Element::new(k, e.text.clone())))
            .collect();
        doc.scenes.push(scene);
    }
    let opts = LayoutOptions {
        title_page: false,
        scene_numbers: true,
        include_notes: false,
        revision_marks: false,
        revision_info: false,
        page_numbers: true,
        paginate: true,
    };
    let mut lay = layout::layout(&doc, &opts);
    // Header on every script page: "BLACK RAIN · SIDES · SHOOT DAY 4 · Draft 6".
    let header: String = {
        let h = content.header.trim();
        // 1" to 7.5" at 10 characters per inch.
        if h.chars().count() > 65 {
            let mut t: String = h.chars().take(64).collect();
            t.push('…');
            t
        } else {
            h.to_string()
        }
    };
    for page in lay.pages.iter_mut() {
        page.lines
            .insert(0, left(-4, LEFT_MARGIN_IN - 0.5, &header));
    }
    if let Some(cover) = cover_page(&content) {
        lay.pages.insert(0, cover);
    }
    let bad = openframe_import_export::unsupported_pdf_characters(&lay);
    if !bad.is_empty() {
        let sample: Vec<String> = bad.iter().take(6).map(|(c, _)| format!("“{c}”")).collect();
        return Err(AppError::export(
            "pdf_unsupported_characters",
            format!(
                "These sides contain characters the standard screenplay PDF font (Courier) can't print, such as {}. \
                 No PDF was written. {HINT_EDIT}",
                sample.join(" ")
            ),
        )
        .with_detail(format!("{} distinct non-WinAnsi character(s)", bad.len())));
    }
    let document = match (&saved_title, &content.day_label) {
        (Some(t), _) => t.clone(),
        (None, Some(d)) => format!("Sides — {d}"),
        (None, None) => "Sides".to_string(),
    };
    let pdf = pdf_export::render_layout(
        &lay,
        &format!("{} — {document}", content.production_title),
        None,
    );
    let mut out = Output::new(Document::new(&document), HINT_EDIT);
    out.pdf = Some((pdf.bytes, lay.pages.len() as u32));
    if a.include_cover.unwrap_or(false) && content.cover.is_none() && a.side_id.is_none() {
        out.warnings
            .push("A cover page needs a shooting day, so none was added.".into());
    }
    let job = Job {
        action: "sides.export_pdf",
        document,
        source_label: content.source_label.clone(),
        scope_label: match (&a.side_id, &content.day_label) {
            (Some(_), _) => "Saved sides".into(),
            (None, Some(d)) if a.strip_ids.as_ref().is_none_or(|v| v.is_empty()) => {
                format!("{d} — all scenes")
            }
            _ => "Selected scenes".into(),
        },
        contents_label: plural(content.scenes.len(), "scene", "scenes"),
        target: a.side_id.clone().map(|id| ("side", id)),
    };
    deliver(core, actor, &dest, fmt, job, out)
}
