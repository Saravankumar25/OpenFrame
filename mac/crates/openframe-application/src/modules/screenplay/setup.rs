//! Page setup, saved presets and custom elements (Screenplay and Production
//! Depth Specification §8, §14, §15, §17; contract doc 32 §2.1, §2.2).
//!
//! - Page setup is saved with each screenplay (`screenplay.page_setup_json`):
//!   paper (US Letter or A4), Courier size, line spacing, first page number,
//!   header and footer with placeholders, the first-page rule, and the export
//!   defaults (title page included, scene numbers, revision marks). `{}`
//!   means the standard preset, which reproduces the layout from before page
//!   setup existed.
//! - Presets: two built-ins (standard screenplay on US Letter and on A4) and
//!   project presets the writer saves (`screenplay_page_preset`).
//! - Custom elements live under Advanced (`screenplay_custom_element`): a name,
//!   an optional Ctrl+Alt+digit shortcut, a look (the built-in element whose
//!   position it takes), case, italic, blank lines before, the element Enter
//!   creates next, and an import and export identity (the Final Draft
//!   paragraph type). A custom element in use can't be deleted.
//!
//! All commands are undoable and human-only (layout settings, doc 32 §2.5).

use std::collections::{HashMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use openframe_import_export::layout::{
    self, FONTS, LineSpacing, MAX_FONT_SIZE, MAX_HEADER_CHARS, MIN_FONT_SIZE, PageSetup, Paper,
};
use openframe_import_export::rules;
use openframe_persistence::rows::{int, opt_text, text, update_fields};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::load_screenplay;
use crate::core::AppCore;
use crate::registry::{Registry, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};
use crate::util::{require_id, required_text};

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("screenplay.page_setup", page_setup).meta(M::read(
        "A screenplay's page setup (paper, font, spacing, header and footer) and the saved presets.",
    ));
    r.command("screenplay.update_page_setup", update_page_setup)
        .meta(
            M::edit("Change a screenplay's page setup, header and footer or title page switch.")
                .hidden(h::PAGE_LAYOUT),
        );
    r.command("screenplay.save_page_preset", save_page_preset)
        .meta(M::edit("Save a page setup as a named project preset.").hidden(h::PAGE_LAYOUT));
    r.command("screenplay.delete_page_preset", delete_page_preset)
        .meta(
            M::soft_delete("Move a saved page setup preset to Recently Deleted.")
                .hidden(h::PAGE_LAYOUT),
        );
    r.query("screenplay.custom_elements", custom_elements).meta(M::read(
        "Custom screenplay elements of a screenplay with their look and how often they are used.",
    ));
    r.command("screenplay.create_custom_element", create_custom_element)
        .meta(M::edit("Define a custom screenplay element.").hidden(h::PAGE_LAYOUT));
    r.command("screenplay.update_custom_element", update_custom_element)
        .meta(
            M::edit("Change a custom screenplay element's name, shortcut or look.")
                .hidden(h::PAGE_LAYOUT),
        );
    r.command("screenplay.delete_custom_element", delete_custom_element)
        .meta(
            M::soft_delete("Move an unused custom screenplay element to Recently Deleted.")
                .hidden(h::PAGE_LAYOUT),
        );
    r.trash_handler(TrashHandler {
        object_type: "screenplay_custom_element",
        table: "screenplay_custom_element",
        label: "Custom screenplay element",
        restore: None,
        purge: purge_custom_element,
    });
    r.trash_handler(TrashHandler {
        object_type: "screenplay_page_preset",
        table: "screenplay_page_preset",
        label: "Page setup preset",
        restore: None,
        purge: purge_page_preset,
    });
}

// ------------------------------------------------------------------ page setup

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ScreenplayPaper {
    #[default]
    Letter,
    A4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ScreenplayLineSpacing {
    #[default]
    Single,
    OneAndHalf,
    Double,
}

/// Page setup saved with a screenplay. Defaults are the standard preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenplayPageSetup {
    pub paper: ScreenplayPaper,
    /// "Courier" (the font the screenplay PDF prints without embedding).
    pub font: String,
    /// Points, 10 to 14 (12 is standard).
    pub font_size: u8,
    pub line_spacing: ScreenplayLineSpacing,
    /// Number printed on the first script page.
    pub first_page_number: u32,
    /// Print the page number on the first script page (off: the first page is unnumbered and
    /// the next one prints "2."). The title page is never numbered.
    pub number_first_page: bool,
    /// Header text (placeholders: {title} {draft} {revision} {date} {page}).
    pub header: String,
    pub footer: String,
    /// Header and footer also on the first script page.
    pub header_on_first_page: bool,
    /// The title page is included when exporting and printing (the export switch).
    pub include_title_page: bool,
    /// Scene numbers in the margins by default.
    pub scene_numbers: bool,
    /// Revision asterisks by default (production revision output).
    pub revision_marks: bool,
}

