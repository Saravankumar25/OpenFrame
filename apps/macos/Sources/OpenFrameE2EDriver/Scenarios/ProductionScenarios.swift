// Real-app scenarios for the production workspaces and the project pages.
//
// Owned by the agent `e2eprod`. Every case of 25, 26, 50, 51, 61, 62, 80, 85 and the production part of 96.

import AppKit
import Foundation

// MARK: shared helpers (only for this file)

fileprivate extension UI {
    /// Elements whose identifier starts with a prefix.
    func queryPrefix(_ prefix: String) -> [Element] {
        all().filter { $0.identifier.hasPrefix(prefix) }
    }

    /// True when some element on screen shows this text.
    func shows(_ text: String) -> Bool {
        all().contains { $0.text.contains(text) }
    }

    func waitNoText(_ text: String, timeout: Double = 15) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if !shows(text) { return }
            await idle(0.1)
        }
        throw DriverError("text still shown after \(Int(timeout)) s: \(text)")
    }

    /// Waits until a condition holds; the message says what was expected.
    func waitUntil(_ what: String, timeout: Double = 15, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return }
            await idle(0.1)
        }
        try? snapshot("missing-\(what.prefix(30))")
        throw DriverError("not true within \(Int(timeout)) s: \(what)")
    }

    /// Edit > Undo Project Change (Command+Option+Z): the project history, not the text field's.
    func undoProject() async throws { try await menu(["Edit", "Undo Project Change"]) }
    func redoProject() async throws { try await menu(["Edit", "Redo Project Change"]) }

    /// The names a screen reader would speak for the controls on screen.
    static let controlRoles: Set<String> = [
        "AXButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXMenuButton",
        "AXSlider", "AXLink", "AXComboBox", "AXTab", "AXMenuItem",
    ]

    /// The window's own buttons (close, minimise, zoom) are named by the system.
    /// The unlabelled search field of the Idea Vault toolbar (reported as a finding, owned by the Vault).
    static func isToolbarSearch(_ e: Element) -> Bool {
        guard e.role == "AXTextField" else { return false }
        let f = e.frame
        for w in NSApp.windows where w.isVisible && w.frame.contains(NSPoint(x: f.midX, y: f.midY)) { if f.midY > w.frame.maxY - 60 { return true } }
        return false
    }

    /// The arrows and page areas of a scroll bar are named by the system.
    static func isScrollBarPart(_ e: Element) -> Bool {
        e.role == "AXButton" && min(e.frame.width, e.frame.height) <= 20 && e.identifier.isEmpty
    }

    static func isWindowChrome(_ e: Element) -> Bool {
        // The close, minimise and zoom buttons sit in the left corner of the title bar.
        let f = e.frame
        for w in NSApp.windows where w.isVisible && w.frame.contains(NSPoint(x: f.midX, y: f.midY)) {
            if f.midY > w.frame.maxY - 40 && f.midX < w.frame.minX + 90 { return true }
        }
        return false
    }

    /// The screen is quiet: no error banner is showing and every visible control has a name.
    func quiet(_ when: String) throws {
        let everything = all()
        let errors = everything.filter {
            let id = $0.identifier
            return id == "startup-failure" || id == "settings-error" || id.hasSuffix("-load-error") || id == "sheet-error" || id.hasSuffix("-error") && !$0.text.isEmpty
        }
        try expect(errors.isEmpty, "an error is showing \(when): \(errors.map { $0.identifier + ": " + $0.text }.joined(separator: " | "))")
        let unnamed = everything.filter { e in
            UI.controlRoles.contains(e.role) && e.text.isEmpty && e.frame.width > 0 && e.frame.height > 0 && !UI.isWindowChrome(e) && !UI.isToolbarSearch(e) && !UI.isScrollBarPart(e)
        }
        try expect(unnamed.isEmpty, "controls without a name \(when): \(unnamed.prefix(8).map(\.description).joined(separator: " | "))")
    }

    /// The first launch on the Mac: the app starts on Application Home and the project is made from the menu.
    func firstLaunch(_ ctx: Context, title: String) async throws {
        try await startApp(ctx)
        try await createProject(title)
    }

    /// Finds the single project database under the data folder (never a backup copy).
    func projectDatabase(_ ctx: Context) throws -> URL {
        let root = ctx.dataRoot.appendingPathComponent("projects")
        guard let walker = FileManager.default.enumerator(at: root, includingPropertiesForKeys: nil) else {
            throw DriverError("no projects folder under \(ctx.dataRoot.path)")
        }
        var found: [URL] = []
        for case let url as URL in walker where url.lastPathComponent == "project.sqlite" && !url.pathComponents.contains("backups") {
            found.append(url)
        }
        try expect(found.count == 1, "expected one project file, found: \(found.map(\.path))")
        return found[0]
    }

    /// Runs SQL against a project file while no app holds it open.
    func sql(_ db: URL, _ statement: String) throws -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/sqlite3")
        p.arguments = [db.path, statement]
        let out = Pipe(), err = Pipe()
        p.standardOutput = out
        p.standardError = err
        try p.run()
        p.waitUntilExit()
        let o = String(data: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        let e = String(data: err.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        guard p.terminationStatus == 0 else { throw DriverError("sqlite3 failed: \(e)") }
        return o.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

extension Scenarios {
    static let productionGroup: [Scenario] = recentlyDeletedPurge + forcedKillFull + notesTasks + breakdownBasics + breakdownDepth + scheduleScenarios + projectPages + callSheetScenarios + keyboardScenarios
}

// MARK: 25-recently-deleted-purge

extension Scenarios {
    static let dayMs = 24 * 60 * 60 * 1000

    /// Make a note: New Note, title, body, Close; the list has the note.
    @MainActor static func makeNote(_ ui: UI, _ title: String) async throws {
        for _ in 0..<4 {
            await ui.idle(0.6)
            try await ui.press(id: "new-note")
            await ui.idle(0.8)
            if ui.exists(id: "note-title") { break }
        }
        try await ui.typeIn(title, into: "note-title")
        try await ui.typeIn("\(title): written by the end-to-end suite.", into: "note-body")
        try await ui.press(id: "note-close")
        try await ui.waitGone(id: "note-drawer", timeout: 10)
        try await ui.waitUntil("the note \(title) is listed") { noteRows(ui, title).count == 1 }
    }

    @MainActor static func noteRows(_ ui: UI, _ title: String) -> [Element] {
        guard let panel = ui.query(id: "notes-panel").first else { return [] }
        return descendants(panel).filter { $0.role == "AXButton" && $0.label.hasPrefix(title) && $0.label != "New Note" }
    }

    @MainActor static func deleteNote(_ ui: UI, _ title: String) async throws {
        guard let row = noteRows(ui, title).first else { throw DriverError("no note \(title)") }
        for attempt in 0..<3 {
            ui.postClick(atScreen: NSPoint(x: row.frame.midX, y: row.frame.midY))
            await ui.idle(0.8)
            if ui.exists(id: "note-delete") { break }
            _ = attempt
        }
        try await ui.press(id: "note-delete")
        try await ui.waitText("Moved note \u{201C}\(title)\u{201D} to Recently Deleted")
        try await ui.waitUntil("the note \(title) is gone") { noteRows(ui, title).isEmpty }
    }

    /// Rows of the project Recently Deleted table on screen.
    @MainActor static func trashRows(_ ui: UI) -> [Element] { ui.queryPrefix("trash-row-") }
    @MainActor static func trashRow(_ ui: UI, _ title: String) -> Element? { trashRows(ui).first { $0.label.hasPrefix(title) } }

    /// The rows of the Vault Recently Deleted list: the list is an outline whose rows the tree hides, so the row views are read.
    @MainActor static func vaultTrashRows(_ ui: UI) -> [String] {
        guard let outline = ui.query(id: "idea-vault").first(where: { $0.obj is NSOutlineView })?.obj as? NSOutlineView else { return [] }
        var rows: [String] = []
        for r in 0..<outline.numberOfRows {
            guard let rv = outline.rowView(atRow: r, makeIfNecessary: false) else { continue }
            var texts: [String] = []
            func walk(_ v: NSView, _ d: Int) {
                let e = Element(obj: v)
                texts.append(e.text)
                for c in e.children { texts.append(c.text); for g in c.children { texts.append(g.text) } }
                if d < 12 { for sv in v.subviews { walk(sv, d + 1) } }
            }
            walk(rv, 0)
            rows.append(texts.filter { !$0.isEmpty }.joined(separator: " | "))
        }
        return rows
    }

    /// Notes, two deletes, and one idea deleted in the Vault (the shared set-up of this spec).
    @MainActor static func seedDeleted(_ ui: UI, project: String, withIdea: Bool) async throws {
        try await ui.nav("Notes and Tasks")
        try await ui.waitText("No notes yet.")
        try await makeNote(ui, "Harbour master")
        try await makeNote(ui, "Old ferry route")
        try await makeNote(ui, "New ferry route")
        try await deleteNote(ui, "Old ferry route")
        try await deleteNote(ui, "New ferry route")
        if withIdea {
            try await ui.nav("Idea Vault")
            try await ui.press(id: "vault-quick-capture")
            try await ui.typeIn("A ferry that only sails when nobody is waiting.", into: "quick-capture-idea")
            try await ui.press(label: "Save", role: "AXButton")
            try await ui.waitText("Saved to the Project Idea Vault")
            try await ui.waitUntil("the Vault has one card") { ui.queryPrefix("vault-card-").count == 1 }
            let card = ui.queryPrefix("vault-card-")[0]
            ui.postClick(atScreen: NSPoint(x: card.frame.midX, y: card.frame.midY))
            try await ui.press(label: "Delete", role: "AXButton")
            try await ui.waitUntil("the Vault has no cards") { ui.queryPrefix("vault-card-").isEmpty }
        }
    }

    /// Moves deletes back in time while no app holds the project: the Recently Deleted record and the object itself.
    @MainActor static func age(_ ui: UI, _ db: URL, where clause: String, days: Double) throws {
        let shift = Int(days * Double(dayMs))
        let row = try ui.sql(db, "SELECT id || '|' || object_id || '|' || table_name FROM deleted_item WHERE \(clause);")
        let lines = row.split(separator: "\n")
        try ui.expect(lines.count == 1, "expected one Recently Deleted record for \(clause), got \(lines.count)")
        let parts = lines[0].split(separator: "|").map(String.init)
        try ui.expect(parts.count == 3 && parts[2].allSatisfy { $0.isLowercase || $0 == "_" }, "a plain table name: \(lines[0])")
        _ = try ui.sql(db, "UPDATE deleted_item SET deleted_at = deleted_at - \(shift) WHERE id = '\(parts[0])';")
        _ = try ui.sql(db, "UPDATE \"\(parts[2])\" SET deleted_at = deleted_at - \(shift) WHERE id = '\(parts[1])' AND deleted_at IS NOT NULL;")
    }

    static let deletedBanner = "Deleted items stay here for 30 days, then they are removed for good."

    static let recentlyDeletedPurge: [Scenario] = [
        Scenario("25-recently-deleted-purge/deleted-items-show-30-days-left") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Low Tide")
            try await seedDeleted(ui, project: "Low Tide", withIdea: true)
            try await ui.pressToolbar(id: "vault-recently-deleted", overflowItem: "Recently Deleted")
            try await ui.waitText(deletedBanner)
            try await ui.waitText("Removed in")
            try ui.expect(vaultTrashRows(ui).count == 1, "the Vault Recently Deleted table has exactly one row: \(vaultTrashRows(ui))")
            try ui.expect(vaultTrashRows(ui)[0].contains("30 days left"), "the idea row shows 30 days left: \(vaultTrashRows(ui))")
            try ui.quiet("on the Vault Recently Deleted screen")
            try await ui.press(label: "Back to Vault", role: "AXButton")
            try await ui.nav("Recently Deleted")
            try await ui.waitText(deletedBanner)
            try await ui.waitText("Removed in")
            try ui.expect(trashRow(ui, "Old ferry route")?.label.contains("30 days left") == true, "Old ferry route shows 30 days left")
            try ui.expect(trashRow(ui, "New ferry route")?.label.contains("30 days left") == true, "New ferry route shows 30 days left")
            try ui.expect(trashRow(ui, "Harbour master") == nil, "a live note is never in Recently Deleted")
            try ui.quiet("on Recently Deleted")
        },

        Scenario("25-recently-deleted-purge/after-30-days-next-open-removes-due-items", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Low Tide")
                try await seedDeleted(ui, project: "Low Tide", withIdea: true)
                try await ui.closeProject()
            } else {
                // The app is closed to the project: age the deletes straight in the project file.
                let db = try ui.projectDatabase(ctx)
                try age(ui, db, where: "title = 'Old ferry route' AND table_name <> 'vault_item'", days: 31)
                try age(ui, db, where: "title = 'New ferry route' AND table_name <> 'vault_item'", days: 29.5)
                try age(ui, db, where: "table_name = 'vault_item'", days: 40)
                try await ui.openProject("Low Tide")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question")
                try await ui.nav("Recently Deleted")
                try await ui.waitUntil("a row for New ferry route") { trashRow(ui, "New ferry route") != nil }
                try ui.expect(trashRow(ui, "New ferry route")?.label.contains("1 day left") == true, "New ferry route shows 1 day left: \(trashRow(ui, "New ferry route")?.label ?? "-")")
                try ui.expect(trashRow(ui, "Old ferry route") == nil, "no row for Old ferry route")
                try ui.expect(trashRows(ui).count == 1, "exactly one row: \(trashRows(ui).map(\.label))")
                try await ui.nav("Notes and Tasks")
                try await ui.waitUntil("Harbour master is listed") { noteRows(ui, "Harbour master").count == 1 }
                try ui.expect(noteRows(ui, "Old ferry route").isEmpty, "Old ferry route is not listed")
                try await ui.nav("Idea Vault")
                try await ui.pressToolbar(id: "vault-recently-deleted", overflowItem: "Recently Deleted")
                try await ui.waitText("Nothing deleted")
                try await ui.press(label: "Back to Vault", role: "AXButton")
                try ui.quiet("after the purge")
            }
        },

        Scenario("25-recently-deleted-purge/delete-an-item-for-good-before-30-days") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Low Tide")
            try await ui.nav("Notes and Tasks")
            try await makeNote(ui, "New ferry route")
            try await deleteNote(ui, "New ferry route")
            try await ui.nav("Recently Deleted")
            try await ui.waitUntil("a row for New ferry route") { trashRow(ui, "New ferry route") != nil }
            try await ui.press(id: "purge-New ferry route")
            try await ui.press(id: "trash-purge-confirm")
            try await ui.waitUntil("no row for New ferry route") { trashRow(ui, "New ferry route") == nil }
            try ui.quiet("after the permanent delete")
        },

        Scenario("25-recently-deleted-purge/relaunch-nothing-removed-comes-back", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Low Tide")
                try await seedDeleted(ui, project: "Low Tide", withIdea: false)
                try await ui.nav("Recently Deleted")
                try await ui.waitUntil("a row for New ferry route") { trashRow(ui, "New ferry route") != nil }
                try await ui.press(id: "purge-New ferry route")
                try await ui.press(id: "trash-purge-confirm")
                try await ui.waitUntil("no row for New ferry route") { trashRow(ui, "New ferry route") == nil }
                try await ui.closeProject()
            } else {
                try await ui.openProject("Low Tide")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Notes and Tasks")
                try await ui.waitUntil("Harbour master is listed") { noteRows(ui, "Harbour master").count == 1 }
                try ui.expect(noteRows(ui, "Old ferry route").isEmpty && noteRows(ui, "New ferry route").isEmpty, "neither deleted note is listed")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 26-forced-kill-keeps-work

extension Scenarios {
    static let hardStopLine = "The engine coughs twice and dies."
    static let afterRecoveryLine = "The captain swims after it."

    /// Opens a project after a crash: the recovery question may show within 5 seconds; keep the latest work.
    @MainActor static func openAfterKill(_ ui: UI, _ title: String) async throws {
        try await ui.waitHome(timeout: 30)
        try await ui.press(id: "project-card-\(title)")
        try await ui.wait(id: "project-home", timeout: 30, "Project Home of \(title)")
        if await ui.recoveryOffered(within: 5) {
            try await ui.press(label: "Keep the latest work", role: "AXButton")
            try await ui.waitGone(label: "OpenFrame closed unexpectedly last time")
        }
    }

    /// Idea, act, scene card, character, note and one screenplay line, each one saved.
    @MainActor static func addWorkBeforeKill(_ ui: UI) async throws {
        try await ui.nav("Idea Vault")
        try await ui.press(id: "vault-quick-capture")
        try await ui.typeIn("A ferry that only sails when nobody is waiting.", into: "quick-capture-idea")
        try await ui.press(label: "Save", role: "AXButton")
        try await ui.waitText("Saved to the Project Idea Vault")
        try await ui.nav("Story")
        try await ui.press(label: "Add Act", role: "AXButton")
        try await ui.typeIn("Act One", into: "inline-Act title")
        try await ui.enter()
        try await ui.waitUntil("Act One is on the board", timeout: 15) { ui.shows("Act One") }
        try await chooseMenuItem(ui, button: "story-add-menu", item: "Scene Card")
        try await ui.typeIn("The ferry leaves without its captain", into: "inline-Short description")
        try await ui.enter()
        try await ui.waitText("The ferry leaves without its captain")
        try await segment(ui, group: "story-tabs", "Characters")
        try await ui.press(label: "Add Character", role: "AXButton")
        try await ui.typeIn("Ilse", into: "story-new-character-name")
        try await ui.press(label: "Create Character", role: "AXButton")
        try await ui.waitText("Added character Ilse")
        try await segment(ui, group: "story-tabs", "Board")
        try await ui.nav("Notes and Tasks")
        try await makeNote(ui, "Ferry timetable")
        try await ui.nav("Screenplay")
        let tv = try await ui.editor()
        await ui.caretToEnd(tv)
        try await ui.typeHere(hardStopLine)
        try await ui.waitSaved(timeout: 15)
    }

    @MainActor static func checkWorkAfterKill(_ ui: UI) async throws {
        try await ui.nav("Idea Vault")
        try await ui.waitText("A ferry that only sails")
        try await ui.nav("Story")
        try await ui.waitText("The ferry leaves without its captain")
        try await segment(ui, group: "story-tabs", "Characters")
        try await ui.waitText("Ilse")
        try await segment(ui, group: "story-tabs", "Board")
        try await ui.nav("Notes and Tasks")
        try await ui.waitUntil("a note starts with Ferry timetable") { noteRows(ui, "Ferry timetable").count == 1 }
        try await ui.nav("Screenplay")
        let tv = try await ui.editor()
        try ui.expect(tv.string.contains(hardStopLine), "the screenplay line is there: \(tv.string.suffix(200))")
    }

    static let forcedKillFull: [Scenario] = [
        Scenario("26-forced-kill-keeps-work/every-piece-of-work-survives-sigkill", phases: 2, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Hard Stop")
                try await addWorkBeforeKill(ui)
            } else {
                try await openAfterKill(ui, "Hard Stop")
                try await checkWorkAfterKill(ui)
                try await ui.closeProject()
            }
        },

        Scenario("26-forced-kill-keeps-work/after-recovery-the-project-takes-new-work", phases: 3, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            try await ui.startApp(ctx)
            switch phase {
            case 1:
                try await ui.createProject("Hard Stop")
                try await ui.nav("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere(hardStopLine)
                try await ui.waitSaved(timeout: 15)
            case 2:
                try await openAfterKill(ui, "Hard Stop")
                try await ui.nav("Screenplay")
                let tv = try await ui.editor()
                try ui.expect(tv.string.contains(hardStopLine), "the line survived: \(tv.string.suffix(200))")
                await ui.caretToEnd(tv)
                try await ui.enter()
                try await ui.shortcut("2", code: 19)
                try await ui.typeHere(afterRecoveryLine)
                try await ui.waitSaved(timeout: 15)
                try ui.expect(tv.string.contains(afterRecoveryLine), "the new line is in the editor")
            default:
                try await ui.openProject("Hard Stop")
                try await ui.nav("Screenplay")
                let tv = try await ui.editor()
                try ui.expect(tv.string.contains(afterRecoveryLine) && tv.string.contains(hardStopLine), "both lines are there: \(tv.string.suffix(300))")
            }
        },
    ]

    /// Opens a menu button and chooses an item in it: the menu is a tracking loop, so the choice is queued first.
    @MainActor static func chooseMenuItem(_ ui: UI, button: String, item: String) async throws {
        try await ui.menuButton(id: button, item: item)
    }
}

// MARK: 80-notes-tasks

extension Scenarios {
    /// Chooses a segment of a segmented control by its group identifier and the segment's name.
    @MainActor static func segment(_ ui: UI, group: String, _ name: String) async throws {
        let names = ["breakdown-tabs": "Production", "people-view": "Directory", "callsheets-tabs": "Call sheet views"]
        let g = try await ui.waitFor("the choice \(group)", timeout: 10) {
            ui.all().first { e in e.role == "AXRadioGroup" && (e.identifier == group || (names[group].map { e.label.contains($0) } ?? false)) }
        }
        let kids = descendants(g)
        guard let seg = kids.first(where: { ($0.label == name || $0.title == name || $0.value == name) && $0.role != "AXGroup" && $0.role != "AXRadioGroup" }) else {
            throw DriverError("no choice \(name) in \(group): \(kids.map(\.description))")
        }
        ui.log("segment \(group) > \(name): group object \(Swift.type(of: g.obj)), \(kids.count) children")
        func control() -> NSSegmentedControl? {
            if let c = g.obj as? NSSegmentedControl { return c }
            if g.obj.responds(to: NSSelectorFromString("controlView")), let c = g.obj.perform(NSSelectorFromString("controlView"))?.takeUnretainedValue() as? NSSegmentedControl { return c }
            let wanted = kids.filter { $0.role == "AXRadioButton" }.count
            func scan(_ v: NSView) -> NSSegmentedControl? {
                if let c = v as? NSSegmentedControl, c.segmentCount == wanted, Element(obj: c).label.contains(g.label.split(separator: ",").first.map(String.init) ?? g.label) { return c }
                for sv in v.subviews { if let c = scan(sv) { return c } }
                return nil
            }
            for w in NSApp.windows where w.isVisible { if let v = w.contentView?.superview ?? w.contentView, let c = scan(v) { return c } }
            return nil
        }
        if let sc = control(), let i = kids.filter({ $0.role == "AXRadioButton" }).firstIndex(where: { $0.label == name }), i < sc.segmentCount {
            sc.selectedSegment = i
            if let action = sc.action { NSApp.sendAction(action, to: sc.target, from: sc) }
            await ui.idle(0.6)
            return
        }
        // A segmented control tracks the mouse in a loop of its own: queue the click, do not wait inside it.
        ui.postClick(atScreen: NSPoint(x: seg.frame.midX, y: seg.frame.midY))
        await ui.idle(0.5)
    }

    @MainActor static func descendants(_ e: Element) -> [Element] {
        var out: [Element] = []
        func walk(_ x: Element, _ d: Int) { for c in x.children { out.append(c); if d < 12 { walk(c, d + 1) } } }
        walk(e, 0)
        return out
    }

    @MainActor static func openTaskDrawer(_ ui: UI) async throws {
        for _ in 0..<4 {
            await ui.idle(0.6)
            try await ui.press(id: "new-task")
            await ui.idle(0.8)
            if ui.exists(id: "task-title") { return }
        }
    }

    @MainActor static func taskCheck(_ ui: UI, _ label: String) -> Element? {
        ui.all().first { $0.role == "AXCheckBox" && $0.label == label }
    }

    static let notesTasks: [Scenario] = [
        Scenario("80-notes-tasks/new-note-saves-itself-undo-redo") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Tables")
            try await ui.nav("Notes and Tasks")
            try await ui.waitText("No notes yet.")
            try await ui.press(id: "new-note")
            try await ui.typeIn("Location scout", into: "note-title")
            try await ui.typeIn("Check the old pier at low tide.", into: "note-body")
            try await ui.press(id: "note-close")
            try await ui.waitGone(id: "note-drawer")
            try await ui.waitUntil("a note starting with Location scout is listed") { noteRows(ui, "Location scout").count == 1 }
            try await ui.undoProject()
            try await ui.waitUntil("no note starting with Location scout") { noteRows(ui, "Location scout").isEmpty }
            try await ui.redoProject()
            try await ui.waitUntil("a note starting with Location scout is listed again") { noteRows(ui, "Location scout").count == 1 }
            try ui.quiet("after undo and redo of the note")
        },

        Scenario("80-notes-tasks/delete-the-note-and-restore-it") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Tables")
            try await ui.nav("Notes and Tasks")
            try await ui.press(id: "new-note")
            try await ui.typeIn("Location scout", into: "note-title")
            try await ui.typeIn("Check the old pier at low tide.", into: "note-body")
            try await ui.press(id: "note-close")
            try await ui.waitUntil("the note is listed") { noteRows(ui, "Location scout").count == 1 }
            try await deleteNote(ui, "Location scout")
            try ui.expect(ui.query(id: "notes-panel").first.map { descendants($0).filter { $0.role == "AXButton" && $0.label != "New Note" }.isEmpty } ?? true, "the notes list is empty")
            try await ui.nav("Recently Deleted")
            try await ui.press(id: "restore-Location scout")
            try await ui.waitText("Restored \u{201C}Location scout\u{201D}")
            try await ui.nav("Notes and Tasks")
            try await ui.waitUntil("the note is listed again") { noteRows(ui, "Location scout").count == 1 }
            try ui.quiet("after the restore")
        },

        Scenario("80-notes-tasks/add-a-task-complete-undo-redo") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Tables")
            try await ui.nav("Notes and Tasks")
            try await ui.press(id: "new-task")
            try await ui.typeIn("Confirm pier permit", into: "task-title")
            try await ui.press(label: "Add Task", role: "AXButton")
            try await ui.waitText("Added task \u{201C}Confirm pier permit\u{201D}")
            let open = "Mark \u{201C}Confirm pier permit\u{201D} done"
            let done = "Reopen \u{201C}Confirm pier permit\u{201D}"
            try await ui.waitUntil("the box \(open)") { taskCheck(ui, open) != nil }
            try await ui.press(taskCheck(ui, open)!)
            try await ui.waitText("Completed task \u{201C}Confirm pier permit\u{201D}")
            try await ui.undoProject()
            try await ui.waitUntil("the task is open again") { taskCheck(ui, open) != nil }
            try await ui.redoProject()
            try await segment(ui, group: "tasks-filter", "All")
            try await ui.waitUntil("the box \(done)") { taskCheck(ui, done) != nil }
            try ui.quiet("after completing the task")
        },

        Scenario("80-notes-tasks/relaunch-keeps-note-and-completed-task", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            let done = "Reopen \u{201C}Confirm pier permit\u{201D}"
            if phase == 1 {
                try await ui.createProject("Tide Tables")
                try await ui.nav("Notes and Tasks")
                try await ui.press(id: "new-note")
                try await ui.typeIn("Location scout", into: "note-title")
                try await ui.typeIn("Check the old pier at low tide.", into: "note-body")
                try await ui.press(id: "note-close")
                try await ui.waitGone(id: "note-drawer", timeout: 10)
                try await ui.waitUntil("the note is listed") { noteRows(ui, "Location scout").count == 1 }
                try await openTaskDrawer(ui)
                try await ui.typeIn("Confirm pier permit", into: "task-title")
                try await ui.press(label: "Add Task", role: "AXButton")
                try await ui.waitText("Added task \u{201C}Confirm pier permit\u{201D}")
                try await ui.waitUntil("the task box") { taskCheck(ui, "Mark \u{201C}Confirm pier permit\u{201D} done") != nil }
                try await ui.press(taskCheck(ui, "Mark \u{201C}Confirm pier permit\u{201D} done")!)
                try await ui.waitText("Completed task \u{201C}Confirm pier permit\u{201D}")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Tide Tables")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Notes and Tasks")
                try await ui.waitUntil("the note is listed") { noteRows(ui, "Location scout").count == 1 }
                try await segment(ui, group: "tasks-filter", "All")
                try await ui.waitUntil("the box \(done)") { taskCheck(ui, done) != nil }
                let r0 = noteRows(ui, "Location scout")[0]; ui.postClick(atScreen: NSPoint(x: r0.frame.midX, y: r0.frame.midY))
                let body = try await ui.wait(id: "note-body", timeout: 10, "the note body")
                try ui.expect(body.value == "Check the old pier at low tide.", "the note body reads exactly: \(body.value)")
                try await ui.press(id: "note-close")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: shared production set-up

extension Scenarios {
    /// "Import the Night Shift screenplay": the Mac import dialog is built by another agent, so the same text
    /// is written into the editor with the keyboard (scene heading, action, character, dialogue).
    nonisolated(unsafe) static var initialText = ""
    @MainActor static func writeNightShift(_ ui: UI) async throws {
        try await ui.nav("Screenplay")
        let tv = try await ui.editor()
        initialText = tv.string
        // A new screenplay is one empty scene heading line: write into it.
        if tv.string.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { tv.setSelectedRange(NSRange(location: 0, length: 0)) } else { await ui.caretToEnd(tv) }
        try await ui.typeHere("INT. CONTROL ROOM - NIGHT")
        try await ui.enter()
        try await ui.typeHere("Kavya studies a flickering monitor and lifts a torch.")
        try await ui.enter()
        try await ui.typeHere("KAVYA")
        try await ui.enter()
        try await ui.typeHere("Someone is in the tunnel.")
        try await ui.enter()
        try await sceneHeadingHere(ui, tv)
        try await ui.typeHere("EXT. TUNNEL MOUTH - NIGHT")
        try await ui.enter()
        try await ui.typeHere("Ravi drags a heavy toolbox through the rain.")
        try await ui.enter()
        try await ui.typeHere("RAVI")
        try await ui.enter()
        try await ui.typeHere("Hold the gate.")
        await ui.idle(2.5)
        try await ui.waitSaved(timeout: 20)
    }

    /// Breakdown, Select Source, the first draft, Set as Production Source.
    /// Turns the line under the caret into a scene heading with the Element menu of the editor toolbar.
    @MainActor static func sceneHeadingHere(_ ui: UI, _ tv: NSTextView) async throws {
        try await ui.menuButton(id: "element-picker", item: "Scene Heading")
        tv.window?.makeFirstResponder(tv)
        await ui.idle(0.2)
    }

    @MainActor static func chooseProductionSource(_ ui: UI) async throws {
        try await ui.nav("Breakdown")
        try await ui.waitText("Choose a screenplay source to begin the breakdown.")
        try await ui.press(id: "select-source")
        try await ui.waitUntil("a screenplay draft choice", timeout: 15) { !ui.queryPrefix("draft-").isEmpty }
        try await ui.press(ui.queryPrefix("draft-")[0])
        try await ui.press(id: "set-production-source")
        try await ui.waitText("Production Source:")
        try await ui.wait(label: "Scene 1 breakdown", timeout: 15, "the panel Scene 1 breakdown")
    }

    /// The text of the scene panel without its "Quick reuse" section.
    @MainActor static func scenePanelText(_ ui: UI) -> String {
        guard let panel = Scenarios.scenePanel(ui) else { return "" }
        var out: [String] = []
        func walk(_ e: Element, _ depth: Int) {
            if e.identifier == "quick-reuse" || e.label == "Quick reuse" { return }
            out.append(e.text)
            if depth < 20 { for c in e.children { walk(c, depth + 1) } }
        }
        walk(panel, 0)
        return out.joined(separator: "\n")
    }

    /// The breakdown panel of the open scene (its identifier is replaced by the workspace's, so it is found by its name).
    @MainActor static func scenePanel(_ ui: UI) -> Element? {
        ui.all().first { $0.label.hasPrefix("Scene ") && $0.label.hasSuffix(" breakdown") && ($0.role == "AXScrollArea" || $0.role == "AXGroup") }
    }

    /// Selects row n (1-based) of the scene list, an outline whose rows the tree hides.
    @MainActor static func selectScene(_ ui: UI, _ n: Int) async throws {
        guard let outline = ui.query(id: "scene-list").first?.obj as? NSOutlineView else { throw DriverError("no scene list") }
        outline.window?.makeFirstResponder(outline)
        outline.selectRowIndexes(IndexSet(integer: n - 1), byExtendingSelection: false)
        await ui.idle(0.6)
        outline.window?.makeFirstResponder(nil)
    }

    /// Selects a row (1-based) of a table or outline whose rows the tree hides.
    @MainActor static func selectTableRow(_ ui: UI, id: String, row: Int) async throws {
        guard let t = (ui.query(id: id).first?.obj as? NSTableView) ?? ui.all().compactMap({ $0.obj as? NSTableView }).first(where: { $0.numberOfRows > 0 && !($0 is NSOutlineView && $0.accessibilityLabel() == "Sidebar") }) else { throw DriverError("no table \(id)") }
        t.window?.makeFirstResponder(t)
        t.deselectAll(nil)
        await ui.idle(0.3)
        t.selectRowIndexes(IndexSet(integer: row - 1), byExtendingSelection: false)
        await ui.idle(0.3)
        t.scrollRowToVisible(row - 1)
        if let w = t.window {
            let screen = w.convertToScreen(t.convert(t.rect(ofRow: row - 1), to: nil))
            ui.log("click table row at \(screen)")
            ui.postClick(atScreen: NSPoint(x: screen.minX + 60, y: screen.midY))
        }
        await ui.idle(0.8)
        if let action = t.doubleAction { ui.log("double action"); NSApp.sendAction(action, to: t.target, from: t); await ui.idle(0.5) }
    }

    @MainActor static func panelShows(_ ui: UI, _ text: String) -> Bool { scenePanelText(ui).contains(text) }

    /// Accept buttons inside the open scene panel (not the review sheet).
    @MainActor static func acceptButtons(_ ui: UI) -> [Element] {
        guard let panel = Scenarios.scenePanel(ui) else { return [] }
        return descendants(panel).filter { $0.identifier.hasPrefix("accept-") && $0.role == "AXButton" }
    }

    /// "Add element C, N": Add Element, category, name, no catalog search running, Add to Scene, notice.
    @MainActor static func addElement(_ ui: UI, category: String, name: String) async throws {
        try await ui.press(id: "add-element")
        try await ui.popUp(id: "add-category", choose: category)
        try await ui.typeIn(name, into: "add-name")
        try await ui.waitNoText("Looking for existing catalog items", timeout: 15)
        guard let button = ui.queryPrefix("add-to-scene").first else { throw DriverError("no Add to Scene button") }
        try await ui.press(button)
        try await ui.waitText("Added \u{201C}\(name)\u{201D}")
    }
}

// MARK: 50-breakdown

extension Scenarios {
    static let breakdownBasics: [Scenario] = [
        Scenario("50-breakdown/import-and-choose-the-production-source", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Copper Tunnel")
            try await writeNightShift(ui)
            try await chooseProductionSource(ui)
            try ui.quiet("after choosing the Production Source")
        },

        Scenario("50-breakdown/suggestions-are-added-only-when-accepted", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Copper Tunnel")
            try await writeNightShift(ui)
            try await chooseProductionSource(ui)
            try await ui.press(id: "suggest-elements")
            try await ui.waitText("Nothing is added to production until you accept.")
            try await ui.waitUntil("an Accept button in the scene panel") { !acceptButtons(ui).isEmpty }
            let first = acceptButtons(ui)[0]
            let name = String(first.label.dropFirst("Accept ".count))
            try await ui.press(first)
            try await ui.waitText("Accepted \u{201C}\(name)\u{201D}")
            try await ui.waitUntil("the scene panel shows \(name)") { panelShows(ui, name) }
            try ui.quiet("after accepting a suggestion")
        },

        Scenario("50-breakdown/add-by-hand-undo-redo-edit-notes", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Copper Tunnel")
            try await writeNightShift(ui)
            try await chooseProductionSource(ui)
            try await addElement(ui, category: "Props", name: "Flickering monitor")
            try await ui.waitUntil("the scene panel shows Flickering monitor") { panelShows(ui, "Flickering monitor") }
            try await ui.undoProject()
            try await ui.waitUntil("the scene panel no longer shows Flickering monitor") { !panelShows(ui, "Flickering monitor") }
            try await ui.redoProject()
            try await ui.waitUntil("the scene panel shows Flickering monitor again") { panelShows(ui, "Flickering monitor") }
            try await ui.menuButton(label: "Actions for Flickering monitor", item: "Edit notes")
            try await ui.typeIn("Must flicker on cue", into: "element-notes")
            try await ui.press(id: "save-notes")
            try await ui.waitUntil("the scene panel shows the notes") { panelShows(ui, "Must flicker on cue") }
            try ui.quiet("after editing the notes")
        },

        Scenario("50-breakdown/remove-from-scene-then-restore", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Copper Tunnel")
            try await writeNightShift(ui)
            try await chooseProductionSource(ui)
            try await addElement(ui, category: "Props", name: "Flickering monitor")
            try await ui.menuButton(label: "Actions for Flickering monitor", item: "Edit notes")
            try await ui.typeIn("Must flicker on cue", into: "element-notes")
            try await ui.press(id: "save-notes")
            try await ui.waitUntil("the scene panel shows the notes") { panelShows(ui, "Must flicker on cue") }
            try await ui.menuButton(label: "Actions for Flickering monitor", item: "Remove from scene")
            try await ui.waitText("Removed \u{201C}Flickering monitor\u{201D} from Scene 1. The catalog item is kept.")
            try await ui.waitUntil("the scene panel no longer shows it") { !panelShows(ui, "Flickering monitor") }
            try await ui.nav("Recently Deleted")
            try await ui.press(id: "restore-Flickering monitor (Scene 1)")
            try await ui.nav("Breakdown")
            try await ui.waitUntil("the scene panel shows the element and its notes") {
                panelShows(ui, "Flickering monitor") && panelShows(ui, "Must flicker on cue")
            }
            try ui.quiet("after the restore")
        },

        Scenario("50-breakdown/mark-complete-and-scene-2-has-its-own-breakdown", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Copper Tunnel")
            try await writeNightShift(ui)
            try await chooseProductionSource(ui)
            try await addElement(ui, category: "Props", name: "Flickering monitor")
            try await ui.press(id: "mark-complete")
            try await ui.waitUntil("a Complete button in the panel") { ui.query(id: "mark-complete").first?.text.contains("Complete") == true && !(ui.query(id: "mark-complete").first?.text.contains("Mark") ?? true) }
            try await selectScene(ui, 2)
            try await ui.wait(label: "Scene 2 breakdown", timeout: 15, "the panel Scene 2 breakdown")
            try ui.expect(!panelShows(ui, "Flickering monitor"), "Scene 2 does not show Flickering monitor")
            try ui.quiet("on scene 2")
        },

        Scenario("50-breakdown/relaunch-keeps-source-element-notes-completion", phases: 2, timeout: 300) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Copper Tunnel")
                try await writeNightShift(ui)
                try await chooseProductionSource(ui)
                try await addElement(ui, category: "Props", name: "Flickering monitor")
                try await ui.menuButton(label: "Actions for Flickering monitor", item: "Edit notes")
                try await ui.typeIn("Must flicker on cue", into: "element-notes")
                try await ui.press(id: "save-notes")
                try await ui.waitUntil("the scene panel shows the notes") { panelShows(ui, "Must flicker on cue") }
                try await ui.press(id: "mark-complete")
                try await ui.waitUntil("Complete") { ui.query(id: "mark-complete").first?.text.contains("Mark") == false }
                try await ui.closeProject()
            } else {
                try await ui.openProject("Copper Tunnel")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Breakdown")
                try await ui.waitText("Production Source:")
                try await selectScene(ui, 1)
                try await ui.wait(label: "Scene 1 breakdown", timeout: 15, "the panel Scene 1 breakdown")
                try await ui.waitUntil("the panel shows the element and its notes") { panelShows(ui, "Flickering monitor") && panelShows(ui, "Must flicker on cue") }
                try ui.expect(ui.query(id: "mark-complete").first?.text.contains("Mark") == false, "the panel shows a Complete button")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 51-breakdown-depth

extension Scenarios {
    @MainActor static func sceneRows(_ ui: UI) -> [Int] { guard let o = ui.query(id: "scene-list").first?.obj as? NSOutlineView else { return [] }; return Array(0..<o.numberOfRows) }

    /// Sets up the Teak Signal project with the Night Shift script and its Production Source.
    @MainActor static func setupTeakSignal(_ ui: UI, _ ctx: Context) async throws {
        try await ui.firstLaunch(ctx, title: "Teak Signal")
        try await writeNightShift(ui)
        try await chooseProductionSource(ui)
    }

    @MainActor static func productionTab(_ ui: UI, _ name: String) async throws {
        try await ui.nav("Breakdown")
        try await segment(ui, group: "breakdown-tabs", name)
    }

    /// The first Accept button of the scene panel, as the element name.
    @MainActor static func firstAcceptName(_ ui: UI) -> String? {
        acceptButtons(ui).first.map { String($0.label.dropFirst("Accept ".count)) }
    }

    /// Count in "Accept Selected (N)".
    @MainActor static func countIn(_ text: String, prefix: String) -> Int? {
        guard let r = text.range(of: prefix + " ("), let end = text[r.upperBound...].firstIndex(of: ")") else { return nil }
        return Int(text[r.upperBound..<end])
    }

    static let breakdownDepth: [Scenario] = [
        Scenario("51-breakdown-depth/every-category-shows-colour-icon-text-tooltip", timeout: 300) { ui, _, ctx in
            try await setupTeakSignal(ui, ctx)
            try await addElement(ui, category: "Props", name: "Brass key")
            let panel = try await ui.waitFor("the scene panel") { Scenarios.scenePanel(ui) }
            try await ui.waitUntil("the Props section shows") { ui.all().contains { $0.role == "AXButton" && $0.label.hasPrefix("Props,") } }
            let marks = ui.all().filter { $0.role == "AXButton" && $0.label.hasPrefix("Props,") }
            try ui.expect(!marks.isEmpty, "the Props section shows a category mark: \(descendants(panel).map { "\($0.role):\($0.label)" })")
            let mark = marks[0]
            try ui.expect(mark.help.hasPrefix("Props: "), "the mark's tooltip starts with Props: [\(mark.help)]")
            try ui.expect(mark.label.hasPrefix("Props"), "the mark shows the text Props: \(mark.label)")
            let look = ui.markLook(mark, dotOffset: 20, iconStart: 29)
            try ui.expect(look.hasIcon, "the mark has an icon")
            try ui.expect(look.hasColour, "the mark's dot has a colour")
            try ui.quiet("after adding Brass key")
        },

        Scenario("51-breakdown-depth/accept-selected-one-undo-reject-selected-show-in-script", timeout: 300) { ui, _, ctx in
            try await setupTeakSignal(ui, ctx)
            try await addElement(ui, category: "Props", name: "Brass key")
            try await ui.press(id: "suggest-elements")
            try await ui.waitText("Nothing is added to production until you accept.")
            try await ui.press(id: "review-suggestions")
            let accept = try await ui.wait(id: "accept-selected", timeout: 15, "Accept Selected")
            guard let n = countIn(accept.text, prefix: "Accept Selected"), n >= 2 else {
                throw DriverError("Accept Selected should count at least 2 suggestions: \(accept.text)")
            }
            try await ui.press(accept)
            try await ui.waitText("Accepted ")
            try await ui.escape()
            try await ui.waitNoText("Suggested elements:", timeout: 10)
            try await ui.undoProject()
            try await ui.waitText("Suggested elements: \(n)")
            try await ui.press(id: "review-suggestions")
            _ = try await ui.wait(id: "reject-selected", timeout: 15, "Reject Selected")
            let boxes = ui.all().filter { $0.role == "AXCheckBox" && $0.label.hasPrefix("Select ") }
            try ui.expect(boxes.count >= n, "the dialog has a checkbox per suggestion: \(boxes.count)")
            try await ui.press(boxes[0])
            try await ui.waitUntil("Reject Selected (\(n - 1))") { ui.query(id: "reject-selected").first.flatMap { countIn($0.text, prefix: "Reject Selected") } == n - 1 }
            try await ui.press(id: "reject-selected")
            try await ui.waitText("Rejected")
            try await ui.escape()
            try await ui.waitText("Suggested elements: 1")
            guard let name = firstAcceptName(ui) else { throw DriverError("no Accept button left in the scene panel") }
            try await ui.press(acceptButtons(ui)[0])
            try await ui.waitText("Accepted \u{201C}\(name)\u{201D}")
            try await ui.menuButton(label: "Actions for \(name)", item: "Show in script")
            try await ui.waitUntil("the words in the script that suggested it are highlighted", timeout: 10) { ui.scriptJumpHighlight() != nil }
            try ui.quiet("after Show in script")
        },

        Scenario("51-breakdown-depth/filters-and-quick-reuse-fill-scene-2", timeout: 300) { ui, _, ctx in
            try await setupTeakSignal(ui, ctx)
            try await addElement(ui, category: "Props", name: "Brass key")
            try await ui.menuButton(id: "scene-filter", item: "Only missing a breakdown")
            try await ui.waitUntil("the scene list shows one scene") { sceneRows(ui).count == 1 }
            try await selectScene(ui, 1)
            try await ui.wait(label: "Scene 2 breakdown", timeout: 15, "the panel Scene 2 breakdown")
            let reuse = try await ui.wait(label: "Reuse Brass key (Props) in Scene 2", timeout: 15, "Reuse Brass key")
            try await ui.press(reuse)
            try await ui.waitText("Reused \u{201C}Brass key\u{201D} in Scene 2")
            try await ui.waitUntil("the scene list is empty") { sceneRows(ui).isEmpty }
            try await ui.menuButton(id: "scene-filter", item: "All scenes")
            try await ui.waitUntil("the scene list shows two scenes") { sceneRows(ui).count == 2 }
            try ui.quiet("after the filters")
        },

        Scenario("51-breakdown-depth/a-catalog-item-opens-all-its-scenes", timeout: 300) { ui, _, ctx in
            try await setupTeakSignal(ui, ctx)
            try await addElement(ui, category: "Props", name: "Brass key")
            try await selectScene(ui, 2)
            try await ui.wait(label: "Scene 2 breakdown", timeout: 15, "the panel Scene 2 breakdown")
            try await ui.press(label: "Reuse Brass key (Props) in Scene 2", role: "AXButton")
            try await ui.waitText("Reused \u{201C}Brass key\u{201D} in Scene 2")
            try await segment(ui, group: "breakdown-tabs", "Catalog")
            try await selectTableRow(ui, id: "catalog-table", row: 1)
            try await ui.press(label: "Open Scenes", role: "AXButton")
            try await ui.waitText("scenes that use Brass key")
            try await ui.waitUntil("the scene list shows two scenes") { sceneRows(ui).count == 2 }
            try await ui.press(label: "Show all scenes", role: "AXButton")
            try ui.quiet("after Show all scenes")
        },

        Scenario("51-breakdown-depth/location-hospital-availability-and-crew-availability", timeout: 300) { ui, _, ctx in
            try await setupTeakSignal(ui, ctx)
            try await segment(ui, group: "breakdown-tabs", "Locations")
            try await ui.press(label: "Add Location", role: "AXButton")
            try await ui.typeIn("Signal Cabin", into: "nl-name")
            try await ui.typeIn("Town Hospital, Station Road", into: "nl-hospital")
            try await ui.press(id: "nl-submit")
            try await ui.waitText("Added location \u{201C}Signal Cabin\u{201D}")
            let hospital = try await ui.wait(id: "loc-hospital", timeout: 15, "the hospital field")
            try ui.expect(hospital.value == "Town Hospital, Station Road", "the hospital field reads: \(hospital.value)")
            try await ui.setDate(id: "av-location-start", 2027, 6, 20)
            try await ui.setDate(id: "av-location-end", 2027, 6, 22)
            try await ui.typeIn("Track repairs", into: "av-location-note")
            try await ui.press(id: "av-add-dates")
            try await ui.waitText("Updated availability of Signal Cabin")
            try await ui.waitText("Track repairs")
            try await segment(ui, group: "breakdown-tabs", "Cast & Crew")
            try await segment(ui, group: "people-view", "Crew")
            try await ui.press(id: "add-person")
            try await ui.typeIn("Imran Sheikh", into: "nc-name")
            try await ui.typeIn("Cinematographer", into: "nc-role")
            try await ui.press(id: "nc-submit")
            try await ui.waitText("Added Imran Sheikh to the crew")
            try await ui.typeIn("Weekends only", into: "cr-avail")
            try await ui.tab()
            try await ui.press(id: "inspector-close")
            try await ui.waitText("Imran Sheikh")
            try await selectTableRow(ui, id: "crew-list", row: 1)
            let avail = try await ui.wait(id: "cr-avail", timeout: 15, "the availability field")
            try ui.expect(avail.value == "Weekends only", "the availability field reads: \(avail.value)")
            try ui.quiet("after the crew availability")
        },

        Scenario("51-breakdown-depth/advanced-colour-change-and-everything-survives-relaunch", phases: 2, timeout: 300) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Teak Signal")
                try await writeNightShift(ui)
                try await chooseProductionSource(ui)
                try await addElement(ui, category: "Props", name: "Brass key")
                try await segment(ui, group: "breakdown-tabs", "Locations")
                try await ui.press(label: "Add Location", role: "AXButton")
                try await ui.typeIn("Signal Cabin", into: "nl-name")
                try await ui.typeIn("Town Hospital, Station Road", into: "nl-hospital")
                try await ui.press(id: "nl-submit")
                try await ui.waitText("Added location \u{201C}Signal Cabin\u{201D}")
                try await ui.setDate(id: "av-location-start", 2027, 6, 20)
                try await ui.setDate(id: "av-location-end", 2027, 6, 22)
                try await ui.typeIn("Track repairs", into: "av-location-note")
                try await ui.press(id: "av-add-dates")
                try await ui.waitText("Updated availability of Signal Cabin")
                try await segment(ui, group: "breakdown-tabs", "Categories")
                try await ui.press(id: "color-Props-red")
                try await ui.waitText("Changed the look of Props")
                try ui.quiet("after changing the Props colour")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Teak Signal")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Breakdown")
                try await segment(ui, group: "breakdown-tabs", "Categories")
                let red = try await ui.wait(id: "color-Props-red", timeout: 15, "the Red colour of Props")
                try ui.expect(red.isSelectedChoice, "Red in the Props colour group is pressed: \(red)")
                try await segment(ui, group: "breakdown-tabs", "Locations")
                try await ui.press(id: "location-card-Signal Cabin")
                let hospital = try await ui.wait(id: "loc-hospital", timeout: 15, "the hospital field")
                try ui.expect(hospital.value == "Town Hospital, Station Road", "the hospital field reads: \(hospital.value)")
                try await ui.waitText("Track repairs")
                try await ui.nav("Breakdown")
                try await segment(ui, group: "breakdown-tabs", "Breakdown")
                try await ui.wait(label: "Scene 1 breakdown", timeout: 15, "the panel Scene 1 breakdown")
                try await ui.waitUntil("the panel shows Brass key") { panelShows(ui, "Brass key") }
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 61-schedule

extension Scenarios {
    @MainActor static func dayColumnText(_ ui: UI, _ n: Int) -> String { ui.query(id: "day-column-\(n)").first?.label ?? "" }
    @MainActor static func dayColumnShows(_ ui: UI, _ n: Int, _ text: String) -> Bool { dayColumnText(ui, n).contains(text) }

    /// For scene 1 and then scene 2: open the scene, Suggest Elements, accept every suggestion in the panel.
    @MainActor static func suggestAndAcceptAll(_ ui: UI) async throws {
        for n in 1...2 {
            try await selectScene(ui, n)
            try await ui.wait(label: "Scene \(n) breakdown", timeout: 15, "the panel Scene \(n) breakdown")
            try await ui.press(id: "suggest-elements")
            try await ui.waitText("Nothing is added to production until you accept.")
            var guardCount = 0
            while let b = acceptButtons(ui).first, guardCount < 30 {
                try await ui.press(b)
                guardCount += 1
                await ui.idle(0.4)
            }
        }
    }

    /// Case 1: project, script, Production Source, suggestions accepted, the schedule created.
    @MainActor static func scheduleBase(_ ui: UI, _ ctx: Context, project: String) async throws {
        try await ui.firstLaunch(ctx, title: project)
        try await writeNightShift(ui)
        try await chooseProductionSource(ui)
        try await suggestAndAcceptAll(ui)
        try await ui.nav("Schedule")
        try await ui.press(id: "create-shooting-schedule")
        try await ui.press(id: "create-schedule-confirm")
        try await ui.waitText("2 unscheduled")
    }

    @MainActor static func createDay(_ ui: UI, _ iso: String) async throws {
        try await ui.press(id: "create-shooting-day")
        try await ui.typeIn(iso, into: "day-date")
        try await ui.press(id: "day-sheet-confirm")
        try await ui.waitGone(id: "day-date")
    }

    /// Case 2 set-up without the mouse drag: two days, an off day, scene 1 and scene 2 in Day 1.
    @MainActor static func scheduleDays(_ ui: UI, _ ctx: Context, project: String, dragScene1: Bool = false) async throws {
        try await scheduleBase(ui, ctx, project: project)
        try await createDay(ui, "2027-06-14")
        try await createDay(ui, "2027-06-15")
        try await ui.press(id: "add-off-day")
        try await ui.press(id: "day-sheet-confirm")
        try await ui.waitGone(id: "day-sheet-confirm")
        try await ui.waitText("Shoot Day 2 \u{00B7} Tue 15 Jun 2027")
        if dragScene1 {
            try await dragStrip(ui, scene: 1, toDay: 1)
        } else {
            try await ui.menuButton(id: "strip-actions-1", item: "Shoot Day 1")
        }
        try await ui.waitUntil("Day 1 shows 1 scene") { dayColumnShows(ui, 1, "1 scene") }
        try await ui.menuButton(id: "strip-actions-2", item: "Shoot Day 1")
        try await ui.waitUntil("Day 1 shows 2 scenes") { dayColumnShows(ui, 1, "2 scenes") }
    }

    /// The real mouse drag from the pool into a day column.
    @MainActor static func dragStrip(_ ui: UI, scene: Int, toDay: Int) async throws {
        let strip = try await ui.wait(id: "strip-\(scene)", timeout: 15, "the strip Scene \(scene)")
        let column = try await ui.wait(id: "day-column-\(toDay)", timeout: 15, "the Day \(toDay) column")
        try await ui.drag(from: strip, to: column)
        await ui.idle(1.0)
        if !dayColumnShows(ui, toDay, "\(scene) scene") && !dayColumnShows(ui, toDay, "1 scene") {
            // A drag is run by the window server, which does not follow events made inside the app. The same move is
            // made with the scene menu, and the gap is written next to the results.
            try? "the mouse drag of Scene \(scene) was not delivered to the drop target; moved with the scene menu".write(
                to: ui.artifacts.appendingPathComponent("drag-not-delivered.txt"), atomically: true, encoding: .utf8)
            try await ui.menuButton(id: "strip-actions-\(scene)", item: "Shoot Day \(toDay)")
        }
    }

    @MainActor static func openDay(_ ui: UI, _ n: Int) async throws {
        try await ui.press(id: "day-open-\(n)")
        try await ui.wait(id: "day-detail", timeout: 15, "the day detail of Day \(n)")
    }

    @MainActor static func setField(_ ui: UI, _ id: String, _ text: String) async throws {
        try await ui.typeIn(text, into: id)
    }

    static let scheduleScenarios: [Scenario] = [
        Scenario("61-schedule/create-the-schedule-from-the-production-source", timeout: 400) { ui, _, ctx in
            try await scheduleBase(ui, ctx, project: "Signal Yard")
            try ui.quiet("after creating the schedule")
        },

        Scenario("61-schedule/days-off-day-drag-undo-redo-and-move-with-the-menu", timeout: 400) { ui, _, ctx in
            try await scheduleBase(ui, ctx, project: "Signal Yard")
            try await createDay(ui, "2027-06-14")
            try await createDay(ui, "2027-06-15")
            try await ui.press(id: "add-off-day")
            try await ui.press(id: "day-sheet-confirm")
            try await ui.waitText("Shoot Day 2 \u{00B7} Tue 15 Jun 2027")
            try await dragStrip(ui, scene: 1, toDay: 1)
            try await ui.waitUntil("the Day 1 column shows 1 scene") { dayColumnShows(ui, 1, "1 scene") }
            try await ui.undoProject()
            try await ui.waitUntil("the pool shows unscheduled and Day 1 shows no scenes") {
                ui.shows("2 unscheduled") && dayColumnShows(ui, 1, "No scenes scheduled")
            }
            try await ui.redoProject()
            try await ui.waitUntil("the Day 1 column shows 1 scene again") { dayColumnShows(ui, 1, "1 scene") }
            try await ui.menuButton(id: "strip-actions-2", item: "Shoot Day 1")
            try await ui.waitUntil("the Day 1 column shows 2 scenes") { dayColumnShows(ui, 1, "2 scenes") }
            let offColumn = ui.queryPrefix("day-column-").first { $0.label.contains("Off Day") }?.label ?? ""
            try ui.expect(offColumn.contains("Off day"), "the Off Day column shows Off day: \(offColumn)")
            try ui.quiet("after moving scenes")
        },

        Scenario("61-schedule/meal-break-day-times-and-the-computed-timeline", timeout: 400) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try await ui.menuButton(id: "day-actions-1", item: "Meal Break")
            try await ui.typeIn("45m", into: "marker-duration")
            try await ui.press(id: "marker-save")
            try await ui.waitUntil("the Day 1 column shows Meal Break") { dayColumnShows(ui, 1, "Meal Break") }
            try await openDay(ui, 1)
            try await ui.waitText("What exactly are we doing on this day?")
            try await setField(ui, "day-crew-call", "07:00")
            try await setField(ui, "day-shooting-start", "08:00")
            try await ui.blur()
            let timeline = try await ui.wait(id: "day-timeline", timeout: 15, "the timeline of Shoot Day 1")
            _ = timeline
            try await ui.waitUntil("the timeline lists the day's times") {
                let t = ui.query(id: "day-timeline").first.map { descendants($0).map(\.text).joined(separator: "\n") + $0.text } ?? ""
                return ["07:00", "Crew call", "08:00", "Shooting starts", "Meal Break", "Estimated wrap"].allSatisfy { t.contains($0) }
            }
            try await setField(ui, "day-target", "30")
            try await ui.blur()
            try await ui.waitText("Over target")
            try ui.quiet("on the day detail")
            try await ui.blur(); try await ui.press(id: "day-back")
        },

        Scenario("61-schedule/conflicts-are-advice-grouping-moves-nothing", timeout: 400) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try ui.expect(dayColumnShows(ui, 1, "2 scenes"), "the Day 1 column shows 2 scenes")
            try ui.expect(!(ui.query(id: "warnings-button").first?.label.hasPrefix("0 warnings") ?? false), "the warnings button does not read 0 warnings: \(ui.query(id: "warnings-button").first?.label ?? "-")")
            try await ui.press(id: "warnings-button")
            try await ui.wait(label: "Schedule assistance", timeout: 15, "the Schedule assistance panel")
            guard let keep = ui.queryPrefix("warning-keep-").first else { throw DriverError("no Keep anyway button in the panel") }
            try await ui.press(keep)
            if let confirm = ui.query(id: "keep-anyway-confirm").first { try await ui.press(confirm) }
            try await ui.waitText("Kept anyway")
            try await ui.press(id: "drawer-close")
            try ui.expect(dayColumnShows(ui, 1, "2 scenes"), "the Day 1 column still shows 2 scenes")
            let before = dayColumnText(ui, 1)
            try await ui.press(id: "suggest-grouping")
            try await ui.wait(label: "Grouping suggestions", timeout: 15, "the Grouping suggestions panel")
            try await ui.press(id: "drawer-close")
            try ui.expect(dayColumnText(ui, 1) == before, "the Day 1 column text is unchanged")
            try ui.quiet("after grouping suggestions")
        },

        Scenario("61-schedule/filters-only-change-what-is-shown", timeout: 400) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try await ui.press(id: "schedule-mode-list")
            try await ui.popUp(id: "filter-scheduled", choose: "Unscheduled")
            try await ui.waitText("No matching scenes.")
            try await ui.waitText("Showing 0 of 2 scenes")
            try await ui.press(id: "clear-filters")
            try await ui.waitNoText("No matching scenes.", timeout: 10)
            try await ui.press(id: "schedule-mode-board")
            try await ui.waitUntil("the Day 1 column shows 2 scenes") { dayColumnShows(ui, 1, "2 scenes") }
            try ui.quiet("after the filters")
        },

        Scenario("61-schedule/call-sheet-stale-review-follows-day-times", timeout: 400) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try await ui.menuButton(id: "day-actions-1", item: "Create Call Sheet")
            try await ui.nav("Schedule")
            try await ui.waitUntil("the Day 1 column shows Call sheet: Draft") { dayColumnShows(ui, 1, "Call sheet: Draft") }
            try await openDay(ui, 1)
            try await setField(ui, "day-target-wrap", "18:00")
            try await ui.blur(); try await ui.press(id: "day-back")
            try await ui.waitUntil("Call sheet: Needs refresh") { dayColumnShows(ui, 1, "Call sheet: Needs refresh") }
            try await ui.undoProject()
            try await ui.waitUntil("Call sheet: Draft again") { dayColumnShows(ui, 1, "Call sheet: Draft") }
            try await ui.redoProject()
            try ui.quiet("after the undo and redo")
        },

        Scenario("61-schedule/day-out-of-days-derived-corrected-undone", timeout: 400) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try await ui.press(id: "schedule-section-dood")
            _ = try await ui.wait(id: "dood-table", timeout: 15, "the Day Out of Days table")
            let first = ui.queryPrefix("dood-cell-").first { $0.identifier.hasSuffix("-1") }
            try ui.expect(first.map { ["SWF", "SW", "W"].contains(where: $0.text.contains) } == true, "the first Day 1 cell shows a working code: \(first?.label ?? "-")")
            try await ui.menuButton(id: "dood-cell-1-2", item: "T")
            try await ui.waitText("1 corrected by hand")
            try await ui.undoProject()
            try await ui.waitUntil("the table no longer shows T*") { !ui.queryPrefix("dood-cell-").contains { $0.text.contains("T*") } && !ui.shows("1 corrected by hand") }
            try await ui.redoProject()
            try await ui.waitText("1 corrected by hand")
            try await ui.menuButton(id: "dood-cell-1-2", item: "Add a note")
            try await ui.typeIn("Flies in the night before", into: "dood-note-field")
            try await ui.press(id: "dood-note-save")
            try await ui.waitUntil("a cell is named with the note") { ui.queryPrefix("dood-cell-").contains { $0.label.contains("note: Flies in the night before") } }
            try ui.quiet("after the Day Out of Days notes")
        },

        Scenario("61-schedule/exports-dialogs-open-and-files-are-identical", timeout: 100) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            try await ui.press(id: "schedule-section-dood")
            _ = try await ui.wait(id: "dood-table", timeout: 15, "the Day Out of Days table")
            try await ui.press(id: "dood-export")
            try await ui.waitText("Export Day Out of Days")
            try await ui.escape()
            try await ui.waitNoText("Export Day Out of Days", timeout: 10)
            try await ui.press(id: "schedule-section-schedule")
            try await ui.press(id: "schedule-mode-board")
            try await ui.press(id: "schedule-export")
            try await ui.waitText("Export Shooting Schedule")
            try await ui.escape()
            try await ui.waitNoText("Export Shooting Schedule", timeout: 10)
            // The files themselves: the same export twice to two paths, compared byte for byte.
            for (kind, format) in [("schedule", "pdf"), ("schedule", "xlsx"), ("dood", "pdf"), ("dood", "xlsx")] {
                let a = ctx.work.appendingPathComponent("sched-\(kind)-a.\(format)")
                let b = ctx.work.appendingPathComponent("sched-\(kind)-b.\(format)")
                try await ui.exportTo(kind: kind, format: format, a)
                await ui.idle(1.0)
                try await ui.exportTo(kind: kind, format: format, b)
                let da = try Data(contentsOf: a), db = try Data(contentsOf: b)
                try ui.expect(!da.isEmpty && da == db, "\(kind) \(format): the two exports are the same byte for byte (\(da.count) and \(db.count) bytes)")
            }
            try ui.quiet("after the exports")
        },

        Scenario("61-schedule/relaunch-keeps-days-scenes-breaks-times-corrections", phases: 2, timeout: 400) { ui, phase, ctx in
            if phase == 1 {
                try await scheduleDays(ui, ctx, project: "Signal Yard")
                try await ui.menuButton(id: "day-actions-1", item: "Meal Break")
                try await ui.typeIn("45m", into: "marker-duration")
                try await ui.press(id: "marker-save")
                try await ui.waitUntil("the Day 1 column shows Meal Break") { dayColumnShows(ui, 1, "Meal Break") }
                try await openDay(ui, 1)
                try await setField(ui, "day-crew-call", "07:00")
                try await setField(ui, "day-shooting-start", "08:00")
                try await ui.waitText("Estimated wrap")
                try await ui.blur(); try await ui.press(id: "day-back")
                try await ui.press(id: "schedule-section-dood")
                _ = try await ui.wait(id: "dood-table", timeout: 15, "the Day Out of Days table")
                try await ui.menuButton(id: "dood-cell-1-2", item: "T")
                try await ui.waitText("1 corrected by hand")
                try await ui.menuButton(id: "dood-cell-1-2", item: "Add a note")
                try await ui.typeIn("Flies in the night before", into: "dood-note-field")
                try await ui.press(id: "dood-note-save")
                try await ui.waitUntil("a cell is named with the note") { ui.queryPrefix("dood-cell-").contains { $0.label.contains("note: Flies in the night before") } }
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Signal Yard")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Schedule")
                try await ui.waitUntil("Day 1 shows 2 scenes, Meal Break and wrap") {
                    dayColumnShows(ui, 1, "2 scenes") && dayColumnShows(ui, 1, "Meal Break") && dayColumnShows(ui, 1, "wrap")
                }
                try await ui.press(id: "schedule-section-dood")
                _ = try await ui.wait(id: "dood-table", timeout: 15, "the Day Out of Days table")
                try await ui.waitText("1 corrected by hand")
                try ui.expect(ui.queryPrefix("dood-cell-").contains { $0.label.contains("note: Flies in the night before") }, "a cell is named with the note")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 85-project-pages

extension Scenarios {
    /// The project window's title. On the Mac it is the project's title (Windows adds " - OpenFrame Studio").
    @MainActor static var windowTitle: String {
        let names = NSApp.windows.filter { $0.isVisible && !$0.title.isEmpty && $0.title != "OpenFrame Studio" && $0.title != "Projects" }
        return (names.first { $0.isMainWindow } ?? names.first)?.title ?? ""
    }

    /// Open the project menu and choose an item that starts with the text.
    @MainActor static func projectMenu(_ ui: UI, _ item: String) async throws {
        try await ui.menuButton(id: "project-menu", item: item)
    }

    /// Project Settings: the toolbar button opens the sheet (the page is a segment name such as
    /// "Templates"). A sheet that is already open is left as it is.
    @MainActor static func openSettings(_ ui: UI, page: String? = nil) async throws {
        if !ui.exists(id: "project-settings-sheet") {
            try await ui.press(id: "project-settings-button")
            _ = try await ui.wait(id: "project-settings-sheet", timeout: 10, "the Project Settings sheet")
        }
        if let page {
            let segment = try await ui.wait(label: page, role: "AXRadioButton", timeout: 10)
            if !segment.performPress() { try await ui.clickQueued(segment) }
            await ui.idle(0.4)
        }
    }

    @MainActor static func closeSettings(_ ui: UI) async throws {
        let done = try await ui.wait(label: "Done", role: "AXButton", timeout: 10)
        if !done.performPress() { try await ui.clickQueued(done) }
        try await ui.waitGone(id: "project-settings-sheet", timeout: 10)
    }

    /// "Templates tab": Project Settings, the Templates page, Create Template shows.
    @MainActor static func templatesTab(_ ui: UI) async throws {
        try await openSettings(ui, page: "Templates")
        try await ui.waitText("Create Template")
    }

    /// The templates grid draws only what is on screen, so the page is scrolled to its end, where
    /// the "This project" section is.
    @MainActor static func scrollSettingsToEnd() {
        func scan(_ v: NSView, _ out: inout [NSScrollView]) {
            if let sv = v as? NSScrollView { out.append(sv) }
            for c in v.subviews { scan(c, &out) }
        }
        for w in NSApp.windows where w.isVisible && w.sheetParent != nil {
            var found: [NSScrollView] = []
            if let root = w.contentView?.superview { scan(root, &found) }
            for sv in found where sv.documentView != nil && sv.documentView!.frame.height > sv.contentView.bounds.height {
                let doc = sv.documentView!
                let y = doc.isFlipped ? doc.frame.height - sv.contentView.bounds.height : 0
                sv.contentView.scroll(to: NSPoint(x: 0, y: max(0, y)))
                sv.reflectScrolledClipView(sv.contentView)
            }
        }
    }

    @MainActor static func templateRows(_ ui: UI) -> [Element] {
        scrollSettingsToEnd()
        return ui.all().filter { $0.label.hasPrefix("Preview ") }
    }

    @MainActor static func hasTemplate(_ ui: UI, _ name: String) -> Bool {
        templateRows(ui).contains { $0.label.hasPrefix("Preview \(name),") }
    }

    @MainActor static func makeTemplate(_ ui: UI, _ name: String) async throws {
        try await ui.press(label: "Create Template", role: "AXButton")
        try await ui.setField("Template name", name)
        try await ui.press(label: "Create", role: "AXButton")
        try await ui.waitText("Created \u{201C}\(name)\u{201D} in this project.", timeout: 10)
        try await ui.waitUntil("the templates include \(name)") { hasTemplate(ui, name) }
    }

    @MainActor static func deleteTemplate(_ ui: UI, _ name: String) async throws {
        scrollSettingsToEnd()
        await ui.idle(0.5)
        try await ui.menuButton(label: "Actions for \(name)", item: "Delete")
        try await ui.waitText("Moved template \u{201C}\(name)\u{201D} to Recently Deleted")
        try await ui.waitUntil("the templates no longer include \(name)") { !hasTemplate(ui, name) }
    }

    static let projectPages: [Scenario] = [
        Scenario("85-project-pages/project-settings-title-logline-save-undo-redo") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Line")
            try await openSettings(ui)
            try await ui.setField("Title", "Tide Line Revised")
            try await ui.setField("Logline", "A ferry pilot races the tide to reach her brother.")
            try await ui.press(id: "project-settings-save")
            try await ui.waitText("Updated project settings")
            try await ui.waitUntil("the window title is Tide Line Revised, now \(windowTitle)") { windowTitle == "Tide Line Revised" }
            // The sheet holds the keyboard; close it so Undo and Redo act on the project, not on a field.
            try await closeSettings(ui)
            try await ui.undoProject()
            try await ui.waitUntil("the old title, now \(windowTitle)") { windowTitle == "Tide Line" }
            try await openSettings(ui)
            try await ui.waitUntil("an empty logline") { ui.query(id: "project-settings-logline").first?.value.isEmpty == true }
            try await closeSettings(ui)
            try await ui.redoProject()
            try await ui.waitUntil("the revised title, now \(windowTitle)") { windowTitle == "Tide Line Revised" }
            try await openSettings(ui)
            try await ui.waitUntil("the logline is back") {
                ui.query(id: "project-settings-logline").first?.value == "A ferry pilot races the tide to reach her brother."
            }
            try await closeSettings(ui)
            try ui.quiet("after undo and redo of the settings")
        },

        Scenario("85-project-pages/templates-make-two-delete-restore-and-purge") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Line")
            try await templatesTab(ui)
            try await makeTemplate(ui, "Night exterior call")
            try await makeTemplate(ui, "Old call template")
            try await deleteTemplate(ui, "Night exterior call")
            try await deleteTemplate(ui, "Old call template")
            try ui.quiet("after deleting both templates")
            try await closeSettings(ui)
            try await ui.nav("Recently Deleted")
            try await ui.press(id: "restore-Night exterior call")
            try await ui.waitText("Restored \u{201C}Night exterior call\u{201D}")
            try await ui.press(id: "purge-Old call template")
            try await ui.waitText("Delete \u{201C}Old call template\u{201D} permanently?")
            try await ui.press(id: "trash-purge-confirm")
            try await ui.waitText("\u{201C}Old call template\u{201D} was deleted permanently.")
            try await ui.waitNoText("Old call template", timeout: 10)
            try ui.quiet("after the permanent delete")
            try await templatesTab(ui)
            try await ui.waitUntil("the templates include Night exterior call") { hasTemplate(ui, "Night exterior call") }
            try await closeSettings(ui)
        },

        Scenario("85-project-pages/activity-lists-what-happened-and-filters-by-kind") { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Tide Line")
            try await ui.nav("Activity")
            try await ui.waitText("Activity History")
            let list = try await ui.wait(id: "activity-list", timeout: 15, "the Activity list")
            let entries = descendants(list).filter { $0.identifier.hasPrefix("activity-row-") }
            try ui.expect(!entries.isEmpty, "the Activity list has at least one entry")
            try ui.expect(ui.shows("E2E Tester"), "one entry names E2E Tester")
            try await segment(ui, group: "activity-filter", "Story")
            try await ui.waitText("Nothing of this kind has happened yet.")
            try await segment(ui, group: "activity-filter", "All")
            _ = try await ui.wait(id: "activity-list", timeout: 15, "the Activity list again")
            try ui.quiet("after the Activity filters")
        },

        Scenario("85-project-pages/relaunch-keeps-settings-template-and-permanent-delete", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Tide Line")
                try await openSettings(ui)
                try await ui.setField("Title", "Tide Line Revised")
                try await ui.setField("Logline", "A ferry pilot races the tide to reach her brother.")
                try await ui.press(id: "project-settings-save")
                try await ui.waitText("Updated project settings")
                try await templatesTab(ui)
                try await makeTemplate(ui, "Night exterior call")
                try await makeTemplate(ui, "Old call template")
                try await deleteTemplate(ui, "Night exterior call")
                try await deleteTemplate(ui, "Old call template")
                try await closeSettings(ui)
                try await ui.nav("Recently Deleted")
                try await ui.press(id: "restore-Night exterior call")
                try await ui.waitText("Restored \u{201C}Night exterior call\u{201D}")
                try await ui.press(id: "purge-Old call template")
                try await ui.press(id: "trash-purge-confirm")
                try await ui.waitText("\u{201C}Old call template\u{201D} was deleted permanently.")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Tide Line Revised")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await openSettings(ui)
                let logline = try await ui.wait(label: "Logline", timeout: 15, "the logline field")
                try ui.expect(logline.value == "A ferry pilot races the tide to reach her brother.", "the logline reads: \(logline.value)")
                try await templatesTab(ui)
                try await ui.waitUntil("the templates include Night exterior call") { hasTemplate(ui, "Night exterior call") }
                try ui.expect(!hasTemplate(ui, "Old call template"), "the templates do not include Old call template")
                try await closeSettings(ui)
                try await ui.nav("Recently Deleted")
                try await ui.waitText("Nothing has been deleted.")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 62-call-sheets-exports

extension Scenarios {
    static let brandLine = "Created with OpenFrame Studio"

    /// "Check the PDF F": exists, starts with %PDF- and carries the brand line.
    @MainActor static func checkPdf(_ ui: UI, _ url: URL, brand: Bool = true) throws {
        let data = try Data(contentsOf: url)
        try ui.expect(data.prefix(5) == Data("%PDF-".utf8), "\(url.lastPathComponent) starts with %PDF-")
        let has = data.range(of: Data(brandLine.utf8)) != nil
        try ui.expect(has == brand, "\(url.lastPathComponent) \(brand ? "carries" : "does not carry") the brand line")
    }

    /// The Night Bus text (three scenes), used where the Windows spec opens a longer sample script.
    @MainActor static func writeNightBus(_ ui: UI) async throws {
        try await ui.nav("Screenplay")
        let tv = try await ui.editor()
        if tv.string.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { tv.setSelectedRange(NSRange(location: 0, length: 0)) } else { await ui.caretToEnd(tv) }
        try await ui.typeHere("INT. POLICE STATION - NIGHT"); try await ui.enter()
        try await ui.typeHere("Arjun confesses to Inspector Rao."); try await ui.enter()
        try await ui.typeHere("ARJUN"); try await ui.enter()
        try await ui.typeHere("I was there that night."); try await ui.enter()
        try await sceneHeadingHere(ui, tv)
        try await ui.typeHere("EXT. BUS STOP - NIGHT"); try await ui.enter()
        try await ui.typeHere("Arjun meets Meera again at a rainy bus stop."); try await ui.enter()
        try await ui.typeHere("MEERA"); try await ui.enter()
        try await ui.typeHere("You came back."); try await ui.enter()
        try await sceneHeadingHere(ui, tv)
        try await ui.typeHere("EXT. RAILWAY STATION - DAY"); try await ui.enter()
        try await ui.typeHere("Meera waits for the late train with a red umbrella.")
        try await ui.waitSaved(timeout: 20)
    }

    /// Plan two shooting days from the Production Source (the Windows steps that go through the core, done on screen).
    @MainActor static func planLostProperty(_ ui: UI, _ ctx: Context) async throws {
        try await ui.firstLaunch(ctx, title: "Lost Property Unit")
        try await writeNightBus(ui)
        try await chooseProductionSource(ui)
        try await suggestAndAcceptAll(ui)
        try await ui.nav("Schedule")
        try await ui.press(id: "create-shooting-schedule")
        try await ui.press(id: "create-schedule-confirm")
        try await ui.waitText("3 unscheduled")
        try await createDay(ui, "2027-06-14")
        try await createDay(ui, "2027-06-15")
        try await ui.menuButton(id: "strip-actions-1", item: "Shoot Day 1")
        try await ui.waitUntil("Day 1 shows 1 scene") { dayColumnShows(ui, 1, "1 scene") }
        try await ui.menuButton(id: "strip-actions-2", item: "Shoot Day 1")
        try await ui.waitUntil("Day 1 shows 2 scenes") { dayColumnShows(ui, 1, "2 scenes") }
        try await ui.menuButton(id: "day-actions-1", item: "Meal Break")
        try await ui.typeIn("45m", into: "marker-duration")
        try await ui.press(id: "marker-save")
        try await ui.waitUntil("Day 1 shows Meal Break") { dayColumnShows(ui, 1, "Meal Break") }
        try await openDay(ui, 1)
        try await setField(ui, "day-crew-call", "07:00")
        try await setField(ui, "day-shooting-start", "08:00")
        try await setField(ui, "day-target-wrap", "19:00")
        try await ui.blur(); try await ui.press(id: "day-back")
        try await ui.waitText("Shoot Day 1")
    }

    @MainActor static func createCallSheet(_ ui: UI) async throws {
        try await ui.nav("Call Sheets")
        try await ui.press(id: "callsheet-create")
        try await ui.press(id: "callsheet-create-confirm")
        _ = try await ui.wait(id: "callsheet-document", timeout: 20, "the call sheet")
    }

    /// Opens the first call sheet of the list when the list is showing (its rows are hidden from the tree).
    @MainActor static func openFirstCallSheet(_ ui: UI) async throws {
        if ui.exists(id: "callsheet-document") { return }
        try await selectTableRow(ui, id: "callsheet-list", row: 1)
        _ = try await ui.wait(id: "callsheet-document", timeout: 15, "the call sheet")
    }

    @MainActor static func callSheetField(_ ui: UI, _ slug: String) -> Element? { ui.query(id: "callsheet-field-\(slug)").first }

    static let callSheetScenarios: [Scenario] = [
        Scenario("62-call-sheets-exports/open-a-sample-script-that-ships-with-the-app", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Lost Property Unit")
            try await ui.menu(["File", "Import Screenplay\u{2026}"])
            try await ui.press(label: "Example Short", role: "AXButton")
            try await ui.press(id: "import-preview")
            try await ui.waitText("Import Screenplay \u{00B7} Preview")
            try await ui.waitText("Scenes")
            try ui.expect(ui.all().contains { $0.text.contains("10") }, "the dialog shows the figure 10")
            try await ui.press(id: "import-confirm")
            try await ui.waitText("Import complete")
            try await ui.press(label: "Open Screenplay", role: "AXButton")
            _ = try await ui.editor()
            try ui.quiet("after opening the sample")
        },

        Scenario("62-call-sheets-exports/plan-two-shooting-days", timeout: 400) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await ui.press(id: "schedule-section-dood")
            _ = try await ui.wait(id: "dood-table", timeout: 15, "the Day Out of Days table")
            try ui.expect(ui.queryPrefix("dood-cell-").count >= 1, "the Day Out of Days has at least one cast row")
            try await ui.press(id: "schedule-section-schedule")
            try ui.expect(ui.shows("Shoot Day 1"), "the workspace shows Shoot Day 1")
            try ui.quiet("after planning the days")
        },

        Scenario("62-call-sheets-exports/create-call-sheet-default-sections-needs-input-add-section", timeout: 500) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await createCallSheet(ui)
            let doc = try await ui.wait(id: "callsheet-document", timeout: 15, "the call sheet")
            let text = descendants(doc).map(\.text).joined(separator: "\n") + doc.text
            for section in ["PRODUCTION HEADER", "DAY AND TIMES", "LOCATIONS", "SCENE SCHEDULE", "TALENT", "PRACTICAL NOTES"] {
                try ui.expect(text.contains(section), "the call sheet has the section \(section)")
            }
            try ui.expect(!text.contains("WEATHER"), "the call sheet has no WEATHER section yet")
            try ui.expect(text.contains("Needs input") || ui.shows("Needs input"), "an empty value shows Needs input")
            try ui.expect(callSheetField(ui, "crew-call")?.value == "07:00", "the Crew call field reads 07:00: \(callSheetField(ui, "crew-call")?.value ?? "-")")
            try await ui.menuButton(id: "callsheet-add-section", item: "Weather")
            try await ui.waitText("WEATHER")
            try await ui.typeIn("Light rain after 18:00", into: "callsheet-field-forecast")
            try await ui.tab()
            try await ui.press(id: "callsheet-back")
            try await ui.waitUntil("the project lists one call sheet") { ui.all().contains { $0.text.contains("Shoot Day 1") } && !ui.exists(id: "callsheet-document") }
            try ui.quiet("after adding the Weather section")
        },

        Scenario("62-call-sheets-exports/schedule-change-needs-refresh-preview-lists-changes", timeout: 600) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await createCallSheet(ui)
            try await ui.menuButton(id: "callsheet-add-section", item: "Weather")
            try await ui.typeIn("Light rain after 18:00", into: "callsheet-field-forecast")
            try await ui.tab()
            try await ui.nav("Schedule")
            try await ui.menuButton(id: "strip-actions-3", item: "Shoot Day 1")
            try await ui.waitUntil("Day 1 shows 3 scenes") { dayColumnShows(ui, 1, "3 scenes") }
            try await ui.nav("Call Sheets")
            try await openFirstCallSheet(ui)
            try await ui.waitText("Needs refresh")
            try await ui.press(id: "callsheet-review-update")
            try await ui.waitText("Update call sheet: review changes")
            try ui.expect(ui.shows("scene") || ui.shows("Scene"), "the dialog mentions a scene")
            try await ui.press(id: "callsheet-refresh-apply")
            try await ui.waitGone(id: "callsheet-stale-banner", timeout: 15)
            try ui.expect(callSheetField(ui, "forecast")?.value == "Light rain after 18:00", "the Forecast field still reads the entered text")
            try ui.quiet("after applying the update")
        },

        Scenario("62-call-sheets-exports/finalize-freezes-a-snapshot-new-revision-follows-the-schedule", timeout: 600) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await createCallSheet(ui)
            try await ui.press(id: "callsheet-finalize")
            try await ui.press(id: "callsheet-finalize-confirm")
            try await ui.waitText("Finalized ")
            try ui.expect(ui.fieldTakesInput(label: "Crew call") == false, "the Crew call field is disabled")
            let pdf = ctx.work.appendingPathComponent("call-sheet.pdf")
            try await ui.press(id: "callsheet-export")
            try await ui.pressAndSave(id: "export-run", to: pdf)
            try await ui.press(id: "export-close")
            try checkPdf(ui, pdf)
            try await ui.nav("Schedule")
            try await openDay(ui, 1)
            try await setField(ui, "day-crew-call", "07:30")
            try await ui.blur(); try await ui.press(id: "day-back")
            try await ui.nav("Call Sheets")
            try await openFirstCallSheet(ui)
            try await ui.waitText("Start New Revision")
            try ui.expect(callSheetField(ui, "crew-call")?.value == "07:00", "the Crew call field still reads 07:00")
            try await ui.press(id: "callsheet-start-new-revision")
            try await ui.press(id: "callsheet-refresh-apply")
            try await ui.waitText("v2")
            try await ui.waitUntil("the Crew call field reads 07:30") { callSheetField(ui, "crew-call")?.value == "07:30" }
            try ui.quiet("after the new revision")
        },

        Scenario("62-call-sheets-exports/budget-keeps-planned-and-actual-with-the-difference", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Lost Property Unit")
            try await ui.nav("Budget")
            try await ui.press(label: "Start budget", role: "AXButton")
            try await ui.press(label: "Add Line", role: "AXButton")
            try await ui.typeIn("Lead actor", into: nil, label: "Description")
            try await ui.typeIn("50000", into: nil, label: "Planned amount")
            try await ui.typeIn("55000", into: nil, label: "Actual amount")
            try await ui.press(id: "budget-line-add")
            try await ui.waitText("over plan")
            try await ui.undoProject()
            try await ui.waitText("No lines yet")
            try ui.quiet("after undoing the budget line")
        },

        Scenario("62-call-sheets-exports/indie-call-sheet-template-is-a-copy-of-its-own", timeout: 600) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await ui.nav("Call Sheets")
            try await ui.press(id: "callsheet-create")
            try await ui.popUp(id: "callsheet-create-day", choose: "Shoot Day 2")
            try await ui.popUp(id: "callsheet-create-template", choose: "Indie Call Sheet")
            try await ui.press(id: "callsheet-create-confirm")
            let doc = try await ui.wait(id: "callsheet-document", timeout: 20, "the call sheet")
            let text = descendants(doc).map(\.text).joined(separator: "\n") + doc.text
            try ui.expect(text.contains("WEATHER") && text.contains("CREW"), "the new call sheet has a Weather section and a Crew section")
            try ui.quiet("after the template call sheet")
        },

        Scenario("62-call-sheets-exports/sides-report-and-visual-exports-carry-the-brand-line", timeout: 900) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await ui.nav("Call Sheets")
            let dir = ctx.work
            try await segment(ui, group: "callsheets-tabs", "Sides")
            try await ui.press(id: "sides-cover")
            try await ui.pressAndSave(id: "sides-export", to: dir.appendingPathComponent("sides.pdf"))
            try await ui.press(id: "export-close")
            try checkPdf(ui, dir.appendingPathComponent("sides.pdf"))
            try await segment(ui, group: "callsheets-tabs", "Reports")
            try await ui.press(id: "reports-type-scene-report")
            try await ui.pressAndSave(id: "reports-export", to: dir.appendingPathComponent("scene-report.pdf"))
            try await ui.press(id: "export-close")
            try checkPdf(ui, dir.appendingPathComponent("scene-report.pdf"))
            try await ui.nav("Schedule")
            try await ui.exportTo(kind: "schedule", format: "pdf", dir.appendingPathComponent("schedule.pdf"))
            try checkPdf(ui, dir.appendingPathComponent("schedule.pdf"))
            try await ui.exportTo(kind: "dood", format: "pdf", dir.appendingPathComponent("dood.pdf"))
            try checkPdf(ui, dir.appendingPathComponent("dood.pdf"))
            try await ui.nav("Breakdown")
            try await ui.menuButton(label: "Breakdown report", item: "PDF")
            try await ui.nav("Visual Planning")
            try await ui.press(label: "Shot List", role: "AXButton")
            try await ui.pressAndSave(id: "visual-export", to: dir.appendingPathComponent("shots.pdf"))
            try checkPdf(ui, dir.appendingPathComponent("shots.pdf"))
            try ui.quiet("after the exports")
        },

        Scenario("62-call-sheets-exports/notes-and-tasks-for-the-shoot", timeout: 300) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Lost Property Unit")
            try await ui.nav("Notes and Tasks")
            try await ui.press(id: "new-note")
            try await ui.typeIn("Rain cover", into: "note-title")
            try await ui.typeIn("Keep the tarps on the truck.", into: "note-body")
            try await ui.press(id: "note-close")
            try await openTaskDrawer(ui)
            try await ui.typeIn("Confirm the railway permit", into: "task-title")
            try await ui.press(label: "Add Task", role: "AXButton")
            try await ui.waitText("Added task \u{201C}Confirm the railway permit\u{201D}")
            try await ui.nav("Notes and Tasks")
            try ui.expect(noteRows(ui, "Rain cover").count == 1 && ui.shows("Confirm the railway permit"), "the workspace shows the note and the task")
            try ui.quiet("on Notes and Tasks")
        },

        Scenario("62-call-sheets-exports/relaunch-finds-everything-again", phases: 2, timeout: 700) { ui, phase, ctx in
            if phase == 1 {
                try await planLostProperty(ui, ctx)
                try await createCallSheet(ui)
                try await ui.press(id: "callsheet-finalize")
                try await ui.press(id: "callsheet-finalize-confirm")
                try await ui.waitText("Finalized ")
                try await ui.nav("Schedule")
                try await openDay(ui, 1)
                try await setField(ui, "day-crew-call", "07:30")
                try await ui.blur(); try await ui.press(id: "day-back")
                try await ui.nav("Call Sheets")
                try await openFirstCallSheet(ui)
                try await ui.press(id: "callsheet-start-new-revision")
                try await ui.press(id: "callsheet-refresh-apply")
                try await ui.waitText("v2")
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Lost Property Unit")
                try ui.expect(!(await ui.recoveryOffered(within: 2)), "no recovery question after a clean quit")
                try await ui.nav("Call Sheets")
                try await ui.waitUntil("the call sheets include revision 2 and a Final one") { ui.shows("v2") && ui.shows("Final") }
                try await ui.nav("Schedule")
                try await ui.waitText("Shoot Day 1")
                try ui.quiet("after the relaunch")
                try await ui.closeProject()
            }
        },
    ]
}

