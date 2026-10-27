// Real-app scenarios for import, export, packages, PDF and the offline AI.

import AppKit
import Foundation

extension Scenarios {
    static let nightBus = """
    Title: Night Bus

    INT. POLICE STATION - NIGHT

    Arjun confesses to Inspector Rao.

    ARJUN
    I was there that night.

    EXT. BUS STOP - NIGHT

    Arjun meets Meera again at a rainy bus stop.

    MEERA
    You came back.

    EXT. RAILWAY STATION - DAY

    Meera waits for the late train with a red umbrella.
    """

    static let nightShift = """
    Title: Night Shift

    INT. CONTROL ROOM - NIGHT

    Kavya studies a flickering monitor and lifts a torch.

    KAVYA
    Someone is in the tunnel.

    EXT. TUNNEL MOUTH - NIGHT

    Ravi drags a heavy toolbox through the rain.

    RAVI
    Hold the gate.
    """

    static let interchangeGroup: [Scenario] = packagesGroup + visualGroup + engineExportGroup + aiGroup + [
        Scenario("63-every-export/screenplay-exports-and-text-formats-import-back") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Out And Back")
            try await ui.importNightBus()
            let dir = ctx.work.appendingPathComponent("exports-63")
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            var files: [String: URL] = [:]
            for (format, ext) in [("pdf", "pdf"), ("fdx", "fdx"), ("fountain", "fountain"), ("docx", "docx"), ("txt", "txt")] {
                let url = dir.appendingPathComponent("Night Bus.\(ext)")
                try await ui.exportScreenplay(format: format, to: url)
                files[format] = url
                let size = (try? FileManager.default.attributesOfItem(atPath: url.path)[.size] as? Int) ?? 0
                try ui.expect(size >= 100, "\(format) file holds at least 100 bytes, has \(size)")
            }
            let pdf = try Data(contentsOf: files["pdf"]!)
            try ui.expect(pdf.prefix(5) == Data("%PDF-".utf8), "the PDF starts with %PDF-")
            let fountain = try String(contentsOf: files["fountain"]!, encoding: .utf8)
            try ui.expect(fountain.contains("You came back."), "the Fountain file contains the line: \(fountain.prefix(200))")
            for (format, label) in [("fdx", "Final Draft"), ("fountain", "Fountain"), ("docx", "DOCX"), ("txt", "plain text")] {
                let draft = "Back from \(label)"
                let preview = try await ui.importFile(files[format]!, draftName: draft)
                try ui.expect(preview.contains("3"), "\(format): the preview shows 3 scenes: \(preview)")
                try ui.expect(ui.screenText().contains("BUS STOP"), "\(format): a scene heading contains BUS STOP")
                try await ui.tap(id: "import-confirm")
                try await ui.waitText("Import complete", timeout: 30)
                try await ui.waitText(draft, timeout: 10)
                try await ui.tap(id: "import-open-screenplay")
                let tv = try await ui.editor()
                try await ui.waitText(draft, timeout: 10)
                try ui.expect(tv.string.contains("You came back."), "\(format): an element reads You came back.: \(tv.string.prefix(200))")
                try ui.expect(tv.string.contains("BUS STOP"), "\(format): a scene heading contains BUS STOP")
                let scenes = tv.string.components(separatedBy: "\n").filter { $0.hasPrefix("INT.") || $0.hasPrefix("EXT.") }
                try ui.expect(scenes.count == 3, "\(format): the new draft has 3 scenes, found \(scenes)")
            }
        },
        Scenario("63-every-export/story-board-exports-as-pdf") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Out And Back")
            try await ui.importNightBus()
            try await ui.go("Story")
            try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
            try await ui.tap(label: "Add Act", role: "AXButton")
            try await ui.focusByClick(try await ui.wait(id: "inline-Act title"))
            try await ui.typeHere("Act One")
            try await ui.enter()
            try await ui.idle(0.5)
            try ui.snapshot("act")
            try await ui.choose(from: "story-add-menu", ["Scene Card"])
            try await ui.focusByClick(try await ui.wait(id: "inline-Short description"))
            try await ui.typeHere("Meera waits at the bus stop")
            try await ui.enter()
            try await ui.waitText("Meera waits at the bus stop", timeout: 10)
            let pdf = ctx.work.appendingPathComponent("story-board-63.pdf")
            try await ui.choosing(pdf) { try await ui.choose(from: "story-more", ["Export Outline", "PDF"]) }
            try await ui.waitText("Exported", timeout: 30)
            let data = try Data(contentsOf: pdf)
            try ui.expect(data.count >= 500, "the Beat Board PDF holds at least 500 bytes, has \(data.count)")
            try ui.expect(data.prefix(5) == Data("%PDF-".utf8), "the Beat Board file starts with %PDF-")
        },
        Scenario("63-every-export/diagnostics-bundle-holds-no-project-text") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Out And Back")
            try await ui.importNightBus()
            try await ui.go("Story")
            try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
            try await ui.tap(label: "Add Act", role: "AXButton")
            try await ui.focusByClick(try await ui.wait(id: "inline-Act title"))
            try await ui.typeHere("Act One")
            try await ui.enter()
            try await ui.choose(from: "story-add-menu", ["Scene Card"])
            try await ui.focusByClick(try await ui.wait(id: "inline-Short description"))
            try await ui.typeHere("Meera waits at the bus stop")
            try await ui.enter()
            try await ui.waitText("Meera waits at the bus stop", timeout: 10)
            try await ui.openSettings(tab: "Diagnostics")
            let zip = ctx.work.appendingPathComponent("diagnostics-63.zip")
            try await ui.choosing(zip) { try await ui.tap(id: "export-diagnostics") }
            try await ui.waitText("diagnostics", timeout: 30)
            let deadline = Date().addingTimeInterval(30)
            while !FileManager.default.fileExists(atPath: zip.path), Date() < deadline { await ui.idle(0.2) }
            let data = try Data(contentsOf: zip)
            try ui.expect(data.prefix(2) == Data("PK".utf8), "the diagnostics file is a zip (starts with PK)")
            for secret in ["Meera waits at the bus stop", "Arjun", "Inspector Rao", "Meera"] {
                try ui.expect(data.range(of: Data(secret.utf8)) == nil, "the bundle bytes do not contain \"\(secret)\"")
            }
            let listing = ui.run("/usr/bin/unzip", ["-p", zip.path])
            for secret in ["Meera", "Arjun", "Inspector Rao", "bus stop"] {
                try ui.expect(!listing.contains(secret), "the unpacked bundle does not contain \"\(secret)\"")
            }
            try await ui.closeAISettings()
        },
    ]
}

