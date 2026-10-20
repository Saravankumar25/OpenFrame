// Capture tour section: the project window shell, Project Home, and Application Home with projects.
// Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

fileprivate extension UI {
    /// Chooses an item of a project card's menu: the Actions button, else the card's right-click menu.
    func homeCardAction(_ project: String, _ item: String) async throws {
        if (try? await choose("project-card-actions-\(project)", item)) != nil { await idle(0.6); return }
        guard let card = query(id: "project-card-\(project)").first else { throw DriverError("no card for \(project)") }
        let f = card.frame
        guard let menu = capContextMenuItems(at: NSPoint(x: f.midX, y: f.midY)),
              let i = menu.items.firstIndex(where: { $0.title.hasPrefix(item) }) else { throw DriverError("no item \(item) on the card of \(project)") }
        menu.performActionForItem(at: i)
        await idle(0.6)
    }

    /// Types into the project search on Application Home.
    func homeSearch(_ text: String) async throws {
        guard let field = all().compactMap({ ($0.obj as? NSSearchField) ?? (($0.obj as? NSCell)?.controlView as? NSSearchField) }).first(where: { $0.window?.isVisible == true }), let w = field.window else {
            throw DriverError("no project search field")
        }
        w.makeKeyAndOrderFront(nil)
        w.makeFirstResponder(field)
        await idle(0.2)
        guard let editor = field.currentEditor() as? NSTextView else { throw DriverError("the project search took no focus") }
        editor.selectAll(nil)
        editor.insertText(text, replacementRange: editor.selectedRange())
        await idle(0.8)
    }

    /// Takes away the toast at the bottom of the window, when one shows.
    func homeDismissToast() async {
        if let d = query(label: "Dismiss").first(where: { $0.role == "AXButton" }) { _ = d.performPress(); await idle(0.4) }
    }
}

