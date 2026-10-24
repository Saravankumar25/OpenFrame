//! Screenplay page layout: the single pagination engine behind screenplay PDF
//! export, the print preview, the editor's page view and page facts, and the
//! plain-text writer (FSD §123: "print preview equals the generated PDF").
//!
//! Element positions, widths, alignment, case and spacing come from the
//! element rule table ([`crate::rules`]). Page geometry comes from the
//! screenplay's [`PageSetup`]: paper (US Letter or A4, ADR-0008 decision 4),
//! Courier size and line spacing. The standard preset is US Letter, 12 pt
//! Courier, single spacing:
//!
//! | element        | left edge | width    |
//! |----------------|-----------|----------|
//! | heading/action | 1.5"      | 60 chars |
//! | character cue  | 3.7"      | 38 chars |
//! | parenthetical  | 3.1"      | 26 chars |
//! | dialogue       | 2.5"      | 35 chars |
//! | transition     | right-aligned to 7.5" |
//!
//! Body text starts 1" from the top and ends 10/12" above the bottom edge
//! (55 lines on Letter, 59 on A4). A4 keeps the same columns, so a script
//! wraps identically and only gains lines per page.
//!
//! Page numbers ("1.", "2.") sit top right on every script page; the page
//! setup can leave the first script page unnumbered (`number_first_page`),
//! and the title page never has one. An optional header (left) and
//! footer (centred) carry placeholders `{title}`, `{draft}`, `{revision}`,
//! `{date}` and `{page}`. Keep-together rules: a scene heading stays with the
//! start of what follows, a transition with the next scene heading, a shot
//! with its description, a character cue with at least two lines of its
//! speech, a parenthetical with the dialogue after it, and an action
//! paragraph or dialogue split leaves at least two lines on each page.
//! Every page split of one speech, however tall the speech, gets `(MORE)` at
//! the foot (after a dialogue line) and `NAME (CONT'D)` at the top of the
//! next page; two separate speeches by the same character never do, and an
//! imported `(CONT'D)` is never doubled. The markers exist only in the layout.
//! An explicit page break starts a new page.
//!
//! Vertical gaps have one owner: the `space_before` of the next element (rule
//! table). One blank line is 24 pt between baselines, two are 36 pt; a scene
//! heading carries two (after a transition and after ordinary content alike).

use serde::{Deserialize, Serialize};

use crate::model::{CustomStyle, Element, ElementKind, ScreenplayDoc};
use crate::rules::{self, Align, ElementRule};

// Standard preset (US Letter, 12 pt Courier, single spacing). Kept for callers that draw
// their own pages on the standard geometry (sides cover pages).
pub const PAGE_WIDTH_IN: f32 = 8.5;
pub const PAGE_HEIGHT_IN: f32 = 11.0;
pub const LEFT_MARGIN_IN: f32 = 1.5;
pub const RIGHT_EDGE_IN: f32 = 7.5;
pub const TOP_MARGIN_IN: f32 = 1.0;
pub const BOTTOM_MARGIN_IN: f32 = 10.0 / 12.0;
pub const LINE_HEIGHT_IN: f32 = 1.0 / 6.0;
pub const CHAR_WIDTH_IN: f32 = 0.1;
pub const LINES_PER_PAGE: usize = 55;

// Dual dialogue columns (inches).
const DUAL_LEFT_X: f32 = 1.5;
const DUAL_RIGHT_X: f32 = 4.6;
const DUAL_W_IN: f32 = 2.8;

/// Paper size (ADR-0008 decision 4: an explicit setting, never guessed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Paper {
    #[default]
    Letter,
    A4,
}

impl Paper {
    /// Width and height in inches.
    pub fn size_in(self) -> (f32, f32) {
        match self {
            Paper::Letter => (8.5, 11.0),
            Paper::A4 => (8.27, 11.69),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Paper::Letter => "US Letter",
            Paper::A4 => "A4",
        }
    }
}

/// Line spacing presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineSpacing {
    #[default]
    Single,
    OneAndHalf,
    Double,
}

impl LineSpacing {
    pub fn factor(self) -> f32 {
        match self {
            LineSpacing::Single => 1.0,
            LineSpacing::OneAndHalf => 1.5,
            LineSpacing::Double => 2.0,
        }
    }
}

/// Fonts the screenplay PDF can print without embedding (ADR-0008; embedded
/// fonts arrive with ADR-0016).
pub const FONTS: &[&str] = &["Courier"];
/// Allowed Courier sizes in points.
pub const MIN_FONT_SIZE: u8 = 10;
pub const MAX_FONT_SIZE: u8 = 14;
/// Characters allowed in a header or footer.
pub const MAX_HEADER_CHARS: usize = 120;

/// Page setup saved with the screenplay. The default is the standard
/// screenplay preset and reproduces the layout from before page setup existed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageSetup {
    pub paper: Paper,
    pub font: String,
    pub font_size: u8,
    pub line_spacing: LineSpacing,
    /// Number printed on the first script page.
    pub first_page_number: u32,
    /// Print the page number on the first script page. Off is the common spec-script
    /// convention (the first page unnumbered, the next one "2."). The title page never has one.
    pub number_first_page: bool,
    /// Header text (left), with placeholders.
    pub header: String,
    /// Footer text (centred), with placeholders.
    pub footer: String,
    /// Print the header and footer on the first script page too.
    pub header_on_first_page: bool,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            paper: Paper::Letter,
            font: "Courier".into(),
            font_size: 12,
            line_spacing: LineSpacing::Single,
            first_page_number: 1,
            number_first_page: true,
            header: String::new(),
            footer: String::new(),
            header_on_first_page: false,
        }
    }
}

