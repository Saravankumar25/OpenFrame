//! Sides PDF (FSD §58.2 to 58.4, §111; UX §3.32 mock 148): only the scenes of a
//! shooting day (or a selection), laid out by the screenplay PDF engine
//! (Courier 12, industry margins, scene numbers) with a header on every page
//! naming the production, the day and the script draft (FSD §111.4), and an
//! optional call-sheet cover page (§111.3).
//!
//! Source: a saved sides snapshot (`side.content_json`, immutable) or the live
//! selection built by `sides.preview`. Sides never change the screenplay.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::fonts;
use openframe_import_export::layout::{
    self, CHAR_WIDTH_IN, LEFT_MARGIN_IN, LINES_PER_PAGE, LaidOutPage, LayoutOptions, LineStyle,
    PAGE_WIDTH_IN, PageLine, RIGHT_EDGE_IN,
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
    PageLine {
        row,
        x_in: ((PAGE_WIDTH_IN - fonts::columns(text) * CHAR_WIDTH_IN) / 2.0).max(1.0),
        text: text.to_string(),
        style,
    }
}

/// Characters of the running header before the page number ("10." plus a gap) starts.
const HEADER_CHARS: usize = ((RIGHT_EDGE_IN - (LEFT_MARGIN_IN - 0.5)) * 10.0) as usize - 6;

/// Set the running header in lines of at most [`HEADER_CHARS`] characters, breaking only at the
/// " · " separators (a single part longer than a line wraps at its spaces). At most three lines
/// fit the header band; anything beyond that is cut at a fixed width and ends with an ellipsis,
/// so the same header always gives the same lines.
fn fit_header(header: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for part in header.split(" · ").map(str::trim).filter(|p| !p.is_empty()) {
        for piece in layout::wrap(part, HEADER_CHARS) {
            match lines.last_mut() {
                Some(last) if last.chars().count() + 3 + piece.chars().count() <= HEADER_CHARS => {
                    last.push_str(" · ");
                    last.push_str(&piece);
                }
                _ => lines.push(piece),
            }
        }
    }
    if lines.len() > 3 {
        lines.drain(3..);
        if let Some(last) = lines.last_mut() {
            let mut t: String = last.chars().take(HEADER_CHARS - 1).collect();
            t.push('\u{2026}');
            *last = t;
        }
    }
    lines
}

/// Cover page columns (inches from the page edge): the cast table at the cover's text indent.
const COVER_X: f32 = LEFT_MARGIN_IN - 0.5;
const COVER_INDENT: f32 = 0.3;
/// Character, actor and call columns of the cast table: x position and width in characters. The
/// gaps between them are 0.2 inch; the call column ends at the screenplay's right edge.
const CAST_COLUMNS: [(f32, usize); 3] = [(1.3, 22), (3.7, 22), (6.1, 14)];
/// Width in characters of free text under a cover heading (it ends at the right edge).
const COVER_TEXT_CHARS: usize = 62;
/// Centred title lines wrap at this many characters.
const COVER_CENTRE_CHARS: usize = 50;

/// The cover being filled. Nothing is cut: a block that does not fit what is left of a page
/// continues on a new cover page.
struct Cover {
    pages: Vec<Vec<PageLine>>,
    row: i32,
}

