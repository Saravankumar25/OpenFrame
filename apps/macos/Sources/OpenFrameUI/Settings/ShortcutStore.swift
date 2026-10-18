// The shortcuts the person changed, kept in step with the core (app.shortcuts). Menus and
// screens ask this store for the keys of a command, so a change made in Keyboard Shortcuts
// reaches them at once, and a command nobody changed keeps its default.

import Observation
import OpenFrameCore
import SwiftUI

@MainActor
@Observable
final class ShortcutStore {
    static let shared = ShortcutStore()

    private(set) var overrides: [String: [String]] = [:]
    private(set) var loaded = false

    func set(_ answer: ShortcutOverrides) {
        var map: [String: [String]] = [:]
        for item in answer.overrides where ShortcutCatalogue.command(item.commandId) != nil {
            map[item.commandId] = item.keys
        }
        overrides = map
        loaded = true
    }

    /// Reads the saved set. A failed read keeps what is applied now: a passing failure must not
    /// quietly switch the person's shortcuts back to the defaults.
    func reload(core: Core?) async {
        guard let core else { return }
        if let answer = try? await core.appShortcuts() {
            set(answer)
        } else {
            loaded = true
        }
    }

    func chords(_ id: String) -> [String] { ShortcutCatalogue.bindings(id, overrides) }

    /// The first key of a command for a menu item, or nil for "no shortcut".
    func shortcut(_ id: String) -> KeyboardShortcut? {
        chords(id).lazy.compactMap(ShortcutCatalogue.keyboardShortcut(for:)).first
    }

    /// The first key of a command ready for a tooltip ("" when it has none).
    func label(_ id: String) -> String {
        chords(id).first.map(ShortcutCatalogue.display) ?? ""
    }

    /// "New Project (\u{2318}N)" style tooltip for a command, or the plain name when it has no key.
    func tooltip(_ name: String, _ id: String) -> String {
        let keys = label(id)
        return keys.isEmpty ? name : "\(name) (\(keys))"
    }

    /// The same in words, as the toolbar help names keys: "New Project (Command+N)".
    func tooltipWords(_ name: String, _ id: String) -> String {
        guard let first = chords(id).first else { return name }
        return "\(name) (\(ShortcutCatalogue.words(first)))"
    }

    /// Does this key press run the command with the keys now in force?
    func matches(_ event: NSEvent, _ id: String) -> Bool {
        guard case let .chord(chord) = ShortcutCatalogue.press(event) else { return false }
        return chords(id).contains(chord)
    }

    /// For tests: forget everything.
    func resetForTests() {
        overrides = [:]
        loaded = false
    }
}
