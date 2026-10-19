// Keyboard and element commands of the screenplay editor, ported from the Windows editor's
// `commands.ts` and `rules.ts`: element type changes, custom elements, explicit page breaks and
// Enter, Tab and Shift+Enter. Only the pure rules are here, over the editor's block list; the text
// view applies the result.
//
// Page breaks are blocks without text: typing commands never turn one into a text element, and Tab
// leaves it alone. Custom elements keep their definition when split, and Enter at their end creates
// their own next element.
//
// Offsets are UTF-16 units, the unit a text view uses.

import Foundation
import OpenFrameCore

/// A caret in the block list.
public struct EditorCaret: Hashable, Sendable {
    public var block: Int
    /// UTF-16 offset inside the block text.
    public var offset: Int

    public init(block: Int, offset: Int) {
        self.block = block
        self.offset = offset
    }
}

/// The blocks, the caret and the last block of the selection (a range of blocks for element changes).
public struct ScreenplayEditorState: Hashable, Sendable {
    public var blocks: [EditorBlock]
    public var caret: EditorCaret
    /// The last block of the selection; nil when nothing beyond the caret is selected.
    public var selectionEndBlock: Int?

    public init(blocks: [EditorBlock], caret: EditorCaret, selectionEndBlock: Int? = nil) {
        self.blocks = blocks
        self.caret = caret
        self.selectionEndBlock = selectionEndBlock
    }

    public var kinds: [ElementType] { blocks.map(\.kind) }
    public var caretBlock: EditorBlock { blocks[caret.block] }
}

/// What a command did. `handled` is false when the key should fall through; `failure` carries a
/// message to show when the command refused.
public struct EditorCommandResult: Sendable {
    public var handled: Bool
    public var state: ScreenplayEditorState
    public var failure: String?
}

public struct ScreenplayKeyboard: Sendable {
    public static let firstSceneNeedsHeading = "The first scene needs a scene heading."
    public static let pageBreakHasNoText = "A page break has no text to change."

    public var rules: ElementRules
    /// Custom element definitions of the open document.
    public var customDefs: [ScreenplayCustomElementDto]
    public var newId: @Sendable () -> String

    public init(
        rules: ElementRules,
        customDefs: [ScreenplayCustomElementDto] = [],
        newId: @escaping @Sendable () -> String = { UUID().uuidString.lowercased() }
    ) {
        self.rules = rules
        self.customDefs = customDefs
        self.newId = newId
    }

    public func customDef(_ id: String?) -> ScreenplayCustomElementDto? {
        guard let id else { return nil }
        return customDefs.first { $0.id == id }
    }

    public func customDef(forShortcut shortcut: String) -> ScreenplayCustomElementDto? {
        customDefs.first { $0.shortcut == shortcut }
    }

    private func handled(_ s: ScreenplayEditorState) -> EditorCommandResult {
        EditorCommandResult(handled: true, state: s, failure: nil)
    }

    private func unhandled(_ s: ScreenplayEditorState, _ failure: String? = nil) -> EditorCommandResult {
        EditorCommandResult(handled: false, state: s, failure: failure)
    }

    /// The block Enter creates after a custom element (its own next element).
    public func blockAfterCustom(_ block: EditorBlock) -> EditorBlock {
        let next = customDef(block.customElementId)?.nextElement ?? "action"
        if next.hasPrefix("custom:") {
            let id = String(next.dropFirst("custom:".count))
            if customDef(id) != nil { return EditorBlock(id: newId(), kind: .custom, customElementId: id) }
            return EditorBlock(id: newId(), kind: .action)
        }
        if let t = ElementType(rawValue: next), ElementRules.isTextType(t) {
            return EditorBlock(id: newId(), kind: t)
        }
        return EditorBlock(id: newId(), kind: .action)
    }

    private func convertedText(_ text: String, to kind: ElementType) -> String {
        rules.isMultiline(kind) ? text : text.replacingOccurrences(of: "\n", with: " ")
    }

    // MARK: Element changes

    /// Changes the element type of the blocks in the selection. Changing between a heading and a
    /// body element changes identity (it splits or merges scenes). Page breaks in the selection stay
    /// page breaks; custom elements have their own command (`setCustom`).
    public func setType(_ to: ElementType, in state: ScreenplayEditorState) -> EditorCommandResult {
        guard ElementRules.isTextType(to) else { return unhandled(state) }
        let first = state.caret.block
        let last = max(first, state.selectionEndBlock ?? first)
        var s = state
        var changed = false
        var onlyBreaks = true
        for i in first...last {
            let node = s.blocks[i]
            if node.kind == .pageBreak { continue }
            onlyBreaks = false
            if node.kind == to { continue }
            if i == 0 && to != .sceneHeading {
                return unhandled(state, Self.firstSceneNeedsHeading)
            }
            let crosses = (node.kind == .sceneHeading) != (to == .sceneHeading)
            s.blocks[i] = EditorBlock(id: crosses ? newId() : node.id, kind: to, text: convertedText(node.text, to: to))
            changed = true
        }
        if onlyBreaks { return unhandled(state, Self.pageBreakHasNoText) }
        if !changed { return unhandled(state) }
        s.caret.offset = min(s.caret.offset, s.blocks[s.caret.block].text.utf16.count)
        return handled(s)
    }

