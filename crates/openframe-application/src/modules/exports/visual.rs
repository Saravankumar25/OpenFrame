//! Moodboard presentation PDF (FSD §31.5), storyboard sheet PDF (FSD §32.7)
//! and Shot List (FSD §33.6: by scene, group of scenes or whole project).
//! Data comes from the visual module's own read queries, so labels (12A),
//! panel numbers and scene resolution are exactly what the workspace shows.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Align, Banner, Block, Callout, CalloutCol, Card, CardHead, CardPart, Cards, Cell, CellItem,
    Chip, Col, Column, Document, HeaderStyle, Masthead, ReportImage, Row, Section, Side,
    StyledTable, Table, Tone, Txt, pal,
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
                let mut head = CardHead::new(caption.unwrap_or_else(|| kind.to_string()));
                if private {
                    head.right = Some(Chip::label("Internal", Tone::Warn));
                }
                head.eyebrow = Some(kind.to_string());
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
    /// Panels per row (2 to 4, default 3).
    #[serde(default)]
    #[ts(optional)]
    pub columns: Option<u8>,
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
    let columns = a.columns.unwrap_or(3).clamp(2, 4) as usize;
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
    let mut doc = Document::new(&document)
        .landscape(true)
        .footer(footer(&project, &document));
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
    let mut mast = Masthead::new(if boards.len() == 1 {
        boards[0].board.name.clone()
    } else {
        "Storyboards".to_string()
    })
    .eyebrow("Pre-production · Storyboard", pal::ORANGE);
    if let Some(p) = nb(&project) {
        mast.subtitle = Some(p.to_string());
    }
    if boards.len() == 1 {
        mast.lines.push(scene_line(&boards[0]));
    }
    mast.side = Some(Side::Panel {
        lines: vec![
            (plural(total_panels, "panel", "panels"), true),
            (format!("Exported {}", super::today_label()), false),
        ],
    });
    mast.rule = true;
    doc.push(Block::Masthead(mast));
    for (i, b) in boards.iter().enumerate() {
        if i > 0 {
            doc.push(Block::PageBreak);
        }
        let mut facts = vec![plural(b.panels.len(), "panel", "panels")];
        if b.board.scene_removed {
            facts.push("Scene removed from the script (kept for reference)".into());
        } else if b.board.needs_review {
            facts.push("Scene changed since planning".into());
        }
        if boards.len() > 1 {
            let mut banner = Banner::new(b.board.name.clone());
            banner.eyebrow = Some("Storyboard".into());
            banner.right = Some(plural(b.panels.len(), "panel", "panels"));
            banner.sub_left = vec![(scene_line(b), true)];
            if facts.len() > 1 {
                banner.sub_right = vec![(facts[1..].join(" · "), false)];
            }
            banner.keep_next = 150.0;
            doc.push(Block::Banner(banner));
        } else {
            if facts.len() > 1 {
                doc.push(Block::Text(
                    Txt::new(facts[1..].join(" · "))
                        .size(8.8)
                        .color(pal::SLATE)
                        .after(8.0),
                ));
            }
        }
        if b.panels.is_empty() {
            doc.push(Block::Text(
                Txt::new("No panels yet.")
                    .size(8.8)
                    .color(pal::FAINT)
                    .after(6.0),
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
                let mut title = format!("Panel {}", p.number);
                if let Some(l) = &p.shot_label {
                    title.push_str(&format!(" · Shot {l}"));
                }
                let mut head = CardHead::new(title);
                head.fill = pal::WHITE;
                let mut card = Card {
                    head: Some(head),
                    ..Card::new()
                };
                card.parts.push(CardPart::Image {
                    img,
                    placeholder,
                    ratio: 9.0 / 16.0,
                });
                let mut lines: Vec<Txt> = Vec::new();
                let framing: Vec<String> = [&p.framing, &p.movement, &p.angle]
                    .into_iter()
                    .filter_map(|x| nonblank(x.as_deref()))
                    .collect();
                if !framing.is_empty() {
                    lines.push(
                        Txt::new(framing.join(" · "))
                            .size(8.0)
                            .bold()
                            .color(pal::SLATE)
                            .align(Align::Center)
                            .after(3.0),
                    );
                }
                if let Some(d) = nonblank(Some(&p.description)) {
                    lines.push(
                        Txt::new(d)
                            .size(8.8)
                            .color(pal::INK)
                            .align(Align::Center)
                            .leading(1.3)
                            .after(2.0),
                    );
                }
                if let Some(s) = nonblank(p.sound_note.as_deref()) {
                    lines.push(
                        Txt::new(format!("Sound: {s}"))
                            .size(8.0)
                            .color(pal::SLATE)
                            .align(Align::Center),
                    );
                }
                if let Some(ms) = p.duration_ms.filter(|m| *m > 0) {
                    lines.push(
                        Txt::new(format!("Duration: {}", duration_label(ms)))
                            .size(8.0)
                            .color(pal::SLATE)
                            .align(Align::Center),
                    );
                }
                if notes && let Some(n) = nonblank(p.note.as_deref()) {
                    lines.push(
                        Txt::new(format!("Note: {n}"))
                            .size(8.0)
                            .color(pal::FAINT)
                            .align(Align::Center),
                    );
                }
                if !lines.is_empty() {
                    card.parts.push(CardPart::Centered(lines));
                }
                card
            })
            .collect();
        doc.push(Block::Cards(Cards::new(columns, cards)));
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
    let mut doc = Document::new(&document)
        .landscape(true)
        .footer(footer(&project, "Shot List"));
    let mut pics = Pics::default();
    let mut mast = Masthead::new("Shot List").eyebrow("Production · Shot list", pal::ORANGE);
    if let Some(p) = nb(&project) {
        mast.subtitle = Some(p.to_string());
    }
    if single {
        mast.lines.push(scene_title(groups[0].1[0]));
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

    // A picture per shot: its own reference image, else the first attached storyboard panel.
    let any_image = shots.iter().any(|s| asset_of(s).is_some());
    let has = |f: fn(&ShotDto) -> bool| shots.iter().any(f);
    let (has_size, has_move, has_angle, has_lens) = (
        has(|s| s.size.is_some()),
        has(|s| s.movement.is_some()),
        has(|s| s.angle.is_some()),
        has(|s| s.lens.is_some()),
    );
    // Characters, sound, camera notes and storyboard references share one Notes column.
    let has_notes = has(|s| {
        s.sound_note.is_some()
            || s.camera_notes.is_some()
            || !s.characters.is_empty()
            || !s.panels.is_empty()
    });
    for (_, list) in &groups {
        let first = list[0];
        let mut banner = Banner::new(scene_title(first));
        banner.right = Some(plural(list.len(), "shot", "shots"));
        if first.scene_removed {
            banner.sub_left = vec![(
                "Scene removed from the script (kept for reference)".into(),
                true,
            )];
        } else if list.iter().any(|s| s.needs_review) {
            banner.sub_left = vec![("Scene changed since planning".into(), true)];
        }
        banner.keep_next = if any_image { 90.0 } else { 60.0 };
        doc.push(Block::Banner(banner));
        let mut cols = vec![Col::pt("Done", 40.0).align(Align::Center)];
        if any_image {
            cols.push(Col::pt("Image", 96.0));
        }
        cols.push(Col::pt("Shot", 36.0).align(Align::Center));
        cols.push(Col::weight("Description", 2.4));
        if has_size {
            cols.push(Col::pt("Size", 48.0));
        }
        if has_move {
            cols.push(Col::pt("Movement", 60.0));
        }
        if has_angle {
            cols.push(Col::pt("Angle", 50.0));
        }
        if has_lens {
            cols.push(Col::pt("Lens", 44.0));
        }
        if has_notes {
            cols.push(Col::weight("Notes", 1.5));
        }
        let mut t = StyledTable::new(cols);
        t.boxed = true;
        t.header = HeaderStyle::Tint;
        t.continued = Some(scene_title(first));
        let small = |s: String| Cell::text(Txt::new(s).size(8.2).color(pal::INK).leading(1.3));
        for s in list {
            let mut cells = vec![Cell::new().align(Align::Center).item(CellItem::Check)];
            if any_image {
                let cell = match asset_of(s).map(|a| pics.load(a)) {
                    Some((Some(img), _)) => Cell::new().item(CellItem::Image {
                        img,
                        w: 88.0,
                        h: 50.0,
                    }),
                    Some((None, note)) => Cell::new().item(CellItem::Placeholder {
                        w: 88.0,
                        h: 50.0,
                        label: note.unwrap_or_else(|| "No image".into()),
                    }),
                    None => Cell::new().item(CellItem::Placeholder {
                        w: 88.0,
                        h: 50.0,
                        label: "No image".into(),
                    }),
                };
                cells.push(cell);
            }
            cells.push(Cell::text(
                Txt::new(s.label.clone())
                    .size(10.0)
                    .bold()
                    .color(pal::NAVY)
                    .align(Align::Center),
            ));
            cells.push(Cell::text(
                Txt::new(s.description.clone())
                    .size(8.8)
                    .color(pal::INK)
                    .leading(1.35),
            ));
            if has_size {
                cells.push(small(opt(&s.size)));
            }
            if has_move {
                cells.push(small(opt(&s.movement)));
            }
            if has_angle {
                cells.push(small(opt(&s.angle)));
            }
            if has_lens {
                cells.push(small(opt(&s.lens)));
            }
            if has_notes {
                let mut c = Cell::new();
                let line = |t: String| {
                    CellItem::Text(Txt::new(t).size(8.0).color(pal::SLATE).leading(1.3))
                };
                if !s.characters.is_empty() {
                    c = c.item(line(format!("Characters: {}", s.characters.join(", "))));
                }
                if !s.panels.is_empty() {
                    c = c.item(line(format!("Storyboard: {}", panel_refs(s))));
                }
                if let Some(n) = nonblank(s.sound_note.as_deref()) {
                    c = c.item(CellItem::Text(
                        Txt::new(format!("Sound: {n}"))
                            .size(8.0)
                            .color(pal::SLATE)
                            .leading(1.3),
                    ));
                }
                if let Some(n) = nonblank(s.camera_notes.as_deref()) {
                    c = c.item(CellItem::Text(
                        Txt::new(format!("Camera: {n}"))
                            .size(8.0)
                            .color(pal::SLATE)
                            .leading(1.3),
                    ));
                }
                cells.push(c);
            }
            t.rows.push(Row::new(cells));
        }
        doc.push(Block::Styled(t));
    }
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