/// Page geometry derived from a [`PageSetup`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Geometry {
    pub page_width_in: f32,
    pub page_height_in: f32,
    pub font_size_pt: f32,
    pub char_width_in: f32,
    pub line_height_in: f32,
    pub top_in: f32,
    /// Body lines per page.
    pub lines_per_page: usize,
}

impl Geometry {
    pub fn for_setup(s: &PageSetup) -> Self {
        let (w, h) = s.paper.size_in();
        let size = s.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE) as f32;
        let line = size / 72.0 * s.line_spacing.factor();
        let body = h - TOP_MARGIN_IN - BOTTOM_MARGIN_IN;
        Self {
            page_width_in: w,
            page_height_in: h,
            font_size_pt: size,
            // Courier advance width is 0.6 em.
            char_width_in: size * 0.6 / 72.0,
            line_height_in: line,
            top_in: TOP_MARGIN_IN,
            lines_per_page: ((body / line) + 1e-3).floor().max(10.0) as usize,
        }
    }
    /// Characters that fit in `width_in`.
    pub fn chars(&self, width_in: f32) -> usize {
        ((width_in / self.char_width_in) + 1e-3).floor().max(1.0) as usize
    }
    /// Baseline distance from the top of the page, in inches, for a body row.
    pub fn y_in(&self, row: i32) -> f32 {
        self.top_in + (row as f32 + 1.0) * self.line_height_in - 0.035 * self.font_size_pt / 12.0
    }
    fn text_width_in(&self, text: &str) -> f32 {
        // Shaped width for Indian scripts (ADR-0016); character count otherwise.
        crate::fonts::columns(text) * self.char_width_in
    }
}

impl Default for Geometry {
    fn default() -> Self {
        Self::for_setup(&PageSetup::default())
    }
}

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
    Custom,
    CustomItalic,
    More,
    PageNumber,
    SceneNumber,
    RevisionMark,
    Header,
    Footer,
    TitleMain,
    TitleText,
    /// The centred, underlined screenplay title on the first script page (layout only: it is
    /// never part of the script's elements).
    PageTitle,
}

impl LineStyle {
    /// Oblique (italic) Courier in the PDF.
    pub fn oblique(self) -> bool {
        matches!(
            self,
            LineStyle::Lyric | LineStyle::Note | LineStyle::CustomItalic
        )
    }
    /// Bold Courier Prime in the PDF: scene headings and the title page title.
    pub fn bold(self) -> bool {
        matches!(self, LineStyle::TitleMain | LineStyle::Heading)
    }
    /// Lines drawn with an underline.
    pub fn underlined(self) -> bool {
        matches!(self, LineStyle::TitleMain | LineStyle::PageTitle)
    }
    /// Stable name used by the print preview.
    pub fn as_str(self) -> &'static str {
        match self {
            LineStyle::Heading => "heading",
            LineStyle::Action => "action",
            LineStyle::Character => "character",
            LineStyle::Parenthetical => "parenthetical",
            LineStyle::Dialogue => "dialogue",
            LineStyle::Transition => "transition",
            LineStyle::Centered => "centered",
            LineStyle::Shot => "shot",
            LineStyle::Lyric => "lyric",
            LineStyle::Note => "note",
            LineStyle::Custom => "custom",
            LineStyle::CustomItalic => "custom_italic",
            LineStyle::More => "more",
            LineStyle::PageNumber => "page_number",
            LineStyle::SceneNumber => "scene_number",
            LineStyle::RevisionMark => "revision_mark",
            LineStyle::Header => "header",
            LineStyle::Footer => "footer",
            LineStyle::TitleMain => "title_main",
            LineStyle::TitleText => "title_text",
            LineStyle::PageTitle => "page_title",
        }
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaidOutPage {
    pub title_page: bool,
    /// Printed page number (None for the title page).
    pub number: Option<u32>,
    pub lines: Vec<PageLine>,
}

/// Where one scene of the document sits (printed page numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenePages {
    /// Index into [`ScreenplayDoc::scenes`].
    pub scene: usize,
    pub first_page: u32,
    pub last_page: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub pages: Vec<LaidOutPage>,
    pub geometry: Geometry,
    /// Pages of every scene that printed at least one line, in document order.
    pub scenes: Vec<ScenePages>,
    /// Production attribution printed in the bottom margin of every page (sides). Screenplay
    /// files leave it empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
}

impl Layout {
    pub fn script_page_count(&self) -> usize {
        self.pages.iter().filter(|p| !p.title_page).count()
    }
    /// Baseline distance of a line from the top of its page, in inches.
    pub fn line_y_in(&self, l: &PageLine) -> f32 {
        self.geometry.y_in(l.row)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
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
    pub page: PageSetup,
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
            page: PageSetup::default(),
        }
    }
}