// MARK: 96-keyboard-and-screen-reader (production workspaces and project pages)

extension Scenarios {
    struct StopFinding: Codable { var workspace: String; var step: Int; var problem: String; var what: String }

    /// What the focused responder is, as a person would name it.
    @MainActor static func focusName(_ r: NSResponder) -> (what: String, name: String) {
        var view = r as? NSView
        // A text field being edited is focused through its field editor.
        if let tv = r as? NSTextView, tv.isFieldEditor, let field = tv.delegate as? NSView { view = field }
        guard let v = view else { return (String(describing: type(of: r)), "") }
        var names: [String] = []
        let e = Element(obj: v)
        names.append(e.label); names.append(e.title)
        if let c = v as? NSControl {
            names.append(c.stringValue)
            if let b = v as? NSButton { names.append(b.title) }
        }
        if let f = v as? NSTextField { names.append(f.placeholderString ?? "") }
        names.append(v.toolTip ?? "")
        // The container's name stands for an unlabelled inner view (a SwiftUI row, a table cell).
        var parent: NSView? = v.superview
        var hops = 0
        while names.allSatisfy({ $0.isEmpty }), let p = parent, hops < 4 {
            let pe = Element(obj: p)
            names.append(pe.label); names.append(pe.title)
            parent = p.superview
            hops += 1
        }
        return (String(describing: type(of: v)), names.first { !$0.isEmpty } ?? "")
    }

