// Capture tour section: Files, Notes and Tasks, Activity, Recently Deleted, Project Settings with every tab,
// Templates, and the recovery and unavailable screens. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation
import QuartzCore

// MARK: helpers of this section

fileprivate extension UI {
    /// The first element whose identifier starts with `prefix` and that is on screen.
    func pagesFirst(_ prefix: String, role: String? = nil) -> Element? {
        all().first { $0.identifier.hasPrefix(prefix) && $0.frame.width > 0 && (role == nil || $0.role == role) }
    }

    func pagesWaitFirst(_ prefix: String, role: String? = nil, timeout: Double = 10) async throws -> Element {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let e = pagesFirst(prefix, role: role) { return e }
            await idle(0.15)
        }
        throw DriverError("nothing named \(prefix)* on screen")
    }

    /// The file row (list view) whose label starts with this name.
    func pagesFileRow(_ name: String) -> Element? {
        all().first { $0.identifier.hasPrefix("file-row-") && $0.label.hasPrefix(name) }
    }

    func pagesSegment(_ group: String, _ name: String) async throws {
        try await Scenarios.segment(self, group: group, name)
        await idle(0.5)
    }

    /// Writes the invented files the Files page adds: a letter, a location photo and a permit PDF.
    func pagesSourceFiles(_ ctx: Context) throws -> [String: URL] {
        let dir = ctx.work.appendingPathComponent("pages-sources", isDirectory: true)
        try? FileManager.default.removeItem(at: dir)
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        var out: [String: URL] = [:]
        let letter = dir.appendingPathComponent("Depot permit letter.txt")
        try "To the depot manager: we would like to film in bay 4 on the night of 14 June.\n".write(to: letter, atomically: true, encoding: .utf8)
        out["letter"] = letter
        let image = NSImage(size: NSSize(width: 160, height: 100))
        image.lockFocus()
        NSColor(calibratedRed: 0.15, green: 0.22, blue: 0.4, alpha: 1).setFill()
        NSRect(x: 0, y: 0, width: 160, height: 100).fill()
        NSColor.systemYellow.setFill()
        NSRect(x: 20, y: 30, width: 120, height: 30).fill()
        image.unlockFocus()
        if let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) {
            let photo = dir.appendingPathComponent("Bus depot at night.png")
            try png.write(to: photo)
            out["photo"] = photo
        }
        let pdf = dir.appendingPathComponent("Night shoot permit.pdf")
        var box = CGRect(x: 0, y: 0, width: 300, height: 200)
        if let ctxPDF = CGContext(pdf as CFURL, mediaBox: &box, nil) {
            ctxPDF.beginPDFPage(nil)
            ctxPDF.setFillColor(NSColor.black.cgColor)
            ctxPDF.fill(CGRect(x: 20, y: 150, width: 200, height: 8))
            ctxPDF.endPDFPage()
            ctxPDF.closePDF()
            out["pdf"] = pdf
        }
        let linked = dir.appendingPathComponent("Lamp rig quote.txt")
        try "Lamp rig, two nights, with operator.\n".write(to: linked, atomically: true, encoding: .utf8)
        out["linked"] = linked
        return out
    }

    /// Adds one file through Add File, answering the file panel with `url`.
    func pagesAdd(_ url: URL, copy: Bool) async throws {
        try await choosing(url) {
            try await capMenuButton(id: "add-file", item: copy ? "Copy into project" : "Link to file")
        }
        try await waitText(url.deletingPathExtension().lastPathComponent, timeout: 10)
        await idle(0.6)
        pagesRedraw()
        await idle(0.6)
    }

    /// True when the picture's sidebar area is one flat colour: the window was drawn before its content
    /// (another app's window covered it, so AppKit skipped drawing).
    func pagesLooksBlank(_ name: String) -> Bool {
        let url = captureDir.appendingPathComponent("\(name).png")
        guard let rep = NSBitmapImageRep(data: (try? Data(contentsOf: url)) ?? Data()) else { return false }
        let w = rep.pixelsWide, h = rep.pixelsHigh
        var lo = 3.0, hi = 0.0
        for yi in 0..<40 {
            for xi in 0..<30 {
                let x = Int(Double(w) * (0.02 + 0.14 * Double(xi) / 30)), y = Int(Double(h) * (0.15 + 0.35 * Double(yi) / 40))
                guard let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { continue }
                let l = Double(c.redComponent + c.greenComponent + c.blueComponent)
                lo = min(lo, l); hi = max(hi, l)
            }
        }
        return hi - lo < 0.25
    }

    /// Runs a capture (`body` takes the picture called `name`) with the window in front, and takes it
    /// again, up to three times, when the picture came out blank.
    func pagesEnsure(_ name: String, _ body: () async -> Void) async {
        for attempt in 0..<3 {
            pagesRedraw()
            await idle(attempt == 0 ? 0.4 : 1.2)
            await body()
            guard pagesLooksBlank(name) else { return }
            log("capture \(name) came out blank, taking it again")
            CaptureState.names.remove(name)
            CaptureState.count = max(0, CaptureState.count - 1)
        }
    }

    func pagesCap(_ name: String, title: String, area: String, from: String? = nil, via: String? = nil) async {
        await pagesEnsure(name) { await capture(name, title: title, area: area, from: from, via: via) }
    }

    func pagesMenu(id: String, name: String, title: String, area: String, from: String? = nil) async {
        await pagesEnsure(name) { await capMenu(id: id, name: name, title: title, area: area, from: from) }
    }

    func pagesContext(on e: Element, name: String, title: String, area: String, from: String? = nil) async {
        await pagesEnsure(name) { await capContext(on: e, name: name, title: title, area: area, from: from) }
    }

    /// A control inside a sheet whose identifier SwiftUI gave to every element in it.
    func pagesSheetElement(_ sheet: String, role: String, label: String? = nil, timeout: Double = 8) async throws -> Element {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let e = all().first(where: { $0.identifier == sheet && $0.role == role && (label == nil || $0.label == label || $0.title == label) && $0.frame.width > 0 }) { return e }
            await idle(0.15)
        }
        throw DriverError("no \(role) \(label ?? "") in \(sheet)")
    }

    func pagesSheetPress(_ sheet: String, _ label: String) async throws {
        let e = try await pagesSheetElement(sheet, role: "AXButton", label: label)
        try await press(e)
        await idle(0.6)
    }

    func pagesTypeInto(_ e: Element, _ text: String) async throws {
        if let tf = (e.obj as? NSTextField) ?? ((e.obj as? NSCell)?.controlView as? NSTextField), let w = tf.window {
            w.makeKeyAndOrderFront(nil); w.makeFirstResponder(tf)
        } else { try await tap(e) }
        await idle(0.2)
        guard let w = NSApp.keyWindow, let tv = w.firstResponder as? NSTextView else { throw DriverError("no keyboard focus in \(e)") }
        tv.selectAll(nil)
        tv.insertText(text, replacementRange: tv.selectedRange())
        await idle(0.2)
    }

    /// The answered file panel leaves the window undrawn until something happens: bring it forward and draw it.
    func pagesRedraw() {
        guard let w = NSApp.windows.filter({ $0.isVisible && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) else { return }
        NSApp.activate(ignoringOtherApps: true)
        w.makeKeyAndOrderFront(nil)
        w.orderFrontRegardless()
        w.contentView?.needsDisplay = true
        w.contentView?.needsLayout = true
        w.displayIfNeeded()
        w.contentView?.display()
        CATransaction.flush()
    }

    /// Closes the details inspector of Files, if it is open.
    func pagesCloseFile() async {
        if await tryPress(id: "file-close", wait: 1) { await idle(0.5) }
    }

    func pagesNewTask(_ title: String) async throws {
        pagesRedraw()
        try await capAct(id: "new-task")
        _ = try await wait(id: "task-title", timeout: 12)
        try await fill(nil, title, id: "task-title")
        try await capAct(id: "task-save")
        try await waitGone(id: "task-title", timeout: 8)
        await idle(0.6)
    }

    func pagesNewNote(_ title: String, _ body: String) async throws {
        pagesRedraw()
        try await capAct(id: "new-note")
        _ = try await wait(id: "note-title", timeout: 12)
        try await fill(nil, title, id: "note-title")
        try await fill(nil, body, id: "note-body")
        await idle(1.2)
    }
}