    /// Turns the selected blocks into a custom element (never the first heading or a page break).
    public func setCustom(_ customId: String, in state: ScreenplayEditorState) -> EditorCommandResult {
        guard customDef(customId) != nil else { return unhandled(state) }
        let first = state.caret.block
        let last = max(first, state.selectionEndBlock ?? first)
        var s = state
        var changed = false
        for i in first...last {
            let node = s.blocks[i]
            if node.kind == .custom && node.customElementId == customId { continue }
            if i == 0 || node.kind == .pageBreak {
                return unhandled(state, i == 0 ? Self.firstSceneNeedsHeading : Self.pageBreakHasNoText)
            }
            let crosses = node.kind == .sceneHeading
            s.blocks[i] = EditorBlock(id: crosses ? newId() : node.id, kind: .custom, text: node.text, customElementId: customId)
            changed = true
        }
        if !changed { return unhandled(state) }
        s.caret.offset = min(s.caret.offset, s.blocks[s.caret.block].text.utf16.count)
        return handled(s)
    }

    /// Tab and Shift+Tab: the next element of the dialogue cycle. Page breaks and custom elements
    /// are left as they are; the element menu and shortcuts still change them. Tab always counts as
    /// handled: it never moves focus out of the page. A refusal (the first heading) is in `failure`.
    public func tab(forward: Bool, in state: ScreenplayEditorState) -> EditorCommandResult {
        let kind = state.caretBlock.kind
        let to = forward ? ElementRules.tabNext(kind) : ElementRules.tabPrev(kind)
        guard let to else { return handled(state) }
        let r = setType(to, in: state)
        return EditorCommandResult(handled: true, state: r.state, failure: r.failure)
    }

    // MARK: Enter

    /// Enter with a collapsed caret.
    public func enter(in state: ScreenplayEditorState) -> EditorCommandResult {
        var s = state
        let idx = state.caret.block
        let node = state.blocks[idx]
        let kind = node.kind

        // Enter on a selected page break adds an action after it and keeps the break.
        if kind == .pageBreak {
            s.blocks.insert(EditorBlock(id: newId(), kind: .action), at: idx + 1)
            s.caret = EditorCaret(block: idx + 1, offset: 0)
            return handled(s)
        }

        let ns = node.text as NSString
        let length = ns.length
        if length == 0 {
            guard let to = ElementRules.onEnterEmpty(kind) else { return handled(state) }
            let crosses = (kind == .sceneHeading) != (to == .sceneHeading)
            s.blocks[idx] = EditorBlock(id: crosses ? newId() : node.id, kind: to)
            return handled(s)
        }

        let offset = min(max(state.caret.offset, 0), length)
        if offset == length {
            // The element that follows is already the empty one Enter would create: go into it
            // instead of stacking another empty element in front of it.
            if kind != .custom, idx + 1 < s.blocks.count {
                let following = s.blocks[idx + 1]
                if following.text.isEmpty && following.kind == rules.nextOnEnter(kind) {
                    s.caret = EditorCaret(block: idx + 1, offset: 0)
                    return handled(s)
                }
            }
            let next = kind == .custom ? blockAfterCustom(node) : EditorBlock(id: newId(), kind: rules.nextOnEnter(kind))
            s.blocks.insert(next, at: idx + 1)
            s.caret = EditorCaret(block: idx + 1, offset: 0)
        } else if offset == 0 {
            // Keep the element (and its identity) with its text; add a blank one above.
            let blank = EditorBlock(id: newId(), kind: kind, customElementId: node.customElementId)
            s.blocks.insert(blank, at: idx)
            s.caret = EditorCaret(block: idx + 1, offset: 0)
        } else {
            // Split in two; the part after the caret is new, a heading continues as action.
            let afterKind: ElementType = kind == .sceneHeading ? .action : kind
            s.blocks[idx].text = ns.substring(to: offset)
            let tail = EditorBlock(
                id: newId(), kind: afterKind, text: ns.substring(from: offset),
                customElementId: kind == .custom ? node.customElementId : nil)
            s.blocks.insert(tail, at: idx + 1)
            s.caret = EditorCaret(block: idx + 1, offset: 0)
        }
        return handled(s)
    }

    /// Shift+Enter: a line break inside a multi-line element, otherwise the same as Enter.
    public func lineBreak(in state: ScreenplayEditorState) -> EditorCommandResult {
        let node = state.caretBlock
        guard rules.isMultiline(node.kind), node.kind != .pageBreak else { return enter(in: state) }
        var s = state
        let ns = node.text as NSString
        let offset = min(max(state.caret.offset, 0), ns.length)
        s.blocks[state.caret.block].text = ns.replacingCharacters(in: NSRange(location: offset, length: 0), with: "\n")
        s.caret.offset = offset + 1
        return handled(s)
    }

    /// Insert page break (Cmd+Return by default): an explicit page break after the current block,
    /// then a new block of the same kind (an action after a heading or a page break).
    public func insertPageBreak(in state: ScreenplayEditorState) -> EditorCommandResult {
        var s = state
        let idx = state.caret.block
        let node = state.blocks[idx]
        let follow: EditorBlock
        if node.kind == .sceneHeading || node.kind == .pageBreak {
            follow = EditorBlock(id: newId(), kind: .action)
        } else {
            follow = EditorBlock(id: newId(), kind: node.kind, customElementId: node.customElementId)
        }
        s.blocks.insert(contentsOf: [EditorBlock(id: newId(), kind: .pageBreak), follow], at: idx + 1)
        s.caret = EditorCaret(block: idx + 2, offset: 0)
        return handled(s)
    }
}
