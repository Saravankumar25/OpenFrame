//! Screenplay page layout: the single pagination engine behind screenplay PDF
//! export, the plain-text writer and the print preview (FSD §123: "print
//! preview equals the generated PDF").
//!
//! Industry-standard layout for US Letter with 12-pt Courier (10 characters
//! per inch, 6 lines per inch):
//!
//! | element        | left edge | width    |
//! |----------------|-----------|----------|
//! | heading/action | 1.5"      | 60 chars |
//! | character cue  | 3.7"      | 38 chars |
//! | parenthetical  | 3.1"      | 26 chars |
//! | dialogue       | 2.5"      | 35 chars |
//! | transition     | right-aligned to 7.5" |
//!
//! 55 body lines per page starting 1" from the top. Page numbers ("2.") sit
//! top right from the second script page. Dialogue split across a page gets
//! `(MORE)` at the foot and `NAME (CONT'D)` at the top of the next page.
//! Headings are never left alone at the bottom of a page.

use serde::Serialize;

use crate::model::{Element, ElementKind, ScreenplayDoc, character_name};

pub const PAGE_WIDTH_IN: f32 = 8.5;
pub const PAGE_HEIGHT_IN: f32 = 11.0;
pub const LEFT_MARGIN_IN: f32 = 1.5;
pub const RIGHT_EDGE_IN: f32 = 7.5;
pub const TOP_MARGIN_IN: f32 = 1.0;
pub const LINE_HEIGHT_IN: f32 = 1.0 / 6.0;
pub const CHAR_WIDTH_IN: f32 = 0.1;
pub const LINES_PER_PAGE: usize = 55;

const ACTION_X: f32 = 1.5;
const ACTION_W: usize = 60;
const CUE_X: f32 = 3.7;
const CUE_W: usize = 38;
const PAREN_X: f32 = 3.1;
const PAREN_W: usize = 26;
const DIALOGUE_X: f32 = 2.5;
const DIALOGUE_W: usize = 35;
// Dual dialogue columns.
const DUAL_LEFT_X: f32 = 1.5;
const DUAL_RIGHT_X: f32 = 4.6;
const DUAL_W: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Heading,
    Action,
    Character,
    Parenthetical,
    Dialogue,
    Transition,
    Centered,
    Shot,
    Lyric,
    Note,
    More,
    PageNumber,
    SceneNumber,
    RevisionMark,
    TitleMain,
    TitleText,
}