extension Scenarios {
    static let captureProjectPages: [Scenario] = [pagesFiles, pagesNotes, pagesActivityTrash, pagesSettings]

    // MARK: Files

    static let pagesFiles = tour("10-files", order: 10000, importSample: false) { ui, ctx in
        let area = "Files"
        let src = try ui.pagesSourceFiles(ctx)
        try await ui.capNav("Files")
        try? await ui.pagesSegment("files-view", "List")
        await ui.pagesCap("files-empty", title: "Files, empty", area: area)
        await ui.pagesMenu(id: "add-file", name: "pages-files-add-menu", title: "Files, the Add File menu", area: area, from: "files-empty")
        await ui.step("new folder") {
            try await ui.capAct(id: "new-folder")
            _ = try await ui.wait(id: "folder-name", timeout: 6)
            await ui.pagesCap("files-new-folder-sheet", title: "Files, new folder", area: area, from: "files-empty", via: "new-folder")
            try await ui.capTypeIn("References", into: "folder-name")
            await ui.pagesCap("pages-files-new-folder-named", title: "Files, new folder with a name", area: area, from: "files-new-folder-sheet", via: "folder-name")
            try await ui.capAct(id: "folder-confirm")
            try await ui.waitGone(id: "folder-name", timeout: 6)
            await ui.idle(0.8)
            await ui.pagesCap("files-with-folder", title: "Files with a folder", area: area, from: "files-new-folder-sheet", via: "folder-confirm")
        }
        await ui.step("add a copy") {
            if let top = ui.query(id: "folder-top").first { try await ui.press(top); await ui.idle(0.5) }
            guard let letter = src["letter"] else { throw DriverError("no letter file") }
            try await ui.pagesAdd(letter, copy: true)
            await ui.pagesCap("pages-files-one-file", title: "Files, one file added", area: area, from: "pages-files-add-menu", via: "add-file-copy")
        }
        await ui.step("add more files") {
            for key in ["photo", "pdf"] { if let u = src[key] { try await ui.pagesAdd(u, copy: true) } }
            if let u = src["linked"] { try await ui.pagesAdd(u, copy: false) }
            await ui.idle(2.5)
            await ui.pagesCap("pages-files-list", title: "Files, stored and linked files in the list", area: area, from: "pages-files-one-file", via: "add-file")
        }
        await ui.step("grid") {
            try await ui.pagesSegment("files-view", "Grid")
            await ui.pagesCap("pages-files-grid", title: "Files, grid view", area: area, from: "pages-files-list", via: "files-view")
            if let card = ui.pagesFirst("file-card-") {
                try await ui.capCustomAction(card, "Show details")
                await ui.idle(0.8)
                await ui.pagesCap("pages-files-grid-selected", title: "Files, grid view with a file selected", area: area, from: "pages-files-grid", via: "file-card")
                await ui.pagesCloseFile()
            }
            try await ui.pagesSegment("files-view", "List")
        }
        await ui.step("row menus") {
            guard let row = ui.pagesFileRow("Depot permit letter") else { throw DriverError("no letter row") }
            await ui.pagesContext(on: row, name: "pages-files-context-menu", title: "Files, a file's context menu", area: area, from: "pages-files-list")
            if let more = ui.pagesFirst("file-more-") {
                await ui.pagesMenu(id: more.identifier, name: "pages-files-more-menu", title: "Files, a file's More menu", area: area, from: "pages-files-list")
            }
        }
        await ui.step("file details") {
            guard let row = ui.pagesFileRow("Depot permit letter") else { throw DriverError("no letter row") }
            try await ui.capCustomAction(row, "Show details")
            _ = try await ui.wait(id: "file-drawer", timeout: 6)
            await ui.pagesCap("pages-files-selected", title: "Files, a file selected with its details", area: area, from: "pages-files-list", via: "file-row")
            try await ui.fill(nil, "Signed copy still to come from the depot office.", id: "file-notes")
            await ui.idle(1.2)
            await ui.pagesCap("pages-files-details-notes", title: "Files, notes written in a file's details", area: area, from: "pages-files-selected", via: "file-notes")
            try await ui.capMenuButton(id: "file-folder", item: "References")
            await ui.idle(1.0)
            await ui.pagesCap("pages-files-moved", title: "Files, a file moved into a folder", area: area, from: "pages-files-details-notes", via: "file-folder")
            await ui.pagesCloseFile()
        }
        await ui.step("linked details") {
            guard let row = ui.pagesFileRow("Lamp rig quote") else { throw DriverError("no linked row") }
            try await ui.capCustomAction(row, "Show details")
            _ = try await ui.wait(id: "file-drawer", timeout: 6)
            await ui.pagesCap("pages-files-linked-details", title: "Files, a linked file's details", area: area, from: "pages-files-list", via: "file-row")
            await ui.pagesCloseFile()
        }
        await ui.step("folder") {
            guard let f = ui.all().first(where: { $0.identifier.hasPrefix("folder-") && $0.label.hasPrefix("References") }) else { throw DriverError("no folder row") }
            try await ui.press(f)
            await ui.idle(0.8)
            await ui.pagesCap("pages-files-folder-open", title: "Files, a folder open", area: area, from: "files-with-folder", via: "folder-References")
            await ui.pagesContext(on: f, name: "pages-files-folder-menu", title: "Files, a folder's context menu", area: area, from: "pages-files-folder-open")
            await ui.pagesEnsure("pages-files-rename-folder") { await ui.visit(press: "folder-rename", name: "pages-files-rename-folder", title: "Files, rename a folder", area: area, from: "pages-files-folder-open", close: "folder-cancel") }
            await ui.pagesEnsure("pages-files-delete-folder") { await ui.visit(press: "folder-delete", name: "pages-files-delete-folder", title: "Files, delete a folder", area: area, from: "pages-files-folder-open", close: "folder-delete-cancel") }
        }
        await ui.step("subfolder empty") {
            try await ui.capAct(id: "new-folder")
            _ = try await ui.wait(id: "folder-name", timeout: 6)
            await ui.pagesCap("pages-files-new-subfolder", title: "Files, new folder inside a folder", area: area, from: "pages-files-folder-open", via: "new-folder")
            try await ui.capTypeIn("Contracts", into: "folder-name")
            try await ui.capAct(id: "folder-confirm")
            try await ui.waitGone(id: "folder-name", timeout: 6)
            await ui.idle(0.6)
            guard let f = ui.all().first(where: { $0.identifier.hasPrefix("folder-") && $0.label.hasPrefix("Contracts") }) else { throw DriverError("no subfolder row") }
            try await ui.press(f)
            await ui.idle(0.8)
            await ui.pagesCap("pages-files-folder-empty", title: "Files, an empty folder", area: area, from: "pages-files-new-subfolder", via: "folder-Contracts")
            if let all = ui.query(id: "folder-all").first { try await ui.press(all) }
            await ui.idle(0.6)
        }
        await ui.step("search") {
            try await ui.fill(nil, "permit", id: "files-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-files-search", title: "Files, a search", area: area, from: "pages-files-list", via: "files-search")
            try await ui.fill(nil, "costume", id: "files-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-files-search-none", title: "Files, a search with no match", area: area, from: "pages-files-search", via: "files-search")
            try await ui.fill(nil, "", id: "files-search")
            await ui.idle(0.6)
        }
        await ui.step("unavailable link") {
            guard let u = src["linked"] else { throw DriverError("no linked file") }
            try FileManager.default.removeItem(at: u)
            // Leaving the page and coming back reads the files again, so the missing link shows.
            try await ui.capNav("Home")
            try await ui.capNav("Files")
            await ui.idle(1.0)
            await ui.pagesCap("pages-files-unavailable", title: "Files, a linked file that cannot be found", area: area, from: "pages-files-list")
            guard let row = ui.pagesFileRow("Lamp rig quote") else { throw DriverError("no linked row") }
            await ui.pagesContext(on: row, name: "pages-files-unavailable-menu", title: "Files, the menu of a file that cannot be found", area: area, from: "pages-files-unavailable")
            try await ui.capCustomAction(row, "Show details")
            _ = try await ui.wait(id: "file-drawer", timeout: 6)
            await ui.pagesCap("pages-files-unavailable-details", title: "Files, details of a file that cannot be found", area: area, from: "pages-files-unavailable", via: "file-row")
            await ui.pagesCloseFile()
        }
        await ui.step("delete a file") {
            guard let more = ui.all().first(where: { $0.identifier.hasPrefix("file-more-") && $0.label.contains("Bus depot at night") }) else { throw DriverError("no photo row") }
            try await ui.capMenuButton(id: more.identifier, item: "Delete")
            await ui.idle(0.6)
            await ui.pagesCap("pages-files-deleted", title: "Files, a file moved to Recently Deleted", area: area, from: "pages-files-more-menu", via: "file-menu-delete")
        }
    }