/// Word-wrap `text` to `width` Courier columns, keeping explicit line breaks.
/// Words are measured with the PDF fonts' shaped advances
/// ([`crate::fonts::columns`]: one column per character for Windows-1252
/// text), and very long words break between grapheme clusters, never inside a
/// conjunct or before a combining mark.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1) as f32;
    let mut out = Vec::new();
    for para in text.split('\n') {
        let para = para.trim_end();
        if para.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut len = 0.0f32;
        for word in para.split(' ') {
            let wlen = crate::fonts::columns(word);
            if len > 0.0 && len + 1.0 + wlen > width + 1e-3 {
                out.push(std::mem::take(&mut line));
                len = 0.0;
            }
            if wlen > width + 1e-3 {
                // Hard-break very long words.
                for piece in crate::fonts::break_word(word, width) {
                    if len > 0.0 {
                        out.push(std::mem::take(&mut line));
                    }
                    len = crate::fonts::columns(&piece);
                    line = piece;
                }
                continue;
            }
            if len > 0.0 {
                line.push(' ');
                len += 1.0;
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
    /// A paragraph block (action, shot, centered, note, custom).
    Paragraph,
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
    splittable: bool,
    /// Cue text for (MORE)/(CONT'D).
    speaker: Option<String>,
    scene_number: Option<String>,
    /// Index of the scene the block belongs to.
    scene: usize,
    /// Never left alone at the foot of a page: stays with the start of the next block.
    keep_next: bool,
}

impl Block {
    fn new(kind: BlockKind, lines: Vec<BlockLine>, space_before: usize, scene: usize) -> Block {
        let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
        Block {
            kind,
            lines,
            rows,
            space_before,
            splittable: false,
            speaker: None,
            scene_number: None,
            scene,
            keep_next: false,
        }
    }
    /// Rows of this block that must stay on the page with a preceding keep-with-next block.
    fn keep_rows(&self) -> usize {
        match self.kind {
            BlockKind::Speech => speech_keep_rows(&self.lines, self.rows),
            BlockKind::Paragraph if self.splittable => self.rows.min(2),
            _ => self.rows,
        }
    }
}

/// Lines of one element laid out with a rule's column and alignment.
fn rule_lines(
    g: &Geometry,
    r: &ElementRule,
    text: &str,
    style: LineStyle,
    revised: bool,
    start_row: usize,
) -> Vec<BlockLine> {
    let width = g.chars(r.width_in);
    wrap(text, width)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let x = match r.align {
                Align::Left => r.left_in,
                // Every wrapped line is aligned on its own (a wrapped transition stays right-aligned).
                Align::Right => (r.left_in + r.width_in - g.text_width_in(&t)).max(r.left_in),
                Align::Center => r.left_in + ((r.width_in - g.text_width_in(&t)) / 2.0).max(0.0),
            };
            BlockLine {
                x,
                text: t,
                style,
                revised,
                row: start_row + i,
            }
        })
        .collect()
}

fn cased(text: &str, upper: bool) -> String {
    if upper {
        text.to_uppercase()
    } else {
        text.to_string()
    }
}

/// A speech column: positions of cue, parenthetical and dialogue.
struct SpeechColumn {
    cue: ElementRule,
    paren: ElementRule,
    dialogue: ElementRule,
}

impl SpeechColumn {
    fn standard() -> Self {
        Self {
            cue: *rules::rule_for("character"),
            paren: *rules::rule_for("parenthetical"),
            dialogue: *rules::rule_for("dialogue"),
        }
    }
    fn dual(x: f32) -> Self {
        let base = |k: &str, left_in: f32, width_in: f32| ElementRule {
            left_in,
            width_in,
            ..*rules::rule_for(k)
        };
        Self {
            cue: base("character", x + 0.8, DUAL_W_IN),
            paren: base("parenthetical", x + 0.4, DUAL_W_IN - 0.4),
            dialogue: base("dialogue", x, DUAL_W_IN),
        }
    }
}

fn speech_lines(g: &Geometry, elements: &[&Element], col: &SpeechColumn) -> Vec<BlockLine> {
    let mut out = Vec::new();
    let mut row = 0usize;
    for e in elements {
        let revised = e.revision.is_some();
        let (r, style) = match e.kind {
            ElementKind::Character => (&col.cue, LineStyle::Character),
            ElementKind::Parenthetical => (&col.paren, LineStyle::Parenthetical),
            ElementKind::Lyric => (&col.dialogue, LineStyle::Lyric),
            _ => (&col.dialogue, LineStyle::Dialogue),
        };
        let text = if e.kind == ElementKind::Character {
            e.text.trim().trim_end_matches('^').trim().to_uppercase()
        } else {
            e.text.clone()
        };
        let ls = rule_lines(g, r, &text, style, revised, row);
        row += ls.len();
        out.extend(ls);
    }
    out
}

/// Layout of a custom element: the look's column with the custom case, style and spacing.
fn custom_block(g: &Geometry, e: &Element, style: &CustomStyle, scene: usize) -> Block {
    let look = rules::rule_for(&style.look);
    let r = ElementRule {
        align: look.align,
        ..*look
    };
    let line_style = if style.italic {
        LineStyle::CustomItalic
    } else {
        LineStyle::Custom
    };
    let text = cased(&e.text, style.uppercase);
    let mut b = Block::new(
        BlockKind::Paragraph,
        rule_lines(g, &r, &text, line_style, e.revision.is_some(), 0),
        style.space_before.min(3) as usize,
        scene,
    );
    b.splittable = look.splittable;
    b.keep_next = look.keep_with_next;
    b
}

