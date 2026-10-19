// The catalogue of commands that have a keyboard shortcut (Windows: app/shortcuts.ts). The core
// stores only what the person changed and checks every key combination before it is saved
// (app.set_shortcuts); this file lists the commands with their default keys and the on-screen
// route that does the same thing without a keyboard, and finds conflicts between commands.
// Keys are written as logical chords such as "Mod+Shift+N", where Mod is Command on the Mac.

import AppKit
import SwiftUI

enum ShortcutScope: String, Sendable {
    case project, screenplay, vault, home
}

struct ShortcutCommand: Identifiable, Hashable, Sendable {
    let id: String
    let label: String
    let group: String
    let scope: ShortcutScope
    let defaults: [String]
    /// The on-screen way to run the same command.
    let route: String
}

enum ShortcutCatalogue {
    static let maxKeysPerCommand = 4

    private static func element(_ id: String, _ label: String, _ key: String?) -> ShortcutCommand {
        ShortcutCommand(
            id: "screenplay.element_\(id)", label: "\(label) element", group: "Screenplay elements",
            scope: .screenplay, defaults: key.map { [$0] } ?? [], route: "Element menu at the bottom of the page")
    }

    static let commands: [ShortcutCommand] = [
        ShortcutCommand(id: "app.search", label: "Search", group: "General", scope: .project, defaults: ["Mod+K"], route: "Edit menu: Search and Commands"),
        ShortcutCommand(id: "app.assistant", label: "Open or close the assistant", group: "General", scope: .project, defaults: ["Mod+J"], route: "Assistant button in the toolbar"),
        ShortcutCommand(id: "app.project_settings", label: "Open Project Settings", group: "General", scope: .project, defaults: ["Mod+Shift+,"], route: "Project Settings button in the toolbar"),
        ShortcutCommand(id: "app.new_scene_card", label: "New Scene Card", group: "General", scope: .project, defaults: ["Mod+N"], route: "Story: Add"),
        ShortcutCommand(id: "app.quick_capture", label: "Quick Capture (new idea)", group: "General", scope: .project, defaults: ["Mod+Shift+N"], route: "Idea Vault: New"),
        ShortcutCommand(id: "home.new_project", label: "New Project", group: "Project list", scope: .home, defaults: ["Mod+N"], route: "File menu: New Project"),
        ShortcutCommand(id: "home.open_project", label: "Open Project", group: "Project list", scope: .home, defaults: ["Mod+O"], route: "File menu: Open Project"),
        ShortcutCommand(id: "home.search", label: "Search projects", group: "Project list", scope: .home, defaults: ["Mod+F", "Mod+K"], route: "Search box in the toolbar"),
        ShortcutCommand(id: "home.quick_capture", label: "Quick Capture into the Global Idea Vault", group: "Project list", scope: .home, defaults: ["Mod+Shift+N"], route: "Global Idea Vault button in the toolbar"),
        ShortcutCommand(id: "vault.search", label: "Search ideas", group: "Idea Vault", scope: .vault, defaults: ["Mod+F"], route: "Idea Vault: search box"),
        ShortcutCommand(id: "screenplay.find", label: "Find in script", group: "Screenplay", scope: .screenplay, defaults: ["Mod+F"], route: "Screenplay toolbar: Find"),
        ShortcutCommand(id: "screenplay.next_scene", label: "Next scene", group: "Screenplay", scope: .screenplay, defaults: ["Alt+ArrowDown"], route: "Scene navigator on the left"),
        ShortcutCommand(id: "screenplay.previous_scene", label: "Previous scene", group: "Screenplay", scope: .screenplay, defaults: ["Alt+ArrowUp"], route: "Scene navigator on the left"),
        ShortcutCommand(id: "screenplay.next_misspelling", label: "Next spelling mistake", group: "Screenplay", scope: .screenplay, defaults: ["F7"], route: "Right-click an underlined word"),
        ShortcutCommand(id: "screenplay.page_break", label: "Insert page break", group: "Screenplay", scope: .screenplay, defaults: ["Mod+Enter"], route: "Element menu: Insert page break"),
        element("scene_heading", "Scene Heading", "Mod+1"),
        element("action", "Action", "Mod+2"),
        element("character", "Character", "Mod+3"),
        element("parenthetical", "Parenthetical", "Mod+4"),
        element("dialogue", "Dialogue", "Mod+5"),
        element("transition", "Transition", "Mod+6"),
        element("shot", "Shot", "Mod+7"),
        element("note", "General note (not printed)", nil),
    ]

