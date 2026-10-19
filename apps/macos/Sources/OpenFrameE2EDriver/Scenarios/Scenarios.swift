// The list of end-to-end scenarios. Each name is "<shared scenario file>/<case>", so a result
// maps straight to tests/e2e/scenarios/<file>.md.

enum Scenarios {
    static var all: [Scenario] {
        releaseSmoke + teamGate + forcedKill + writingGroup + productionGroup + interchangeGroup + captureGroup + windowFitGroup
    }
}

// MARK: 00-release-smoke

extension Scenarios {
    static let smokeLine = "The driver waits one more minute."

    static let releaseSmoke: [Scenario] = [
        Scenario("00-release-smoke/create-type-close-reopen", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Smoke Night")
                try await ui.go("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("INT. NIGHT BUS - NIGHT")
                try await ui.enter()
                try await ui.typeHere(smokeLine)
                try await ui.waitSaved()
                try ui.snapshot("typed")
                try await ui.closeProject()
                try ui.expect(!ui.exists(id: "project-home"), "the project shell is gone after Close Project")
            } else {
                try await ui.openProject("Smoke Night")
                try ui.expect(!(await ui.recoveryOffered(within: 3)), "no recovery question after a clean quit")
                try await ui.go("Screenplay")
                let tv = try await ui.editor()
                try ui.expect(tv.string.contains(smokeLine), "the typed line is there after a restart: \(tv.string.suffix(200))")
            }
        },
    ]
}

// MARK: team gate (17-internal-build-release)

extension Scenarios {
    static let teamGate: [Scenario] = [
        Scenario("17-internal-build-release/team-gate-wrong-then-right") { ui, _, ctx in
            try await ui.wait(id: "team-password", timeout: 30, "the team gate (this must be an internal build)")
            try await ui.type("wrong-password", into: "team-password")
            try await ui.press(id: "team-gate-continue", role: "AXButton")
            try await ui.wait(id: "team-gate-message", timeout: 20, "the wrong-password message")
            try ui.expect(!ui.exists(id: "home-new-project"), "a wrong password does not open the app")
            try await ui.type(ctx.teamPassword, into: "team-password")
            try await ui.press(id: "team-gate-continue", role: "AXButton")
            try await ui.answerWelcome()
            try await ui.waitHome()
        },
    ]
}

// MARK: 26-forced-kill-keeps-work

extension Scenarios {
    static let killLine = "Meera keeps the ticket in her fist."

    static let forcedKill: [Scenario] = [
        Scenario("26-forced-kill-keeps-work/saved-text-survives-sigkill", phases: 2, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Kill Test")
                try await ui.go("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("EXT. DEPOT - DAWN")
                try await ui.enter()
                try await ui.typeHere(killLine)
                try await ui.waitSaved()
                // The runner now kills the app with SIGKILL, with no chance to clean up.
            } else {
                try await ui.openProject("Kill Test", expectRecovery: true)
                try await ui.go("Screenplay")
                let tv = try await ui.editor()
                try ui.expect(tv.string.contains(killLine), "text saved before a forced kill is there: \(tv.string.suffix(200))")
            }
        },
    ]
}