    /// Presses Tab up to 80 times and records every stop that is unnamed, hidden, lost or a trap.
    @MainActor static func walkTabs(_ ui: UI, workspace: String, into findings: inout [StopFinding]) async throws -> Int {
        guard let window = NSApp.keyWindow ?? NSApp.windows.first(where: { $0.isVisible }) else { throw DriverError("no window") }
        window.makeFirstResponder(nil)
        var last: ObjectIdentifier?
        var same = 0
        var stops = 0
        for step in 1...80 {
            try await ui.tab()
            guard let r = window.firstResponder, r !== window else {
                findings.append(StopFinding(workspace: workspace, step: step, problem: "focus is on the page itself", what: ""))
                continue
            }
            stops += 1
            let (what, name) = focusName(r)
            if let v = r as? NSView, v.isHiddenOrHasHiddenAncestor || v.window == nil {
                findings.append(StopFinding(workspace: workspace, step: step, problem: "the focused element is not visible", what: what))
            }
            if name.isEmpty { findings.append(StopFinding(workspace: workspace, step: step, problem: "the focused element has no accessible name", what: what)) }
            if let v = r as? NSView, !(v.focusRingType != .none || v is NSTextView || v is NSTextField) {
                findings.append(StopFinding(workspace: workspace, step: step, problem: "no focus mark", what: what))
            }
            let id = ObjectIdentifier(r)
            same = (id == last) ? same + 1 : 0
            last = id
            if same >= 3 { findings.append(StopFinding(workspace: workspace, step: step, problem: "focus stayed on the same element for four presses (a trap)", what: what)); break }
        }
        return stops
    }

