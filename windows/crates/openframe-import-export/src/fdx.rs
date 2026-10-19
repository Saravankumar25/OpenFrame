//! Final Draft `.fdx` import and export.
//!
//! Paragraph `Type` mapping (both directions):
//!
//! | FDX                | OpenFrame          |
//! |--------------------|--------------------|
//! | Scene Heading      | scene heading      |
//! | Action / General   | action             |
//! | Character          | character          |
//! | Parenthetical      | parenthetical      |
//! | Dialogue           | dialogue           |
//! | Transition         | transition         |
//! | Shot               | shot               |
//! | Action + Alignment=Center | centered    |
//! | `<DualDialogue>`   | dual flag          |
//! | Text `RevisionID`  | revision mark      |
//! | `StartsNewPage="Yes"` | page break before the paragraph |
//! | any other `Type`   | custom element (identity = the type name) |
//!
//! A paragraph type OpenFrame has no built-in element for (a Final Draft
//! custom element such as "TELEPHONE", a Cast List) is never flattened into
//! action: it is kept as a custom element whose import and export identity is
//! the type name, with its look read from the file's `ElementSettings`
//! (alignment, indent, all caps, italic). Export writes custom elements back
//! under their identity together with matching `ElementSettings`.
//!
//! Script notes, tags, smart-type lists and page layout sections are ignored
//! gracefully and reported in the import warnings.

use std::collections::BTreeMap;

use crate::heuristics::{Confidence, split_scene_number};
use crate::model::{CustomStyle, Element, ElementKind, Scene, ScreenplayDoc, TitlePage, Warning};
use crate::rules;
use crate::xml::{self, Node, XmlWriter};
use crate::{ImportOutcome, SourceFormat};
use openframe_domain::{AppError, AppResult};

fn malformed(detail: impl Into<String>) -> AppError {
    AppError::import(
        "fdx_malformed",
        "This Final Draft file is damaged or incomplete, so it could not be read. Your current project was not changed.",
    )
    .with_detail(detail)
}

/// `ElementSettings` entries read from one file (Final Draft's own templates use about 30).
pub const MAX_ELEMENT_SETTINGS: usize = 256;
/// Custom paragraph types named one by one in the import warnings.
const MAX_LISTED_CUSTOM_TYPES: usize = 20;
/// Distinct custom paragraph types one file may bring in; paragraphs of any
/// further type are imported as action and reported (security review 2026-10-07).
pub const MAX_CUSTOM_TYPES: usize = 256;
/// Characters kept of a custom paragraph type name (its import and export identity).
pub const MAX_CUSTOM_TYPE_CHARS: usize = 60;

/// A custom paragraph type name as OpenFrame keeps it: control characters
/// removed, surrounding space trimmed and at most [`MAX_CUSTOM_TYPE_CHARS`]
/// characters, so every later copy of the name is bounded and two types that
/// only differ past the limit become one identity.
pub fn custom_type_name(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_CUSTOM_TYPE_CHARS)
        .collect();
    match cleaned.trim() {
        "" => "Custom".to_string(),
        t => t.to_string(),
    }
}

struct State {
    doc: ScreenplayDoc,
    /// Paragraph count per kept custom type (at most [`MAX_CUSTOM_TYPES`] keys).
    unknown_types: BTreeMap<String, usize>,
    /// Paragraphs imported as action because their file had too many custom types.
    over_limit_paragraphs: usize,
    /// Script notes attached to paragraphs, kept as notes.
    inline_notes: usize,
    revisions: BTreeMap<String, String>,
    /// Looks of custom paragraph types, read from `ElementSettings`.
    custom_looks: BTreeMap<String, CustomStyle>,
}

impl State {
    fn scene(&mut self) -> &mut Scene {
        if self.doc.scenes.is_empty() {
            self.doc.scenes.push(Scene::default());
        }
        self.doc.scenes.last_mut().unwrap()
    }
    fn custom_style(&self, ty: &str, centered: bool) -> CustomStyle {
        self.custom_looks.get(ty).cloned().unwrap_or_else(|| {
            CustomStyle::imported(ty, if centered { "centered" } else { "action" })
        })
    }
}

