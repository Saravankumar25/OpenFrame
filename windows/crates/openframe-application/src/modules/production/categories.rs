//! Breakdown category identity (Production depth contract section 2; depth spec section 23).
//!
//! Each of the eleven categories has a colour, an icon, a text label and a tooltip, so colour is
//! never the only signal. Normal users configure nothing: the defaults below apply. Advanced
//! settings may change the colour and the icon of a category (`breakdown_category_style` stores
//! overrides only). The text label and the tooltip stay fixed so that a category is always
//! recognisable by name in the app, in exports and to screen readers. Display metadata never
//! changes breakdown data.

use std::collections::HashMap;

use openframe_domain::enums::BreakdownCategory;
use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::parse_category;
use crate::core::AppCore;
use crate::registry::{Registry, hidden as h};
use crate::store::MutationMeta;

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Breakdown");
    r.query("breakdown.categories", list).meta(
        M::read("Breakdown category colours, icons, labels and tooltips.").hidden(h::APPEARANCE),
    );
    r.command("breakdown.set_category_style", set_style).meta(
        M::edit("Change the colour or icon of one breakdown category.").hidden(h::APPEARANCE),
    );
    r.command("breakdown.reset_category_styles", reset).meta(
        M::edit("Restore the default colours and icons of every breakdown category.")
            .hidden(h::APPEARANCE),
    );
}

/// Colour choices. Each meets 3:1 non-text contrast on the light surfaces (WCAG 1.4.11); the
/// unit test below proves it. Tokens are stored, never raw hex, so a theme can remap them.
pub const PALETTE: &[(&str, &str, &str)] = &[
    ("red", "Red", "#c92a2a"),
    ("orange", "Orange", "#c2410c"),
    ("amber", "Amber", "#a16207"),
    ("green", "Green", "#2b8a3e"),
    ("teal", "Teal", "#087f5b"),
    ("cyan", "Cyan", "#0b7285"),
    ("blue", "Blue", "#1864ab"),
    ("indigo", "Indigo", "#4054d6"),
    ("violet", "Violet", "#7048e8"),
    ("pink", "Pink", "#c2255c"),
    ("brown", "Brown", "#8b5a2b"),
    ("slate", "Slate", "#495057"),
];

/// Icon choices (keys the desktop maps to its icon set).
pub const ICONS: &[&str] = &[
    "user", "users", "map-pin", "box", "shirt", "car", "scissors", "flame", "sparkles", "volume",
    "paw", "star", "camera", "palette", "wrench", "tag",
];

/// (colour token, icon, tooltip) per category, in canonical order.
fn default_for(cat: BreakdownCategory) -> (&'static str, &'static str, &'static str) {
    use BreakdownCategory as C;
    match cat {
        C::Cast => (
            "red",
            "user",
            "Cast: characters played by credited performers.",
        ),
        C::Extras => (
            "orange",
            "users",
            "Extras / Background: background performers and crowds.",
        ),
        C::Location => (
            "green",
            "map-pin",
            "Location / Set: where the scene is shot.",
        ),
        C::Props => (
            "violet",
            "box",
            "Props: objects the cast handles or the scene shows.",
        ),
        C::Wardrobe => (
            "pink",
            "shirt",
            "Wardrobe: costumes and clothing the scene needs.",
        ),
        C::Vehicles => (
            "blue",
            "car",
            "Vehicles: cars, bikes, buses and other vehicles on screen.",
        ),
        C::HairMakeup => (
            "teal",
            "scissors",
            "Hair / Makeup: looks, wounds, ageing and special makeup.",
        ),
        C::SpecialEffects => (
            "amber",
            "flame",
            "Special Effects: practical effects on set, such as rain, smoke or fire.",
        ),
        C::Vfx => (
            "indigo",
            "sparkles",
            "VFX: visual effects added in post-production.",
        ),
        C::Sound => (
            "cyan",
            "volume",
            "Sound: playback, special recording or sound effects needed on set.",
        ),
        C::Animals => ("brown", "paw", "Animals: animals and their handlers."),
    }
}

fn hex_of(token: &str) -> &'static str {
    PALETTE
        .iter()
        .find(|(t, _, _)| *t == token)
        .map(|(_, _, hex)| *hex)
        .unwrap_or("#495057")
}

#[derive(Debug, Clone, Serialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownCategoryStyleDto {
    pub category: BreakdownCategory,
    /// Visible text label (always shown next to the colour).
    pub label: String,
    /// Accessible description used for tooltips and screen readers.
    pub tooltip: String,
    /// Palette token, e.g. "red".
    pub color: String,
    /// Hex value of the token for drawing.
    pub color_hex: String,
    /// Icon key, e.g. "box".
    pub icon: String,
    /// True when Advanced settings changed the colour or the icon.
    pub customized: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownPaletteColor {
    pub token: String,
    pub label: String,
    pub hex: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownCategoryStyles {
    pub categories: Vec<BreakdownCategoryStyleDto>,
    pub palette: Vec<BreakdownPaletteColor>,
    pub icons: Vec<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownCategoriesArgs {}

/// Omitted = unchanged.
#[derive(Debug, Deserialize, TS)]
#[ts(export, optional_fields = nullable)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreakdownCategoryStyleArgs {
    pub category: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

/// Effective styles in canonical category order.
pub(crate) fn styles(c: &Connection) -> AppResult<Vec<BreakdownCategoryStyleDto>> {
    let mut stmt = c.prepare("SELECT id, color, icon FROM breakdown_category_style")?;
    let mut over: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    for row in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })? {
        let (id, color, icon) = row?;
        over.insert(id, (color, icon));
    }
    Ok(BreakdownCategory::ALL
        .iter()
        .map(|cat| {
            let (dcolor, dicon, tooltip) = default_for(*cat);
            let (color, icon) = over.get(cat.as_str()).cloned().unwrap_or((None, None));
            // A value a newer or edited project carries that this build does not know falls back.
            let color = color
                .filter(|c| PALETTE.iter().any(|(t, _, _)| t == c))
                .unwrap_or_else(|| dcolor.to_string());
            let icon = icon
                .filter(|i| ICONS.contains(&i.as_str()))
                .unwrap_or_else(|| dicon.to_string());
            BreakdownCategoryStyleDto {
                category: *cat,
                label: cat.as_str().to_string(),
                tooltip: tooltip.to_string(),
                customized: color != dcolor || icon != dicon,
                color_hex: hex_of(&color).to_string(),
                color,
                icon,
            }
        })
        .collect())
}