impl Default for ScreenplayPageSetup {
    fn default() -> Self {
        Self {
            paper: ScreenplayPaper::Letter,
            font: "Courier".into(),
            font_size: 12,
            line_spacing: ScreenplayLineSpacing::Single,
            first_page_number: 1,
            number_first_page: true,
            header: String::new(),
            footer: String::new(),
            header_on_first_page: false,
            include_title_page: true,
            scene_numbers: false,
            revision_marks: false,
        }
    }
}

impl ScreenplayPageSetup {
    /// The layout engine's page setup.
    pub fn engine(&self) -> PageSetup {
        PageSetup {
            paper: match self.paper {
                ScreenplayPaper::Letter => Paper::Letter,
                ScreenplayPaper::A4 => Paper::A4,
            },
            font: self.font.clone(),
            font_size: self.font_size,
            line_spacing: match self.line_spacing {
                ScreenplayLineSpacing::Single => LineSpacing::Single,
                ScreenplayLineSpacing::OneAndHalf => LineSpacing::OneAndHalf,
                ScreenplayLineSpacing::Double => LineSpacing::Double,
            },
            first_page_number: self.first_page_number,
            number_first_page: self.number_first_page,
            header: self.header.clone(),
            footer: self.footer.clone(),
            header_on_first_page: self.header_on_first_page,
        }
    }

    fn validate(mut self) -> AppResult<Self> {
        if !FONTS
            .iter()
            .any(|f| f.eq_ignore_ascii_case(self.font.trim()))
        {
            return Err(AppError::validation(
                "font",
                "Screenplay pages use Courier. Other fonts arrive with embedded font support.",
            ));
        }
        self.font = FONTS[0].to_string();
        if !(MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&self.font_size) {
            return Err(AppError::validation(
                "fontSize",
                format!(
                    "Choose a size from {MIN_FONT_SIZE} to {MAX_FONT_SIZE} points (12 is standard)."
                ),
            ));
        }
        if !(1..=9_999).contains(&self.first_page_number) {
            return Err(AppError::validation(
                "firstPageNumber",
                "The first page number must be between 1 and 9999.",
            ));
        }
        for (v, what) in [(&mut self.header, "header"), (&mut self.footer, "footer")] {
            let t = v.trim().to_string();
            if t.chars().count() > MAX_HEADER_CHARS {
                return Err(AppError::validation(
                    what,
                    format!("The {what} can be at most {MAX_HEADER_CHARS} characters."),
                ));
            }
            if t.chars().any(|c| c.is_control()) {
                return Err(AppError::validation(
                    what,
                    format!("The {what} must be a single line."),
                ));
            }
            *v = t;
        }
        Ok(self)
    }
}