/// Built-in Final Draft paragraph types (everything else is a custom element).
fn builtin_kind(ty: &str, centered: bool) -> Option<ElementKind> {
    Some(match ty {
        "Action" | "General" => {
            if centered {
                ElementKind::Centered
            } else {
                ElementKind::Action
            }
        }
        "Character" => ElementKind::Character,
        "Dialogue" => ElementKind::Dialogue,
        "Parenthetical" => ElementKind::Parenthetical,
        "Transition" => ElementKind::Transition,
        "Shot" => ElementKind::Shot,
        _ => return None,
    })
}

/// The look of a custom paragraph type from its `ElementSettings`: alignment
/// first, then the nearest built-in left indent.
fn look_from_settings(es: &Node) -> CustomStyle {
    let ty = custom_type_name(es.attr("Type").unwrap_or("Custom"));
    let ty = ty.as_str();
    let para = es.child("ParagraphSpec");
    let align = para.and_then(|p| p.attr("Alignment")).unwrap_or("Left");
    let indent: Option<f32> = para
        .and_then(|p| p.attr("LeftIndent"))
        .and_then(|v| v.trim().parse().ok());
    let look = match align {
        "Center" => "centered",
        "Right" => "transition",
        _ => match indent {
            Some(x) => rules::custom_looks()
                .filter(|r| r.align == rules::Align::Left)
                .min_by(|a, b| (a.left_in - x).abs().total_cmp(&(b.left_in - x).abs()))
                .map(|r| r.kind)
                .unwrap_or("action"),
            None => "action",
        },
    };
    let style = es
        .child("FontSpec")
        .and_then(|f| f.attr("Style"))
        .unwrap_or("");
    let space_before: Option<u8> = para
        .and_then(|p| p.attr("SpaceBefore"))
        .and_then(|v| v.trim().parse::<u32>().ok())
        .map(|pt| (pt / 12).min(3) as u8);
    CustomStyle {
        uppercase: style.split('+').any(|s| s == "AllCaps"),
        italic: style.split('+').any(|s| s == "Italic"),
        space_before: space_before.unwrap_or(1),
        ..CustomStyle::imported(ty, look)
    }
}

fn paragraph_text(p: &Node) -> (String, Option<String>) {
    let mut text = String::new();
    let mut revision = None;
    for t in p.children_named("Text") {
        text.push_str(&t.text());
        if let Some(id) = t
            .attr("RevisionID")
            .filter(|id| *id != "0" && !id.is_empty())
        {
            revision.get_or_insert_with(|| id.to_string());
        }
    }
    // Text nested in other wrappers (e.g. DynamicLabel) is ignored deliberately.
    (text.replace('\r', ""), revision)
}

