// Real-app scenarios for the Windows port of the production screens and Home: the call sheet in the
// PDF's order that wraps on a narrow window, Sides for selected cast, the side panel beside a low
// row, the Daily View export, the cover photo menu and "Found on this Mac".

import AppKit
import Foundation

// MARK: helpers (only for this file)

fileprivate extension UI {
    /// Chooses an item of a menu button by the start of its title, without the mouse.
    func chooseMenuItem(id: String, item: String) async throws {
        let e = try await wait(id: id, timeout: 15, "the menu button \(id)")
        guard let cell = e.obj as? NSPopUpButtonCell, let menu = cell.menu else { throw DriverError("not a menu button: \(e)") }
        menu.update()
        guard let i = menu.items.firstIndex(where: { !$0.isSeparatorItem && $0.title.hasPrefix(item) }) else {
            throw DriverError("no item \(item) in \(id): \(menu.items.map(\.title))")
        }
        menu.performActionForItem(at: i)
        await idle(0.5)
    }

    /// Waits until an element is found.
    func find(_ what: String, timeout: Double = 15, _ match: () -> Element?) async throws -> Element {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let e = match() { return e }
            await idle(0.1)
        }
        throw DriverError("not found within \(Int(timeout)) s: \(what)")
    }

    /// The main window of the app.
    var mainWindow: NSWindow? { NSApp.windows.first { $0.isVisible && $0.isMainWindow } ?? NSApp.windows.first { $0.isVisible } }

    /// Every element under one element, as far as twelve levels down.
    func below(_ e: Element) -> [Element] { Scenarios.descendants(e) }
}

extension Scenarios {
    /// Register in Scenarios.swift: `+ productionPortScenarios` in `productionGroup` or `all`.
    static let productionPortScenarios: [Scenario] = [
        Scenario("62-call-sheets-exports/call-sheet-reads-like-the-pdf-and-wraps-when-narrow", timeout: 600) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await createCallSheet(ui)
            let doc = try await ui.wait(id: "callsheet-document", timeout: 15, "the call sheet")
            let text = ui.below(doc).map(\.text).joined(separator: "\n")
            // Heading by heading, as the exported PDF.
            let order = ["PRODUCTION", "LOCATIONS", "SCENE SCHEDULE", "Cast & Crew", "TALENT / CAST", "Production notes", "PRACTICAL / SAFETY INFORMATION"]
            var at = text.startIndex
            for h in order {
                guard let r = text.range(of: h, range: at..<text.endIndex) else { throw DriverError("the heading \(h) is missing or out of order") }
                at = r.upperBound
            }
            try ui.expect(!text.contains("PRODUCTION HEADER") && !text.contains("PRACTICAL NOTES"), "an old section name still shows")
            try ui.expect(ui.exists(id: "callsheet-section-masthead") && ui.exists(id: "callsheet-section-day-and-times"), "the masthead and the crew call box show")
            try ui.expect(text.contains("Est. wrap") && text.contains("Production emergency contact"), "the PDF's field names show")
            // A narrow window: the paper fits its width (no sideways scrolling).
            let window = try ui.mainWindow.orThrow("no main window")
            let before = window.frame
            window.setFrame(NSRect(x: before.minX, y: before.minY, width: 900, height: before.height), display: true)
            await ui.idle(1.0)
            let paper = try await ui.wait(id: "callsheet-document", timeout: 5, "the call sheet")
            try ui.expect(paper.frame.maxX <= window.frame.maxX + 1 && paper.frame.minX >= window.frame.minX - 1,
                          "the paper (\(paper.frame)) is wider than the window (\(window.frame))")
            window.setFrame(before, display: true)
            try await ui.press(id: "callsheet-back")
        },

