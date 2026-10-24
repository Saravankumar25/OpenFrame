//! Moodboard presentation PDF (FSD §31.5), storyboard sheet PDF (FSD §32.7)
//! and Shot List (FSD §33.6: by scene, group of scenes or whole project).
//! Data comes from the visual module's own read queries, so labels (12A),
//! panel numbers and scene resolution are exactly what the workspace shows.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, BRAND_LINE, Block, Callout, CalloutCol, Card, CardHead, CardPart, Cards, Cell, CellItem,
    Chip, Col, Column, CompactHeader, Document, HeaderStyle, Masthead, Panel, ReportImage, Rgb,
    Rich, Row, Run, SceneBanner, Section, Side, StyledTable, Table, TableBand, Tone, Txt, pal,
    text_width,
};
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{
    Fmt, HINT_EDIT, HINT_TABLES, Job, Output, deliver, destination, footer, image_warnings,
    load_image, nonblank, plural, require_export,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;
use crate::modules::visual::moodboard::{self, MoodboardIdArgs};
use crate::modules::visual::shots::{self, ShotDto, ShotListArgs};
use crate::modules::visual::storyboard::{self, StoryboardIdArgs, StoryboardListArgs};
use crate::util::AssetInfo;

fn project(core: &AppCore) -> AppResult<String> {
    core.project()?.store.read(project_title)
}

/// Image counters for warnings.
#[derive(Default)]
struct Pics {
    missing: usize,
    unreadable: usize,
}

impl Pics {
    /// The picture, or a placeholder note when the file is missing or not printable; the
    /// reference itself is never changed.
    fn load(&mut self, asset: &AssetInfo) -> (Option<ReportImage>, Option<String>) {
        match load_image(asset) {
            Ok(img) => (Some(img), None),
            Err("unreadable") | Err("too large") => {
                self.unreadable += 1;
                (None, Some("Image can't be printed".into()))
            }
            Err(_) => {
                self.missing += 1;
                (
                    None,
                    Some(format!("Image unavailable: {}", asset.original_name)),
                )
            }
        }
    }
}

// ================================================================== moodboard

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportMoodboardArgs {
    pub moodboard_id: String,
    /// "pdf"
    pub format: String,
    pub path: String,
    /// "Project title and board name" (default true).
    #[serde(default)]
    #[ts(optional)]
    pub include_title: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub include_images: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub include_captions: Option<bool>,
    /// Internal board notes and items marked internal (default false).
    #[serde(default)]
    #[ts(optional)]
    pub include_internal_notes: Option<bool>,
}

pub(super) fn export_moodboard(
    core: &AppCore,
    actor: &Actor,
    a: ExportMoodboardArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf])?;
    crate::util::require_id(&a.moodboard_id, "moodboard")?;
    let with_title = a.include_title.unwrap_or(true);
    let with_images = a.include_images.unwrap_or(true);
    let with_captions = a.include_captions.unwrap_or(true);
    let internal = a.include_internal_notes.unwrap_or(false);
    let dest = destination(core, &a.path, fmt)?;
    let board = moodboard::get(
        core,
        actor,
        MoodboardIdArgs {
            id: a.moodboard_id.clone(),
        },
    )?;
    let project = project(core)?;

    let mut items: Vec<_> = board
        .items
        .iter()
        .filter(|i| internal || !i.is_private)
        .collect();
    // Reading order of the canvas: top to bottom, then left to right.
    items.sort_by_key(|i| (i.y / 40, i.x, i.z));
    let document = format!("Moodboard: {}", board.board.name);
    let title = if with_title {
        document.clone()
    } else {
        "Moodboard".to_string()
    };
    let mut doc = Document::new(&title).landscape(true);
    doc = doc.footer(if with_title {
        footer(&project, &document)
    } else {
        "Moodboard".to_string()
    });

    let mut pics = Pics::default();
    let mut cards: Vec<Card> = Vec::new();
    let mut texts: Vec<(Option<String>, String, bool, &'static str)> = Vec::new();
    let mut image_count = 0;
    for it in &items {
        let caption = if with_captions {
            nonblank(it.caption.as_deref())
        } else {
            None
        };
        match it.kind.as_str() {
            "image" => {
                image_count += 1;
                let (mut img, mut placeholder) = (None, None);
                if with_images {
                    match it.asset.as_ref() {
                        Some(a) => (img, placeholder) = pics.load(a),
                        None => placeholder = Some("Image unavailable".to_string()),
                    }
                }
                let mut card = Card::new();
                if img.is_some() || placeholder.is_some() {
                    card.parts.push(CardPart::Image {
                        img,
                        placeholder,
                        ratio: 0.72,
                    });
                }
                let mut lines = Vec::new();
                if let Some(c) = caption {
                    lines.push(
                        Txt::new(c)
                            .size(9.0)
                            .color(pal::INK)
                            .align(Align::Center)
                            .leading(1.3),
                    );
                }
                if it.is_private {
                    lines.push(
                        Txt::label("Internal")
                            .size(6.4)
                            .color(Tone::Warn.text())
                            .align(Align::Center),
                    );
                }
                if !lines.is_empty() {
                    card.parts.push(CardPart::Centered(lines));
                }
                if !card.parts.is_empty() {
                    cards.push(card);
                }
            }
            "note" => {
                let body = nonblank(it.body.as_deref()).unwrap_or_default();
                if !body.is_empty() || caption.is_some() {
                    texts.push((caption, body, it.is_private, "Note"));
                }
            }
            _ => {
                let mut line = nonblank(it.link_title.as_deref()).unwrap_or_default();
                if let Some(u) = &it.url {
                    if line.is_empty() {
                        line = u.clone();
                    } else {
                        line = format!("{line} ({u})");
                    }
                }
                texts.push((caption, line, it.is_private, "Link"));
            }
        }
    }
    if cards.is_empty() && texts.is_empty() && !(internal && board.notes.is_some()) {
        return Err(AppError::export(
            "nothing_to_export",
            "This moodboard has nothing to export with the selected options.",
        ));
    }
    let mut mast = if with_title {
        let mut m = Masthead::new(board.board.name.clone())
            .eyebrow("Visual development · Moodboard", pal::ORANGE);
        if let Some(p) = nb(&project) {
            m.subtitle = Some(p.to_string());
        }
        match (&board.board.scene_number, &board.board.scene_heading) {
            (Some(n), Some(h)) => m.lines.push(format!("Reference for Scene {n}: {h}")),
            (None, Some(h)) => m.lines.push(format!("Reference for {h}")),
            _ => {}
        }
        m.side = Some(Side::Panel {
            lines: vec![
                (plural(image_count, "image", "images"), true),
                (format!("Exported {}", super::today_label()), false),
            ],
        });
        m
    } else {
        Masthead::new("Moodboard")
    };
    mast.rule = true;
    doc.push(Block::Masthead(mast));
    if !cards.is_empty() {
        doc.push(Block::Cards(Cards::new(3, cards)));
    }
    if !texts.is_empty() {
        doc.push(Block::Section(Section::new("Notes and links", None)));
        let cards: Vec<Card> = texts
            .into_iter()
            .map(|(caption, body, private, kind)| {
                // The kind label is the title when there is no caption, so it is never printed twice.
                let titled = caption.is_some();
                let mut head = CardHead::new(caption.unwrap_or_else(|| kind.to_string()));
                if private {
                    head.right = Some(Chip::label("Internal", Tone::Warn));
                }
                if titled {
                    head.eyebrow = Some(kind.to_string());
                }
                Card {
                    head: Some(head),
                    parts: if body.is_empty() {
                        Vec::new()
                    } else {
                        vec![CardPart::Text(
                            Txt::new(body).size(8.8).color(pal::INK).leading(1.35),
                        )]
                    },
                    ..Card::new()
                }
            })
            .collect();
        doc.push(Block::Cards(Cards::new(2, cards)));
    }
    if internal && let Some(n) = nonblank(board.notes.as_deref()) {
        doc.push(Block::Section(Section::new(
            "Internal notes",
            Some("Not for distribution".into()),
        )));
        doc.push(Block::Callout(Callout {
            tone: Tone::Warn,
            cols: vec![{
                let mut c = CalloutCol::new(1.0);
                c.lines
                    .push(Txt::new(n).size(9.0).color(Tone::Warn.text()).leading(1.4));
                c
            }],
        }));
    }
    let mut out = Output::new(doc, HINT_EDIT);
    out.warnings = image_warnings(pics.missing, pics.unreadable, "image");
    let job = Job {
        action: "moodboard.export_pdf",
        document,
        source_label: board.board.name.clone(),
        scope_label: "Current moodboard".into(),
        contents_label: plural(image_count, "image", "images"),
        target: Some(("moodboard", board.board.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

// ================================================================== storyboard

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStoryboardSheetArgs {
    /// Storyboards to include in order (None = every storyboard).
    #[serde(default)]
    #[ts(optional)]
    pub storyboard_ids: Option<Vec<String>>,
    /// "pdf"
    pub format: String,
    pub path: String,
    /// Panel notes ("shot notes", FSD §32.7). Default false.
    #[serde(default)]
    #[ts(optional)]
    pub include_notes: Option<bool>,
    /// Panels per row: 2 (default) or 3 in the dialog; 4 is still accepted from older callers.
    #[serde(default)]
    #[ts(optional)]
    pub columns: Option<u8>,
}

const MUTED: Rgb = Rgb::hex(0x66728a);

/// Page furniture only: a long name is shortened with an ellipsis, never wrapped.
fn shortened(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

fn duration_label(ms: i64) -> String {
    let s = ms as f64 / 1000.0;
    if (s - s.round()).abs() < 0.05 {
        format!("{}s", s.round() as i64)
    } else {
        format!("{s:.1}s")
    }
}

pub(super) fn export_storyboards(
    core: &AppCore,
    actor: &Actor,
    a: ExportStoryboardSheetArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf])?;
    let notes = a.include_notes.unwrap_or(false);
    let columns = a.columns.unwrap_or(2).clamp(2, 4) as usize;
    if a.storyboard_ids.as_ref().is_some_and(|v| v.is_empty()) {
        return Err(AppError::validation(
            "storyboardIds",
            "Select at least one storyboard to export.",
        ));
    }
    let dest = destination(core, &a.path, fmt)?;
    let ids: Vec<String> = match &a.storyboard_ids {
        Some(v) => v.clone(),
        None => storyboard::list(core, actor, StoryboardListArgs {})?
            .into_iter()
            .map(|s| s.id)
            .collect(),
    };
    if ids.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            "There are no storyboards to export yet.",
        ));
    }
    let boards = ids
        .iter()
        .map(|id| storyboard::get(core, actor, StoryboardIdArgs { id: id.clone() }))
        .collect::<AppResult<Vec<_>>>()?;
    let project = project(core)?;
    let document = if boards.len() == 1 {
        format!("Storyboard: {}", boards[0].board.name)
    } else {
        "Storyboards".to_string()
    };
    let mut doc = Document::new(&document);
    let mut pics = Pics::default();
    let mut panels = 0;
    let scene_line = |b: &crate::modules::visual::storyboard::StoryboardDto| -> String {
        match (&b.board.scene_number, &b.board.scene_heading) {
            (Some(n), Some(h)) => format!("Scene {n}: {h}"),
            (None, Some(h)) => h.clone(),
            _ => "Standalone storyboard".into(),
        }
    };
    let total_panels: usize = boards.iter().map(|b| b.panels.len()).sum();
    doc.push(Block::CompactHeader(CompactHeader {
        eyebrow: "Pre-production · Storyboard".into(),
        title: if boards.len() == 1 {
            boards[0].board.name.clone()
        } else {
            "Storyboards".to_string()
        },
        project: nb(&project).map(str::to_string),
        scene_line: (boards.len() == 1).then(|| scene_line(&boards[0])),
        strong: plural(total_panels, "panel", "panels"),
        quiet: format!("· Exported {}", super::today_label()),
    }));
    for (i, b) in boards.iter().enumerate() {
        if i > 0 {
            doc.push(Block::PageBreak);
        }
        // Every page of a storyboard names it, so a loose page is never anonymous.
        let mut head: Vec<String> = nb(&project).map(str::to_uppercase).into_iter().collect();
        if boards.len() > 1 {
            head.push("STORYBOARDS".into());
            head.push(shortened(&b.board.name.to_uppercase(), 44));
        } else {
            head.push("STORYBOARD".into());
        }
        doc.push(Block::PageHeader(head.join(" · ")));
        // The banner stays with the first row; every storyboard starts on a page of its own.
        let status = if b.board.scene_removed {
            Some((
                "Scene removed from the script (kept for reference)",
                Tone::Danger,
            ))
        } else if b.board.needs_review {
            Some(("Scene changed since planning", Tone::Warn))
        } else {
            None
        };
        if boards.len() > 1 {
            doc.push(Block::SceneBanner(SceneBanner {
                eyebrow: "Storyboard".into(),
                title: b.board.name.clone(),
                scene_line: scene_line(b),
                count: plural(b.panels.len(), "panel", "panels"),
                status: status.map(|(t, tone)| (t.to_string(), tone)),
            }));
        } else if let Some((text, _)) = status {
            doc.push(Block::Text(
                Txt::new(text).size(8.8).color(pal::SLATE).after(8.0),
            ));
        }
        if b.panels.is_empty() {
            doc.push(Block::Text(
                Txt::new("No panels yet.").size(8.8).color(MUTED).after(6.0),
            ));
            continue;
        }
        let cards: Vec<Card> = b
            .panels
            .iter()
            .map(|p| {
                panels += 1;
                let (img, placeholder) = match p.visual_kind.as_str() {
                    "placeholder" => (None, Some("Blank panel".to_string())),
                    _ => match p.asset.as_ref() {
                        Some(a) => pics.load(a),
                        None => (None, Some("Image unavailable".to_string())),
                    },
                };
                let mut panel = Panel::new(p.number.to_string());
                panel.shot = p.shot_label.as_ref().map(|l| format!("Shot {l}"));
                panel.img = img;
                panel.placeholder = placeholder;
                panel.chips = [&p.framing, &p.movement, &p.angle]
                    .into_iter()
                    .filter_map(|x| nonblank(x.as_deref()))
                    .collect();
                panel.action = nonblank(Some(&p.description));
                if let Some(s) = nonblank(p.sound_note.as_deref()) {
                    panel.lines.push(("Sound:".into(), s));
                }
                if let Some(ms) = p.duration_ms.filter(|m| *m > 0) {
                    panel.lines.push(("Duration:".into(), duration_label(ms)));
                }
                if notes && let Some(n) = nonblank(p.note.as_deref()) {
                    panel.note = Some(("Note:".into(), n));
                }
                Card {
                    parts: vec![CardPart::Panel(Box::new(panel))],
                    border: Rgb::hex(0xb9c2d1),
                    ..Card::new()
                }
            })
            .collect();
        let mut grid = Cards::new(columns, cards);
        grid.gap = if columns >= 3 { 8.0 } else { 10.0 };
        grid.widow_repair = true;
        doc.push(Block::Cards(grid));
    }
    let mut out = Output::new(doc, HINT_EDIT);
    out.warnings = image_warnings(pics.missing, pics.unreadable, "panel image");
    let job = Job {
        action: "storyboard.export_sheet",
        document,
        source_label: if boards.len() == 1 {
            boards[0].board.name.clone()
        } else {
            "Storyboards".into()
        },
        scope_label: match &a.storyboard_ids {
            Some(v) if v.len() == 1 => "Current storyboard".into(),
            Some(v) => plural(v.len(), "selected storyboard", "selected storyboards"),
            None => "All storyboards".into(),
        },
        contents_label: plural(panels, "panel", "panels"),
        target: (boards.len() == 1).then(|| ("storyboard", boards[0].board.id.clone())),
    };
    deliver(core, actor, &dest, fmt, job, out)
}

