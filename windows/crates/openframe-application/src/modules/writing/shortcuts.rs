//! Custom keyboard shortcuts (SCRIPT-019).
//!
//! The list of commands and their default keys lives with the screens that run them (the
//! desktop app's shortcut catalogue), because every command there also has an on-screen route.
//! This module stores only what the person changed: for a command id, the keys that replace its
//! defaults (an empty list means "no shortcut"). A command without an entry uses its defaults,
//! so "restore default" removes the entry and "restore all" removes them all.
//!
//! Keys are written as logical chords such as `Mod+Shift+N` or `Alt+ArrowDown` (`Mod` is Ctrl
//! on Windows and Command on macOS). Every chord is validated here so a damaged or hand-made
//! request can never store a shortcut that would block typing or the clipboard. Conflicts
//! between commands are resolved by the screen before saving; a batch of changes is written in
//! one step so moving a shortcut from one command to another can never leave both holding it.

use std::collections::{BTreeMap, HashSet};

use openframe_domain::{Actor, AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;

use super::{read_app_setting, require_local, write_app_setting};
use crate::core::AppCore;
use crate::events::AppEvent;
use crate::registry::Registry;

/// `sys_settings` key: a JSON object `{ commandId: [chord, ...] }`.
pub const SHORTCUTS_KEY: &str = "keyboard.shortcuts";
pub const MAX_KEYS_PER_COMMAND: usize = 4;
pub const MAX_COMMANDS: usize = 300;
const MAX_CHANGES: usize = 50;

pub fn register(r: &mut Registry) {
    use crate::registry::{OperationMetadata as M, hidden as h};
    r.query("app.shortcuts", list).meta(
        M::app_query("Keyboard shortcuts the person at this computer has changed.")
            .hidden(h::PREFERENCE),
    );
    r.command("app.set_shortcuts", set).meta(
        M::app_command("Change, remove or restore keyboard shortcuts.").hidden(h::PREFERENCE),
    );
    r.command("app.reset_shortcuts", reset_all).meta(
        M::app_command("Restore every keyboard shortcut to its default.")
            .destructive()
            .irreversible()
            .confirm()
            .hidden(h::PREFERENCE),
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutOverride {
    pub command_id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutOverrides {
    pub overrides: Vec<ShortcutOverride>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShortcutChange {
    pub command_id: String,
    /// The new keys (empty = no shortcut). `null` restores the command's defaults.
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetShortcutsArgs {
    pub changes: Vec<ShortcutChange>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShortcutsArgs {}

const MODIFIERS: [&str; 3] = ["Mod", "Alt", "Shift"];
const NAMED_KEYS: &[&str] = &[
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Enter",
    "Escape",
    "Tab",
    "Backspace",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Space",
    "Insert",
];
const PUNCTUATION: &[&str] = &["[", "]", ";", "'", ",", ".", "/", "\\", "-", "=", "`"];
/// Kept for the clipboard and select-all in every text field.
const RESERVED: &[&str] = &["Mod+A", "Mod+C", "Mod+V", "Mod+X"];
/// Owned by the screenplay's custom elements (each custom element may take one of them under
/// View, Custom elements), so no other command may hold them.
const CUSTOM_ELEMENT_KEYS: &[&str] = &[
    "Mod+Alt+1",
    "Mod+Alt+2",
    "Mod+Alt+3",
    "Mod+Alt+4",
    "Mod+Alt+5",
    "Mod+Alt+6",
    "Mod+Alt+7",
    "Mod+Alt+8",
    "Mod+Alt+9",
];

fn is_function_key(k: &str) -> bool {
    k.strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n))
}

/// Validate one chord and return it in canonical form (`Mod`, `Alt`, `Shift`, then the key).
pub fn canonical_chord(raw: &str) -> AppResult<String> {
    let invalid = || AppError::validation("keys", "That key combination can't be used.");
    if raw.len() > 40 {
        return Err(invalid());
    }
    let parts: Vec<&str> = raw.split('+').map(str::trim).collect();
    let (key, mods) = parts.split_last().ok_or_else(invalid)?;
    let key = if key.len() == 1 {
        key.to_ascii_uppercase()
    } else {
        key.to_string()
    };
    let mut seen = HashSet::new();
    for raw in mods {
        // Modifier names are accepted in any case ("shift" is "Shift").
        let m = MODIFIERS
            .iter()
            .find(|x| x.eq_ignore_ascii_case(raw))
            .ok_or_else(invalid)?;
        if !seen.insert(*m) {
            return Err(invalid());
        }
    }
    let single_char = key.len() == 1
        && key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    let known = single_char
        || PUNCTUATION.contains(&key.as_str())
        || NAMED_KEYS.contains(&key.as_str())
        || is_function_key(&key);
    if !known {
        return Err(invalid());
    }
    let has_mod = seen.contains("Mod") || seen.contains("Alt");
    if !has_mod && !is_function_key(&key) {
        return Err(AppError::validation(
            "keys",
            "Add Ctrl or Alt to that key, otherwise it would stop you typing.",
        ));
    }
    let mut out: Vec<&str> = MODIFIERS
        .iter()
        .copied()
        .filter(|m| seen.contains(m))
        .collect();
    out.push(&key);
    let chord = out.join("+");
    if RESERVED.contains(&chord.as_str()) {
        return Err(AppError::validation(
            "keys",
            "That shortcut is kept for copy, cut, paste and select all.",
        ));
    }
    if CUSTOM_ELEMENT_KEYS.contains(&chord.as_str()) {
        return Err(AppError::validation(
            "keys",
            "Ctrl+Alt+1 to Ctrl+Alt+9 belong to the screenplay's custom elements. Choose another shortcut.",
        ));
    }
    Ok(chord)
}

fn valid_command_id(id: &str) -> bool {
    let mut parts = id.split('.');
    let ok_part = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    id.len() <= 64
        && id.contains('.')
        && parts
            .next()
            .is_some_and(|p| ok_part(p) && p.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
        && parts.all(ok_part)
}

fn load(core: &AppCore) -> AppResult<BTreeMap<String, Vec<String>>> {
    let raw = read_app_setting(core, SHORTCUTS_KEY)?;
    let map: BTreeMap<String, Vec<String>> = raw
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    // Anything that no longer validates (a hand-edited database) is dropped, never applied.
    Ok(map
        .into_iter()
        .filter(|(k, _)| valid_command_id(k))
        .filter_map(|(k, keys)| {
            let keys: AppResult<Vec<String>> = keys.iter().map(|c| canonical_chord(c)).collect();
            keys.ok()
                .filter(|v| v.len() <= MAX_KEYS_PER_COMMAND)
                .map(|v| (k, v))
        })
        .take(MAX_COMMANDS)
        .collect())
}

fn to_dto(map: BTreeMap<String, Vec<String>>) -> ShortcutOverrides {
    ShortcutOverrides {
        overrides: map
            .into_iter()
            .map(|(command_id, keys)| ShortcutOverride { command_id, keys })
            .collect(),
    }
}

fn save(core: &AppCore, map: &BTreeMap<String, Vec<String>>) -> AppResult<()> {
    if map.is_empty() {
        write_app_setting(core, SHORTCUTS_KEY, None)?;
    } else {
        write_app_setting(core, SHORTCUTS_KEY, Some(&json!(map)))?;
    }
    core.emit(AppEvent::Module {
        module: "writing".into(),
        name: "shortcutsChanged".into(),
        payload: json!({ "count": map.len() }),
    });
    Ok(())
}

fn list(core: &AppCore, _a: &Actor, _: ShortcutsArgs) -> AppResult<ShortcutOverrides> {
    Ok(to_dto(load(core)?))
}

fn set(core: &AppCore, actor: &Actor, a: SetShortcutsArgs) -> AppResult<ShortcutOverrides> {
    require_local(actor, "keyboard shortcuts")?;
    if a.changes.is_empty() || a.changes.len() > MAX_CHANGES {
        return Err(AppError::invalid_input("Nothing to change."));
    }
    let _guard = super::settings_lock();
    let mut map = load(core)?;
    // Validate everything first so a bad entry changes nothing.
    let mut staged: Vec<(String, Option<Vec<String>>)> = Vec::with_capacity(a.changes.len());
    for ch in a.changes {
        if !valid_command_id(&ch.command_id) {
            return Err(AppError::invalid_input("Unknown command."));
        }
        let keys = match ch.keys {
            None => None,
            Some(keys) => {
                if keys.len() > MAX_KEYS_PER_COMMAND {
                    return Err(AppError::validation(
                        "keys",
                        format!("A command can have at most {MAX_KEYS_PER_COMMAND} shortcuts."),
                    ));
                }
                let mut out: Vec<String> = Vec::with_capacity(keys.len());
                for k in &keys {
                    let c = canonical_chord(k)?;
                    if out.contains(&c) {
                        return Err(AppError::validation(
                            "keys",
                            "That shortcut is already on this command.",
                        ));
                    }
                    out.push(c);
                }
                Some(out)
            }
        };
        staged.push((ch.command_id, keys));
    }
    for (id, keys) in staged {
        match keys {
            None => {
                map.remove(&id);
            }
            Some(k) => {
                map.insert(id, k);
            }
        }
    }
    if map.len() > MAX_COMMANDS {
        return Err(AppError::invalid_input("Too many changed shortcuts."));
    }
    save(core, &map)?;
    Ok(to_dto(map))
}

fn reset_all(core: &AppCore, actor: &Actor, _: ShortcutsArgs) -> AppResult<ShortcutOverrides> {
    require_local(actor, "keyboard shortcuts")?;
    let _guard = super::settings_lock();
    let map = BTreeMap::new();
    save(core, &map)?;
    Ok(to_dto(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fsd_script_019_chords_are_canonical_and_safe() {
        assert_eq!(canonical_chord("Shift+Mod+n").unwrap(), "Mod+Shift+N");
        assert_eq!(canonical_chord("Alt+ArrowDown").unwrap(), "Alt+ArrowDown");
        assert_eq!(canonical_chord("F7").unwrap(), "F7");
        assert_eq!(canonical_chord("Mod+Alt+0").unwrap(), "Mod+Alt+0");
        // Ctrl+Alt+1 to 9 belong to custom elements.
        for d in 1..=9 {
            let e = canonical_chord(&format!("mod+alt+{d}")).unwrap_err();
            assert!(e.message.contains("custom elements"), "{}", e.message);
        }
        assert_eq!(canonical_chord("Mod+/").unwrap(), "Mod+/");
        // Typing keys, the clipboard and nonsense are refused.
        for bad in [
            "K",
            "Shift+K",
            "Enter",
            "ArrowDown",
            "Mod+C",
            "Mod+V",
            "Mod+X",
            "Mod+A",
            "Mod+Mod+K",
            "Ctrl+K",
            "Mod+",
            "",
            "Mod+F99",
            "Mod+Shift+\u{00E9}",
            "Mod+K+J",
        ] {
            assert!(canonical_chord(bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn command_ids_are_dotted_lowercase_names() {
        assert!(valid_command_id("screenplay.element_action"));
        assert!(valid_command_id("app.search"));
        assert!(!valid_command_id("Search"));
        assert!(!valid_command_id("app."));
        assert!(!valid_command_id("app.séarch"));
        assert!(!valid_command_id(&format!("a.{}", "b".repeat(70))));
    }
}