fn handle_paragraph(st: &mut State, p: &Node, dual: bool) {
    if p.attr("StartsNewPage") == Some("Yes") && !st.doc.scenes.is_empty() {
        // A page break before this paragraph (before a heading it ends the previous scene).
        st.scene().elements.push(Element::page_break());
    }
    if let Some(dd) = p.child("DualDialogue") {
        for inner in dd.children_named("Paragraph") {
            handle_paragraph(st, inner, true);
        }
        return;
    }
    let ty = p.attr("Type").unwrap_or("General");
    let (text, rev_id) = paragraph_text(p);
    let revision = rev_id.map(|id| {
        st.revisions
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Revision {id}"))
    });
    let t = text.trim_end();
    if ty == "Scene Heading" {
        let (heading, inline_number) = split_scene_number(t.trim());
        let number = p
            .attr("Number")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or(inline_number);
        if !heading.is_empty() {
            st.doc.scenes.push(Scene {
                heading,
                number,
                elements: Vec::new(),
            });
        }
        push_inline_notes(st, p);
        return;
    }
    if t.trim().is_empty() {
        push_inline_notes(st, p);
        return;
    }
    let centered = p.attr("Alignment") == Some("Center");
    let ty = if ty.trim().is_empty() {
        "General"
    } else {
        ty.trim()
    };
    let mut e = match builtin_kind(ty, centered) {
        Some(kind) => {
            let text = match kind {
                ElementKind::Action | ElementKind::Centered | ElementKind::Dialogue => {
                    t.to_string()
                }
                _ => t.trim().to_string(),
            };
            Element::new(kind, text)
        }
        None => {
            let name = custom_type_name(ty);
            let known = st.unknown_types.contains_key(&name);
            if known || st.unknown_types.len() < MAX_CUSTOM_TYPES {
                *st.unknown_types.entry(name.clone()).or_default() += 1;
                Element::custom(t.to_string(), st.custom_style(&name, centered))
            } else {
                st.over_limit_paragraphs += 1;
                Element::new(
                    if centered {
                        ElementKind::Centered
                    } else {
                        ElementKind::Action
                    },
                    t.to_string(),
                )
            }
        }
    }
    .dual(dual);
    e.revision = revision;
    st.scene().elements.push(e);
    push_inline_notes(st, p);
}