    private static let byId: [String: ShortcutCommand] = Dictionary(uniqueKeysWithValues: commands.map { ($0.id, $0) })

    static func command(_ id: String) -> ShortcutCommand? { byId[id] }
    static func label(_ id: String) -> String { byId[id]?.label ?? id }

    // MARK: Bindings and conflicts

    /// The keys a command uses: the person's own, or the defaults.
    static func bindings(_ id: String, _ overrides: [String: [String]]) -> [String] {
        overrides[id] ?? byId[id]?.defaults ?? []
    }

    /// The screenplay editor and the Idea Vault are parts of the project window, so a project
    /// shortcut works in both; they never show at the same time.
    private static func overlaps(_ a: ShortcutScope, _ b: ShortcutScope) -> Bool {
        if a == b { return true }
        let pair: Set<ShortcutScope> = [a, b]
        return pair.contains(.project) && (pair.contains(.screenplay) || pair.contains(.vault))
    }

    /// Commands (other than `except`) that already use `chord` where `scope` is active.
    static func whoUses(_ chord: String, scope: ShortcutScope, _ overrides: [String: [String]], except: String? = nil) -> [String] {
        commands.filter { c in
            c.id != except && overlaps(c.scope, scope) && bindings(c.id, overrides).contains(chord)
        }.map(\.id)
    }

    struct Conflict: Hashable {
        let chord: String
        let commandId: String
        let otherId: String
    }

    /// Every pair of commands sharing a chord in overlapping scopes (a damaged or old saved set).
    static func conflicts(_ overrides: [String: [String]]) -> [Conflict] {
        var out: [Conflict] = []
        for c in commands {
            for chord in bindings(c.id, overrides) {
                for other in whoUses(chord, scope: c.scope, overrides, except: c.id) {
                    out.append(Conflict(chord: chord, commandId: c.id, otherId: other))
                }
            }
        }
        return out
    }

    // MARK: Showing a chord

    private static let keyNames: [String: String] = [
        "ArrowUp": "\u{2191}", "ArrowDown": "\u{2193}", "ArrowLeft": "\u{2190}", "ArrowRight": "\u{2192}",
        "Escape": "Esc", "Enter": "\u{21A9}", "Backspace": "\u{232B}", "Delete": "\u{2326}", "Tab": "\u{21E5}",
        "PageUp": "Page Up", "PageDown": "Page Down", "Space": "Space",
    ]

    /// How a chord is shown on the Mac: "\u{21E7}\u{2318}N" for Mod+Shift+N.
    static func display(_ chord: String) -> String {
        let parts = chord.split(separator: "+", omittingEmptySubsequences: false).map(String.init)
        // "Mod+," splits cleanly; a trailing "+" key does not exist in the grammar.
        var mods = ""
        var key = ""
        for (index, part) in parts.enumerated() {
            if index == parts.count - 1 {
                key = keyNames[part] ?? part
            } else {
                mods += part
            }
        }
        // Mac order: Control, Option, Shift, Command.
        var shown = ""
        if mods.contains("Alt") { shown += "\u{2325}" }
        if mods.contains("Shift") { shown += "\u{21E7}" }
        if mods.contains("Mod") { shown += "\u{2318}" }
        return shown + key
    }

    /// The same chord in words, for tooltips and the "Command+N" style the Mac menus use in help.
    static func words(_ chord: String) -> String {
        chord.split(separator: "+").map { part -> String in
            switch part {
            case "Mod": "Command"
            case "Alt": "Option"
            default: String(part)
            }
        }.joined(separator: "+")
    }

    // MARK: Chords from key presses

    enum Press: Equatable {
        /// A modifier on its own, or a key with no meaning: keep listening.
        case ignore
        case chord(String)
        /// A combination the shortcut grammar cannot hold (Control combinations).
        case unsupported
    }