// ================================================================== shot list

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportShotListArgs {
    /// "pdf" | "csv" | "xlsx"
    pub format: String,
    pub path: String,
    /// Scenes to include (screenplay scene ids, resolved by identity). None = whole project.
    #[serde(default)]
    #[ts(optional)]
    pub scene_ids: Option<Vec<String>>,
    /// Also shots of scenes removed from the script (whole project only).
    #[serde(default)]
    #[ts(optional)]
    pub include_removed: Option<bool>,
}

fn opt(s: &Option<String>) -> String {
    s.clone().unwrap_or_default()
}

fn scene_title(s: &ShotDto) -> String {
    match &s.scene_number {
        Some(n) => format!("Scene {n}: {}", s.scene_heading),
        None => s.scene_heading.clone(),
    }
}

/// A picture for a shot: its own reference image, else the first attached storyboard panel.
fn asset_of(s: &ShotDto) -> Option<&AssetInfo> {
    s.reference_asset
        .as_ref()
        .or_else(|| s.panels.iter().find_map(|p| p.asset.as_ref()))
}

fn panel_refs(s: &ShotDto) -> String {
    s.panels
        .iter()
        .map(|p| format!("{} #{}", p.storyboard_name, p.number))
        .collect::<Vec<_>>()
        .join(", ")
}