    // MARK: Notes and Tasks

    static let pagesNotes = tour("10-notes", order: 10200, importSample: false) { ui, _ in
        let area = "Notes and Tasks"
        try await ui.capNav("Notes and Tasks")
        await ui.pagesCap("notes-empty", title: "Notes and Tasks, empty", area: area)
        await ui.step("note") {
            try await ui.capAct(id: "new-note")
            _ = try await ui.wait(id: "note-title", timeout: 8)
            await ui.pagesCap("notes-new-note", title: "Notes and Tasks, a new note", area: area, from: "notes-empty", via: "new-note")
            try await ui.fill(nil, "Depot permit call", id: "note-title")
            try await ui.fill(nil, "Ask the depot manager about the lamp rig before Friday.", id: "note-body")
            await ui.idle(1.4)
            await ui.pagesCap("notes-note-written", title: "Notes and Tasks, a note written", area: area, from: "notes-new-note", via: "note-body")
            try await ui.capAct(id: "note-pin")
            await ui.idle(0.8)
            await ui.pagesCap("pages-notes-pinned", title: "Notes and Tasks, a pinned note", area: area, from: "notes-note-written", via: "note-pin")
            _ = await ui.tryPress(id: "note-close")
            await ui.idle(0.6)
        }
        await ui.step("second note") {
            try await ui.pagesNewNote("Night bus route", "Route 41 runs past the depot every forty minutes after midnight.")
            _ = await ui.tryPress(id: "note-close")
            await ui.idle(0.6)
            await ui.pagesCap("pages-notes-two", title: "Notes and Tasks, two notes and no tasks", area: area, from: "notes-note-written")
        }
        await ui.step("task") {
            try await ui.capAct(id: "new-task")
            _ = try await ui.wait(id: "task-title", timeout: 8)
            await ui.pagesCap("tasks-new-task", title: "Notes and Tasks, a new task", area: area, from: "notes-empty", via: "new-task")
            try await ui.fill(nil, "Book the lamp rig", id: "task-title")
            await ui.pagesCap("pages-tasks-titled", title: "Notes and Tasks, a new task with a title", area: area, from: "tasks-new-task", via: "task-title")
            try await ui.capAct(id: "task-has-due")
            await ui.idle(0.6)
            await ui.pagesCap("pages-tasks-due", title: "Notes and Tasks, a task with a due date", area: area, from: "pages-tasks-titled", via: "task-has-due")
            await ui.pagesMenu(id: "task-owner", name: "pages-tasks-owner-menu", title: "Notes and Tasks, the task owner menu", area: area, from: "pages-tasks-due")
            if ui.exists(id: "task-related-open") { try await ui.capAct(id: "task-related-open"); await ui.idle(0.6) }
            if ui.exists(id: "task-related-search") {
                try await ui.fill(nil, "Depot", id: "task-related-search")
                await ui.idle(1.2)
                await ui.pagesCap("pages-tasks-related-search", title: "Notes and Tasks, searching for something to link a task to", area: area, from: "pages-tasks-due", via: "task-related-search")
                try await ui.fill(nil, "", id: "task-related-search")
                await ui.idle(0.4)
            }
            try await ui.capAct(id: "task-save")
            try await ui.waitGone(id: "task-title", timeout: 8)
            await ui.idle(0.8)
            await ui.pagesCap("notes-and-tasks", title: "Notes and Tasks with notes and a task", area: area, from: "tasks-new-task", via: "task-save")
        }
        await ui.step("more tasks") {
            try await ui.pagesNewTask("Print the call sheet for night one")
            try await ui.pagesNewTask("Send the route map to the driver")
            guard let check = ui.all().first(where: { $0.identifier.hasPrefix("task-check-") && $0.label.contains("Print the call sheet") }) else { throw DriverError("no task check box") }
            try await ui.press(check)
            await ui.idle(1.0)
            await ui.pagesCap("pages-tasks-one-done", title: "Notes and Tasks, a task marked done", area: area, from: "notes-and-tasks", via: "task-check")
        }
        await ui.step("task filters") {
            try await ui.pagesSegment("tasks-filter", "Done")
            await ui.pagesCap("pages-tasks-filter-done", title: "Notes and Tasks, done tasks", area: area, from: "pages-tasks-one-done", via: "tasks-filter")
            try await ui.pagesSegment("tasks-filter", "All")
            await ui.pagesCap("pages-tasks-filter-all", title: "Notes and Tasks, all tasks", area: area, from: "pages-tasks-filter-done", via: "tasks-filter")
            await ui.pagesMenu(id: "tasks-owner", name: "pages-tasks-owner-filter", title: "Notes and Tasks, the owner filter", area: area, from: "pages-tasks-filter-all")
        }
        await ui.step("done task editor") {
            guard let row = ui.all().first(where: { $0.identifier.hasPrefix("task-row-") && $0.label.contains("Print the call sheet") }) else { throw DriverError("no done task row") }
            await ui.pagesContext(on: row, name: "pages-tasks-context-menu", title: "Notes and Tasks, a task's context menu", area: area, from: "pages-tasks-filter-all")
            try await ui.press(row)
            _ = try await ui.wait(id: "task-title", timeout: 8)
            await ui.pagesCap("pages-tasks-done-editor", title: "Notes and Tasks, a done task open", area: area, from: "pages-tasks-filter-all", via: "task-row")
            try await ui.capAct(id: "task-cancel")
            await ui.idle(0.6)
        }
        await ui.step("open task editor") {
            guard let row = ui.all().first(where: { $0.identifier.hasPrefix("task-row-") && $0.label.contains("Book the lamp rig") }) else { throw DriverError("no open task row") }
            try await ui.press(row)
            _ = try await ui.wait(id: "task-title", timeout: 8)
            await ui.pagesCap("pages-tasks-editor", title: "Notes and Tasks, an open task being edited", area: area, from: "pages-tasks-filter-all", via: "task-row")
            try await ui.capAct(id: "task-cancel")
            await ui.idle(0.6)
            try await ui.pagesSegment("tasks-filter", "Open")
        }
        await ui.step("note row") {
            guard let row = ui.all().first(where: { $0.identifier.hasPrefix("note-row-") && $0.label.contains("Night bus route") }) else { throw DriverError("no note row") }
            await ui.pagesContext(on: row, name: "pages-notes-context-menu", title: "Notes and Tasks, a note's context menu", area: area, from: "pages-notes-two")
            try await ui.press(row)
            _ = try await ui.wait(id: "note-title", timeout: 8)
            await ui.pagesCap("pages-notes-open", title: "Notes and Tasks, a note open", area: area, from: "pages-notes-two", via: "note-row")
            _ = await ui.tryPress(id: "note-close")
            await ui.idle(0.6)
        }
        await ui.step("notes search") {
            try await ui.fill(nil, "depot", id: "notes-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-notes-search", title: "Notes and Tasks, a note search", area: area, from: "notes-and-tasks", via: "notes-search")
            try await ui.fill(nil, "wardrobe", id: "notes-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-notes-search-none", title: "Notes and Tasks, a note search with no match", area: area, from: "pages-notes-search", via: "notes-search")
            try await ui.fill(nil, "", id: "notes-search")
            await ui.idle(0.5)
        }
    }