    private static let keyCodeToKey: [UInt16: String] = [
        18: "1", 19: "2", 20: "3", 21: "4", 23: "5", 22: "6", 26: "7", 28: "8", 25: "9", 29: "0",
        33: "[", 30: "]", 41: ";", 39: "'", 43: ",", 47: ".", 44: "/", 42: "\\", 27: "-", 24: "=", 50: "`",
        0: "A", 11: "B", 8: "C", 2: "D", 14: "E", 3: "F", 5: "G", 4: "H", 34: "I", 38: "J", 40: "K", 37: "L",
        46: "M", 45: "N", 31: "O", 35: "P", 12: "Q", 15: "R", 1: "S", 17: "T", 32: "U", 9: "V", 13: "W",
        7: "X", 16: "Y", 6: "Z",
        36: "Enter", 76: "Enter", 53: "Escape", 48: "Tab", 51: "Backspace", 49: "Space", 117: "Delete",
        123: "ArrowLeft", 124: "ArrowRight", 125: "ArrowDown", 126: "ArrowUp",
        115: "Home", 119: "End", 116: "PageUp", 121: "PageDown", 114: "Insert",
        122: "F1", 120: "F2", 99: "F3", 118: "F4", 96: "F5", 97: "F6", 98: "F7", 100: "F8", 101: "F9",
        109: "F10", 103: "F11", 111: "F12", 105: "F13", 107: "F14", 113: "F15", 106: "F16", 64: "F17",
        79: "F18", 80: "F19", 90: "F20",
    ]

    /// The chord a key press makes. Letters follow the layout the person types with (so Command+Z
    /// stays Z on a German keyboard); digits and punctuation use the physical key.
    static func press(keyCode: UInt16, characters: String?, command: Bool, option: Bool, shift: Bool, control: Bool) -> Press {
        // Modifier keys on their own have key codes of their own (54-63) and no entry here.
        guard let physical = keyCodeToKey[keyCode] else { return .ignore }
        var key = physical
        if physical.count == 1, physical.first?.isLetter == true,
           let typed = characters?.uppercased(), typed.count == 1, typed.first?.isLetter == true,
           typed.unicodeScalars.allSatisfy({ $0.isASCII }) {
            key = typed
        }
        if control { return .unsupported }
        var parts: [String] = []
        if command { parts.append("Mod") }
        if option { parts.append("Alt") }
        if shift { parts.append("Shift") }
        parts.append(key)
        return .chord(parts.joined(separator: "+"))
    }

    @MainActor
    static func press(_ event: NSEvent) -> Press {
        let flags = event.modifierFlags
        return press(
            keyCode: event.keyCode, characters: event.charactersIgnoringModifiers,
            command: flags.contains(.command), option: flags.contains(.option),
            shift: flags.contains(.shift), control: flags.contains(.control))
    }

    // MARK: SwiftUI

    /// The first key of a command as a SwiftUI shortcut, or nil when it has none or SwiftUI
    /// cannot hold it.
    static func keyboardShortcut(for chord: String) -> KeyboardShortcut? {
        let parts = chord.split(separator: "+", omittingEmptySubsequences: false).map(String.init)
        guard let keyName = parts.last else { return nil }
        var modifiers: EventModifiers = []
        for part in parts.dropLast() {
            switch part {
            case "Mod": modifiers.insert(.command)
            case "Alt": modifiers.insert(.option)
            case "Shift": modifiers.insert(.shift)
            default: return nil
            }
        }
        let key: KeyEquivalent
        switch keyName {
        case "Enter": key = .return
        case "Escape": key = .escape
        case "Tab": key = .tab
        case "Space": key = .space
        case "Backspace": key = .delete
        case "Delete": key = .deleteForward
        case "ArrowUp": key = .upArrow
        case "ArrowDown": key = .downArrow
        case "ArrowLeft": key = .leftArrow
        case "ArrowRight": key = .rightArrow
        case "Home": key = .home
        case "End": key = .end
        case "PageUp": key = .pageUp
        case "PageDown": key = .pageDown
        default:
            if keyName.count == 1, let c = keyName.lowercased().first {
                key = KeyEquivalent(c)
            } else if keyName.hasPrefix("F"), let n = Int(keyName.dropFirst()), (1...24).contains(n),
                      let scalar = Unicode.Scalar(0xF704 + n - 1) {
                key = KeyEquivalent(Character(scalar))
            } else {
                return nil
            }
        }
        return KeyboardShortcut(key, modifiers: modifiers)
    }
}
