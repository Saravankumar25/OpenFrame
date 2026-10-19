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
//! - scene numbers are the production labels of the Screenplay module
//!   (`numbering`): derived from order until a draft is imported with numbers,
//!   locked or numbered by hand (ADR-0015). Omitted scenes print as OMITTED;
//! - `dual = 1` marks dual-dialogue speech. On export a flag on either speaker
//!   is widened to the whole two-speaker block;
//! - `page_break` elements are explicit page breaks; `custom` elements carry
//!   their custom element's name, look and import and export identity;
//! - `screenplay.title_page_json` keys: `title`, `writtenBy`, `credit`,
//!   `source`, `contactName`, `address`, `phone`, `email`, `contact`,
//!   `draftLine`, `revisionLine`, `date`, `representation`, `notes`,
//!   `copyright` (aliases such as `author`/`authors` are read too);
//! - `screenplay.page_setup_json`: paper, font, spacing, page numbering,
//!   header and footer and export defaults (`screenplay::setup`).

use std::collections::{HashMap, HashSet};

use openframe_domain::enums::ElementType;
use openframe_domain::{AppError, AppResult};
use openframe_import_export::{
    CustomStyle, Element, ElementKind, ExportOptions, LayoutOptions, Scene, ScreenplayDoc,
    TitlePage, Warning,
};
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Map, Value};

use crate::modules::screenplay::edit::{MAX_ELEMENT_BYTES, MAX_HEADING_CHARS};
use crate::modules::screenplay::setup::{self, MAX_CUSTOM_ELEMENTS, ScreenplayPageSetup};
use crate::modules::screenplay::{NewElement, NewScene, numbering};

#[derive(Debug, Clone)]
pub(crate) struct DraftInfo {
    pub screenplay_id: String,
    pub draft_name: String,
    pub revision_label: Option<String>,
    pub revision_color: Option<String>,
    /// Date of the revision record (Start Revision), if this draft is a revision.
    pub revision_date: Option<String>,
    pub screenplay_title: String,
    pub title_page_json: String,
    pub page_setup: ScreenplayPageSetup,
}

