//! DOCX (Office Open XML) screenplay import and export.
//!
//! Import reads the package through `openframe_security::archive` (entry-name
//! validation, size and compression-ratio limits — zip bombs are rejected
//! before anything is inflated), then classifies `word/document.xml`
//! paragraphs:
//!
//! 1. paragraph styles named like screenplay elements ("Scene Heading",
//!    "Action", "Character", "Dialogue", "Parenthetical", "Transition", "Shot")
//!    are trusted;
//! 2. otherwise indentation (`w:ind`, inherited from styles) and alignment feed
//!    the shared screenplay classifier together with text patterns.
//!
//! Character formatting (bold, italics, fonts, colours) and Word's own heading
//! styles are never treated as screenplay structure (FSD §19.5).
//!
//! Export writes a minimal valid OOXML package with screenplay paragraph
//! styles (Courier New 12pt, US Letter, 1.5" left margin), an unnumbered title
//! page section and page numbers from the second script page.

use std::collections::HashMap;
use std::path::Path;

use openframe_domain::{AppError, AppResult};
use openframe_security::archive;

use crate::heuristics::{ClassifyOptions, Confidence, LineHint, RawLine, classify};
use crate::model::{Element, ElementKind, ScreenplayDoc, Warning};
use crate::xml::{self, Child, Node, XmlWriter};
use crate::{ImportOutcome, SourceFormat};

const MAX_DOCUMENT_XML: u64 = 32 << 20;
const MAX_STYLES_XML: u64 = 16 << 20;

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::import(
        "docx_invalid",
        "This file is not a readable Word document (.docx). Your current project was not changed.",
    )
    .with_detail(detail)
}

#[derive(Debug, Clone, Default)]
struct StyleInfo {
    name: String,
    based_on: Option<String>,
    ind_left: Option<i64>,
    jc: Option<String>,
}

fn twips(v: &str) -> Option<i64> {
    v.trim().parse::<f64>().ok().map(|f| f as i64)
}

fn para_props(ppr: Option<&Node>) -> (Option<String>, Option<i64>, Option<String>, bool) {
    let Some(ppr) = ppr else {
        return (None, None, None, false);
    };
    let style = ppr
        .child("pStyle")
        .and_then(|s| s.attr("val"))
        .map(|s| s.to_string());
    let ind = ppr
        .child("ind")
        .and_then(|i| i.attr("left").or_else(|| i.attr("start")))
        .and_then(twips);
    let jc = ppr
        .child("jc")
        .and_then(|j| j.attr("val"))
        .map(|s| s.to_string());
    let page_break_before = ppr
        .child("pageBreakBefore")
        .map(|n| n.attr("val") != Some("0") && n.attr("val") != Some("false"))
        .unwrap_or(false);
    (style, ind, jc, page_break_before)
}

fn parse_styles(root: &Node) -> HashMap<String, StyleInfo> {
    let mut map = HashMap::new();
    for s in root.children_named("style") {
        let Some(id) = s.attr("styleId") else {
            continue;
        };
        let name = s
            .child("name")
            .and_then(|n| n.attr("val"))
            .unwrap_or(id)
            .to_string();
        let based_on = s
            .child("basedOn")
            .and_then(|b| b.attr("val"))
            .map(|b| b.to_string());
        let (_, ind, jc, _) = para_props(s.child("pPr"));
        map.insert(
            id.to_string(),
            StyleInfo {
                name,
                based_on,
                ind_left: ind,
                jc,
            },
        );
    }
    map
}

fn resolve<'a>(
    styles: &'a HashMap<String, StyleInfo>,
    id: &str,
    f: impl Fn(&'a StyleInfo) -> Option<&'a str>,
) -> Option<&'a str> {
    let mut cur = styles.get(id);
    let mut guard = 0;
    while let Some(s) = cur {
        if let Some(v) = f(s) {
            return Some(v);
        }
        guard += 1;
        if guard > 20 {
            break;
        }
        cur = s.based_on.as_deref().and_then(|b| styles.get(b));
    }
    None
}

fn resolve_ind(styles: &HashMap<String, StyleInfo>, id: &str) -> Option<i64> {
    let mut cur = styles.get(id);
    let mut guard = 0;
    while let Some(s) = cur {
        if s.ind_left.is_some() {
            return s.ind_left;
        }
        guard += 1;
        if guard > 20 {
            break;
        }
        cur = s.based_on.as_deref().and_then(|b| styles.get(b));
    }
    None
}