/// Final Draft script notes attached to a paragraph become OpenFrame notes right after it,
/// so their text is kept (notes never print).
fn push_inline_notes(st: &mut State, p: &Node) {
    for note in p.children_named("ScriptNote") {
        let text: Vec<String> = note
            .children_named("Paragraph")
            .map(|np| paragraph_text(np).0.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if !text.is_empty() {
            st.scene()
                .elements
                .push(Element::new(ElementKind::Note, text.join("\n")));
            st.inline_notes += 1;
        }
    }
}

fn parse_title_page(tp: &Node) -> TitlePage {
    let mut lines: Vec<(String, String)> = Vec::new(); // (alignment, text)
    if let Some(content) = tp.child("Content") {
        for p in content.children_named("Paragraph") {
            let (text, _) = paragraph_text(p);
            lines.push((
                p.attr("Alignment").unwrap_or("Left").to_string(),
                text.trim().to_string(),
            ));
        }
    }
    title_page_from_lines(&lines)
}

/// Interpret title-page lines given as (alignment "Center"/"Right"/"Left", text):
/// first centered block = title; "Written by" = credit, followed by authors;
/// further centered text = source; right-aligned = draft; left-aligned = contact.
pub(crate) fn title_page_from_lines(lines: &[(String, String)]) -> TitlePage {
    let mut page = TitlePage::default();
    let mut i = 0;
    let centered = |a: &str| a == "Center";
    // Credit and source lines end the title even without a blank line between them.
    let is_credit = |lower: &str| lower == "written by" || lower == "by" || lower.ends_with(" by");
    let is_source = |lower: &str| {
        ["based on", "adapted from", "inspired by"]
            .iter()
            .any(|p| lower.starts_with(p))
    };
    // Title: first non-empty centered block.
    while i < lines.len() && lines[i].1.is_empty() {
        i += 1;
    }
    let mut title = Vec::new();
    while i < lines.len()
        && !lines[i].1.is_empty()
        && centered(&lines[i].0)
        && !is_credit(&lines[i].1.to_lowercase())
        && !is_source(&lines[i].1.to_lowercase())
    {
        title.push(lines[i].1.clone());
        i += 1;
    }
    if !title.is_empty() {
        page.set("Title", title.join("\n"));
    }
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut authors = Vec::new();
    let mut credit: Option<String> = None;
    let mut source = Vec::new();
    let mut authors_done = false;
    while i < lines.len() {
        let (align, text) = &lines[i];
        i += 1;
        if text.is_empty() {
            if !authors.is_empty() {
                authors_done = true;
            }
            continue;
        }
        let lower = text.to_lowercase();
        if centered(align) {
            if credit.is_none() && authors.is_empty() && is_credit(&lower) {
                credit = Some(text.clone());
            } else if is_source(&lower) {
                authors_done = authors_done || !authors.is_empty();
                source.push(text.clone());
            } else if credit.is_some() && !authors_done {
                authors.push(text.clone());
            } else {
                source.push(text.clone());
            }
        } else if align == "Right" {
            right.push(text.clone());
        } else {
            left.push(text.clone());
        }
    }
    if let Some(c) = credit.filter(|c| !c.eq_ignore_ascii_case("written by")) {
        page.set("Credit", c);
    }
    if !authors.is_empty() {
        page.set("Author", authors.join("\n"));
    }
    if !source.is_empty() {
        page.set("Source", source.join("\n"));
    }
    if !right.is_empty() {
        page.set("Draft date", right.join("\n"));
    }
    if !left.is_empty() {
        page.set("Contact", left.join("\n"));
    }
    page
}

/// Parse FDX bytes.
pub fn parse(bytes: &[u8]) -> AppResult<ImportOutcome> {
    let root = xml::parse(bytes).map_err(|e| match e {
        xml::XmlError::Malformed(d) => malformed(d),
        xml::XmlError::TooLarge => malformed("document exceeds structural limits"),
    })?;
    if root.name != "FinalDraft" {
        return Err(AppError::import(
            "fdx_invalid",
            "This file is not a Final Draft screenplay (.fdx). Your current project was not changed.",
        )
        .with_detail(format!("root element {}", root.name)));
    }
    let content = root
        .child("Content")
        .ok_or_else(|| malformed("missing <Content>"))?;
    let mut st = State {
        doc: ScreenplayDoc::default(),
        unknown_types: BTreeMap::new(),
        over_limit_paragraphs: 0,
        inline_notes: 0,
        revisions: BTreeMap::new(),
        custom_looks: BTreeMap::new(),
    };
    // Bounded: a file may declare at most MAX_ELEMENT_SETTINGS paragraph styles that count.
    for es in root
        .children_named("ElementSettings")
        .take(MAX_ELEMENT_SETTINGS)
    {
        if let Some(ty) = es.attr("Type").map(str::trim)
            && builtin_kind(ty, false).is_none()
            && ty != "Scene Heading"
            && !ty.is_empty()
        {
            // The first settings of a type win, as in Final Draft.
            st.custom_looks
                .entry(custom_type_name(ty))
                .or_insert_with(|| look_from_settings(es));
        }
    }
    if let Some(revs) = root.child("Revisions") {
        for r in revs.children_named("Revision") {
            if let (Some(id), Some(name)) = (r.attr("ID"), r.attr("Name")) {
                st.revisions.insert(
                    id.to_string(),
                    name.trim().trim_end_matches(" Rev.").trim().to_string(),
                );
            }
        }
    }
    // Paragraphs, and dual dialogue written straight under Content by older files.
    // Anything else under Content is reported, never dropped silently.
    let mut unknown_blocks: BTreeMap<String, usize> = BTreeMap::new();
    for node in content.elements() {
        match node.name.as_str() {
            "Paragraph" => handle_paragraph(&mut st, node, false),
            "DualDialogue" => {
                for inner in node.children_named("Paragraph") {
                    handle_paragraph(&mut st, inner, true);
                }
            }
            other if unknown_blocks.len() < MAX_LISTED_CUSTOM_TYPES => {
                *unknown_blocks
                    .entry(other.chars().take(40).collect())
                    .or_default() += 1;
            }
            _ => {}
        }
    }
    for (name, n) in &unknown_blocks {
        st.doc.warnings.push(Warning::attention(
            "fdx_unknown_block",
            format!("{n} “{name}” block(s) in the script could not be read and were left out; check the script against the original."),
        ));
    }
    if let Some(tp) = root.child("TitlePage") {
        st.doc.title_page = parse_title_page(tp);
    }
    // Report what was ignored.
    let kinds = st.unknown_types.len();
    if kinds > MAX_LISTED_CUSTOM_TYPES {
        st.doc.warnings.push(Warning::attention(
            "fdx_many_custom_elements",
            format!(
                "This file uses {kinds} paragraph types that OpenFrame has no built-in element for; the first {MAX_LISTED_CUSTOM_TYPES} are listed."
            ),
        ));
    }
    if st.over_limit_paragraphs > 0 {
        st.doc.warnings.push(Warning::attention(
            "fdx_custom_types_over_limit",
            format!(
                "This file has more than {MAX_CUSTOM_TYPES} custom paragraph types. {} paragraph(s) of the others were imported as action; review them after the import.",
                st.over_limit_paragraphs
            ),
        ));
    }
    for (ty, n) in st.unknown_types.iter().take(MAX_LISTED_CUSTOM_TYPES) {
        st.doc.warnings.push(Warning::info(
            "fdx_custom_element",
            format!(
                "{n} “{ty}” paragraph(s) were kept as the custom element “{ty}”. You can change how it looks under Advanced."
            ),
        ));
    }
    let ignored: Vec<&str> = [
        "ScriptNotes",
        "SmartType",
        "HeaderAndFooter",
        "PageLayout",
        "TagData",
        "Cast",
        "Watermarking",
        "ElementSettings",
        "Macros",
        "Actors",
        "SceneNumberOptions",
        "SplitState",
        "DisplayBoards",
        "MoresAndContinueds",
        "LockedPages",
        "Revisions",
        "TextState",
        "Spelling",
        "Tags",
        "ListItems",
    ]
    .into_iter()
    .filter(|n| root.child(n).is_some())
    .collect();
    if root
        .child("ScriptNotes")
        .map(|n| n.elements().next().is_some())
        .unwrap_or(false)
    {
        st.doc.warnings.push(Warning::attention(
            "fdx_script_notes_ignored",
            "Final Draft ScriptNotes are not imported. Keep the original file if you need them.",
        ));
    }
    if root
        .child("TagData")
        .map(|n| n.elements().next().is_some())
        .unwrap_or(false)
    {
        st.doc.warnings.push(Warning::info(
            "fdx_tags_ignored",
            "Final Draft tagging (breakdown tags) was not imported.",
        ));
    }
    if st.inline_notes > 0 {
        st.doc.warnings.push(Warning::info(
            "fdx_script_notes_kept",
            format!(
                "{} Final Draft script note(s) attached to paragraphs were kept as notes in the scene (notes never print).",
                st.inline_notes
            ),
        ));
    }
    // Settings sections are layout only: no screenplay text is lost, but say what was not used.
    let layout_only: Vec<&str> = ignored
        .into_iter()
        .filter(|n| {
            !matches!(
                *n,
                "ScriptNotes" | "TagData" | "ElementSettings" | "Revisions"
            )
        })
        .collect();
    if !layout_only.is_empty() {
        st.doc.warnings.push(Warning::info(
            "fdx_settings_not_used",
            format!(
                "Final Draft settings in this file ({}) were not used; OpenFrame applies the screenplay's own page setup.",
                layout_only.join(", ")
            ),
        ));
    }
    st.doc
        .scenes
        .retain(|s| !(s.heading.is_empty() && s.elements.is_empty()));
    if st.doc.scenes.is_empty() {
        return Err(AppError::import(
            "fdx_empty",
            "This Final Draft file contains no screenplay text. Your current project was not changed.",
        ));
    }
    for (idx, s) in st.doc.scenes.iter().enumerate() {
        if !s.heading.is_empty() && s.elements.is_empty() {
            st.doc.warnings.push(
                Warning::attention(
                    "empty_scene",
                    format!("“{}” has no content under its heading.", s.heading),
                )
                .at(idx),
            );
        }
    }
    Ok(ImportOutcome {
        doc: st.doc,
        confidence: Confidence::high(),
        format: SourceFormat::Fdx,
    })
}

// ------------------------------------------------------------------ writer

#[derive(Debug, Clone, Copy, Default)]
pub struct FdxOptions {
    pub scene_numbers: bool,
    pub include_notes: bool,
    pub revision_marks: bool,
    pub title_page: bool,
}

fn fdx_type(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Action | ElementKind::Centered => "Action",
        ElementKind::Character => "Character",
        ElementKind::Dialogue | ElementKind::Lyric => "Dialogue",
        ElementKind::Parenthetical => "Parenthetical",
        ElementKind::Transition => "Transition",
        ElementKind::Shot => "Shot",
        _ => "General",
    }
}