/// The stored page setup of a screenplay (unknown or damaged JSON reads as the standard preset).
pub(crate) fn load_page_setup(
    c: &Connection,
    screenplay_id: &str,
) -> AppResult<ScreenplayPageSetup> {
    let json: Option<String> = c
        .query_row(
            "SELECT page_setup_json FROM screenplay WHERE id=?1",
            [screenplay_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayPagePreset {
    /// A preset id, or "standard-letter" / "standard-a4" for the built-ins.
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub setup: ScreenplayPageSetup,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayPageSetupInfo {
    pub screenplay_id: String,
    pub setup: ScreenplayPageSetup,
    pub presets: Vec<ScreenplayPagePreset>,
    /// Body lines per page with this setup.
    pub lines_per_page: u32,
    pub fonts: Vec<String>,
    pub min_font_size: u8,
    pub max_font_size: u8,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayIdArgs {
    pub screenplay_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayUpdatePageSetupArgs {
    pub screenplay_id: String,
    pub setup: ScreenplayPageSetup,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplaySavePagePresetArgs {
    pub name: String,
    pub setup: ScreenplayPageSetup,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayIdOnlyArgs {
    pub id: String,
}

fn built_in_presets() -> Vec<ScreenplayPagePreset> {
    vec![
        ScreenplayPagePreset {
            id: "standard-letter".into(),
            name: "Standard screenplay (US Letter)".into(),
            built_in: true,
            setup: ScreenplayPageSetup::default(),
        },
        ScreenplayPagePreset {
            id: "standard-a4".into(),
            name: "Standard screenplay (A4)".into(),
            built_in: true,
            setup: ScreenplayPageSetup {
                paper: ScreenplayPaper::A4,
                ..Default::default()
            },
        },
    ]
}

fn presets(c: &Connection) -> AppResult<Vec<ScreenplayPagePreset>> {
    let mut out = built_in_presets();
    let mut stmt = c.prepare(
        "SELECT id, name, settings_json FROM screenplay_page_preset WHERE deleted_at IS NULL ORDER BY lower(name), id",
    )?;
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, name, json) = row?;
        out.push(ScreenplayPagePreset {
            id,
            name,
            built_in: false,
            setup: serde_json::from_str(&json).unwrap_or_default(),
        });
    }
    Ok(out)
}

fn page_setup(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayIdArgs,
) -> AppResult<ScreenplayPageSetupInfo> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.screenplay_id, "screenplay")?;
    core.project()?.store.read(|c| {
        load_screenplay(c, &a.screenplay_id)?;
        let setup = load_page_setup(c, &a.screenplay_id)?;
        let g = layout::Geometry::for_setup(&setup.engine());
        Ok(ScreenplayPageSetupInfo {
            screenplay_id: a.screenplay_id.clone(),
            lines_per_page: g.lines_per_page as u32,
            setup,
            presets: presets(c)?,
            fonts: FONTS.iter().map(|f| f.to_string()).collect(),
            min_font_size: MIN_FONT_SIZE,
            max_font_size: MAX_FONT_SIZE,
        })
    })
}

fn update_page_setup(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayUpdatePageSetupArgs,
) -> AppResult<ScreenplayPageSetup> {
    require_id(&a.screenplay_id, "screenplay")?;
    let setup = a.setup.validate()?;
    let json = serde_json::to_string(&setup).map_err(|e| AppError::internal(e.to_string()))?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.update_page_setup",
            "Changed the page setup",
            Capability::Edit,
        )
        .target("screenplay", &a.screenplay_id),
        |tx| {
            let c = tx.conn();
            load_screenplay(c, &a.screenplay_id)?;
            update_fields(
                c,
                "screenplay",
                &a.screenplay_id,
                &[("page_setup_json", text(json.clone()))],
                &["page_setup_json"],
                None,
                "screenplay",
            )?;
            Ok(())
        },
    )?;
    Ok(setup)
}

fn save_page_preset(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplaySavePagePresetArgs,
) -> AppResult<ScreenplayPagePreset> {
    let name = required_text(&a.name, "Preset name", 80)?;
    let setup = a.setup.validate()?;
    let json = serde_json::to_string(&setup).map_err(|e| AppError::internal(e.to_string()))?;
    let id = new_id();
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.save_page_preset",
            format!("Saved the page setup preset “{name}”"),
            Capability::Edit,
        ),
        |tx| {
            let c = tx.conn();
            let taken = built_in_presets().iter().any(|p| p.name.eq_ignore_ascii_case(&name))
                || c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM screenplay_page_preset WHERE deleted_at IS NULL AND lower(name)=lower(?1))",
                    [&name],
                    |r| r.get::<_, bool>(0),
                )?;
            if taken {
                return Err(AppError::validation(
                    "name",
                    "A preset with this name already exists. Choose another name.",
                ));
            }
            let now = now_ms();
            c.execute(
                "INSERT INTO screenplay_page_preset(id, name, settings_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![id, name, json, now],
            )?;
            Ok(())
        },
    )?;
    Ok(ScreenplayPagePreset {
        id,
        name,
        built_in: false,
        setup,
    })
}

