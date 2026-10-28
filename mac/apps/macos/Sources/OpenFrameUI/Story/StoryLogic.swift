// Pure Beat Board helpers: locating items, computing drop targets, keyboard moves, filtering,
// the heading rule used by Build Screenplay and the rows of the Outline. The board data always
// comes from the core; these functions only read it (they decide nothing the core decides).
// Mirrors apps/desktop/src/workspaces/story/model.ts so Mac and Windows move things the same way.

import Foundation
import OpenFrameCore

extension StoryItemRef {
    /// The selection key used on both platforms: "card:<id>".
    var key: String { "\(kind.rawValue):\(id)" }

    init?(key: String) {
        guard let colon = key.firstIndex(of: ":"),
              let kind = StoryItemKind(rawValue: String(key[..<colon])) else { return nil }
        self.init(kind: kind, id: String(key[key.index(after: colon)...]))
    }
}

extension StoryItem {
    var id: String {
        switch self {
        case let .sequence(s): s.id
        case let .beat(b): b.id
        case let .card(c): c.id
        }
    }

    var kind: StoryItemKind {
        switch self {
        case .sequence: .sequence
        case .beat: .beat
        case .card: .card
        }
    }

    var ref: StoryItemRef { StoryItemRef(kind: kind, id: id) }

    /// The text of a card or beat, or the title of a sequence, as shown on the board.
    var shownText: String {
        switch self {
        case let .sequence(s): s.title
        case let .beat(b): b.text
        case let .card(c): c.shortDescription
        }
    }

    var children: [StoryItem] {
        if case let .sequence(s) = self { s.items } else { [] }
    }
}

extension StoryContainerRef {
    static func act(_ id: String) -> StoryContainerRef { StoryContainerRef(parentType: .act, parentId: id) }
    static func sequence(_ id: String) -> StoryContainerRef { StoryContainerRef(parentType: .sequence, parentId: id) }
    static let parking = StoryContainerRef(parentType: .parking, parentId: nil)
    static let unassigned = StoryContainerRef(parentType: .unassigned, parentId: nil)

    var key: String { "\(parentType.rawValue):\(parentId ?? "")" }
}

/// An item with the place it sits in.
struct StoryLocated {
    var item: StoryItem
    var container: StoryContainerRef
    var index: Int
    var siblings: [StoryItem]
}

/// Where a drop would put the dragged items.
struct StoryDropHint: Equatable {
    var target: StoryContainerRef
    /// Insert before this sibling; nil means at the end of the container.
    var before: StoryItemRef?
}

enum StoryDropSide { case before, after }

enum StoryLogic {
    // MARK: Walking the board

    static func sequences(_ board: StoryBoardState) -> [StorySequenceDto] {
        var out: [StorySequenceDto] = []
        func take(_ items: [StoryItem]) {
            for case let .sequence(s) in items { out.append(s) }
        }
        board.acts.forEach { take($0.items) }
        take(board.unassigned)
        return out
    }

    static func children(_ board: StoryBoardState, of c: StoryContainerRef) -> [StoryItem] {
        switch c.parentType {
        case .act: board.acts.first { $0.id == c.parentId }?.items ?? []
        case .sequence: sequences(board).first { $0.id == c.parentId }?.items ?? []
        case .parking: board.parking
        case .unassigned: board.unassigned
        }
    }

    /// Every container with its children: acts, sequences, the Parking Lot, Unassigned.
    static func containers(_ board: StoryBoardState) -> [(container: StoryContainerRef, items: [StoryItem])] {
        var out: [(StoryContainerRef, [StoryItem])] = []
        for a in board.acts { out.append((.act(a.id), a.items)) }
        for s in sequences(board) { out.append((.sequence(s.id), s.items)) }
        out.append((.parking, board.parking))
        out.append((.unassigned, board.unassigned))
        return out
    }

    static func locate(_ board: StoryBoardState, _ ref: StoryItemRef) -> StoryLocated? {
        for (container, items) in containers(board) {
            if let index = items.firstIndex(where: { $0.kind == ref.kind && $0.id == ref.id }) {
                return StoryLocated(item: items[index], container: container, index: index, siblings: items)
            }
        }
        return nil
    }

    /// Every scene card in reading order (acts, then the Parking Lot, then Unassigned).
    static func allCards(_ board: StoryBoardState) -> [StorySceneCardDto] {
        var out: [StorySceneCardDto] = []
        func walk(_ items: [StoryItem]) {
            for item in items {
                switch item {
                case let .card(c): out.append(c)
                case let .sequence(s): walk(s.items)
                case .beat: break
                }
            }
        }
        board.acts.forEach { walk($0.items) }
        walk(board.parking)
        walk(board.unassigned)
        return out
    }