fn write_paragraph(
    w: &mut XmlWriter,
    e: &Element,
    rev_ids: &BTreeMap<String, usize>,
    opts: &FdxOptions,
    new_page: bool,
) {
    let ty = match (&e.kind, &e.custom) {
        (ElementKind::Custom, Some(c)) => c.export_name.as_str(),
        _ => fdx_type(e.kind),
    };
    let mut attrs: Vec<(&str, &str)> = vec![("Type", ty)];
    if e.kind == ElementKind::Centered {
        attrs.push(("Alignment", "Center"));
    }
    if new_page {
        attrs.push(("StartsNewPage", "Yes"));
    }
    w.open("Paragraph", &attrs);
    let text = match e.kind {
        ElementKind::Note => format!("[Note: {}]", e.text.trim()),
        // Text is written as stored: Final Draft shows these elements in capitals through
        // their element settings, and re-importing must give back the same text.
        ElementKind::Character | ElementKind::Transition | ElementKind::Shot => {
            e.text.trim().to_string()
        }
        _ => e.text.clone(),
    };
    let rev = e
        .revision
        .as_ref()
        .filter(|_| opts.revision_marks)
        .and_then(|r| rev_ids.get(r))
        .map(|n| n.to_string());
    match (&rev, e.kind == ElementKind::Lyric) {
        (Some(id), _) => w.open("Text", &[("RevisionID", id.as_str())]),
        (None, true) => w.open("Text", &[("Style", "Italic")]),
        _ => w.open("Text", &[]),
    }
    w.text(&text);
    w.close("Text");
    w.close("Paragraph");
}

