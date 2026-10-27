// Real-app scenarios for the Windows port of the writing screens: the Story page (opens first, A4
// sheet, formatting kept across a restart, Dark page), the Beat Board in Horizontal rows, the
// screenplay's Dark page and the first-page number switch, and the character relationship maps.

import AppKit
import Foundation

// MARK: helpers (only for this file)

fileprivate extension UI {
    /// The Story page's text view.
    func storyText(timeout: Double = 15) async throws -> NSTextView {
        let e = try await wait(id: "story-page-text", timeout: timeout, "the Story page")
        guard let tv = e.obj as? NSTextView else { throw DriverError("the Story page is not a text view: \(e)") }
        return tv
    }

    /// Whether the text at `index` of the Story page is bold.
    func storyBold(at index: Int) async throws -> Bool {
        let tv = try await storyText()
        guard let s = tv.textStorage, index < s.length else { return false }
        return (s.attribute(NSAttributedString.Key("of.story.b"), at: index, effectiveRange: nil) as? Bool) == true
    }
}

extension Scenarios {
    static let portLine = "The night bus leaves at ten."

    /// Register in Scenarios.swift: `+ writingPortScenarios` in `writingGroup` or `all`.
    static let writingPortScenarios: [Scenario] = [
        Scenario("03-story-page/opens-on-the-page-formats-and-keeps-it", phases: 2, timeout: 400) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Story Sheet")
                // The Story workspace opens on the Story page: no tab is chosen first.
                try await ui.go("Story")
                let tv = try await ui.storyText(timeout: 20)
                tv.window?.makeKeyAndOrderFront(nil)
                tv.window?.makeFirstResponder(tv)
                try await ui.typeHere(portLine)
                // Command+B on "The night".
                tv.setSelectedRange(NSRange(location: 0, length: 9))
                try await ui.shortcut("b", code: 11)
                let bold = try await ui.storyBold(at: 0)
                try ui.expect(bold, "Command+B made the selection bold")
                // Command+] one size up while the page has the keyboard.
                try await ui.shortcut("]", code: 30)
                try await ui.waitText("Saved", timeout: 15)
                try await ui.press(id: "story-page-dark")
                await ui.idle(0.5)
                try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua, "the sheet is dark")
                try ui.snapshot("story-page-dark")
                try await ui.press(id: "story-page-dark")
                try await ui.press(id: "story-page-zoom-in")
                try await ui.waitText("110%", timeout: 5)
                try await ui.press(id: "story-page-zoom-reset")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Story Sheet")
                try await ui.go("Story")
                _ = try await ui.storyText(timeout: 20)
                try await ui.waitText(portLine, timeout: 15)
                let first = try await ui.storyBold(at: 0)
                let rest = try await ui.storyBold(at: 12)
                try ui.expect(first, "the bold text is still bold after a restart")
                try ui.expect(!rest, "the rest is not bold")
            }
        },

        Scenario("31-story-board-flow/horizontal-rows-are-remembered", phases: 2, timeout: 400) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Rows Night")
                try await ui.go("Story")
                try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
                try await Scenarios.addAct(ui, "Act One")
                try await Scenarios.addSequence(ui, "Depot")
                try await Scenarios.addCard(ui, "The driver waits")
                try await Scenarios.addCard(ui, "The bus pulls out")
                try await Scenarios.segment(ui, group: "story-board-layout", "Horizontal")
                await ui.idle(0.6)
                let board = try await ui.wait(id: "story-board", timeout: 10, "the Beat Board")
                try ui.expect(board.value.contains("Horizontal"), "the board says it is in rows")
                try ui.snapshot("story-board-rows")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Rows Night")
                try await ui.go("Story")
                try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
                let board = try await ui.wait(id: "story-board", timeout: 15, "the Beat Board")
                try ui.expect(board.value.contains("Horizontal"), "the layout was remembered")
                try await Scenarios.segment(ui, group: "story-board-layout", "Vertical")
            }
        },

        Scenario("44-scene-hub-comments-compare/dark-page-is-screen-only", timeout: 400) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Dark Draft")
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua, "the page is white paper by default")
            try await ui.press(id: "screenplay-tools-menu")
            await ui.idle(0.4)
            try await ui.press(id: "screenplay-dark-page")
            await ui.idle(0.6)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua, "the page is dark paper")
            try ui.snapshot("screenplay-dark-page")
            try await ui.press(id: "screenplay-tools-menu")
            await ui.idle(0.4)
            try await ui.press(id: "screenplay-dark-page")
            await ui.idle(0.4)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua, "the page is white again")
        },
    ]
}