        Scenario("61-schedule/side-panel-stays-beside-a-low-row", timeout: 500) { ui, _, ctx in
            try await scheduleDays(ui, ctx, project: "Signal Yard")
            let window = try ui.mainWindow.orThrow("no main window")
            let before = window.frame
            // A short window, so the scene is low in the list.
            window.setFrame(NSRect(x: before.minX, y: before.minY, width: before.width, height: 560), display: true)
            await ui.idle(0.8)
            try await ui.press(id: "schedule-mode-list")
            try await ui.chooseMenuItem(id: "strip-actions-2", item: "Details")
            let drawer = try await ui.wait(id: "strip-drawer", timeout: 15, "the scene panel")
            let content = window.contentLayoutRect.offsetBy(dx: window.frame.minX, dy: window.frame.minY)
            try ui.expect(drawer.frame.height > 200, "the panel is \(drawer.frame.height) points tall")
            try ui.expect(drawer.frame.minY >= content.minY - 1 && drawer.frame.maxY <= content.maxY + 1,
                          "the panel (\(drawer.frame)) is outside the window's content (\(content))")
            try ui.expect(ui.exists(id: "strip-drawer-heading"), "the panel shows the scene heading")
            try await ui.press(id: "drawer-close")
            window.setFrame(before, display: true)
        },

        Scenario("62-call-sheets-exports/sides-for-selected-cast-leave-out-scenes-without-them", timeout: 500) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await ui.go("Sides")
            _ = try await ui.wait(id: "sides-tab", timeout: 15, "the Sides tab")
            try await segment(ui, group: "sides-for", "Selected cast")
            let cast = try await ui.find("a cast member") { ui.all().first { $0.identifier.hasPrefix("sides-cast-") } }
            try ui.expect(cast.text.contains("scene"), "the cast name shows its scene count: \(cast.text)")
            try await ui.press(cast)
            _ = try await ui.find("the line under the scenes") { ui.query(id: "sides-filter-status").first }
            let line = ui.query(id: "sides-filter-status").first?.text ?? ""
            try ui.expect(line.contains("have the selected cast") || line.contains("No scene on this day"), "the line says how many scenes are in: \(line)")
        },

        Scenario("62-call-sheets-exports/daily-view-exports-the-day-as-its-own-document", timeout: 500) { ui, _, ctx in
            try await planLostProperty(ui, ctx)
            try await ui.go("Daily View")
            try await ui.press(id: "daily-export")
            try await ui.waitText("Export Daily View")
            try await ui.waitText("Shoot Day 1 - Daily View.pdf")
            try ui.expect(!ui.all().contains { $0.text == "CSV" || $0.text.contains("XLSX") }, "the Daily View offers CSV or XLSX")
            try await ui.press(id: "export-close")
        },

        Scenario("home/cover-photo-menu-and-found-on-this-mac", timeout: 300) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Kept Project")
            try await ui.closeProject()
            try await ui.createProject("Forgotten Project")
            try await ui.closeProject()
            try await ui.waitHome()
            // The card menu offers a cover photo; Remove shows only once one is set.
            let menu = try await ui.wait(id: "project-card-actions-Kept Project", timeout: 15, "the card menu")
            guard let cell = menu.obj as? NSPopUpButtonCell, let items = cell.menu?.items.map(\.title) else { throw DriverError("not a menu button") }
            try ui.expect(items.contains("Change Cover Photo\u{2026}"), "the card menu has Change Cover Photo: \(items)")
            try ui.expect(!items.contains("Remove Cover Photo"), "Remove Cover Photo shows with no cover set")
            // Remove from Recent: the project shows under Found on this Mac, and Open adds it back.
            try await ui.chooseMenuItem(id: "project-card-actions-Forgotten Project", item: "Remove from Recent")
            try await ui.waitText("Found on this Mac")
            try await ui.press(id: "home-found-open-Forgotten Project")
            _ = try await ui.wait(id: "project-home", timeout: 30, "the found project")
            try await ui.closeProject()
            try await ui.waitHome()
            try ui.expect(ui.exists(id: "project-card-Forgotten Project"), "the found project is back in the list")
            try ui.expect(!ui.exists(id: "home-found"), "the project is still listed as found")
        },
    ]
}

private extension Optional {
    func orThrow(_ message: String) throws -> Wrapped {
        guard let v = self else { throw DriverError(message) }
        return v
    }
}