impl LineStyle {
    /// Oblique (italic) Courier in the PDF.
    pub fn oblique(self) -> bool {
        matches!(self, LineStyle::Lyric | LineStyle::Note)
    }
    pub fn bold(self) -> bool {
        matches!(self, LineStyle::TitleMain)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageLine {
    /// Body row, 0 = first line 1" from the top. Negative rows are header space.
    pub row: i32,
    /// Left edge of the text in inches from the page's left edge.
    pub x_in: f32,
    pub text: String,
    pub style: LineStyle,
}

impl PageLine {
    /// Baseline distance from the top of the page, in inches.
    pub fn y_in(&self) -> f32 {
        TOP_MARGIN_IN + (self.row as f32 + 1.0) * LINE_HEIGHT_IN - 0.035
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaidOutPage {
    pub title_page: bool,
    /// Printed page number (None for the title page).
    pub number: Option<u32>,
    pub lines: Vec<PageLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub pages: Vec<LaidOutPage>,
}

impl Layout {
    pub fn script_page_count(&self) -> usize {
        self.pages.iter().filter(|p| !p.title_page).count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOptions {
    pub title_page: bool,
    pub scene_numbers: bool,
    pub include_notes: bool,
    /// Asterisks in the right margin beside revised lines.
    pub revision_marks: bool,
    /// Revision line on the title page.
    pub revision_info: bool,
    pub page_numbers: bool,
    /// Split into pages (false = one continuous page, used by the TXT writer).
    pub paginate: bool,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            title_page: true,
            scene_numbers: false,
            include_notes: false,
            revision_marks: false,
            revision_info: true,
            page_numbers: true,
            paginate: true,
        }
    }
}

/// Word-wrap `text` to `width` characters, keeping explicit line breaks.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let para = para.trim_end();
        if para.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut len = 0usize;
        for word in para.split(' ') {
            let wlen = word.chars().count();
            if len > 0 && len + 1 + wlen > width {
                out.push(std::mem::take(&mut line));
                len = 0;
            }
            if wlen > width {
                // Hard-break very long words.
                let chars: Vec<char> = word.chars().collect();
                for chunk in chars.chunks(width) {
                    if len > 0 {
                        out.push(std::mem::take(&mut line));
                    }
                    line = chunk.iter().collect();
                    len = chunk.len();
                }
                continue;
            }
            if len > 0 {
                line.push(' ');
                len += 1;
            }
            line.push_str(word);
            len += wlen;
        }
        out.push(line);
    }
    out
}

#[derive(Debug, Clone)]
struct BlockLine {
    x: f32,
    text: String,
    style: LineStyle,
    revised: bool,
    /// Row offset within the block (dual columns share rows).
    row: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Heading,
    Action,
    Speech,
    Dual,
    Transition,
    PageBreak,
}

#[derive(Debug, Clone)]
struct Block {
    kind: BlockKind,
    lines: Vec<BlockLine>,
    rows: usize,
    space_before: usize,
    /// Speaker for (MORE)/(CONT'D).
    speaker: Option<String>,
    scene_number: Option<String>,
}

fn single(kind: BlockKind, lines: Vec<BlockLine>, space_before: usize) -> Block {
    let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
    Block {
        kind,
        lines,
        rows,
        space_before,
        speaker: None,
        scene_number: None,
    }
}

fn lines_for(
    x: f32,
    width: usize,
    text: &str,
    style: LineStyle,
    revised: bool,
    start_row: usize,
) -> Vec<BlockLine> {
    wrap(text, width)
        .into_iter()
        .enumerate()
        .map(|(i, t)| BlockLine {
            x,
            text: t,
            style,
            revised,
            row: start_row + i,
        })
        .collect()
}

fn speech_lines(
    elements: &[&Element],
    cue_x: f32,
    paren_x: f32,
    dia_x: f32,
    dia_w: usize,
    paren_w: usize,
) -> Vec<BlockLine> {
    let mut out = Vec::new();
    let mut row = 0usize;
    for e in elements {
        let revised = e.revision.is_some();
        let (x, w, style) = match e.kind {
            ElementKind::Character => (cue_x, CUE_W, LineStyle::Character),
            ElementKind::Parenthetical => (paren_x, paren_w, LineStyle::Parenthetical),
            ElementKind::Lyric => (dia_x, dia_w, LineStyle::Lyric),
            _ => (dia_x, dia_w, LineStyle::Dialogue),
        };
        let text = if e.kind == ElementKind::Character {
            e.text.trim().trim_end_matches('^').trim().to_uppercase()
        } else {
            e.text.clone()
        };
        let ls = lines_for(x, w, &text, style, revised, row);
        row += ls.len();
        out.extend(ls);
    }
    out
}

fn build_blocks(doc: &ScreenplayDoc, opts: &LayoutOptions) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut number = 0usize;
    for scene in &doc.scenes {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let mut b = single(
                BlockKind::Heading,
                lines_for(
                    ACTION_X,
                    ACTION_W,
                    &scene.heading.to_uppercase(),
                    LineStyle::Heading,
                    false,
                    0,
                ),
                1,
            );
            b.scene_number = Some(scene.number.clone().unwrap_or_else(|| number.to_string()));
            blocks.push(b);
        }
        let els = &scene.elements;
        let mut i = 0;
        while i < els.len() {
            let e = &els[i];
            match e.kind {
                ElementKind::Character
                | ElementKind::Parenthetical
                | ElementKind::Dialogue
                | ElementKind::Lyric => {
                    // Gather one speech (cue + following speech elements), or a dual run.
                    let dual = e.dual;
                    let mut j = i + 1;
                    while j < els.len()
                        && matches!(
                            els[j].kind,
                            ElementKind::Parenthetical
                                | ElementKind::Dialogue
                                | ElementKind::Lyric
                                | ElementKind::Character
                        )
                        && (els[j].kind != ElementKind::Character || (dual && els[j].dual))
                        && els[j].dual == dual
                        && !(els[j].kind == ElementKind::Character
                            && dual
                            && count_cues(&els[i..j]) >= 2)
                    {
                        j += 1;
                    }
                    let group: Vec<&Element> = els[i..j]
                        .iter()
                        .filter(|e| opts.include_notes || e.kind != ElementKind::Note)
                        .collect();
                    let second = group
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, e)| e.kind == ElementKind::Character)
                        .map(|(k, _)| k);
                    if let (true, Some(split)) = (dual, second) {
                        let left = speech_lines(
                            &group[..split],
                            DUAL_LEFT_X + 0.8,
                            DUAL_LEFT_X + 0.4,
                            DUAL_LEFT_X,
                            DUAL_W,
                            DUAL_W - 4,
                        );
                        let right = speech_lines(
                            &group[split..],
                            DUAL_RIGHT_X + 0.8,
                            DUAL_RIGHT_X + 0.4,
                            DUAL_RIGHT_X,
                            DUAL_W,
                            DUAL_W - 4,
                        );
                        let mut lines = left;
                        lines.extend(right);
                        blocks.push(single(BlockKind::Dual, lines, 1));
                    } else {
                        let speaker = group
                            .iter()
                            .find(|e| e.kind == ElementKind::Character)
                            .map(|e| character_name(&e.text));
                        let mut b = single(
                            BlockKind::Speech,
                            speech_lines(&group, CUE_X, PAREN_X, DIALOGUE_X, DIALOGUE_W, PAREN_W),
                            1,
                        );
                        b.speaker = speaker;
                        blocks.push(b);
                    }
                    i = j;
                    continue;
                }
                ElementKind::Transition => {
                    let t = e.text.trim().to_uppercase();
                    let len = t.chars().count() as f32;
                    let x = (RIGHT_EDGE_IN - len * CHAR_WIDTH_IN).max(ACTION_X);
                    blocks.push(single(
                        BlockKind::Transition,
                        lines_for(
                            x,
                            ACTION_W,
                            &t,
                            LineStyle::Transition,
                            e.revision.is_some(),
                            0,
                        ),
                        1,
                    ));
                }
                ElementKind::Centered => {
                    let lines = wrap(&e.text, ACTION_W)
                        .into_iter()
                        .enumerate()
                        .map(|(r, t)| {
                            let len = t.chars().count() as f32;
                            BlockLine {
                                x: ACTION_X
                                    + (ACTION_W as f32 * CHAR_WIDTH_IN - len * CHAR_WIDTH_IN) / 2.0,
                                text: t,
                                style: LineStyle::Centered,
                                revised: e.revision.is_some(),
                                row: r,
                            }
                        })
                        .collect();
                    blocks.push(single(BlockKind::Action, lines, 1));
                }
                ElementKind::Action | ElementKind::Shot => {
                    let style = if e.kind == ElementKind::Shot {
                        LineStyle::Shot
                    } else {
                        LineStyle::Action
                    };
                    let text = if e.kind == ElementKind::Shot {
                        e.text.to_uppercase()
                    } else {
                        e.text.clone()
                    };
                    blocks.push(single(
                        BlockKind::Action,
                        lines_for(ACTION_X, ACTION_W, &text, style, e.revision.is_some(), 0),
                        1,
                    ));
                }
                ElementKind::Note => {
                    if opts.include_notes {
                        let text = format!("[Note: {}]", e.text.trim());
                        blocks.push(single(
                            BlockKind::Action,
                            lines_for(ACTION_X, ACTION_W, &text, LineStyle::Note, false, 0),
                            1,
                        ));
                    }
                }
                ElementKind::PageBreak => blocks.push(single(BlockKind::PageBreak, vec![], 0)),
                ElementKind::Section | ElementKind::Synopsis => {}
            }
            i += 1;
        }
    }
    blocks
}