fn hint_for_style(name: &str) -> Option<LineHint> {
    let n: String = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    Some(match n.as_str() {
        "sceneheading" | "slugline" | "scene" => LineHint::Heading,
        "action" | "general" | "description" => LineHint::Kind(ElementKind::Action),
        "character" | "charactername" | "cue" => LineHint::Kind(ElementKind::Character),
        "dialogue" | "dialog" => LineHint::Kind(ElementKind::Dialogue),
        "parenthetical" | "parenthesis" | "wryly" => LineHint::Kind(ElementKind::Parenthetical),
        "transition" => LineHint::Kind(ElementKind::Transition),
        "shot" => LineHint::Kind(ElementKind::Shot),
        "screenplaynote" => LineHint::Kind(ElementKind::Note),
        "centered" | "centeredtext" => LineHint::Kind(ElementKind::Centered),
        "lyrics" | "lyric" => LineHint::Kind(ElementKind::Lyric),
        _ => return None,
    })
}

#[derive(Debug, Clone)]
struct Para {
    text: String,
    style_name: Option<String>,
    ind: Option<i64>,
    jc: Option<String>,
    page_break_before: bool,
    in_table: bool,
}

fn run_text(node: &Node, out: &mut String, page_break: &mut bool) {
    for c in &node.children {
        match c {
            Child::Text(_) => {}
            Child::Node(n) => match n.name.as_str() {
                "t" => out.push_str(&n.text()),
                "tab" | "ptab" => out.push('\t'),
                "br" | "cr" => {
                    if n.attr("type") == Some("page") {
                        *page_break = true;
                    } else {
                        out.push('\n');
                    }
                }
                "noBreakHyphen" => out.push('-'),
                // Deleted tracked changes, field instructions and properties are not text.
                "del" | "instrText" | "delText" | "rPr" | "pPr" | "fldData"
                | "footnoteReference" | "commentReference" => {}
                _ => run_text(n, out, page_break),
            },
        }
    }
}

fn collect_paras(
    node: &Node,
    styles: &HashMap<String, StyleInfo>,
    in_table: bool,
    out: &mut Vec<Para>,
) {
    for n in node.elements() {
        match n.name.as_str() {
            "p" => {
                let (style_id, ind, jc, pbb) = para_props(n.child("pPr"));
                let style_name = style_id
                    .as_deref()
                    .and_then(|id| styles.get(id))
                    .map(|s| s.name.clone())
                    .or(style_id.clone());
                let ind =
                    ind.or_else(|| style_id.as_deref().and_then(|id| resolve_ind(styles, id)));
                let jc = jc.or_else(|| {
                    style_id
                        .as_deref()
                        .and_then(|id| resolve(styles, id, |s| s.jc.as_deref()))
                        .map(|s| s.to_string())
                });
                let mut text = String::new();
                let mut page_break = false;
                run_text(n, &mut text, &mut page_break);
                // Trailing tab + asterisk is an OpenFrame revision mark, not text.
                let text = text
                    .strip_suffix("\t*")
                    .map(|s| s.to_string())
                    .unwrap_or(text);
                out.push(Para {
                    text,
                    style_name,
                    ind,
                    jc,
                    page_break_before: pbb,
                    in_table,
                });
                // A section break (sectPr inside pPr) starts a new page, like a page break.
                let section_break = n.child("pPr").and_then(|p| p.child("sectPr")).is_some();
                if page_break || section_break {
                    out.push(Para {
                        text: String::new(),
                        style_name: None,
                        ind: None,
                        jc: None,
                        page_break_before: true,
                        in_table,
                    });
                }
            }
            "tbl" | "tr" | "tc" => collect_paras(n, styles, true, out),
            "sdt" | "sdtContent" | "customXml" | "smartTag" | "ins" => {
                collect_paras(n, styles, in_table, out)
            }
            _ => {}
        }
    }
}