fn view(c: &Connection) -> AppResult<BreakdownCategoryStyles> {
    Ok(BreakdownCategoryStyles {
        categories: styles(c)?,
        palette: PALETTE
            .iter()
            .map(|(t, l, hex)| BreakdownPaletteColor {
                token: t.to_string(),
                label: l.to_string(),
                hex: hex.to_string(),
            })
            .collect(),
        icons: ICONS.iter().map(|s| s.to_string()).collect(),
    })
}

fn list(
    core: &AppCore,
    actor: &Actor,
    _: BreakdownCategoriesArgs,
) -> AppResult<BreakdownCategoryStyles> {
    actor.require(Capability::View, "view the breakdown")?;
    let s = core.project()?;
    s.store.read(view)
}

fn set_style(
    core: &AppCore,
    actor: &Actor,
    args: BreakdownCategoryStyleArgs,
) -> AppResult<BreakdownCategoryStyles> {
    let cat = parse_category(&args.category)?;
    if let Some(c) = &args.color
        && !PALETTE.iter().any(|(t, _, _)| t == c)
    {
        return Err(AppError::validation(
            "color",
            "Choose one of the listed colours.",
        ));
    }
    if let Some(i) = &args.icon
        && !ICONS.contains(&i.as_str())
    {
        return Err(AppError::validation(
            "icon",
            "Choose one of the listed icons.",
        ));
    }
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.set_category_style",
            format!("Changed the look of {}", cat.as_str()),
            Capability::Edit,
        )
        .target("breakdown_category_style", cat.as_str()),
        |tx| {
            let (dcolor, dicon, _) = default_for(cat);
            let cur = styles(tx.conn())?
                .into_iter()
                .find(|d| d.category == cat)
                .ok_or_else(|| AppError::not_found("category"))?;
            let color = args.color.clone().unwrap_or(cur.color);
            let icon = args.icon.clone().unwrap_or(cur.icon);
            let now = now_ms();
            if color == dcolor && icon == dicon {
                tx.conn().execute(
                    "DELETE FROM breakdown_category_style WHERE id = ?1",
                    [cat.as_str()],
                )?;
            } else {
                tx.conn().execute(
                    "INSERT INTO breakdown_category_style(id, color, icon, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
                     ON CONFLICT(id) DO UPDATE SET color = excluded.color, icon = excluded.icon,
                       updated_at = excluded.updated_at, rev = rev + 1",
                    params![cat.as_str(), color, icon, now],
                )?;
            }
            Ok(())
        },
    )?;
    s.store.read(view)
}

fn reset(
    core: &AppCore,
    actor: &Actor,
    _: BreakdownCategoriesArgs,
) -> AppResult<BreakdownCategoryStyles> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "breakdown.reset_category_styles",
            "Restored the default category colours and icons",
            Capability::Edit,
        ),
        |tx| {
            tx.conn()
                .execute("DELETE FROM breakdown_category_style", [])?;
            Ok(())
        },
    )?;
    s.store.read(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(hex: &str) -> f64 {
        let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap();
        let ch = |shift: u32| {
            let c = ((v >> shift) & 0xff) as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(16) + 0.7152 * ch(8) + 0.0722 * ch(0)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_palette_colour_meets_non_text_contrast_on_light_surfaces() {
        // White panels and the warm page background of the default look.
        for surface in ["#ffffff", "#f6f4ef"] {
            for (token, _, hex) in PALETTE {
                let r = contrast(hex, surface);
                assert!(r >= 3.0, "{token} {hex} on {surface}: {r:.2} < 3:1");
            }
        }
    }

    #[test]
    fn defaults_are_distinct_and_valid_for_all_eleven_categories() {
        assert_eq!(BreakdownCategory::ALL.len(), 11);
        let mut colors = std::collections::HashSet::new();
        let mut icons = std::collections::HashSet::new();
        for cat in BreakdownCategory::ALL {
            let (c, i, tip) = default_for(*cat);
            assert!(PALETTE.iter().any(|(t, _, _)| *t == c), "{cat}: colour {c}");
            assert!(ICONS.contains(&i), "{cat}: icon {i}");
            assert!(
                tip.starts_with(cat.as_str()) && tip.ends_with('.'),
                "{cat}: tooltip names the category"
            );
            assert!(colors.insert(c), "{cat}: colour {c} used twice");
            assert!(icons.insert(i), "{cat}: icon {i} used twice");
        }
    }
}