fn count_cues(els: &[Element]) -> usize {
    els.iter()
        .filter(|e| e.kind == ElementKind::Character)
        .count()
}

struct Paginator {
    opts: LayoutOptions,
    lpp: usize,
    pages: Vec<LaidOutPage>,
    current: Vec<PageLine>,
    row: usize,
}

impl Paginator {
    fn new_page(&mut self) {
        let lines = std::mem::take(&mut self.current);
        self.pages.push(LaidOutPage {
            title_page: false,
            number: None,
            lines,
        });
        self.row = 0;
    }
    fn place(&mut self, block_lines: &[BlockLine], scene_number: Option<&str>) {
        let base = self.row as i32;
        let mut max_row = 0usize;
        for l in block_lines {
            let row = base + l.row as i32;
            self.current.push(PageLine {
                row,
                x_in: l.x,
                text: l.text.clone(),
                style: l.style,
            });
            if l.revised && self.opts.revision_marks {
                self.current.push(PageLine {
                    row,
                    x_in: RIGHT_EDGE_IN + 0.2,
                    text: "*".into(),
                    style: LineStyle::RevisionMark,
                });
            }
            max_row = max_row.max(l.row + 1);
        }
        if let (Some(n), true) = (scene_number, self.opts.scene_numbers) {
            let len = n.chars().count() as f32;
            self.current.push(PageLine {
                row: base,
                x_in: ACTION_X - 0.5 - len * CHAR_WIDTH_IN,
                text: n.to_string(),
                style: LineStyle::SceneNumber,
            });
            self.current.push(PageLine {
                row: base,
                x_in: RIGHT_EDGE_IN + 0.1,
                text: n.to_string(),
                style: LineStyle::SceneNumber,
            });
        }
        self.row += max_row;
    }
    fn remaining(&self) -> usize {
        self.lpp.saturating_sub(self.row)
    }
}

