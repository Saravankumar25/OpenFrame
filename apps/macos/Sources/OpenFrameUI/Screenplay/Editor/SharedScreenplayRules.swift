// The shared element rules (elementRules.json, the same file the Windows editor reads) as the
// text view's layout and keyboard rules.

import AppKit
import OpenFrameCore

@MainActor
public final class SharedScreenplayRules: ScreenplayEditorRules {
    public static let shared = SharedScreenplayRules()
    public let table: ElementRules

    public init(table: ElementRules? = nil) {
        if let table {
            self.table = table
        } else if let url = PackagedResources.url("elementRules", "json"),
                  let loaded = try? ElementRules(contentsOf: url) {
            self.table = loaded
        } else {
            assertionFailure("elementRules.json is missing from the OpenFrameUI resources")
            self.table = ElementRules(rules: [])
        }
    }

    /// Element menu order (Command+1 to Command+7), then note.
    public var menuKinds: [ElementType] { ElementRules.shortcutOrder + [.note] }

    public func label(for kind: ElementType) -> String { table.label(for: kind) }

    public func look(for kind: ElementType) -> ElementLook {
        guard let r = table.rule(for: kind) else { return .standard(kind) }
        let align: NSTextAlignment = r.align == "right" ? .right : r.align == "center" ? .center : .left
        return ElementLook(leftInches: r.leftIn, widthInches: r.widthIn, alignment: align, uppercase: r.uppercase,
                           bold: kind == .sceneHeading, italic: r.italic, linesBefore: r.spaceBefore)
    }

    public func isMultiline(_ kind: ElementType) -> Bool { table.isMultiline(kind) }

    public func kindAfterEnter(_ kind: ElementType, text: String) -> ElementType { table.nextOnEnter(kind) }

    public func kindForEmptyEnter(_ kind: ElementType) -> ElementType? { ElementRules.onEnterEmpty(kind) }

    public func kindAfterTab(_ kind: ElementType, text: String, backward: Bool) -> ElementType? {
        backward ? ElementRules.tabPrev(kind) : ElementRules.tabNext(kind)
    }

    public func kind(forShortcutDigit digit: Int) -> ElementType? {
        let order = ElementRules.shortcutOrder
        return digit >= 1 && digit <= order.count ? order[digit - 1] : nil
    }
}