fn delete_page_preset(core: &AppCore, actor: &Actor, a: ScreenplayIdOnlyArgs) -> AppResult<()> {
    require_id(&a.id, "preset")?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.delete_page_preset",
            "Deleted a page setup preset",
            Capability::SoftDelete,
        )
        .target("screenplay_page_preset", &a.id),
        |tx| {
            let name: String = tx
                .conn()
                .query_row(
                    "SELECT name FROM screenplay_page_preset WHERE id=?1 AND deleted_at IS NULL",
                    [&a.id],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| AppError::not_found("preset"))?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "screenplay_page_preset",
                    table: "screenplay_page_preset",
                    id: &a.id,
                    title: Some(name),
                    parent_type: None,
                    parent_id: None,
                    position: None,
                },
            )
        },
    )
}

fn purge_page_preset(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM screenplay_page_preset WHERE id=?1",
        [&row.object_id],
    )?;
    Ok(())
}

// ------------------------------------------------------------ custom elements

/// Built-in elements a custom element may take its look from.
pub const CUSTOM_LOOKS: &[&str] = &[
    "action",
    "centered",
    "character",
    "parenthetical",
    "dialogue",
    "transition",
    "shot",
];
/// Elements Enter may create after a custom element (besides other custom elements).
const NEXT_BUILT_INS: &[&str] = &[
    "scene_heading",
    "action",
    "character",
    "parenthetical",
    "dialogue",
    "transition",
    "shot",
    "note",
];
/// Paragraph types Final Draft already uses for built-in elements.
const RESERVED_EXPORT_NAMES: &[&str] = &[
    "Scene Heading",
    "Action",
    "General",
    "Character",
    "Dialogue",
    "Parenthetical",
    "Transition",
    "Shot",
];
const MAX_NAME: usize = 40;
/// Custom elements per screenplay (contract doc 32 §2.1). Bounds hostile imports.
pub const MAX_CUSTOM_ELEMENTS: usize = 64;
const MAX_EXPORT_NAME: usize = 60;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCustomElementDto {
    pub id: String,
    pub screenplay_id: String,
    pub name: String,
    /// "Ctrl+Alt+1" … "Ctrl+Alt+9", or none.
    pub shortcut: Option<String>,
    /// action | centered | character | parenthetical | dialogue | transition | shot
    pub look: String,
    pub uppercase: bool,
    pub italic: bool,
    pub space_before: u8,
    /// A built-in element type or "custom:<id>".
    pub next_element: String,
    /// Import and export identity (Final Draft paragraph type).
    pub export_name: String,
    /// Elements using it in live drafts.
    #[ts(type = "number")]
    pub usage_count: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

const CUSTOM_COLS: &str =
    "ce.id, ce.screenplay_id, ce.name, ce.shortcut, ce.look, ce.uppercase, ce.italic,
    ce.space_before, ce.next_element, ce.export_name, ce.rev,
    (SELECT count(*) FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
       JOIN screenplay_draft d ON d.id = s.draft_id
      WHERE e.custom_element_id = ce.id AND s.deleted_at IS NULL AND d.deleted_at IS NULL)";

fn map_custom(r: &rusqlite::Row<'_>) -> rusqlite::Result<ScreenplayCustomElementDto> {
    Ok(ScreenplayCustomElementDto {
        id: r.get(0)?,
        screenplay_id: r.get(1)?,
        name: r.get(2)?,
        shortcut: r.get(3)?,
        look: r.get(4)?,
        uppercase: r.get(5)?,
        italic: r.get(6)?,
        space_before: r.get::<_, i64>(7)?.clamp(0, 3) as u8,
        next_element: r.get(8)?,
        export_name: r.get(9)?,
        rev: r.get(10)?,
        usage_count: r.get(11)?,
    })
}

/// Live custom elements of a screenplay, oldest first.
pub(crate) fn custom_elements_of(
    c: &Connection,
    screenplay_id: &str,
) -> AppResult<Vec<ScreenplayCustomElementDto>> {
    let mut stmt = c.prepare(&format!(
        "SELECT {CUSTOM_COLS} FROM screenplay_custom_element ce
         WHERE ce.screenplay_id=?1 AND ce.deleted_at IS NULL ORDER BY ce.created_at, ce.id"
    ))?;
    let rows = stmt
        .query_map([screenplay_id], map_custom)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// One live custom element.
pub(crate) fn custom_element(c: &Connection, id: &str) -> AppResult<ScreenplayCustomElementDto> {
    c.query_row(
        &format!(
            "SELECT {CUSTOM_COLS} FROM screenplay_custom_element ce WHERE ce.id=?1 AND ce.deleted_at IS NULL"
        ),
        [id],
        map_custom,
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("custom element"))
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayCustomElementArgs {
    /// Required to create; ignored when updating.
    #[serde(default)]
    #[ts(optional)]
    pub screenplay_id: Option<String>,
    /// Required to update.
    #[serde(default)]
    #[ts(optional)]
    pub id: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub name: Option<String>,
    /// Empty string clears the shortcut.
    #[serde(default)]
    #[ts(optional)]
    pub shortcut: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub look: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub uppercase: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub italic: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub space_before: Option<u8>,
    #[serde(default)]
    #[ts(optional)]
    pub next_element: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub export_name: Option<String>,
}

/// Values of a custom element after defaults and validation.
pub(crate) struct CustomSpec {
    pub name: String,
    pub shortcut: Option<String>,
    pub look: String,
    pub uppercase: bool,
    pub italic: bool,
    pub space_before: u8,
    pub next_element: String,
    pub export_name: String,
}

fn validate_shortcut(s: &str) -> AppResult<Option<String>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let normal = s.replace(' ', "");
    let digit = normal
        .strip_prefix("Ctrl+Alt+")
        .or_else(|| normal.strip_prefix("ctrl+alt+"))
        .filter(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit() && c != '0'));
    match digit {
        Some(d) => Ok(Some(format!("Ctrl+Alt+{d}"))),
        None => Err(AppError::validation(
            "shortcut",
            "Custom elements use Ctrl+Alt+1 to Ctrl+Alt+9 (Ctrl+1 to Ctrl+7 are the built-in elements).",
        )),
    }
}

/// Check a custom element against the screenplay's other custom elements.
pub(crate) fn check_custom(
    c: &Connection,
    screenplay_id: &str,
    except: Option<&str>,
    spec: &CustomSpec,
) -> AppResult<()> {
    if !CUSTOM_LOOKS.contains(&spec.look.as_str()) {
        return Err(AppError::validation(
            "look",
            "Choose how the element looks: Action, Centered, Character, Parenthetical, Dialogue, Transition or Shot.",
        ));
    }
    if rules::RULES.iter().any(|r| {
        r.label.eq_ignore_ascii_case(&spec.name) || r.kind.eq_ignore_ascii_case(&spec.name)
    }) {
        return Err(AppError::validation(
            "name",
            "That is the name of a built-in element. Choose another name.",
        ));
    }
    if RESERVED_EXPORT_NAMES
        .iter()
        .any(|n| n.eq_ignore_ascii_case(&spec.export_name))
    {
        return Err(AppError::validation(
            "exportName",
            "That name is used by Final Draft for a built-in element. Choose another export name.",
        ));
    }
    let others: Vec<ScreenplayCustomElementDto> = custom_elements_of(c, screenplay_id)?
        .into_iter()
        .filter(|x| Some(x.id.as_str()) != except)
        .collect();
    if others
        .iter()
        .any(|x| x.name.eq_ignore_ascii_case(&spec.name))
    {
        return Err(AppError::validation(
            "name",
            "Another custom element already has this name.",
        ));
    }
    if others
        .iter()
        .any(|x| x.export_name.eq_ignore_ascii_case(&spec.export_name))
    {
        return Err(AppError::validation(
            "exportName",
            "Another custom element already uses this export name.",
        ));
    }
    if let Some(sc) = &spec.shortcut
        && others
            .iter()
            .any(|x| x.shortcut.as_deref() == Some(sc.as_str()))
    {
        return Err(AppError::validation(
            "shortcut",
            format!("{sc} is already the shortcut of another custom element."),
        ));
    }
    match spec.next_element.strip_prefix("custom:") {
        Some(id) => {
            let own = except == Some(id);
            if !own && !others.iter().any(|x| x.id == id) {
                return Err(AppError::validation(
                    "nextElement",
                    "The next element must be a built-in element or a custom element of this screenplay.",
                ));
            }
        }
        None => {
            if !NEXT_BUILT_INS.contains(&spec.next_element.as_str()) {
                return Err(AppError::validation(
                    "nextElement",
                    "The next element must be a built-in element or a custom element of this screenplay.",
                ));
            }
        }
    }
    Ok(())
}

fn spec_from(
    a: &ScreenplayCustomElementArgs,
    base: Option<&ScreenplayCustomElementDto>,
) -> AppResult<CustomSpec> {
    let name = match (&a.name, base) {
        (Some(n), _) => required_text(n, "Name", MAX_NAME)?,
        (None, Some(b)) => b.name.clone(),
        (None, None) => return Err(AppError::required("Name")),
    };
    let shortcut = match (&a.shortcut, base) {
        (Some(s), _) => validate_shortcut(s)?,
        (None, Some(b)) => b.shortcut.clone(),
        (None, None) => None,
    };
    let export_name = match (&a.export_name, base) {
        (Some(n), _) if !n.trim().is_empty() => required_text(n, "Export name", MAX_EXPORT_NAME)?,
        (Some(_), _) | (None, None) => name.to_uppercase(),
        (None, Some(b)) => b.export_name.clone(),
    };
    if export_name.chars().any(|c| c.is_control()) {
        return Err(AppError::validation(
            "exportName",
            "The export name must be a single line.",
        ));
    }
    let space_before = a.space_before.or(base.map(|b| b.space_before)).unwrap_or(1);
    if space_before > 3 {
        return Err(AppError::validation(
            "spaceBefore",
            "Blank lines before an element can be 0 to 3.",
        ));
    }
    Ok(CustomSpec {
        name,
        shortcut,
        look: a
            .look
            .clone()
            .or(base.map(|b| b.look.clone()))
            .unwrap_or_else(|| "action".into()),
        uppercase: a.uppercase.or(base.map(|b| b.uppercase)).unwrap_or(false),
        italic: a.italic.or(base.map(|b| b.italic)).unwrap_or(false),
        space_before,
        next_element: a
            .next_element
            .clone()
            .or(base.map(|b| b.next_element.clone()))
            .unwrap_or_else(|| "action".into()),
        export_name,
    })
}

/// Insert a custom element row (also used by imports).
pub(crate) fn insert_custom_tx(
    tx: &Tx<'_>,
    screenplay_id: &str,
    spec: &CustomSpec,
) -> AppResult<String> {
    let id = new_id();
    let now = now_ms();
    tx.conn().execute(
        "INSERT INTO screenplay_custom_element(id, screenplay_id, name, shortcut, look, uppercase, italic, space_before,
                                               next_element, export_name, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            id,
            screenplay_id,
            spec.name,
            spec.shortcut,
            spec.look,
            spec.uppercase,
            spec.italic,
            spec.space_before as i64,
            spec.next_element,
            spec.export_name,
            now
        ],
    )?;
    Ok(id)
}

