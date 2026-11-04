// File menu sheets over an open Project Settings sheet, and the print preview's PDF in the
// accessibility tree. On the Mac a sheet may carry its own sheet, so File > Export Screenplay...
// over Project Settings shows the export sheet on top of it; the command never does nothing.
// Cancel returns to Project Settings, and the runner's Command+Q then quits cleanly.

import AppKit
import Foundation

extension Scenarios {
    static let interchangeFixGroup: [Scenario] = [
        Scenario("63-every-export/sheet-fix-export-over-project-settings") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Over Settings")
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToEnd(tv)
            try await ui.typeHere("Noor counts the reels twice.")
            try await ui.openProjectSettings(page: "Writing")
            try await ui.wait(id: "project-settings-sheet", timeout: 10, "the Project Settings sheet")
            try await ui.menu(["File", "Export Screenplay\u{2026}"])
            try await ui.wait(id: "export-confirm", timeout: 20, "the Export Screenplay sheet over Project Settings")
            let export = NSApp.windows.first { w in w.isVisible && w.isSheet && w.sheetParent?.isSheet == true }
            try ui.expect(export != nil, "the export sheet is attached to the Project Settings sheet")
            try ui.expect(!NSApp.windows.contains { !$0.isVisible && $0.isSheet && $0.sheetParent != nil },
                          "no sheet waits hidden in a queue")
            try ui.snapshot("export-over-settings")
            try await ui.press(id: "export-cancel")
            try await ui.waitGone(id: "export-confirm", timeout: 10)
            try ui.expect(ui.exists(id: "project-settings-sheet"), "Project Settings is still open after Cancel")
            try await Scenarios.closeSettings(ui)
        },

        Scenario("63-every-export/sheet-fix-print-preview-pdf-reachable") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Readable Preview")
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToEnd(tv)
            try await ui.typeHere("Lena reads the last page aloud.")
            try await ui.menu(["File", "Print\u{2026}"])
            try await ui.wait(id: "print-preview-sheet", timeout: 30, "the Print preview")
            _ = try await ui.wait(id: "print-preview-pdf", timeout: 30, "the PDF in the Print preview")
            let found = ui.query(id: "print-preview-pdf")
            try ui.expect(found.contains { $0.label.hasPrefix("Print preview.") },
                          "the PDF is labelled for VoiceOver: \(found.map(\.description))")
            try ui.expect(found.contains { !$0.children.isEmpty }, "the PDF's own element is inside the container")
            try ui.snapshot("print-preview")
            try await ui.press(id: "print-close")
            try await ui.waitGone(id: "print-preview-sheet", timeout: 10)
        },
    ]
}
