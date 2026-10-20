// Capture tour section: Breakdown. Primitives and helpers are in CaptureTour.swift.
// Three scenarios: the source and the page layout (05-breakdown-source), the scene's elements
// (05-breakdown-elements) and the empty project (05-breakdown-empty).

import AppKit
import Foundation

private let bdArea = "Breakdown"

fileprivate extension UI {
    /// Captures with the project window active: an inactive window draws its sidebar material blank.
    func bdCapture(_ name: String, title: String, area: String, from: String? = nil, via: String? = nil, settle: Double = 0.8) async {
        if !hasOverlay, let main = NSApp.windows.filter({ $0.isVisible && $0.sheetParent == nil && $0.canBecomeMain && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }), !main.isKeyWindow {
            NSApp.activate(ignoringOtherApps: true)
            main.makeKeyAndOrderFront(nil)
            await idle(0.6)
        }
        // The window server sometimes hands back the sidebar before its material is drawn (one flat colour): wait for it.
        await idle(settle)
        let deadline = Date().addingTimeInterval(4)
        while Date() < deadline, let w = capturePrimary(), let rep = captureRender(w), bdSidebarBlank(rep) { await idle(0.4) }
        await capture(name, title: title, area: area, from: from, via: via, settle: 0.15)
    }

    /// True when the sidebar strip of a window picture is one flat colour (no title, no rows drawn).
    func bdSidebarBlank(_ rep: NSBitmapImageRep) -> Bool {
        var seen = Set<String>()
        for gx in 0..<8 { for gy in 0..<30 {
            let x = Int(Double(rep.pixelsWide) * (0.02 + 0.012 * Double(gx))), y = Int(Double(rep.pixelsHigh) * (0.08 + 0.008 * Double(gy)))
            if let c = rep.colorAt(x: x, y: y) { seen.insert(String(format: "%.2f%.2f%.2f", c.redComponent, c.greenComponent, c.blueComponent)) }
        } }
        return seen.count <= 1
    }

    /// Hide scenes and Hide panel are remembered on this Mac: start every scenario with both shown.
    func breakdownShowPanes() async {
        for id in ["breakdown-toggle-scenes", "breakdown-toggle-panel"] {
            if let b = query(id: id).first(where: { $0.role == "AXButton" }), b.text.contains("Show") {
                _ = b.performPress()
                await idle(0.5)
            }
        }
    }

    /// Opens Breakdown, chooses the imported draft as the Production Source, with an optional reason.
    func breakdownSetSource(capturing: Bool) async throws {
        try await capNav("Breakdown")
        try await waitText("Choose a screenplay source to begin the breakdown.")
        try await capAct(id: "select-source")
        let deadline = Date().addingTimeInterval(15)
        while Date() < deadline, capQueryPrefix("draft-").isEmpty { await idle(0.2) }
        guard let d = capQueryPrefix("draft-").first(where: { $0.role == "AXButton" }) ?? capQueryPrefix("draft-").first else { throw DriverError("no draft choice") }
        if capturing {
            await capture("breakdown-source-sheet", title: "Breakdown, choose the production source", area: bdArea, from: "breakdown-no-source", via: "select-source")
            try await capTypeIn("The table read draft", into: "source-reason")
            await capture("breakdown-source-sheet-reason", title: "Choose the production source, a reason typed", area: bdArea, from: "breakdown-source-sheet", via: "source-reason")
        }
        try await press(d)
        try await capAct(id: "set-production-source")
        try await waitText("Production Source:")
        await idle(0.8)
    }

    func breakdownSelectScene(_ n: Int) async throws {
        try await Scenarios.capSelectScene(self, n)
        try await wait(label: "Scene \(n) breakdown", timeout: 15)
        await idle(0.4)
    }

    /// Names of the confirmed elements of the scene shown, in panel order.
    func breakdownElementNames() -> [String] {
        capQueryPrefix("element-actions-").map { String($0.identifier.dropFirst("element-actions-".count)) }
    }

    /// The script of the scene shown, as its text view.
    func breakdownScript() -> NSTextView? { query(id: "script-text").first?.obj as? NSTextView }

    /// Selects a word of the script the way a drag does, so the tag controls turn on.
    func breakdownSelectWord(_ word: String) async throws {
        guard let tv = breakdownScript() else { throw DriverError("no script text") }
        let r = (tv.string as NSString).range(of: word)
        guard r.location != NSNotFound else { throw DriverError("no \(word) in the script") }
        tv.window?.makeFirstResponder(tv)
        tv.setSelectedRange(r)
        tv.scrollRangeToVisible(r)
        await idle(0.8)
    }

    /// The right-click menu of the script with a word selected, drawn open on that word.
    func breakdownScriptContext(_ word: String, name: String, title: String, from: String) async {
        await step("context menu \(name)") {
            try await breakdownSelectWord(word)
            guard let tv = breakdownScript(), let w = capturePrimary() else { throw DriverError("no script text") }
            let r = (tv.string as NSString).range(of: word)
            let rect = tv.firstRect(forCharacterRange: r, actualRange: nil)
            let center = NSPoint(x: rect.midX, y: rect.midY)
            guard let menu = capContextMenuItems(at: center) else { throw DriverError("no context menu on the script") }
            await capture(name, title: title, area: bdArea, from: from, via: "script-text", menu: menu.copy() as? NSMenu ?? menu,
                          anchor: CGPoint(x: center.x - w.frame.minX, y: w.frame.maxY - center.y + 8))
        }
    }

    /// A confirmed element whose row is on screen (its actions menu can be clicked), other than `except`.
    func breakdownVisibleElement(except: String) -> String? {
        guard let w = capturePrimary() else { return nil }
        return capQueryPrefix("element-actions-").first { e in
            let f = e.frame
            return !e.identifier.hasSuffix("-\(except)") && f.width > 0 && f.minY > w.frame.minY + 80 && f.maxY < w.frame.maxY - 120
        }.map { String($0.identifier.dropFirst("element-actions-".count)) }
    }

    /// The disclosure button of a category section in the elements panel.
    func breakdownCategoryButton(_ category: String) -> Element? {
        all().first { $0.role == "AXButton" && $0.label.hasPrefix("\(category), ") }
    }
}