/// Parse a .docx file.
pub fn parse(path: &Path) -> AppResult<ImportOutcome> {
    // Nothing is extracted to disk: only two named parts are read, each through
    // `read_entry`, which enforces a hard byte cap on the *decompressed* stream
    // (zip bombs fail there). `archive::inspect` cannot be used for OOXML because
    // its entry-name rule rejects the mandatory `_rels/.rels` part. The declared directory
    // size is still bounded up front (a DOCX has a few dozen parts).
    archive::check_directory(path, archive::ArchiveLimits::DOCUMENT.max_entries as u64)?;
    let content_types = archive::read_entry(path, "[Content_Types].xml", MAX_STYLES_XML)?;
    if content_types.is_none() {
        return Err(invalid("[Content_Types].xml missing"));
    }
    let doc_xml = archive::read_entry(path, "word/document.xml", MAX_DOCUMENT_XML)?
        .ok_or_else(|| invalid("word/document.xml missing"))?;
    let styles = match archive::read_entry(path, "word/styles.xml", MAX_STYLES_XML)? {
        Some(bytes) => xml::parse(&bytes)
            .map(|r| parse_styles(&r))
            .unwrap_or_default(),
        None => HashMap::new(),
    };
    let root = xml::parse(&doc_xml).map_err(|e| invalid(format!("{e:?}")))?;
    let body = root.child("body").ok_or_else(|| invalid("no body"))?;
    let mut paras = Vec::new();
    collect_paras(body, &styles, false, &mut paras);
    parse_paragraphs(paras)
}

fn parse_paragraphs(paras: Vec<Para>) -> AppResult<ImportOutcome> {
    let mut warnings = Vec::new();
    let has_screenplay_styles = paras
        .iter()
        .any(|p| p.style_name.as_deref().and_then(hint_for_style).is_some());
    let word_headings = paras.iter().any(|p| {
        p.style_name
            .as_deref()
            .map(|s| s.to_lowercase().starts_with("heading"))
            .unwrap_or(false)
    });

    // Title page: content before the first page break when no scene heading precedes it.
    let first_break = paras.iter().position(|p| p.page_break_before);
    let mut start = 0;
    let mut title_lines: Vec<(String, String)> = Vec::new();
    if let Some(fb) = first_break {
        let before = &paras[..fb];
        let has_heading = before.iter().any(|p| {
            crate::heuristics::heading_match(&p.text) != crate::heuristics::HeadingMatch::No
                || p.style_name.as_deref().and_then(hint_for_style) == Some(LineHint::Heading)
        });
        let non_empty = before.iter().filter(|p| !p.text.trim().is_empty()).count();
        if !has_heading && non_empty > 0 && non_empty <= 25 {
            for p in before {
                let align = match p.jc.as_deref() {
                    Some("center") => "Center",
                    Some("right") | Some("end") => "Right",
                    _ => "Left",
                };
                title_lines.push((align.to_string(), p.text.trim().to_string()));
            }
            start = fb;
        }
    }

    // Indentation relative to the most common left indent (the action margin).
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for p in &paras[start..] {
        if !p.text.trim().is_empty() {
            *counts.entry(p.ind.unwrap_or(0)).or_default() += 1;
        }
    }
    let margin = counts
        .iter()
        .max_by_key(|(k, c)| (**c, -**k))
        .map(|(k, _)| *k)
        .unwrap_or(0);
    let deep = paras[start..]
        .iter()
        .filter(|p| !p.text.trim().is_empty() && p.ind.unwrap_or(0) - margin >= 720)
        .count();
    let total = paras[start..]
        .iter()
        .filter(|p| !p.text.trim().is_empty())
        .count();
    let use_indentation = total > 0 && deep * 10 >= total && deep >= 2;

    let mut lines = Vec::new();
    let mut gap = false;
    let mut new_page = false;
    for p in &paras[start..] {
        if p.page_break_before {
            new_page = true;
        }
        if p.text.trim().is_empty() {
            gap = true;
            continue;
        }
        let hint = p.style_name.as_deref().and_then(hint_for_style);
        let right = matches!(p.jc.as_deref(), Some("right") | Some("end"));
        let indent = if right {
            Some(4.5)
        } else if use_indentation {
            Some(((p.ind.unwrap_or(0) - margin).max(0)) as f32 / 1440.0)
        } else {
            None
        };
        // Soft line breaks inside one paragraph are the author's; keep them for action/dialogue.
        lines.push(RawLine {
            text: p.text.replace('\t', " ").trim().to_string(),
            indent,
            gap_before: gap || has_screenplay_styles,
            new_page,
            hint,
            dual: p.in_table
                && matches!(
                    hint,
                    Some(LineHint::Kind(
                        ElementKind::Character
                            | ElementKind::Dialogue
                            | ElementKind::Parenthetical
                            | ElementKind::Lyric
                    ))
                ),
        });
        gap = false;
        new_page = false;
    }
    let (mut doc, mut stats) = classify(
        &lines,
        ClassifyOptions {
            join_wrapped: true,
            use_indentation: use_indentation || has_screenplay_styles,
        },
    );

    if !title_lines.is_empty() {
        doc.title_page = crate::fdx::title_page_from_lines(&title_lines);
    }

    let hinted = lines.iter().filter(|l| l.hint.is_some()).count();
    let confidence = if !lines.is_empty() && hinted * 10 >= lines.len() * 9 {
        Confidence::high()
    } else {
        if hinted == 0 {
            stats.used_indentation = use_indentation;
        }
        // Structure inferred from layout is never reported as certain.
        stats
            .confidence()
            .capped(crate::heuristics::ConfidenceLevel::Medium, 0.79)
    };
    if stats.strong_headings + stats.uncertain_headings == 0 && stats.cues == 0 {
        return Err(crate::not_a_screenplay(SourceFormat::Docx));
    }
    if !has_screenplay_styles {
        warnings.push(Warning::info(
            "docx_heuristic",
            "This document has no screenplay paragraph styles, so its structure was recognised from text patterns and indentation. Review the preview.",
        ));
    }
    if word_headings {
        warnings.push(Warning::info(
            "docx_word_headings",
            "Word headings were read as ordinary text unless they look like scene headings.",
        ));
    }
    doc.warnings.splice(0..0, warnings);
    Ok(ImportOutcome {
        doc,
        confidence,
        format: SourceFormat::Docx,
    })
}