/// Resolves the custom elements of an imported document to custom elements of
/// the target screenplay, creating the missing ones. Existing names and
/// identities are read once per import, every lookup is a hash lookup, the
/// suffix search is bounded and the number of custom elements per screenplay
/// is capped, so a hostile file with thousands of look-alike styles costs
/// linear time (security review 2026-10-07). When the screenplay has no room
/// left for another custom element, [`CustomImporter::resolve`] answers `None`
/// and the caller imports the paragraph as action: an import is never refused
/// for this.
pub(crate) struct CustomImporter {
    screenplay_id: String,
    by_export: HashMap<String, String>,
    names: HashSet<String>,
    count: usize,
}

impl CustomImporter {
    pub(crate) fn load(c: &Connection, screenplay_id: &str) -> AppResult<Self> {
        let existing = custom_elements_of(c, screenplay_id)?;
        let mut names: HashSet<String> = rules::RULES
            .iter()
            .flat_map(|r| [r.label.to_lowercase(), r.kind.to_lowercase()])
            .collect();
        names.extend(existing.iter().map(|x| x.name.to_lowercase()));
        Ok(Self {
            screenplay_id: screenplay_id.to_string(),
            count: existing.len(),
            by_export: existing
                .into_iter()
                .map(|x| (x.export_name.to_lowercase(), x.id))
                .collect(),
            names,
        })
    }