fileprivate extension UI {
    /// A key press for the sheet on top (or the key window): the sheet is made key first, as a
    /// click on it would, then the key goes through the window's key equivalents and key handling.
    func keyToSheet(_ chars: String, code: UInt16, _ mods: NSEvent.ModifierFlags = []) async throws {
        let main = NSApp.windows.first(where: { $0.isVisible && $0.attachedSheet != nil })
        var w: NSWindow? = main?.attachedSheet
        while let inner = w?.attachedSheet { w = inner }
        let target = w ?? NSApp.keyWindow ?? windows.last
        guard let target else { throw DriverError("no window for a key press") }
        target.makeKeyAndOrderFront(nil)
        await idle(0.2)
        log("key \(code) to \(Swift.type(of: target)) first responder \(target.firstResponder.map { String(describing: Swift.type(of: $0)) } ?? "none")")
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            guard let ev = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: mods, timestamp: ProcessInfo.processInfo.systemUptime,
                                            windowNumber: target.windowNumber, context: nil, characters: chars, charactersIgnoringModifiers: chars,
                                            isARepeat: false, keyCode: code) else { continue }
            if type == .keyDown && !mods.isEmpty { log("key equivalent handled: \(target.performKeyEquivalent(with: ev))") } else { NSApp.sendEvent(ev) }
        }
        await idle(0.3)
    }

    /// Presses a control the way a person does: the accessibility press if the control takes it,
    /// else a real click. The click is queued (mouse down and up together), because a segmented
    /// control or a text field tracks the mouse until the button comes up.
    func tap(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 15) async throws {
        try await tap(try await wait(id: id, label: label, role: role, timeout: timeout))
    }

    func tap(_ e: Element) async throws {
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        guard e.isEnabled else { throw DriverError("control is disabled: \(e)") }
        if e.performPress() { await idle(); return }
        try await focusByClick(e)
    }

    /// Opens Settings (Command+Comma) on a tab.
    func openSettings(tab: String? = nil) async throws {
        // The app menu's Settings item is the same command as Command+Comma and works without an active app.
        guard let appMenu = NSApp.mainMenu?.items.first?.title else { throw DriverError("no app menu") }
        try await menu([appMenu, "Settings\u{2026}"])
        if let tab {
            let deadline = Date().addingTimeInterval(10)
            while Date() < deadline {
                if let e = query(label: tab).first(where: { ["AXRadioButton", "AXButton", "AXTab"].contains($0.role) }) { log("press \(tab)"); try await tap(e); await idle(0.5); return }
                await idle(0.2)
            }
            throw DriverError("the Settings tab \(tab) did not show: \(all().map(\.description).filter { $0.contains("Radio") || $0.contains("Tab") })")
        }
    }

    /// Closes the Settings window the way a person does (Command+W).
    func closeAISettings() async throws {
        for w in NSApp.windows where w.isVisible && ["General", "Appearance", "Offline AI", "Diagnostics"].contains(w.title) { w.performClose(nil) }
        await idle(0.3)
    }

    /// Runs a tool and returns what it printed.
    func run(_ path: String, _ args: [String]) -> String {
        (try? runThrowing(path, args)) ?? ""
    }

    func runThrowing(_ path: String, _ args: [String]) throws -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: path)
        p.arguments = args
        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = Pipe()
        try p.run()
        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        return String(decoding: data, as: UTF8.self)
    }

    /// Starts the app and makes the window big, as a person drags the window edge: a narrow window
    /// cuts the right end of the Story and Production toolbars off.
    func launch(_ ctx: Context) async throws {
        try await startApp(ctx)
        if let w = NSApp.windows.first(where: { $0.isVisible && $0.canBecomeMain }), let screen = w.screen ?? NSScreen.main {
            let area = screen.visibleFrame
            let size = NSSize(width: min(1500, area.width), height: min(950, area.height))
            w.setFrame(NSRect(x: area.minX + 10, y: area.maxY - size.height - 10, width: size.width, height: size.height), display: true)
            await idle(0.5)
        }
    }

    /// A real click that puts the keyboard focus in a text field. Mouse down in a text field starts a
    /// tracking loop that waits for the mouse up, so both events are queued, not sent one by one.
    func focusByClick(_ e: Element) async throws {
        let frame = e.frame
        guard frame.width > 0, frame.height > 0 else { throw DriverError("element has no frame: \(e)") }
        let center = NSPoint(x: frame.midX, y: frame.midY)
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }) else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            guard let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                              windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: type == .leftMouseDown ? 1 : 0) else { continue }
            NSApp.postEvent(ev, atStart: false)
        }
        await idle(0.4)
    }

    /// Chooses an item from a pop-up menu on screen (a SwiftUI Menu button). The button is pressed
    /// the way a person does, which opens the menu; while it is open the item is performed. The
    /// open menu runs its own tracking loop, so a run loop timer picks the item.
    /// `path` walks into submenus.
    func choose(from id: String, _ path: [String], label: String? = nil) async throws {
        for attempt in 1...3 {
            do { try await chooseOnce(from: id, path, label: label); return }
            catch let e as DriverError where attempt < 3 && e.description.contains("did not open") {
                log("the menu did not open, trying again (\(attempt))")
                await idle(1.0)
            }
        }
    }

    private func chooseOnce(from id: String, _ path: [String], label: String? = nil) async throws {
        let button = try await wait(id: label == nil ? id : nil, label: label, role: nil, timeout: 15, "the menu button \(id)")
        log("choose \(path.joined(separator: " > ")) from \(id)")
        guard let rootMenu = button.obj.value(forKey: "menu") as? NSMenu else { throw DriverError("no menu behind \(button)") }
        let box = MenuOutcome()
        let started = Date()
        let timer = Timer(timeInterval: 0.1, repeats: true) { _ in
            MainActor.assumeIsolated {
                if box.done { return }
                if Date().timeIntervalSince(started) > 8 { box.failure = "the menu never showed items"; box.done = true; rootMenu.cancelTracking(); return }
                guard !rootMenu.items.isEmpty else { return }
                var menu = rootMenu
                for (i, title) in path.enumerated() {
                    guard let item = menu.items.first(where: { $0.title == title || $0.title.hasPrefix(title) }) else {
                        box.failure = "no menu item \(title) in \(menu.items.map(\.title))"; box.done = true; rootMenu.cancelTracking(); return
                    }
                    if i == path.count - 1 {
                        guard item.isEnabled else { box.failure = "menu item is disabled: \(title)"; box.done = true; rootMenu.cancelTracking(); return }
                        box.done = true
                        rootMenu.cancelTracking()
                        menu.performActionForItem(at: menu.index(of: item))
                    } else {
                        guard let sub = item.submenu else { box.failure = "\(title) has no submenu"; box.done = true; rootMenu.cancelTracking(); return }
                        menu = sub
                    }
                }
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        defer { timer.invalidate() }
        if let control = (button.obj as? NSCell)?.controlView as? NSControl {
            control.performClick(nil)
        } else if !button.performPress() {
            try await click(button)
        }
        if let f = box.failure { throw DriverError(f) }
        if !box.done { throw DriverError("the menu did not open: \(button)") }
        await idle(0.3)
    }

    /// The shared step "Import the Night Bus screenplay" (Night Shift for other scenarios).
    func importScript(_ text: String) async throws {
        try await menu(["File", "Import Screenplay…"])
        try await type(text, into: "import-paste")
        try await tap(id: "import-preview")
        try await waitText("Import Screenplay · Preview", timeout: 30)
        try await tap(id: "import-confirm")
        try await waitText("Import complete", timeout: 30)
        try await tap(id: "import-open-screenplay")
        _ = try await editor()
    }

    /// The Export Screenplay dialog, a format, the Save panel answered with `url`, and the report.
    func exportScreenplay(format: String, to url: URL, closeReport: Bool = true) async throws {
        try await menu(["File", "Export Screenplay…"])
        try await wait(id: "export-confirm", timeout: 20)
        if format == "txt" { try await tap(id: "export-more-formats") }
        try await tap(id: "export-format-\(format)")
        try await choosing(url) { try await tap(id: "export-confirm") }
        try await waitText("Export complete", timeout: 60)
        try expect(FileManager.default.fileExists(atPath: url.path), "the \(format) file exists on disk")
        if closeReport { try await tap(id: "export-close"); try await waitGone(id: "export-sheet") }
    }

    /// File > Import Screenplay, Browse with the file chosen, Preview. Returns the scene count chip text.
    @discardableResult
    func importFile(_ url: URL, draftName: String? = nil) async throws -> String {
        try await menu(["File", "Import Screenplay…"])
        try await choosing(url) { try await tap(id: "import-browse") }
        try await tap(id: "import-preview")
        try await waitText("Import Screenplay · Preview", timeout: 30)
        if let draftName { try await type(draftName, into: "import-draft-name") }
        return query(id: "import-fact-scenes").map(\.text).joined(separator: " ")
    }

    func importNightBus() async throws { try await importScript(Scenarios.nightBus) }
}

@MainActor private final class MenuOutcome {
    var done = false
    var failure: String?
}

// MARK: findings and quiet

/// Checks that let a scenario run on to its end and report every miss together.
@MainActor final class Findings {
    private(set) var items: [String] = []
    func check(_ ok: Bool, _ message: @autoclosure () -> String) { if !ok { items.append(message()) } }
    func add(_ message: String) { items.append(message) }
    func finish() throws {
        if !items.isEmpty { throw DriverError("\(items.count) finding(s): " + items.joined(separator: " | ")) }
    }
}

fileprivate extension UI {
    func subrole(_ e: Element) -> String {
        let sel = NSSelectorFromString("accessibilitySubrole")
        guard e.obj.responds(to: sel), let raw = e.obj.perform(sel)?.takeUnretainedValue() as? String else { return "" }
        return raw
    }

    /// "Check: the screen is quiet": every visible control has an accessible name, and no system
    /// panel opened that nobody asked for. (Console warnings are not visible from outside the app.)
    func quiet(_ context: String = "") throws {
        let named = ["AXButton", "AXMenuButton", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXTextField", "AXTextArea", "AXLink"]
        let unnamed = all().filter { e in
            named.contains(e.role) && e.text.isEmpty
                && !["AXCloseButton", "AXMinimizeButton", "AXZoomButton", "AXFullScreenButton", "AXToolbarButton",
                     "AXIncrementArrow", "AXDecrementArrow", "AXIncrementPage", "AXDecrementPage"].contains(subrole(e))
        }
        try expect(unnamed.isEmpty, "\(context) the screen is not quiet, controls without a name: \(unnamed.prefix(6).map(\.description))")
        try expect(unexpectedPanels().isEmpty, "\(context) a system panel opened that nobody asked for: \(unexpectedPanels())")
    }

    func fileSize(_ url: URL) -> Int { (try? FileManager.default.attributesOfItem(atPath: url.path)[.size] as? Int) ?? 0 }

    func waitFile(_ url: URL, timeout: Double = 30) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if FileManager.default.fileExists(atPath: url.path) { return }
            await idle(0.2)
        }
        throw DriverError("the file does not exist on disk: \(url.path)")
    }

    /// The text of the element with this identifier.
    /// Opens "See sources" under the newest answer, when it has sources.
    func openLastSources() async throws {
        if let button = query(id: "ai-see-sources").last { try await tap(button) }
    }

    func textOf(id: String) -> String { query(id: id).map(\.text).joined(separator: " ") }
}

// MARK: 70-packages

