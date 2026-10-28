// Capture tour section: AI Assistant, packages and licence screens. Primitives and helpers are in CaptureTour.swift.
//
// The development app carries no Offline AI engine (Contents/Resources/ai is absent), so the
// assistant panel shows its "not available" state; the conversation, the download card and the
// answers cannot be reached in this build. Packages run end to end: the driver answers the Save
// and Open panels with files in the scenario's own work folder (see Panels.swift).

import AppKit
import Foundation

extension Scenarios {
    static let captureAI: [Scenario] = [
        tour("11-ai", order: 11000) { ui, _ in
            let area = "AI Assistant"
            try await ui.capNav("Project Home")
            await ui.step("ai closed on home") {
                try await ui.aiClose()
                await ui.capture("ai-closed-home", title: "Project Home, AI Assistant closed", area: area, from: "project-home", via: "ai-toggle")
            }
            await ui.step("ai panel") {
                try await ui.aiOpen()
                await ui.capture("ai-panel", title: "AI Assistant, not available in this build", area: area, from: "ai-closed-home", via: "ai-toggle")
            }
            await ui.capMenu(id: "ai-settings-menu", name: "ai-settings-menu", title: "AI Assistant, settings menu", area: area, from: "ai-panel")
            await ui.step("ai offline settings") {
                try await ui.capMenuButton(id: "ai-settings-menu", item: "Offline AI settings")
                _ = try await ui.wait(id: "settings-ai", timeout: 10)
                ui.aiFrontSettings()
                await ui.idle(1.5)
                await ui.capture("ai-offline-ai-settings", title: "Settings, Offline AI, opened from the assistant", area: area, from: "ai-settings-menu", via: "ai-open-settings")
                if ui.aiScrollToEnd() {
                    await ui.capture("ai-offline-ai-settings-details", title: "Settings, Offline AI, technical details and AI health", area: area, from: "ai-offline-ai-settings", via: "scroll")
                }
                await ui.capRecover()
                ui.aiFrontMain()
            }
            for w in ["Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Schedule", "Call Sheets", "Files"] {
                await ui.step("ai open in \(w)") {
                    try await ui.capNav(w)
                    await ui.idle(0.8)
                    try await ui.aiOpen()
                    await ui.capture("ai-panel-\(ui.slug(w))", title: "AI Assistant open in \(w)", area: area, from: "ai-panel", via: "sidebar")
                }
            }
            await ui.step("ai resize wider") {
                try await ui.capNav("Screenplay")
                try await ui.aiOpen()
                try await ui.aiResize(steps: 14)
                await ui.capture("ai-panel-wide", title: "AI Assistant dragged to its widest", area: area, from: "ai-panel-screenplay", via: "ai-resize")
            }
            await ui.step("ai resize narrower") {
                try await ui.aiResize(steps: -24)
                await ui.capture("ai-panel-narrow", title: "AI Assistant dragged to its narrowest", area: area, from: "ai-panel-wide", via: "ai-resize")
                try await ui.aiResize(steps: 3)
            }
            await ui.step("ai closed with close button") {
                try await ui.capAct(id: "ai-close")
                try await ui.waitGone(id: "ai-panel", timeout: 8)
                await ui.capture("ai-closed-screenplay", title: "Screenplay, AI Assistant closed with Close", area: area, from: "ai-panel-screenplay", via: "ai-close")
            }
            await ui.step("ai closed in breakdown") {
                try await ui.capNav("Breakdown")
                await ui.capture("ai-closed-breakdown", title: "Breakdown, AI Assistant closed", area: area, from: "ai-closed-screenplay", via: "sidebar")
            }
            await ui.step("ai sidebar hidden") {
                try await ui.aiOpen()
                try await ui.menu(["View", "Hide Sidebar"])
                await ui.idle(0.8)
                await ui.capture("ai-panel-sidebar-hidden", title: "AI Assistant open, sidebar hidden", area: area, from: "ai-panel-breakdown", via: "menu-view")
                try await ui.menu(["View", "Show Sidebar"])
                await ui.idle(0.6)
            }
            await ui.step("ai narrow window") {
                try await ui.aiClose()
                ui.capBigWindow(width: 1000)
                await ui.idle(0.8)
                try await ui.aiOpen()
                await ui.capture("ai-panel-narrow-window", title: "AI Assistant in a narrow window, sidebar folded away", area: area, from: "ai-closed-breakdown", via: "ai-toggle")
                ui.capBigWindow()
                await ui.idle(0.4)
                if ui.menuItemEnabled(["View", "Show Sidebar"]) { try? await ui.menu(["View", "Show Sidebar"]) }
                await ui.idle(0.6)
            }
            await ui.step("ai search palette") {
                ui.aiFrontMain()
                await ui.idle(0.4)
                try await ui.menu(["Edit", "Search and Commands\u{2026}"])
                _ = try await ui.wait(id: "palette-field", timeout: 6)
                try await ui.capTypeIn("assistant", into: "palette-field")
                await ui.idle(0.8)
                await ui.capture("ai-search-palette", title: "Search and Commands, no match for assistant", area: area, from: "ai-panel-breakdown", via: "menu-edit")
                await ui.capRecover()
                try? await ui.escape()
            }
        },
    ]