impl Cover {
    fn new() -> Self {
        Self {
            pages: vec![Vec::new()],
            row: 4,
        }
    }
    fn next_page(&mut self) {
        self.pages.push(Vec::new());
        self.row = 0;
    }
    /// Room for `rows` rows, or a new page.
    fn need(&mut self, rows: usize) {
        if self.row + rows as i32 > LINES_PER_PAGE as i32 && self.row > 0 {
            self.next_page();
        }
    }
    fn put(&mut self, line: PageLine) {
        self.pages.last_mut().expect("a page").push(line);
    }
    fn line(&mut self, x: f32, text: &str, style: LineStyle) {
        self.need(1);
        self.put(PageLine {
            row: self.row,
            x_in: x,
            text: text.to_string(),
            style,
        });
        self.row += 1;
    }
    fn centred(&mut self, text: &str, style: LineStyle) {
        for w in layout::wrap(text, COVER_CENTRE_CHARS) {
            self.need(1);
            let l = centered(self.row, &w, style);
            self.put(l);
            self.row += 1;
        }
    }
    /// Wrapped free text at the cover indent.
    fn text(&mut self, x: f32, text: &str) {
        for w in layout::wrap(text, COVER_TEXT_CHARS) {
            self.line(x, &w, LineStyle::TitleText);
        }
    }
    fn gap(&mut self, rows: i32) {
        self.row += rows;
    }
    /// A table row: each cell wraps inside its own column, and the row is as tall as its tallest cell.
    fn table_row(&mut self, cells: [&str; 3]) {
        let wrapped: Vec<Vec<String>> = cells
            .iter()
            .zip(CAST_COLUMNS)
            .map(|(t, (_, w))| layout::wrap(t, w))
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
        self.need(height);
        for (lines, (x, _)) in wrapped.iter().zip(CAST_COLUMNS) {
            for (i, l) in lines.iter().enumerate() {
                self.put(PageLine {
                    row: self.row + i as i32,
                    x_in: x,
                    text: l.clone(),
                    style: LineStyle::TitleText,
                });
            }
        }
        self.row += height as i32;
    }
}