fileprivate extension UI {
    /// First launch of the packages specs: project 'Harbour Lights' with the Night Shift screenplay.
    /// Adds a private note to the scene shown in the Scene Hub (Screenplay Tools menu, Scene Hub).
    func addPrivateNote(_ text: String) async throws {
        try await choose(from: "screenplay-tools-menu", ["Scene Hub"])
        try await tap(label: "Add private note", role: "AXButton")
        try await type(text, label: "Private note")
        try await tap(label: "Save", role: "AXButton")
        try await waitText(text, timeout: 10)
    }

    func harbourStart(_ ctx: Context) async throws {
        try await launch(ctx)
        try await createProject("Harbour Lights")
        try await importScript(Scenarios.nightShift)
    }

    func packageDir(_ ctx: Context) throws -> URL {
        let dir = ctx.work.appendingPathComponent("package-out")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    /// File > Back Up Project, Browse (answered with `url`), Create Backup, Close.
    @discardableResult
    func makeBackup(to url: URL) async throws -> PanelInfo {
        try await menu(["File", "Back Up Project…"])
        try await waitText("A recoverable copy of supported project state.", timeout: 20)
        let info = try await choosing(url) { try await tap(id: "backup-browse") }
        try await tap(id: "backup-create")
        try await waitText("Backup created.", timeout: 60)
        try await waitFile(url)
        try await tap(id: "backup-close")
        try await waitGone(id: "backup-sheet")
        return info
    }

    func makeReviewPackage(to url: URL) async throws {
        try await menu(["File", "Share for Review", "Export Review Package (script)…"])
        try await waitText("Script Review Package", timeout: 20)
        try await waitText("Private notes", timeout: 20)
        try await choosing(url) { try await tap(id: "exchange-export-confirm") }
        try await waitText("Package exported.", timeout: 30)
        try await tap(id: "exchange-export-close")
        try await waitGone(id: "exchange-export-sheet")
    }

    func closeToHome() async throws {
        try await menu(["File", "Close Project"])
        try await waitHome()
    }

    /// The Restore Backup or Package dialog from Application Home, up to the choice sheet.
    func openPackageSheet(_ file: URL) async throws -> PanelInfo {
        try await menu(["File", "Restore Backup or Package…"])
        try await wait(id: "open-project-choose", timeout: 20)
        return try await choosing(file) { try await tap(id: "open-project-choose") }
    }
}

extension Scenarios {
    static let packagesGroup: [Scenario] = [
        Scenario("70-packages/create-backup-through-save-dialog") { ui, _, ctx in
            let f = Findings()
            let dir = try ui.packageDir(ctx)
            try await ui.harbourStart(ctx)
            // Steps 3 to 6: the private note, in the Scene Hub of the Screenplay Tools menu.
            try await ui.addPrivateNote("Harbour private note for the restore check")
            try await ui.menu(["File", "Back Up Project…"])
            try await ui.waitText("A recoverable copy of supported project state.", timeout: 20)
            let backup = dir.appendingPathComponent("Harbour Lights backup.ofbackup")
            let info = try await ui.choosing(backup) { try await ui.tap(id: "backup-browse") }
            f.check(info.title == "Save backup to", "the Browse panel is titled \"Save backup to\", was \"\(info.title)\"")
            try await ui.waitText(backup.path, timeout: 10)
            f.check(ui.textOf(id: "backup-location").contains(backup.path), "the Backup location field shows the full path, shows \"\(ui.textOf(id: "backup-location"))\"")
            try await ui.tap(id: "backup-create")
            try await ui.waitText("Backup created.", timeout: 60)
            f.check(FileManager.default.fileExists(atPath: backup.path), "the backup file exists on disk")
            try await ui.tap(id: "backup-close")
            try await ui.waitGone(id: "backup-sheet")
            try ui.quiet("after the backup:")
            try f.finish()
        },
        Scenario("70-packages/export-full-project-and-review-package") { ui, _, ctx in
            let f = Findings()
            let dir = try ui.packageDir(ctx)
            try await ui.harbourStart(ctx)
            try await ui.makeBackup(to: dir.appendingPathComponent("Harbour Lights backup.ofbackup"))
            try await ui.menu(["File", "Export Full Project…"])
            try await ui.waitText("Private project transfer.", timeout: 20)
            let project = dir.appendingPathComponent("Harbour Lights.ofproject")
            let info = try await ui.choosing(project) { try await ui.tap(id: "export-project-confirm") }
            f.check(info.title == "Export Project Package", "the panel is titled \"Export Project Package\", was \"\(info.title)\"")
            try await ui.waitText("Project package exported.", timeout: 60)
            try await ui.tap(id: "export-project-close")
            try await ui.waitGone(id: "export-project-sheet")
            f.check(FileManager.default.fileExists(atPath: project.path), "the project package file exists on disk")
            try await ui.menu(["File", "Share for Review", "Export Review Package (script)…"])
            try await ui.waitText("Script Review Package", timeout: 20)
            try await ui.waitText("Private notes", timeout: 20)
            let review = dir.appendingPathComponent("Harbour Lights review.ofscriptreview")
            let info2 = try await ui.choosing(review) { try await ui.tap(id: "exchange-export-confirm") }
            f.check(info2.title == "Export Screenplay review package", "the panel is titled \"Export Screenplay review package\", was \"\(info2.title)\"")
            try await ui.waitText("Package exported.", timeout: 30)
            try await ui.tap(id: "exchange-export-close")
            try await ui.waitGone(id: "exchange-export-sheet")
            f.check(FileManager.default.fileExists(atPath: review.path), "the review package file exists on disk")
            try await ui.menu(["File", "Import & Export History…"])
            try await ui.wait(id: "history-tabs", timeout: 20)
            try await ui.tap(label: "Package log")
            try await ui.waitText("Created backup", timeout: 10)
            try await ui.waitText("Exported project package", timeout: 10)
            try await ui.keyToSheet("\u{1b}", code: 53)
            try await ui.waitGone(id: "history-tabs", timeout: 10)
            try ui.quiet("after the exports:")
            try f.finish()
        },
        Scenario("70-packages/review-round-trip-comment-respond-import") { ui, _, ctx in
            let f = Findings()
            let dir = try ui.packageDir(ctx)
            try await ui.harbourStart(ctx)
            let review = dir.appendingPathComponent("Harbour Lights review.ofscriptreview")
            try await ui.makeReviewPackage(to: review)
            let info = try await ui.choosing(review) { try await ui.menu(["File", "Open Review Package…"]) }
            f.check(info.title == "Open Review Package", "the panel is titled \"Open Review Package\", was \"\(info.title)\"")
            try await ui.waitText("Review package. Comments are separate from the original project.", timeout: 20)
            try await ui.type("Tighten the harbour line.", into: "review-comment-field")
            try await ui.idle(0.6)
            try await ui.keyToSheet("\r", code: 36, [.command])
            try await ui.waitText("Yours", timeout: 10)
            let response = dir.appendingPathComponent("Harbour Lights reply.ofresponse")
            let info2 = try await ui.choosing(response) { try await ui.tap(label: "Export Response Package", role: "AXButton") }
            f.check(info2.title == "Export Response Package", "the panel is titled \"Export Response Package\", was \"\(info2.title)\"")
            try await ui.waitText("Response package exported.", timeout: 30)
            f.check(FileManager.default.fileExists(atPath: response.path), "the response package file exists on disk")
            try await ui.tap(id: "review-close")
            try await ui.waitGone(id: "review-viewer")
            try await ui.menu(["File", "Import Exchange Package…"])
            try await ui.wait(id: "exchange-import-choose", timeout: 20)
            let info3 = try await ui.choosing(response) { try await ui.tap(id: "exchange-import-choose") }
            f.check(info3.title == "Import Exchange Package", "the panel is titled \"Import Exchange Package\", was \"\(info3.title)\"")
            try await ui.wait(id: "exchange-changes", timeout: 30)
            // Show Comments: open the comments group of the preview.
            let expand = ui.all().first { $0.identifier.hasPrefix("exchange-expand-") && $0.label.contains("omment") }
            if let expand { try await ui.tap(expand) } else { f.add("the preview has no comments group to show: \(ui.all().filter { $0.identifier.hasPrefix("exchange-expand-") }.map(\.description))") }
            try await ui.waitText("Tighten the harbour line.", timeout: 10)
            try await ui.tap(id: "exchange-accept-safe")
            try await ui.waitText("Imported", timeout: 30)
            try await ui.tap(id: "exchange-import-close")
            try await ui.waitGone(id: "exchange-import-sheet")
            try ui.quiet("after the response import:")
            try f.finish()
        },
        Scenario("70-packages/restore-backup-collision-opens-a-copy") { ui, _, ctx in
            let f = Findings()
            let dir = try ui.packageDir(ctx)
            try await ui.harbourStart(ctx)
            try await ui.addPrivateNote("Harbour private note for the restore check")
            let backup = dir.appendingPathComponent("Harbour Lights backup.ofbackup")
            try await ui.makeBackup(to: backup)
            try await ui.closeToHome()
            let info = try await ui.openPackageSheet(backup)
            f.check(info.title == "Open Project Package", "the panel is titled \"Open Project Package\", was \"\(info.title)\"")
            try await ui.waitText("This project already exists on this computer", timeout: 30)
            f.check(ui.screenText().contains("The private note in this package will become yours."), "the sheet says the private note will become yours")
            try await ui.waitText("Open as Copy", timeout: 10)
            try await ui.tap(label: "Open as Copy", role: "AXButton")
            try await ui.waitText("Opened as a copy.", timeout: 60)
            try await ui.tap(id: "open-project-close")
            try await ui.waitGone(id: "open-project-sheet")
            let title = NSApp.windows.filter(\.isVisible).map(\.title).joined(separator: " | ")
            f.check(title.contains("Harbour Lights (copy)"), "the window title names the copy: \(title)")
            try ui.quiet("after the restore:")
            try f.finish()
        },
        Scenario("70-packages/damaged-backup-is-refused") { ui, _, ctx in
            let f = Findings()
            let dir = try ui.packageDir(ctx)
            try await ui.harbourStart(ctx)
            let damaged = dir.appendingPathComponent("damaged.ofbackup")
            var bytes = Data("PK".utf8)
            bytes.append(contentsOf: [0x03, 0x04])
            bytes.append(Data(" this is not a whole package".utf8))
            try bytes.write(to: damaged)
            try await ui.closeToHome()
            try await ui.wait(id: "project-card-Harbour Lights", timeout: 20)
            let before = ui.all().map(\.identifier).filter { $0.hasPrefix("project-card-") }.sorted()
            try await ui.menu(["File", "Restore Backup or Package…"])
            try await ui.wait(id: "open-project-choose", timeout: 20)
            try await ui.choosing(damaged) { try await ui.tap(id: "open-project-choose") }
            try await ui.wait(id: "open-project-error", timeout: 30)
            let said = ui.textOf(id: "open-project-error").lowercased()
            f.check(said.contains("package") || said.contains("archive"), "the message names a package or archive: \"\(said)\"")
            f.check(ui.exists(id: "open-project-choose"), "Choose Package… is still offered")
            try await ui.tap(id: "open-project-cancel")
            try await ui.waitGone(id: "open-project-sheet")
            let after = ui.all().map(\.identifier).filter { $0.hasPrefix("project-card-") }.sorted()
            f.check(before == after && !before.isEmpty, "the project cards are the same: \(before) then \(after)")
            try f.finish()
        },
        Scenario("70-packages/relaunch-lists-original-and-copy", phases: 2) { ui, phase, ctx in
            let dir = try ui.packageDir(ctx)
            if phase == 1 {
                try await ui.harbourStart(ctx)
                let backup = dir.appendingPathComponent("Harbour Lights backup.ofbackup")
                try await ui.makeBackup(to: backup)
                try await ui.closeToHome()
                try await ui.openPackageSheet(backup)
                try await ui.waitText("Open as Copy", timeout: 30)
                try await ui.tap(label: "Open as Copy", role: "AXButton")
                try await ui.waitText("Opened as a copy.", timeout: 60)
                try await ui.tap(id: "open-project-close")
                try await ui.waitGone(id: "open-project-sheet")
            } else {
                try await ui.startApp(ctx)
                let ids = ui.all().map(\.identifier).filter { $0.hasPrefix("project-card-") }
                try ui.expect(ids.contains("project-card-Harbour Lights (copy)"), "Application Home lists the copy: \(ids)")
                try ui.expect(ids.contains("project-card-Harbour Lights"), "Application Home lists the original: \(ids)")
            }
        },
    ]
}

// MARK: 86-visual-planning

import ApplicationServices

fileprivate extension UI {

    /// A real mouse drag in screen coordinates (Cocoa, origin bottom left): press, a few small
    /// moves, the move to the target, release. The events go straight to the window, one after the
    /// other, with the time between them a hand takes.
    func dragAlong(from a: NSPoint, to b: NSPoint, steps: Int = 14) async throws {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(a) }) else { throw DriverError("no window under the drag start \(a)") }
        w.makeKeyAndOrderFront(nil)
        func send(_ type: NSEvent.EventType, _ p: NSPoint) {
            guard let ev = NSEvent.mouseEvent(with: type, location: w.convertPoint(fromScreen: p), modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                              windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: type == .leftMouseUp ? 0 : 1) else { return }
            NSApp.sendEvent(ev)
        }
        send(.leftMouseDown, a)
        await idle(0.15)
        for i in 1...steps {
            let t = CGFloat(i) / CGFloat(steps)
            send(.leftMouseDragged, NSPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t))
            await idle(0.04)
        }
        await idle(0.1)
        send(.leftMouseUp, b)
        await idle(0.5)
    }

    func center(_ e: Element) -> NSPoint { NSPoint(x: e.frame.midX, y: e.frame.midY) }

    /// Moodboards and Storyboards are Production tabs (as on Windows).
    func openVisual(_ tab: String) async throws {
        try await goProduction(tab, id: tab.lowercased())
    }

    /// The storyboard panels in reading order, as "Panel N, picture" or "Panel N, empty".
    func panelKinds() -> [String] {
        all().filter { $0.identifier.hasPrefix("panel-") && !$0.identifier.hasPrefix("panel-shot") && $0.label.hasPrefix("Panel ") }
            .sorted { (a, b) in abs(a.frame.maxY - b.frame.maxY) > 20 ? a.frame.maxY > b.frame.maxY : a.frame.minX < b.frame.minX }
            .map { $0.label.contains("picture") ? "picture" : $0.label.contains("empty") ? "empty" : $0.label }
    }

    func prepareMoodboard() async throws {
        try await openVisual("Moodboards")
        try await tap(id: "moodboard-new")
        try await type("Overall Look", into: "mb-name")
        try await tap(id: "moodboard-new-sheet-primary")
        try await wait(id: "moodboard-add-note", timeout: 15)
        try await tap(id: "moodboard-add-note")
        try await focusByClick(try await wait(id: "new-note"))
        try await typeHere("Cold blue light, warm windows")
        try await idle(0.3)
        try await tap(id: "moodboard-note-sheet-primary")
        _ = try await wait(label: "Note: Cold blue light, warm windows", timeout: 15)
    }

    /// Where the note tile is drawn, in top-left screen points. The accessibility frame of a canvas
    /// tile is the frame of the whole scroll content, so the tile is found the way a person finds it,
    /// by looking: the one block on the canvas that differs from the canvas colour.
    func noteTileBounds() async throws -> CGRect {
        guard let w = NSApp.windows.first(where: { $0.isVisible && $0.canBecomeMain }), let screen = w.screen ?? NSScreen.main,
              let view = w.contentView?.superview ?? w.contentView,
              let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { throw DriverError("no window to look at") }
        view.cacheDisplay(in: view.bounds, to: rep)
        let scale = CGFloat(rep.pixelsWide) / view.bounds.width
        func color(_ x: CGFloat, _ y: CGFloat) -> NSColor { (rep.colorAt(x: Int(x * scale), y: Int(y * scale)) ?? .black).usingColorSpace(.sRGB) ?? .black }
        let bg = color(480, 215)
        var minX = CGFloat.infinity, minY = CGFloat.infinity, maxX = -CGFloat.infinity, maxY = -CGFloat.infinity
        var y: CGFloat = 215
        while y < 830 {
            var x: CGFloat = 480
            while x < min(view.bounds.width - 30, 1480) {
                let c = color(x, y)
                if abs(c.redComponent - bg.redComponent) > 0.03 || abs(c.greenComponent - bg.greenComponent) > 0.03 || abs(c.blueComponent - bg.blueComponent) > 0.03 {
                    minX = min(minX, x); maxX = max(maxX, x); minY = min(minY, y); maxY = max(maxY, y)
                }
                x += 4
            }
            y += 4
        }
        guard minX < maxX else { throw DriverError("no note tile is drawn on the canvas") }
        let f = w.frame
        let top = screen.frame.maxY - f.maxY
        return CGRect(x: f.minX + minX, y: top + minY, width: maxX - minX, height: maxY - minY)
    }

    func dragNoteTile() async throws -> (before: CGRect, after: CGRect) {
        _ = try await wait(label: "Note: Cold blue light, warm windows", timeout: 15)
        let before = try await noteTileBounds()
        // Cocoa screen points have the origin at the bottom left; the hit test uses the top left.
        let screenH = (NSApp.windows.first?.screen ?? NSScreen.main)?.frame.maxY ?? 0
        let start = NSPoint(x: before.midX, y: screenH - before.midY)
        log("drag start \(start) tile \(before) screenH \(screenH) windows \(NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(start) }.map { "\(Swift.type(of: $0)) \($0.frame)" })")
        try await dragAlong(from: start, to: NSPoint(x: start.x + 160, y: start.y - 90))
        let after = try await noteTileBounds()
        return (before, after)
    }

    func drawTwoStrokesAndSave() async throws {
        try await choose(from: "storyboard-add-panel", ["Draw / Sketch"])
        let sheet = try await wait(id: "sketch-sheet", timeout: 15)
        let f = sheet.frame
        try await dragAlong(from: NSPoint(x: f.minX + f.width * 0.3, y: f.minY + f.height * 0.35), to: NSPoint(x: f.minX + f.width * 0.7, y: f.minY + f.height * 0.65))
        try await dragAlong(from: NSPoint(x: f.minX + f.width * 0.3, y: f.minY + f.height * 0.65), to: NSPoint(x: f.minX + f.width * 0.7, y: f.minY + f.height * 0.35))
        try await tap(id: "sketch-undo")
        try await tap(id: "sketch-save")
        try await waitGone(id: "sketch-sheet", timeout: 15)
    }
}