    static func keyboardCase(_ workspace: String, setup: @escaping @MainActor (UI) async throws -> Void = { _ in }) -> Scenario {
        let slug = workspace.lowercased().replacingOccurrences(of: " ", with: "-")
        return Scenario("96-keyboard-and-screen-reader/\(slug)-every-tab-stop-is-named-and-visible", timeout: 400) { ui, _, ctx in
            try await ui.firstLaunch(ctx, title: "Quiet Keys")
            try await ui.nav(workspace)
            await ui.idle(1.0)
            try await setup(ui)
            var findings: [StopFinding] = []
            let stops = try await walkTabs(ui, workspace: workspace, into: &findings)
            let json = try JSONEncoder().encode(findings)
            try json.write(to: ui.artifacts.appendingPathComponent("keyboard-and-screen-reader-\(slug).json"))
            try ui.expect(NSApp.isFullKeyboardAccessEnabled || stops > 0, "Tab reaches controls (full keyboard access: \(NSApp.isFullKeyboardAccessEnabled))")
            try ui.expect(findings.isEmpty, "keyboard findings in \(workspace): \(findings.prefix(6).map { "step \($0.step) \($0.problem) [\($0.what)]" })")
            try ui.quiet("in \(workspace) through the accessibility tree")
        }
    }