    /// The custom element for `style`, created when missing, or `None` when the
    /// screenplay already holds [`MAX_CUSTOM_ELEMENTS`].
    pub(crate) fn resolve(
        &mut self,
        tx: &Tx<'_>,
        style: &openframe_import_export::CustomStyle,
    ) -> AppResult<Option<String>> {
        let export_name: String = style
            .export_name
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_EXPORT_NAME)
            .collect();
        let export_name = if export_name.trim().is_empty() {
            "IMPORTED".to_string()
        } else {
            export_name.trim().to_string()
        };
        if let Some(id) = self.by_export.get(&export_name.to_lowercase()) {
            return Ok(Some(id.clone()));
        }
        if self.count >= MAX_CUSTOM_ELEMENTS {
            return Ok(None);
        }
        let base: String = style
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(MAX_NAME)
            .collect();
        let base = if base.trim().is_empty() {
            "Imported element".to_string()
        } else {
            base.trim().to_string()
        };
        let short: String = base.chars().take(MAX_NAME - 5).collect();
        // Each clash is a different existing name, so the cap bounds the search
        // and a free name always exists within it.
        let Some(name) = std::iter::once(base.clone())
            .chain(
                (2..=MAX_CUSTOM_ELEMENTS + rules::RULES.len() * 2 + 1)
                    .map(|n| format!("{short} ({n})")),
            )
            .find(|n| !self.names.contains(&n.to_lowercase()))
        else {
            return Ok(None);
        };
        let spec = CustomSpec {
            name: name.clone(),
            shortcut: None,
            look: if CUSTOM_LOOKS.contains(&style.look.as_str()) {
                style.look.clone()
            } else {
                "action".into()
            },
            uppercase: style.uppercase,
            italic: style.italic,
            space_before: style.space_before.min(3),
            next_element: "action".into(),
            export_name: export_name.clone(),
        };
        let id = insert_custom_tx(tx, &self.screenplay_id, &spec)?;
        self.names.insert(name.to_lowercase());
        self.by_export
            .insert(export_name.to_lowercase(), id.clone());
        self.count += 1;
        Ok(Some(id))
    }
}