fn build_blocks(doc: &ScreenplayDoc, opts: &LayoutOptions, g: &Geometry) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut number = 0usize;
    let heading_rule = rules::rule_for("scene_heading");
    for (si, scene) in doc.scenes.iter().enumerate() {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let mut b = Block::new(
                BlockKind::Heading,
                rule_lines(
                    g,
                    heading_rule,
                    &scene.heading.to_uppercase(),
                    LineStyle::Heading,
                    false,
                    0,
                ),
                heading_rule.space_before as usize,
                si,
            );
            b.scene_number = Some(scene.number.clone().unwrap_or_else(|| number.to_string()));
            b.keep_next = heading_rule.keep_with_next;
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
                    let group: Vec<&Element> = els[i..j].iter().collect();
                    let second = group
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, e)| e.kind == ElementKind::Character)
                        .map(|(k, _)| k);
                    if let (true, Some(split)) = (dual, second) {
                        let mut lines =
                            speech_lines(g, &group[..split], &SpeechColumn::dual(DUAL_LEFT_X));
                        lines.extend(speech_lines(
                            g,
                            &group[split..],
                            &SpeechColumn::dual(DUAL_RIGHT_X),
                        ));
                        blocks.push(Block::new(BlockKind::Dual, lines, 1, si));
                    } else {
                        let speaker = group
                            .iter()
                            .find(|e| e.kind == ElementKind::Character)
                            .map(|e| e.text.trim().trim_end_matches('^').trim().to_uppercase());
                        let mut b = Block::new(
                            BlockKind::Speech,
                            speech_lines(g, &group, &SpeechColumn::standard()),
                            1,
                            si,
                        );
                        b.speaker = speaker;
                        b.splittable = true;
                        blocks.push(b);
                    }
                    i = j;
                    continue;
                }
                ElementKind::Transition => {
                    let r = rules::rule_for("transition");
                    let mut b = Block::new(
                        BlockKind::Transition,
                        rule_lines(
                            g,
                            r,
                            &e.text.trim().to_uppercase(),
                            LineStyle::Transition,
                            e.revision.is_some(),
                            0,
                        ),
                        r.space_before as usize,
                        si,
                    );
                    b.keep_next = r.keep_with_next;
                    blocks.push(b);
                }
                ElementKind::Centered | ElementKind::Action | ElementKind::Shot => {
                    let (r, style) = match e.kind {
                        ElementKind::Centered => (rules::rule_for("centered"), LineStyle::Centered),
                        ElementKind::Shot => (rules::rule_for("shot"), LineStyle::Shot),
                        _ => (rules::rule_for("action"), LineStyle::Action),
                    };
                    let mut b = Block::new(
                        BlockKind::Paragraph,
                        rule_lines(
                            g,
                            r,
                            &cased(&e.text, r.uppercase),
                            style,
                            e.revision.is_some(),
                            0,
                        ),
                        r.space_before as usize,
                        si,
                    );
                    b.splittable = r.splittable;
                    b.keep_next = r.keep_with_next;
                    blocks.push(b);
                }
                ElementKind::Custom => {
                    let style = e
                        .custom
                        .clone()
                        .unwrap_or_else(|| CustomStyle::imported("Custom", "action"));
                    blocks.push(custom_block(g, e, &style, si));
                }
                ElementKind::Note => {
                    if opts.include_notes {
                        let text = format!("[Note: {}]", e.text.trim());
                        let mut b = Block::new(
                            BlockKind::Paragraph,
                            rule_lines(
                                g,
                                rules::rule_for("action"),
                                &text,
                                LineStyle::Note,
                                false,
                                0,
                            ),
                            1,
                            si,
                        );
                        b.splittable = true;
                        blocks.push(b);
                    }
                }
                ElementKind::PageBreak => {
                    blocks.push(Block::new(BlockKind::PageBreak, vec![], 0, si))
                }
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

struct Paginator<'a> {
    opts: &'a LayoutOptions,
    g: Geometry,
    lpp: usize,
    pages: Vec<LaidOutPage>,
    current: Vec<PageLine>,
    row: usize,
    /// (scene index, first page index, last page index) as lines are placed.
    scene_pages: Vec<(usize, usize, usize)>,
}

