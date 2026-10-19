// The shared element rule table of the screenplay editor, ported from the Windows editor's
// `rules.ts`, `schema.ts` and `elementRules.json`. The table is data (labels, shortcuts, margins in
// inches, spacing, keep-with-next, line breaks, the element Enter creates next). The app bundles a
// copy of `elementRules.json`; the table is loaded from a path, so Windows and Mac read one format.
//
// Context rules only suggest the next element: the writer can always override with the element menu
// or the element shortcuts.

import Foundation
import OpenFrameCore

/// One row of `elementRules.json`. `kind` is a stored element type or a look (centered, lyric).
public struct ElementRule: Codable, Sendable, Hashable {
    public var kind: String
    public var label: String
    public var shortcut: String?
    /// Left margin of the element, in inches from the page edge.
    public var leftIn: Double
    /// Width of the element, in inches.
    public var widthIn: Double
    public var align: String
    public var uppercase: Bool
    public var italic: Bool
    /// Blank lines before the element.
    public var spaceBefore: Double
    /// Multi-line elements accept Shift+Enter line breaks.
    public var multiline: Bool
    public var nextOnEnter: String
    public var keepWithNext: Bool
    public var splittable: Bool
    public var speech: Bool
    public var printed: Bool
    public var fdxType: String
    public var fountain: String
    public var customLook: Bool
}

public struct ElementRules: Sendable {
    public let rules: [ElementRule]

    public init(rules: [ElementRule]) {
        self.rules = rules
    }

    public init(data: Data) throws {
        self.rules = try JSONDecoder().decode([ElementRule].self, from: data)
    }

    public init(contentsOf url: URL) throws {
        try self.init(data: Data(contentsOf: url))
    }

    /// The shared rule of a kind (stored element type or look).
    public func rule(for kind: String) -> ElementRule? {
        rules.first { $0.kind == kind }
    }

    public func rule(for type: ElementType) -> ElementRule? {
        rule(for: type.rawValue)
    }

    /// The menu label of an element type.
    public func label(for type: ElementType) -> String {
        if type == .custom { return "Custom element" }
        return rule(for: type)?.label ?? type.rawValue
    }

    /// Multi-line elements accept Shift+Enter line breaks. A custom element always does.
    public func isMultiline(_ type: ElementType) -> Bool {
        if type == .custom { return true }
        return rule(for: type)?.multiline ?? false
    }

    /// Enter at the end of a non-empty element: the element that follows, from the rule table
    /// (custom elements name their own next element, see `ScreenplayKeyboard`).
    public func nextOnEnter(_ type: ElementType) -> ElementType {
        let next = rule(for: type)?.nextOnEnter ?? "action"
        if let t = ElementType(rawValue: next), t != .custom, t != .pageBreak { return t }
        return .action
    }

    /// Enter in an empty element converts it instead of adding another blank line. An empty action
    /// becomes a scene heading (Enter twice starts a new scene). Nil means nothing changes.
    public static func onEnterEmpty(_ type: ElementType) -> ElementType? {
        switch type {
        case .sceneHeading, .pageBreak: return nil
        case .action: return .sceneHeading
        case .parenthetical: return .dialogue
        default: return .action
        }
    }

    /// Tab cycles within the dialogue block. Page breaks and custom elements stay as they are (nil):
    /// the writer chose them on purpose.
    public static func tabNext(_ type: ElementType) -> ElementType? {
        switch type {
        case .pageBreak, .custom: return nil
        case .action: return .character
        case .character: return .parenthetical
        case .parenthetical: return .dialogue
        case .dialogue: return .parenthetical
        case .sceneHeading: return .action
        default: return .action
        }
    }

    /// Shift+Tab goes back (nil for page breaks and custom elements).
    public static func tabPrev(_ type: ElementType) -> ElementType? {
        switch type {
        case .pageBreak, .custom: return nil
        case .character: return .action
        case .parenthetical: return .character
        case .dialogue: return .character
        case .action: return .sceneHeading
        default: return .action
        }
    }

    /// The built-in elements in menu order, which is also the order of their shortcuts.
    public static let shortcutOrder: [ElementType] = [
        .sceneHeading, .action, .character, .parenthetical, .dialogue, .transition, .shot,
    ]

    /// Every element type a block can have, in menu order.
    public static let elementTypes: [ElementType] = shortcutOrder + [.note, .pageBreak, .custom]

    /// Types a writer can type text into (not a page break, not a bare custom element).
    public static func isTextType(_ type: ElementType) -> Bool {
        type != .pageBreak && type != .custom
    }
}
