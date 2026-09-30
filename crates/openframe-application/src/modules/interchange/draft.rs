//! Conversion between the screenplay hub tables and the interchange model.
//!
//! Reads never mutate; writes happen only inside the caller's `store.mutate`
//! transaction (screenplay import apply).
//!
//! Conventions shared with the Screenplay module (hub-table contract):
//! - the scene heading lives on `screenplay_scene.heading`; body elements are
//!   never `scene_heading` (the Screenplay module's rule). A leading
//!   `scene_heading` element (written by Build Screenplay) is tolerated on
//!   export and skipped;
//! - display scene numbers are derived from `position` order (every live
//!   scene, as in the Screenplay workspace), never stored;
//! - `dual = 1` marks dual-dialogue speech. On export a flag on either speaker
//!   is widened to the whole two-speaker block;
//! - `screenplay.title_page_json` keys: `title`, `writtenBy`, `credit`,
//!   `contact`, `draftLine`, `notes`, `source`, `copyright` (aliases such as
//!   `author`/`authors` are read too).

use openframe_domain::enums::ElementType;
use openframe_domain::{AppError, AppResult};
use openframe_import_export::{Element, ElementKind, Scene, ScreenplayDoc, TitlePage, Warning};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};

use crate::modules::screenplay::edit::{MAX_ELEMENT_BYTES, MAX_HEADING_CHARS};
use crate::modules::screenplay::{NewElement, NewScene};
use crate::store::Tx;

#[derive(Debug, Clone)]
pub(crate) struct DraftInfo {
    pub draft_name: String,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    pub screenplay_title: String,
    pub title_page_json: String,
}

