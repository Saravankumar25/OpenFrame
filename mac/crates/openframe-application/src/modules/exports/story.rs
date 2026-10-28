//! Beat Board export (FSD §60.2 "Outline/Beat Board: PDF"; Import/Export
//! §6 Beat Board PDF / Outline PDF; UX §3.x mock 072): acts, sequences,
//! beats and scene cards in board order, with optional notes, comments and
//! Parking Lot. "Outline" is a readable hierarchy; "Board" is a portrait
//! card board with three cards to a row (PDF design package v2). TXT and CSV
//! carry the same outline.

use std::collections::{HashMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult};
use openframe_import_export::report::{
    Block, Board, BoardComment, BoardCount, BoardExtra, BoardItem, BoardItems, BoardKey,
    BoardLayout, Column, Document, Masthead, Rgb, Side, Table, Txt, pal,
};
use rusqlite::Connection;
use serde::Deserialize;
use ts_rs::TS;

use super::style::nb;
use super::{
    Fmt, HINT_TEXT, Job, Output, deliver, destination, nonblank, plural, require_export, subtitle,
    today_label,
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
    /// "outline" (default) or "board" (portrait card board; PDF only).
    #[serde(default)]
    #[ts(optional)]
    pub layout: Option<String>,
    /// "board" (current Beat Board, default), "selected" or "project".
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
    comments: Vec<Thread>,
    children: Vec<Node>,
    /// Scene card number in board order (cards outside the Parking Lot).
    number: Option<usize>,
}

struct Tray {
    /// None for the film's board; the episode title otherwise.
    episode: Option<String>,
    acts: Vec<Node>,
    unassigned: Vec<Node>,
    parking: Vec<Node>,
}

/// A comment thread on a board item, with its replies.
#[derive(Debug, Clone)]
struct Thread {
    status: String,
    author: String,
    body: String,
    replies: Vec<(String, String)>,
}

type Key = (String, Option<String>);

fn take(map: &mut HashMap<Key, Vec<(i64, String, Node)>>, key: &Key) -> Vec<Node> {
    let mut v = map.remove(key).unwrap_or_default();
    v.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    v.into_iter().map(|t| t.2).collect()
}

fn load_comments(c: &Connection) -> AppResult<HashMap<String, Vec<Thread>>> {
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
    let mut out: HashMap<String, Vec<Thread>> = HashMap::new();
    for (id, _, target, author, status, body) in rows.iter().filter(|r| r.1.is_none()) {
        out.entry(target.clone()).or_default().push(Thread {
            status: status.clone(),
            author: author.clone(),
            body: body.trim().to_string(),
            replies: rows
                .iter()
                .filter(|r| r.1.as_deref() == Some(id.as_str()))
                .map(|(_, _, _, ra, _, rb)| (ra.clone(), rb.trim().to_string()))
                .collect(),
        });
    }
    Ok(out)
}

fn load_board(
    c: &Connection,
    episode: Option<&str>,
    comments: &HashMap<String, Vec<Thread>>,
) -> AppResult<Tray> {
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
    let mut board = Tray {
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

/// The lines of a thread as plain text (TXT and CSV).
fn thread_lines(t: &Thread) -> Vec<String> {
    let mut v = vec![format!(
        "Comment ({}) from {}: {}",
        t.status, t.author, t.body
    )];
    for (a, b) in &t.replies {
        v.push(format!("Reply from {a}: {b}"));
    }
    v
}

fn extras(n: &Node, o: &Opts) -> Vec<String> {
    let mut v = Vec::new();
    if o.notes
        && let Some(note) = &n.note
    {
        v.push(format!("Note: {note}"));
    }
    if o.comments {
        v.extend(n.comments.iter().flat_map(thread_lines));
    }
    v
}

// ------------------------------------------------------------------ PDF (outline and board)

/// Secondary text that passes 4.5:1 on white.
const MUTED: Rgb = Rgb::hex(0x66728a);
/// Cards to a row on the card board.
const BOARD_COLUMNS: usize = 3;

/// "LOW TIDE RADIO · BEAT BOARD": names the production on every page of a printed board.
fn running_header(project: &str, document: &str) -> String {
    match nb(project) {
        Some(p) => format!("{} · {}", p.to_uppercase(), document.to_uppercase()),
        None => document.to_uppercase(),
    }
}

fn pdf_item(n: &Node, o: &Opts) -> BoardItem {
    let beat = n.kind == Kind::Beat;
    BoardItem {
        number: n.number,
        beat,
        heading: if beat { String::new() } else { n.title.clone() },
        text: if beat {
            n.title.clone()
        } else {
            n.text.clone()
        },
        extras: pdf_extras(n, o),
    }
}

/// The note, then the comment threads, as the renderer wants them.
fn pdf_extras(n: &Node, o: &Opts) -> Vec<BoardExtra> {
    let mut v = Vec::new();
    if o.notes
        && let Some(note) = &n.note
    {
        v.push(BoardExtra::Note(note.clone()));
    }
    if o.comments {
        v.extend(n.comments.iter().map(|t| {
            BoardExtra::Comment(BoardComment {
                status: t.status.clone(),
                author: t.author.clone(),
                body: t.body.clone(),
                replies: t.replies.clone(),
            })
        }));
    }
    v
}

/// "2 sequences · 12 scene cards · 3 beats": what is inside a container, zero parts left out.
fn inside(children: &[Node], with_sequences: bool) -> Option<String> {
    let mut c = Counts::default();
    count(children, &mut c);
    let parts: Vec<String> = [
        (
            c.sequences * usize::from(with_sequences),
            "sequence",
            "sequences",
        ),
        (c.cards, "scene card", "scene cards"),
        (c.beats, "beat", "beats"),
    ]
    .iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, one, many)| plural(*n, one, many))
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// The ancestors of a page that starts inside a container, as the "continued" strip names them.
fn join_strip(parts: &[Option<&str>]) -> Option<String> {
    let v: Vec<&str> = parts.iter().flatten().copied().collect();
    (!v.is_empty()).then(|| v.join(" \u{203a} "))
}

fn empty_line(text: &str) -> Block {
    Block::Text(Txt::new(text).size(8.4).color(MUTED).after(4.0))
}

/// What the pieces of one board need to know while they are built.
struct Build<'a> {
    o: &'a Opts,
    cards: bool,
}

