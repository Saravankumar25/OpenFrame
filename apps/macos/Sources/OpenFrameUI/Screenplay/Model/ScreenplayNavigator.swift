// The optional columns of the scene navigator (SCRIPT-017), as Windows has them: Page, Notes,
// Revision and Production number. Which columns are shown is kept in the layout record that
// both apps read (`screenplay.view_state`, key "navColumns"). The core counts pages and notes
// (`writing.navigator_columns`); nothing here works a value out.

import Foundation
import OpenFrameCore

enum NavigatorColumn: String, CaseIterable, Sendable {
    case page
    case notes
    case revision
    case production

    /// The name in the Navigator columns menu.
    var title: String {
        switch self {
        case .page: "Page"
        case .notes: "Notes"
        case .revision: "Revision"
        case .production: "Production number"
        }
    }

    /// The short heading above the column.
    var heading: String {
        switch self {
        case .page: "Page"
        case .notes: "Notes"
        case .revision: "Rev."
        case .production: "Prod."
        }
    }

    /// Known columns only, each once, in display order.
    static func normalize(_ raw: [String]) -> [NavigatorColumn] {
        allCases.filter { raw.contains($0.rawValue) }
    }

    /// The columns a saved layout holds; anything unknown is ignored, and an old layout shows none.
    static func from(_ state: JSONValue?) -> [NavigatorColumn] {
        guard case let .object(o)? = state, case let .array(items)? = o["navColumns"] else { return [] }
        return normalize(items.compactMap { if case let .string(s) = $0 { s } else { nil } })
    }

    /// The saved layout with only the columns changed (other keys belong to the mode, the last
    /// screenplay and Windows, and stay).
    static func merged(_ state: JSONValue?, columns: [NavigatorColumn]) -> JSONValue {
        var o: [String: JSONValue] = [:]
        if case let .object(existing)? = state { o = existing }
        o["navColumns"] = .array(columns.map { .string($0.rawValue) })
        return .object(o)
    }

    /// Show a hidden column or hide a shown one.
    static func toggled(_ columns: [NavigatorColumn], _ column: NavigatorColumn) -> [NavigatorColumn] {
        normalize((columns.contains(column) ? columns.filter { $0 != column } : columns + [column]).map(\.rawValue))
    }
}

/// What the core reports for the columns of one draft.
struct NavigatorColumnData: Equatable {
    var rows: [String: NavigatorSceneColumns]
    var hasRevisions: Bool
    var hasProductionNumbers: Bool

    init(_ r: WritingNavigatorColumns) {
        rows = Dictionary(r.rows.map { ($0.sceneId, $0) }, uniquingKeysWith: { a, _ in a })
        hasRevisions = r.hasRevisions
        hasProductionNumbers = r.hasProductionNumbers
    }

    /// The columns that can show something for this draft. Page and Notes always can; until the
    /// data has loaded every column is offered.
    static func available(_ data: NavigatorColumnData?) -> Set<NavigatorColumn> {
        var out: Set<NavigatorColumn> = [.page, .notes]
        if data == nil { return Set(NavigatorColumn.allCases) }
        if data?.hasRevisions == true { out.insert(.revision) }
        if data?.hasProductionNumbers == true { out.insert(.production) }
        return out
    }

    /// The columns to draw: the chosen ones that this draft has data for.
    static func shown(_ chosen: [NavigatorColumn], _ data: NavigatorColumnData?) -> [NavigatorColumn] {
        let available = available(data)
        return chosen.filter { available.contains($0) }
    }

    /// The menu line of a column: a check mark when shown, and why it cannot be chosen.
    static func menuTitle(_ column: NavigatorColumn, chosen: [NavigatorColumn], data: NavigatorColumnData?) -> String {
        let missing = data != nil && !available(data).contains(column)
        let note = missing ? (column == .revision ? " (revision drafts only)" : " (none yet)") : ""
        return column.title + note
    }

    static func isDisabled(_ column: NavigatorColumn, chosen: [NavigatorColumn], data: NavigatorColumnData?) -> Bool {
        data != nil && !available(data).contains(column) && !chosen.contains(column)
    }

    /// One cell: the text shown and the words VoiceOver speaks. Nil for an empty cell, which is
    /// not announced on every row.
    static func cell(_ column: NavigatorColumn, _ c: NavigatorSceneColumns?) -> (value: String, label: String)? {
        guard let c else { return nil }
        switch column {
        case .page:
            guard let page = c.page else { return nil }
            return (String(Int(page)), "Page \(Int(page))")
        case .notes:
            guard c.notes > 0 else { return nil }
            let n = Int(c.notes)
            return (String(n), "\(n) note\(n == 1 ? "" : "s")")
        case .revision:
            guard let r = c.revision else { return nil }
            let word = r.kind == .added ? "New" : r.kind.rawValue
            return (word, "Revision: \(word)")
        case .production:
            guard let p = c.productionNumber, !p.isEmpty else { return nil }
            return (p, "Production number \(p)")
        }
    }
}

extension ScreenplayWorkspaceModel {
    /// The index of a scene's heading among the editor's paragraphs.
    func paragraphIndex(ofScene sceneId: String) -> Int? {
        guard let scenes = document?.scenes, let i = scenes.firstIndex(where: { $0.id == sceneId }) else { return nil }
        return scenes.prefix(i).reduce(0) { $0 + 1 + $1.elements.count }
    }
}
