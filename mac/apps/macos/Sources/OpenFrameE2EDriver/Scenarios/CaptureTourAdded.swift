// The capture tour, the screens added on 2026-10-27 and after: the library setup on a new Mac,
// Settings > Storage with a project move, the Story page typed word by word, its dark page and zoom,
// the Beat Board in rows, the screenplay dark page, a cover photo and "Found on this Mac" on Home,
// and the Assistant memory card. Real AI answers are not captured: the development build has no AI engine.
//
// The library setup scenario needs a development build: the runner sets OPENFRAME_LIBRARY_SETUP=1
// for scenarios whose name holds "first-start".

import AppKit
import Foundation

extension Scenarios {
    static let captureAdded: [Scenario] = [addedLibrarySetup, addedPages, addedHome]

    /// First start on a new Mac: the three ways to set up the library.
    static let addedLibrarySetup = Scenario("zz-capture/14-first-start-library", timeout: 400) { ui, _, ctx in
        let A = "Library setup"
        ui.captureBegin(order: 14000)
        try await ui.wait(id: "library-setup", timeout: 30, "the library setup screen (run with OPENFRAME_LIBRARY_SETUP=1)")
        ui.capBigWindow()
        await ui.capSetMode()
        ui.capBigWindow()
        await ui.capture("library-setup-default", title: "Set up your OpenFrame library, the default location", area: A)
        await ui.step("library setup new location") {
            try await Scenarios.segment(ui, group: "library-setup-choice", "Choose a new location")
            await ui.capture("library-setup-new-location", title: "Set up your library, choose a new location", area: A, from: "library-setup-default", via: "library-setup-choice")
            let parent = ctx.work.appendingPathComponent("new-library-\(UUID().uuidString.prefix(6))", isDirectory: true)
            try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)
            _ = try await ui.choosing(parent) { try await ui.press(id: "library-setup-choose-parent", role: "AXButton") }
            _ = try await ui.wait(id: "library-setup-new-path", timeout: 15)
            await ui.capture("library-setup-new-location-chosen", title: "Set up your library, a new folder chosen", area: A, from: "library-setup-new-location", via: "library-setup-choose-parent")
        }
        await ui.step("library setup locate") {
            try await Scenarios.segment(ui, group: "library-setup-choice", "Locate existing OpenFrame data")
            await ui.capture("library-setup-locate", title: "Set up your library, locate existing data", area: A, from: "library-setup-default", via: "library-setup-choice")
            let empty = ctx.work.appendingPathComponent("empty-\(UUID().uuidString.prefix(6))", isDirectory: true)
            try FileManager.default.createDirectory(at: empty, withIntermediateDirectories: true)
            _ = try await ui.choosing(empty) { try await ui.press(id: "library-setup-choose-existing", role: "AXButton") }
            try await ui.wait(id: "library-setup-preview", timeout: 15, "what the core found in the folder")
            await ui.capture("library-setup-locate-nothing-found", title: "Set up your library, nothing found in the chosen folder", area: A, from: "library-setup-locate", via: "library-setup-choose-existing")
        }
        try await Scenarios.segment(ui, group: "library-setup-choice", "Use the default location")
        try await ui.press(id: "library-setup-create-default", role: "AXButton")
        try await ui.answerWelcome()
        try await ui.waitHome()
    }

    /// Night Bus: the Story page, the Beat Board in rows, the screenplay dark page and Settings > Storage.
    static let addedPages = tour("14-added-pages", order: 14100) { ui, ctx in
        let A = "Story"
        await ui.step("story page typed word by word") {
            try await ui.capNav("Story")
            try await ui.capTab("Story")
            let tv = try await ui.storyText(timeout: 20)
            tv.window?.makeKeyAndOrderFront(nil)
            tv.window?.makeFirstResponder(tv)
            // One character at a time, as a person types: the placeholder must be gone after the first one.
            for ch in "Meera takes the last bus" { try await ui.typeHere(String(ch)) }
            await ui.idle(0.6)
            await ui.capture("story-page-first-words", title: "Story page, the first words typed", area: A, via: "story-page-text")
            await ui.idle(2.0)
            try await ui.press(id: "story-page-dark")
            await ui.idle(0.6)
            await ui.capture("story-page-dark-sheet", title: "Story page on the dark sheet", area: A, from: "story-page-first-words", via: "story-page-dark")
            try await ui.press(id: "story-page-dark")
            try await ui.press(id: "story-page-zoom-in")
            try await ui.press(id: "story-page-zoom-in")
            await ui.idle(0.5)
            await ui.capture("story-page-zoomed", title: "Story page zoomed to 120 percent", area: A, from: "story-page-first-words", via: "story-page-zoom-in")
            try await ui.press(id: "story-page-zoom-reset")
        }
        await ui.step("beat board rows") {
            try await ui.capTab("Beat Board")
            try await Scenarios.addAct(ui, "Act One")
            try await Scenarios.addSequence(ui, "Depot")
            try await Scenarios.addCard(ui, "Meera misses the last bus")
            try await Scenarios.addCard(ui, "The driver will not wait")
            try await Scenarios.segment(ui, group: "story-board-layout", "Horizontal")
            await ui.idle(0.8)
            await ui.capture("story-board-horizontal", title: "Beat Board in rows (Horizontal)", area: A, via: "story-board-layout")
            try await Scenarios.segment(ui, group: "story-board-layout", "Vertical")
            await ui.idle(0.6)
            await ui.capture("story-board-vertical", title: "Beat Board in columns (Vertical)", area: A, from: "story-board-horizontal", via: "story-board-layout")
        }
        await ui.step("screenplay dark page") {
            try await ui.capNav("Screenplay")
            _ = try await ui.editor()
            try await Scenarios.chooseMenuItem(ui, button: "screenplay-tools-menu", item: "Dark page")
            await ui.idle(0.8)
            await ui.capture("screenplay-dark-page", title: "Screenplay on the dark page", area: "Screenplay", via: "screenplay-tools-menu")
            try await Scenarios.chooseMenuItem(ui, button: "screenplay-tools-menu", item: "Light page")
            await ui.idle(0.4)
        }
        let S = "Settings"
        await ui.step("settings storage") {
            try await ui.openProjectSettings(page: "Storage")
            try await ui.wait(id: "storage-project-move", timeout: 15, "Move Project")
            await ui.capture("project-settings-storage", title: "Settings, Storage: library, projects folder, Global Idea Vault, this project, app data", area: S, via: "project-settings-tabs")
            let dest = ctx.dataRoot.appendingPathComponent("moved-projects", isDirectory: true)
            try FileManager.default.createDirectory(at: dest, withIntermediateDirectories: true)
            _ = try await ui.choosing(dest) { try await ui.press(id: "storage-project-move", role: "AXButton") }
            try await ui.wait(id: "storage-task-confirm", timeout: 10, "the move confirmation")
            await ui.capture("storage-move-project-confirm", title: "Move Project, the confirmation", area: S, from: "project-settings-storage", via: "storage-project-move")
            try await ui.press(id: "storage-task-confirm", role: "AXButton")
            try await ui.wait(id: "storage-task-done", timeout: 120, "the move result")
            await ui.capture("storage-move-project-done", title: "Move Project, the project moved", area: S, from: "storage-move-project-confirm", via: "storage-task-confirm")
            try await ui.press(id: "storage-task-close", role: "AXButton")
            await ui.idle(0.4)
            try await ui.closeStorageSettings()
        }
    }

    /// Home: the card menu with the cover photo items, a cover photo, and Found on this Mac.
    static let addedHome = tour("14-added-home", order: 14300, importSample: false) { ui, ctx in
        let A = "Home"
        try await ui.closeProject()
        try await ui.waitHome()
        await ui.capMenu(id: "project-card-actions-Night Bus", name: "home-card-menu", title: "Project card menu with Change Cover Photo", area: A, from: "home-filled")
        await ui.step("cover photo") {
            let pic = ctx.work.appendingPathComponent("cover-\(UUID().uuidString.prefix(6)).png")
            try Scenarios.capCoverPicture(to: pic)
            _ = try await ui.choosing(pic) { try await ui.chooseMenuItem(id: "project-card-actions-Night Bus", item: "Change Cover Photo\u{2026}") }
            try await ui.waitText("Cover photo changed", timeout: 15)
            await ui.idle(1.0)
            await ui.capture("home-cover-photo", title: "Home, a project with a cover photo", area: A, from: "home-card-menu", via: "Change Cover Photo")
        }
        await ui.step("found on this mac") {
            for d in ui.query(label: "Dismiss") where d.role == "AXButton" { _ = d.performPress() }
            try await ui.chooseMenuItem(id: "project-card-actions-Night Bus", item: "Remove from Recent")
            try await ui.waitText("Found on this Mac", timeout: 15)
            await ui.idle(0.6)
            await ui.capture("home-found-on-this-mac", title: "Home, Found on this Mac", area: A, from: "home-card-menu", via: "Remove from Recent")
            try await ui.press(id: "home-found-open-Night Bus")
            _ = try await ui.wait(id: "project-home", timeout: 30, "the found project")
        }
    }

    /// A small landscape picture to use as a cover photo.
    static func capCoverPicture(to url: URL) throws {
        let w = 800, h = 450
        guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: w, pixelsHigh: h, bitsPerSample: 8, samplesPerPixel: 4,
                                         hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
              let ctx = NSGraphicsContext(bitmapImageRep: rep) else { throw DriverError("no canvas for the cover picture") }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = ctx
        let rect = NSRect(x: 0, y: 0, width: w, height: h)
        NSGradient(starting: NSColor(red: 0.08, green: 0.12, blue: 0.25, alpha: 1), ending: NSColor(red: 0.85, green: 0.45, blue: 0.2, alpha: 1))?.draw(in: rect, angle: 90)
        NSColor(white: 1, alpha: 0.85).setFill()
        for i in 0..<9 { NSRect(x: 60 + i * 80, y: 90, width: 50, height: 28).fill() }
        NSGraphicsContext.restoreGraphicsState()
        guard let png = rep.representation(using: .png, properties: [:]) else { throw DriverError("no cover picture") }
        try png.write(to: url)
    }
}
