//! Keyboard labels. Windows shows `Ctrl`, `Alt`, `Shift`; macOS shows the Command, Option and
//! Shift symbols. The web view sends the same logical shortcut on both systems; only the label
//! differs. Feature code asks for a label and never checks the platform itself.

use crate::{Os, current_os};

/// The platform's primary modifier key, as shown to the user.
pub fn primary_modifier_label() -> &'static str {
    primary_modifier_label_for(current_os())
}

/// The platform's secondary modifier key (`Alt` or Option), as shown to the user.
pub fn secondary_modifier_label() -> &'static str {
    secondary_modifier_label_for(current_os())
}

/// Render a shortcut such as `"Mod+Shift+S"` for display: `Mod` becomes the platform's primary
/// modifier, `Alt` becomes Option on macOS, keys are joined with `+` on Windows and without a
/// separator on macOS, following each system's convention.
pub fn display_shortcut(logical: &str) -> String {
    display_shortcut_for(current_os(), logical)
}

pub(crate) fn primary_modifier_label_for(os: Os) -> &'static str {
    match os {
        Os::MacOs => "\u{2318}",
        Os::Windows | Os::Other => "Ctrl",
    }
}

pub(crate) fn secondary_modifier_label_for(os: Os) -> &'static str {
    match os {
        Os::MacOs => "\u{2325}",
        Os::Windows | Os::Other => "Alt",
    }
}

pub(crate) fn display_shortcut_for(os: Os, logical: &str) -> String {
    let parts: Vec<&str> = logical
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let mapped = parts.iter().map(|p| match (*p, os) {
        ("Mod", _) => primary_modifier_label_for(os).to_string(),
        ("Alt", Os::MacOs) => secondary_modifier_label_for(os).to_string(),
        ("Shift", Os::MacOs) => "\u{21E7}".to_string(),
        ("Enter", Os::MacOs) => "\u{21A9}".to_string(),
        ("Backspace", Os::MacOs) => "\u{232B}".to_string(),
        (other, _) => other.to_string(),
    });
    match os {
        Os::MacOs => mapped.collect::<Vec<_>>().join(""),
        Os::Windows | Os::Other => mapped.collect::<Vec<_>>().join("+"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_labels() {
        assert_eq!(
            display_shortcut_for(Os::Windows, "Mod+Shift+S"),
            "Ctrl+Shift+S"
        );
        assert_eq!(display_shortcut_for(Os::Windows, "Mod+Z"), "Ctrl+Z");
        assert_eq!(display_shortcut_for(Os::Windows, "Alt+Enter"), "Alt+Enter");
    }

    #[test]
    fn mac_labels() {
        assert_eq!(
            display_shortcut_for(Os::MacOs, "Mod+Shift+S"),
            "\u{2318}\u{21E7}S"
        );
        assert_eq!(
            display_shortcut_for(Os::MacOs, "Alt+Enter"),
            "\u{2325}\u{21A9}"
        );
    }

    #[test]
    fn whitespace_and_empty_parts_are_ignored() {
        assert_eq!(display_shortcut_for(Os::Windows, " Mod + S "), "Ctrl+S");
        assert_eq!(display_shortcut_for(Os::Windows, "Mod++S"), "Ctrl+S");
        assert_eq!(display_shortcut_for(Os::Windows, ""), "");
    }

    #[test]
    fn current_platform_is_one_of_the_two_shipped_adapters_or_other() {
        let label = primary_modifier_label();
        assert!(label == "Ctrl" || label == "\u{2318}");
        let _ = secondary_modifier_label();
        let _ = display_shortcut("Mod+S");
    }
}