impl Paginator<'_> {
    fn new_page(&mut self) {
        let lines = std::mem::take(&mut self.current);
        self.pages.push(LaidOutPage {
            title_page: false,
            number: None,
            lines,
        });
        self.row = 0;
    }
    fn note_scene(&mut self, scene: usize) {
        let page = self.pages.len();
        match self.scene_pages.last_mut() {
            Some(last) if last.0 == scene => last.2 = page,
            _ => self.scene_pages.push((scene, page, page)),
        }
    }
    fn place(&mut self, block_lines: &[BlockLine], scene_number: Option<&str>, scene: usize) {
        if !block_lines.is_empty() {
            self.note_scene(scene);
        }
        let base = self.row as i32;
        let mut max_row = 0usize;
        let right = rules::rule_for("action");
        let right_edge = right.left_in + right.width_in;
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
                    x_in: right_edge + 0.2,
                    text: "*".into(),
                    style: LineStyle::RevisionMark,
                });
            }
            max_row = max_row.max(l.row + 1);
        }
        if let (Some(n), true) = (scene_number, self.opts.scene_numbers) {
            self.current.push(PageLine {
                row: base,
                x_in: right.left_in - 0.5 - self.g.text_width_in(n),
                text: n.to_string(),
                style: LineStyle::SceneNumber,
            });
            self.current.push(PageLine {
                row: base,
                x_in: right_edge + 0.1,
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
    let g = Geometry::for_setup(&opts.page);
    let blocks = build_blocks(doc, opts, &g);
    let lpp = if opts.paginate {
        g.lines_per_page
    } else {
        usize::MAX / 4
    };
    let mut p = Paginator {
        opts,
        g,
        lpp,
        pages: Vec::new(),
        current: Vec::new(),
        row: 0,
        scene_pages: Vec::new(),
    };

    for (bi, block) in blocks.iter().enumerate() {
        if block.kind == BlockKind::PageBreak {
            if p.row > 0 {
                p.new_page();
            }
            continue;
        }
        if block.kind == BlockKind::Speech {
            place_speech(&mut p, block);
            continue;
        }
        let space = if p.row == 0 { 0 } else { block.space_before };
        // A keep-with-next block needs the start of what follows on its page.
        let needed = space + block.rows + follow_rows(&blocks, bi);
        if needed <= p.remaining() {
            p.row += space;
            p.place(&block.lines, block.scene_number.as_deref(), block.scene);
            continue;
        }
        // Does not fit: try to split, otherwise move to the next page.
        let available = p.remaining().saturating_sub(space);
        match block.kind {
            // At least two lines stay on each side of the split.
            BlockKind::Paragraph if block.splittable && block.rows >= 4 && available >= 2 => {
                p.row += space;
                let (a, b) = split_rows(&block.lines, available.min(block.rows - 2));
                p.place(&a, None, block.scene);
                p.new_page();
                place_overflowing(&mut p, &b, None, block.scene);
            }
            _ => {
                if p.row > 0 {
                    p.new_page();
                }
                place_overflowing(
                    &mut p,
                    &block.lines,
                    block.scene_number.as_deref(),
                    block.scene,
                );
            }
        }
    }
    if !p.current.is_empty() || p.pages.is_empty() {
        p.new_page();
    }

    let first = opts.page.first_page_number.max(1);
    let scenes = p
        .scene_pages
        .iter()
        .map(|&(scene, a, b)| ScenePages {
            scene,
            first_page: first + a as u32,
            last_page: first + b as u32,
        })
        .collect();
    let mut pages = p.pages;
    decorate_pages(&mut pages, doc, opts, &g, first);
    if opts.title_page && !doc.title_page.is_empty() {
        pages.splice(0..0, title_pages(doc, opts, &g));
    }
    Layout {
        pages,
        geometry: g,
        scenes,
        brand: None,
    }
}

/// Fill `{title}`, `{draft}`, `{revision}`, `{date}` and `{page}`.
pub fn fill_placeholders(template: &str, doc: &ScreenplayDoc, page: u32) -> String {
    let tp = &doc.title_page;
    let one_line = |v: Option<&str>| v.unwrap_or("").replace('\n', " ").trim().to_string();
    template
        .replace("{title}", &one_line(tp.title()))
        .replace(
            "{draft}",
            &one_line(tp.get("Draft").or_else(|| tp.get("Draft date"))),
        )
        .replace("{revision}", &one_line(tp.get("Revision")))
        .replace(
            "{date}",
            &one_line(tp.get("Date").or_else(|| tp.get("Draft date"))),
        )
        .replace("{page}", &page.to_string())
}

/// Number the script pages and add page numbers, header and footer.
fn decorate_pages(
    pages: &mut [LaidOutPage],
    doc: &ScreenplayDoc,
    opts: &LayoutOptions,
    g: &Geometry,
    first: u32,
) {
    let action = rules::rule_for("action");
    let right_edge = action.left_in + action.width_in;
    let header_row = -3;
    // Footer baseline about half an inch above the bottom edge.
    let footer_row =
        (((g.page_height_in - 0.5 - g.top_in) / g.line_height_in).floor() as i32 - 1).max(1);
    for (i, page) in pages.iter_mut().enumerate() {
        let n = first + i as u32;
        page.number = Some(n);
        let decorate = opts.paginate && (i >= 1 || opts.page.header_on_first_page);
        // Every script page shows its number, except the first when the setup leaves it off.
        let numbered =
            opts.paginate && opts.page_numbers && (i >= 1 || opts.page.number_first_page);
        let mut page_label_x = right_edge;
        if numbered {
            let label = format!("{n}.");
            page_label_x = right_edge - g.text_width_in(&label);
            page.lines.push(PageLine {
                row: header_row,
                x_in: page_label_x,
                text: label,
                style: LineStyle::PageNumber,
            });
        }
        let has_header = decorate && !opts.page.header.trim().is_empty();
        if i == 0 && opts.paginate {
            running_title(page, doc, g, has_header);
        }
        if decorate && !opts.page.header.trim().is_empty() {
            let text = fill_placeholders(opts.page.header.trim(), doc, n);
            // Leave two characters before the page number.
            let room = g.chars((page_label_x - action.left_in - 2.0 * g.char_width_in).max(0.5));
            let text: String = text.chars().take(room).collect();
            if !text.trim().is_empty() {
                page.lines.push(PageLine {
                    row: header_row,
                    x_in: action.left_in,
                    text,
                    style: LineStyle::Header,
                });
            }
        }
        if decorate && !opts.page.footer.trim().is_empty() {
            let text = fill_placeholders(opts.page.footer.trim(), doc, n);
            let text: String = text.chars().take(g.chars(action.width_in)).collect();
            if !text.trim().is_empty() {
                let x =
                    action.left_in + ((action.width_in - g.text_width_in(&text)) / 2.0).max(0.0);
                page.lines.push(PageLine {
                    row: footer_row,
                    x_in: x,
                    text,
                    style: LineStyle::Footer,
                });
            }
        }
        page.lines
            .sort_by(|a, b| a.row.cmp(&b.row).then(a.x_in.total_cmp(&b.x_in)));
    }
}

/// Longest running title line, in characters (leaves a clear gap before the page number).
const PAGE_TITLE_CHARS: usize = 50;

/// The centred, underlined title near the top of the first script page: layout only, never an
/// element of the script. It sits in the header band above the body, so it never takes a body
/// row and never shifts the script. When the page also prints a header text, the title moves
/// one row down so the two never overlap.
fn running_title(page: &mut LaidOutPage, doc: &ScreenplayDoc, g: &Geometry, header_on_page: bool) {
    let Some(title) = doc.title_page.title() else {
        return;
    };
    let title = title.replace('\n', " ").trim().to_uppercase();
    if title.is_empty() {
        return;
    }
    let mut lines = wrap(&title, PAGE_TITLE_CHARS);
    if lines.len() > 2 {
        // A third line would touch the first body row: keep two and end the second with an
        // ellipsis, cut at a fixed width so the result is always the same.
        lines.drain(2..);
        if let Some(last) = lines.last_mut() {
            let mut t: String = last.chars().take(PAGE_TITLE_CHARS - 1).collect();
            t.push('\u{2026}');
            *last = t;
        }
    }
    let action = rules::rule_for("action");
    // Lines run downward from the page number row, so a long title never climbs into the top
    // margin. Row -1 stays empty above the body.
    let first_row = if header_on_page { -2 } else { -3 };
    for (k, text) in lines.into_iter().enumerate() {
        let x = action.left_in + ((action.width_in - g.text_width_in(&text)) / 2.0).max(0.0);
        page.lines.push(PageLine {
            row: first_row + k as i32,
            x_in: x,
            text,
            style: LineStyle::PageTitle,
        });
    }
}

/// Rows after block `bi` that must share its page: what a keep-with-next block stays with (a
/// scene heading with the start of its scene, a transition with the next scene heading, a shot
/// with its description). A heading is not chained to another heading.
fn follow_rows(blocks: &[Block], bi: usize) -> usize {
    let b = &blocks[bi];
    if !b.keep_next {
        return 0;
    }
    match blocks.get(bi + 1) {
        Some(n)
            if n.kind != BlockKind::PageBreak
                && (n.kind != BlockKind::Heading || b.kind == BlockKind::Transition) =>
        {
            n.space_before + n.keep_rows() + follow_rows(blocks, bi + 1)
        }
        _ => 0,
    }
}

/// Where a speech can break so that `available` rows are not exceeded: the number of rows kept
/// on this page. The cue keeps at least two lines of speech with it, one row is left for
/// `(MORE)`, at least two rows go to the next page and the break follows a dialogue line.
/// `relaxed` accepts any line but a cue (a speech with no dialogue lines to break after).
fn speech_split(lines: &[BlockLine], available: usize, relaxed: bool) -> Option<usize> {
    let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
    let (min_rows, min_first) = if relaxed { (3, 2) } else { (5, 3) };
    if available < min_first + 1 || rows < min_rows {
        return None;
    }
    let max_first = (available - 1).min(rows - 2);
    (min_first..=max_first)
        .rev()
        .find(|&k| split_ok(lines, k, relaxed))
}

/// May a speech break after `k` rows: after a dialogue line (any line but a cue when relaxed),
/// and not in the middle of a wrapped cue.
fn split_ok(lines: &[BlockLine], k: usize, relaxed: bool) -> bool {
    let last = lines.iter().rfind(|l| l.row == k - 1);
    let next = lines.iter().find(|l| l.row == k);
    let last_ok = last.is_some_and(|l| {
        if relaxed {
            l.style != LineStyle::Character
        } else {
            l.style == LineStyle::Dialogue || l.style == LineStyle::Lyric
        }
    });
    last_ok && next.is_some_and(|l| l.style != LineStyle::Character)
}

/// Rows of a speech that must share a page with whatever it follows: the whole speech when it is
/// too short to break, otherwise the rows before its first break point and the `(MORE)` row.
fn speech_keep_rows(lines: &[BlockLine], rows: usize) -> usize {
    if rows < 5 {
        return rows;
    }
    (3..=rows - 2)
        .find(|&k| split_ok(lines, k, false))
        .map_or(rows, |k| k + 1)
}

/// Place one speech block. Every page break inside it puts `(MORE)` at the foot and the
/// cue with `(CONT'D)` at the top of the next page, however tall the speech is. Short speeches
/// that do not fit move to the next page whole.
fn place_speech(p: &mut Paginator<'_>, block: &Block) {
    let cue = *rules::rule_for("character");
    let mut lines = block.lines.clone();
    let mut space = block.space_before;
    loop {
        let space_now = if p.row == 0 { 0 } else { space };
        let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
        if space_now + rows <= p.remaining() {
            p.row += space_now;
            p.place(&lines, None, block.scene);
            return;
        }
        let available = p.remaining().saturating_sub(space_now);
        let split = speech_split(&lines, available, false)
            .or_else(|| speech_split(&lines, available, true).filter(|_| p.row == 0));
        match split {
            Some(k) => {
                p.row += space_now;
                let (a, b) = split_rows(&lines, k);
                p.place(&a, None, block.scene);
                let more_row = p.row as i32;
                p.current.push(PageLine {
                    row: more_row,
                    x_in: cue.left_in,
                    text: "(MORE)".into(),
                    style: LineStyle::More,
                });
                p.new_page();
                lines = continued_speech(&p.g, &cue, block.speaker.as_deref(), b);
                space = 0;
            }
            // Try again on a fresh page.
            None if p.row > 0 => p.new_page(),
            // Nothing can break (not reachable for a speech with two or more lines).
            None => {
                place_overflowing(p, &lines, None, block.scene);
                return;
            }
        }
    }
}

/// The rest of a speech under its cue with `(CONT'D)`: an imported marker is kept, never doubled.
/// A long cue wraps inside its column.
fn continued_speech(
    g: &Geometry,
    cue: &ElementRule,
    speaker: Option<&str>,
    rest: Vec<BlockLine>,
) -> Vec<BlockLine> {
    let name = speaker.unwrap_or_default().trim();
    let text = if name.contains("(CONT'D)") {
        name.to_string()
    } else if name.is_empty() {
        "(CONT'D)".to_string()
    } else {
        format!("{name} (CONT'D)")
    };
    let mut out = rule_lines(g, cue, &text, LineStyle::Character, false, 0);
    let shift = out.len();
    out.extend(rest.into_iter().map(|mut l| {
        l.row += shift;
        l
    }));
    out
}

/// Place lines that may be longer than a page, breaking at page boundaries.
fn place_overflowing(
    p: &mut Paginator<'_>,
    lines: &[BlockLine],
    scene_number: Option<&str>,
    scene: usize,
) {
    let rows = lines.iter().map(|l| l.row + 1).max().unwrap_or(0);
    if rows <= p.remaining() {
        p.place(lines, scene_number, scene);
        return;
    }
    let mut rest: Vec<BlockLine> = lines.to_vec();
    let mut first = true;
    while !rest.is_empty() {
        let left = rest.iter().map(|l| l.row + 1).max().unwrap_or(0);
        let mut avail = p.remaining().max(1);
        // Never strand a single line at the top of the next page.
        if left > avail && left - avail == 1 && avail > 2 {
            avail -= 1;
        }
        let (a, b) = split_rows(&rest, avail);
        p.place(&a, if first { scene_number } else { None }, scene);
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

fn centered(g: &Geometry, text: &str, row: i32, style: LineStyle) -> PageLine {
    PageLine {
        row,
        x_in: ((g.page_width_in - g.text_width_in(text)) / 2.0).max(1.0),
        text: text.to_string(),
        style,
    }
}

/// Title page columns: contact block bottom left, draft block bottom right.
const TITLE_LEFT_CHARS: usize = 34;
const TITLE_RIGHT_CHARS: usize = 26;
const TITLE_CENTER_CHARS: usize = 50;
/// Clear space kept between the two lower blocks, in inches. The left block gives up columns
/// to the right block's real width, so the two never touch (a 34 + 26 character pair filled the
/// whole 60 character text area and ran together).
const TITLE_GUTTER_IN: f32 = 0.4;
/// Empty rows between the centred block and the lower blocks.
const TITLE_BLOCK_GAP_ROWS: i32 = 2;

fn wrapped_lines(value: &str, width: usize) -> Vec<String> {
    value
        .split('\n')
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .flat_map(|l| wrap(l, width))
        .collect()
}

/// The centred title, credit, author and source block starting at row `start`: its lines and the
/// first free row below it.
fn title_centre(doc: &ScreenplayDoc, g: &Geometry, start: i32) -> (Vec<PageLine>, i32) {
    let tp = &doc.title_page;
    let mut lines = Vec::new();
    let mut row = start;
    if let Some(title) = tp.title() {
        for w in wrapped_lines(&title.to_uppercase(), TITLE_CENTER_CHARS) {
            lines.push(centered(g, &w, row, LineStyle::TitleMain));
            row += 1;
        }
    }
    row += 3;
    let credit = tp.get("Credit").unwrap_or("Written by");
    if tp.author().is_some() {
        lines.push(centered(g, credit.trim(), row, LineStyle::TitleText));
        row += 2;
        for a in wrapped_lines(tp.author().unwrap_or_default(), TITLE_CENTER_CHARS) {
            lines.push(centered(g, &a, row, LineStyle::TitleText));
            row += 1;
        }
    }
    if let Some(src) = tp.get("Source") {
        row += 2;
        for s in wrapped_lines(src, TITLE_CENTER_CHARS) {
            lines.push(centered(g, &s, row, LineStyle::TitleText));
            row += 1;
        }
    }
    (lines, row)
}

/// The title page: a centred block about 3" down and two bottom blocks, contact and legal text on
/// the left, draft details and representation on the right. Each lower block wraps inside its
/// own width and the two keep a gutter. They grow upward from the last body row; when they are
/// too tall for the space under the centred block, the centred block moves up (to the top of the
/// body at most), and a surplus beyond that continues on further title pages, so nothing is cut.
fn title_pages(doc: &ScreenplayDoc, opts: &LayoutOptions, g: &Geometry) -> Vec<LaidOutPage> {
    let tp = &doc.title_page;
    let lpp = g.lines_per_page as i32;
    let action = rules::rule_for("action");
    let right_edge = action.left_in + action.width_in;

    // Bottom right: draft, date, revision, representation.
    let mut right: Vec<String> = Vec::new();
    for key in ["Draft", "Draft date", "Date"] {
        if let Some(d) = tp.get(key) {
            right.extend(wrapped_lines(d, TITLE_RIGHT_CHARS));
        }
    }
    if let (true, Some(r)) = (opts.revision_info, tp.get("Revision")) {
        right.extend(wrapped_lines(r, TITLE_RIGHT_CHARS));
    }
    if let Some(r) = tp.get("Representation") {
        right.push(String::new());
        right.extend(wrapped_lines(r, TITLE_RIGHT_CHARS));
    }
    // Bottom left: notes, contact, copyright, in the room the right block leaves.
    let right_width = right
        .iter()
        .map(|r| g.text_width_in(r))
        .fold(0.0f32, f32::max);
    let left_chars = g
        .chars(right_edge - right_width - TITLE_GUTTER_IN - action.left_in)
        .min(TITLE_LEFT_CHARS);
    let mut left: Vec<String> = Vec::new();
    if let Some(n) = tp.get("Notes") {
        left.extend(wrapped_lines(n, left_chars));
        left.push(String::new());
    }
    for key in ["Contact", "Copyright"] {
        if let Some(c) = tp.get(key) {
            left.extend(wrapped_lines(c, left_chars));
        }
    }
    for block in [&mut left, &mut right] {
        while block.last().is_some_and(String::is_empty) {
            block.pop();
        }
    }

    let default_start = (3.0 / g.line_height_in).round() as i32;
    let need = left.len().max(right.len()) as i32;
    let (mut lines, mut end) = title_centre(doc, g, default_start);
    let room = lpp - (end + TITLE_BLOCK_GAP_ROWS);
    if need > room {
        // The block has the same height wherever it starts, so this moves it exactly far enough.
        (lines, end) = title_centre(doc, g, (default_start - (need - room)).max(0));
    }

    let mut pages = Vec::new();
    let mut top = end + TITLE_BLOCK_GAP_ROWS;
    let mut first = true;
    loop {
        let room = (lpp - top).max(1) as usize;
        for (block, is_left) in [(&mut left, true), (&mut right, false)] {
            let n = block.len().min(room);
            // A block that finishes on its first page is anchored to the bottom; any other
            // page fills from the top.
            let anchor = if first && n == block.len() {
                lpp - n as i32
            } else {
                top
            };
            for (i, text) in block.drain(..n).enumerate() {
                if text.is_empty() {
                    continue;
                }
                let x_in = if is_left {
                    action.left_in
                } else {
                    right_edge - g.text_width_in(&text)
                };
                lines.push(PageLine {
                    row: anchor + i as i32,
                    x_in,
                    text,
                    style: LineStyle::TitleText,
                });
            }
        }
        lines.sort_by(|a, b| a.row.cmp(&b.row).then(a.x_in.total_cmp(&b.x_in)));
        pages.push(LaidOutPage {
            title_page: true,
            number: None,
            lines: std::mem::take(&mut lines),
        });
        if left.is_empty() && right.is_empty() {
            break;
        }
        first = false;
        top = 0;
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scene;

    fn no_title() -> LayoutOptions {
        LayoutOptions {
            title_page: false,
            ..Default::default()
        }
    }

    #[test]
    fn wrapping_respects_width_and_breaks() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("a\nb", 10), vec!["a", "b"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn standard_geometry_is_the_industry_page() {
        let g = Geometry::default();
        assert_eq!(g.lines_per_page, LINES_PER_PAGE);
        assert_eq!(g.chars(6.0), 60);
        assert!((g.line_height_in - LINE_HEIGHT_IN).abs() < 1e-6);
        assert!((g.char_width_in - CHAR_WIDTH_IN).abs() < 1e-6);
        let a4 = Geometry::for_setup(&PageSetup {
            paper: Paper::A4,
            ..Default::default()
        });
        assert_eq!(a4.lines_per_page, 59);
        assert_eq!(a4.chars(6.0), 60, "A4 keeps the same columns");
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
        let lay = layout(&doc, &no_title());
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
        // The first script page shows "1." like every other page.
        assert!(
            lay.pages[0]
                .lines
                .iter()
                .any(|l| l.style == LineStyle::PageNumber && l.text == "1.")
        );
    }

    #[test]
    fn first_page_has_a_layout_only_title_line_and_bold_scene_headings() {
        let mut doc = ScreenplayDoc::default();
        doc.title_page.set("Title", "Monsoon Letters");
        let mut s = Scene::new("INT. KITCHEN - DAWN");
        s.elements.push(Element::new(ElementKind::Action, "Rain."));
        doc.scenes.push(s);
        let lay = layout(&doc, &no_title());
        let first = &lay.pages[0];
        let title: Vec<&PageLine> = first
            .lines
            .iter()
            .filter(|l| l.style == LineStyle::PageTitle)
            .collect();
        assert_eq!(title.len(), 1);
        assert_eq!(title[0].text, "MONSOON LETTERS");
        assert!(
            title[0].row < 0,
            "the title sits above the body, never in it"
        );
        assert!(title[0].style.underlined() && !title[0].style.bold());
        // It is not a script element: the body is exactly what the script holds.
        let body: Vec<&str> = first
            .lines
            .iter()
            .filter(|l| l.row >= 0)
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(body, vec!["INT. KITCHEN - DAWN", "Rain."]);
        assert!(LineStyle::Heading.bold() && !LineStyle::Action.bold());
        // Later pages and unpaginated text never get it.
        let flat = layout(
            &doc,
            &LayoutOptions {
                paginate: false,
                ..no_title()
            },
        );
        assert!(
            !flat
                .pages
                .iter()
                .flat_map(|p| p.lines.iter())
                .any(|l| l.style == LineStyle::PageTitle)
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
        let lay = layout(&doc, &no_title());
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

    #[test]
    fn wrapped_transition_lines_are_each_right_aligned() {
        let mut doc = ScreenplayDoc::default();
        let mut s = Scene::new("INT. A - DAY");
        s.elements.push(Element::new(
            ElementKind::Transition,
            "SLOW DISSOLVE THROUGH THE RAIN AND THE NEON AND THE LONG NIGHT TO:",
        ));
        doc.scenes.push(s);
        let lay = layout(&doc, &no_title());
        let lines: Vec<&PageLine> = lay.pages[0]
            .lines
            .iter()
            .filter(|l| l.style == LineStyle::Transition)
            .collect();
        assert_eq!(lines.len(), 2);
        for l in lines {
            let end = l.x_in + l.text.chars().count() as f32 * CHAR_WIDTH_IN;
            assert!(
                (end - RIGHT_EDGE_IN).abs() < 1e-4,
                "{} ends at {end}",
                l.text
            );
        }
    }
}
