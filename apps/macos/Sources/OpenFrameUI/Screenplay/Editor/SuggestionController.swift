// The state of the suggestion list while the writer types (Windows `suggestPlugin.ts`): what is
// being typed, the items, the highlighted one, and whether the writer closed the list. Pure
// logic with no view, so it can be tested without a window. The text view feeds it the caret's
// element after every change and draws the list from it.
//
// Nothing is inserted unless the writer accepts an item. Typing brings a closed list back; moving
// the caret around the same word does not.

import Foundation
import OpenFrameCore

@MainActor
final class ScreenplaySuggestionController {
    private(set) var context: SuggestContext?
    private(set) var blockId: String?
    private(set) var items: [SuggestItem] = []
    /// Highlighted item; -1 = none (Enter then keeps its normal meaning).
    private(set) var index = -1
    private(set) var dismissed = false

    private var cache: (blockId: String?, values: ScriptValues)?

    var isOpen: Bool { context != nil && !dismissed && !items.isEmpty }

    /// The script values are kept while the writer stays in the same element; leaving it, a line
    /// break, a paste or a refresh reads them again.
    func invalidateValues() { cache = nil }

    /// Recomputes the list. Returns true when anything the person can see changed.
    @discardableResult
    func update(
        kind: ElementType, text: String, offset: Int, blockId: String?, selectionIsEmpty: Bool, canEdit: Bool,
        sources: WritingSuggestionSources?, textChanged: Bool, values: () -> ScriptValues
    ) -> Bool {
        let before = visibleState
        let previous = (context, self.blockId, index, dismissed)
        guard selectionIsEmpty, canEdit, let c = ScreenplaySuggest.contextAt(kind, text, offset) else {
            close(keepDismissed: false)
            return before != visibleState
        }
        let scriptValues: ScriptValues
        if let hit = cache, hit.blockId == blockId, blockId != nil {
            scriptValues = hit.values
        } else {
            scriptValues = values()
            cache = (blockId, scriptValues)
        }
        let found = ScreenplaySuggest.suggestions(c, script: scriptValues, project: sources, canCreate: true)
        if found.isEmpty {
            let keep = !textChanged && previous.3
            close(keepDismissed: keep)
            return before != visibleState
        }
        let same = previous.0.map { $0.kind == c.kind && $0.query == c.query } == true && previous.1 == blockId
        let wasDismissed = textChanged ? false : (same ? previous.3 : false)
        var next = c.query.trimmingCharacters(in: .whitespaces).isEmpty ? -1 : 0
        if same, previous.2 < found.count { next = previous.2 }
        context = c
        self.blockId = blockId
        items = found
        index = next
        dismissed = wasDismissed
        return before != visibleState
    }

    private func close(keepDismissed: Bool) {
        context = nil
        blockId = nil
        items = []
        index = -1
        dismissed = keepDismissed
    }

    /// Everything the list shows: used to tell whether a redraw is needed.
    private var visibleState: [String] {
        guard isOpen else { return [] }
        return items.map(\.label) + ["#\(index)"]
    }

    /// Arrow keys: true when the list took the key. With nothing highlighted the arrows keep
    /// moving the caret.
    func move(by step: Int) -> Bool {
        guard isOpen, index >= 0 else { return false }
        let n = items.count
        index = ((index + step) % n + n) % n
        return true
    }

    /// Escape: true when a list was open and is now closed.
    func dismiss() -> Bool {
        guard isOpen else { return false }
        dismissed = true
        return true
    }

    func select(_ i: Int) {
        if items.indices.contains(i) { index = i }
    }

    var highlighted: SuggestItem? { items.indices.contains(index) ? items[index] : nil }
}