extension Scenarios {
    static let visualGroup: [Scenario] = [
        Scenario("86-visual-planning/moodboard-note-is-dragged") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Pictures First")
            try await ui.importNightBus()
            try await ui.prepareMoodboard()
            let (before, after) = try await ui.dragNoteTile()
            try ui.expect(after.midX - before.midX > 100, "the tile moved more than 100 pixels right: \(before) to \(after)")
            try ui.expect(after.midY - before.midY > 50, "the tile moved more than 50 pixels down: \(before) to \(after)")
        },
        Scenario("86-visual-planning/sketch-dialog-draws-undoes-and-saves") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Pictures First")
            try await ui.importNightBus()
            try await ui.openVisual("Storyboards")
            try await ui.wait(id: "storyboard-add-panel", timeout: 20)
            try await ui.choose(from: "storyboard-add-panel", ["Draw / Sketch"])
            try await ui.wait(id: "sketch-sheet", timeout: 15)
            try ui.snapshot("sketch-open")
            try await ui.tap(id: "sketch-cancel")
            try await ui.waitGone(id: "sketch-sheet", timeout: 10)
            try await ui.drawTwoStrokesAndSave()
            try await ui.waitText("Panel 1", timeout: 15)
            let kinds = ui.panelKinds()
            try ui.expect(kinds == ["picture"], "the storyboard has one panel and it shows a picture: \(kinds)")
        },
        Scenario("86-visual-planning/last-panel-is-dragged-to-the-front") { ui, _, ctx in
            try await ui.launch(ctx)
            try await ui.createProject("Pictures First")
            try await ui.importNightBus()
            try await ui.openVisual("Storyboards")
            try await ui.wait(id: "storyboard-add-panel", timeout: 20)
            try await ui.drawTwoStrokesAndSave()
            try await ui.choose(from: "storyboard-add-panel", ["Empty Panel"])
            try await ui.choose(from: "storyboard-add-panel", ["Empty Panel"])
            try await ui.waitText("Panel 3", timeout: 15)
            try ui.expect(ui.panelKinds() == ["picture", "empty", "empty"], "the panels are picture, empty, empty: \(ui.panelKinds())")
            let panels = ui.all().filter { $0.identifier.hasPrefix("panel-") && !$0.identifier.hasPrefix("panel-shot") && $0.label.hasPrefix("Panel ") }
            guard let third = panels.first(where: { $0.label.hasPrefix("Panel 3") }), let first = panels.first(where: { $0.label.hasPrefix("Panel 1") }) else { throw DriverError("panels 1 and 3 not found: \(panels)") }
            try await ui.dragAlong(from: ui.center(third), to: ui.center(first))
            try await ui.idle(1.0)
            let after = ui.panelKinds()
            try ui.expect(after.first == "empty", "the first panel is now an empty one: \(after)")
        },
        Scenario("86-visual-planning/note-and-panel-order-survive-a-restart", phases: 2) { ui, phase, ctx in
            let mark = ctx.work.appendingPathComponent("moodboard-position-86.txt")
            if phase == 1 {
                try await ui.launch(ctx)
                try await ui.createProject("Pictures First")
                try await ui.importNightBus()
                try await ui.prepareMoodboard()
                let (_, after) = try await ui.dragNoteTile()
                try await ui.idle(0.8)
                // Position of the tile relative to the canvas, which does not move with the window.
                try "\(Int(after.minX)) \(Int(after.minY))".write(to: mark, atomically: true, encoding: .utf8)
                try await ui.openVisual("Storyboards")
                try await ui.wait(id: "storyboard-add-panel", timeout: 20)
                try await ui.drawTwoStrokesAndSave()
                try await ui.choose(from: "storyboard-add-panel", ["Empty Panel"])
                try await ui.choose(from: "storyboard-add-panel", ["Empty Panel"])
                try await ui.waitText("Panel 3", timeout: 15)
                let panels = ui.all().filter { $0.identifier.hasPrefix("panel-") && !$0.identifier.hasPrefix("panel-shot") && $0.label.hasPrefix("Panel ") }
                guard let third = panels.first(where: { $0.label.hasPrefix("Panel 3") }), let first = panels.first(where: { $0.label.hasPrefix("Panel 1") }) else { throw DriverError("panels not found") }
                try await ui.dragAlong(from: ui.center(third), to: ui.center(first))
                try await ui.idle(1.0)
                try ui.expect(ui.panelKinds() == ["empty", "picture", "empty"], "order before restart: \(ui.panelKinds())")
                try await ui.closeToHome()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Pictures First")
                try await ui.openVisual("Moodboards")
                _ = try await ui.wait(label: "Note: Cold blue light, warm windows", timeout: 20)
                let tile = try await ui.noteTileBounds()
                let saved = try String(contentsOf: mark, encoding: .utf8).split(separator: " ").compactMap { Int($0) }
                let now = [Int(tile.minX), Int(tile.minY)]
                try ui.expect(abs(now[0] - saved[0]) <= 2 && abs(now[1] - saved[1]) <= 2, "the note tile is where it was dropped: saved \(saved), now \(now)")
                try await ui.openVisual("Storyboards")
                try await ui.waitText("Panel 3", timeout: 20)
                try ui.expect(ui.panelKinds() == ["empty", "picture", "empty"], "three panels in order empty, picture, empty: \(ui.panelKinds())")
            }
        },
    ]
}

// MARK: 41-screenplay-engine (import and export parts)

fileprivate extension UI {
    /// New project 'Signal Box' with a typed two page script: heading, action, dialogue, an explicit page break.
    func writeSignalBox(_ ctx: Context) async throws {
        try await launch(ctx)
        try await createProject("Signal Box")
        try await go("Screenplay")
        let tv = try await editor()
        log("new screenplay editor text: \(tv.string.debugDescription)")
        tv.selectAll(nil)
        try await typeHere("INT. SIGNAL BOX - NIGHT")
        try await enter()
        try await typeHere("The telephone rings twice.")
        try await enter()
        try await typeHere("ASHA")
        try await keyToSheet("\u{1b}", code: 53)
        try await enter()
        try await typeHere("Who is there?")
        try await keyToSheet("\r", code: 36, [.command])
        try await typeHere("Nobody answers.")
        try await waitSaved()
    }

    /// The export dialog with the title page and scene numbers switches set as asked, then the Save panel.
    func exportDraft(format: String, to url: URL, titlePage: Bool) async throws {
        try await menu(["File", "Export Screenplay…"])
        try await wait(id: "export-confirm", timeout: 20)
        if format == "txt" { try await tap(id: "export-more-formats") }
        try await tap(id: "export-format-\(format)")
        let on = query(id: "export-title-page").first.map { $0.value == "1" } ?? true
        if on != titlePage { try await tap(id: "export-title-page") }
        try await choosing(url) { try await tap(id: "export-confirm") }
        try await waitText("Export complete", timeout: 60)
    }
}