    // MARK: Activity and Recently Deleted

    static let pagesActivityTrash = tour("10-activity-deleted", order: 10400) { ui, _ in
        try await ui.capNav("Activity")
        await ui.pagesCap("activity", title: "Activity", area: "Activity")
        var last = "activity"
        for f in ["Drafts", "Story", "Production", "Exchange"] {
            await ui.step("activity \(f)") {
                try await ui.pagesSegment("activity-filter", f)
                let n = "pages-activity-\(ui.slug(f))"
                await ui.pagesCap(n, title: "Activity, \(f) only", area: "Activity", from: last, via: "activity-filter")
                last = n
            }
        }
        try? await ui.pagesSegment("activity-filter", "All")

        let area = "Recently Deleted"
        try await ui.capNav("Recently Deleted")
        await ui.pagesCap("recently-deleted-empty", title: "Recently Deleted, empty", area: area)
        await ui.step("global empty") {
            try await ui.pagesSegment("trash-scope", "Global Idea Vault")
            await ui.pagesCap("pages-deleted-global-empty", title: "Recently Deleted, the global Idea Vault, empty", area: area, from: "recently-deleted-empty", via: "trash-scope")
            try await ui.pagesSegment("trash-scope", "This project")
        }
        await ui.step("delete things") {
            try await ui.capNav("Notes and Tasks")
            try await ui.pagesNewNote("Old catering list", "Tea urn, sandwiches, the van at 2 am.")
            try await ui.capAct(id: "note-delete")
            try await ui.waitGone(id: "note-title", timeout: 8)
            await ui.idle(0.8)
            try await ui.pagesNewNote("Unused title idea", "Last Stop, or The Depot.")
            try await ui.capAct(id: "note-delete")
            await ui.idle(0.8)
            try await ui.pagesNewTask("Call the second bus company")
            guard let row = ui.all().first(where: { $0.identifier.hasPrefix("task-row-") && $0.label.contains("second bus company") }) else { throw DriverError("no task row") }
            try await ui.press(row)
            _ = try await ui.wait(id: "task-title", timeout: 8)
            try await ui.capAct(id: "task-delete")
            await ui.idle(0.8)
        }
        try await ui.capNav("Recently Deleted")
        await ui.idle(0.8)
        await ui.pagesCap("pages-deleted-items", title: "Recently Deleted with items", area: area, from: "recently-deleted-empty")
        await ui.step("trash search") {
            try await ui.fill(nil, "catering", id: "trash-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-deleted-search", title: "Recently Deleted, a search", area: area, from: "pages-deleted-items", via: "trash-search")
            try await ui.fill(nil, "lighting", id: "trash-search")
            await ui.idle(0.8)
            await ui.pagesCap("pages-deleted-search-none", title: "Recently Deleted, a search with no match", area: area, from: "pages-deleted-search", via: "trash-search")
            try await ui.fill(nil, "", id: "trash-search")
            await ui.idle(0.6)
        }
        await ui.step("purge confirmation") {
            try await ui.capAct(id: "purge-Unused title idea")
            await ui.idle(0.8)
            await ui.pagesCap("pages-deleted-purge-confirm", title: "Recently Deleted, delete permanently asks first", area: area, from: "pages-deleted-items", via: "purge-Unused title idea")
            if await ui.tryPress(id: "trash-purge-cancel", wait: 2) {} else { _ = await ui.tryPress(label: "Keep in Recently Deleted", wait: 2) }
            await ui.idle(0.6)
        }
        await ui.step("restore") {
            try await ui.capAct(id: "restore-Old catering list")
            await ui.idle(0.8)
            await ui.pagesCap("pages-deleted-restored", title: "Recently Deleted, an item restored", area: area, from: "pages-deleted-items", via: "restore-Old catering list")
        }
        await ui.step("restored note") {
            try await ui.capNav("Notes and Tasks")
            await ui.idle(0.8)
            await ui.pagesCap("pages-notes-restored", title: "Notes and Tasks, a restored note back in the list", area: "Notes and Tasks", from: "pages-deleted-restored")
        }
    }