    static let capturePackages: [Scenario] = [
        tour("11-packages", order: 11200, timeout: 900) { ui, ctx in
            let area = "Packages"
            let dir = try ui.pkDir(ctx)
            await ui.capMainMenu("File", name: "menu-file-with-project", from: "screenplay-editor")
            await ui.step("share for review menu") {
                guard let file = NSApp.mainMenu?.items.first(where: { $0.title == "File" })?.submenu,
                      let share = file.items.first(where: { $0.title == "Share for Review" })?.submenu else { throw DriverError("no Share for Review menu") }
                let y = 30 + CGFloat(file.items.firstIndex(where: { $0.title == "Share for Review" }) ?? 8) * 22
                await ui.capture("package-share-for-review-menu", title: "File, Share for Review", area: area, from: "menu-file-with-project", via: "menu-file",
                                 menu: share.copy() as? NSMenu ?? share, anchor: CGPoint(x: 260, y: y))
            }
            // Empty states first, before any package exists.
            await ui.step("history empty") {
                try await ui.menu(["File", "Import & Export History\u{2026}"])
                _ = try await ui.wait(id: "history-tabs", timeout: 15)
                await ui.capture("package-import-export-history", title: "Import and Export History, nothing yet", area: area, from: "menu-file-with-project", via: "menu-file")
                await ui.capRecover()
            }
            await ui.step("review queue empty") {
                try await ui.menu(["File", "Review Queue\u{2026}"])
                await ui.idle(1.2)
                await ui.capture("package-review-queue", title: "Review Queue, nothing waiting", area: area, from: "menu-file-with-project", via: "menu-file")
                await ui.capRecover()
            }
            await ui.step("import exchange empty") {
                try await ui.menu(["File", "Import Exchange Package\u{2026}"])
                _ = try await ui.wait(id: "exchange-import-choose", timeout: 15)
                await ui.capture("package-import-exchange-package", title: "Import Exchange Package, choose a file", area: area, from: "menu-file-with-project", via: "menu-file")
                await ui.capRecover()
            }
            await ui.step("print") {
                try await ui.menu(["File", "Print\u{2026}"])
                await ui.idle(1.5)
                await ui.capture("package-print", title: "File, Print", area: area, from: "menu-file-with-project", via: "menu-file")
                await ui.capRecover()
            }
            // Back Up Project: the sheet, a label and a place, portable, then the backup made.
            let backup = dir.appendingPathComponent("Night Bus backup.ofbackup")
            await ui.step("backup") {
                try await ui.menu(["File", "Back Up Project\u{2026}"])
                try await ui.waitText("A recoverable copy of supported project state.", timeout: 20)
                await ui.capture("package-back-up-project", title: "Create Backup", area: area, from: "menu-file-with-project", via: "menu-file")
                try await ui.capTypeIn("before-rewrite", into: "backup-label")
                try await ui.choosing(backup) { try await ui.capAct(id: "backup-browse") }
                try await ui.waitText(backup.path, timeout: 10)
                await ui.capture("package-backup-ready", title: "Create Backup, a label and a place chosen", area: area, from: "package-back-up-project", via: "backup-browse")
                try await ui.capAct(id: "backup-portable")
                await ui.capture("package-backup-portable", title: "Create Backup, portable backup ticked", area: area, from: "package-backup-ready", via: "backup-portable")
                try await ui.capAct(id: "backup-portable")
                try await ui.capAct(id: "backup-create")
                try await ui.waitText("Backup created.", timeout: 60)
                await ui.capture("package-backup-created", title: "Create Backup, backup created", area: area, from: "package-backup-ready", via: "backup-create")
                try await ui.capAct(id: "backup-close")
                try await ui.waitGone(id: "backup-sheet")
            }
            await ui.step("export full project") {
                try await ui.menu(["File", "Export Full Project\u{2026}"])
                try await ui.waitText("Private project transfer.", timeout: 20)
                await ui.capture("package-export-full-project", title: "Export Full Project", area: area, from: "menu-file-with-project", via: "menu-file")
                if ui.exists(id: "export-project-portable") {
                    try await ui.capAct(id: "export-project-portable")
                    await ui.capture("package-export-full-project-portable", title: "Export Full Project, linked files copied in", area: area, from: "package-export-full-project", via: "export-project-portable")
                    try await ui.capAct(id: "export-project-portable")
                }
                try await ui.choosing(dir.appendingPathComponent("Night Bus.ofproject")) { try await ui.capAct(id: "export-project-confirm") }
                try await ui.waitText("Project package exported.", timeout: 60)
                await ui.capture("package-export-full-project-done", title: "Export Full Project, package exported", area: area, from: "package-export-full-project", via: "export-project-confirm")
                try await ui.capAct(id: "export-project-close")
                try await ui.waitGone(id: "export-project-sheet")
            }
            // Share for Review: every package type's sheet as it opens.
            // This build's Share for Review menu offers the script and the Beat Board packages only.
            let shares: [(String, String)] = [("Export Review Package (script)\u{2026}", "script"), ("Beat Board Package\u{2026}", "beat-board")]
            for (item, key) in shares {
                await ui.step("share \(key)") {
                    try await ui.menu(["File", "Share for Review", item])
                    _ = try await ui.wait(id: "exchange-export-sheet", timeout: 15)
                    await ui.idle(1.5)
                    await ui.capture("package-share-\(key)", title: "Share for Review, \(item.replacingOccurrences(of: "\u{2026}", with: ""))", area: area, from: "package-share-for-review-menu", via: "menu-file")
                    if key == "beat-board", ui.pkSegment("One Act") {
                        await ui.idle(1.0)
                        await ui.capture("package-share-beat-board-one-act", title: "Beat Board Package, one act", area: area, from: "package-share-beat-board", via: "exchange-scope")
                    }
                    await ui.capRecover()
                }
            }
            let review = dir.appendingPathComponent("Night Bus review.ofscriptreview")
            await ui.step("share script scopes and export") {
                try await ui.menu(["File", "Share for Review", "Export Review Package (script)\u{2026}"])
                try await ui.waitText("Private notes", timeout: 20)
                if ui.pkSegment("Specific scenes only") {
                    await ui.idle(1.2)
                    await ui.capture("package-share-script-choose-scenes", title: "Script Review Package, no scene chosen yet", area: area, from: "package-share-script", via: "exchange-scope")
                    if let first = ui.capQueryPrefix("exchange-scene-").first(where: { $0.identifier != "exchange-scene-list" }) {
                        try await ui.press(first)
                        await ui.idle(1.0)
                        await ui.capture("package-share-script-one-scene", title: "Script Review Package, one scene chosen", area: area, from: "package-share-script-choose-scenes", via: "exchange-scene")
                    }
                    if ui.pkSegment("Script + comments") {
                        await ui.idle(1.0)
                        await ui.capture("package-share-script-comments", title: "Script Review Package, script and comments", area: area, from: "package-share-script", via: "exchange-scope")
                    }
                    _ = ui.pkSegment("Full script")
                    await ui.idle(1.0)
                }
                try await ui.choosing(review) { try await ui.capAct(id: "exchange-export-confirm") }
                try await ui.waitText("Package exported.", timeout: 30)
                await ui.capture("package-share-script-done", title: "Script Review Package, exported", area: area, from: "package-share-script", via: "exchange-export-confirm")
                try await ui.capAct(id: "exchange-export-close")
                try await ui.waitGone(id: "exchange-export-sheet")
            }
            // The review round trip: open the package as a reviewer, comment, send a response, import it.
            let response = dir.appendingPathComponent("Night Bus reply.ofresponse")
            await ui.step("open review package") {
                try await ui.choosing(review) { try await ui.menu(["File", "Open Review Package\u{2026}"]) }
                try await ui.waitText("Review package. Comments are separate from the original project.", timeout: 20)
                await ui.idle(0.8)
                await ui.capture("package-review-viewer", title: "Review package open for comments", area: area, from: "menu-file-with-project", via: "menu-file")
                try await ui.capTypeIn("Tighten the depot line before the bus pulls out.", into: "review-comment-field")
                await ui.capture("package-review-comment-typed", title: "Review package, a comment typed", area: area, from: "package-review-viewer", via: "review-comment-field")
                try await ui.capAct(id: "review-add-comment")
                try await ui.waitText("Yours", timeout: 10)
                await ui.capture("package-review-comment-added", title: "Review package, a comment added", area: area, from: "package-review-comment-typed", via: "review-add-comment")
                try await ui.choosing(response) { try await ui.capAct(label: "Export Response Package", role: "AXButton") }
                try await ui.waitText("Response package exported.", timeout: 30)
                await ui.capture("package-review-response-exported", title: "Review package, response exported", area: area, from: "package-review-comment-added", via: "review-export-response")
                try await ui.capAct(id: "review-close")
                try await ui.waitGone(id: "review-viewer")
            }
            await ui.step("import response") {
                try await ui.menu(["File", "Import Exchange Package\u{2026}"])
                _ = try await ui.wait(id: "exchange-import-choose", timeout: 15)
                try await ui.choosing(response) { try await ui.capAct(id: "exchange-import-choose") }
                _ = try await ui.wait(id: "exchange-changes", timeout: 30)
                await ui.idle(0.8)
                await ui.capture("package-import-preview", title: "Import Exchange Package, the changes to review", area: area, from: "package-import-exchange-package", via: "exchange-import-choose")
                if let expand = ui.all().first(where: { $0.identifier.hasPrefix("exchange-expand-") }) {
                    try await ui.press(expand)
                    await ui.idle(0.8)
                    await ui.capture("package-import-preview-expanded", title: "Import Exchange Package, a group opened", area: area, from: "package-import-preview", via: expand.identifier)
                }
                try await ui.capAct(id: "exchange-accept-safe")
                try await ui.waitText("Imported", timeout: 30)
                await ui.capture("package-import-done", title: "Import Exchange Package, imported", area: area, from: "package-import-preview", via: "exchange-accept-safe")
                try await ui.capAct(id: "exchange-import-close")
                try await ui.waitGone(id: "exchange-import-sheet")
            }
            await ui.step("review queue after import") {
                try await ui.menu(["File", "Review Queue\u{2026}"])
                await ui.idle(1.2)
                await ui.capture("package-review-queue-after-import", title: "Review Queue after a response import", area: area, from: "package-import-done", via: "menu-file")
                await ui.capRecover()
            }
            await ui.step("history filled") {
                try await ui.menu(["File", "Import & Export History\u{2026}"])
                _ = try await ui.wait(id: "history-tabs", timeout: 15)
                var last = "package-import-export-history"
                for tab in ["Imports", "Review records", "My reviews", "Package log"] {
                    if !ui.pkSegment(tab) { try await ui.capAct(label: tab) }
                    await ui.idle(0.9)
                    let n = "package-history-\(ui.slug(tab))"
                    await ui.capture(n, title: "Import and Export History, \(tab)", area: area, from: last, via: "history-tabs")
                    last = n
                }
                await ui.capRecover()
            }
            // Restore Backup or Package: the chooser, a damaged file, then the real backup.
            await ui.step("restore chooser") {
                try await ui.menu(["File", "Restore Backup or Package\u{2026}"])
                _ = try await ui.wait(id: "open-project-choose", timeout: 15)
                await ui.capture("package-restore", title: "Restore Backup or Package, choose a package", area: area, from: "menu-file-with-project", via: "menu-file")
                let damaged = dir.appendingPathComponent("damaged.ofbackup")
                var bytes = Data("PK".utf8)
                bytes.append(contentsOf: [0x03, 0x04])
                bytes.append(Data(" this is not a whole package".utf8))
                try bytes.write(to: damaged)
                try await ui.choosing(damaged) { try await ui.capAct(id: "open-project-choose") }
                _ = try await ui.wait(id: "open-project-error", timeout: 30)
                await ui.capture("package-restore-damaged", title: "Restore Backup or Package, a damaged file refused", area: area, from: "package-restore", via: "open-project-choose")
                await ui.capRecover()
            }
            await ui.step("restore backup collision") {
                try await ui.menu(["File", "Restore Backup or Package\u{2026}"])
                _ = try await ui.wait(id: "open-project-choose", timeout: 15)
                try await ui.choosing(backup) { try await ui.capAct(id: "open-project-choose") }
                try await ui.waitText("This project already exists on this computer", timeout: 30)
                await ui.idle(0.6)
                await ui.capture("package-restore-collision", title: "Restore Backup or Package, the project already exists", area: area, from: "package-restore", via: "open-project-choose")
                try await ui.capAct(label: "Open as Copy", role: "AXButton")
                try await ui.waitText("Opened as a copy.", timeout: 60)
                await ui.capture("package-restore-opened-copy", title: "Restore Backup or Package, opened as a copy", area: area, from: "package-restore-collision", via: "open-as-copy")
                await ui.capRecover()
            }
        },
    ]
}