extension Scenarios {
    static let engineExportGroup: [Scenario] = [
        Scenario("41-screenplay-engine/export-pdf-and-fdx-then-import-fdx-back") { ui, _, ctx in
            let f = Findings()
            try await ui.writeSignalBox(ctx)
            let dir = ctx.work.appendingPathComponent("exports-41")
            try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
            let pdf = dir.appendingPathComponent("signal-box.pdf")
            try await ui.exportDraft(format: "pdf", to: pdf, titlePage: true)
            f.check(ui.screenText().contains("Pages"), "the export report shows the page count")
            f.check(ui.queryText(startsWith: "Pages").contains { $0.contains("2") }, "the export reports 2 pages: \(ui.queryText(startsWith: "Pages"))")
            try await ui.tap(id: "export-close")
            try await ui.waitGone(id: "export-sheet")
            f.check(ui.fileSize(pdf) > 500, "the PDF is larger than 500 bytes: \(ui.fileSize(pdf))")
            let pdfBytes = (try? Data(contentsOf: pdf)) ?? Data()
            f.check(pdfBytes.range(of: Data("OpenFrame".utf8)) == nil, "the PDF does not contain the word OpenFrame")
            let fdx = dir.appendingPathComponent("signal-box.fdx")
            try await ui.exportDraft(format: "fdx", to: fdx, titlePage: true)
            try await ui.tap(id: "export-close")
            try await ui.waitGone(id: "export-sheet")
            let xml = (try? String(contentsOf: fdx, encoding: .utf8)) ?? ""
            f.check(xml.contains("StartsNewPage=\"Yes\""), "the FDX file has a paragraph that starts a new page")
            f.check(xml.contains("Number=\"1\""), "the FDX file has scene number 1: \(xml.prefix(1500))")
            let preview = try await ui.importFile(fdx, draftName: "From Final Draft")
            f.check(preview.contains("1"), "the FDX preview shows its scene: \(preview)")
            try await ui.tap(id: "import-confirm")
            try await ui.waitText("Import complete", timeout: 30)
            try await ui.tap(id: "import-open-screenplay")
            let tv = try await ui.editor()
            try await ui.waitText("From Final Draft", timeout: 10)
            f.check(tv.string.contains("Nobody answers."), "the imported draft holds the script text: \(tv.string.prefix(200))")
            f.check(tv.string.contains("The telephone rings twice."), "the imported draft holds the action line")
            try f.finish()
        },
        Scenario("41-screenplay-engine/telugu-and-hindi-print-in-the-pdf") { ui, _, ctx in
            let f = Findings()
            try await ui.writeSignalBox(ctx)
            let tv = try await ui.editor()
            try await ui.keyToSheet("\r", code: 36, [.command])
            tv.window?.makeFirstResponder(tv)
            try await ui.typeHere("మీరా రైలు స్టేషన్\u{200C}లో వేచి ఉంది. मीरा ने कहा, “क्षमा करें।”")
            try await ui.waitSaved()
            f.check(tv.string.contains("మీరా రైలు"), "the editor shows the line containing మీరా రైలు")
            let pdf = ctx.work.appendingPathComponent("signal-box-te.pdf")
            try await ui.exportDraft(format: "pdf", to: pdf, titlePage: false)
            try await ui.tap(id: "export-close")
            try await ui.waitGone(id: "export-sheet")
            let bytes = (try? Data(contentsOf: pdf)) ?? Data()
            let text = String(decoding: bytes, as: UTF8.self)
            f.check(text.contains("NotoSansTelugu") || text.contains("Noto Sans Telugu"), "the PDF embeds the font Noto Sans Telugu")
            f.check(text.contains("NotoSansDevanagari") || text.contains("Noto Sans Devanagari"), "the PDF embeds the font Noto Sans Devanagari")
            try f.finish()
        },
    ]
}

fileprivate extension UI {
    func queryText(startsWith prefix: String) -> [String] { all().map(\.text).filter { $0.hasPrefix(prefix) } }
}

// MARK: AI (72, 73, 74, 75, 76, 78)

fileprivate extension UI {
    /// Presses a button of an alert or confirmation dialog on screen, by its title, as a click does.
    func pressAlertButton(_ title: String) async throws {
        func buttons(_ v: NSView) -> [NSButton] { (v as? NSButton).map { [$0] } ?? [] + v.subviews.flatMap(buttons) }
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("Alert") {
            if let b = w.contentView.map(buttons)?.first(where: { $0.title == title }) {
                log("press \(title) in the dialog (window \(w.title), key \(w.isKeyWindow), button enabled \(b.isEnabled), target \(String(describing: b.target)) action \(String(describing: b.action)))")
                w.makeKeyAndOrderFront(nil)
                await idle(0.3)
                b.performClick(nil)
                await idle(0.5)
                return
            }
        }
        throw DriverError("no dialog button \(title)")
    }

    /// An installed model file of this run's data folder, found by a part of its name.
    func modelFile(_ part: String, _ ctx: Context) -> URL? {
        guard let walker = FileManager.default.enumerator(at: ctx.dataRoot, includingPropertiesForKeys: nil) else { return nil }
        for case let url as URL in walker where url.pathExtension == "gguf" && url.lastPathComponent.contains(part) { return url }
        return nil
    }

    /// Makes `url` a real copy (not a link), then flips one byte in its middle: the size stays the same.
    func changeOneByte(_ url: URL) throws {
        let data = try Data(contentsOf: url)
        let tmp = url.appendingPathExtension("tmp")
        try data.write(to: tmp)
        try FileManager.default.removeItem(at: url)
        try FileManager.default.moveItem(at: tmp, to: url)
        let h = try FileHandle(forUpdating: url)
        defer { try? h.close() }
        try h.seek(toOffset: UInt64(data.count / 2))
        let b = data[data.count / 2]
        try h.write(contentsOf: Data([b ^ 0xFF]))
    }

    func scopeLine() -> String { textOf(id: "ai-scope-line") }

    /// "Reopen the assistant here": close it if open, open it, wait for the scope line.
    func reopenAssistantHere() async throws {
        try await closeAssistant()
        try await openAssistant()
        _ = try await wait(id: "ai-scope-line", timeout: 20)
        await idle(0.5)
    }

    /// "Go to Tasks": close the assistant, open Notes and Tasks, wait for the Tasks section.
    func goTasks() async throws {
        try await closeAssistant()
        try await go("Notes and Tasks")
        _ = try await wait(id: "tasks-panel", timeout: 20)
        await idle(0.6)
    }

    /// Titles of the open tasks: the check boxes that offer "Mark ... done".
    func openTasks() -> [String] {
        all().filter { $0.role == "AXCheckBox" && $0.label.hasPrefix("Mark \u{201C}") }
            .map { $0.label.replacingOccurrences(of: "Mark \u{201C}", with: "").replacingOccurrences(of: "\u{201D} done", with: "") }
    }

    func newestCardState() -> String { query(id: "ai-change-state").last?.text ?? "" }
    func cardCount() -> Int {
        let t = textOf(id: "ai-change-count")
        return Int(t.split(separator: " ").first ?? "") ?? 0
    }

    static let modelsDir = URL(fileURLWithPath: "/Users/m.udarshreddy/Desktop/OpenFrame/.ai-assets/models")
    /// The three verified model files, in the order they are imported: language model, then the two search models.
    static let modelFiles = ["gemma-3-1b-it-q8-0-v1", "bge-small-en-v1.5-q8-0-v1", "multilingual-e5-small-q8-0-v1"]
        .map { modelsDir.appendingPathComponent($0).appendingPathComponent("model.gguf") }

    func assistantOpen() -> Bool { exists(id: "ai-panel") }

    func openAssistant() async throws {
        if assistantOpen() { return }
        await idle(0.8)
        try await keyToSheet("j", code: 38, [.command])
        _ = try await wait(id: "ai-panel", timeout: 12, "the AI Assistant panel after Command+J")
    }

    func closeAssistant() async throws {
        if assistantOpen() { try await tap(label: "Close AI Assistant", role: "AXButton") }
        try await waitGone(id: "ai-panel", timeout: 10)
    }

    /// The download card's "Import a model file" for each pinned model, answered with the real files.
    func installModels() async throws {
        try await openAssistant()
        _ = try await wait(id: "ai-download-card", timeout: 30, "the download card")
        for file in Self.modelFiles {
            try await choosing(file) { try await tap(id: "ai-download-import") }
            // The import hashes the file; the card comes back when it is done.
            let deadline = Date().addingTimeInterval(180)
            while Date() < deadline {
                if exists(id: "ai-input") { return }
                if let btn = query(id: "ai-download-import").first, btn.isEnabled { break }
                await idle(0.5)
            }
        }
        _ = try await wait(id: "ai-input", timeout: 120, "the Ask box after the models are in")
    }

    /// "Ask Q": type it, press Enter, wait for a new result under it and no working status.
    @discardableResult
    func ask(_ q: String, timeout: Double = 240) async throws -> String {
        try await openAssistant()
        let before = query(id: "ai-question").count
        let input = try await wait(id: "ai-input", timeout: 60, "an enabled Ask box")
        let deadline0 = Date().addingTimeInterval(60)
        while !input.isEnabled, Date() < deadline0 { await idle(0.3) }
        // The assistant says "Preparing project context" while it indexes the project; a person waits for it.
        let idx = Date().addingTimeInterval(90)
        while exists(id: "ai-index-notice"), Date() < idx { await idle(0.5) }
        try await focusByClick(input)
        try await type(q, into: "ai-input")
        try await enter()
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if query(id: "ai-question").count > before && !exists(id: "ai-working") && !exists(id: "ai-pending-question") { break }
            await idle(0.5)
        }
        try expect(query(id: "ai-question").count > before, "a result shows after the question \"\(q)\" within \(Int(timeout)) s")
        try expect(!exists(id: "ai-working"), "no working status remains")
        return lastResultKind()
    }

    /// "answer", "navigate", "clarify", "proposal", "suggestion", "denied", "failed" ... of the newest result.
    func lastResultKind() -> String {
        let kinds = all().map(\.identifier).filter { $0.hasPrefix("ai-result-") }
        return kinds.last.map { String($0.dropFirst("ai-result-".count)) } ?? ""
    }

    /// "Ask if open Q": if the Ask box shows within 5 seconds, type Q and press Enter; nothing more is waited for.
    func askIfOpen(_ q: String) async throws {
        let deadline = Date().addingTimeInterval(5)
        while !exists(id: "ai-input"), Date() < deadline { await idle(0.3) }
        guard let input = query(id: "ai-input").first, input.isEnabled else { return }
        try await focusByClick(input)
        try await type(q, into: "ai-input")
        try await enter()
    }

    func lastAnswerText() -> String { query(id: "ai-answer-text").last?.text ?? "" }

    /// Processes of the offline AI engine that run from an app bundle's ai folder.
    func engines(anyData: Bool = false) -> [String] {
        let root = ProcessInfo.processInfo.environment["OPENFRAME_DATA_ROOT"] ?? "/nonexistent"
        return run("/bin/ps", ["-axo", "pid=,command="]).split(separator: "\n").map(String.init)
            .filter { anyData || $0.contains(root) }
            .filter { $0.contains("llama-server") && $0.contains("/ai/") && !$0.contains("/bin/ps") && !$0.contains("--list-devices") }
    }

    func waitEngines(running: Bool, timeout: Double) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if engines().isEmpty != running { return }
            await idle(0.5)
        }
        throw DriverError("engines \(running ? "never started" : "still running"): \(engines())")
    }
}