// ------------------------------------------------------------------ writer

#[derive(Debug, Clone, Copy, Default)]
pub struct DocxOptions {
    pub title_page: bool,
    pub scene_numbers: bool,
    pub include_notes: bool,
    pub revision_marks: bool,
    pub revision_info: bool,
}

const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const R_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn style_id(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Action => "Action",
        ElementKind::Character => "Character",
        ElementKind::Dialogue => "Dialogue",
        ElementKind::Parenthetical => "Parenthetical",
        ElementKind::Transition => "Transition",
        ElementKind::Shot => "Shot",
        ElementKind::Note => "ScreenplayNote",
        ElementKind::Centered => "Centered",
        ElementKind::Lyric => "Lyrics",
        _ => "Action",
    }
}

fn paragraph(w: &mut XmlWriter, style: &str, text: &str, revised: bool) {
    w.open("w:p", &[]);
    w.open("w:pPr", &[]);
    w.empty("w:pStyle", &[("w:val", style)]);
    w.close("w:pPr");
    let lines: Vec<&str> = text.split('\n').collect();
    for (i, l) in lines.iter().enumerate() {
        w.open("w:r", &[]);
        if i > 0 {
            w.empty("w:br", &[]);
        }
        w.open("w:t", &[("xml:space", "preserve")]);
        w.text(l);
        w.close("w:t");
        w.close("w:r");
    }
    if revised {
        w.open("w:r", &[]);
        w.empty("w:tab", &[]);
        w.open("w:t", &[]);
        w.text("*");
        w.close("w:t");
        w.close("w:r");
    }
    w.close("w:p");
}

fn page_break(w: &mut XmlWriter) {
    w.raw("<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>");
}

const SECT_PAGE: &str = "<w:pgSz w:w=\"12240\" w:h=\"15840\"/><w:pgMar w:top=\"1440\" w:right=\"1440\" w:bottom=\"1440\" w:left=\"2160\" w:header=\"720\" w:footer=\"720\" w:gutter=\"0\"/>";

