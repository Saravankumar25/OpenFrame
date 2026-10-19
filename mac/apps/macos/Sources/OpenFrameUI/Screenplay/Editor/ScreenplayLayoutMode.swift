// The three ways to lay out the screenplay workspace (Mac phase M4), as Windows names them:
// Standard, Writing Room (the script with a side pane for notes, comments and drafts) and Focus
// (only the page). The choice is kept in the same layout record the Windows app reads
// (`screenplay.view_state`, key "mode"), so a project opens the way the writer left it.

import Foundation
import OpenFrameCore

enum ScreenplayMode: String, CaseIterable, Sendable {
    case standard
    case writingRoom
    case focus

    var title: String {
        switch self {
        case .standard: "Standard Mode"
        case .writingRoom: "Writing Room Mode"
        case .focus: "Focus Mode"
        }
    }

    /// The mode a saved layout holds; anything unknown is Standard.
    static func from(_ state: JSONValue?) -> ScreenplayMode {
        if case let .object(o)? = state, case let .string(s)? = o["mode"], let mode = ScreenplayMode(rawValue: s) { return mode }
        return .standard
    }

    /// The saved layout with only the mode changed (other keys belong to Windows and stay).
    static func merged(_ state: JSONValue?, mode: ScreenplayMode) -> JSONValue {
        var o: [String: JSONValue] = [:]
        if case let .object(existing)? = state { o = existing }
        o["mode"] = .string(mode.rawValue)
        return .object(o)
    }
}