    // MARK: Project Settings and Templates

    static let pagesSettings = tour("10-settings", order: 10600, importSample: false) { ui, _ in
        let area = "Project Settings"
        try await ui.menu(["Go", "Project Settings"])
        _ = try await ui.wait(id: "project-settings-sheet", timeout: 10)
        await ui.idle(1.0)
        await ui.pagesCap("pages-settings-general", title: "Project Settings, project settings", area: area, via: "Go > Project Settings")
        await ui.step("general edit") {
            try await ui.fill(nil, "A night bus driver finds the last passenger never gets off.", id: "project-settings-logline")
            await ui.idle(0.6)
            await ui.pagesCap("pages-settings-general-edited", title: "Project Settings, a change not yet saved", area: area, from: "pages-settings-general", via: "project-settings-logline")
            try await ui.fill(nil, "", id: "project-settings-title")
            await ui.idle(0.6)
            await ui.pagesCap("pages-settings-title-error", title: "Project Settings, the title is required", area: area, from: "pages-settings-general-edited", via: "project-settings-title")
            try await ui.fill(nil, "Night Bus", id: "project-settings-title")
            await ui.pagesMenu(id: "project-settings-type", name: "pages-settings-type-menu", title: "Project Settings, the project type menu", area: area, from: "pages-settings-general-edited")
            await ui.pagesMenu(id: "project-settings-status", name: "pages-settings-status-menu", title: "Project Settings, the status menu", area: area, from: "pages-settings-general-edited")
            try await ui.capAct(id: "project-settings-save")
            await ui.idle(1.0)
            await ui.pagesCap("pages-settings-saved", title: "Project Settings, changes saved", area: area, from: "pages-settings-general-edited", via: "project-settings-save")
        }
        var last = "pages-settings-general"
        for (tab, slug) in [("Writing", "writing"), ("Keyboard shortcuts", "shortcuts"), ("Offline AI", "offline-ai"), ("Advanced", "advanced"), ("Appearance", "appearance")] {
            await ui.step("settings tab \(tab)") {
                try await ui.pagesSegment("project-settings-tabs", tab)
                await ui.idle(0.6)
                let n = "pages-settings-\(slug)"
                await ui.pagesCap(n, title: "Project Settings, \(tab)", area: area, from: last, via: "project-settings-tabs")
                last = n
            }
        }
        await ui.step("writing dictionary") {
            try await ui.pagesSegment("project-settings-tabs", "Writing")
            try await ui.fill(nil, "Kadalkkara", id: "writing-dictionary-new")
            try await ui.capAct(id: "writing-dictionary-add")
            await ui.idle(0.8)
            await ui.pagesCap("pages-settings-writing-word", title: "Project Settings, a word added to the dictionary", area: area, from: "pages-settings-writing", via: "writing-dictionary-add")
            await ui.pagesMenu(id: "writing-main-language", name: "pages-settings-language-menu", title: "Project Settings, the main language menu", area: area, from: "pages-settings-writing")
        }

        let tarea = "Templates"
        await ui.step("templates") {
            try await ui.pagesSegment("project-settings-tabs", "Templates")
            await ui.idle(1.0)
            await ui.pagesCap("pages-templates", title: "Templates", area: tarea, from: "pages-settings-general", via: "project-settings-tabs")
        }
        await ui.step("template preview") {
            let card = try await ui.pagesWaitFirst("template-card-builtin-")
            try await ui.press(card)
            _ = try await ui.wait(id: "template-preview-sheet", timeout: 8)
            await ui.pagesCap("pages-templates-preview", title: "Templates, a built-in template previewed", area: tarea, from: "pages-templates", via: "template-card")
            try await ui.pagesSheetPress("template-preview-sheet", "Close")
        }
        await ui.step("template actions") {
            let menu = try await ui.pagesWaitFirst("template-actions-builtin-")
            await ui.pagesMenu(id: menu.identifier, name: "pages-templates-actions-menu", title: "Templates, a built-in template's actions", area: tarea, from: "pages-templates")
        }
        await ui.step("template create") {
            try await ui.capAct(id: "template-create")
            let name = try await ui.pagesSheetElement("template-edit-sheet", role: "AXTextField")
            await ui.pagesCap("pages-templates-new", title: "Templates, create a template", area: tarea, from: "pages-templates", via: "template-create")
            await ui.pagesEnsure("pages-templates-type-menu") {
                await ui.capMenu(label: "Type", name: "pages-templates-type-menu", title: "Templates, the template type menu", area: tarea, from: "pages-templates-new")
            }
            try await ui.pagesTypeInto(name, "Short night shoot call sheet")
            await ui.idle(0.5)
            await ui.pagesCap("pages-templates-new-filled", title: "Templates, a new template filled in", area: tarea, from: "pages-templates-new", via: "template-name")
            try await ui.pagesSheetPress("template-edit-sheet", "Create")
            try await ui.waitGone(id: "template-edit-sheet", timeout: 8)
            await ui.idle(1.0)
            await ui.pagesCap("pages-templates-saved", title: "Templates, a template of your own", area: tarea, from: "pages-templates-new-filled", via: "template-save")
        }
        await ui.step("own template") {
            guard let menu = ui.all().first(where: { $0.identifier.hasPrefix("template-actions-") && !$0.identifier.hasPrefix("template-actions-builtin-") && $0.frame.width > 0 }) else { throw DriverError("no own template") }
            await ui.pagesMenu(id: menu.identifier, name: "pages-templates-own-menu", title: "Templates, your template's actions", area: tarea, from: "pages-templates-saved")
            try await ui.capMenuButton(id: menu.identifier, item: "Edit")
            _ = try await ui.pagesSheetElement("template-edit-sheet", role: "AXTextField")
            await ui.pagesCap("pages-templates-edit", title: "Templates, edit a template", area: tarea, from: "pages-templates-own-menu", via: "template-menu-edit")
            try await ui.pagesSheetPress("template-edit-sheet", "Cancel")
            try await ui.waitGone(id: "template-edit-sheet", timeout: 8)
            try await ui.capMenuButton(id: menu.identifier, item: "Delete")
            await ui.idle(0.8)
            await ui.pagesCap("pages-templates-delete-confirm", title: "Templates, delete a template asks first", area: tarea, from: "pages-templates-own-menu", via: "template-menu-delete")
            if await ui.tryPress(id: "template-delete-cancel", wait: 2) {} else { _ = await ui.tryPress(label: "Cancel", wait: 2) }
            await ui.idle(0.5)
        }
        await ui.step("template apply") {
            guard let card = ui.all().first(where: { $0.identifier.hasPrefix("template-card-builtin-") && $0.label.lowercased().contains("call sheet") && $0.frame.width > 0 })
                ?? ui.pagesFirst("template-card-builtin-") else { throw DriverError("no built-in template") }
            try await ui.press(card)
            _ = try await ui.wait(id: "template-preview-sheet", timeout: 8)
            let skip = ["Close", "Edit", "Use in This Project", "Save to My Templates"]
            guard let create = ui.all().first(where: { $0.identifier == "template-preview-sheet" && $0.role == "AXButton" && !skip.contains($0.label) }) else { throw DriverError("no create button") }
            try await ui.press(create)
            _ = try await ui.wait(id: "template-apply-sheet", timeout: 8)
            await ui.pagesCap("pages-templates-apply", title: "Templates, create a document from a template", area: tarea, from: "pages-templates-preview", via: "template-preview-create")
            try await ui.pagesSheetPress("template-apply-sheet", "Cancel")
            await ui.idle(0.6)
        }
        await ui.capRecover()
        await ui.step("templates from project home") {
            try await ui.capNav("Home")
            try await ui.capAct(label: "Templates")
            _ = try await ui.wait(id: "project-settings-templates", timeout: 8)
            await ui.idle(0.8)
            await ui.pagesCap("pages-templates-from-home", title: "Templates opened from Project Home", area: tarea, from: "project-home", via: "Templates")
        }
    }

    // MARK: Recovery and a project that cannot be found

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
                // The project opens where it was left (the Screenplay), or the recovery question comes first.
                try await ui.capWaitUntil("the project window", timeout: 30) { ui.exists(id: "sidebar-home") || ui.exists(id: "project-home") || ui.windows.contains { $0.sheetParent != nil } }
                let offered = await ui.recoveryOffered(within: 10)
                await ui.capture("recovery-alert", title: "Recovery question after the app closed unexpectedly", area: "Recovery", from: "home-after-crash", via: "project-card-Night Bus")
                if offered { try? await ui.press(label: "Keep the latest work", role: "AXButton") }
                await ui.capRecover()
                await ui.idle(0.8)
                await ui.capture("pages-recovery-kept", title: "Project Home after keeping the latest work", area: "Recovery", from: "recovery-alert", via: "Keep the latest work")
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