/// Lay out the document into pages.
pub fn layout(doc: &ScreenplayDoc, opts: &LayoutOptions) -> Layout {
    let blocks = build_blocks(doc, opts);
    let lpp = if opts.paginate {
        LINES_PER_PAGE
    } else {
        usize::MAX / 4
    };
    let mut p = Paginator {
        opts: *opts,
        lpp,
        pages: Vec::new(),
        current: Vec::new(),
        row: 0,
    };

    for (bi, block) in blocks.iter().enumerate() {
        if block.kind == BlockKind::PageBreak {
            if p.row > 0 {
                p.new_page();
            }
            continue;
        }
        let space = if p.row == 0 { 0 } else { block.space_before };
        let mut needed = space + block.rows;
        if block.kind == BlockKind::Heading {
            // Keep the heading with at least two lines of what follows.
            if let Some(next) = blocks
                .get(bi + 1)
                .filter(|n| n.kind != BlockKind::PageBreak && n.kind != BlockKind::Heading)
            {
                needed += next.space_before + next.rows.min(2);
            }
        }
        if needed <= p.remaining() {
            p.row += space;
            p.place(&block.lines, block.scene_number.as_deref());
            continue;
        }
        // Does not fit: try to split, otherwise move to the next page.
        let available = p.remaining().saturating_sub(space);
        match block.kind {
            BlockKind::Action
                if block.rows >= 4 && available >= 2 && block.rows - available >= 2 =>
            {
                p.row += space;
                let (a, b) = split_rows(&block.lines, available);
                p.place(&a, None);
                p.new_page();
                place_overflowing(&mut p, &b, None);
            }
            BlockKind::Speech if available >= 4 && block.rows >= 4 => {
                // Keep cue + at least two lines, reserve one row for (MORE),
                // leave at least one dialogue line for the next page, and split
                // only after a dialogue line.
                let max_first = available - 1;
                let mut split = None;
                for k in (3..=max_first.min(block.rows - 1)).rev() {
                    let last = block.lines.iter().rfind(|l| l.row == k - 1);
                    let next = block.lines.iter().find(|l| l.row == k);
                    let last_is_dialogue = last
                        .map(|l| l.style == LineStyle::Dialogue || l.style == LineStyle::Lyric)
                        .unwrap_or(false);
                    let next_is_speech = next
                        .map(|l| l.style != LineStyle::Character)
                        .unwrap_or(false);
                    if last_is_dialogue && next_is_speech {
                        split = Some(k);
                        break;
                    }
                }
                if let Some(k) = split {
                    p.row += space;
                    let (a, b) = split_rows(&block.lines, k);
                    p.place(&a, None);
                    let more_row = p.row as i32;
                    p.current.push(PageLine {
                        row: more_row,
                        x_in: CUE_X,
                        text: "(MORE)".into(),
                        style: LineStyle::More,
                    });
                    p.new_page();
                    let speaker = block.speaker.clone().unwrap_or_default();
                    let mut cont = vec![BlockLine {
                        x: CUE_X,
                        text: format!("{speaker} (CONT'D)"),
                        style: LineStyle::Character,
                        revised: false,
                        row: 0,
                    }];
                    cont.extend(b.into_iter().map(|mut l| {
                        l.row += 1;
                        l
                    }));
                    place_overflowing(&mut p, &cont, None);
                } else {
                    p.new_page();
                    place_overflowing(&mut p, &block.lines, block.scene_number.as_deref());
                }
            }
            _ => {
                if p.row > 0 {
                    p.new_page();
                }
                place_overflowing(&mut p, &block.lines, block.scene_number.as_deref());
            }
        }
    }
    if !p.current.is_empty() || p.pages.is_empty() {
        p.new_page();
    }

    // Number script pages and add page numbers (top right, from page 2).
    let mut pages = p.pages;
    for (i, page) in pages.iter_mut().enumerate() {
        let n = (i + 1) as u32;
        page.number = Some(n);
        if opts.page_numbers && opts.paginate && n >= 2 {
            let label = format!("{n}.");
            let len = label.chars().count() as f32;
            page.lines.push(PageLine {
                row: -3,
                x_in: RIGHT_EDGE_IN - len * CHAR_WIDTH_IN,
                text: label,
                style: LineStyle::PageNumber,
            });
        }
        page.lines
            .sort_by(|a, b| a.row.cmp(&b.row).then(a.x_in.total_cmp(&b.x_in)));
    }
    if opts.title_page && (!doc.title_page.is_empty()) {
        pages.insert(0, title_page(doc, opts));
    }
    Layout { pages }
}