pub(crate) fn load_draft_info(c: &Connection, draft_id: &str) -> AppResult<DraftInfo> {
    let mut info = c
        .query_row(
            "SELECT d.name, d.revision_label, d.revision_color, s.title, s.title_page_json, s.id,
                    (SELECT revision_date FROM screenplay_revision r WHERE r.draft_id = d.id)
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
                    screenplay_id: r.get(5)?,
                    revision_date: r.get(6)?,
                    page_setup: ScreenplayPageSetup::default(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("draft"))?;
    info.page_setup = setup::load_page_setup(c, &info.screenplay_id)?;
    Ok(info)
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedScene {
    pub id: String,
    pub lineage_id: String,
    /// Display number: the production label (derived from order until frozen).
    pub label: String,
    /// The label is stored (imported, frozen at lock, inserted or set by hand).
    pub stored_label: bool,
    pub heading: String,
    pub synopsis: Option<String>,
    pub omitted: bool,
    pub elements: Vec<Element>,
}

/// Custom element styles of a screenplay by id (deleted ones included, so old text still prints).
pub(crate) fn custom_styles(
    c: &Connection,
    screenplay_id: &str,
) -> AppResult<HashMap<String, CustomStyle>> {
    let mut stmt = c.prepare(
        "SELECT id, name, export_name, look, uppercase, italic, space_before
         FROM screenplay_custom_element WHERE screenplay_id = ?1",
    )?;
    let rows = stmt
        .query_map([screenplay_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                CustomStyle {
                    name: r.get(1)?,
                    export_name: r.get(2)?,
                    look: r.get(3)?,
                    uppercase: r.get(4)?,
                    italic: r.get(5)?,
                    space_before: r.get::<_, i64>(6)?.clamp(0, 3) as u8,
                },
            ))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(rows)
}

pub(crate) fn load_scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<LoadedScene>> {
    let screenplay_id: String = c.query_row(
        "SELECT screenplay_id FROM screenplay_draft WHERE id = ?1",
        [draft_id],
        |r| r.get(0),
    )?;
    let styles = custom_styles(c, &screenplay_id)?;
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, synopsis, omitted, production_number FROM screenplay_scene
         WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    type Row = (String, String, String, Option<String>, bool, Option<String>);
    let rows: Vec<Row> = stmt
        .query_map([draft_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get::<_, i64>(4)? != 0,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let labels = numbering::production_labels(
        &rows
            .iter()
            .map(|r| numbering::NumberRow {
                stored: r.5.clone(),
            })
            .collect::<Vec<_>>(),
    );
    let mut el = c.prepare(
        "SELECT element_type, text, dual, revision_mark, custom_element_id FROM screenplay_element
         WHERE scene_id = ?1 ORDER BY position, id",
    )?;
    let mut out = Vec::with_capacity(rows.len());
    for ((id, lineage_id, heading, synopsis, omitted, stored), label) in
        rows.into_iter().zip(labels)
    {
        type ElRow = (String, String, bool, Option<String>, Option<String>);
        let raw: Vec<ElRow> = el
            .query_map([&id], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get::<_, i64>(2)? != 0,
                    r.get(3)?,
                    r.get(4)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        let mut heading = heading;
        let mut elements = Vec::with_capacity(raw.len());
        for (k, (ty, text, dual, revision, custom_id)) in raw.into_iter().enumerate() {
            let revision = revision.filter(|r| !r.trim().is_empty());
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
                    revision,
                    ..Element::new(ElementKind::Action, text.to_uppercase())
                });
                continue;
            }
            let element = match ElementKind::from_stored(&ty) {
                Some(ElementKind::Custom) => {
                    let style = custom_id
                        .as_ref()
                        .and_then(|i| styles.get(i))
                        .cloned()
                        .unwrap_or_else(|| CustomStyle::imported("Custom", "action"));
                    Element::custom(text, style)
                }
                Some(ElementKind::PageBreak) => Element::page_break(),
                Some(kind) => Element::new(kind, text).dual(dual),
                None => continue,
            };
            elements.push(Element {
                revision,
                ..element
            });
        }
        normalize_dual(&mut elements);
        out.push(LoadedScene {
            id,
            lineage_id,
            label,
            stored_label: stored.is_some(),
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
    if let Some(d) = json_str(&map, &["date"]).or_else(|| info.revision_date.clone()) {
        tp.set("Date", d);
    }
    let computed: Vec<String> = [
        info.revision_label.clone().filter(|s| !s.trim().is_empty()),
        info.revision_color
            .clone()
            .filter(|s| !s.trim().is_empty())
            .map(|c| format!("{c} Revision")),
    ]
    .into_iter()
    .flatten()
    .collect();
    match json_str(&map, &["revisionLine"]) {
        Some(r) => tp.set("Revision", r),
        None if !computed.is_empty() => tp.set("Revision", computed.join(" · ")),
        None => {}
    }
    // Contact block: name, address, phone, email, then any free text.
    let contact: Vec<String> = ["contactName", "address", "phone", "email", "contact"]
        .iter()
        .filter_map(|k| json_str(&map, &[k]))
        .collect();
    if !contact.is_empty() {
        tp.set("Contact", contact.join("\n"));
    }
    if let Some(r) = json_str(&map, &["representation"]) {
        tp.set("Representation", r);
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
    put("date", tp.get("Date"));
    put("revisionLine", tp.get("Revision"));
    put("representation", tp.get("Representation"));
    put("notes", tp.get("Notes"));
    put("source", tp.get("Source"));
    put("copyright", tp.get("Copyright"));
    Value::Object(map).to_string()
}

/// Export options for a draft: the dialog's choices on top of the screenplay's page setup.
pub(crate) fn export_options(
    info: &DraftInfo,
    title_page: Option<bool>,
    scene_numbers: Option<bool>,
    include_notes: bool,
    revision_marks: Option<bool>,
    revision_info: bool,
) -> ExportOptions {
    let s = &info.page_setup;
    ExportOptions {
        title_page: title_page.unwrap_or(s.include_title_page),
        scene_numbers: scene_numbers.unwrap_or(s.scene_numbers),
        include_notes,
        revision_marks: revision_marks.unwrap_or(s.revision_marks),
        revision_info,
        page: s.engine(),
    }
}

/// The screen layout options of a draft (page view, page facts, lock baseline).
pub(crate) fn screen_layout(info: &DraftInfo) -> LayoutOptions {
    export_options(info, None, None, false, None, true).layout()
}

/// Build the export document from loaded scenes.
/// `scene_ids` limits the scope; numbers always come from the full draft.
/// Stored production labels always travel (FDX keeps them); derived numbers
/// only when scene numbers are asked for.
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
        if (scene_numbers || s.stored_label) && !scene.heading.trim().is_empty() {
            scene.number = Some(s.label.clone());
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
        ElementKind::PageBreak => ElementType::PageBreak,
        ElementKind::Custom => ElementType::Custom,
        ElementKind::Lyric => {
            if prev_is_speech {
                ElementType::Dialogue
            } else {
                ElementType::Action
            }
        }
        ElementKind::Synopsis => return None,
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
    let centered = doc
        .scenes
        .iter()
        .flat_map(|s| &s.elements)
        .filter(|e| e.kind == ElementKind::Centered)
        .count();
    if centered > 0 {
        warnings.push(Warning::info(
            "centered_as_action",
            format!("{centered} centered line(s) were imported as action."),
        ));
    }
    // Custom element identities beyond the per-screenplay cap are imported as action and
    // listed for review, so a file with thousands of styles cannot flood the project.
    let mut identities: Vec<String> = Vec::new();
    let mut over: Vec<String> = Vec::new();
    for e in doc.scenes.iter_mut().flat_map(|s| s.elements.iter_mut()) {
        let Some(style) = e.custom.as_ref().filter(|_| e.kind == ElementKind::Custom) else {
            continue;
        };
        let key = style.export_name.to_lowercase();
        if identities.contains(&key) {
            continue;
        }
        if identities.len() < MAX_CUSTOM_ELEMENTS {
            identities.push(key);
            continue;
        }
        if over.len() < 10 && !over.contains(&style.name) {
            over.push(style.name.clone());
        }
        e.kind = ElementKind::Action;
        e.custom = None;
    }
    if !over.is_empty() {
        warnings.push(Warning::attention(
            "custom_elements_over_limit",
            format!(
                "This file has more than {MAX_CUSTOM_ELEMENTS} kinds of custom element. The others (such as {}) were imported as action; review them after the import.",
                over.join(", ")
            ),
        ));
    }
    // Scene numbers become production labels when they are valid and unique.
    let mut seen: HashSet<String> = HashSet::new();
    let mut dropped = 0;
    for s in &mut doc.scenes {
        if let Some(n) = s.number.take() {
            match numbering::validate_label(&n) {
                Ok(l) if seen.insert(l.clone()) => s.number = Some(l),
                _ => dropped += 1,
            }
        }
    }
    if dropped > 0 {
        warnings.push(Warning::attention(
            "scene_numbers_not_kept",
            format!(
                "{dropped} scene number(s) were not kept because they repeat or use characters other than letters and digits. Those scenes get numbers from their position."
            ),
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
    pub element_count: usize,
    /// Paragraphs imported as action because the screenplay had no room for
    /// another custom element, and the custom types they had (first 20).
    pub no_room_paragraphs: usize,
    pub no_room_types: Vec<String>,
    pub no_room_type_count: usize,
}

/// Custom element types named one by one in the "no room" import warning.
const MAX_LISTED_NO_ROOM_TYPES: usize = 20;

impl ImportContent {
    /// The attention warning for paragraphs imported as action because the
    /// screenplay already held the most custom elements it can have.
    pub fn no_room_warning(&self) -> Option<Warning> {
        if self.no_room_paragraphs == 0 {
            return None;
        }
        let more = self
            .no_room_type_count
            .saturating_sub(self.no_room_types.len());
        let names = self
            .no_room_types
            .iter()
            .map(|t| format!("“{t}”"))
            .collect::<Vec<_>>()
            .join(", ");
        let others = if more > 0 {
            format!(" and {more} more")
        } else {
            String::new()
        };
        Some(Warning::attention(
            "custom_elements_no_room",
            format!(
                "This screenplay already has {MAX_CUSTOM_ELEMENTS} custom elements, so {} paragraph(s) of the custom element type(s) {names}{others} were imported as action. Their text is kept; review them after the import.",
                self.no_room_paragraphs
            ),
        ))
    }
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

/// Stored scenes for an imported document. `custom_id` resolves a custom
/// element's style to a custom element of the target screenplay (created when
/// missing), so unknown source elements are kept, never flattened.
pub(crate) fn to_new_scenes(
    doc: &ScreenplayDoc,
    custom_id: &mut dyn FnMut(&CustomStyle) -> AppResult<Option<String>>,
) -> AppResult<ImportContent> {
    let mut scenes = Vec::with_capacity(doc.scenes.len());
    let mut element_count = 0;
    let mut no_room_paragraphs = 0;
    let mut no_room_types: Vec<String> = Vec::new();
    let mut no_room_seen: HashSet<String> = HashSet::new();
    for s in &doc.scenes {
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
            elements.push(NewElement::new(
                ElementType::Note,
                format!("Full scene heading: {full_heading}"),
            ));
        }
        let mut prev_speech = false;
        for e in &s.elements {
            let Some(ty) = stored_type(e.kind, prev_speech) else {
                continue;
            };
            if ty == ElementType::PageBreak {
                elements.push(NewElement::new(ElementType::PageBreak, ""));
                prev_speech = false;
                continue;
            }
            let fallback;
            let style = match (ty, &e.custom) {
                (ElementType::Custom, Some(style)) => Some(style),
                (ElementType::Custom, None) => {
                    fallback = CustomStyle::imported("Imported element", "action");
                    Some(&fallback)
                }
                _ => None,
            };
            let (ty, custom_element_id) = match style {
                Some(style) => match custom_id(style)? {
                    Some(id) => (ty, Some(id)),
                    // No room for another custom element: keep the text as action.
                    None => {
                        no_room_paragraphs += 1;
                        if no_room_seen.insert(style.export_name.to_lowercase())
                            && no_room_types.len() < MAX_LISTED_NO_ROOM_TYPES
                        {
                            no_room_types.push(style.name.clone());
                        }
                        (ElementType::Action, None)
                    }
                },
                None => (ty, None),
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
                elements.push(NewElement {
                    element_type: ty,
                    text: piece,
                    dual: e.dual,
                    custom_element_id: custom_element_id.clone(),
                    revision_mark: e.revision.clone().filter(|r| !r.trim().is_empty()),
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
            production_number: s.number.clone(),
            production_number_source: s.number.as_ref().map(|_| "imported".to_string()),
            elements,
            ..Default::default()
        });
    }
    Ok(ImportContent {
        scenes,
        element_count,
        no_room_paragraphs,
        no_room_types,
        no_room_type_count: no_room_seen.len(),
    })
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

    #[test]
    fn imported_scene_numbers_are_validated_and_unique() {
        let mut doc = ScreenplayDoc::default();
        for n in ["12", "12A", "12", "1 2", "७"] {
            let mut s = Scene::new("INT. A - DAY");
            s.number = Some(n.into());
            doc.scenes.push(s);
        }
        let w = normalize_for_project(&mut doc);
        let kept: Vec<Option<&str>> = doc.scenes.iter().map(|s| s.number.as_deref()).collect();
        assert_eq!(kept, vec![Some("12"), Some("12A"), None, None, None]);
        assert!(w.iter().any(|w| w.code == "scene_numbers_not_kept"));
    }
}