fn document_xml(doc: &ScreenplayDoc, opts: &DocxOptions) -> String {
    let mut w = XmlWriter::new();
    w.open("w:document", &[("xmlns:w", W_NS), ("xmlns:r", R_NS)]);
    w.open("w:body", &[]);
    if opts.title_page && !doc.title_page.is_empty() {
        let tp = &doc.title_page;
        for _ in 0..14 {
            paragraph(&mut w, "TitlePageText", "", false);
        }
        if let Some(t) = tp.title() {
            for l in t.split('\n') {
                paragraph(&mut w, "TitlePageTitle", &l.trim().to_uppercase(), false);
            }
        }
        if let Some(a) = tp.author() {
            paragraph(&mut w, "TitlePageText", "", false);
            paragraph(
                &mut w,
                "TitlePageText",
                tp.get("Credit").unwrap_or("Written by"),
                false,
            );
            paragraph(&mut w, "TitlePageText", "", false);
            for l in a.split('\n') {
                paragraph(&mut w, "TitlePageText", l.trim(), false);
            }
        }
        if let Some(s) = tp.get("Source") {
            paragraph(&mut w, "TitlePageText", "", false);
            for l in s.split('\n') {
                paragraph(&mut w, "TitlePageText", l.trim(), false);
            }
        }
        for _ in 0..10 {
            paragraph(&mut w, "TitlePageLeft", "", false);
        }
        let mut right_keys = vec!["Draft", "Draft date"];
        if opts.revision_info {
            right_keys.push("Revision");
        }
        for key in right_keys {
            if let Some(d) = tp.get(key) {
                for l in d.split('\n') {
                    paragraph(&mut w, "TitlePageRight", l.trim(), false);
                }
            }
        }
        for key in ["Contact", "Notes", "Copyright"] {
            if let Some(c) = tp.get(key) {
                for l in c.split('\n') {
                    paragraph(&mut w, "TitlePageLeft", l.trim(), false);
                }
            }
        }
        // End of the title page section (no header, not numbered).
        w.raw(&format!(
            "<w:p><w:pPr><w:sectPr>{SECT_PAGE}<w:pgNumType w:start=\"0\"/></w:sectPr></w:pPr></w:p>"
        ));
    }
    let mut number = 0usize;
    for scene in &doc.scenes {
        if !scene.heading.trim().is_empty() {
            number += 1;
            let h = scene.heading.trim().to_uppercase();
            let n = scene.number.clone().unwrap_or_else(|| number.to_string());
            let text = if opts.scene_numbers {
                format!("{n}  {h}")
            } else {
                h
            };
            paragraph(&mut w, "SceneHeading", &text, false);
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
                page_break(&mut w);
                i += 1;
                continue;
            }
            if e.dual && e.kind.is_speech() {
                let mut j = i;
                while j < els.len() && els[j].dual && els[j].kind.is_speech() {
                    j += 1;
                }
                let run = &els[i..j];
                let split = run
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find(|(_, e)| e.kind == ElementKind::Character)
                    .map(|(k, _)| k)
                    .unwrap_or(run.len());
                w.raw("<w:tbl><w:tblPr><w:tblW w:w=\"8640\" w:type=\"dxa\"/><w:tblLayout w:type=\"fixed\"/><w:tblBorders><w:top w:val=\"nil\"/><w:left w:val=\"nil\"/><w:bottom w:val=\"nil\"/><w:right w:val=\"nil\"/><w:insideH w:val=\"nil\"/><w:insideV w:val=\"nil\"/></w:tblBorders></w:tblPr><w:tblGrid><w:gridCol w:w=\"4320\"/><w:gridCol w:w=\"4320\"/></w:tblGrid><w:tr>");
                for col in [&run[..split], &run[split..]] {
                    w.raw("<w:tc><w:tcPr><w:tcW w:w=\"4320\" w:type=\"dxa\"/></w:tcPr>");
                    if col.is_empty() {
                        w.raw("<w:p/>");
                    }
                    for d in col {
                        let style = match d.kind {
                            ElementKind::Character => "DualCharacter",
                            ElementKind::Parenthetical => "DualParenthetical",
                            _ => "DualDialogue",
                        };
                        let text = if d.kind == ElementKind::Character {
                            d.text.trim().to_uppercase()
                        } else {
                            d.text.clone()
                        };
                        paragraph(
                            &mut w,
                            style,
                            &text,
                            opts.revision_marks && d.revision.is_some(),
                        );
                    }
                    w.raw("</w:tc>");
                }
                w.raw("</w:tr></w:tbl>");
                i = j;
                continue;
            }
            let text = match e.kind {
                ElementKind::Character | ElementKind::Transition | ElementKind::Shot => {
                    e.text.trim().to_uppercase()
                }
                _ => e.text.clone(),
            };
            paragraph(
                &mut w,
                style_id(e.kind),
                &text,
                opts.revision_marks && e.revision.is_some(),
            );
            i += 1;
        }
    }
    w.raw(&format!(
        "<w:sectPr><w:headerReference w:type=\"default\" r:id=\"rIdHeader1\"/>{SECT_PAGE}<w:pgNumType w:start=\"1\"/><w:titlePg/></w:sectPr>"
    ));
    w.close("w:body");
    w.close("w:document");
    w.out
}