// ---- shot list layout

const CAMERA_PT: f32 = 8.2;
/// Side padding of every shot list cell; the column widths are measured with it.
const SHOT_PAD: f32 = 4.0;
const CAMERA_HEAD: Rgb = Rgb::hex(0xe2e9f1);
const CAMERA_NAMES: [(&str, f32); 4] = [
    ("Size", 40.0),
    ("Movement", 44.0),
    ("Angle", 40.0),
    ("Lens", 40.0),
];

/// A camera field of a shot (0 size, 1 movement, 2 angle, 3 lens); blank counts as none.
fn cam(s: &ShotDto, k: usize) -> Option<&str> {
    let v = match k {
        0 => &s.size,
        1 => &s.movement,
        2 => &s.angle,
        _ => &s.lens,
    };
    v.as_deref().map(str::trim).filter(|v| !v.is_empty())
}

/// Which columns exist and how wide they are, decided once for the whole export.
struct ShotGrid {
    image: bool,
    shot_w: f32,
    /// Camera columns that have data: index into [`CAMERA_NAMES`] and width.
    camera: Vec<(usize, f32)>,
}

impl ShotGrid {
    fn measure(shots: &[ShotDto]) -> ShotGrid {
        let image = shots.iter().any(|s| asset_of(s).is_some());
        // The shot tile is as wide as its longest label needs (the tile primitive's own size).
        let tile = shots
            .iter()
            .map(|s| text_width(&s.label, 9.0, true) + 14.0)
            .fold(28.0, f32::max);
        let shot_w = (tile + 2.0 * SHOT_PAD).max(38.0);
        let mut camera = Vec::new();
        for (k, (name, min)) in CAMERA_NAMES.iter().enumerate() {
            if !shots.iter().any(|s| cam(s, k).is_some()) {
                continue;
            }
            // The longest word sets the width, so no word breaks mid-word and no header wraps.
            let word = shots
                .iter()
                .filter_map(|s| cam(s, k))
                .flat_map(str::split_whitespace)
                .map(|w| text_width(w, CAMERA_PT, k == 0))
                .fold(0.0, f32::max);
            let head = text_width(&name.to_uppercase(), 7.0, true) + 0.8 * name.len() as f32;
            let w = ((word + 2.0).max(head + 1.5) + 2.0 * SHOT_PAD).clamp(*min, 84.0);
            camera.push((k, w));
        }
        ShotGrid {
            image,
            shot_w,
            camera,
        }
    }
}