/// Cover page(s) with the call-sheet information (FSD §111.3). Long names and notes wrap inside
/// their columns and the cover grows onto further pages, so no text is ever cut.
fn cover_pages(c: &ScheduleSidesContent) -> Vec<LaidOutPage> {
    let Some(cover) = c.cover.as_ref() else {
        return Vec::new();
    };
    let mut v = Cover::new();
    v.centred(&c.production_title.to_uppercase(), LineStyle::TitleMain);
    v.gap(1);
    v.centred("SIDES", LineStyle::TitleText);
    let day = [c.day_label.clone(), c.date_label.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    if !day.is_empty() {
        v.centred(&day, LineStyle::TitleText);
    }
    if c.show_draft_name {
        v.centred(&c.source_label, LineStyle::TitleText);
    }
    v.gap(3);
    let x = COVER_X;
    let indent = x + COVER_INDENT;
    if let Some(call) = cover.crew_call.as_deref().filter(|s| !s.trim().is_empty()) {
        v.text(x, &format!("CREW CALL: {call}"));
        v.gap(1);
    }
    if !cover.locations.is_empty() {
        v.need(2);
        v.line(x, "LOCATION", LineStyle::TitleText);
        for l in &cover.locations {
            v.text(indent, l);
        }
        v.gap(1);
    }
    if !cover.cast.is_empty() {
        v.need(4);
        v.line(x, "CAST", LineStyle::TitleText);
        v.table_row(["CHARACTER", "ACTOR", "CALL"]);
        for m in &cover.cast {
            let cells = [
                m.character.as_str(),
                m.actor.as_deref().unwrap_or("Needs input"),
                m.call_time.as_deref().unwrap_or("Needs input"),
            ];
            let rows = cells
                .iter()
                .zip(CAST_COLUMNS)
                .map(|(t, (_, w))| layout::wrap(t, w).len())
                .max()
                .unwrap_or(1);
            // A table that continues on a new cover page repeats its column titles.
            if v.row + rows as i32 > LINES_PER_PAGE as i32 {
                v.next_page();
                v.table_row(["CHARACTER", "ACTOR", "CALL"]);
            }
            v.table_row(cells);
        }
        v.gap(1);
    }
    if let Some(n) = cover.notes.as_deref().filter(|s| !s.trim().is_empty()) {
        v.need(2);
        v.line(x, "NOTES", LineStyle::TitleText);
        v.text(indent, n);
    }
    v.pages
        .into_iter()
        .map(|lines| LaidOutPage {
            title_page: true,
            number: None,
            lines,
        })
        .collect()
}

/// The laid-out pages of a sides document: the selected scenes in screenplay format under the
/// running header, preceded by the optional cover page and carrying the production attribution.
/// Pure, so it can be tested without a project.
fn build_layout(content: &ScheduleSidesContent) -> layout::Layout {
    // Screenplay-formatted pages for the selected scenes.
    let mut doc = ScreenplayDoc::default();
    for s in &content.scenes {
        let mut scene = Scene::new(s.heading.clone());
        scene.number = Some(s.number.clone());
        scene.elements = s
            .elements
            .iter()
            .filter_map(|e| {
                kind(&e.element_type).map(|k| Element::new(k, e.text.clone()).dual(e.dual))
            })
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
        page: Default::default(),
    };
    let mut lay = layout::layout(&doc, &opts);
    // Running header on every script page: "PROJECT · SIDES · SHOOT DAY 4 · DRAFT". It is set in
    // the header band beside the page number and wraps at the " · " separators when it is too
    // long for one line, so nothing is clipped and it never reaches the page number.
    let header_lines = fit_header(content.header.trim());
    let first_row = -3 - (header_lines.len() as i32 - 1);
    for page in lay.pages.iter_mut() {
        for (k, line) in header_lines.iter().enumerate() {
            page.lines.insert(
                0,
                PageLine {
                    row: first_row + k as i32,
                    x_in: LEFT_MARGIN_IN - 0.5,
                    text: line.clone(),
                    style: LineStyle::Header,
                },
            );
        }
    }
    lay.pages.splice(0..0, cover_pages(content));
    // Sides are pages of the writer's script: like screenplay files they carry no OpenFrame mark
    // and no brand line (owner decision 2026-10-27).
    lay
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

    let lay = build_layout(&content);
    let bad = openframe_import_export::unsupported_pdf_characters(&lay);
    if !bad.is_empty() {
        let sample: Vec<String> = bad.iter().take(6).map(|(c, _)| format!("“{c}”")).collect();
        return Err(AppError::export(
            "pdf_unsupported_characters",
            format!(
                "These sides contain characters the screenplay PDF fonts can't print, such as {}. \
                 No PDF was written. {HINT_EDIT}",
                sample.join(" ")
            ),
        )
        .with_detail(format!("{} distinct non-WinAnsi character(s)", bad.len())));
    }
    let document = match (&saved_title, &content.day_label) {
        (Some(t), _) => t.clone(),
        (None, Some(d)) => format!("Sides: {d}"),
        (None, None) => "Sides".to_string(),
    };
    let pdf = pdf_export::render_layout(
        &lay,
        &format!("{}: {document}", content.production_title),
        None,
    );
    let mut out = Output::new(Document::new(&document), HINT_EDIT);
    out.unbranded = true;
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
                format!("{d}, all scenes")
            }
            _ => "Selected scenes".into(),
        },
        contents_label: plural(content.scenes.len(), "scene", "scenes"),
        target: a.side_id.clone().map(|id| ("side", id)),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::schedule::docs::{
        ScheduleSidesCover, ScheduleSidesCoverCast, ScheduleSidesElement, ScheduleSidesScene,
    };

    fn content(scenes: usize, cover: Option<ScheduleSidesCover>) -> ScheduleSidesContent {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../templates/samples/Example-Feature.fountain");
        let doc = openframe_import_export::import_bytes(
            &std::fs::read(path).unwrap(),
            openframe_import_export::SourceFormat::Fountain,
        )
        .unwrap()
        .doc;
        let scenes = doc
            .scenes
            .iter()
            .filter(|s| !s.heading.trim().is_empty())
            .take(scenes)
            .enumerate()
            .map(|(i, s)| ScheduleSidesScene {
                scene_id: format!("s{i}"),
                number: (i + 1).to_string(),
                heading: s.heading.clone(),
                elements: s
                    .elements
                    .iter()
                    .map(|e| ScheduleSidesElement {
                        element_type: e.kind.as_str().to_string(),
                        text: e.text.clone(),
                        dual: e.dual,
                    })
                    .collect(),
            })
            .collect();
        ScheduleSidesContent {
            header: "THE TOWER AT PEDDA REVU · SIDES · SHOOT DAY 4 · Shooting Draft 6".into(),
            production_title: "The Tower at Pedda Revu".into(),
            day_label: Some("Shoot day 4".into()),
            date_label: Some("Monday 14 June 2027".into()),
            source_label: "The Tower at Pedda Revu: Shooting Draft 6".into(),
            show_draft_name: true,
            cover,
            scenes,
        }
    }

    fn long_cover() -> ScheduleSidesCover {
        ScheduleSidesCover {
            crew_call: Some("06:30".into()),
            locations: vec![
                "Rayapuram Bus Stand, Old Bus Stand Road, Rayapuram, Andhra Pradesh 533001".into(),
            ],
            cast: [
                ("LAKSHMI", "Venkata Subramanyam Chakravarthy Rao"),
                ("FAROOQ", "Mohammed Farooq Siddiqui"),
                ("SUBBAMMA THE RADIO REPAIR SHOP KEEPER", "Needs"),
            ]
            .iter()
            .map(|(c, a)| ScheduleSidesCoverCast {
                character: c.to_string(),
                actor: Some(a.to_string()),
                call_time: Some("07:15".into()),
            })
            .collect(),
            notes: Some("Rain cover on standby. Bring the generator.".into()),
        }
    }

    fn cast(n: usize) -> Vec<ScheduleSidesCoverCast> {
        (0..n)
            .map(|i| ScheduleSidesCoverCast {
                character: format!("CHARACTER NUMBER {i} WITH A LONG DESCRIPTIVE NAME"),
                actor: Some(format!("Venkata Subramanyam Chakravarthy Rao the {i}th")),
                call_time: Some("07:15 AM".into()),
            })
            .collect()
    }

    fn cover_text(pages: &[LaidOutPage]) -> String {
        pages
            .iter()
            .flat_map(|p| p.lines.iter())
            .map(|l| l.text.as_str())
            .collect()
    }

    fn letters(s: &str) -> Vec<char> {
        let mut v: Vec<char> = s.chars().filter(|c| c.is_alphanumeric()).collect();
        v.sort_unstable();
        v
    }

    fn width_in(l: &PageLine) -> f32 {
        l.text.chars().count() as f32 * CHAR_WIDTH_IN
    }

    /// Nothing on the cover may overlap, touch another line on its row, or leave the page.
    fn assert_clean(pages: &[LaidOutPage]) {
        for p in pages {
            for a in &p.lines {
                assert!(
                    a.row >= 0 && a.row < LINES_PER_PAGE as i32,
                    "{:?} row {}",
                    a.text,
                    a.row
                );
                assert!(
                    a.x_in + width_in(a) <= RIGHT_EDGE_IN + 1e-3,
                    "{:?} runs off",
                    a.text
                );
                for b in &p.lines {
                    if std::ptr::eq(a, b) || a.row != b.row || a.x_in >= b.x_in {
                        continue;
                    }
                    assert!(
                        a.x_in + width_in(a) + 0.15 <= b.x_in,
                        "{:?} touches {:?}",
                        a.text,
                        b.text
                    );
                }
            }
        }
    }

    #[test]
    fn cover_wraps_long_names_inside_their_columns_and_cuts_nothing() {
        let mut cover = long_cover();
        cover.cast.extend(cast(3));
        cover.notes = Some("Rain cover on standby. ".repeat(12));
        let c = content(2, Some(cover.clone()));
        let pages = cover_pages(&c);
        assert_eq!(pages.len(), 1);
        assert_clean(&pages);
        let printed = cover_text(&pages);
        assert!(!printed.contains('…'), "an ellipsis means text was cut");
        let mut wanted = String::new();
        wanted.push_str(&c.production_title.to_uppercase());
        wanted.push_str("SIDES");
        wanted.push_str(c.day_label.as_deref().unwrap());
        wanted.push_str(c.date_label.as_deref().unwrap());
        wanted.push_str(&c.source_label);
        wanted.push_str("CREW CALL: 06:30LOCATION");
        for l in &cover.locations {
            wanted.push_str(l);
        }
        wanted.push_str("CASTCHARACTERACTORCALL");
        for m in &cover.cast {
            wanted.push_str(&m.character);
            wanted.push_str(m.actor.as_deref().unwrap_or("Needs input"));
            wanted.push_str(m.call_time.as_deref().unwrap_or("Needs input"));
        }
        wanted.push_str("NOTES");
        wanted.push_str(cover.notes.as_deref().unwrap());
        assert_eq!(letters(&printed), letters(&wanted));
    }

    #[test]
    fn cover_continues_on_a_second_page_with_the_cast_header_repeated() {
        let mut cover = long_cover();
        cover.cast = cast(40);
        let c = content(1, Some(cover));
        let pages = cover_pages(&c);
        assert!(pages.len() >= 2, "{} cover page(s)", pages.len());
        assert_clean(&pages);
        for p in &pages[1..] {
            assert!(
                p.lines.iter().any(|l| l.text == "CHARACTER"),
                "column titles repeat"
            );
        }
        let printed = cover_text(&pages);
        for i in 0..40 {
            assert!(
                printed.contains(&format!("NUMBER{i}"))
                    || printed.contains(&format!("NUMBER {i}"))
                    || printed.contains(&i.to_string())
            );
        }
        // Every page of the sides is the cover pages first, then the script.
        let lay = build_layout(&c);
        assert!(lay.pages[..pages.len()].iter().all(|p| p.title_page));
        assert!(!lay.pages[pages.len()].title_page);
    }

    #[test]
    fn dual_dialogue_is_kept_in_sides_and_old_snapshots_still_load() {
        let mut c = content(1, None);
        c.scenes[0].elements = [
            ("character", "MEERA", true),
            ("dialogue", "Left side.", true),
            ("character", "ARJUN", true),
            ("dialogue", "Right side.", true),
            ("character", "RAVI", false),
            ("dialogue", "Alone.", false),
        ]
        .iter()
        .map(|(t, x, d)| ScheduleSidesElement {
            element_type: t.to_string(),
            text: x.to_string(),
            dual: *d,
        })
        .collect();
        let lay = build_layout(&c);
        let cue_x = |name: &str| {
            lay.pages
                .iter()
                .flat_map(|p| p.lines.iter())
                .find(|l| l.text == name)
                .map(|l| l.x_in)
                .unwrap()
        };
        assert!(cue_x("MEERA") < 3.0 && cue_x("ARJUN") > 5.0, "two columns");
        assert!(
            (cue_x("RAVI") - 3.7).abs() < 1e-3,
            "an ordinary speech stays centred"
        );
        let old: ScheduleSidesElement =
            serde_json::from_str(r#"{"elementType":"action","text":"x"}"#).unwrap();
        assert!(!old.dual);
    }

    #[test]
    fn sides_use_the_same_gaps_as_the_screenplay() {
        let mut c = content(1, None);
        c.scenes[0].elements = vec![
            ScheduleSidesElement {
                element_type: "action".into(),
                text: "Rain.".into(),
                dual: false,
            },
            ScheduleSidesElement {
                element_type: "transition".into(),
                text: "CUT TO:".into(),
                dual: false,
            },
        ];
        let mut second = c.scenes[0].clone();
        second.heading = "INT. NEXT - NIGHT".into();
        c.scenes.push(second);
        let lay = build_layout(&c);
        let row = |t: &str| {
            lay.pages
                .iter()
                .flat_map(|p| p.lines.iter())
                .find(|l| l.text == t)
                .unwrap()
                .row
        };
        assert_eq!(
            row("INT. NEXT - NIGHT") - row("CUT TO:"),
            3,
            "36 pt after a transition"
        );
        assert_eq!(row("Rain.") - row(&c.scenes[0].heading.to_uppercase()), 2);
    }

    /// Sample sides PDFs for visual review (not a regression test): `$SCREENPLAY_CHECK_DIR`
    /// or `release-out/screenplay-check/`.
    #[test]
    #[ignore = "writes sample PDFs for review"]
    fn write_sides_check_pdfs() {
        let dir = std::env::var("SCREENPLAY_CHECK_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../release-out/screenplay-check")
            });
        std::fs::create_dir_all(&dir).unwrap();
        for (name, c) in [
            ("e-sides-cover.pdf", content(6, Some(long_cover()))),
            ("g-sides-long-cover.pdf", {
                let mut cover = long_cover();
                cover.cast.extend(cast(45));
                content(6, Some(cover))
            }),
            ("f-sides-no-cover.pdf", content(6, None)),
        ] {
            let lay = build_layout(&c);
            let pdf = pdf_export::render_layout(&lay, name, None);
            std::fs::write(dir.join(name), pdf.bytes).unwrap();
        }
    }
}