fn style_def(id: &str, name: &str, ppr: &str, rpr: &str) -> String {
    format!(
        "<w:style w:type=\"paragraph\" w:customStyle=\"1\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/><w:basedOn w:val=\"Normal\"/><w:qFormat/><w:pPr>{ppr}</w:pPr><w:rPr>{rpr}</w:rPr></w:style>"
    )
}

fn styles_xml() -> String {
    let before = "<w:spacing w:before=\"240\" w:after=\"0\"/>";
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n");
    s.push_str(&format!("<w:styles xmlns:w=\"{W_NS}\">"));
    s.push_str("<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Courier New\" w:hAnsi=\"Courier New\" w:cs=\"Courier New\" w:eastAsia=\"Courier New\"/><w:sz w:val=\"24\"/><w:szCs w:val=\"24\"/><w:lang w:val=\"en-US\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:before=\"0\" w:after=\"0\" w:line=\"240\" w:lineRule=\"exact\"/></w:pPr></w:pPrDefault></w:docDefaults>");
    s.push_str("<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/><w:qFormat/></w:style>");
    s.push_str(&style_def(
        "SceneHeading",
        "Scene Heading",
        &format!("<w:keepNext/>{before}"),
        "<w:caps/>",
    ));
    s.push_str(&style_def("Action", "Action", before, ""));
    s.push_str(&style_def(
        "Character",
        "Character",
        &format!("<w:keepNext/>{before}<w:ind w:left=\"3168\"/>"),
        "<w:caps/>",
    ));
    s.push_str(&style_def(
        "Parenthetical",
        "Parenthetical",
        "<w:keepNext/><w:ind w:left=\"2304\" w:right=\"2448\"/>",
        "",
    ));
    s.push_str(&style_def(
        "Dialogue",
        "Dialogue",
        "<w:ind w:left=\"1440\" w:right=\"2160\"/>",
        "",
    ));
    s.push_str(&style_def(
        "Transition",
        "Transition",
        &format!("{before}<w:jc w:val=\"right\"/>"),
        "<w:caps/>",
    ));
    s.push_str(&style_def(
        "Shot",
        "Shot",
        &format!("<w:keepNext/>{before}"),
        "<w:caps/>",
    ));
    s.push_str(&style_def(
        "Centered",
        "Centered",
        &format!("{before}<w:jc w:val=\"center\"/>"),
        "",
    ));
    s.push_str(&style_def(
        "Lyrics",
        "Lyrics",
        "<w:ind w:left=\"1440\" w:right=\"2160\"/>",
        "<w:i/>",
    ));
    s.push_str(&style_def(
        "ScreenplayNote",
        "Screenplay Note",
        before,
        "<w:i/><w:color w:val=\"666666\"/>",
    ));
    s.push_str(&style_def(
        "DualCharacter",
        "Character",
        &format!("<w:keepNext/>{before}<w:ind w:left=\"1152\"/>"),
        "<w:caps/>",
    ));
    s.push_str(&style_def(
        "DualParenthetical",
        "Parenthetical",
        "<w:keepNext/><w:ind w:left=\"576\" w:right=\"576\"/>",
        "",
    ));
    s.push_str(&style_def(
        "DualDialogue",
        "Dialogue",
        "<w:ind w:left=\"0\" w:right=\"288\"/>",
        "",
    ));
    s.push_str(&style_def(
        "TitlePageTitle",
        "Title Page Title",
        "<w:jc w:val=\"center\"/>",
        "<w:b/><w:caps/>",
    ));
    s.push_str(&style_def(
        "TitlePageText",
        "Title Page Text",
        "<w:jc w:val=\"center\"/>",
        "",
    ));
    s.push_str(&style_def("TitlePageLeft", "Title Page Contact", "", ""));
    s.push_str(&style_def(
        "TitlePageRight",
        "Title Page Draft",
        "<w:jc w:val=\"right\"/>",
        "",
    ));
    s.push_str("</w:styles>");
    s
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/header1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"/><Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/></Types>"#;

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/></Relationships>"#;

const DOC_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdHeader1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/header" Target="header1.xml"/></Relationships>"#;

fn header_xml() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:hdr xmlns:w=\"{W_NS}\"><w:p><w:pPr><w:jc w:val=\"right\"/></w:pPr><w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>2</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r><w:r><w:t>.</w:t></w:r></w:p></w:hdr>"
    )
}