pub(crate) fn load_draft_info(c: &Connection, draft_id: &str) -> AppResult<DraftInfo> {
    c.query_row(
        "SELECT d.name, d.revision_label, d.revision_color, s.title, s.title_page_json
         FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id
         WHERE d.id = ?1 AND d.deleted_at IS NULL AND s.deleted_at IS NULL",
        [draft_id],
        |r| {
            Ok(DraftInfo {
                draft_name: r.get(0)?,
                revision_label: r.get(1)?,
                revision_color: r.get(2)?,
                screenplay_title: r.get(3)?,
                title_page_json: r.get(4)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("draft"))
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedScene {
    pub id: String,
    /// Display number derived from order (1-based).
    pub number: usize,
    pub heading: String,
    pub synopsis: Option<String>,
    pub omitted: bool,
    pub elements: Vec<Element>,
}

pub(crate) fn load_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<LoadedScene>> {
    let mut stmt = c.prepare(
        "SELECT id, heading, synopsis, omitted FROM screenplay_scene
         WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows: Vec<(String, String, Option<String>, bool)> = stmt
        .query_map([draft_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? != 0))
        })?
        .collect::<Result<_, _>>()?;
    let mut el = c.prepare(
        "SELECT element_type, text, dual, revision_mark FROM screenplay_element WHERE scene_id = ?1 ORDER BY position, id",
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, (id, heading, synopsis, omitted)) in rows.into_iter().enumerate() {
        let raw: Vec<(String, String, bool, Option<String>)> = el
            .query_map([&id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0, r.get(3)?))
            })?
            .collect::<Result<_, _>>()?;
        let mut heading = heading;
        let mut elements = Vec::with_capacity(raw.len());
        for (k, (ty, text, dual, revision)) in raw.into_iter().enumerate() {
            if ty == "scene_heading" {
                // Heading stored as an element too: use it when the scene row has none.
                if k == 0 && heading.trim().is_empty() {
                    heading = text.clone();
                }
                if text.trim() == heading.trim() || k == 0 {
                    continue;
                }
                // A heading element in the middle of a scene is printed as action-level text.
                elements.push(Element {
                    kind: ElementKind::Action,
                    text: text.to_uppercase(),
                    dual: false,
                    revision,
                });
                continue;
            }
            let Some(kind) = ElementKind::from_stored(&ty) else {
                continue;
            };
            elements.push(Element {
                kind,
                text,
                dual,
                revision: revision.filter(|r| !r.trim().is_empty()),
            });
        }
        normalize_dual(&mut elements);
        out.push(LoadedScene {
            id,
            number: i + 1,
            heading,
            synopsis,
            omitted,
            elements,
        });
    }
    Ok(out)
}

/// Widen a dual flag on either speaker to the whole two-speaker block.
pub(crate) fn normalize_dual(els: &mut [Element]) {
    let n = els.len();
    let mut i = 0;
    while i < n {
        if els[i].kind == ElementKind::Character && els[i].dual {
            // This speech.
            let mut j = i + 1;
            while j < n
                && matches!(
                    els[j].kind,
                    ElementKind::Dialogue | ElementKind::Parenthetical | ElementKind::Lyric
                )
            {
                els[j].dual = true;
                j += 1;
            }
            // Previous adjacent speech, if this is the second speaker and it is not yet marked.
            if i > 0 && els[i - 1].kind.is_speech() && !els[i - 1].dual {
                let mut k = i;
                while k > 0 && els[k - 1].kind.is_speech() {
                    k -= 1;
                    els[k].dual = true;
                    if els[k].kind == ElementKind::Character {
                        break;
                    }
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
}

fn json_str(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| {
        map.get(*k)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

/// Title page for export: stored title page data with sensible fallbacks.
pub(crate) fn title_page_for_export(info: &DraftInfo) -> TitlePage {
    let map: Map<String, Value> = serde_json::from_str::<Value>(&info.title_page_json)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let mut tp = TitlePage::default();
    tp.set(
        "Title",
        json_str(&map, &["title"]).unwrap_or_else(|| info.screenplay_title.clone()),
    );
    if let Some(c) = json_str(&map, &["credit"]) {
        tp.set("Credit", c);
    }
    if let Some(a) = json_str(&map, &["writtenBy", "author", "authors", "writer"]) {
        tp.set("Author", a);
    }
    if let Some(s) = json_str(&map, &["source", "basedOn"]) {
        tp.set("Source", s);
    }
    tp.set(
        "Draft",
        json_str(&map, &["draftLine", "draft", "draftDate"])
            .unwrap_or_else(|| info.draft_name.clone()),
    );
    let revision: Vec<String> = [
        info.revision_label.clone().filter(|s| !s.trim().is_empty()),
        info.revision_color
            .clone()
            .filter(|s| !s.trim().is_empty())
            .map(|c| format!("{c} Revision")),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !revision.is_empty() {
        tp.set("Revision", revision.join(" · "));
    }
    if let Some(c) = json_str(&map, &["contact"]) {
        tp.set("Contact", c);
    }
    if let Some(n) = json_str(&map, &["notes"]) {
        tp.set("Notes", n);
    }
    if let Some(c) = json_str(&map, &["copyright"]) {
        tp.set("Copyright", c);
    }
    tp
}

/// Title page JSON stored on a screenplay created by import.
pub(crate) fn title_page_to_json(tp: &TitlePage) -> String {
    let mut map = Map::new();
    let mut put = |key: &str, v: Option<&str>| {
        if let Some(v) = v.map(str::trim).filter(|v| !v.is_empty()) {
            map.insert(key.to_string(), Value::String(v.to_string()));
        }
    };
    put("title", tp.title());
    put("writtenBy", tp.author());
    put("credit", tp.get("Credit"));
    put("contact", tp.get("Contact"));
    put(
        "draftLine",
        tp.get("Draft").or_else(|| tp.get("Draft date")),
    );
    put("notes", tp.get("Notes"));
    put("source", tp.get("Source"));
    put("copyright", tp.get("Copyright"));
    Value::Object(map).to_string()
}

/// Build the export document from loaded scenes.
/// `scene_ids` limits the scope; display numbers always come from the full draft order.
pub(crate) fn build_doc(
    info: &DraftInfo,
    scenes: &[LoadedScene],
    scene_ids: Option<&[String]>,
    scene_numbers: bool,
) -> ScreenplayDoc {
    let mut doc = ScreenplayDoc {
        title_page: title_page_for_export(info),
        ..Default::default()
    };
    for s in scenes {
        if let Some(ids) = scene_ids
            && !ids.iter().any(|id| id == &s.id)
        {
            continue;
        }
        let mut scene = if s.omitted {
            Scene {
                heading: "OMITTED".into(),
                number: None,
                elements: Vec::new(),
            }
        } else {
            let mut elements = Vec::with_capacity(s.elements.len() + 1);
            // Scene synopsis travels as planning material: written to Fountain
            // (`= …`) only when notes are included, never printed.
            if let Some(syn) = s
                .synopsis
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            {
                elements.push(Element::new(ElementKind::Synopsis, syn.replace('\n', " ")));
            }
            elements.extend(s.elements.iter().cloned());
            Scene {
                heading: s.heading.clone(),
                number: None,
                elements,
            }
        };
        if scene_numbers && !scene.heading.trim().is_empty() {
            scene.number = Some(s.number.to_string());
        }
        doc.scenes.push(scene);
    }
    doc
}

// ------------------------------------------------------------------ import

/// Mapping of interchange-only constructs onto stored element types.
pub(crate) fn stored_type(kind: ElementKind, prev_is_speech: bool) -> Option<ElementType> {
    Some(match kind {
        ElementKind::Action | ElementKind::Centered => ElementType::Action,
        ElementKind::Character => ElementType::Character,
        ElementKind::Dialogue => ElementType::Dialogue,
        ElementKind::Parenthetical => ElementType::Parenthetical,
        ElementKind::Transition => ElementType::Transition,
        ElementKind::Shot => ElementType::Shot,
        ElementKind::Note | ElementKind::Section => ElementType::Note,
        ElementKind::Lyric => {
            if prev_is_speech {
                ElementType::Dialogue
            } else {
                ElementType::Action
            }
        }
        ElementKind::Synopsis | ElementKind::PageBreak => return None,
    })
}

fn is_printed(kind: ElementKind) -> bool {
    !matches!(
        kind,
        ElementKind::Note | ElementKind::Section | ElementKind::Synopsis | ElementKind::PageBreak
    )
}

/// Prepare a parsed document for the project and describe what changes on
/// the way in (returned as warnings, shown in the preview and the report).
///
/// Material before the first heading ("FADE IN:", an opening title card)
/// joins the start of the first scene: the Screenplay workspace numbers every
/// scene by position, so a heading-less opening scene would shift every scene
/// number away from the source's numbering.
pub(crate) fn normalize_for_project(doc: &mut ScreenplayDoc) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let has_opening = doc.scenes.len() > 1 && doc.scenes[0].heading.trim().is_empty();
    if has_opening {
        let opening = doc.scenes.remove(0);
        let printed: Vec<&str> = opening
            .elements
            .iter()
            .filter(|e| is_printed(e.kind))
            .map(|e| e.text.trim())
            .collect();
        if !printed.is_empty() {
            let joined = printed.join(" ");
            let mut sample: String = joined.chars().take(60).collect();
            if joined.chars().count() > 60 {
                sample.push('…');
            }
            warnings.push(Warning::info(
                "opening_material_moved",
                format!("Text before the first scene heading (“{sample}”) was placed at the start of scene 1."),
            ));
        }
        let first = &mut doc.scenes[0];
        let mut els = opening.elements;
        els.append(&mut first.elements);
        first.elements = els;
        for w in &mut doc.warnings {
            if let Some(i) = w.scene.as_mut() {
                *i = i.saturating_sub(1);
            }
        }
    }
    let mut centered = 0;
    let mut dropped = 0;
    for s in &doc.scenes {
        for e in &s.elements {
            match e.kind {
                ElementKind::Centered => centered += 1,
                ElementKind::PageBreak => dropped += 1,
                _ => {}
            }
        }
    }
    if centered > 0 {
        warnings.push(Warning::info(
            "centered_as_action",
            format!("{centered} centered line(s) were imported as action."),
        ));
    }
    if dropped > 0 {
        warnings.push(Warning::info(
            "page_breaks_dropped",
            "Forced page breaks were not imported; OpenFrame paginates screenplays automatically.",
        ));
    }
    for (i, s) in doc.scenes.iter().enumerate() {
        if s.heading.trim().chars().count() > MAX_HEADING_CHARS {
            warnings.push(
                Warning::attention(
                    "heading_shortened",
                    format!(
                        "A scene heading longer than {MAX_HEADING_CHARS} characters was shortened; the full text was kept as a note in the scene."
                    ),
                )
                .at(i),
            );
        }
    }
    warnings
}

/// Stored content for an imported draft (new identities everywhere —
/// imported content is a new copy, Import/Export §14.2).
pub(crate) struct ImportContent {
    pub scenes: Vec<NewScene>,
    /// (scene index, element index, revision label) to restore after insert.
    pub revisions: Vec<(usize, usize, String)>,
    pub element_count: usize,
}

/// Remove characters that can't be stored (NUL and other controls except line
/// breaks); tabs become spaces.
fn storable(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\t' => Some(' '),
            '\n' => Some('\n'),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

/// Split text into pieces of at most `max` bytes, preferring line breaks.
fn split_long(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in text.split_inclusive('\n') {
        if cur.len() + line.len() > max && !cur.is_empty() {
            out.push(cur.trim_end().to_string());
            cur.clear();
        }
        if line.len() > max {
            for ch in line.chars() {
                if cur.len() + ch.len_utf8() > max {
                    out.push(std::mem::take(&mut cur));
                }
                cur.push(ch);
            }
        } else {
            cur.push_str(line);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim_end().to_string());
    }
    out
}

pub(crate) fn to_new_scenes(doc: &ScreenplayDoc) -> ImportContent {
    let mut scenes = Vec::with_capacity(doc.scenes.len());
    let mut revisions = Vec::new();
    let mut element_count = 0;
    for (si, s) in doc.scenes.iter().enumerate() {
        let full_heading = storable(&s.heading).replace('\n', " ").trim().to_string();
        let heading: String = full_heading.chars().take(MAX_HEADING_CHARS).collect();
        let synopsis: Vec<String> = s
            .elements
            .iter()
            .filter(|e| e.kind == ElementKind::Synopsis)
            .map(|e| storable(e.text.trim()))
            .collect();
        let mut elements = Vec::with_capacity(s.elements.len());
        if heading.len() < full_heading.len() {
            elements.push(NewElement {
                element_type: ElementType::Note,
                text: format!("Full scene heading: {full_heading}"),
                dual: false,
            });
        }
        let mut prev_speech = false;
        for e in &s.elements {
            let Some(ty) = stored_type(e.kind, prev_speech) else {
                continue;
            };
            let mut text = storable(&e.text);
            if e.kind == ElementKind::Section {
                text = format!("Section: {}", text.trim());
            }
            if matches!(
                ty,
                ElementType::Character
                    | ElementType::Parenthetical
                    | ElementType::Transition
                    | ElementType::Shot
            ) {
                text = text
                    .split('\n')
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            for piece in split_long(&text, MAX_ELEMENT_BYTES) {
                if let Some(rev) = e.revision.as_ref().filter(|r| !r.trim().is_empty()) {
                    revisions.push((si, elements.len(), rev.clone()));
                }
                elements.push(NewElement {
                    element_type: ty,
                    text: piece,
                    dual: e.dual,
                });
                element_count += 1;
            }
            prev_speech = matches!(
                ty,
                ElementType::Character | ElementType::Dialogue | ElementType::Parenthetical
            );
        }
        scenes.push(NewScene {
            heading,
            synopsis: if synopsis.is_empty() {
                None
            } else {
                Some(synopsis.join("\n"))
            },
            elements,
            ..Default::default()
        });
    }
    ImportContent {
        scenes,
        revisions,
        element_count,
    }
}

/// Revision marks carried by the source (FDX), restored on the inserted rows.
pub(crate) fn apply_revisions(
    tx: &Tx<'_>,
    draft_id: &str,
    revisions: &[(usize, usize, String)],
) -> AppResult<()> {
    let c = tx.conn();
    let mut stmt = c.prepare(
        "UPDATE screenplay_element SET revision_mark = ?1
         WHERE position = ?4
           AND scene_id = (SELECT id FROM screenplay_scene WHERE draft_id = ?2 AND position = ?3 AND deleted_at IS NULL)",
    )?;
    for (si, ei, rev) in revisions {
        stmt.execute(params![rev, draft_id, (*si + 1) as i64, (*ei + 1) as i64])?;
    }
    Ok(())
}

/// Merge an imported title page into the stored title page JSON (keys used by
/// the Screenplay workspace: title, writtenBy, contact, draftLine, notes; plus
/// credit/source/copyright kept for export). Only non-empty values overwrite.
pub(crate) fn merge_title_page(stored: &str, imported: &TitlePage) -> String {
    let mut map: Map<String, Value> = serde_json::from_str::<Value>(stored)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    if let Ok(Value::Object(add)) = serde_json::from_str::<Value>(&title_page_to_json(imported)) {
        for (k, v) in add {
            map.insert(k, v);
        }
    }
    Value::Object(map).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_flag_on_second_speaker_widens_to_block() {
        let mut els = vec![
            Element::new(ElementKind::Action, "x"),
            Element::new(ElementKind::Character, "A"),
            Element::new(ElementKind::Dialogue, "hi"),
            Element::new(ElementKind::Character, "B").dual(true),
            Element::new(ElementKind::Dialogue, "yo"),
            Element::new(ElementKind::Action, "y"),
        ];
        normalize_dual(&mut els);
        let flags: Vec<bool> = els.iter().map(|e| e.dual).collect();
        assert_eq!(flags, vec![false, true, true, true, true, false]);
    }
}