/// The notes under a shot's description, in the builder's fixed order and wording.
fn shot_notes(s: &ShotDto) -> Option<Rich> {
    let mut paras: Vec<Vec<Run>> = Vec::new();
    let mut line = |label: &str, text: String| {
        paras.push(vec![
            Run::new(label, true, pal::NAVY),
            Run::new(text, false, pal::SLATE),
        ]);
    };
    if !s.characters.is_empty() {
        line("Characters:", s.characters.join(", "));
    }
    if !s.panels.is_empty() {
        line("Storyboard:", panel_refs(s));
    }
    if let Some(n) = nonblank(s.sound_note.as_deref()) {
        line("Sound:", n);
    }
    if let Some(n) = nonblank(s.camera_notes.as_deref()) {
        line("Camera:", n);
    }
    (!paras.is_empty()).then_some(Rich {
        paras,
        size: 8.0,
        leading: 1.3,
        gap: 2.0,
    })
}

/// Distinct values in order of first appearance.
fn distinct<'a>(values: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for v in values {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// One scene: its band (with status and, for a one-scene document, the coverage strip) and the
/// shot rows, under the shared column grid.
fn scene_table(list: &[&ShotDto], grid: &ShotGrid, single: bool, pics: &mut Pics) -> StyledTable {
    let first = list[0];
    let mut band = TableBand::new(first.scene_heading.clone());
    band.eyebrow = first.scene_number.as_ref().map(|n| format!("Scene {n}"));
    band.right = plural(list.len(), "shot", "shots");
    if first.scene_removed {
        band.fill = pal::SLATE;
        band.status = Some((
            "Scene removed from the script (kept for reference)".into(),
            Tone::Danger,
        ));
    } else if list.iter().any(|s| s.needs_review) {
        band.status = Some(("Scene changed since planning".into(), Tone::Warn));
    }
    if single {
        let lenses = distinct(list.iter().filter_map(|s| cam(s, 3)));
        let moves = distinct(list.iter().filter_map(|s| cam(s, 1)));
        let people = distinct(
            list.iter()
                .flat_map(|s| s.characters.iter().map(String::as_str)),
        );
        for (label, v) in [
            ("Lenses", lenses),
            ("Movement", moves),
            ("Characters in these shots", people),
        ] {
            if !v.is_empty() {
                band.strip.push((label.into(), v.join(" · ")));
            }
        }
    }

    let mut cols = vec![Col::pt("Done", 32.0).align(Align::Center)];
    if grid.image {
        cols.push(Col::pt("Image", 88.0));
    }
    cols.push(Col::pt("Shot", grid.shot_w).align(Align::Center));
    cols.push(Col::weight("Description", 1.0));
    for (k, w) in &grid.camera {
        cols.push(Col::pt(CAMERA_NAMES[*k].0, *w));
    }
    let head = |label: &str, align: Align, camera: bool| {
        let c = Cell::text(
            Txt::new(label.to_uppercase())
                .size(7.0)
                .bold()
                .track(0.8)
                .color(pal::SLATE),
        )
        .align(align);
        if camera { c.fill(CAMERA_HEAD) } else { c }
    };
    let first_camera = cols.len() - grid.camera.len();
    let header: Vec<Cell> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| head(&c.header, c.align, i >= first_camera))
        .collect();
    let mut t = StyledTable::new(cols);
    t.band = Some(band);
    t.boxed = true;
    t.header = HeaderStyle::Tint;
    t.header_cells = Some(header);
    t.pad_x = SHOT_PAD;
    t.pad_y = 5.0;
    for s in list {
        let mut cells = vec![Cell::new().align(Align::Center).item(CellItem::Check)];
        if grid.image {
            let item = match asset_of(s).map(|a| pics.load(a)) {
                Some((Some(img), _)) => CellItem::Image {
                    img,
                    w: 80.0,
                    h: 45.0,
                },
                Some((None, note)) => CellItem::Placeholder {
                    w: 80.0,
                    h: 45.0,
                    label: note.unwrap_or_else(|| "No image".into()),
                },
                None => CellItem::Placeholder {
                    w: 80.0,
                    h: 45.0,
                    label: "No image".into(),
                },
            };
            cells.push(Cell::new().item(item));
        }
        cells.push(
            Cell::new()
                .align(Align::Center)
                .item(CellItem::Tile(s.label.clone())),
        );
        let mut desc = Cell::text(
            Txt::new(s.description.clone())
                .size(8.8)
                .color(pal::INK)
                .leading(1.35),
        );
        if let Some(n) = shot_notes(s) {
            desc = desc.item(CellItem::Rich(n));
        }
        cells.push(desc);
        for (k, _) in &grid.camera {
            let text = match cam(s, *k) {
                Some(v) if *k == 0 => Txt::new(v).size(CAMERA_PT).bold().color(pal::NAVY),
                Some(v) => Txt::new(v).size(CAMERA_PT).color(pal::INK),
                None => Txt::new("–").size(CAMERA_PT).color(MUTED),
            };
            cells.push(Cell::text(text.leading(1.3)).fill(pal::PANEL));
        }
        t.rows.push(Row::new(cells));
    }
    t
}

