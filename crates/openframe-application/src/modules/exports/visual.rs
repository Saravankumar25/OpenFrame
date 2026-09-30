//! Moodboard presentation PDF (FSD §31.5), storyboard sheet PDF (FSD §32.7)
//! and Shot List (FSD §33.6: by scene, group of scenes or whole project).
//! Data comes from the visual module's own read queries, so labels (12A),
//! panel numbers and scene resolution are exactly what the workspace shows.

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Grid, GridCell, Table};
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_EDIT, HINT_TABLES, Job, Output, deliver, destination, footer, image_warnings,
    load_image, nonblank, plural, require_export, subtitle,
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
    fn cell_image(&mut self, asset: Option<&AssetInfo>, cell: &mut GridCell) {
        let Some(asset) = asset else { return };
        match load_image(asset) {
            Ok(img) => cell.image = Some(img),
            Err("unreadable") | Err("too large") => {
                self.unreadable += 1;
                cell.placeholder = Some("Image can't be printed".into());
            }
            Err(_) => {
                self.missing += 1;
                cell.placeholder = Some(format!("Image unavailable — {}", asset.original_name));
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
    let document = format!("Moodboard — {}", board.board.name);
    let title = if with_title {
        document.clone()
    } else {
        "Moodboard".to_string()
    };
    let mut doc = Document::new(&title).landscape(true);
    if with_title {
        let scene = match (&board.board.scene_number, &board.board.scene_heading) {
            (Some(n), Some(h)) => Some(format!("Reference for Scene {n} — {h}")),
            (None, Some(h)) => Some(format!("Reference for {h}")),
            _ => None,
        };
        doc = doc
            .subtitle(subtitle(&project, scene.as_deref()))
            .footer(footer(&project, &document));
    } else {
        doc = doc.footer("Moodboard");
    }

    let mut pics = Pics::default();
    let mut cells = Vec::new();
    let mut texts: Vec<(Option<String>, String, bool)> = Vec::new();
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
                let mut cell = GridCell::default();
                if with_images {
                    pics.cell_image(it.asset.as_ref(), &mut cell);
                    if it.asset.is_none() {
                        cell.placeholder = Some("Image unavailable".into());
                    }
                }
                if let Some(c) = caption {
                    cell.lines.push(c);
                }
                if it.is_private {
                    cell.notes.push("Internal".into());
                }
                if cell.image.is_some() || cell.placeholder.is_some() || !cell.lines.is_empty() {
                    cells.push(cell);
                }
            }
            "note" => {
                let body = nonblank(it.body.as_deref()).unwrap_or_default();
                if !body.is_empty() || caption.is_some() {
                    texts.push((caption, body, it.is_private));
                }
            }
            _ => {
                let mut line = nonblank(it.link_title.as_deref()).unwrap_or_default();
                if let Some(u) = &it.url {
                    if line.is_empty() {
                        line = u.clone();
                    } else {
                        line = format!("{line} — {u}");
                    }
                }
                texts.push((caption, line, it.is_private));
            }
        }
    }
    if cells.is_empty() && texts.is_empty() && !(internal && board.notes.is_some()) {
        return Err(AppError::export(
            "nothing_to_export",
            "This moodboard has nothing to export with the selected options.",
        ));
    }
    if !cells.is_empty() {
        doc.push(Block::Grid(Grid {
            columns: 3,
            frame_ratio: 0.72,
            cells,
        }));
    }
    if !texts.is_empty() {
        doc.push(Block::Heading {
            text: "Notes and links".into(),
            level: 2,
        });
        for (caption, body, private) in texts {
            let mut line = match caption {
                Some(c) if !body.is_empty() => format!("{c}: {body}"),
                Some(c) => c,
                None => body,
            };
            if private {
                line = format!("(Internal) {line}");
            }
            doc.push(Block::Paragraph(line));
        }
    }
    if internal && let Some(n) = nonblank(board.notes.as_deref()) {
        doc.push(Block::Heading {
            text: "Internal notes".into(),
            level: 2,
        });
        doc.push(Block::Paragraph(n));
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
    /// Panels per row (2–4, default 3).
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
        format!("Storyboard — {}", boards[0].board.name)
    } else {
        "Storyboards".to_string()
    };
    let mut doc = Document::new(&document)
        .landscape(true)
        .subtitle(subtitle(&project, None))
        .footer(footer(&project, &document));
    let mut pics = Pics::default();
    let mut panels = 0;
    for (i, b) in boards.iter().enumerate() {
        if i > 0 {
            doc.push(Block::PageBreak);
        }
        if boards.len() > 1 {
            doc.push(Block::Heading {
                text: b.board.name.clone(),
                level: 1,
            });
        }
        let scene = match (&b.board.scene_number, &b.board.scene_heading) {
            (Some(n), Some(h)) => format!("Scene {n} — {h}"),
            (None, Some(h)) => h.clone(),
            _ => "Standalone storyboard".into(),
        };
        let mut facts = vec![scene, plural(b.panels.len(), "panel", "panels")];
        if b.board.scene_removed {
            facts.push("Scene removed from the script (kept for reference)".into());
        } else if b.board.needs_review {
            facts.push("Scene changed since planning".into());
        }
        doc.push(Block::Note(facts.join(" · ")));
        if b.panels.is_empty() {
            doc.push(Block::Paragraph("No panels yet.".into()));
            continue;
        }
        let cells = b
            .panels
            .iter()
            .map(|p| {
                panels += 1;
                let mut cell = GridCell::default();
                match p.visual_kind.as_str() {
                    "placeholder" => cell.placeholder = Some("Blank panel".into()),
                    _ => {
                        pics.cell_image(p.asset.as_ref(), &mut cell);
                        if p.asset.is_none() {
                            cell.placeholder = Some("Image unavailable".into());
                        }
                    }
                }
                let mut title = format!("Panel {}", p.number);
                if let Some(l) = &p.shot_label {
                    title.push_str(&format!(" · Shot {l}"));
                }
                cell.title = Some(title);
                if let Some(d) = nonblank(Some(&p.description)) {
                    cell.lines.push(d);
                }
                let framing: Vec<String> = [&p.framing, &p.movement, &p.angle]
                    .into_iter()
                    .filter_map(|x| nonblank(x.as_deref()))
                    .collect();
                if !framing.is_empty() {
                    cell.lines.push(framing.join(" · "));
                }
                if let Some(s) = nonblank(p.sound_note.as_deref()) {
                    cell.lines.push(format!("Sound: {s}"));
                }
                if let Some(ms) = p.duration_ms.filter(|m| *m > 0) {
                    cell.lines.push(format!("Duration: {}", duration_label(ms)));
                }
                if notes && let Some(n) = nonblank(p.note.as_deref()) {
                    cell.notes.push(format!("Note: {n}"));
                }
                cell
            })
            .collect();
        doc.push(Block::Grid(Grid {
            columns,
            frame_ratio: 9.0 / 16.0,
            cells,
        }));
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
        Some(n) => format!("Scene {n} — {}", s.scene_heading),
        None => s.scene_heading.clone(),
    }
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
        format!("Shot List — {}", scene_title(groups[0].1[0]))
    } else {
        "Shot List".to_string()
    };
    let mut doc = Document::new(&document)
        .landscape(true)
        .subtitle(subtitle(&project, None))
        .footer(footer(&project, "Shot List"));
    let columns = || {
        vec![
            Column::new("Shot").weight(0.45),
            Column::new("Description").weight(2.2),
            Column::new("Size").weight(0.7),
            Column::new("Movement").weight(0.8),
            Column::new("Angle").weight(0.7),
            Column::new("Lens").weight(0.5),
            Column::new("Characters"),
            Column::new("Sound"),
            Column::new("Camera notes"),
            Column::new("Storyboard"),
        ]
    };
    for (_, list) in &groups {
        let first = list[0];
        if !single {
            doc.push(Block::Heading {
                text: scene_title(first),
                level: 2,
            });
        }
        let mut facts = vec![plural(list.len(), "shot", "shots")];
        if first.scene_removed {
            facts.push("Scene removed from the script (kept for reference)".into());
        } else if list.iter().any(|s| s.needs_review) {
            facts.push("Scene changed since planning".into());
        }
        doc.push(Block::Note(facts.join(" · ")));
        let mut t = Table::new("", columns());
        for s in list {
            t.push(vec![
                s.label.clone(),
                s.description.clone(),
                opt(&s.size),
                opt(&s.movement),
                opt(&s.angle),
                opt(&s.lens),
                s.characters.join(", "),
                opt(&s.sound_note),
                opt(&s.camera_notes),
                panel_refs(s),
            ]);
        }
        doc.push(Block::Table(t));
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