    // MARK: Labels

    private static func firstLine(_ s: String) -> String {
        s.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: "\n", omittingEmptySubsequences: false).first
            .map { $0.trimmingCharacters(in: .whitespaces) } ?? ""
    }

    static func label(_ item: StoryItem) -> String {
        switch item {
        case let .sequence(s): return s.title
        case let .beat(b):
            let f = firstLine(b.text)
            return f.isEmpty ? "Untitled beat" : f
        case let .card(c):
            let f = firstLine(c.shortDescription)
            if !f.isEmpty { return f }
            if let h = c.sceneHeading, !h.isEmpty { return h }
            return "Untitled scene"
        }
    }

    static func containerLabel(_ board: StoryBoardState, _ c: StoryContainerRef) -> String {
        switch c.parentType {
        case .parking: "Parking Lot"
        case .unassigned: "Unassigned"
        case .act: board.acts.first { $0.id == c.parentId }?.title ?? "Act"
        case .sequence: sequences(board).first { $0.id == c.parentId }?.title ?? "Sequence"
        }
    }

    static func plural(_ n: Int, _ word: String) -> String { "\(n) \(word)\(n == 1 ? "" : "s")" }

    static func actCountLabel(_ a: StoryActDto) -> String {
        let scenes = Int(a.cardCount)
        if scenes > 0 { return plural(scenes, "scene") }
        return plural(a.items.count, "item")
    }

    // MARK: Dropping

    /// Can the dragged kinds be dropped into `target`?
    static func canDrop(kinds: Set<StoryItemKind>, into target: StoryContainerRef, moving: Set<String>) -> Bool {
        if kinds.contains(.sequence) { return target.parentType == .act && kinds == [.sequence] }
        if target.parentType == .sequence, let id = target.parentId, moving.contains("sequence:\(id)") { return false }
        return true
    }

    /// Drop next to an item (before or after it) in the item's container.
    static func dropBeside(_ board: StoryBoardState, over: StoryItemRef, placement: StoryDropSide, moving: Set<String>) -> StoryDropHint? {
        guard let loc = locate(board, over) else { return nil }
        let sibs = loc.siblings.filter { !moving.contains($0.ref.key) }
        guard let idx = sibs.firstIndex(where: { $0.kind == over.kind && $0.id == over.id }) else {
            // Hovering one of the moving items: resolve to the next sibling that does not move.
            let after = loc.siblings.dropFirst(loc.index + 1).first { !moving.contains($0.ref.key) }
            return StoryDropHint(target: loc.container, before: after?.ref)
        }
        if placement == .before { return StoryDropHint(target: loc.container, before: over) }
        let next = idx + 1 < sibs.count ? sibs[idx + 1] : nil
        return StoryDropHint(target: loc.container, before: next?.ref)
    }

    /// Drop into a container: at the start (over a sequence header) or at the end.
    static func dropInto(_ board: StoryBoardState, _ c: StoryContainerRef, atStart: Bool, moving: Set<String>) -> StoryDropHint {
        if !atStart { return StoryDropHint(target: c, before: nil) }
        let first = children(board, of: c).first { !moving.contains($0.ref.key) }
        return StoryDropHint(target: c, before: first?.ref)
    }

    /// True when dropping would leave a single item exactly where it is.
    static func isNoop(_ board: StoryBoardState, refs: [StoryItemRef], hint: StoryDropHint) -> Bool {
        guard refs.count == 1, let loc = locate(board, refs[0]), loc.container == hint.target else { return false }
        let next = loc.index + 1 < loc.siblings.count ? loc.siblings[loc.index + 1] : nil
        guard let before = hint.before else { return next == nil }
        return next.map { $0.kind == before.kind && $0.id == before.id } ?? false
    }

    /// Keyboard reorder among siblings: -1 up, +1 down.
    static func stepHint(_ board: StoryBoardState, _ ref: StoryItemRef, direction: Int) -> StoryDropHint? {
        guard let loc = locate(board, ref) else { return nil }
        if direction < 0 {
            guard loc.index > 0 else { return nil }
            return StoryDropHint(target: loc.container, before: loc.siblings[loc.index - 1].ref)
        }
        guard loc.index < loc.siblings.count - 1 else { return nil }
        let afterNext = loc.index + 2 < loc.siblings.count ? loc.siblings[loc.index + 2] : nil
        return StoryDropHint(target: loc.container, before: afterNext?.ref)
    }

    /// Move to the end of the previous or next act (the keyboard alternative to dragging between
    /// act columns). Sequences move between acts too; cards and beats go to the act's own list.
    static func adjacentActHint(_ board: StoryBoardState, _ ref: StoryItemRef, direction: Int) -> StoryDropHint? {
        guard let loc = locate(board, ref) else { return nil }
        let actId: String?
        switch loc.container.parentType {
        case .act: actId = loc.container.parentId
        case .sequence:
            actId = sequences(board).first { $0.id == loc.container.parentId }?.actId
        default: actId = nil
        }
        guard let actId, let i = board.acts.firstIndex(where: { $0.id == actId }) else { return nil }
        let j = i + direction
        guard board.acts.indices.contains(j) else { return nil }
        return StoryDropHint(target: .act(board.acts[j].id), before: nil)
    }

    /// Selected refs in board order (acts, sequences, Parking Lot, Unassigned).
    static func boardOrder(_ board: StoryBoardState, _ refs: [StoryItemRef]) -> [StoryItemRef] {
        var order: [String] = []
        func walk(_ items: [StoryItem]) {
            for item in items {
                order.append(item.ref.key)
                if case let .sequence(s) = item { walk(s.items) }
            }
        }
        board.acts.forEach { walk($0.items) }
        walk(board.parking)
        walk(board.unassigned)
        let rank = Dictionary(order.enumerated().map { ($1, $0) }, uniquingKeysWith: { a, _ in a })
        return refs.sorted { (rank[$0.key] ?? Int.max) < (rank[$1.key] ?? Int.max) }
    }

    // MARK: Navigation

    struct FlatEntry { var ref: StoryItemRef; var actId: String? }

    /// Visible items in reading order, for arrow-key focus movement.
    static func flattenVisible(_ board: StoryBoardState, collapsed: Set<String>, visible: (StoryItem) -> Bool) -> [FlatEntry] {
        var out: [FlatEntry] = []
        func walk(_ items: [StoryItem], _ actId: String?) {
            for item in items where visible(item) {
                out.append(FlatEntry(ref: item.ref, actId: actId))
                if case let .sequence(s) = item, !collapsed.contains(s.id) { walk(s.items, actId) }
            }
        }
        for a in board.acts where !collapsed.contains(a.id) { walk(a.items, a.id) }
        walk(board.parking, "parking")
        walk(board.unassigned, "unassigned")
        return out
    }

    // MARK: Filtering

    static func matchesFilter(_ item: StoryItem, _ query: String) -> Bool {
        let needle = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if needle.isEmpty { return true }
        func has(_ s: String?) -> Bool { (s ?? "").lowercased().contains(needle) }
        switch item {
        case let .card(c): return has(c.shortDescription) || has(c.sceneHeading) || has(c.notes)
        case let .beat(b): return has(b.text) || has(b.note)
        case let .sequence(s): return has(s.title) || s.items.contains { matchesFilter($0, query) }
        }
    }

    // MARK: Build rules

    /// Mirrors Rust `build::heading_is_valid`: a standard location prefix plus a place.
    static func headingIsValid(_ heading: String?) -> Bool {
        let u = (heading ?? "").trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let prefixes = ["INT./EXT.", "EXT./INT.", "INT/EXT", "EXT/INT", "I/E", "INT.", "EXT.", "EST.", "INT ", "EXT ", "EST "]
        for p in prefixes where u.hasPrefix(p) {
            var rest = Substring(u.dropFirst(p.count))
            while let f = rest.first, f == "." || f.isWhitespace { rest = rest.dropFirst() }
            if rest.unicodeScalars.contains(where: { CharacterSet.alphanumerics.contains($0) }) { return true }
        }
        return false
    }

    struct Destination: Identifiable, Hashable {
        var container: StoryContainerRef
        var label: String
        var depth: Int
        var id: String { container.key }
    }

    /// Destination options for the Move dialog.
    static func moveDestinations(_ board: StoryBoardState, kinds: Set<StoryItemKind>) -> [Destination] {
        var out: [Destination] = []
        let onlySequences = !kinds.isEmpty && kinds == [.sequence]
        for a in board.acts {
            out.append(Destination(container: .act(a.id), label: a.title, depth: 0))
            if !onlySequences {
                for case let .sequence(s) in a.items { out.append(Destination(container: .sequence(s.id), label: s.title, depth: 1)) }
            }
        }
        if !kinds.contains(.sequence) { out.append(Destination(container: .parking, label: "Parking Lot", depth: 0)) }
        return out
    }

    /// Where "Add Scene Card" puts a card: after the selected item, into a selected sequence, or
    /// the end of the story.
    static func defaultInsert(_ board: StoryBoardState, selected: [String]) -> (container: StoryContainerRef?, index: Int?) {
        if let key = selected.last, let ref = StoryItemRef(key: key) {
            if ref.kind == .sequence { return (.sequence(ref.id), nil) }
            if let loc = locate(board, ref), loc.container.parentType != .unassigned { return (loc.container, loc.index + 1) }
        }
        if let lastAct = board.acts.last {
            if let lastSeq = lastAct.items.last, case let .sequence(s) = lastSeq { return (.sequence(s.id), nil) }
            return (.act(lastAct.id), nil)
        }
        return (nil, nil)
    }

    // MARK: Label colours

    static let colorNames = ["yellow", "orange", "red", "pink", "purple", "blue", "teal", "green", "gray"]

    /// The theme token for a label colour name.
    static func colorToken(_ name: String) -> String {
        switch name {
        case "orange": "label-orange"
        case "pink": "label-pink"
        case "gray": "label-gray"
        default: name
        }
    }
}