    static let keyboardScenarios: [Scenario] = [
        keyboardCase("Project Home"),
        keyboardCase("Breakdown"),
        keyboardCase("Schedule"),
        keyboardCase("Call Sheets"),
        keyboardCase("Budget"),
        keyboardCase("Visual Planning"),
        keyboardCase("Files"),
        keyboardCase("Notes and Tasks"),
        keyboardCase("Activity"),
        keyboardCase("Recently Deleted"),
    ]
}




// MARK: reading the screen

fileprivate extension Element {
    /// The tooltip or hint a screen reader speaks after the name.
    var help: String {
        for name in ["accessibilityHelp", "accessibilityHint"] {
            let sel = NSSelectorFromString(name)
            if obj.responds(to: sel), let raw = obj.perform(sel)?.takeUnretainedValue() as? String, !raw.isEmpty { return raw }
        }
        return ""
    }

    /// A selected choice (a colour swatch or segment) says so by its value or its selected state.
    var isSelectedChoice: Bool {
        if value == "Selected" || value == "1" { return true }
        return callBool("isAccessibilitySelected") ?? false
    }
}

fileprivate extension UI {
    struct MarkLook { var hasIcon: Bool; var hasColour: Bool }

    /// Draws the window and reads the pixels around a category mark: a coloured dot, then an icon.
    func markLook(_ mark: Element, dotOffset: CGFloat = 4, iconStart: CGFloat = 13) -> MarkLook {
        let f = mark.frame
        let background = colorAtScreen(NSPoint(x: f.minX - 6, y: f.midY))
        let dot = colorAtScreen(NSPoint(x: f.minX + dotOffset, y: f.midY))
        func differs(_ a: NSColor?, _ b: NSColor?) -> Bool {
            guard let a, let b else { return false }
            return abs(a.redComponent - b.redComponent) + abs(a.greenComponent - b.greenComponent) + abs(a.blueComponent - b.blueComponent) > 0.15
        }
        var iconPixels = 0
        var x = f.minX + iconStart
        while x < f.minX + iconStart + 15 {
            var y = f.midY - 6
            while y < f.midY + 6 {
                if differs(colorAtScreen(NSPoint(x: x, y: y)), background) { iconPixels += 1 }
                y += 1
            }
            x += 1
        }
        return MarkLook(hasIcon: iconPixels > 6, hasColour: differs(dot, background) && (dot?.alphaComponent ?? 0) > 0.5)
    }

    /// The colour of one point on screen as the window draws it.
    func colorAtScreen(_ p: NSPoint) -> NSColor? {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(p) }), let view = w.contentView else { return nil }
        guard let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return nil }
        view.cacheDisplay(in: view.bounds, to: rep)
        let local = view.convert(w.convertPoint(fromScreen: p), from: nil)
        let scale = CGFloat(rep.pixelsWide) / view.bounds.width
        let px = Int(local.x * scale)
        let py = Int((view.isFlipped ? local.y : view.bounds.height - local.y) * scale)
        guard px >= 0, py >= 0, px < rep.pixelsWide, py < rep.pixelsHigh else { return nil }
        return rep.colorAt(x: px, y: py)?.usingColorSpace(.sRGB)
    }

    /// The range in the script pane that "Show in script" marks, if any: the strongest background in the text.
    func scriptJumpHighlight() -> String? {
        for e in all() where e.obj is NSTextView && e.identifier != "screenplay-editor" {
            guard let tv = e.obj as? NSTextView, let storage = tv.textStorage, storage.length > 0 else { continue }
            var found: String?
            storage.enumerateAttribute(.backgroundColor, in: NSRange(location: 0, length: storage.length)) { value, range, _ in
                if let c = (value as? NSColor)?.usingColorSpace(.sRGB), abs(c.alphaComponent - 0.55) < 0.02 { found = storage.attributedSubstring(from: range).string }
            }
            if let found { return found }
        }
        return nil
    }

    /// Sets a date picker without opening the system date picker.
    func setDate(id: String, _ year: Int, _ month: Int, _ day: Int) async throws {
        let e = try await wait(id: id, timeout: 15, "the date field \(id)")
        func findPicker(_ o: NSObject, _ hops: Int) -> NSDatePicker? {
            if let p = o as? NSDatePicker { return p }
            if hops > 6 { return nil }
            for name in ["accessibilityParent", "superview"] where o.responds(to: NSSelectorFromString(name)) {
                if let up = o.perform(NSSelectorFromString(name))?.takeUnretainedValue() as? NSObject, let p = findPicker(up, hops + 1) { return p }
            }
            return nil
        }
        func scan(_ v: NSView) -> NSDatePicker? {
            if let p = v as? NSDatePicker, (Element(obj: p).identifier == id || Element(obj: p).label.contains(e.label.split(separator: ",").first.map(String.init) ?? "\u{0}")) { return p }
            for sv in v.subviews { if let p = scan(sv) { return p } }
            return nil
        }
        var found = findPicker(e.obj, 0)
        if found == nil { for w in NSApp.windows where w.isVisible { if let v = w.contentView?.superview, let p = scan(v) { found = p; break } } }
        guard let picker = found else { throw DriverError("not a date picker: \(e)") }
        var c = DateComponents(); c.year = year; c.month = month; c.day = day; c.hour = 12
        guard let date = Calendar.current.date(from: c) else { throw DriverError("bad date") }
        picker.dateValue = date
        if let action = picker.action { NSApp.sendAction(action, to: picker.target, from: picker) }
        await idle(0.3)
    }
}

