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
//!
//! Anything else (Cast List, act breaks, outline paragraphs, script notes,
//! tags, smart-type lists, page layout) is ignored gracefully and reported in
//! the import warnings; paragraphs with unknown types keep their text as
//! action so no screenplay content is lost.

use std::collections::BTreeMap;

use crate::heuristics::{Confidence, split_scene_number};
use crate::model::{Element, ElementKind, Scene, ScreenplayDoc, TitlePage, Warning};
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

struct State {
    doc: ScreenplayDoc,
    unknown_types: BTreeMap<String, usize>,
    revisions: BTreeMap<String, String>,
}

impl State {
    fn scene(&mut self) -> &mut Scene {
        if self.doc.scenes.is_empty() {
            self.doc.scenes.push(Scene::default());
        }
        self.doc.scenes.last_mut().unwrap()
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
        if heading.is_empty() {
            return;
        }
        st.doc.scenes.push(Scene {
            heading,
            number,
            elements: Vec::new(),
        });
        return;
    }
    if t.trim().is_empty() {
        return;
    }
    let kind = match ty {
        "Action" | "General" => {
            if p.attr("Alignment") == Some("Center") {
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
        other => {
            *st.unknown_types.entry(other.to_string()).or_default() += 1;
            ElementKind::Action
        }
    };
    let text = match kind {
        ElementKind::Action | ElementKind::Centered | ElementKind::Dialogue => t.to_string(),
        _ => t.trim().to_string(),
    };
    let mut e = Element::new(kind, text).dual(dual);
    e.revision = revision;
    st.scene().elements.push(e);
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
    // Title: first non-empty centered block.
    while i < lines.len() && lines[i].1.is_empty() {
        i += 1;
    }
    let mut title = Vec::new();
    while i < lines.len() && !lines[i].1.is_empty() && centered(&lines[i].0) {
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
            if credit.is_none()
                && authors.is_empty()
                && (lower == "written by" || lower == "by" || lower.ends_with(" by"))
            {
                credit = Some(text.clone());
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
        revisions: BTreeMap::new(),
    };
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
    for p in content.children_named("Paragraph") {
        handle_paragraph(&mut st, p, false);
    }
    if let Some(tp) = root.child("TitlePage") {
        st.doc.title_page = parse_title_page(tp);
    }
    // Report what was ignored.
    for (ty, n) in &st.unknown_types {
        st.doc.warnings.push(Warning::info(
            "unsupported_fdx_paragraph",
            format!(
                "{n} “{ty}” paragraph(s) have no OpenFrame equivalent and were imported as action."
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
    let _ = ignored; // Formatting/settings sections are layout-only; ignoring them loses no screenplay content.
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
) {
    let ty = fdx_type(e.kind);
    let mut attrs: Vec<(&str, &str)> = vec![("Type", ty)];
    if e.kind == ElementKind::Centered {
        attrs.push(("Alignment", "Center"));
    }
    w.open("Paragraph", &attrs);
    let text = match e.kind {
        ElementKind::Note => format!("[Note: {}]", e.text.trim()),
        ElementKind::Character | ElementKind::Transition | ElementKind::Shot => {
            e.text.trim().to_uppercase()
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
    for scene in &doc.scenes {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let num = if opts.scene_numbers {
                Some(scene.number.clone().unwrap_or_else(|| number.to_string()))
            } else {
                scene.number.clone()
            };
            match &num {
                Some(n) => w.open(
                    "Paragraph",
                    &[("Number", n.as_str()), ("Type", "Scene Heading")],
                ),
                None => w.open("Paragraph", &[("Type", "Scene Heading")]),
            }
            w.open("Text", &[]);
            w.text(&scene.heading.trim().to_uppercase());
            w.close("Text");
            w.close("Paragraph");
            w.raw("\n");
        }
        let els: Vec<&Element> = scene
            .elements
            .iter()
            .filter(|e| match e.kind {
                ElementKind::Note => opts.include_notes,
                ElementKind::Section | ElementKind::Synopsis | ElementKind::PageBreak => false,
                _ => true,
            })
            .collect();
        let mut i = 0;
        while i < els.len() {
            let e = els[i];
            if e.dual && e.kind.is_speech() {
                let mut j = i;
                while j < els.len() && els[j].dual && els[j].kind.is_speech() {
                    j += 1;
                }
                w.open("Paragraph", &[]);
                w.open("DualDialogue", &[]);
                for d in &els[i..j] {
                    write_paragraph(&mut w, d, &rev_ids, opts);
                }
                w.close("DualDialogue");
                w.close("Paragraph");
                w.raw("\n");
                i = j;
                continue;
            }
            write_paragraph(&mut w, e, &rev_ids, opts);
            w.raw("\n");
            i += 1;
        }
    }
    w.close("Content");
    w.raw("\n");
    if opts.title_page && !doc.title_page.is_empty() {
        let tp = &doc.title_page;
        w.open("TitlePage", &[]);
        w.open("Content", &[]);
        for _ in 0..16 {
            tp_paragraph(&mut w, "Center", "");
        }
        if let Some(t) = tp.title() {
            for l in t.split('\n') {
                tp_paragraph(&mut w, "Center", &l.trim().to_uppercase());
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
        for key in ["Draft", "Draft date", "Revision"] {
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
        assert!(
            d.warnings
                .iter()
                .any(|w| w.code == "unsupported_fdx_paragraph")
        );
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