// MARK: Outline rows

/// One row of the Outline tree.
struct StoryOutlineRow: Identifiable, Hashable {
    enum Kind: Hashable {
        case act(String)
        case item(StoryItemRef)
        /// A place at the end of a container where a drop can land (no text).
        case end(StoryContainerRef)
        /// The heading of the Unassigned or Parking Lot section.
        case header(StoryContainerRef)
    }
    var kind: Kind
    var depth: Int
    var prefix: String
    var text: String
    /// The container the row's item sits in (nil for acts).
    var container: StoryContainerRef?
    var id: String {
        switch kind {
        case let .act(id): "act:\(id)"
        case let .item(r): r.key
        case let .end(c): "end:\(c.key)"
        case let .header(c): "header:\(c.key)"
        }
    }
    /// The accessible name of the row, "Scene: Boats fold themselves at dawn".
    var label: String { prefix + (text.isEmpty ? "blank" : text) }
}

extension StoryLogic {
    /// The rows of the Outline in order: acts, their items, then Unassigned and the Parking Lot.
    static func outlineRows(_ board: StoryBoardState, collapsed: Set<String>, filter: String, includeEnds: Bool = false) -> [StoryOutlineRow] {
        var out: [StoryOutlineRow] = []
        func end(_ c: StoryContainerRef, _ depth: Int) {
            if includeEnds { out.append(StoryOutlineRow(kind: .end(c), depth: depth, prefix: "", text: "", container: c)) }
        }
        func rows(_ items: [StoryItem], depth: Int, in container: StoryContainerRef) {
            for item in items where matchesFilter(item, filter) {
                let prefix: String
                switch item {
                case .sequence: prefix = "Sequence: "
                case .beat: prefix = "Beat: "
                case .card: prefix = "Scene: "
                }
                let text: String
                switch item {
                case let .card(c): text = c.shortDescription.isEmpty ? (c.sceneHeading ?? "") : c.shortDescription
                default: text = item.shownText
                }
                out.append(StoryOutlineRow(kind: .item(item.ref), depth: depth, prefix: prefix, text: text, container: container))
                if case let .sequence(s) = item, !collapsed.contains(s.id) {
                    rows(s.items, depth: min(depth + 1, 2), in: .sequence(s.id))
                    end(.sequence(s.id), min(depth + 1, 2))
                }
            }
        }
        for a in board.acts {
            out.append(StoryOutlineRow(kind: .act(a.id), depth: 0, prefix: "Act: ", text: a.title, container: nil))
            if !collapsed.contains(a.id) {
                rows(a.items, depth: 1, in: .act(a.id))
                end(.act(a.id), 1)
            }
        }
        if includeEnds, !board.unassigned.isEmpty {
            out.append(StoryOutlineRow(kind: .header(.unassigned), depth: 0, prefix: "", text: "Unassigned (\(board.unassigned.count))", container: nil))
        }
        rows(board.unassigned, depth: 1, in: .unassigned)
        if includeEnds, !board.unassigned.isEmpty { end(.unassigned, 1) }
        if includeEnds {
            out.append(StoryOutlineRow(kind: .header(.parking), depth: 0, prefix: "", text: "Parking Lot (\(board.parking.count))", container: nil))
        }
        rows(board.parking, depth: 1, in: .parking)
        if includeEnds { end(.parking, 1) }
        return out
    }
}