/// Place lines that may be longer than a page, breaking at page boundaries.
fn place_overflowing(p: &mut Paginator, lines: &[BlockLine], scene_number: Option<&str>) {
    let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
    if rows <= p.remaining() {
        p.place(lines, scene_number);
        return;
    }
    let mut rest: Vec<BlockLine> = lines.to_vec();
    let mut first = true;
    while !rest.is_empty() {
        let avail = p.remaining().max(1);
        let (a, b) = split_rows(&rest, avail);
        p.place(&a, if first { scene_number } else { None });
        first = false;
        rest = b;
        if !rest.is_empty() {
            p.new_page();
        }
    }
}

/// Split block lines into rows `< k` and rows `>= k` (renumbered from 0).
fn split_rows(lines: &[BlockLine], k: usize) -> (Vec<BlockLine>, Vec<BlockLine>) {
    let a = lines.iter().filter(|l| l.row < k).cloned().collect();
    let b = lines
        .iter()
        .filter(|l| l.row >= k)
        .cloned()
        .map(|mut l| {
            l.row -= k;
            l
        })
        .collect();
    (a, b)
}

fn centered(text: &str, row: i32, style: LineStyle) -> PageLine {
    let len = text.chars().count() as f32;
    PageLine {
        row,
        x_in: ((PAGE_WIDTH_IN - len * CHAR_WIDTH_IN) / 2.0).max(1.0),
        text: text.to_string(),
        style,
    }
}

