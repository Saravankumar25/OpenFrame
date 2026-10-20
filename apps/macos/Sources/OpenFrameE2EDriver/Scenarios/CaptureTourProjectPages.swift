// Capture tour section: Files, Notes and Tasks, Activity, Recently Deleted, Templates, settings tabs, recovery. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureProjectPages: [Scenario] = [
        tour("10-files-notes", order: 10000, importSample: false) { ui, _ in
            try await ui.capNav("Files")
            await ui.capture("files-empty", title: "Files, empty", area: "Files")
            await ui.exploreMenu("add-file", area: "Files", prefix: "files-add", from: "files-empty", extraSkip: ["file", "copy"])
            await ui.step("folder") {
                try await ui.capAct(id: "new-folder")
                _ = try await ui.wait(id: "folder-name", timeout: 6)
                await ui.capture("files-new-folder-sheet", title: "New folder", area: "Files", from: "files-empty", via: "new-folder")
                try await ui.capTypeIn("References", into: "folder-name")
                try await ui.capAct(id: "folder-confirm")
                await ui.idle(0.8)
                await ui.capture("files-with-folder", title: "Files with a folder", area: "Files", from: "files-new-folder-sheet")
            }
            await ui.exploreMenu("files-view", area: "Files", prefix: "files-view", from: "files-empty", limit: 3)
            try await ui.capNav("Notes and Tasks")
            await ui.capture("notes-empty", title: "Notes and Tasks, empty", area: "Notes and Tasks")
            await ui.step("note") {
                try await ui.capAct(id: "new-note")
                _ = try await ui.wait(id: "note-title", timeout: 8)
                await ui.capture("notes-new-note", title: "New note", area: "Notes and Tasks", from: "notes-empty", via: "new-note")
                try await ui.fill(nil, "Depot permit call", id: "note-title")
                try await ui.fill(nil, "Ask the depot manager about the lamp rig before Friday.", id: "note-body")
                await ui.idle(1.2)
                await ui.capture("notes-note-written", title: "A note written", area: "Notes and Tasks", from: "notes-new-note", via: "note-body")
                await ui.exploreButtons([("note-pin", "Pin the note")], area: "Notes and Tasks", prefix: "notes", from: "notes-note-written")
                _ = await ui.tryPress(id: "note-close")
            }
            await ui.step("task") {
                try await ui.capAct(id: "new-task")
                _ = try await ui.wait(id: "task-title", timeout: 8)
                await ui.capture("tasks-new-task", title: "New task", area: "Notes and Tasks", from: "notes-empty", via: "new-task")
                try await ui.fill(nil, "Book the lamp rig", id: "task-title")
                await ui.exploreButtons([("task-has-due", "Due date"), ("task-related-open", "Link to something")], area: "Notes and Tasks", prefix: "tasks", from: "tasks-new-task")
                try? await ui.capAct(id: "task-save")
                await ui.idle(0.8)
                await ui.capture("notes-and-tasks", title: "Notes and Tasks with a note and a task", area: "Notes and Tasks", from: "tasks-new-task", via: "task-save")
            }
            await ui.exploreMenu("tasks-filter", area: "Notes and Tasks", prefix: "tasks-filter", from: "notes-and-tasks", limit: 4)
            try await ui.capNav("Activity")
            await ui.capture("activity", title: "Activity", area: "Activity")
            await ui.exploreMenu("activity-filter", area: "Activity", prefix: "activity-filter", from: "activity", limit: 5)
            try await ui.capNav("Recently Deleted")
            await ui.capture("recently-deleted-empty", title: "Recently Deleted, empty", area: "Recently Deleted")
        },
    ]

    static let captureLifecycle: [Scenario] = [
        Scenario("zz-capture/12-recovery", phases: 2, timeout: 300, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            ui.captureBegin(order: 12000 + (phase - 1) * 100)
            try await ui.startApp(ctx)
            ui.capBigWindow()
            if phase == 1 { await ui.capSetMode() }
            if phase == 1 {
                try await ui.createProject("Night Bus")
                try await ui.capNav("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("INT. DEPOT - NIGHT")
                try await ui.waitSaved()
            } else {
                try await ui.waitHome()
                await ui.capture("home-after-crash", title: "Application Home after a forced quit", area: "Recovery")
                try await ui.press(id: "project-card-Night Bus")
                _ = try await ui.wait(id: "project-home", timeout: 30)
                let offered = await ui.recoveryOffered(within: 10)
                await ui.capture("recovery-alert", title: "Recovery question after the app closed unexpectedly", area: "Recovery", from: "home-after-crash", via: "project-card-Night Bus")
                if offered { try? await ui.press(label: "Keep the latest work", role: "AXButton") }
                await ui.capRecover()
            }
        },
        Scenario("zz-capture/12-unavailable", phases: 2, timeout: 300, end: { _ in .quit }) { ui, phase, ctx in
            ui.captureBegin(order: 12500 + (phase - 1) * 100)
            try await ui.startApp(ctx)
            ui.capBigWindow()
            if phase == 1 { await ui.capSetMode() }
            if phase == 1 {
                try await ui.createProject("Night Bus")
                try await ui.closeProject()
                // The project folder is moved away while the app is closed, so the card reports it missing.
                let root = ctx.dataRoot.appendingPathComponent("projects")
                let away = ctx.dataRoot.appendingPathComponent("moved-away")
                try? FileManager.default.removeItem(at: away)
                try FileManager.default.moveItem(at: root, to: away)
            } else {
                try await ui.waitHome()
                await ui.capture("home-project-unavailable", title: "Application Home, a project that cannot be found", area: "Recovery")
                if let card = ui.query(id: "project-card-Night Bus").first { try? await ui.press(card) }
                await ui.idle(1.0)
                await ui.capture("project-unavailable", title: "Project unavailable", area: "Recovery", from: "home-project-unavailable", via: "project-card-Night Bus")
                await ui.capRecover()
            }
        },
    ]
}