/// The document with a one-line running header when it runs to more than one page. The probe
/// render is branded like the real one, so both have the same page body.
fn running_header_when_paged(mut doc: Document, line: String) -> Document {
    let mut probe = doc.clone();
    probe.brand = Some(BRAND_LINE.to_string());
    if probe.to_pdf().pages > 1 {
        doc.running_header = Some(line);
    }
    doc
}
pub(super) fn export_shots(
    core: &AppCore,
    actor: &Actor,
    a: ExportShotListArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Csv, Fmt::Xlsx])?;
    if a.scene_ids.as_ref().is_some_and(|v| v.is_empty()) {
        return Err(AppError::validation(
            "sceneIds",
            "Select at least one scene to export.",
        ));
    }
    let dest = destination(core, &a.path, fmt)?;
    let shots: Vec<ShotDto> = match &a.scene_ids {
        None => shots::list(
            core,
            actor,
            ShotListArgs {
                scene_id: None,
                scene_lineage_id: None,
                include_removed: a.include_removed,
            },
        )?,
        Some(ids) => {
            let mut all = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for id in ids {
                for s in shots::list(
                    core,
                    actor,
                    ShotListArgs {
                        scene_id: Some(id.clone()),
                        scene_lineage_id: None,
                        include_removed: None,
                    },
                )? {
                    if seen.insert(s.id.clone()) {
                        all.push(s);
                    }
                }
            }
            all
        }
    };
    if shots.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            if a.scene_ids.is_some() {
                "The selected scenes have no shots yet."
            } else {
                "There are no shots to export yet."
            },
        ));
    }
    let project = project(core)?;
    let mut groups: Vec<(String, Vec<&ShotDto>)> = Vec::new();
    for s in &shots {
        match groups.iter_mut().find(|(l, _)| *l == s.scene_lineage_id) {
            Some((_, v)) => v.push(s),
            None => groups.push((s.scene_lineage_id.clone(), vec![s])),
        }
    }
    let single = groups.len() == 1;
    let document = if single {
        format!("Shot List: {}", scene_title(groups[0].1[0]))
    } else {
        "Shot List".to_string()
    };
    let mut doc = Document::new(&document);
    let mut pics = Pics::default();
    let mut mast = Masthead::new("Shot List").eyebrow("Production · Shot list", pal::SLATE);
    mast.title_size = 20.0;
    if let Some(p) = nb(&project) {
        mast.subtitle = Some(p.to_string());
    }
    mast.side = Some(Side::Panel {
        lines: vec![
            (
                format!(
                    "{} in {}",
                    plural(shots.len(), "shot", "shots"),
                    plural(groups.len(), "scene", "scenes")
                ),
                true,
            ),
            (format!("Exported {}", super::today_label()), false),
        ],
    });
    mast.rule = true;
    doc.push(Block::Masthead(mast));

    // One column grid for the whole document, so scenes line up from table to table.
    let grid = ShotGrid::measure(&shots);
    for (_, list) in &groups {
        let table = scene_table(list, &grid, single, &mut pics);
        // A short scene is never split; a longer one breaks between rows.
        doc.push(if list.len() <= 3 {
            Block::Together(vec![Block::Styled(table)])
        } else {
            Block::Styled(table)
        });
    }
    let mut header = vec![nb(&project).map_or_else(String::new, str::to_uppercase)];
    header.push("SHOT LIST".into());
    if single && let Some(n) = &groups[0].1[0].scene_number {
        header.push(format!("SCENE {}", n.to_uppercase()));
    }
    header.retain(|h| !h.is_empty());
    let doc = running_header_when_paged(doc, header.join(" · "));
    let mut flat = Table::new(
        "Shot List",
        vec![
            Column::right("Scene"),
            Column::new("Scene heading"),
            Column::new("Shot"),
            Column::right("Order"),
            Column::new("Description"),
            Column::new("Size"),
            Column::new("Movement"),
            Column::new("Angle"),
            Column::new("Lens"),
            Column::new("Characters"),
            Column::new("Sound note"),
            Column::new("Camera notes"),
            Column::new("Storyboard"),
            Column::new("Status"),
        ],
    );
    for s in &shots {
        flat.push(vec![
            opt(&s.scene_number),
            s.scene_heading.clone(),
            s.label.clone(),
            s.order.to_string(),
            s.description.clone(),
            opt(&s.size),
            opt(&s.movement),
            opt(&s.angle),
            opt(&s.lens),
            s.characters.join(", "),
            opt(&s.sound_note),
            opt(&s.camera_notes),
            panel_refs(s),
            if s.scene_removed {
                "Scene removed".into()
            } else if s.needs_review {
                "Scene changed since planning".into()
            } else {
                String::new()
            },
        ]);
    }
    let mut out = Output::new(doc, HINT_TABLES);
    out.tables.push(flat);
    let job = Job {
        action: "shot.export_list",
        document,
        source_label: "Shot Lists".into(),
        scope_label: match &a.scene_ids {
            Some(v) if v.len() == 1 => "Current scene".into(),
            Some(v) => plural(v.len(), "selected scene", "selected scenes"),
            None => "Whole project".into(),
        },
        contents_label: format!(
            "{} in {}",
            plural(shots.len(), "shot", "shots"),
            plural(groups.len(), "scene", "scenes")
        ),
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}
