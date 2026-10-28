// The gold reference project "Low Tide Radio" (tests/gold-project) in the real app: the full
// project package built by `cargo run -p openframe-gold-project` is restored through
// File > Restore Backup or Package, then every workspace is opened and checked for facts of
// the film, with a picture of each.

import AppKit
import Foundation

fileprivate extension UI {
    /// The first of `texts` that shows on screen within the timeout, or nil.
    func seeAny(_ texts: [String], timeout: Double = 20) async -> String? {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            let screen = screenText()
            if let hit = texts.first(where: { screen.contains($0) }) { return hit }
            await idle(0.3)
        }
        return nil
    }

    /// Opens a workspace, checks that one of the facts shows, and takes a picture.
    func checkWorkspace(_ name: String, _ facts: [String], _ f: Findings, timeout: Double = 20) async throws {
        try await go(name)
        // Story opens on the Story page; the facts checked here are on the Beat Board.
        if name == "Story" { try await Scenarios.segment(self, group: "story-tabs", "Beat Board") }
        let hit = await seeAny(facts, timeout: timeout)
        f.check(hit != nil, "\(name): none of \(facts) is on screen; it shows \(screenText().prefix(300))")
        log("\(name): saw \(hit ?? "nothing")")
        try snapshot("gold-\(Self.workspaceKey(name))")
        windowPicture("gold-\(Self.workspaceKey(name))")
    }

    /// A second picture through the window server: some workspaces draw their content in layers
    /// that the view cache used by `snapshot` leaves transparent. Needs Screen Recording access;
    /// without it the system returns nothing and only the first picture exists.
    func windowPicture(_ name: String) {
        guard let w = NSApp.windows.first(where: { $0.isVisible && $0.title.contains("Low Tide Radio") }),
              let image = CGWindowListCreateImage(.null, .optionIncludingWindow, CGWindowID(w.windowNumber), [.boundsIgnoreFraming, .bestResolution])
        else { return }
        let rep = NSBitmapImageRep(cgImage: image)
        if let png = rep.representation(using: .png, properties: [:]) {
            try? png.write(to: artifacts.appendingPathComponent("\(name)-window.png"))
        }
    }
}

extension Scenarios {
    static let goldGroup: [Scenario] = [
        Scenario("gold/open-every-workspace", timeout: 900) { ui, _, ctx in
            let f = Findings()
            let built = ctx.fixtures.deletingLastPathComponent()
                .appendingPathComponent("gold-project/outputs/packages/Low Tide Radio.ofproject")
            try ui.expect(FileManager.default.fileExists(atPath: built.path),
                          "the gold package is missing; run cargo run -p openframe-gold-project first: \(built.path)")
            // The package travels with the run, as a file the person received would.
            let incoming = ctx.dataRoot.appendingPathComponent("Incoming", isDirectory: true)
            try FileManager.default.createDirectory(at: incoming, withIntermediateDirectories: true)
            let pkg = incoming.appendingPathComponent("Low Tide Radio.ofproject")
            try? FileManager.default.removeItem(at: pkg)
            try FileManager.default.copyItem(at: built, to: pkg)

            try await ui.startApp(ctx)
            try await ui.menu(["File", "Restore Backup or Package…"])
            try await ui.wait(id: "open-project-choose", timeout: 20, "the Restore Backup or Package sheet")
            try await ui.choosing(pkg) { try await ui.press(id: "open-project-choose") }
            try await ui.wait(id: "open-project-no-collision", timeout: 60, "the package check (no project with this identity yet)")
            let facts = ui.query(id: "open-project-fact-project").map(\.text).joined()
            f.check(facts.contains("Low Tide Radio"), "the package names the project Low Tide Radio: \(facts)")
            try ui.snapshot("gold-package-checked")
            try await ui.press(id: "open-project-confirm")
            try await ui.wait(id: "open-project-done", timeout: 180, "Project opened.")
            try await ui.press(id: "open-project-close")
            try await ui.wait(id: "project-home", timeout: 30, "Project Home of Low Tide Radio")

            try await ui.checkWorkspace("Project Home", ["Low Tide Radio"], f)
            let home = ui.screenText()
            // Project Home as on Windows: Continue, Quick Access and Recent, no counts.
            f.check(home.lowercased().contains("quick access"), "Project Home shows Quick Access")

            // Screenplay: the production draft with its first scene, a character and the 1999 radio voice.
            try await ui.go("Screenplay")
            let tv = try await ui.editor(timeout: 60)
            let script = tv.string
            for needle in ["EXT. OPEN SEA - NIGHT (1999)", "ANJALI", "Kadalkkara to all boats", "EXT. KADALKKARA JETTY - NIGHT"] {
                f.check(script.contains(needle), "Screenplay: the editor shows \"\(needle)\"")
            }
            f.check(ui.screenText().contains("Blue Revision"), "Screenplay: the open draft is the Blue Revision")
            ui.log("Screenplay: \(script.count) characters in the editor")
            try ui.snapshot("gold-screenplay")

            try await ui.checkWorkspace("Story", ["Act One: The Call", "Prologue. A storm.", "Static"], f)
            try await ui.checkWorkspace("Idea Vault", ["Treatment: Low Tide Radio (v3)", "The night of 14 November 1999"], f)
            try await ui.checkWorkspace("Breakdown", ["EXT. HARBOUR BREAKWATER - NIGHT", "COAST RADIO STATION", "Quarter-inch reel 14-11-1999"], f)
            try await ui.checkWorkspace("Schedule", ["Shoot Day 1", "Shoot Day 14"], f)
            try await ui.checkWorkspace("Call Sheets", ["Shoot Day 1", "LOW TIDE RADIO", "Kayal Pictures"], f)
            // The plan is 4,60,00,000 rupees with 10 percent contingency; the lines add up to 3,78,75,000.
            let totals = ["4,60,00,000", "46,000,000", "3,78,75,000", "37,875,000"]
            try await ui.checkWorkspace("Budget", totals + ["Story rights"], f)
            let budget = ui.screenText()
            f.check(totals.contains(where: { budget.contains($0) }),
                    "Budget: a total of the film's budget is on screen")
            try await ui.checkWorkspace("Visual Planning", ["Look of the film", "listening night", "1999 operations room", "breakwater"], f)
            try await ui.checkWorkspace("Files", ["Location release, Coast Radio Station (signed).pdf", "Contracts, releases and permits"], f)
            try await ui.checkWorkspace("Notes and Tasks", ["Director's statement", "Sound philosophy"], f)
            try await ui.checkWorkspace("Activity", ["opened from project package", "Meenakshi Varma", "Arun Joseph commented"], f)
            try await ui.checkWorkspace("Recently Deleted", ["Costume (moved to the wardrobe file)", "Breakwater (old thumbnails)", "Rough one-liner"], f)
            try f.finish()
        },
    ]
}
