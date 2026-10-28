// 98-quit: Command+Q works with a sheet open (Mac convention; Windows: the close gate never waits
// on a dialog). Phase 1 types into the screenplay, leaves sheets open and ends with the runner's
// Command+Q, which must quit within 5 seconds with exit code 0. Phase 2 opens the project again:
// no crash question, the typed line is there, and the runner never had to fall back to the menu.

import AppKit
import Foundation

extension Scenarios {
    static let quitGroup: [Scenario] = [
        Scenario("98-quit/command-q-with-project-settings-open", phases: 2) { ui, phase, ctx in
            try await Scenarios.quitWithSheets(ui, phase, ctx, project: "Quiet Exit", line: "Mara locks the shop and walks out.") {
                try await ui.openProjectSettings(page: "Storage")
            }
        },

        Scenario("98-quit/command-q-with-export-sheet-open", phases: 2) { ui, phase, ctx in
            try await Scenarios.quitWithSheets(ui, phase, ctx, project: "Quiet Export", line: "Ines seals the envelope.") {
                try await ui.menu(["File", "Export Screenplay\u{2026}"])
                try await ui.wait(id: "export-confirm", timeout: 20, "the Export Screenplay sheet")
            }
        },

        Scenario("98-quit/command-q-with-print-preview-open", phases: 2) { ui, phase, ctx in
            try await Scenarios.quitWithSheets(ui, phase, ctx, project: "Quiet Print", line: "Tomas folds the last page.") {
                try await ui.menu(["File", "Print\u{2026}"])
                try await ui.wait(id: "print-preview-sheet", timeout: 30, "the Print preview")
            }
        },
    ]

    @MainActor static func quitWithSheets(_ ui: UI, _ phase: Int, _ ctx: Context, project: String, line: String,
                                          openSheets: () async throws -> Void) async throws {
        try await ui.startApp(ctx)
        if phase == 1 {
            try await ui.createProject(project)
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToEnd(tv)
            try await ui.typeHere(line)
            // No wait for Saved: the quit itself must store what the editor holds.
            try await openSheets()
            try ui.expect(NSApp.windows.contains { $0.attachedSheet != nil }, "a sheet is open when Command+Q is pressed")
            try ui.snapshot("sheets-open")
            return
        }
        let out = URL(fileURLWithPath: ctx.env["OPENFRAME_E2E_OUT"] ?? "/nonexistent", isDirectory: true)
        let ignored = FileManager.default.fileExists(atPath: out.appendingPathComponent("cmdq-ignored-1.txt").path)
        try ui.expect(!ignored, "Command+Q quit the app with sheets open (the runner did not fall back to the Quit menu)")
        // The editors' store answered the quit, not the 5 second cap: phase 1's log has the
        // Command+Q and the app's "will terminate" close together.
        let log = (try? String(contentsOf: out.appendingPathComponent("steps-1.log"), encoding: .utf8)) ?? ""
        func time(of marker: String) -> Double? {
            log.split(separator: "\n").first { $0.contains(marker) }
                .flatMap { Double($0.trimmingCharacters(in: .whitespaces).split(separator: " ").first ?? "") }
        }
        let asked = time(of: "quit with Command+Q"), ended = time(of: "the app reports it will terminate")
        try ui.expect(asked != nil && ended != nil, "phase 1 logged the quit and its end")
        if let asked, let ended {
            try ui.expect(ended - asked < 3, "the app quit \(String(format: "%.2f", ended - asked)) s after Command+Q (under 3 s, so the editors' store answered and not the 5 s cap)")
        }
        try await ui.waitHome()
        try await ui.press(id: "project-card-\(project)")
        try await ui.wait(id: "project-home", timeout: 30, "Project Home of \(project)")
        try ui.expect(!(await ui.recoveryOffered(within: 3)), "the project closed cleanly on quit (no crash question)")
        try await ui.go("Screenplay")
        _ = try await ui.editor()
        let deadline = Date().addingTimeInterval(15)
        var text = ""
        while Date() < deadline {
            text = (ui.query(id: "screenplay-editor").first?.obj as? NSTextView)?.string ?? ""
            if text.contains(line) { break }
            await ui.idle(0.2)
        }
        try ui.expect(text.contains(line), "the line typed before quitting was stored: \(text.prefix(200))")
    }
}