/// `ElementSettings` for every custom element in the document, so Final Draft
/// (and a later import) knows how each custom paragraph type looks.
fn write_custom_settings(w: &mut XmlWriter, doc: &ScreenplayDoc) {
    let mut seen: Vec<&str> = Vec::new();
    for e in doc.scenes.iter().flat_map(|s| s.elements.iter()) {
        let Some(c) = e.custom.as_ref().filter(|_| e.kind == ElementKind::Custom) else {
            continue;
        };
        if seen.contains(&c.export_name.as_str()) {
            continue;
        }
        seen.push(c.export_name.as_str());
        let look = rules::rule_for(&c.look);
        let align = match look.align {
            rules::Align::Left => "Left",
            rules::Align::Center => "Center",
            rules::Align::Right => "Right",
        };
        let style: Vec<&str> = [(c.uppercase, "AllCaps"), (c.italic, "Italic")]
            .into_iter()
            .filter(|(on, _)| *on)
            .map(|(_, s)| s)
            .collect();
        let style = style.join("+");
        let left = format!("{:.2}", look.left_in);
        let right = format!("{:.2}", look.left_in + look.width_in);
        let space = (c.space_before.min(3) as u32 * 12).to_string();
        let paginate = match look.kind {
            "character" | "parenthetical" | "dialogue" | "transition" | "shot" => look.fdx_type,
            _ => "Action",
        };
        w.open("ElementSettings", &[("Type", c.export_name.as_str())]);
        w.empty(
            "FontSpec",
            &[
                ("Font", "Courier Final Draft"),
                ("Size", "12"),
                ("Style", style.as_str()),
            ],
        );
        w.empty(
            "ParagraphSpec",
            &[
                ("Alignment", align),
                ("FirstIndent", "0.00"),
                ("Leading", "Regular"),
                ("LeftIndent", left.as_str()),
                ("RightIndent", right.as_str()),
                ("SpaceBefore", space.as_str()),
                ("Spacing", "1"),
                ("StartsNewPage", "No"),
            ],
        );
        w.empty(
            "Behavior",
            &[
                ("PaginateAs", paginate),
                ("ReturnKey", "Action"),
                ("Shortcut", ""),
            ],
        );
        w.close("ElementSettings");
        w.raw("\n");
    }
}