fn title_page(doc: &ScreenplayDoc, opts: &LayoutOptions) -> LaidOutPage {
    let tp = &doc.title_page;
    let mut lines = Vec::new();
    let mut row = 18;
    if let Some(title) = tp.title() {
        for t in title.split('\n').filter(|t| !t.trim().is_empty()) {
            for w in wrap(&t.trim().to_uppercase(), 50) {
                lines.push(centered(&w, row, LineStyle::TitleMain));
                row += 1;
            }
        }
    }
    row += 3;
    let credit = tp.get("Credit").unwrap_or("Written by");
    if tp.author().is_some() {
        lines.push(centered(credit.trim(), row, LineStyle::TitleText));
        row += 2;
        for a in tp
            .author()
            .unwrap_or_default()
            .split('\n')
            .filter(|a| !a.trim().is_empty())
        {
            lines.push(centered(a.trim(), row, LineStyle::TitleText));
            row += 1;
        }
    }
    if let Some(src) = tp.get("Source") {
        row += 2;
        for s in src.split('\n').filter(|s| !s.trim().is_empty()) {
            for w in wrap(s.trim(), 50) {
                lines.push(centered(&w, row, LineStyle::TitleText));
                row += 1;
            }
        }
    }
    // Bottom-left: notes then contact. Bottom-right: draft / revision lines.
    let mut left: Vec<String> = Vec::new();
    if let Some(n) = tp.get("Notes") {
        left.extend(n.split('\n').flat_map(|l| wrap(l.trim(), 34)));
        left.push(String::new());
    }
    if let Some(c) = tp.get("Contact") {
        left.extend(c.split('\n').flat_map(|l| wrap(l.trim(), 34)));
    }
    if let Some(c) = tp.get("Copyright") {
        left.push(c.trim().to_string());
    }
    let mut right: Vec<String> = Vec::new();
    for key in ["Draft", "Draft date"] {
        if let Some(d) = tp.get(key) {
            right.extend(d.split('\n').map(|l| l.trim().to_string()));
        }
    }
    if let (true, Some(r)) = (opts.revision_info, tp.get("Revision")) {
        right.extend(r.split('\n').map(|l| l.trim().to_string()));
    }
    let left_start = (LINES_PER_PAGE as i32 - left.len() as i32).max(row + 2);
    for (i, l) in left.iter().enumerate() {
        if !l.is_empty() {
            lines.push(PageLine {
                row: left_start + i as i32,
                x_in: LEFT_MARGIN_IN,
                text: l.clone(),
                style: LineStyle::TitleText,
            });
        }
    }
    let right_start = (LINES_PER_PAGE as i32 - right.len() as i32).max(row + 2);
    for (i, r) in right.iter().enumerate() {
        let len = r.chars().count() as f32;
        lines.push(PageLine {
            row: right_start + i as i32,
            x_in: RIGHT_EDGE_IN - len * CHAR_WIDTH_IN,
            text: r.clone(),
            style: LineStyle::TitleText,
        });
    }
    lines.sort_by(|a, b| a.row.cmp(&b.row).then(a.x_in.total_cmp(&b.x_in)));
    LaidOutPage {
        title_page: true,
        number: None,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scene;

    #[test]
    fn wrapping_respects_width_and_breaks() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("a\nb", 10), vec!["a", "b"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn long_dialogue_splits_with_more_and_contd() {
        let mut doc = ScreenplayDoc::default();
        let mut s = Scene::new("INT. HALL - NIGHT");
        s.elements
            .push(Element::new(ElementKind::Action, "x ".repeat(29 * 48)));
        s.elements
            .push(Element::new(ElementKind::Character, "MEERA"));
        s.elements
            .push(Element::new(ElementKind::Dialogue, "Talk. ".repeat(80)));
        doc.scenes.push(s);
        let lay = layout(
            &doc,
            &LayoutOptions {
                title_page: false,
                ..Default::default()
            },
        );
        assert!(lay.pages.len() >= 2);
        let all: Vec<&PageLine> = lay.pages.iter().flat_map(|p| p.lines.iter()).collect();
        assert!(all.iter().any(|l| l.style == LineStyle::More));
        assert!(all.iter().any(|l| l.text == "MEERA (CONT'D)"));
        for page in &lay.pages {
            assert!(page.lines.iter().all(|l| l.row < LINES_PER_PAGE as i32));
        }
        assert!(
            lay.pages[1]
                .lines
                .iter()
                .any(|l| l.style == LineStyle::PageNumber && l.text == "2.")
        );
        assert!(
            !lay.pages[0]
                .lines
                .iter()
                .any(|l| l.style == LineStyle::PageNumber)
        );
    }

    #[test]
    fn heading_is_never_orphaned() {
        let mut doc = ScreenplayDoc::default();
        let mut s = Scene::new("INT. A - DAY");
        // Fill exactly up to the last two rows of the page.
        s.elements.push(Element::new(
            ElementKind::Action,
            (0..52).map(|_| "line").collect::<Vec<_>>().join("\n"),
        ));
        doc.scenes.push(s);
        let mut s2 = Scene::new("INT. B - DAY");
        s2.elements
            .push(Element::new(ElementKind::Action, "Something happens."));
        doc.scenes.push(s2);
        let lay = layout(
            &doc,
            &LayoutOptions {
                title_page: false,
                ..Default::default()
            },
        );
        let heading_page = lay
            .pages
            .iter()
            .position(|p| p.lines.iter().any(|l| l.text == "INT. B - DAY"))
            .unwrap();
        let action_page = lay
            .pages
            .iter()
            .position(|p| p.lines.iter().any(|l| l.text == "Something happens."))
            .unwrap();
        assert_eq!(heading_page, action_page);
    }
}