extension Scenarios {
    static func isAnswered(_ kind: String) -> Bool { ["answer", "navigate", "clarify", "proposal", "suggestion"].contains(kind) }

    static func aiStart(_ ui: UI, _ ctx: Context, install: Bool = true) async throws {
        try await ui.launch(ctx)
        try await ui.createProject("Night Bus")
        try await ui.importNightBus()
        if install { try await ui.installModels() }
    }

    static let aiGroup: [Scenario] = [
        Scenario("72-ai-assistant/ready-after-models-no-names-shown", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.openAssistant()
            try await ui.wait(id: "ai-input", timeout: 30)
            f.check(ui.screenText().contains("AI Ready"), "the screen shows \"AI Ready\": \(ui.textOf(id: "ai-mode-chip"))")
            let text = ui.screenText().lowercased()
            for word in ["download", "install offline ai", "gemma", "gguf", "llama", "q8_0", "embedding"] {
                f.check(!text.contains(word), "no visible text says \"\(word)\"")
            }
            f.check(ui.engines().isEmpty, "the chat engine is not running yet: \(ui.engines())")
            try f.finish()
        },
        Scenario("72-ai-assistant/settings-shows-installed-parts-and-remove", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.openSettings(tab: "Offline AI")
            try await ui.waitText("OpenFrame AI runs on this computer after a one-time download", timeout: 20)
            let details = try await ui.wait(label: "Technical details", timeout: 15)
            ui.log("technical details element: \(details) subrole \(ui.subrole(details))")
            try await ui.tap(details)
            try await ui.idle(1.0)
            ui.log(ui.all().filter { $0.identifier.hasPrefix("settings-ai") || $0.role.contains("Disclosure") }.map(\.description).joined(separator: "\n"))
            let rows = ui.query(id: "settings-ai-component")
            f.check(rows.count == 4, "the table of Offline AI components has exactly four rows, has \(rows.count)")
            f.check(rows.contains { $0.text.contains("Gemma-Terms-of-Use") }, "one row names Gemma-Terms-of-Use")
            let hashed = rows.filter { r in r.text.split(whereSeparator: { !$0.isHexDigit }).contains { $0.count == 64 } }
            f.check(hashed.count == rows.count && !rows.isEmpty, "every row shows a 64-character SHA-256 hash: \(rows.map(\.text))")
            f.check(ui.screenText().contains("read-only"), "the technical details say read-only")
            f.check(ui.exists(label: "Remove OpenFrame AI…"), "the button Remove OpenFrame AI… is shown")
            for name in ["Download OpenFrame AI", "Download Offline AI", "Update Offline AI"] {
                f.check(!ui.exists(label: name), "there is no button \"\(name)\"")
            }
            try await ui.closeAISettings()
            try f.finish()
        },
        Scenario("72-ai-assistant/exact-answer-from-project-data-with-sources", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Screenplay")
            try await ui.openAssistant()
            try await ui.choose(from: "ai-scope", ["Current Screenplay"])
            let kind = try await ui.ask("How many scenes are in the script?")
            f.check(kind == "answer", "the result is an Answer, is \(kind)")
            // A plain answer: no "Exact" badge or step list in the conversation (OpenFrameAI.docx 4.1).
            f.check(ui.query(id: "ai-confidence").isEmpty && ui.query(id: "ai-steps").isEmpty, "the answer carries no badge or step list")
            f.check(ui.lastAnswerText().contains("3"), "the answer contains 3: \(ui.lastAnswerText())")
            f.check(ui.query(id: "ai-source").isEmpty, "its sources stay closed until asked for")
            try await ui.openLastSources()
            f.check(ui.query(id: "ai-source").count >= 1, "See sources lists at least one source")
            f.check(!ui.engines().isEmpty, "the chat engine is now running")
            try f.finish()
        },
        Scenario("72-ai-assistant/meaning-question-uses-project-context", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.openAssistant()
            try await ui.choose(from: "ai-scope", ["Whole Project"])
            let kind = try await ui.ask("Which scene happens at the bus stop?")
            f.check(kind == "answer" || kind == "navigate", "the result is an Answer or a Navigate, is \(kind)")
            try await ui.openLastSources()
            let chips = ui.query(id: "ai-source").filter { $0.text.uppercased().contains("BUS STOP") }
            f.check(!chips.isEmpty, "one of its sources names BUS STOP: \(ui.query(id: "ai-source").map(\.text))")
            try await ui.go("Project Home")
            if let chip = ui.query(id: "ai-source").last(where: { $0.text.uppercased().contains("BUS STOP") }) {
                try await ui.tap(chip)
                try await ui.wait(id: "screenplay-editor", timeout: 20, "the Screenplay workspace after clicking the source")
            }
            try f.finish()
        },
        Scenario("72-ai-assistant/navigation-command-opens-the-place", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Project Home")
            let kind = try await ui.ask("Open Scene 2.")
            f.check(kind == "navigate", "the result is a Navigate, is \(kind)")
            try await ui.wait(id: "screenplay-editor", timeout: 20, "the Screenplay workspace")
            try f.finish()
        },
        Scenario("72-ai-assistant/no-network-engines-listen-on-this-computer-only", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Screenplay")
            try await ui.openAssistant()
            try await ui.choose(from: "ai-scope", ["Current Screenplay"])
            let kind = try await ui.ask("How many scenes are in the script?")
            f.check(kind == "answer", "the result is an Answer, is \(kind)")
            let engines = ui.engines()
            f.check(!engines.isEmpty, "at least one engine is running")
            for e in engines {
                f.check(e.contains(Bundle.main.bundlePath + "/Contents/Resources/ai/"), "the engine runs from the bundled AI folder of the app under test: \(e.prefix(200))")
                f.check(e.contains("--host 127.0.0.1"), "the engine listens on 127.0.0.1 only: \(e.prefix(300))")
                f.check(e.contains("--offline") || e.contains("LLAMA_OFFLINE"), "the engine was started in offline mode: \(e)")
                f.check(!e.contains("--api-key"), "no engine has an API key on its command line")
            }
            let pids = ([String(getpid())] + engines.compactMap { $0.split(separator: " ").first.map(String.init) }).joined(separator: ",")
            let net = ui.run("/usr/sbin/lsof", ["-nP", "-a", "-p", pids, "-i"])
            for line in net.split(separator: "\n").dropFirst() {
                let l = String(line)
                let outside = l.contains("->") && !(l.contains("->127.0.0.1") || l.contains("->[::1]") || l.contains("->localhost"))
                f.check(!outside, "a process of the app holds a connection to an outside address: \(l)")
                f.check(!(l.contains("LISTEN") && !(l.contains("127.0.0.1:") || l.contains("[::1]:") || l.contains("localhost:"))), "a process listens on an outside address: \(l)")
            }
            try f.finish()
        },
        Scenario("72-ai-assistant/stop-offline-ai-frees-the-engines", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.ask("How many scenes are in the script?")
            try await ui.waitEngines(running: true, timeout: 30)
            try await ui.choose(from: "ai-settings-menu", ["Stop Offline AI"])
            try await ui.waitText("Offline AI stopped", timeout: 15)
            try await ui.waitEngines(running: false, timeout: 30)
            let kind = try await ui.ask("How many scenes are in the script?")
            f.check(kind == "answer", "the result is an Answer, is \(kind)")
            f.check(!ui.engines().isEmpty, "at least one engine is running again")
            try f.finish()
        },
        Scenario("72-ai-assistant/conversation-is-still-there-after-a-relaunch", phases: 2, timeout: 900) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                for q in ["How many scenes are in the script?", "Which scene happens at the bus stop?", "Open Scene 2.", "How many scenes are in the script?"] {
                    try await ui.ask(q)
                }
                try ui.expect(ui.query(id: "ai-question").count >= 4, "there are at least 4 results in the panel")
                try await ui.closeToHome()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                try await ui.wait(id: "ai-input", timeout: 60)
                try await ui.waitText("AI Ready", timeout: 30)
                let n = ui.query(id: "ai-question").count
                try ui.expect(n >= 4, "the panel shows at least as many results as before, shows \(n)")
            }
        },
        Scenario("72-ai-assistant/docked-panel-remembers-open-or-closed", phases: 3, timeout: 600) { ui, phase, ctx in
            switch phase {
            case 1:
                try await aiStart(ui, ctx, install: false)
                try await ui.openAssistant()
                try ui.expect(ui.assistantOpen(), "the AI Assistant is open")
            case 2:
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.wait(id: "ai-panel", timeout: 20, "the AI Assistant opens again after the window loads again")
                try await ui.tap(label: "Close AI Assistant", role: "AXButton")
                try await ui.waitGone(id: "ai-panel", timeout: 10)
            default:
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.idle(2)
                try ui.expect(!ui.assistantOpen(), "the AI Assistant stays closed")
            }
        },
        Scenario("73-ai-workspaces/assistant-takes-the-scope-of-each-workspace", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            let checks: [(String, [String])] = [
                ("Project Home", ["Using: Whole Project"]), ("Idea Vault", ["Using: Whole Project"]), ("Story", ["Using: Beat Board"]),
                ("Breakdown", ["Using: Production Workspace"]), ("Schedule", ["Using: Production Workspace"]),
                ("Call Sheets", ["Using: Production Workspace"]), ("Screenplay", ["Using: Current Scene", "Using: Current Screenplay"]),
                ("Notes and Tasks", ["Using: Whole Project"]),
            ]
            for (ws, expected) in checks {
                try await ui.go(ws)
                try await ui.reopenAssistantHere()
                f.check(expected.contains(ui.scopeLine()), "in \(ws) the scope line reads \(expected), reads \"\(ui.scopeLine())\"")
            }
            try f.finish()
        },
        Scenario("73-ai-workspaces/keyboard-shortcut-opens-and-closes-the-assistant", timeout: 600) { ui, _, ctx in
            try await aiStart(ui, ctx, install: false)
            try await ui.closeAssistant()
            try await ui.keyToSheet("j", code: 38, [.command])
            try await ui.wait(id: "ai-panel", timeout: 10, "the AI Assistant opens after Command+J")
            try await ui.keyToSheet("j", code: 38, [.command])
            try await ui.waitGone(id: "ai-panel", timeout: 10)
        },
        Scenario("73-ai-workspaces/story-board-question-uses-the-story-board", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Story")
            try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
            try await ui.openAssistant()
            let kind = try await ui.ask("What is on the Beat Board?")
            f.check(isAnswered(kind), "the question was answered, result is \(kind)")
            f.check(ui.scopeLine() == "Using: Beat Board", "the scope line reads Using: Beat Board, reads \"\(ui.scopeLine())\"")
            try f.finish()
        },
        Scenario("73-ai-workspaces/screenplay-question-is-about-the-screenplay", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Screenplay")
            try await ui.openAssistant()
            try await ui.choose(from: "ai-scope", ["Current Screenplay"])
            let kind = try await ui.ask("Who speaks in this screenplay?")
            f.check(isAnswered(kind), "the question was answered, result is \(kind)")
            let a = ui.lastAnswerText().uppercased() + " " + ui.query(id: "ai-items").map(\.text).joined(separator: " ").uppercased()
            f.check(a.contains("ARJUN") || a.contains("MEERA"), "the answer names ARJUN or MEERA: \(a)")
            try f.finish()
        },
        Scenario("73-ai-workspaces/production-question-uses-the-production-workspace", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.go("Breakdown")
            try await ui.openAssistant()
            let kind = try await ui.ask("Which locations are used in the script?")
            f.check(isAnswered(kind), "the question was answered, result is \(kind)")
            try await ui.openLastSources()
            let a = (ui.lastAnswerText() + " " + ui.query(id: "ai-source").map(\.text).joined(separator: " ") + " " + ui.query(id: "ai-items").map(\.text).joined(separator: " ")).uppercased()
            f.check(a.contains("BUS STOP") || a.contains("RAILWAY STATION") || a.contains("POLICE STATION"), "the answer or its sources name a location: \(a)")
            try f.finish()
        },
        Scenario("75-ai-crash-safe-apply/killed-while-applying-all-or-nothing", phases: 2, timeout: 600, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            let mark = ctx.work.appendingPathComponent("change-count-75.txt")
            if phase == 1 {
                try await aiStart(ui, ctx)
                let kind = try await ui.ask("Create a task to scout the railway station.")
                try ui.expect(kind == "proposal", "the result is a Proposal, is \(kind)")
                let n = ui.cardCount()
                try ui.expect(n >= 1, "the card says how many changes it makes, at least one")
                try String(n).write(to: mark, atomically: true, encoding: .utf8)
                try await ui.tap(id: "ai-apply")
                // The runner kills the app with SIGKILL at once.
            } else {
                // "End every process of the app at once, engines included": the engines the runner did not end.
                for e in ui.engines() { if let pid = e.split(separator: " ").first { _ = ui.run("/bin/kill", ["-9", String(pid)]) } }
                let n = Int(try String(contentsOf: mark, encoding: .utf8)) ?? 1
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.goTasks()
                await ui.idle(1.5)
                let tasks = ui.openTasks()
                try ui.expect(tasks.isEmpty || tasks.count == n, "there are either no open tasks or exactly \(n): \(tasks)")
                try await ui.openAssistant()
                try await ui.wait(id: "ai-input", timeout: 60)
                try ui.expect(ui.query(id: "ai-question").count >= 1, "the conversation is restored with at least one result")
                try ui.expect(ui.exists(id: "ai-apply") || ui.exists(id: "ai-change-state") || ui.exists(label: "Proposed Changes"), "the card is still shown")
                let applied = ui.newestCardState().hasPrefix("Applied")
                if tasks.count == n { try ui.expect(applied, "there are \(n) tasks, so the card's state says Applied: \"\(ui.newestCardState())\"") }
                else { try ui.expect(!applied, "there are no tasks, so the card's state does not say Applied: \"\(ui.newestCardState())\"") }
            }
        },
        Scenario("75-ai-crash-safe-apply/killed-while-the-engine-answers", phases: 2, timeout: 600, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.openAssistant()
                let input = try await ui.wait(id: "ai-input", timeout: 60)
                try await ui.focusByClick(input)
                try await ui.type("Which scene happens at the railway station?", into: "ai-input")
                try await ui.enter()
                try await ui.waitEngines(running: true, timeout: 120)
                // The runner kills the app with SIGKILL now.
            } else {
                // Within 15 seconds no engine from the bundled AI folder is running, whoever started it.
                let t0 = Date()
                try await ui.waitEngines(running: false, timeout: 15)
                ui.log("engines gone \(Int(Date().timeIntervalSince(t0))) s after the relaunch began")
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                let kind = try await ui.ask("Create a task to check the bus timetable.")
                try ui.expect(["proposal", "answer", "clarify"].contains(kind), "the result is a Proposal, an Answer or a Clarify, is \(kind)")
            }
        },
        Scenario("76-ai-files-integrity/missing-model-turns-ai-off-and-offers-download", phases: 2, timeout: 600) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.closeToHome()
                guard let gemma = ui.modelFile("gemma", ctx) else { throw DriverError("the language model file is not in the data folder") }
                try FileManager.default.removeItem(at: gemma)
            } else {
                let f = Findings()
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                try await ui.waitText("An OpenFrame AI file is missing or damaged", timeout: 15)
                f.check(ui.screenText().contains("Download OpenFrame AI again"), "the panel shows Download OpenFrame AI again")
                f.check(!ui.screenText().contains("Reinstall OpenFrame Studio to repair it"), "the panel does not show the reinstall message")
                f.check(!ui.exists(id: "ai-input"), "the panel has no question box")
                try await ui.openSettings(tab: "Offline AI")
                try await ui.waitText("Needs a new download", timeout: 15)
                try await ui.closeAISettings()
                try await ui.goTasks()
                try await ui.tap(id: "new-task")
                try await ui.type("Confirm pier permit", into: "task-title")
                try await ui.tap(label: "Add Task", role: "AXButton")
                try await ui.waitText("Added task", timeout: 15)
                await ui.idle(1)
                f.check(ui.openTasks() == ["Confirm pier permit"], "the open tasks are exactly Confirm pier permit: \(ui.openTasks())")
                f.check(ui.engines().isEmpty, "no engine runs: \(ui.engines())")
                try f.finish()
            }
        },
        Scenario("76-ai-files-integrity/download-again-restores-files-and-ai-answers", phases: 2, timeout: 1500) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.closeToHome()
                guard let gemma = ui.modelFile("gemma", ctx) else { throw DriverError("the language model file is not in the data folder") }
                try FileManager.default.removeItem(at: gemma)
            } else {
                let f = Findings()
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                try await ui.waitText("An OpenFrame AI file is missing or damaged", timeout: 15)
                let started = Date()
                try await ui.tap(id: "ai-download-start")
                _ = try await ui.wait(id: "ai-input", timeout: 1200, "the Ask box after Download again (real download from the model host)")
                ui.log("Download again took \(Int(Date().timeIntervalSince(started))) s")
                let kind = try await ui.ask("Scouting the railway station needs a task.")
                f.check(["proposal", "answer", "clarify"].contains(kind), "the result is a proposal, an answer or a question back, is \(kind)")
                f.check(!ui.engines().isEmpty, "at least one engine runs")
                try f.finish()
            }
        },
        Scenario("76-ai-files-integrity/altered-model-of-the-right-size-is-caught", phases: 2, timeout: 600) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.closeToHome()
                guard let bge = ui.modelFile("bge", ctx) else { throw DriverError("the search model file is not in the data folder") }
                try ui.changeOneByte(bge)
            } else {
                let f = Findings()
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                await ui.idle(1)
                try await ui.askIfOpen("How many scenes are in the script?")
                try await ui.waitText("An OpenFrame AI file is missing or damaged", timeout: 240)
                f.check(!ui.screenText().contains("Reinstall OpenFrame Studio to repair it"), "nothing on the screen shows the reinstall message")
                f.check(ui.engines().isEmpty, "no engine runs: \(ui.engines())")
                try f.finish()
            }
        },
        Scenario("76-ai-files-integrity/altered-engine-library-asks-for-a-reinstall", timeout: 900) { ui, phase, ctx in
            let env = ProcessInfo.processInfo.environment
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.closeToHome()
                let fm = FileManager.default
                let bundle = Bundle.main.bundleURL
                let work = ctx.work.appendingPathComponent("copy-76")
                try? fm.removeItem(at: work)
                try fm.createDirectory(at: work, withIntermediateDirectories: true)
                let copy = work.appendingPathComponent(bundle.lastPathComponent)
                _ = ui.run("/bin/cp", ["-cR", bundle.path, copy.path])
                // The app marks files it writes as downloaded (LSFileQuarantineEnabled); this copy is
                // the test's own, so the mark comes off before it starts, or macOS would stop it with
                // a "damaged" dialog on the screen.
                _ = ui.run("/usr/bin/xattr", ["-dr", "com.apple.quarantine", copy.path])
                guard let lib = fm.enumerator(at: copy, includingPropertiesForKeys: nil)?.compactMap({ $0 as? URL }).first(where: { $0.lastPathComponent == "libggml-base.0.dylib" }) else { throw DriverError("the copy of the build has no engine library") }
                try ui.changeOneByte(lib)
                let childRoot = work.appendingPathComponent("data")
                _ = ui.run("/bin/cp", ["-cR", ctx.dataRoot.path, childRoot.path])
                let out = URL(fileURLWithPath: env["OPENFRAME_E2E_OUT"] ?? NSTemporaryDirectory()).appendingPathComponent("copy-run")
                try fm.createDirectory(at: out, withIntermediateDirectories: true)
                let exec = copy.appendingPathComponent("Contents/MacOS/" + (Bundle.main.executableURL?.lastPathComponent ?? "OFIntApp"))
                var childEnv = env
                childEnv["OPENFRAME_DATA_ROOT"] = childRoot.path
                childEnv["OPENFRAME_E2E_PHASE"] = "2"
                childEnv["OPENFRAME_E2E_OUT"] = out.path
                let p = Process()
                p.executableURL = exec
                p.environment = childEnv
                try p.run()
                let deadline = Date().addingTimeInterval(600)
                while p.isRunning, Date() < deadline { await ui.idle(0.5) }
                if p.isRunning { p.terminate(); throw DriverError("the copy of the build did not finish in 10 minutes") }
                let result = (try? String(contentsOf: out.appendingPathComponent("result-2.json"), encoding: .utf8)) ?? ""
                try ui.expect(result.contains("\"status\" : \"pass\""), "the run of the altered copy failed: \(result)")
            } else {
                let f = Findings()
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                await ui.idle(1)
                try await ui.askIfOpen("How many scenes are in the script?")
                try await ui.waitText("Reinstall OpenFrame Studio to repair it", timeout: 240)
                f.check(!ui.exists(label: "Download OpenFrame AI"), "the panel does not offer Download OpenFrame AI")
                f.check(ui.engines().isEmpty, "no engine runs: \(ui.engines())")
                try f.finish()
            }
        },
        Scenario("78-ai-download/before-the-click-size-shown-and-no-network-use", timeout: 300) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx, install: false)
            try await ui.openAssistant()
            try await ui.waitText("A one-time download of", timeout: 15)
            let text = ui.screenText()
            f.check(text.contains("GB. It needs") && text.contains("of free space"), "the card shows the size and the free space needed")
            f.check(text.contains("uses the internet once"), "the card says the download uses the internet once")
            f.check(!ui.exists(id: "ai-input"), "the panel has no question box")
            let net = ui.run("/usr/sbin/lsof", ["-nP", "-a", "-p", String(getpid()), "-i"])
            let outside = net.split(separator: "\n").dropFirst().filter { $0.contains("->") && !$0.contains("127.0.0.1") && !$0.contains("[::1]") }
            f.check(outside.isEmpty, "the app asked no host anything: \(outside)")
            f.check(ui.modelFile("gemma", ctx) == nil, "nothing was downloaded")
            try f.finish()
        },
        Scenario("78-ai-download/paused-download-resumes-and-ai-becomes-ready", timeout: 1500) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx, install: false)
            try await ui.openAssistant()
            try await ui.waitText("A one-time download of", timeout: 15)
            let started = Date()
            try await ui.tap(id: "ai-download-start")
            try await ui.waitText("Downloading:", timeout: 90)
            try await ui.tap(id: "ai-download-pause")
            try await ui.waitText("Paused at", timeout: 30)
            let pausedLine = ui.textOf(id: "ai-download-line")
            ui.log("paused: \(pausedLine)")
            try await ui.idle(2)
            try await ui.tap(id: "ai-download-start")
            _ = try await ui.wait(id: "ai-input", timeout: 1200, "AI ready after the resumed download")
            ui.log("download with a pause took \(Int(Date().timeIntervalSince(started))) s")
            f.check(ui.screenText().contains("AI Ready"), "the screen shows AI Ready")
            try f.finish()
        },
        Scenario("78-ai-download/ai-answers-with-no-network", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            let kind = try await ui.ask("Summarise this project in one sentence.")
            f.check(!ui.lastAnswerText().trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, "the answer is not empty, result is \(kind): \(ui.lastAnswerText())")
            let net = ui.run("/usr/sbin/lsof", ["-nP", "-a", "-p", String(getpid()), "-i"])
            let outside = net.split(separator: "\n").dropFirst().filter { $0.contains("->") && !$0.contains("127.0.0.1") && !$0.contains("[::1]") }
            f.check(outside.isEmpty, "the host was asked nothing more: \(outside)")
            try f.finish()
        },
        Scenario("78-ai-download/restart-finds-the-ai-and-asks-no-host", phases: 2, timeout: 600) { ui, phase, ctx in
            if phase == 1 {
                try await aiStart(ui, ctx)
                try await ui.closeToHome()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Night Bus")
                try await ui.openAssistant()
                try await ui.wait(id: "ai-input", timeout: 30, "ready within 30 seconds")
                let net = ui.run("/usr/sbin/lsof", ["-nP", "-a", "-p", String(getpid()), "-i"])
                let outside = net.split(separator: "\n").dropFirst().filter { $0.contains("->") && !$0.contains("127.0.0.1") && !$0.contains("[::1]") }
                try ui.expect(outside.isEmpty, "the host was asked nothing: \(outside)")
            }
        },
        Scenario("78-ai-download/remove-damaged-file-set-aside-pause-and-resume", timeout: 1800) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.openSettings(tab: "Offline AI")
            try await ui.tap(id: "settings-ai-remove")
            try await ui.wait(id: "settings-ai-remove-confirm", timeout: 15)
            try await ui.pressAlertButton("Remove")
            let removedBy = Date().addingTimeInterval(20)
            while !ui.screenText().contains("OpenFrame AI was removed."), Date() < removedBy { await ui.idle(0.3) }
            if !ui.screenText().contains("OpenFrame AI was removed.") {
                f.add("Remove OpenFrame AI…, then Remove did not remove anything and showed no message within 20 s (the confirmation button was pressed)")
                // Carry on from the state the step should have made: nothing downloaded.
                try? FileManager.default.removeItem(at: ctx.dataRoot.appendingPathComponent("appdata/models"))
                try await ui.idle(5)
            }
            f.check(ui.screenText().contains("Not downloaded"), "the screen shows Not downloaded")
            // A language model file with one byte changed, offered to the card: it must be set aside.
            let bad = ctx.work.appendingPathComponent("damaged-gemma-78.gguf")
            try? FileManager.default.removeItem(at: bad)
            _ = ui.run("/bin/cp", ["-c", UI.modelFiles[0].path, bad.path])
            try ui.changeOneByte(bad)
            try await ui.choosing(bad) { try await ui.tap(id: "ai-download-import") }
            try await ui.idle(5)
            let screen = ui.screenText()
            f.check(screen.contains("damaged and has been set aside"), "the card shows \"damaged and has been set aside\": \(ui.query(id: "ai-download-error").map(\.text) + ui.query(id: "ai-download-problem").map(\.text))")
            try await ui.tap(id: "ai-download-start")
            try await ui.waitText("Downloading:", timeout: 120)
            try await ui.tap(id: "ai-download-pause")
            try await ui.waitText("Paused at", timeout: 30)
            try await ui.tap(label: "Resume download", role: "AXButton")
            try await ui.waitText("AI Ready", timeout: 1200)
            try await ui.closeAISettings()
            try await ui.openAssistant()
            _ = try await ui.wait(id: "ai-input", timeout: 60)
            try f.finish()
        },
        Scenario("74-ai-change-sets/proposal-arrives-and-nothing-changes-before-apply", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.goTasks()
            f.check(ui.openTasks().isEmpty, "there are no open tasks: \(ui.openTasks())")
            try await ui.openAssistant()
            let kind = try await ui.ask("Create a task to scout the railway station.")
            f.check(kind == "proposal", "the result is a Proposal, is \(kind)")
            f.check(ui.exists(label: "Proposed Changes"), "a Proposed Changes card shows")
            f.check(ui.screenText().contains("Notes & Tasks"), "the card names Notes & Tasks")
            f.check(ui.exists(id: "ai-apply") && ui.exists(id: "ai-reject"), "the card offers Apply Changes and Reject")
            f.check(ui.cardCount() >= 1, "the card says how many changes it makes, at least one: \"\(ui.textOf(id: "ai-change-count"))\"")
            try await ui.goTasks()
            f.check(ui.openTasks().isEmpty, "there are still no open tasks: \(ui.openTasks())")
            try f.finish()
        },
        Scenario("74-ai-change-sets/apply-undo-redo", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.ask("Create a task to scout the railway station.")
            let n = ui.cardCount()
            try await ui.tap(id: "ai-apply")
            let deadline = Date().addingTimeInterval(20)
            while !ui.newestCardState().hasPrefix("Applied"), Date() < deadline { await ui.idle(0.3) }
            f.check(ui.newestCardState().hasPrefix("Applied \(n) change"), "the card state reads Applied \(n) change(s): \"\(ui.newestCardState())\"")
            try await ui.goTasks()
            let t0 = Date().addingTimeInterval(10)
            while ui.openTasks().count != n, Date() < t0 { await ui.idle(0.3) }
            let tasks = ui.openTasks()
            if tasks.isEmpty { ui.log(ui.all().filter { $0.identifier.hasPrefix("task") }.map(\.description).joined(separator: "\n")) }
            f.check(tasks.count == n && tasks.allSatisfy { $0.lowercased().contains("railway station") }, "there are exactly \(n) open tasks that mention railway station: \(tasks)")
            try await ui.menu(["Edit", "Undo Project Change"])
            try await ui.idle(1.0)
            f.check(ui.openTasks().isEmpty, "one Undo reversed the whole Change Set: \(ui.openTasks())")
            try await ui.menu(["Edit", "Redo Project Change"])
            try await ui.idle(1.0)
            f.check(ui.openTasks().count == n, "Redo brings back \(n) open tasks: \(ui.openTasks())")
            try f.finish()
        },
        Scenario("74-ai-change-sets/reject-leaves-the-project-unchanged", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.goTasks()
            let before = ui.openTasks()
            try await ui.openAssistant()
            let kind = try await ui.ask("Create a task to book the bus for the night shoot.")
            f.check(kind == "proposal", "the result is a Proposal, is \(kind)")
            try await ui.tap(id: "ai-reject")
            let deadline = Date().addingTimeInterval(20)
            while !ui.newestCardState().hasPrefix("Rejected"), Date() < deadline { await ui.idle(0.3) }
            f.check(ui.newestCardState() == "Rejected. Nothing was changed.", "the card state reads Rejected. Nothing was changed.: \"\(ui.newestCardState())\"")
            try await ui.goTasks()
            f.check(ui.openTasks() == before, "the open tasks are the same as before: \(ui.openTasks()) vs \(before)")
            try f.finish()
        },
        Scenario("74-ai-change-sets/proposal-whose-target-changed-must-be-checked-again", timeout: 600) { ui, _, ctx in
            let f = Findings()
            try await aiStart(ui, ctx)
            try await ui.goTasks()
            try await ui.tap(id: "new-task")
            try await ui.type("Confirm pier permit", into: "task-title")
            try await ui.tap(label: "Add Task", role: "AXButton")
            try await ui.waitText("Added task", timeout: 15)
            try await ui.openAssistant()
            let kind = try await ui.ask("Mark the task Confirm pier permit as done.")
            f.check(kind == "proposal", "the result is a Proposal, is \(kind)")
            try await ui.goTasks()
            try await ui.tap(label: "Mark \u{201C}Confirm pier permit\u{201D} done", role: nil)
            try await ui.waitText("Completed task", timeout: 15)
            try await ui.openAssistant()
            try await ui.tap(id: "ai-apply")
            let deadline = Date().addingTimeInterval(20)
            while !ui.exists(id: "ai-stale-reason"), Date() < deadline { await ui.idle(0.3) }
            f.check(ui.screenText().contains("out of date"), "the card is marked out of date")
            f.check(!ui.exists(id: "ai-apply"), "the card no longer offers Apply Changes")
            f.check(ui.exists(id: "ai-recheck"), "the card offers Re-check & Review")
            try f.finish()
        },
    ]
}