impl Build<'_> {
    /// Items of one run: the first row travels with the heading before it, the rest follow.
    fn items(
        &self,
        nodes: &[&Node],
        nested: bool,
        parked: bool,
        strip: Option<String>,
    ) -> (Vec<Block>, Vec<Block>) {
        let layout = if self.cards {
            BoardLayout::Cards {
                columns: BOARD_COLUMNS,
            }
        } else {
            BoardLayout::List { nested }
        };
        let take = if self.cards { BOARD_COLUMNS } else { 1 };
        let make = |part: &[&Node]| -> Vec<Block> {
            if part.is_empty() {
                return Vec::new();
            }
            vec![Block::Board(Board::Items(BoardItems {
                items: part.iter().map(|n| pdf_item(n, self.o)).collect(),
                layout: layout.clone(),
                parked,
                strip: strip.clone(),
            }))]
        };
        let at = nodes.len().min(take);
        (make(&nodes[..at]), make(&nodes[at..]))
    }

    /// The children of an act or an area, in data order: sequences with their bands, and runs of
    /// loose beats and cards. `head` is the bar (and episode bar) that opens the container; it
    /// stays with the first thing under it.
    fn container(
        &self,
        head: Vec<Block>,
        children: &[Node],
        strip: Option<String>,
        parked: bool,
        out: &mut Vec<Block>,
    ) {
        let has_sequences = children.iter().any(|c| c.kind == Kind::Sequence);
        let mut lead = head;
        let mut i = 0;
        if children.is_empty() {
            lead.push(empty_line("No sequences, beats or scene cards yet."));
        }
        while i < children.len() {
            let n = &children[i];
            let mut tail: Vec<Block> = Vec::new();
            if n.kind == Kind::Sequence {
                i += 1;
                let seq_strip =
                    join_strip(&[strip.as_deref(), Some(&format!("Sequence: {}", n.title))]);
                lead.push(Block::Board(Board::Sequence {
                    title: n.title.clone(),
                    hint: inside(&n.children, false),
                    extras: pdf_extras(n, self.o),
                    strip: strip.clone(),
                }));
                if n.children.is_empty() {
                    lead.push(empty_line("No beats or scene cards yet."));
                } else {
                    let refs: Vec<&Node> = n.children.iter().collect();
                    let (first, rest) = self.items(&refs, true, parked, seq_strip);
                    lead.extend(first);
                    tail = rest;
                }
            } else {
                let start = i;
                while i < children.len() && children[i].kind != Kind::Sequence {
                    i += 1;
                }
                if has_sequences {
                    lead.push(Block::Board(Board::Label {
                        text: "Not in a sequence".into(),
                        strip: strip.clone(),
                    }));
                }
                let refs: Vec<&Node> = children[start..i].iter().collect();
                let (first, rest) = self.items(&refs, false, parked, strip.clone());
                lead.extend(first);
                tail = rest;
            }
            out.push(Block::Together(std::mem::take(&mut lead)));
            out.extend(tail);
        }
        if !lead.is_empty() {
            out.push(Block::Together(lead));
        }
    }

    fn board(&self, b: &Tray, multi: bool, out: &mut Vec<Block>) {
        let episode = multi.then(|| {
            b.episode
                .clone()
                .unwrap_or_else(|| "Film Beat Board".into())
        });
        let mut episode_bar = episode.as_ref().map(|title| {
            vec![Block::Board(Board::Episode {
                eyebrow: b.episode.as_ref().map(|_| "Episode".to_string()),
                title: title.clone(),
            })]
        });
        for act in &b.acts {
            let mut head = episode_bar.take().unwrap_or_default();
            head.push(Block::Board(Board::Act {
                title: act.title.clone(),
                hint: inside(&act.children, true),
                extras: pdf_extras(act, self.o),
            }));
            let strip = join_strip(&[episode.as_deref(), Some(&act.title)]);
            self.container(head, &act.children, strip, false, out);
        }
        let areas: [(&str, &str, &Vec<Node>, bool); 2] = [
            ("Unassigned", "Not placed in an act", &b.unassigned, false),
            (
                "Parking Lot",
                "Held outside the board order \u{b7} not numbered",
                &b.parking,
                true,
            ),
        ];
        for (title, hint, items, parked) in areas {
            if items.is_empty() {
                continue;
            }
            let mut head = episode_bar.take().unwrap_or_default();
            head.push(Block::Board(Board::Area {
                title: title.into(),
                hint: Some(hint.into()),
                parking: parked,
                strip: join_strip(&[episode.as_deref()]),
            }));
            let strip = join_strip(&[episode.as_deref(), Some(title)]);
            self.container(head, items, strip, parked, out);
        }
    }
}