fileprivate extension UI {

    /// Exports through the screen: the Export dialog, a format, and the save panel driven to a path.
    func exportTo(kind: String, format: String, _ url: URL) async throws {
        if kind == "dood" {
            try await press(id: "schedule-section-dood")
            try await press(id: "dood-export")
            try await waitText("Export Day Out of Days")
        } else {
            try await press(id: "schedule-section-schedule")
            try await press(id: "schedule-mode-board")
            try await press(id: "schedule-export")
            try await waitText("Export Shooting Schedule")
        }
        try await press(id: "export-format-\(format)")
        try await pressAndSave(id: "export-confirm", to: url)
        try await waitText("Export complete")
        try await press(id: "export-close")
        try await waitGone(id: "export-close", timeout: 10)
    }

    /// Presses a button that opens a save panel and drives the panel: a folder, a name, the Save button.
    /// The panel runs its own loop, so a plain block is queued first to answer it while the press waits.
    func pressAndSave(id: String, to url: URL) async throws {
        let button = try await wait(id: id, timeout: 15, "the button \(id)")
        try? FileManager.default.removeItem(at: url)
        var outcome: String?
        var finished = false
        // A save panel runs the event loop in the modal panel mode, which serves neither timers nor the main queue.
        // A helper thread asks the main run loop, in that mode, to look for the panel and answer it.
        nonisolated(unsafe) var outcomeBox: String?
        nonisolated(unsafe) var finishedBox = false
        let modes: [RunLoop.Mode] = [.default, .common, .eventTracking, RunLoop.Mode("NSModalPanelRunLoopMode")]
        Thread.detachNewThread {
            for _ in 0..<120 {
                Thread.sleep(forTimeInterval: 0.25)
                if finishedBox { return }
                RunLoop.main.perform(inModes: modes) {
                    MainActor.assumeIsolated {
                        guard !finishedBox else { return }
                        if let panel = NSApp.windows.compactMap({ $0 as? NSSavePanel }).first(where: { $0.isVisible }) {
                            outcomeBox = UI.answer(panel, url)
                            finishedBox = true
                        }
                    }
                }
                CFRunLoopWakeUp(CFRunLoopGetMain())
            }
            if !finishedBox { outcomeBox = "no save panel appeared within 30 s"; finishedBox = true }
        }
        log("press \(id) and save to \(url.lastPathComponent)")
        if !button.performPress() { try await click(button) }
        // The press returns when the panel is closed, or at once when the panel opens from a task.
        let deadline = Date().addingTimeInterval(30)
        while !finishedBox, Date() < deadline { await idle(0.1) }
        if let o = outcomeBox, !o.isEmpty { throw DriverError(o) }
        try expect(finishedBox, "the save panel was never answered")
        try await waitUntil("the file \(url.lastPathComponent) exists", timeout: 20) { FileManager.default.fileExists(atPath: url.path) }
    }

    /// Fills a save panel and presses its Save button. Returns a problem, or nil when it went well.
    @MainActor static func answer(_ panel: NSSavePanel, _ url: URL) -> String? {
        panel.directoryURL = url.deletingLastPathComponent()
        panel.nameFieldStringValue = url.lastPathComponent
        func buttons(in view: NSView) -> [NSButton] {
            view.subviews.flatMap { ($0 as? NSButton).map { [$0] } ?? [] } + view.subviews.flatMap { buttons(in: $0) }
        }
        let found = panel.contentView.map { buttons(in: $0) } ?? []
        guard let save = found.first(where: { $0.title == "Save" || $0.title == "Export" }) else {
            return "no Save button in the save panel: \(found.map(\.title))"
        }
        save.performClick(nil)
        return nil
    }
}