// MARK: helpers for this section

fileprivate extension UI {
    var aiPanelOpen: Bool { exists(id: "ai-panel") }

    /// Opens the assistant panel with its toolbar button when it is closed, and waits for it.
    func aiOpen() async throws {
        if aiPanelOpen { return }
        // Command+J, as the toolbar button's own shortcut: the toolbar item does not take an accessibility press.
        try await shortcut("j", code: 38)
        _ = try await wait(id: "ai-panel", timeout: 10, "the AI Assistant panel")
        await idle(1.2)
    }

    /// Closes the assistant panel with Command+J when it is open.
    func aiClose() async throws {
        guard aiPanelOpen else { return }
        try await shortcut("j", code: 38)
        try await waitGone(id: "ai-panel", timeout: 8)
        await idle(0.6)
    }

    /// Widens (positive) or narrows (negative) the panel through its resize handle, 24 points a step.
    func aiResize(steps: Int) async throws {
        let e = try await wait(id: "ai-resize", timeout: 6, "the AI Assistant resize handle")
        let sel = steps > 0 ? "accessibilityPerformIncrement" : "accessibilityPerformDecrement"
        for _ in 0..<abs(steps) {
            let fresh = query(id: "ai-resize").first ?? e
            _ = fresh.callBool(sel)
            await idle(0.05)
        }
        await idle(0.6)
    }

    /// Scrolls the key window's largest scroll view to its end. False when there is nothing to scroll.
    func aiScrollToEnd() -> Bool {
        guard let w = NSApp.keyWindow, let content = w.contentView else { return false }
        var best: NSScrollView?
        func walk(_ v: NSView) {
            if let s = v as? NSScrollView, let doc = s.documentView, doc.frame.height > s.contentSize.height + 20,
               s.frame.height > (best?.frame.height ?? 0) { best = s }
            for c in v.subviews { walk(c) }
        }
        walk(content)
        guard let s = best, let doc = s.documentView else { return false }
        let y = doc.isFlipped ? doc.frame.height - s.contentSize.height : 0
        s.contentView.scroll(to: NSPoint(x: 0, y: y))
        s.reflectScrolledClipView(s.contentView)
        return true
    }

    /// Brings the Settings window to the front and makes it key, so the picture is of it.
    func aiFrontSettings() {
        let names = ["Settings", "General", "Appearance", "Offline AI", "Diagnostics"]
        if let w = NSApp.windows.first(where: { $0.isVisible && names.contains($0.title) }) {
            NSApp.activate(ignoringOtherApps: true)
            w.makeKeyAndOrderFront(nil)
        }
    }

    /// Gives the keyboard back to the project window after the Settings window closed.
    func aiFrontMain() {
        if let main = NSApp.windows.filter({ $0.isVisible && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) {
            NSApp.activate(ignoringOtherApps: true)
            main.makeKeyAndOrderFront(nil)
        }
    }

    /// A folder for this scenario's packages, inside its own work folder.
    func pkDir(_ ctx: Context) throws -> URL {
        let d = ctx.work.appendingPathComponent("capture-packages")
        try FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    /// Chooses the segment with this label in any visible segmented control (a segmented Picker).
    func pkSegment(_ label: String) -> Bool {
        var found: NSSegmentedControl?
        var index = 0
        func walk(_ v: NSView) {
            guard found == nil else { return }
            if let s = v as? NSSegmentedControl {
                for i in 0..<s.segmentCount where s.label(forSegment: i) == label { found = s; index = i; return }
            }
            for c in v.subviews { walk(c) }
        }
        for w in NSApp.windows where w.isVisible { if let c = w.contentView { walk(c) } }
        guard let s = found else { return false }
        s.selectedSegment = index
        s.sendAction(s.action, to: s.target)
        log("segment \(label)")
        return true
    }
}
