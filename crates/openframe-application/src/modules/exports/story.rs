//! Story Board export (FSD §60.2 "Outline/Story Board: PDF"; Import/Export
//! §6 Story Board PDF / Outline PDF; UX §3.x mock 072): acts, sequences,
//! beats and scene cards in board order, with optional notes, comments and
//! Parking Lot. "Outline" is a readable hierarchy; "Board" is a landscape
//! card grid per act. TXT and CSV carry the same outline.

use std::collections::{HashMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{Block, Column, Document, Grid, GridCell, Table};
use rusqlite::Connection;
use serde::Deserialize;
use ts_rs::TS;

use super::{
    Fmt, HINT_TEXT, Job, Output, deliver, destination, footer, nonblank, plural, require_export,
    subtitle,
};
use crate::core::AppCore;
use crate::modules::schedule::board::project_title;

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStoryOutlineArgs {
    /// "pdf" | "txt" | "csv"
    pub format: String,
    /// Absolute destination chosen in the Save dialog.
    pub path: String,
    /// "outline" (default) or "board" (landscape card grid; PDF only).
    #[serde(default)]
    #[ts(optional)]
    pub layout: Option<String>,
    /// "board" (current Story Board, default), "selected" or "project".
    #[serde(default)]
    #[ts(optional)]
    pub scope: Option<String>,
    /// Episode whose board is current (episodic projects).
    #[serde(default)]
    #[ts(optional)]
    pub episode_id: Option<String>,
    /// Selected acts, sequences, beats or cards (scope "selected"). Children of
    /// a selected container are included.
    #[serde(default)]
    #[ts(optional)]
    pub item_ids: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub include_parking: Option<bool>,
    /// Act/sequence/beat/card notes (never private notes).
    #[serde(default)]
    #[ts(optional)]
    pub include_notes: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub include_comments: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Act,
    Sequence,
    Beat,
    Card,
}

#[derive(Debug, Clone)]
struct Node {
    kind: Kind,
    id: String,
    /// Act/sequence title, beat text, card heading.
    title: String,
    /// Card short description.
    text: String,
    note: Option<String>,
    comments: Vec<String>,
    children: Vec<Node>,
    /// Scene card number in board order (cards outside the Parking Lot).
    number: Option<usize>,
}

struct Board {
    /// None for the film's board; the episode title otherwise.
    episode: Option<String>,
    acts: Vec<Node>,
    unassigned: Vec<Node>,
    parking: Vec<Node>,
}

type Key = (String, Option<String>);

fn take(map: &mut HashMap<Key, Vec<(i64, String, Node)>>, key: &Key) -> Vec<Node> {
    let mut v = map.remove(key).unwrap_or_default();
    v.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    v.into_iter().map(|t| t.2).collect()
}

fn load_comments(c: &Connection) -> AppResult<HashMap<String, Vec<String>>> {
    let mut st = c.prepare(
        "SELECT id, parent_id, target_id, author_name, status, body FROM comment
         WHERE deleted_at IS NULL AND target_type IN ('story_act','story_sequence','story_beat','story_scene_card')
         ORDER BY created_at, id",
    )?;
    let rows: Vec<(String, Option<String>, String, String, String, String)> = st
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    // Threads in order: each root followed by its replies.
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (id, parent, target, author, status, body) in rows.iter().filter(|r| r.1.is_none()) {
        let _ = parent;
        let list = out.entry(target.clone()).or_default();
        list.push(format!("Comment ({status}) — {author}: {}", body.trim()));
        for (_, _, _, ra, _, rb) in rows.iter().filter(|r| r.1.as_deref() == Some(id.as_str())) {
            list.push(format!("Reply — {ra}: {}", rb.trim()));
        }
    }
    Ok(out)
}

fn load_board(
    c: &Connection,
    episode: Option<&str>,
    comments: &HashMap<String, Vec<String>>,
) -> AppResult<Board> {
    let mut map: HashMap<Key, Vec<(i64, String, Node)>> = HashMap::new();
    let node = |kind, id: String, title: String, text: String, note: Option<String>| Node {
        comments: comments.get(&id).cloned().unwrap_or_default(),
        kind,
        id,
        title,
        text,
        note: nonblank(note.as_deref()),
        children: Vec::new(),
        number: None,
    };
    {
        let mut st = c.prepare(
            "SELECT id, parent_type, parent_id, short_description, scene_heading, notes, position
             FROM story_scene_card WHERE episode_id IS ?1 AND deleted_at IS NULL",
        )?;
        let rows: Vec<(
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            i64,
        )> = st
            .query_map([episode], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        for (id, pt, pid, desc, heading, notes, pos) in rows {
            let n = node(
                Kind::Card,
                id.clone(),
                heading.unwrap_or_default().trim().to_string(),
                desc.trim().to_string(),
                notes,
            );
            map.entry((pt, pid)).or_default().push((pos, id, n));
        }
    }
    {
        let mut st = c.prepare(
            "SELECT id, parent_type, parent_id, text, note, position FROM story_beat
             WHERE episode_id IS ?1 AND deleted_at IS NULL AND state = 'active'",
        )?;
        let rows: Vec<(String, String, Option<String>, String, Option<String>, i64)> = st
            .query_map([episode], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        for (id, pt, pid, text, note, pos) in rows {
            let n = node(
                Kind::Beat,
                id.clone(),
                text.trim().to_string(),
                String::new(),
                note,
            );
            map.entry((pt, pid)).or_default().push((pos, id, n));
        }
    }
    let acts: Vec<(String, String, Option<String>)> = {
        let mut st = c.prepare(
            "SELECT id, title, note FROM story_act WHERE episode_id IS ?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        st.query_map([episode], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?
    };
    let act_ids: HashSet<&str> = acts.iter().map(|a| a.0.as_str()).collect();
    {
        let mut st = c.prepare(
            "SELECT id, act_id, title, note, position FROM story_sequence WHERE episode_id IS ?1 AND deleted_at IS NULL",
        )?;
        let rows: Vec<(String, Option<String>, String, Option<String>, i64)> = st
            .query_map([episode], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;
        for (id, act, title, note, pos) in rows {
            let mut n = node(
                Kind::Sequence,
                id.clone(),
                title.trim().to_string(),
                String::new(),
                note,
            );
            n.children = take(&mut map, &("sequence".into(), Some(id.clone())));
            // A sequence whose act is gone sits in Unassigned (as on the board).
            let key = match act.filter(|a| act_ids.contains(a.as_str())) {
                Some(a) => ("act".to_string(), Some(a)),
                None => ("unassigned".to_string(), None),
            };
            map.entry(key).or_default().push((pos, id, n));
        }
    }
    let acts = acts
        .into_iter()
        .map(|(id, title, note)| {
            let mut n = node(
                Kind::Act,
                id.clone(),
                title.trim().to_string(),
                String::new(),
                note,
            );
            n.children = take(&mut map, &("act".into(), Some(id)));
            n
        })
        .collect();
    let mut board = Board {
        episode: None,
        acts,
        unassigned: take(&mut map, &("unassigned".into(), None)),
        parking: take(&mut map, &("parking".into(), None)),
    };
    let mut n = 0;
    for list in [&mut board.acts, &mut board.unassigned] {
        number_cards(list, &mut n);
    }
    Ok(board)
}

fn number_cards(nodes: &mut [Node], n: &mut usize) {
    for node in nodes {
        if node.kind == Kind::Card {
            *n += 1;
            node.number = Some(*n);
        }
        number_cards(&mut node.children, n);
    }
}

/// Keep selected nodes (with their whole subtree) and the ancestors that lead to them.
fn filter_selected(nodes: Vec<Node>, sel: &HashSet<String>) -> Vec<Node> {
    nodes
        .into_iter()
        .filter_map(|mut n| {
            if sel.contains(&n.id) {
                return Some(n);
            }
            n.children = filter_selected(std::mem::take(&mut n.children), sel);
            (!n.children.is_empty()).then_some(n)
        })
        .collect()
}

#[derive(Default)]
struct Counts {
    acts: usize,
    sequences: usize,
    beats: usize,
    cards: usize,
}

fn count(nodes: &[Node], c: &mut Counts) {
    for n in nodes {
        match n.kind {
            Kind::Act => c.acts += 1,
            Kind::Sequence => c.sequences += 1,
            Kind::Beat => c.beats += 1,
            Kind::Card => c.cards += 1,
        }
        count(&n.children, c);
    }
}

struct Opts {
    notes: bool,
    comments: bool,
}

fn card_title(n: &Node) -> String {
    let num = n
        .number
        .map(|x| format!("Scene Card {x}"))
        .unwrap_or_else(|| "Scene Card".into());
    if n.title.is_empty() {
        num
    } else {
        format!("{num} · {}", n.title)
    }
}

fn extras(n: &Node, o: &Opts) -> Vec<String> {
    let mut v = Vec::new();
    if o.notes
        && let Some(note) = &n.note
    {
        v.push(format!("Note: {note}"));
    }
    if o.comments {
        v.extend(n.comments.iter().cloned());
    }
    v
}

// ------------------------------------------------------------------ outline PDF

/// Acts and sequences are headings; beats and cards are indented under their
/// container so an act-level card after a sequence doesn't read as part of it.
fn outline_blocks(nodes: &[Node], depth: u8, o: &Opts, out: &mut Vec<Block>) {
    for n in nodes {
        let (level, child_depth) = match n.kind {
            Kind::Act => {
                out.push(Block::Heading {
                    text: n.title.clone(),
                    level: 1,
                });
                (0, 0)
            }
            Kind::Sequence => {
                out.push(Block::Heading {
                    text: format!("Sequence — {}", n.title),
                    level: 2,
                });
                (depth, depth + 1)
            }
            Kind::Card => {
                let mut line = card_title(n);
                if !n.text.is_empty() {
                    line.push_str(&format!(" — {}", n.text));
                }
                out.push(Block::Indented {
                    text: line,
                    level: depth,
                    muted: false,
                });
                (depth, depth + 1)
            }
            Kind::Beat => {
                out.push(Block::Indented {
                    text: format!("Beat: {}", n.title),
                    level: depth,
                    muted: false,
                });
                (depth, depth + 1)
            }
        };
        for e in extras(n, o) {
            out.push(Block::Indented {
                text: e,
                level: level + 1,
                muted: true,
            });
        }
        outline_blocks(&n.children, child_depth, o, out);
    }
}

// ------------------------------------------------------------------ board PDF

fn cell(n: &Node, o: &Opts) -> GridCell {
    match n.kind {
        Kind::Card => GridCell {
            title: Some(
                n.number
                    .map(|x| format!("Scene Card {x}"))
                    .unwrap_or_else(|| "Scene Card".into()),
            ),
            lines: [n.title.clone(), n.text.clone()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect(),
            notes: extras(n, o),
            ..Default::default()
        },
        _ => GridCell {
            title: Some("Beat".into()),
            lines: vec![n.title.clone()],
            notes: extras(n, o),
            ..Default::default()
        },
    }
}

fn board_blocks(items: &[Node], o: &Opts, out: &mut Vec<Block>) {
    let mut run: Vec<GridCell> = Vec::new();
    let flush = |run: &mut Vec<GridCell>, out: &mut Vec<Block>| {
        if !run.is_empty() {
            out.push(Block::Grid(Grid {
                columns: 4,
                frame_ratio: 0.5,
                cells: std::mem::take(run),
            }));
        }
    };
    for n in items {
        if n.kind == Kind::Sequence {
            flush(&mut run, out);
            out.push(Block::Heading {
                text: format!("Sequence — {}", n.title),
                level: 2,
            });
            for e in extras(n, o) {
                out.push(Block::Note(e));
            }
            let cells: Vec<GridCell> = n.children.iter().map(|c| cell(c, o)).collect();
            if cells.is_empty() {
                out.push(Block::Note("No beats or scene cards yet.".into()));
            } else {
                out.push(Block::Grid(Grid {
                    columns: 4,
                    frame_ratio: 0.5,
                    cells,
                }));
            }
        } else {
            run.push(cell(n, o));
        }
    }
    flush(&mut run, out);
}

// ------------------------------------------------------------------ text / csv

fn text_lines(nodes: &[Node], depth: usize, o: &Opts, out: &mut String) {
    let pad = "  ".repeat(depth);
    for n in nodes {
        match n.kind {
            Kind::Act => out.push_str(&format!("\n{pad}{}\n", n.title.to_uppercase())),
            Kind::Sequence => out.push_str(&format!("\n{pad}SEQUENCE — {}\n", n.title)),
            Kind::Card => {
                out.push_str(&format!("{pad}{}\n", card_title(n)));
                if !n.text.is_empty() {
                    out.push_str(&format!("{pad}  {}\n", n.text));
                }
            }
            Kind::Beat => out.push_str(&format!("{pad}Beat: {}\n", n.title)),
        }
        for e in extras(n, o) {
            out.push_str(&format!("{pad}  {e}\n"));
        }
        text_lines(&n.children, depth + 1, o, out);
    }
}

struct Ctx<'a> {
    episode: &'a str,
    act: &'a str,
    sequence: &'a str,
    area: &'a str,
}

fn csv_rows(nodes: &[Node], ctx: &Ctx<'_>, o: &Opts, t: &mut Table) {
    for n in nodes {
        let (kind, act, seq) = match n.kind {
            Kind::Act => ("Act", n.title.as_str(), ""),
            Kind::Sequence => ("Sequence", ctx.act, n.title.as_str()),
            Kind::Beat => ("Beat", ctx.act, ctx.sequence),
            Kind::Card => ("Scene Card", ctx.act, ctx.sequence),
        };
        let mut row = vec![
            ctx.episode.to_string(),
            ctx.area.to_string(),
            act.to_string(),
            seq.to_string(),
            kind.to_string(),
            n.number.map(|x| x.to_string()).unwrap_or_default(),
            match n.kind {
                Kind::Card => n.title.clone(),
                _ => String::new(),
            },
            match n.kind {
                Kind::Card => n.text.clone(),
                Kind::Beat => n.title.clone(),
                _ => String::new(),
            },
        ];
        if o.notes {
            row.push(n.note.clone().unwrap_or_default());
        }
        if o.comments {
            row.push(n.comments.join("\n"));
        }
        t.push(row);
        let child = Ctx {
            episode: ctx.episode,
            act,
            sequence: seq,
            area: ctx.area,
        };
        csv_rows(&n.children, &child, o, t);
    }
}

// ------------------------------------------------------------------ op

pub(super) fn export(
    core: &AppCore,
    actor: &Actor,
    a: ExportStoryOutlineArgs,
) -> AppResult<super::ExportResult> {
    require_export(actor)?;
    let fmt = Fmt::parse(&a.format, &[Fmt::Pdf, Fmt::Txt, Fmt::Csv])?;
    let board_layout = match a.layout.as_deref().unwrap_or("outline") {
        "outline" => false,
        "board" => true,
        _ => return Err(AppError::invalid_input("Choose Board PDF or Outline PDF.")),
    };
    if board_layout && fmt != Fmt::Pdf {
        return Err(AppError::invalid_input(
            "The board layout is a PDF. Choose Outline for plain text or CSV.",
        ));
    }
    let scope = a.scope.as_deref().unwrap_or("board");
    if !matches!(scope, "board" | "selected" | "project") {
        return Err(AppError::invalid_input(
            "Choose what to export: the current Story Board, selected items or the entire project.",
        ));
    }
    let selected: HashSet<String> = a.item_ids.clone().unwrap_or_default().into_iter().collect();
    if scope == "selected" && selected.is_empty() {
        return Err(AppError::validation(
            "itemIds",
            "Select at least one act, sequence, beat or scene card to export.",
        ));
    }
    let o = Opts {
        notes: a.include_notes.unwrap_or(false),
        comments: a.include_comments.unwrap_or(false),
    };
    let parking = a.include_parking.unwrap_or(false);
    let dest = destination(core, &a.path, fmt)?;
    let session = core.project()?;
    let (project, boards) = session.store.read(|c| {
        let comments = if o.comments {
            load_comments(c)?
        } else {
            HashMap::new()
        };
        let mut boards = Vec::new();
        if scope == "project" {
            let film = load_board(c, None, &comments)?;
            boards.push(film);
            let mut st = c.prepare(
                "SELECT e.id, e.title FROM episode e LEFT JOIN season s ON s.id = e.season_id
                 WHERE e.deleted_at IS NULL ORDER BY COALESCE(s.position, 0), e.position, e.id",
            )?;
            let eps: Vec<(String, String)> = st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            for (id, title) in eps {
                let mut b = load_board(c, Some(&id), &comments)?;
                b.episode = Some(title);
                boards.push(b);
            }
        } else {
            let mut b = load_board(c, a.episode_id.as_deref(), &comments)?;
            if let Some(ep) = &a.episode_id {
                let title: Option<String> = c
                    .query_row(
                        "SELECT title FROM episode WHERE id = ?1 AND deleted_at IS NULL",
                        [ep],
                        |r| r.get(0),
                    )
                    .ok();
                b.episode = Some(title.ok_or_else(|| AppError::not_found("episode"))?);
            }
            boards.push(b);
        }
        Ok((project_title(c)?, boards))
    })?;

    // Scope filtering.
    let boards: Vec<Board> = boards
        .into_iter()
        .map(|mut b| {
            if scope == "selected" {
                b.acts = filter_selected(std::mem::take(&mut b.acts), &selected);
                b.unassigned = filter_selected(std::mem::take(&mut b.unassigned), &selected);
                b.parking = filter_selected(std::mem::take(&mut b.parking), &selected);
            }
            if !parking {
                b.parking.clear();
            }
            b
        })
        .filter(|b| !(b.acts.is_empty() && b.unassigned.is_empty() && b.parking.is_empty()))
        .collect();
    if boards.is_empty() {
        return Err(AppError::export(
            "nothing_to_export",
            if scope == "selected" {
                "None of the selected items are on the Story Board any more. Select items and try again."
            } else {
                "The Story Board is empty, so there is nothing to export yet."
            },
        ));
    }
    let multi = boards.len() > 1;

    let mut counts = Counts::default();
    for b in &boards {
        count(&b.acts, &mut counts);
        count(&b.unassigned, &mut counts);
        count(&b.parking, &mut counts);
    }
    let document = if board_layout {
        "Story Board"
    } else {
        "Story Board Outline"
    };
    let scope_label = match scope {
        "selected" => "Selected items only".to_string(),
        "project" => "Entire project".to_string(),
        _ => "Current Story Board".to_string(),
    };
    let source_label = match (&boards[0].episode, multi) {
        (Some(ep), false) => format!("Story Board — {ep}"),
        _ => "Current Story Board".to_string(),
    };

    // ---- PDF
    let mut doc = Document::new(document)
        .subtitle(subtitle(&project, Some(&source_label)))
        .footer(footer(&project, document))
        .landscape(board_layout);
    for (i, b) in boards.iter().enumerate() {
        if board_layout && i > 0 {
            doc.push(Block::PageBreak);
        }
        if multi {
            doc.push(Block::Heading {
                text: b
                    .episode
                    .clone()
                    .unwrap_or_else(|| "Film Story Board".into()),
                level: 1,
            });
        }
        let sections: [(&str, &Vec<Node>); 2] =
            [("Unassigned", &b.unassigned), ("Parking Lot", &b.parking)];
        if board_layout {
            for act in &b.acts {
                doc.push(Block::SectionBar(act.title.clone()));
                for e in extras(act, &o) {
                    doc.push(Block::Note(e));
                }
                let mut blocks = Vec::new();
                board_blocks(&act.children, &o, &mut blocks);
                if blocks.is_empty() {
                    doc.push(Block::Note(
                        "No sequences, beats or scene cards yet.".into(),
                    ));
                }
                doc.blocks.extend(blocks);
            }
            for (label, items) in sections {
                if !items.is_empty() {
                    doc.push(Block::SectionBar(label.into()));
                    board_blocks(items, &o, &mut doc.blocks);
                }
            }
        } else {
            outline_blocks(&b.acts, 0, &o, &mut doc.blocks);
            for (label, items) in sections {
                if !items.is_empty() {
                    doc.push(Block::Heading {
                        text: label.into(),
                        level: 1,
                    });
                    outline_blocks(items, 0, &o, &mut doc.blocks);
                }
            }
        }
    }
    // ---- TXT
    let mut text = format!(
        "{}\n{}\n",
        document.to_uppercase(),
        subtitle(&project, Some(&source_label))
    );
    for b in &boards {
        if multi {
            text.push_str(&format!(
                "\n=== {} ===\n",
                b.episode
                    .clone()
                    .unwrap_or_else(|| "Film Story Board".into())
            ));
        }
        text_lines(&b.acts, 0, &o, &mut text);
        if !b.unassigned.is_empty() {
            text.push_str("\nUNASSIGNED\n");
            text_lines(&b.unassigned, 1, &o, &mut text);
        }
        if !b.parking.is_empty() {
            text.push_str("\nPARKING LOT\n");
            text_lines(&b.parking, 1, &o, &mut text);
        }
    }
    text.push_str("\nPrivate notes excluded.\n");

    // ---- CSV
    let mut cols = vec![
        Column::new("Episode"),
        Column::new("Area"),
        Column::new("Act"),
        Column::new("Sequence"),
        Column::new("Type"),
        Column::right("Card #"),
        Column::new("Heading"),
        Column::new("Description").weight(2.0),
    ];
    if o.notes {
        cols.push(Column::new("Notes").weight(1.5));
    }
    if o.comments {
        cols.push(Column::new("Comments").weight(1.5));
    }
    let mut table = Table::new("Story Board", cols);
    for b in &boards {
        let ep = b.episode.clone().unwrap_or_default();
        for (area, items) in [
            ("Board", &b.acts),
            ("Unassigned", &b.unassigned),
            ("Parking Lot", &b.parking),
        ] {
            csv_rows(
                items,
                &Ctx {
                    episode: &ep,
                    act: "",
                    sequence: "",
                    area,
                },
                &o,
                &mut table,
            );
        }
    }

    let mut out = Output::new(doc, HINT_TEXT);
    out.text = Some(text);
    out.tables.push(table);
    let contents = [
        (counts.acts, "act", "acts"),
        (counts.sequences, "sequence", "sequences"),
        (counts.cards, "scene card", "scene cards"),
        (counts.beats, "beat", "beats"),
    ]
    .iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, one, many)| plural(*n, one, many))
    .collect::<Vec<_>>()
    .join(" · ");
    let job = Job {
        action: "story.export_board",
        document: document.to_string(),
        source_label,
        scope_label,
        contents_label: contents,
        target: None,
    };
    deliver(core, actor, &dest, fmt, job, out)
}