extension Scenarios {
    static let captureBreakdown: [Scenario] = [
        // The source, the page layout, filters, export and tagging words in the script.
        tour("05-breakdown-source", order: 5000) { ui, _ in
            try await ui.capNav("Breakdown")
            try await ui.waitText("Choose a screenplay source to begin the breakdown.")
            await ui.bdCapture("breakdown-no-source", title: "Breakdown, choose a screenplay source", area: bdArea)
            try await ui.breakdownSetSource(capturing: true)
            await ui.breakdownShowPanes()
            await ui.bdCapture("breakdown-scenes", title: "Breakdown, source set, scene 1 open", area: bdArea, from: "breakdown-source-sheet", via: "set-production-source")
            await ui.step("scene 3") {
                try await ui.breakdownSelectScene(3)
                await ui.bdCapture("breakdown-scene-3", title: "Breakdown, scene 3 selected", area: bdArea, from: "breakdown-scenes", via: "scene-list")
                try await ui.breakdownSelectScene(1)
            }
            await ui.step("hide scenes and panel") {
                try await ui.capAct(id: "breakdown-toggle-scenes")
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-scenes-hidden", title: "Breakdown, scene list hidden", area: bdArea, from: "breakdown-scenes", via: "breakdown-toggle-scenes")
                try await ui.capAct(id: "breakdown-toggle-panel")
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-script-only", title: "Breakdown, scene list and panel hidden", area: bdArea, from: "breakdown-scenes-hidden", via: "breakdown-toggle-panel")
                try await ui.capAct(id: "breakdown-toggle-scenes")
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-panel-hidden", title: "Breakdown, scene panel hidden", area: bdArea, from: "breakdown-script-only", via: "breakdown-toggle-scenes")
                try await ui.capAct(id: "breakdown-toggle-panel")
                await ui.idle(0.4)
            }
            await ui.breakdownShowPanes()
            await ui.capMenu(id: "scene-filter", name: "breakdown-filter-menu", title: "Breakdown, scene filter menu", area: bdArea, from: "breakdown-scenes")
            await ui.step("filter complete") {
                try await ui.choose("scene-filter", "Complete")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-filter-none", title: "Breakdown, no scenes match the filter", area: bdArea, from: "breakdown-filter-menu", via: "scene-filter", settle: 1.5)
            }
            await ui.step("filter missing") {
                try await ui.choose("scene-filter", "Only missing")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-filter-missing", title: "Breakdown, scenes missing a breakdown", area: bdArea, from: "breakdown-filter-menu", via: "scene-filter", settle: 1.5)
            }
            await ui.step("filter reset") { try await ui.choose("scene-filter", "All scenes"); await ui.idle(0.6) }
            await ui.step("change source") {
                try await ui.capAct(id: "change-source")
                _ = try await ui.wait(id: "screenplay-drafts", timeout: 8)
                await ui.bdCapture("breakdown-change-source", title: "Breakdown, change the production source", area: bdArea, from: "breakdown-scenes", via: "change-source")
                await ui.capRecover()
            }
            await ui.step("export") {
                try await ui.capAct(id: "breakdown-export")
                _ = try await ui.wait(id: "export-run", timeout: 8)
                await ui.bdCapture("breakdown-export-sheet", title: "Export Breakdown, PDF", area: bdArea, from: "breakdown-scenes", via: "breakdown-export")
                if (try? await ui.capAct(label: "CSV", role: "AXRadioButton")) != nil {
                    await ui.idle(0.4)
                    await ui.bdCapture("breakdown-export-csv", title: "Export Breakdown, CSV", area: bdArea, from: "breakdown-export-sheet", via: "export-format")
                }
                if (try? await ui.capAct(label: "Spreadsheet (XLSX)", role: "AXRadioButton")) != nil {
                    await ui.idle(0.4)
                    await ui.bdCapture("breakdown-export-xlsx", title: "Export Breakdown, spreadsheet", area: bdArea, from: "breakdown-export-sheet", via: "export-format")
                }
                for id in ["export-option-currentOnly", "export-option-includeNotes"] { try? await ui.capAct(id: id) }
                await ui.idle(0.4)
                await ui.bdCapture("breakdown-export-options", title: "Export Breakdown, options changed", area: bdArea, from: "breakdown-export-sheet", via: "export-option-currentOnly")
                if !(await ui.tryPress(id: "export-cancel", wait: 2)) { await ui.capRecover() }
                await ui.idle(0.4)
            }
            await ui.step("script selection") {
                try await ui.breakdownSelectWord("REGISTER")
                _ = try await ui.wait(id: "tag-selection", timeout: 6)
                await ui.bdCapture("breakdown-script-selection", title: "Breakdown, words selected in the script", area: bdArea, from: "breakdown-scenes", via: "script-text")
            }
            await ui.breakdownScriptContext("REGISTER", name: "breakdown-script-context-menu", title: "Breakdown, right-click on selected script words", from: "breakdown-script-selection")
            await ui.step("reselect") { try await ui.breakdownSelectWord("REGISTER") }
            await ui.capMenu(id: "script-tag-menu", name: "breakdown-tag-menu", title: "Breakdown, Tag Selection as menu", area: bdArea, from: "breakdown-script-selection")
            await ui.step("tag sheet") {
                if ui.query(id: "tag-selection").isEmpty { try await ui.breakdownSelectWord("REGISTER") }
                try await ui.capAct(id: "tag-selection")
                _ = try await ui.wait(id: "add-name", timeout: 6)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-tag-sheet", title: "Breakdown, tag the selected words", area: bdArea, from: "breakdown-script-selection", via: "tag-selection")
                try await ui.choose("add-category", "Props")
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-tag-sheet-props", title: "Tag the selected words as a prop", area: bdArea, from: "breakdown-tag-sheet", via: "add-category")
                try await ui.capAct(id: "add-to-scene")
                try await ui.waitGone(id: "add-name", timeout: 8)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-tagged", title: "Breakdown, words tagged as a prop", area: bdArea, from: "breakdown-tag-sheet-props", via: "add-to-scene")
            }
            // Leave the script without a selection or the keyboard, so the app quits cleanly.
            if let tv = ui.breakdownScript() { tv.setSelectedRange(NSRange(location: 0, length: 0)); tv.window?.makeFirstResponder(nil) }
            await ui.idle(0.5)
        },

        // The elements of a scene: suggestions, review, accepting, categories, menus, notes, adding by hand.
        tour("05-breakdown-elements", order: 5100) { ui, _ in
            try await ui.breakdownSetSource(capturing: false)
            await ui.breakdownShowPanes()
            try await ui.breakdownSelectScene(1)
            await ui.step("suggest") {
                try await ui.capAct(id: "suggest-elements")
                try await ui.waitText("Nothing is added to production until you accept.")
                await ui.bdCapture("breakdown-suggestions", title: "Breakdown, suggested elements waiting for review", area: bdArea, from: "breakdown-scenes", via: "suggest-elements")
            }
            await ui.step("review sheet") {
                try await ui.capAct(id: "review-suggestions")
                _ = try await ui.wait(id: "accept-all-safe", timeout: 8)
                await ui.bdCapture("breakdown-review-sheet", title: "Review suggested elements", area: bdArea, from: "breakdown-suggestions", via: "review-suggestions")
                if let box = ui.all().first(where: { $0.role == "AXCheckBox" && $0.label.hasPrefix("Select ") }) {
                    try await ui.press(box)
                    await ui.idle(0.5)
                    await ui.bdCapture("breakdown-review-deselected", title: "Review suggested elements, one left out", area: bdArea, from: "breakdown-review-sheet", via: box.label)
                }
                await ui.capRecover()
            }
            await ui.step("edit suggestion") {
                guard let edit = ui.all().first(where: { $0.role == "AXButton" && $0.label.hasPrefix("Edit ") }) else { throw DriverError("no Edit on a suggestion") }
                try await ui.press(edit)
                _ = try await ui.wait(id: "accept-edited", timeout: 8)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-edit-suggestion", title: "Edit a suggestion before accepting", area: bdArea, from: "breakdown-suggestions", via: edit.label)
                await ui.capRecover()
            }
            await ui.step("accept one") {
                guard let b = Scenarios.capAccepts(ui).first else { throw DriverError("no suggestion to accept") }
                try await ui.press(b)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-accepted-one", title: "Breakdown, one suggestion accepted", area: bdArea, from: "breakdown-suggestions", via: b.identifier)
            }
            await ui.step("reject one") {
                guard let r = ui.all().last(where: { $0.role == "AXButton" && $0.label.hasPrefix("Reject ") && $0.identifier != "reject-selected" }) else { throw DriverError("no Reject") }
                try await ui.press(r)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-rejected-one", title: "Breakdown, a suggestion rejected", area: bdArea, from: "breakdown-accepted-one", via: r.label)
            }
            await ui.step("accept all") {
                var n = 0
                while n < 30, let b = Scenarios.capAccepts(ui).first { try await ui.press(b); n += 1; await ui.idle(0.4) }
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-all-accepted", title: "Breakdown, every suggestion accepted", area: bdArea, from: "breakdown-accepted-one")
            }
            await ui.step("category collapsed") {
                guard let c = ui.breakdownCategoryButton("Cast") ?? ui.all().first(where: { $0.role == "AXButton" && $0.label.contains(" elements") }) else { throw DriverError("no category header") }
                try await ui.press(c)
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-category-collapsed", title: "Breakdown, a category collapsed", area: bdArea, from: "breakdown-all-accepted", via: c.label)
                if let again = ui.all().first(where: { $0.role == "AXButton" && $0.label == c.label }) { try await ui.press(again) } else { try await ui.press(c) }
                await ui.idle(0.4)
            }
            let names = ui.breakdownElementNames()
            let first = names.first ?? "Kiran"
            await ui.step("element selected") {
                guard let row = ui.query(id: "element-\(first)").first else { throw DriverError("no element row \(first)") }
                try await ui.tap(row)
                await ui.idle(0.5)
                await ui.bdCapture("breakdown-element-selected", title: "Breakdown, an element selected", area: bdArea, from: "breakdown-all-accepted", via: "element-\(first)")
            }
            await ui.capMenu(id: "element-actions-\(first)", name: "breakdown-element-actions-menu", title: "Breakdown, element actions menu", area: bdArea, from: "breakdown-element-selected")
            await ui.step("element context menu") {
                guard let row = ui.query(id: "element-\(first)").first else { throw DriverError("no element row \(first)") }
                await ui.capContext(on: row, name: "breakdown-element-context-menu", title: "Breakdown, element right-click menu", area: bdArea, from: "breakdown-element-selected")
            }
            await ui.step("element notes") {
                try await ui.choose("element-actions-\(first)", "Edit notes")
                _ = try await ui.wait(id: "element-notes", timeout: 8)
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-notes-sheet", title: "Breakdown, notes for an element", area: bdArea, from: "breakdown-element-actions-menu", via: "Edit notes")
                try await ui.capTypeIn("Wears the school bag in every scene", into: "element-notes")
                await ui.bdCapture("breakdown-notes-filled", title: "Element notes typed", area: bdArea, from: "breakdown-notes-sheet", via: "element-notes")
                try await ui.capAct(id: "save-notes")
                try await ui.waitGone(id: "element-notes", timeout: 8)
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-element-notes", title: "Breakdown, an element with notes", area: bdArea, from: "breakdown-notes-filled", via: "save-notes")
            }
            await ui.step("show in script") {
                let withEvidence = names.first { n in
                    guard let m = ui.popUpMenu(id: "element-actions-\(n)") else { return false }
                    return m.items.contains { $0.title == "Show in script" && $0.isEnabled }
                } ?? first
                try await ui.choose("element-actions-\(withEvidence)", "Show in script")
                await ui.idle(1.0)
                await ui.bdCapture("breakdown-show-in-script", title: "Breakdown, an element shown in the script", area: bdArea, from: "breakdown-element-actions-menu", via: "Show in script")
            }
            await ui.step("mark complete") {
                try await ui.capAct(id: "mark-complete")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-scene-complete", title: "Breakdown, scene marked complete", area: bdArea, from: "breakdown-all-accepted", via: "mark-complete")
            }
            await ui.step("add element") {
                try await ui.capAct(id: "add-element")
                _ = try await ui.wait(id: "add-name", timeout: 6)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-add-element-sheet", title: "Add an element to the scene", area: bdArea, from: "breakdown-scene-complete", via: "add-element")
                await ui.capMenu(id: "add-category", name: "breakdown-add-category-menu", title: "Add element, category menu", area: bdArea, from: "breakdown-add-element-sheet")
                try await ui.choose("add-category", "Special Effects")
                try await ui.capTypeIn("Flickering monitor", into: "add-name")
                await ui.idle(1.2)
                await ui.bdCapture("breakdown-add-element-filled", title: "Add an element, a new item named", area: bdArea, from: "breakdown-add-element-sheet", via: "add-name")
                try await ui.capAct(id: "add-to-scene")
                try await ui.waitGone(id: "add-name", timeout: 8)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-element-added", title: "Breakdown, an element added by hand", area: bdArea, from: "breakdown-add-element-filled", via: "add-to-scene")
            }
            await ui.step("add existing") {
                try await ui.capAct(id: "add-element")
                _ = try await ui.wait(id: "add-name", timeout: 6)
                await ui.idle(0.8)
                try await ui.choose("add-category", "Props")
                try await ui.capTypeIn("Umbrella", into: "add-name")
                await ui.idle(1.5)
                await ui.bdCapture("breakdown-add-element-existing", title: "Add an element, an existing catalog item found", area: bdArea, from: "breakdown-add-element-sheet", via: "add-name")
                await ui.capRecover()
            }
            await ui.step("remove element") {
                guard let victim = ui.breakdownVisibleElement(except: first) else { throw DriverError("no element row on screen") }
                try await ui.choose("element-actions-\(victim)", "Remove from scene")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-element-removed", title: "Breakdown, an element removed, with Undo", area: bdArea, from: "breakdown-element-added", via: "Remove from scene")
                if await ui.tryPress(id: "breakdown-notice-undo", wait: 2) {
                    await ui.idle(0.8)
                    await ui.bdCapture("breakdown-element-restored", title: "Breakdown, removal undone", area: bdArea, from: "breakdown-element-removed", via: "breakdown-notice-undo")
                }
            }
            await ui.step("scene 2") {
                try await ui.breakdownSelectScene(2)
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-scene-2", title: "Breakdown, scene 2 with quick reuse", area: bdArea, from: "breakdown-scenes", via: "scene-list")
                if let reuse = ui.all().first(where: { $0.role == "AXButton" && $0.label.hasPrefix("Reuse ") }) {
                    try await ui.press(reuse)
                    await ui.idle(0.8)
                    await ui.bdCapture("breakdown-reused", title: "Breakdown, an item reused from a nearby scene", area: bdArea, from: "breakdown-scene-2", via: reuse.label)
                }
            }
            await ui.step("catalog filter") {
                try await ui.breakdownSelectScene(1)
                try await ui.choose("element-actions-\(first)", "Show all scenes")
                _ = try await ui.wait(id: "show-all-scenes", timeout: 8)
                await ui.idle(0.6)
                await ui.bdCapture("breakdown-catalog-filter", title: "Breakdown, the scenes that use one item", area: bdArea, from: "breakdown-element-actions-menu", via: "Show all scenes with this item")
                try await ui.capAct(id: "show-all-scenes")
                await ui.idle(0.6)
            }
            await ui.step("filter category") {
                try await ui.choose("scene-filter", "Cast")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-filter-cast", title: "Breakdown, scenes filtered to Cast", area: bdArea, from: "breakdown-scenes", via: "scene-filter")
                try await ui.choose("scene-filter", "Any category")
                await ui.idle(0.4)
            }
            await ui.step("filter complete") {
                try await ui.choose("scene-filter", "Complete")
                await ui.idle(0.8)
                await ui.bdCapture("breakdown-filter-complete", title: "Breakdown, complete scenes only", area: bdArea, from: "breakdown-scenes", via: "scene-filter")
                try await ui.choose("scene-filter", "All scenes")
                await ui.idle(0.4)
            }
        },

        // A new project with no screenplay.
        tour("05-breakdown-empty", order: 5200, project: false) { ui, _ in
            await ui.capSetMode()
            ui.capBigWindow()
            try await ui.createProject("Empty Short")
            try await ui.capNav("Breakdown")
            try await ui.waitText("Choose a screenplay source to begin the breakdown.")
            await ui.idle(1.5)
            await ui.bdCapture("breakdown-empty-no-source", title: "Breakdown in an empty project", area: bdArea, from: "project-home", via: "Breakdown")
            await ui.step("empty select source") {
                try await ui.capAct(id: "select-source")
                await ui.idle(1.5)
                await ui.bdCapture("breakdown-empty-source-sheet", title: "Choose the production source, no drafts yet", area: bdArea, from: "breakdown-empty-no-source", via: "select-source")
                await ui.capRecover()
            }
        },
    ]
}