// ------------------------------------------------------------------ text / csv

fn text_lines(nodes: &[Node], depth: usize, o: &Opts, out: &mut String) {
    let pad = "  ".repeat(depth);
    for n in nodes {
        match n.kind {
            Kind::Act => out.push_str(&format!("\n{pad}{}\n", n.title.to_uppercase())),
            Kind::Sequence => out.push_str(&format!("\n{pad}SEQUENCE: {}\n", n.title)),
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
            row.push(
                n.comments
                    .iter()
                    .flat_map(thread_lines)
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
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
            "Choose what to export: the current Beat Board, selected items or the entire project.",
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
    let boards: Vec<Tray> = boards
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
                "None of the selected items are on the Beat Board any more. Select items and try again."
            } else {
                "The Beat Board is empty, so there is nothing to export yet."
            },
        ));
    }
    let multi = boards.len() > 1;

    // On the board (acts and Unassigned) and in the Parking Lot, counted apart.
    let mut counts = Counts::default();
    let mut parked_counts = Counts::default();
    for b in &boards {
        count(&b.acts, &mut counts);
        count(&b.unassigned, &mut counts);
        count(&b.parking, &mut parked_counts);
    }
    let document = if board_layout {
        "Beat Board"
    } else {
        "Beat Board Outline"
    };
    let scope_label = match scope {
        "selected" => "Selected items only".to_string(),
        "project" => "Entire project".to_string(),
        _ => "Current Beat Board".to_string(),
    };
    let source_label = match (&boards[0].episode, multi) {
        (Some(ep), false) => format!("Beat Board: {ep}"),
        _ => "Current Beat Board".to_string(),
    };

    // ---- PDF (PDF design package v2: act bars, sequence bands, number tiles, boxed cards)
    let mut doc = Document::new(document).running_header(Some(running_header(&project, document)));
    {
        let mut mast = Masthead::new(document).eyebrow(
            if board_layout {
                "Story · Beat Board"
            } else {
                "Story · Outline"
            },
            pal::SLATE,
        );
        mast.title_size = 28.0;
        if let Some(p) = nb(&project) {
            mast.subtitle = Some(p.to_string());
        }
        mast.lines.push(source_label.clone());
        let mut lines = vec![
            (scope_label.clone(), true),
            (format!("Exported {}", today_label()), false),
        ];
        if scope == "selected" {
            lines.push(("Acts and sequences shown for context".to_string(), false));
        }
        mast.side = Some(Side::Panel { lines });
        doc.push(Block::Masthead(mast));
    }
    let tile = |label: &str, on: usize, parked: usize| {
        (on + parked > 0).then(|| BoardCount {
            label: label.to_string(),
            value: on.to_string(),
            sub: (parked > 0).then(|| format!("+ {parked} in Parking Lot")),
        })
    };
    let tiles: Vec<BoardCount> = [
        tile("Acts", counts.acts, 0),
        tile("Sequences", counts.sequences, 0),
        tile("Scene cards", counts.cards, parked_counts.cards),
        tile("Beats", counts.beats, parked_counts.beats),
    ]
    .into_iter()
    .flatten()
    .collect();
    doc.push(Block::Board(Board::Counts(tiles)));
    let parked_any = boards.iter().any(|b| !b.parking.is_empty());
    doc.push(Block::Board(Board::Key(BoardKey {
        notes: o.notes,
        comments: o.comments,
        includes: [
            o.notes.then(|| "Notes".to_string()),
            o.comments.then(|| "Comments".to_string()),
            parking.then(|| {
                if parked_any {
                    "Parking Lot".to_string()
                } else {
                    "Parking Lot (none)".to_string()
                }
            }),
        ]
        .into_iter()
        .flatten()
        .collect(),
        foot: Some("Private notes excluded.".into()),
    })));
    let build = Build {
        o: &o,
        cards: board_layout,
    };
    for (i, b) in boards.iter().enumerate() {
        if board_layout && i > 0 {
            doc.push(Block::PageBreak);
        }
        build.board(b, multi, &mut doc.blocks);
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
                    .unwrap_or_else(|| "Film Beat Board".into())
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
    let mut table = Table::new("Beat Board", cols);
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
        (
            counts.cards + parked_counts.cards,
            "scene card",
            "scene cards",
        ),
        (counts.beats + parked_counts.beats, "beat", "beats"),
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