extension Scenarios {
    static let captureHome: [Scenario] = [
        tour("01-home", order: 1000, importSample: true) { ui, ctx in
            await ui.capture("home-import-toast", title: "Example screenplay imported, with its Undo toast", area: "Project Home")
            await ui.homeDismissToast()
            try await ui.capNav("Project Home")
            await ui.homeDismissToast()
            await ui.capture("project-home", title: "Project Home of Night Bus", area: "Project Home", from: "home-import-toast", via: "sidebar-home")
            await ui.projectMenuPeek(name: "project-menu", area: "Project Home", from: "project-home")
            await ui.capMenu(id: "project-new-menu", name: "project-new-menu", title: "New menu in the toolbar", area: "Project Home", from: "project-home")
            await ui.capMenu(id: "project-home-status", name: "project-home-status-menu", title: "Project Home, status menu", area: "Project Home", from: "project-home")
            for (menu, name) in [("OpenFrame Studio", "menu-app-project"), ("File", "menu-file-project"), ("Edit", "menu-edit-project"), ("View", "menu-view-project"),
                                 ("Go", "menu-go"), ("Window", "menu-window-project"), ("Help", "menu-help-project")] {
                await ui.capMainMenu(menu, name: name, from: "project-home")
            }
            await ui.step("rename") {
                try await ui.choose("project-menu", "Rename")
                _ = try await ui.wait(id: "rename-title", timeout: 6)
                await ui.capture("project-rename-sheet", title: "Rename Project", area: "Project Home", from: "project-menu", via: "project-menu")
                try await ui.capTypeIn("", into: "rename-title")
                await ui.capture("project-rename-empty", title: "Rename Project, a title is required", area: "Project Home", from: "project-rename-sheet", via: "rename-title")
                await ui.capRecover()
            }
            await ui.step("sidebar hidden") {
                try await ui.menu(["View", "Hide Sidebar"])
                await ui.idle(0.8)
                await ui.capture("project-home-sidebar-hidden", title: "Project Home with the sidebar hidden", area: "Project Home", from: "project-home", via: "sidebar-toggle")
                try await ui.menu(["View", "Show Sidebar"])
                await ui.idle(0.8)
            }
            await ui.step("back and forward") {
                try await ui.capNav("Project Home")
                try await ui.capNav("Story")
                await ui.idle(0.6)
                try await ui.menu(["Go", "Back"])
                _ = try await ui.wait(id: "project-home", timeout: 8)
                await ui.idle(0.6)
                await ui.capture("project-home-after-back", title: "Project Home after Back, Forward available", area: "Project Home", from: "project-home", via: "nav-back")
            }
            await ui.step("ai panel open") {
                try await ui.capNav("Project Home")
                try await ui.shortcut("j", code: 38)
                await ui.idle(1.5)
                await ui.capture("project-home-ai-panel", title: "Project Home with the AI Assistant open", area: "AI Assistant", from: "project-home", via: "ai-toggle")
                try await ui.shortcut("j", code: 38)
                await ui.idle(0.8)
            }
            await ui.step("search palette project") {
                try await ui.capAct(id: "project-search-button")
                _ = try await ui.wait(id: "palette-field", timeout: 6)
                await ui.capture("search-palette-project", title: "Search and Commands inside a project", area: "Search", from: "project-home", via: "project-search-button")
                try await ui.capTypeIn("kiran", into: "palette-field")
                await ui.idle(1.0)
                await ui.capture("search-palette-results", title: "Search results inside a project", area: "Search", from: "search-palette-project", via: "palette-field")
                try? await ui.escape()
                await ui.capRecover()
            }
            if ui.exists(id: "palette-field") { try? await ui.escape(); await ui.idle(0.4) }
            await ui.step("shortcuts sheet project") {
                try await ui.menu(["Help", "Keyboard Shortcuts\u{2026}"])
                _ = try await ui.wait(label: "Filter commands", timeout: 8)
                await ui.capture("project-shortcuts-sheet", title: "Keyboard Shortcuts over a project", area: "Settings", from: "menu-help-project", via: "menu-help")
                await ui.capRecover()
            }

            // Application Home with one project.
            await ui.step("all projects") {
                try await ui.capAct(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-filled", title: "Application Home with a project", area: "Home", from: "project-home", via: "close-project")
                if let card = ui.query(id: "project-card-Night Bus").first { await ui.capContext(on: card, name: "home-card-menu", title: "Project card, right-click menu", area: "Home", from: "home-filled") }
                await ui.capMenu(id: "project-card-actions-Night Bus", name: "home-card-actions-menu", title: "Project card, Actions menu", area: "Home", from: "home-filled")
                if ui.exists(id: "home-global-vault") { await ui.visit(press: "home-global-vault", name: "home-global-vault", title: "Global Idea Vault", area: "Home", from: "home-filled", close: nil) }
            }
            await ui.step("home rename sheet") {
                try await ui.homeCardAction("Night Bus", "Rename")
                _ = try await ui.wait(id: "rename-title", timeout: 6)
                await ui.capture("home-rename-sheet", title: "Rename Project from Application Home", area: "Home", from: "home-card-actions-menu", via: "project-card-actions-Night Bus")
                await ui.capRecover()
            }
            await ui.step("home delete sheet") {
                try await ui.homeCardAction("Night Bus", "Delete Project")
                _ = try await ui.wait(id: "delete-confirm-field", timeout: 6)
                await ui.capture("home-delete-sheet", title: "Delete Project, asks for the name", area: "Home", from: "home-card-actions-menu", via: "project-card-actions-Night Bus")
                try await ui.capTypeIn("Night Bus", into: "delete-confirm-field")
                await ui.capture("home-delete-typed", title: "Delete Project, name typed, Delete Project ready", area: "Home", from: "home-delete-sheet", via: "delete-confirm-field")
                await ui.capRecover()
            }

            // An empty project next to Night Bus.
            await ui.step("empty project") {
                try await ui.createProject("Empty Short")
                await ui.homeDismissToast()
                await ui.capture("project-home-empty", title: "Project Home of a new, empty project", area: "Project Home", from: "new-project-filled", via: "new-project-create")
                try await ui.capAct(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-two-projects", title: "Application Home with two projects", area: "Home", from: "project-home-empty", via: "close-project")
            }
            await ui.step("home search") {
                try await ui.homeSearch("night")
                await ui.capture("home-search-results", title: "Application Home, project search", area: "Home", from: "home-two-projects", via: "Search projects")
                try await ui.homeSearch("zzzz")
                await ui.capture("home-search-none", title: "Application Home, no project matches", area: "Home", from: "home-two-projects", via: "Search projects")
                try await ui.homeSearch("")
            }
            await ui.step("home pinned") {
                try await ui.homeCardAction("Night Bus", "Pin")
                await ui.idle(0.8)
                await ui.capture("home-pinned", title: "Application Home, a pinned project", area: "Home", from: "home-card-actions-menu", via: "project-card-actions-Night Bus")
            }
            await ui.step("home archived") {
                try await ui.homeCardAction("Empty Short", "Archive")
                await ui.idle(0.8)
                await ui.capture("home-one-archived", title: "Application Home, one project archived", area: "Home", from: "home-pinned", via: "project-card-actions-Empty Short")
                try await ui.capAct(id: "home-open-archived")
                await ui.idle(0.8)
                await ui.capture("home-archived-page", title: "Archived projects", area: "Home", from: "home-one-archived", via: "home-open-archived")
                try await ui.capAct(label: "Restore Empty Short")
                await ui.idle(0.8)
                await ui.capture("home-archived-empty", title: "Archived projects, none left", area: "Home", from: "home-archived-page", via: "Restore")
                try await ui.capAct(id: "home-back-to-projects")
                await ui.idle(0.6)
            }
        },
    ]
}