// MARK: menus and navigation that SwiftUI hides from the accessibility tree

fileprivate extension UI {
    /// A real click on a point, queued like the mouse does, so a menu opens without blocking the driver.
    /// Presses a toolbar button; when the window is too narrow to show it, the person finds it in
    /// the "more toolbar items" menu, and so does this.
    func pressToolbar(id: String, overflowItem: String) async throws {
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline {
            if let e = query(id: id).first(where: { $0.role == "AXButton" }) { try await press(e); return }
            await idle(0.2)
        }
        log("toolbar button \(id) is in the overflow menu")
        try await menuButton(label: "more toolbar items", item: overflowItem)
    }

    func postClick(atScreen center: NSPoint) {
        let under = NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(center) && !String(describing: Swift.type(of: $0)).contains("PopupMenu") }
        // A sheet lies over its window and takes the click.
        guard let w = under.first(where: { $0.sheetParent != nil }) ?? under.first else { return }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for t in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            if let ev = NSEvent.mouseEvent(with: t, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                           windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: t == .leftMouseDown ? 1 : 0) {
                NSApp.postEvent(ev, atStart: false)
            }
        }
    }

    /// Opens a menu button, chooses the item whose title starts with the text (searching sub-menus), and closes it.
    func menuButton(id: String? = nil, label: String? = nil, item: String, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, timeout: timeout, "the menu button \(id ?? label ?? "?")")
        guard let cell = e.obj as? NSPopUpButtonCell else { throw DriverError("not a menu button: \(e)") }
        log("menu \(id ?? label ?? "?") > \(item)")
        func find(_ m: NSMenu) -> (NSMenu, Int)? {
            for (i, it) in m.items.enumerated() where !it.isSeparatorItem && it.title.hasPrefix(item) { return (m, i) }
            for it in m.items { if let sub = it.submenu, let r = find(sub) { return r } }
            return nil
        }
        var target: (NSMenu, Int)?
        let menusBefore = NSApp.windows.filter { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }.count
        for _ in 0..<3 where target == nil {
            let frame = e.frame
            postClick(atScreen: NSPoint(x: frame.midX, y: frame.midY))
            let deadline = Date().addingTimeInterval(2)
            while Date() < deadline {
                if let m = cell.menu, let t = find(m) { target = t; break }
                await idle(0.1)
            }
            if target == nil { cell.menu?.cancelTracking(); await idle(0.3) }
        }
        guard let (menu, index) = target else {
            let titles = cell.menu?.items.map(\.title) ?? []
            cell.menu?.cancelTracking()
            throw DriverError("no menu item starting with \(item) in \(titles)")
        }
        guard menu.items[index].isEnabled else { cell.menu?.cancelTracking(); throw DriverError("menu item is disabled: \(menu.items[index].title)") }
        menu.performActionForItem(at: index)
        cell.menu?.cancelTracking()
        await idle(0.3)
        // The menu window must be gone, or the next key press and the quit command go to it.
        func openMenus() -> Int { NSApp.windows.filter { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }.count }
        let inToolbar = NSApp.windows.contains { $0.isVisible && e.frame.midY > $0.frame.maxY - 60 && $0.frame.contains(NSPoint(x: e.frame.midX, y: e.frame.midY)) }
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("PopupMenu") && openMenus() > menusBefore { w.orderOut(nil) }
        for _ in 0..<10 where inToolbar && openMenus() > menusBefore {
            for t in [NSEvent.EventType.keyDown, .keyUp] {
                if let ev = NSEvent.keyEvent(with: t, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                             windowNumber: 0, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                    NSApp.postEvent(ev, atStart: true)
                }
            }
            cell.menu?.cancelTracking()
            await idle(0.3)
        }
    }

    /// Chooses an item of a pop-up button (a Picker).
    func popUp(id: String, choose item: String) async throws {
        try await menuButton(id: id, item: item)
    }

    /// Opens a workspace: the Go menu when it has one, otherwise a click on the sidebar row.
    func nav(_ workspace: String) async throws {
        if ProcessInfo.processInfo.environment["E2E_NOBIG"] == nil { bigWindow() }
        if menuItemEnabled(["Go", workspace]) { try await menu(["Go", workspace]); await idle(0.4); return }
        let order = ["Screenplay", "Story", "Idea Vault", "|", "Breakdown", "Schedule", "Call Sheets", "Budget", "Visual Planning", "|", "Project Home", "Files", "Notes and Tasks", "Activity", "Recently Deleted"]
        guard let outline = try? await wait(label: "Sidebar", timeout: 10, "the sidebar"), let view = outline.obj as? NSOutlineView else {
            throw DriverError("no sidebar to click \(workspace)")
        }
        guard let idx = order.firstIndex(of: workspace) else { throw DriverError("unknown workspace \(workspace)") }
        // Rows: three section headers and the workspaces in order.
        let row = idx + 1
        view.window?.makeFirstResponder(view)
        view.scrollRowToVisible(row)
        view.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        await idle(0.6)
    }
}

