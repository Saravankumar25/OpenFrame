// Steps that many scenarios share: the team gate, creating and opening a project, moving between
// workspaces and typing in the screenplay. Each is written the way a person does it.

import AppKit

extension UI {
    /// Waits for the first screen. On an internal build that is the team gate: type the team
    /// password and continue. Ends on Application Home.
    func startApp(_ ctx: Context) async throws {
        let deadline = Date().addingTimeInterval(30)
        while Date() < deadline {
            if exists(id: "team-password") {
                try expect(!ctx.teamPassword.isEmpty, "the team gate shows but OPENFRAME_E2E_TEAM_PASSWORD is empty")
                try await type(ctx.teamPassword, into: "team-password")
                try await press(id: "team-gate-continue", role: "AXButton")
                try await waitGone(id: "team-password", timeout: 30)
                break
            }
            if exists(id: "home-new-project") || exists(id: "home-empty") || exists(id: "project-home") || exists(id: "welcome-name") { break }
            if exists(id: "startup-failure") { throw DriverError("the app could not start: \(screenText())") }
            await idle(0.2)
        }
        try await answerWelcome()
        try await waitHome()
    }

    /// The first launch asks for the person's name (spec 01, case 1). Every scenario answers
    /// 'E2E Tester' like the Windows suite.
    func answerWelcome(_ name: String = "E2E Tester") async throws {
        let deadline = Date().addingTimeInterval(4)
        while Date() < deadline {
            if exists(id: "welcome-name") {
                try await type(name, into: "welcome-name")
                try await press(id: "welcome-continue", role: "AXButton")
                try await waitGone(id: "welcome-name", timeout: 15)
                return
            }
            await idle(0.2)
        }
    }

    func waitHome(timeout: Double = 30) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if exists(id: "home-new-project") || exists(label: "New Project", role: "AXButton") { return }
            await idle(0.2)
        }
        throw DriverError("Application Home did not show: \(screenText().prefix(400))")
    }

    func createProject(_ title: String) async throws {
        try await menu(["File", "New Project…"])
        // The sheet may still be sliding in when its field is asked to take the keyboard.
        var typed = false
        for attempt in 1...3 where !typed {
            do {
                try await type(title, into: "new-project-title")
                typed = true
            } catch where attempt < 3 {
                await idle(0.5)
            }
        }
        try await press(id: "new-project-create", role: "AXButton")
        try await wait(id: "project-home", timeout: 30, "Project Home after creating \(title)")
    }

    func openProject(_ title: String, expectRecovery: Bool = false) async throws {
        try await waitHome()
        try await press(id: "project-card-\(title)")
        try await wait(id: "project-home", timeout: 30, "Project Home of \(title)")
        let offered = await recoveryOffered(within: expectRecovery ? 10 : 2)
        if expectRecovery { try expect(offered, "the recovery question shows after a crash") }
        if offered {
            try await press(label: "Keep the latest work", role: "AXButton")
            try await waitGone(label: "OpenFrame closed unexpectedly last time")
        }
    }

    /// The question the app asks when a project did not close cleanly.
    func recoveryOffered(within seconds: Double) async -> Bool {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline {
            if all().contains(where: { $0.text.contains("OpenFrame closed unexpectedly last time") }) { return true }
            await idle(0.2)
        }
        return false
    }

    func closeProject() async throws {
        try await menu(["File", "Close Project"])
        try await waitHome()
    }

    /// Opens a workspace with the Go menu (Command+Control+number), the keyboard path.
    func go(_ workspace: String) async throws {
        if menuItemEnabled(["Go", workspace]) {
            try await menu(["Go", workspace])
        } else {
            try await press(id: "sidebar-\(Self.workspaceKey(workspace))")
        }
        await idle(0.4)
    }

    static func workspaceKey(_ title: String) -> String {
        switch title {
        case "Project Home": "home"
        case "Screenplay": "screenplay"
        case "Story": "story"
        case "Idea Vault": "ideaVault"
        case "Breakdown": "breakdown"
        case "Schedule": "schedule"
        case "Call Sheets": "callSheets"
        case "Budget": "budget"
        case "Visual Planning": "visual"
        case "Files": "files"
        case "Notes and Tasks": "notes"
        case "Activity": "activity"
        case "Recently Deleted": "recentlyDeleted"
        default: title
        }
    }

    // MARK: screenplay

    func editor(timeout: Double = 30) async throws -> NSTextView {
        let e = try await wait(id: "screenplay-editor", timeout: timeout, "the screenplay editor")
        guard let tv = e.obj as? NSTextView else { throw DriverError("the screenplay editor is not a text view: \(e)") }
        tv.window?.makeKeyAndOrderFront(nil)
        tv.window?.makeFirstResponder(tv)
        await idle(0.3)
        return tv
    }

    /// Puts the caret at the end of the script.
    func caretToEnd(_ tv: NSTextView) async {
        tv.setSelectedRange(NSRange(location: (tv.string as NSString).length, length: 0))
        await idle(0.1)
    }

    func waitSaved(timeout: Double = 20) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            // The editor toolbar shows the state when the window is wide enough; the project
            // sidebar always does.
            if (query(id: "editor-save-status") + query(id: "save-status")).contains(where: { $0.text.contains("Saved") }) { return }
            await idle(0.2)
        }
        throw DriverError("the editor never said Saved: \(query(id: "editor-save-status").map(\.text))")
    }
}