fn core_xml(title: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title><dc:creator>OpenFrame Studio</dc:creator></cp:coreProperties>",
        xml::escape(title)
    )
}

/// The package parts of a screenplay DOCX, in write order.
pub fn package_parts(doc: &ScreenplayDoc, opts: &DocxOptions) -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "[Content_Types].xml".to_string(),
            CONTENT_TYPES.as_bytes().to_vec(),
        ),
        ("_rels/.rels".to_string(), ROOT_RELS.as_bytes().to_vec()),
        (
            "word/_rels/document.xml.rels".to_string(),
            DOC_RELS.as_bytes().to_vec(),
        ),
        (
            "word/document.xml".to_string(),
            document_xml(doc, opts).into_bytes(),
        ),
        ("word/styles.xml".to_string(), styles_xml().into_bytes()),
        ("word/header1.xml".to_string(), header_xml().into_bytes()),
        (
            "docProps/core.xml".to_string(),
            core_xml(doc.title_page.title().unwrap_or("Screenplay")).into_bytes(),
        ),
    ]
}

/// Build the DOCX package in memory.
///
/// `openframe_security::archive::write_zip` would be the natural writer, but
/// its entry-name validation rejects OOXML's mandatory `_rels/.rels` part.
/// Entry names here are fixed constants (never user input), so the zip-slip
/// concern that validation guards against does not arise.
pub fn to_bytes(doc: &ScreenplayDoc, opts: &DocxOptions) -> AppResult<Vec<u8>> {
    use std::io::Write;
    let parts = package_parts(doc, opts);
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let err = |e: zip::result::ZipError| {
        AppError::export("docx_failed", "The Word document could not be created.")
            .with_detail(e.to_string())
    };
    for (name, bytes) in &parts {
        zip.start_file(name.as_str(), o).map_err(err)?;
        zip.write_all(bytes)?;
    }
    Ok(zip.finish().map_err(err)?.into_inner())
}

/// Write the DOCX atomically to `dest`.
pub fn write(doc: &ScreenplayDoc, opts: &DocxOptions, dest: &Path) -> AppResult<()> {
    let bytes = to_bytes(doc, opts)?;
    crate::write_atomic(dest, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scene;

    #[test]
    fn style_names_map_to_elements() {
        assert_eq!(hint_for_style("Scene Heading"), Some(LineHint::Heading));
        assert_eq!(
            hint_for_style("DIALOGUE"),
            Some(LineHint::Kind(ElementKind::Dialogue))
        );
        assert_eq!(hint_for_style("Heading 1"), None);
    }

    #[test]
    fn written_docx_parses_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut doc = ScreenplayDoc::default();
        doc.title_page.set("Title", "Black Rain");
        doc.title_page.set("Author", "Nisha Verma");
        let mut s = Scene::new("INT. HOUSE - DAY");
        s.elements
            .push(Element::new(ElementKind::Action, "Rain & wind."));
        s.elements
            .push(Element::new(ElementKind::Character, "MEERA"));
        s.elements
            .push(Element::new(ElementKind::Parenthetical, "(softly)"));
        s.elements
            .push(Element::new(ElementKind::Dialogue, "Stay."));
        doc.scenes.push(s);
        let p = dir.path().join("x.docx");
        write(
            &doc,
            &DocxOptions {
                title_page: true,
                scene_numbers: true,
                ..Default::default()
            },
            &p,
        )
        .unwrap();
        let back = parse(&p).unwrap();
        assert_eq!(back.doc.scenes.len(), 1);
        assert_eq!(back.doc.scenes[0].heading, "INT. HOUSE - DAY");
        assert_eq!(back.doc.scenes[0].elements, doc.scenes[0].elements);
        assert_eq!(back.doc.title_page.title(), Some("BLACK RAIN"));
        assert_eq!(back.doc.title_page.author(), Some("Nisha Verma"));
    }
}