fn tp_paragraph(w: &mut XmlWriter, align: &str, text: &str) {
    w.open("Paragraph", &[("Alignment", align), ("Type", "Action")]);
    w.open("Text", &[]);
    w.text(text);
    w.close("Text");
    w.close("Paragraph");
}

/// Write a valid Final Draft 8+ XML document.
pub fn write(doc: &ScreenplayDoc, opts: &FdxOptions) -> String {
    let mut rev_ids: BTreeMap<String, usize> = BTreeMap::new();
    if opts.revision_marks {
        for s in &doc.scenes {
            for e in &s.elements {
                if let Some(r) = &e.revision {
                    let next = rev_ids.len() + 1;
                    rev_ids.entry(r.clone()).or_insert(next);
                }
            }
        }
    }
    let mut w = XmlWriter::new();
    w.open(
        "FinalDraft",
        &[
            ("DocumentType", "Script"),
            ("Template", "No"),
            ("Version", "5"),
        ],
    );
    w.raw("\n");
    w.open("Content", &[]);
    w.raw("\n");
    let mut number = 0usize;
    // An explicit page break is written as `StartsNewPage` on the next paragraph.
    let mut page_break = false;
    for scene in &doc.scenes {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let num = if opts.scene_numbers {
                Some(scene.number.clone().unwrap_or_else(|| number.to_string()))
            } else {
                scene.number.clone()
            };
            let mut attrs: Vec<(&str, &str)> = Vec::new();
            if let Some(n) = &num {
                attrs.push(("Number", n.as_str()));
            }
            if std::mem::take(&mut page_break) {
                attrs.push(("StartsNewPage", "Yes"));
            }
            attrs.push(("Type", "Scene Heading"));
            w.open("Paragraph", &attrs);
            w.open("Text", &[]);
            w.text(scene.heading.trim());
            w.close("Text");
            w.close("Paragraph");
            w.raw("\n");
        }
        let els: Vec<&Element> = scene
            .elements
            .iter()
            .filter(|e| match e.kind {
                ElementKind::Note => opts.include_notes,
                ElementKind::Section | ElementKind::Synopsis => false,
                _ => true,
            })
            .collect();
        let mut i = 0;
        while i < els.len() {
            let e = els[i];
            if e.kind == ElementKind::PageBreak {
                page_break = true;
                i += 1;
                continue;
            }
            if e.dual && e.kind.is_speech() {
                let mut j = i;
                while j < els.len() && els[j].dual && els[j].kind.is_speech() {
                    j += 1;
                }
                if std::mem::take(&mut page_break) {
                    w.open("Paragraph", &[("StartsNewPage", "Yes")]);
                } else {
                    w.open("Paragraph", &[]);
                }
                w.open("DualDialogue", &[]);
                for d in &els[i..j] {
                    write_paragraph(&mut w, d, &rev_ids, opts, false);
                }
                w.close("DualDialogue");
                w.close("Paragraph");
                w.raw("\n");
                i = j;
                continue;
            }
            write_paragraph(&mut w, e, &rev_ids, opts, std::mem::take(&mut page_break));
            w.raw("\n");
            i += 1;
        }
    }
    w.close("Content");
    w.raw("\n");
    write_custom_settings(&mut w, doc);
    if opts.title_page && !doc.title_page.is_empty() {
        let tp = &doc.title_page;
        w.open("TitlePage", &[]);
        w.open("Content", &[]);
        for _ in 0..16 {
            tp_paragraph(&mut w, "Center", "");
        }
        if let Some(t) = tp.title() {
            for l in t.split('\n') {
                tp_paragraph(&mut w, "Center", l.trim());
            }
        }
        if let Some(a) = tp.author() {
            tp_paragraph(&mut w, "Center", "");
            tp_paragraph(&mut w, "Center", tp.get("Credit").unwrap_or("Written by"));
            tp_paragraph(&mut w, "Center", "");
            for l in a.split('\n') {
                tp_paragraph(&mut w, "Center", l.trim());
            }
        }
        if let Some(s) = tp.get("Source") {
            tp_paragraph(&mut w, "Center", "");
            for l in s.split('\n') {
                tp_paragraph(&mut w, "Center", l.trim());
            }
        }
        for _ in 0..12 {
            tp_paragraph(&mut w, "Left", "");
        }
        for key in ["Draft", "Draft date", "Date", "Revision", "Representation"] {
            if let Some(d) = tp.get(key) {
                for l in d.split('\n') {
                    tp_paragraph(&mut w, "Right", l.trim());
                }
            }
        }
        for key in ["Contact", "Notes", "Copyright"] {
            if let Some(c) = tp.get(key) {
                for l in c.split('\n') {
                    tp_paragraph(&mut w, "Left", l.trim());
                }
            }
        }
        w.close("Content");
        w.close("TitlePage");
        w.raw("\n");
    }
    if !rev_ids.is_empty() {
        w.open(
            "Revisions",
            &[
                ("ActiveSet", "1"),
                ("RevisionMode", "Off"),
                ("RevisionsShown", "Active"),
            ],
        );
        let mut by_id: Vec<(&String, &usize)> = rev_ids.iter().collect();
        by_id.sort_by_key(|(_, id)| **id);
        for (name, id) in by_id {
            let id = id.to_string();
            let label = format!("{name} Rev.");
            w.empty(
                "Revision",
                &[
                    ("ID", id.as_str()),
                    ("Mark", "*"),
                    ("Name", label.as_str()),
                    ("FullRevision", "No"),
                ],
            );
        }
        w.close("Revisions");
        w.raw("\n");
    }
    w.close("FinalDraft");
    w.raw("\n");
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_fdx_with_dual_and_unknown_types() {
        let src = r#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Template="No" Version="4">
<Content>
<Paragraph Number="7" Type="Scene Heading"><SceneProperties Length="1/8"/><Text>INT. HOUSE - DAY</Text></Paragraph>
<Paragraph Type="Action"><Text>Rain </Text><Text Style="Bold">hammers</Text><Text> the roof.</Text></Paragraph>
<Paragraph><DualDialogue>
<Paragraph Type="Character"><Text>MEERA</Text></Paragraph>
<Paragraph Type="Dialogue"><Text>Now!</Text></Paragraph>
<Paragraph Type="Character"><Text>ARJUN</Text></Paragraph>
<Paragraph Type="Dialogue"><Text>Wait!</Text></Paragraph>
</DualDialogue></Paragraph>
<Paragraph Type="Cast List"><Text>MEERA, ARJUN</Text></Paragraph>
</Content>
</FinalDraft>"#;
        let d = parse(src.as_bytes()).unwrap().doc;
        assert_eq!(d.scenes.len(), 1);
        assert_eq!(d.scenes[0].number.as_deref(), Some("7"));
        assert_eq!(d.scenes[0].elements[0].text, "Rain hammers the roof.");
        assert!(d.scenes[0].elements[1..5].iter().all(|e| e.dual));
        assert!(d.warnings.iter().any(|w| w.code == "fdx_custom_element"));
    }

    #[test]
    fn rejects_non_fdx_and_malformed() {
        assert_eq!(
            parse(b"<html><body/></html>").unwrap_err().code_str(),
            "import.fdx_invalid"
        );
        assert_eq!(
            parse(b"<FinalDraft><Content><Paragraph>")
                .unwrap_err()
                .code_str(),
            "import.fdx_malformed"
        );
    }
}
