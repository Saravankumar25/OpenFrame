//! Element rule table (contract doc 32 §2.2): one profile per screenplay
//! element drives the editor (labels, shortcuts, what Enter creates), the page
//! layout (position, width, alignment, case, spacing, keep-together and split
//! rules) and the interchange mapping (Final Draft paragraph type, Fountain).
//!
//! The editor reads the same table from
//! `apps/desktop/src/workspaces/screenplay/editor/elementRules.json`, which a
//! test regenerates from [`RULES`] so the two can never drift.
//!
//! Positions are in inches from the paper's left edge. Widths are in inches
//! and turn into characters through the font size (12 pt Courier prints 10
//! characters per inch, so a 6.0" action column holds 60 characters).

use serde::Serialize;

/// Horizontal alignment inside the element's column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementRule {
    /// Stored element type (`screenplay_element.element_type`) or an
    /// interchange-only kind (`centered`, `lyric`).
    pub kind: &'static str,
    pub label: &'static str,
    /// Default shortcut (the person can change it under Settings, Keyboard shortcuts).
    pub shortcut: Option<&'static str>,
    pub left_in: f32,
    pub width_in: f32,
    pub align: Align,
    pub uppercase: bool,
    pub italic: bool,
    /// Blank lines before the element.
    pub space_before: u8,
    /// Accepts line breaks inside the element.
    pub multiline: bool,
    /// What Enter at the end of a non-empty element creates.
    pub next_on_enter: &'static str,
    /// Never left alone at the foot of a page: kept with the start of what follows.
    pub keep_with_next: bool,
    /// May be split across a page boundary.
    pub splittable: bool,
    /// Part of a speech block (cue, parenthetical, dialogue).
    pub speech: bool,
    /// Printed in clean screenplay output.
    pub printed: bool,
    /// Final Draft paragraph `Type`.
    pub fdx_type: &'static str,
    /// How the element is written in Fountain.
    pub fountain: &'static str,
    /// The element is usable as the look of a custom element.
    pub custom_look: bool,
}

const fn rule(kind: &'static str, label: &'static str) -> ElementRule {
    ElementRule {
        kind,
        label,
        shortcut: None,
        left_in: 1.5,
        width_in: 6.0,
        align: Align::Left,
        uppercase: false,
        italic: false,
        space_before: 1,
        multiline: false,
        next_on_enter: "action",
        keep_with_next: false,
        splittable: false,
        speech: false,
        printed: true,
        fdx_type: "Action",
        fountain: "plain paragraph",
        custom_look: false,
    }
}

/// The built-in element profiles. Order is the element menu order.
pub const RULES: &[ElementRule] = &[
    ElementRule {
        shortcut: Some("Ctrl+1"),
        uppercase: true,
        next_on_enter: "action",
        keep_with_next: true,
        fdx_type: "Scene Heading",
        fountain: "line starting INT., EXT., I/E or a forced '.'",
        ..rule("scene_heading", "Scene Heading")
    },
    ElementRule {
        shortcut: Some("Ctrl+2"),
        multiline: true,
        next_on_enter: "character",
        splittable: true,
        custom_look: true,
        ..rule("action", "Action")
    },
    ElementRule {
        shortcut: Some("Ctrl+3"),
        left_in: 3.7,
        width_in: 3.8,
        uppercase: true,
        next_on_enter: "dialogue",
        keep_with_next: true,
        speech: true,
        fdx_type: "Character",
        fountain: "upper-case line followed by dialogue (forced with '@')",
        custom_look: true,
        ..rule("character", "Character")
    },
    ElementRule {
        shortcut: Some("Ctrl+4"),
        left_in: 3.1,
        width_in: 2.6,
        space_before: 0,
        next_on_enter: "dialogue",
        keep_with_next: true,
        speech: true,
        fdx_type: "Parenthetical",
        fountain: "line in parentheses inside dialogue",
        custom_look: true,
        ..rule("parenthetical", "Parenthetical")
    },
    ElementRule {
        shortcut: Some("Ctrl+5"),
        left_in: 2.5,
        width_in: 3.5,
        space_before: 0,
        multiline: true,
        next_on_enter: "action",
        splittable: true,
        speech: true,
        fdx_type: "Dialogue",
        fountain: "lines under a character cue",
        custom_look: true,
        ..rule("dialogue", "Dialogue")
    },
    ElementRule {
        shortcut: Some("Ctrl+6"),
        align: Align::Right,
        uppercase: true,
        next_on_enter: "scene_heading",
        fdx_type: "Transition",
        fountain: "upper-case line ending in TO: (forced with '>')",
        custom_look: true,
        ..rule("transition", "Transition")
    },
    ElementRule {
        shortcut: Some("Ctrl+7"),
        uppercase: true,
        next_on_enter: "action",
        fdx_type: "Shot",
        fountain: "upper-case line (forced with '!')",
        custom_look: true,
        ..rule("shot", "Shot")
    },
    ElementRule {
        italic: true,
        multiline: true,
        printed: false,
        fdx_type: "General",
        fountain: "[[note]]",
        ..rule("note", "General note (not printed)")
    },
    ElementRule {
        space_before: 0,
        printed: true,
        shortcut: Some("Ctrl+Enter"),
        fdx_type: "StartsNewPage",
        fountain: "===",
        ..rule("page_break", "Page break")
    },
    ElementRule {
        align: Align::Center,
        custom_look: true,
        fountain: ">text<",
        ..rule("centered", "Centered")
    },
    ElementRule {
        left_in: 2.5,
        width_in: 3.5,
        space_before: 0,
        italic: true,
        multiline: true,
        speech: true,
        fdx_type: "Dialogue",
        fountain: "~lyric",
        ..rule("lyric", "Lyric")
    },
];

/// The profile for a kind (`action` when unknown, so layout never fails).
pub fn rule_for(kind: &str) -> &'static ElementRule {
    RULES.iter().find(|r| r.kind == kind).unwrap_or(&RULES[1])
}

/// Looks a custom element may use (the built-in element whose position it takes).
pub fn custom_looks() -> impl Iterator<Item = &'static ElementRule> {
    RULES.iter().filter(|r| r.custom_look)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_stored_element_type_has_a_rule() {
        for t in openframe_domain::enums::ElementType::ALL {
            if *t == openframe_domain::enums::ElementType::Custom {
                continue;
            }
            assert_eq!(rule_for(t.as_str()).kind, t.as_str(), "{t}");
        }
    }

    #[test]
    fn next_elements_are_known_kinds() {
        for r in RULES {
            assert!(
                RULES.iter().any(|x| x.kind == r.next_on_enter),
                "{} -> {}",
                r.kind,
                r.next_on_enter
            );
        }
    }

    #[test]
    fn standard_columns_match_the_industry_layout() {
        // 12 pt Courier, 10 characters per inch.
        let chars = |k: &str| (rule_for(k).width_in * 10.0).round() as u32;
        assert_eq!(chars("action"), 60);
        assert_eq!(chars("character"), 38);
        assert_eq!(chars("parenthetical"), 26);
        assert_eq!(chars("dialogue"), 35);
    }
}
