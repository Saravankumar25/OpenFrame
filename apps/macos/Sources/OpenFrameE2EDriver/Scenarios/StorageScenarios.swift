// 71-storage (Windows: tests/e2e/specs/71-storage.test.mjs, doc 40): the library setup screen on
// a new Mac, Settings > Storage naming every place with its actions, and a project moved while the
// app runs that shows again from its new folder.
//
// The first-start cases need the app started like a new Mac: the runner sets
// OPENFRAME_LIBRARY_SETUP=1 for scenarios whose name starts with "71-storage/first-start" (debug
// build on a test data folder only; the core ignores it otherwise).

import AppKit
import Foundation

extension Scenarios {
    static let storageGroup: [Scenario] = [
        Scenario("71-storage/first-start-setup-then-straight-home", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                try await ui.passTeamGate(ctx)
                try await ui.wait(id: "library-setup", timeout: 30, "the library setup screen (run with OPENFRAME_LIBRARY_SETUP=1)")
                try ui.expect(!ui.exists(id: "home-new-project"), "Home does not show before a library is chosen")
                try await ui.waitText("Set up your OpenFrame library")
                let suggested = try await ui.wait(id: "library-setup-suggested", timeout: 10, "the suggested folder")
                try ui.expect(suggested.text.hasSuffix("/OpenFrame"), "the suggested library is a folder named OpenFrame: \(suggested.text)")
                try await ui.waitText("iCloud Drive does not sync it with Desktop & Documents")
                try ui.expect(!FileManager.default.fileExists(atPath: suggested.text), "nothing is created before a choice")
                try ui.snapshot("library-setup")
                try await ui.press(id: "library-setup-create-default", role: "AXButton")
                try await ui.answerWelcome()
                try await ui.waitHome()
                try ui.expect(FileManager.default.fileExists(atPath: suggested.text), "Create Library Here made the library")
            } else {
                // The library is remembered: no setup screen on the next start.
                try await ui.passTeamGate(ctx)
                let deadline = Date().addingTimeInterval(30)
                while Date() < deadline && !ui.exists(id: "home-new-project") && !ui.exists(id: "home-empty") {
                    try ui.expect(!ui.exists(id: "library-setup"), "the setup screen showed again after the library was set up")
                    await ui.idle(0.2)
                }
                try await ui.answerWelcome()
                try await ui.waitHome()
            }
        },

        Scenario("71-storage/first-start-locate-refuses-an-empty-folder") { ui, _, ctx in
            try await ui.passTeamGate(ctx)
            try await ui.wait(id: "library-setup", timeout: 30, "the library setup screen (run with OPENFRAME_LIBRARY_SETUP=1)")
            let empty = ctx.work.appendingPathComponent("empty-\(UUID().uuidString)", isDirectory: true)
            try FileManager.default.createDirectory(at: empty, withIntermediateDirectories: true)
            try await Scenarios.segment(ui, group: "library-setup-choice", "Locate existing OpenFrame data")
            try await ui.choosing(empty) { try await ui.press(id: "library-setup-choose-existing", role: "AXButton") }
            try await ui.wait(id: "library-setup-preview", timeout: 15, "what the core found in the folder")
            try ui.expect(!ui.exists(id: "library-setup-connect"), "an empty folder offers no Connect Library")
            try ui.expect(ui.exists(id: "library-setup"), "a refused folder stays on the setup screen")
            try ui.snapshot("library-setup-refused")
        },

        Scenario("71-storage/settings-storage-names-every-place") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Ledger")
            try await ui.openProjectSettings(page: "Storage")
            for id in ["storage-library", "storage-projects", "storage-vault", "storage-project", "storage-app-data"] {
                try await ui.wait(id: id, timeout: 15, "the \(id) section of Settings > Storage")
            }
            for id in ["storage-library-open", "storage-library-move", "storage-library-locate", "storage-projects-change",
                       "storage-vault-restore", "storage-vault-locate", "storage-project-open", "storage-project-move", "storage-app-data-open"] {
                try ui.expect(ui.exists(id: id), "Settings > Storage has \(id)")
            }
            try await ui.waitText("Changing it does not move the projects you already have.")
            try ui.snapshot("settings-storage")
            try await ui.closeStorageSettings()
        },

        Scenario("71-storage/move-project-shows-it-again", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            let dest = ctx.dataRoot.appendingPathComponent("moved-projects", isDirectory: true)
            if phase == 1 {
                try FileManager.default.createDirectory(at: dest, withIntermediateDirectories: true)
                try await ui.createProject("Moving Night")
                try await ui.openProjectSettings(page: "Storage")
                try await ui.wait(id: "storage-project-move", timeout: 15, "Move Project")
                try await ui.choosing(dest) { try await ui.press(id: "storage-project-move", role: "AXButton") }
                try await ui.wait(id: "storage-task-confirm", timeout: 10, "the move confirmation")
                try await ui.waitText("Linked files outside the project are not moved.")
                try await ui.press(id: "storage-task-confirm", role: "AXButton")
                try await ui.wait(id: "storage-task-done", timeout: 120, "the move result")
                try await ui.waitText("The project is now at \(dest.path)/")
                try ui.snapshot("project-moved")
                try await ui.press(id: "storage-task-close", role: "AXButton")
                try ui.expect(ui.exists(id: "project-settings-sheet"), "the project stays open while it moves")
                let path = try await ui.wait(id: "storage-project-path", timeout: 15, "the project folder")
                try ui.expect(path.value.hasPrefix(dest.path), "Settings shows the new folder: \(path.value)")
                try await ui.closeStorageSettings()
                try await ui.wait(id: "project-home", timeout: 15, "Project Home after the move")
                try await ui.closeProject()
            } else {
                // The moved project opens from Recent Projects at its new place.
                try await ui.openProject("Moving Night")
                try await ui.openProjectSettings(page: "Storage")
                let path = try await ui.wait(id: "storage-project-path", timeout: 15, "the project folder")
                try ui.expect(path.value.hasPrefix(dest.path), "the project opened from its new folder: \(path.value)")
                try await ui.closeStorageSettings()
            }
        },
    ]
}

extension UI {
    /// Done on Project Settings. macOS does not quit while a sheet is attached to the window, so a
    /// scenario closes the sheet as a person would before the app quits.
    func closeStorageSettings() async throws {
        try await press(id: "project-settings-done", role: "AXButton")
        try await waitGone(id: "project-settings-sheet", timeout: 10)
    }

    /// The team password of an internal build, without waiting for Home (the library setup
    /// screen comes after it).
    func passTeamGate(_ ctx: Context) async throws {
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            if exists(id: "team-password") {
                try expect(!ctx.teamPassword.isEmpty, "the team gate shows but OPENFRAME_E2E_TEAM_PASSWORD is empty")
                try await type(ctx.teamPassword, into: "team-password")
                try await press(id: "team-gate-continue", role: "AXButton")
                try await waitGone(id: "team-password", timeout: 30)
                return
            }
            if exists(id: "library-setup") || exists(id: "home-new-project") || exists(id: "home-empty") || exists(id: "welcome-name") { return }
            if exists(id: "startup-failure") { throw DriverError("the app could not start: \(screenText())") }
            await idle(0.2)
        }
    }
}