fn custom_elements(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayIdArgs,
) -> AppResult<Vec<ScreenplayCustomElementDto>> {
    actor.require(Capability::View, "view the screenplay")?;
    require_id(&a.screenplay_id, "screenplay")?;
    core.project()?.store.read(|c| {
        load_screenplay(c, &a.screenplay_id)?;
        custom_elements_of(c, &a.screenplay_id)
    })
}

fn create_custom_element(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayCustomElementArgs,
) -> AppResult<ScreenplayCustomElementDto> {
    let screenplay_id = a
        .screenplay_id
        .clone()
        .ok_or_else(|| AppError::required("A screenplay"))?;
    require_id(&screenplay_id, "screenplay")?;
    let spec = spec_from(&a, None)?;
    let s = core.project()?;
    let id = s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.create_custom_element",
            format!("Added the custom element “{}”", spec.name),
            Capability::Edit,
        )
        .target("screenplay", &screenplay_id),
        |tx| {
            let c = tx.conn();
            load_screenplay(c, &screenplay_id)?;
            if custom_elements_of(c, &screenplay_id)?.len() >= MAX_CUSTOM_ELEMENTS {
                return Err(AppError::validation(
                    "custom_elements",
                    format!("A screenplay can have at most {MAX_CUSTOM_ELEMENTS} custom elements."),
                ));
            }
            check_custom(c, &screenplay_id, None, &spec)?;
            insert_custom_tx(tx, &screenplay_id, &spec)
        },
    )?;
    s.store.read(|c| custom_element(c, &id))
}