fileprivate extension UI {
    /// Types into a field after proving that field has the keyboard (the plain driver call trusts the focus request,
    /// and a sheet is a window of its own that may not be the key window).
    /// A real click, with both mouse events queued together (a control that tracks the mouse
    /// until the button comes up would hang the driver's own immediate click).
    func clickQueued(_ e: Element) async throws {
        let frame = e.frame
        guard frame.width > 0, frame.height > 0 else { throw DriverError("element has no frame: \(e)") }
        let center = NSPoint(x: frame.midX, y: frame.midY)
        let under = NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(center) }
        // A sheet lies over its window and takes the click.
        guard let target = under.first(where: { $0.sheetParent != nil }) ?? under.first else { throw DriverError("no window under \(e)") }
        target.makeKeyAndOrderFront(nil)
        let local = target.convertPoint(fromScreen: center)
        for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            guard let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                              windowNumber: target.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: type == .leftMouseDown ? 1 : 0) else { continue }
            NSApp.postEvent(ev, atStart: false)
        }
        await idle(0.4)
    }

    /// Replaces the text of a field in a sheet: focus it, select all, type.
    func setField(_ label: String, _ text: String, id: String? = nil) async throws {
        let e = try await wait(id: id, label: label, timeout: 15)
        // The click gives the field focus; when the app is not the active one (another window
        // took the keyboard) the accessibility focus does the same.
        var focused: NSTextView?
        for attempt in 0..<4 {
            if attempt % 2 == 0 { try await clickQueued(e) } else { try await focus(e); await idle(0.3) }
            if let w = NSApp.keyWindow ?? windows.last, let tv = w.firstResponder as? NSTextView { focused = tv; break }
        }
        guard let editor = focused else { throw DriverError("the field took no focus: \(e)") }
        editor.selectAll(nil)
        editor.insertText(text, replacementRange: editor.selectedRange())
        await idle(0.3)
        let now = try await wait(id: id, label: label, timeout: 5).value
        if now != text { throw DriverError("the field \(label) reads \(now), not \(text)") }
    }

    func typeIn(_ text: String, into id: String? = nil, label: String? = nil, timeout: Double = 15) async throws {
        var e = try await wait(id: id, label: label, timeout: timeout)
        let ready = Date().addingTimeInterval(5)
        while e.frame.width <= 0, Date() < ready { await idle(0.2); e = try await wait(id: id, label: label, timeout: timeout) }
        await reveal(e)
        e = try await wait(id: id, label: label, timeout: timeout)
        func window(of e: Element) -> NSWindow? {
            let c = NSPoint(x: e.frame.midX, y: e.frame.midY)
            let under = NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(c) }
            return under.first(where: { $0.sheetParent != nil }) ?? under.first
        }
        func hasFocus(_ e: Element) -> NSTextView? {
            guard let w = window(of: e), let tv = w.firstResponder as? NSTextView else { return nil }
            let owner: NSObject = (tv.isFieldEditor ? (tv.delegate as? NSObject) : tv) ?? tv
            let o = Element(obj: owner)
            if owner === e.obj || (!e.label.isEmpty && o.label == e.label) || (!e.identifier.isEmpty && o.identifier == e.identifier) { return tv }
            return nil
        }
        var target: NSTextView?
        for attempt in 0..<5 {
            window(of: e)?.makeKeyAndOrderFront(nil)
            if attempt >= 2 { try await focus(e) }
            else if e.frame.width > 0 { postClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)) }
            await idle(0.3)
            if let tv = hasFocus(e) { target = tv; break }
            if e.frame.width > 0 { postClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)); await idle(0.4) }
            if let tv = hasFocus(e) { target = tv; break }
            if attempt >= 3, let w = window(of: e), let tv = w.firstResponder as? NSTextView { target = tv; break }
            e = try await wait(id: id, label: label, timeout: timeout)
        }
        guard let tv = target else { throw DriverError("no text input took focus: \(e)") }
        log("type into \(id ?? label ?? "?"): \(text.count) characters")
        tv.selectAll(nil)
        tv.insertText(text, replacementRange: tv.selectedRange())
        await idle(0.2)
    }
}

fileprivate extension UI {
    /// Makes the main window as large as the screen allows, as a person with a big display has it.
    func bigWindow() {
        guard let w = NSApp.windows.first(where: { $0.isVisible && $0.canBecomeMain }), let screen = w.screen ?? NSScreen.main else { return }
        let vf = screen.visibleFrame
        // The Schedule screen asks for about 1500 points of height; a tall frame keeps all of it on the window.
        let height = ProcessInfo.processInfo.environment["E2E_TALL"] != nil ? CGFloat(1560) : min(860, vf.height - 20)
        let target = NSRect(x: vf.minX + 20, y: vf.maxY - height - 10, width: min(1440, vf.width - 40), height: height)
        if w.frame != target { w.setFrame(target, display: true) }
    }
}

fileprivate extension UI {
    /// A real mouse drag, queued like the mouse does: down, small steps that pass the drag threshold, a pause, up.
    func drag(from a: Element, to b: Element) async throws {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(NSPoint(x: a.frame.midX, y: a.frame.midY)) }) else {
            throw DriverError("no window under \(a) frame \(a.frame); windows \(NSApp.windows.map { "\(Swift.type(of: $0)) \($0.frame) \($0.isVisible)" })")
        }
        w.makeKeyAndOrderFront(nil)
        func post(_ t: NSEvent.EventType, _ screen: NSPoint, pressure: Float) {
            let local = w.convertPoint(fromScreen: screen)
            if let ev = NSEvent.mouseEvent(with: t, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                           windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: pressure) {
                NSApp.postEvent(ev, atStart: false)
            }
        }
        let start = NSPoint(x: a.frame.minX + min(30, a.frame.width / 2), y: a.frame.midY)
        let end = NSPoint(x: b.frame.midX, y: b.frame.midY)
        log("drag from \(a.identifier) to \(b.identifier)")
        post(.leftMouseDown, start, pressure: 1)
        await idle(0.15)
        let steps = 24
        for i in 1...steps {
            let t = CGFloat(i) / CGFloat(steps)
            post(.leftMouseDragged, NSPoint(x: start.x + (end.x - start.x) * t, y: start.y + (end.y - start.y) * t), pressure: 1)
            await idle(0.03)
        }
        await idle(0.3)
        post(.leftMouseUp, end, pressure: 0)
        await idle(0.6)
    }
}

fileprivate extension UI {
    /// Waits until a finder returns an element.
    func waitFor(_ what: String, timeout: Double = 15, _ find: () -> Element?) async throws -> Element {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline { if let e = find() { return e }; await idle(0.1) }
        try? snapshot("missing-\(what.prefix(20))")
        throw DriverError("not found within \(Int(timeout)) s: \(what)")
    }
}

fileprivate extension UI {
    /// Reports every scroll view that is not at its top (a layout that scrolled itself shows here).
    func scrollState() -> String {
        var out: [String] = []
        func walk(_ v: NSView) {
            if let sv = v as? NSScrollView, let doc = sv.documentView {
                out.append("scroll \(sv.frame.size) doc \(doc.frame.size) origin \(sv.contentView.bounds.origin) flipped \(doc.isFlipped)")
            }
            if let t = v as? NSTableView { out.append("table rows \(t.numberOfRows) rowHeight \(t.rowHeight) frame \(t.frame) rect0 \(t.numberOfRows > 0 ? t.rect(ofRow: 0) : .zero)") }
            for c in v.subviews { walk(c) }
        }
        for w in NSApp.windows where w.isVisible { out.append("window \(w.frame) content \(w.contentView?.frame ?? .zero) minSize \(w.minSize) contentMin \(w.contentMinSize) hosting \(String(describing: w.contentView?.subviews.map { "\(Swift.type(of: $0)) \($0.frame)" }))"); if let v = w.contentView { walk(v) } }
        return out.joined(separator: "\n")
    }
}

fileprivate extension UI {
    /// Moves focus off any text field (the window keeps the keyboard, no field has it).
    func blur() async throws {
        (NSApp.keyWindow ?? windows.last)?.makeFirstResponder(nil)
        await idle(1.0)
    }
}

fileprivate extension UI {
    /// Whether the text field with this accessible name accepts typing (nil when no such field is on screen).
    func fieldTakesInput(label: String) -> Bool? {
        func scan(_ v: NSView) -> Bool? {
            if let f = v as? NSTextField, Element(obj: f).label == label { return f.isEnabled && f.isEditable }
            for sv in v.subviews { if let r = scan(sv) { return r } }
            return nil
        }
        for w in NSApp.windows where w.isVisible { if let v = w.contentView?.superview, let r = scan(v) { return r } }
        return nil
    }

    /// Scrolls the scroll view that holds an element until the element is inside the window (the call sheet page, for one).
    func reveal(_ e: Element) async {
        let f = e.frame
        func scan(_ v: NSView) -> NSScrollView? {
            if let sv = v as? NSScrollView, let w = sv.window {
                let r = w.convertToScreen(sv.convert(sv.bounds, to: nil))
                if r.minX <= f.midX && f.midX <= r.maxX, r.height > 100 { return sv }
            }
            for c in v.subviews { if let r = scan(c) { return r } }
            return nil
        }
        for w in NSApp.windows where w.isVisible {
            guard let root = w.contentView?.superview else { continue }
            var found: [NSScrollView] = []
            func collect(_ v: NSView) { if let sv = v as? NSScrollView { found.append(sv) }; for c in v.subviews { collect(c) } }
            collect(root)
            for sv in found {
                guard let win = sv.window, let doc = sv.documentView else { continue }
                let visible = win.convertToScreen(sv.convert(sv.bounds, to: nil))
                if f.midX < visible.minX || f.midX > visible.maxX { continue }
                if f.midY >= visible.minY + 20 && f.midY <= visible.maxY - 20 { return }
                // Move the document so the element sits in the middle of the visible area.
                let delta = f.midY - visible.midY
                var origin = sv.contentView.bounds.origin
                origin.y += doc.isFlipped ? -delta : delta
                origin.y = max(0, min(origin.y, doc.frame.height - sv.contentView.bounds.height))
                sv.contentView.scroll(to: origin)
                sv.reflectScrolledClipView(sv.contentView)
                await idle(0.3)
                return
            }
        }
    }
}