fn update_custom_element(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayCustomElementArgs,
) -> AppResult<ScreenplayCustomElementDto> {
    let id =
        a.id.clone()
            .ok_or_else(|| AppError::required("A custom element"))?;
    require_id(&id, "custom element")?;
    let s = core.project()?;
    let base = s.store.read(|c| custom_element(c, &id))?;
    let spec = spec_from(&a, Some(&base))?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.update_custom_element",
            format!("Changed the custom element “{}”", spec.name),
            Capability::Edit,
        )
        .target("screenplay_custom_element", &id),
        |tx| {
            let c = tx.conn();
            let cur = custom_element(c, &id)?;
            check_custom(c, &cur.screenplay_id, Some(&id), &spec)?;
            update_fields(
                c,
                "screenplay_custom_element",
                &id,
                &[
                    ("name", text(spec.name.clone())),
                    ("shortcut", opt_text(spec.shortcut.clone())),
                    ("look", text(spec.look.clone())),
                    ("uppercase", int(spec.uppercase as i64)),
                    ("italic", int(spec.italic as i64)),
                    ("space_before", int(spec.space_before as i64)),
                    ("next_element", text(spec.next_element.clone())),
                    ("export_name", text(spec.export_name.clone())),
                ],
                &[
                    "name",
                    "shortcut",
                    "look",
                    "uppercase",
                    "italic",
                    "space_before",
                    "next_element",
                    "export_name",
                ],
                None,
                "custom element",
            )?;
            // Every draft showing it re-reads its document.
            let mut stmt = c.prepare(
                "SELECT DISTINCT s.draft_id FROM screenplay_element e JOIN screenplay_scene s ON s.id = e.scene_id
                  WHERE e.custom_element_id=?1",
            )?;
            let drafts: Vec<String> = stmt
                .query_map([&id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            for d in drafts {
                super::bump_seq(c, &d)?;
            }
            Ok(())
        },
    )?;
    s.store.read(|c| custom_element(c, &id))
}

fn delete_custom_element(core: &AppCore, actor: &Actor, a: ScreenplayIdOnlyArgs) -> AppResult<()> {
    require_id(&a.id, "custom element")?;
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "screenplay.delete_custom_element",
            "Deleted a custom element",
            Capability::SoftDelete,
        )
        .target("screenplay_custom_element", &a.id),
        |tx| {
            let c = tx.conn();
            let cur = custom_element(c, &a.id)?;
            let any_use: i64 = c.query_row(
                "SELECT count(*) FROM screenplay_element WHERE custom_element_id=?1",
                [&a.id],
                |r| r.get(0),
            )?;
            if any_use > 0 {
                return Err(AppError::conflict(format!(
                    "“{}” is used by {any_use} element(s) in this screenplay or its drafts. Change those elements to another type first.",
                    cur.name
                )));
            }
            c.execute(
                "UPDATE screenplay_custom_element SET next_element='action', updated_at=?2, rev=rev+1
                  WHERE next_element=?1 AND id<>?3",
                params![format!("custom:{}", a.id), now_ms(), a.id],
            )?;
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "screenplay_custom_element",
                    table: "screenplay_custom_element",
                    id: &a.id,
                    title: Some(cur.name.clone()),
                    parent_type: Some("screenplay"),
                    parent_id: Some(cur.screenplay_id.clone()),
                    position: None,
                },
            )
        },
    )
}

fn purge_custom_element(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    let c = tx.conn();
    let used: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM screenplay_element WHERE custom_element_id=?1)",
        [&row.object_id],
        |r| r.get(0),
    )?;
    if used {
        return Err(AppError::conflict(
            "This custom element is used by screenplay text, so it can't be permanently deleted. Restore it instead.",
        ));
    }
    c.execute(
        "DELETE FROM screenplay_custom_element WHERE id=?1",
        [&row.object_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_are_ctrl_alt_digits() {
        assert_eq!(
            validate_shortcut("ctrl+alt+3").unwrap().as_deref(),
            Some("Ctrl+Alt+3")
        );
        assert_eq!(validate_shortcut("  ").unwrap(), None);
        assert!(validate_shortcut("Ctrl+1").is_err());
        assert!(validate_shortcut("Ctrl+Alt+0").is_err());
        assert!(validate_shortcut("Ctrl+Alt+12").is_err());
    }

    #[test]
    fn page_setup_validation() {
        assert!(ScreenplayPageSetup::default().validate().is_ok());
        let bad = |f: fn(&mut ScreenplayPageSetup)| {
            let mut s = ScreenplayPageSetup::default();
            f(&mut s);
            s.validate().is_err()
        };
        assert!(bad(|s| s.font = "Comic Sans".into()));
        assert!(bad(|s| s.font_size = 9));
        assert!(bad(|s| s.font_size = 15));
        assert!(bad(|s| s.first_page_number = 0));
        assert!(bad(|s| s.header = "a\nb".into()));
        assert!(bad(|s| s.footer = "x".repeat(MAX_HEADER_CHARS + 1)));
        // Unknown or damaged stored JSON reads as the standard preset.
        let parsed: ScreenplayPageSetup = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed, ScreenplayPageSetup::default());
        assert_eq!(parsed.engine(), PageSetup::default());
    }
}
