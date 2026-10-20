// Capture tour section: shell and Project Home. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureHome: [Scenario] = [
        tour("01-home", order: 1000, importSample: true) { ui, ctx in
            await ui.capture("project-home-new", title: "Project Home of Night Bus", area: "Project Home")
            try await ui.capNav("Project Home")
            await ui.capture("project-home", title: "Project Home", area: "Project Home", from: "project-home-new", via: "sidebar-home")
            await ui.projectMenuPeek(name: "project-menu", area: "Project Home", from: "project-home")
            await ui.capMainMenu("File", name: "menu-file-project", from: "project-home")
            await ui.capMainMenu("Edit", name: "menu-edit-project", from: "project-home")
            await ui.capMainMenu("Go", name: "menu-go", from: "project-home")
            await ui.step("rename") {
                try await ui.choose("project-menu", "Rename")
                _ = try await ui.wait(id: "rename-title", timeout: 6)
                await ui.capture("project-rename-sheet", title: "Rename Project", area: "Project Home", from: "project-menu", via: "project-menu")
                await ui.capRecover()
            }
            await ui.step("ai panel closed open") {
                try await ui.shortcut("j", code: 38)
                await ui.idle(1.5)
                await ui.capture("project-home-ai-panel", title: "Project Home with the AI Assistant open", area: "AI Assistant", from: "project-home", via: "ai-toggle")
                try await ui.capAct(id: "ai-toggle")
            }
            await ui.step("search palette project") {
                try await ui.shortcut("k", code: 40)
                _ = try await ui.wait(id: "palette-field", timeout: 6)
                await ui.capture("search-palette-project", title: "Search and Commands inside a project", area: "Search", from: "project-home", via: "shortcut-k")
                try await ui.capTypeIn("bus", into: "palette-field")
                await ui.idle(0.8)
                await ui.capture("search-palette-results", title: "Search results", area: "Search", from: "search-palette-project", via: "palette-field")
                await ui.capRecover()
                try? await ui.escape()
            }
            await ui.step("all projects") {
                try await ui.capAct(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-filled", title: "Application Home with a project", area: "Home", from: "project-home", via: "close-project")
                if let card = ui.query(id: "project-card-Night Bus").first { await ui.capContext(on: card, name: "home-card-menu", title: "Project card menu", area: "Home", from: "home-filled") }
                if ui.exists(id: "home-global-vault") { await ui.visit(press: "home-global-vault", name: "home-global-vault", title: "Global Idea Vault", area: "Home", from: "home-filled", close: nil) }
            }
            await ui.step("second project") {
                try await ui.createProject("Harbour Lights")
                try await ui.capAct(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-two-projects", title: "Application Home with two projects", area: "Home", from: "home-filled")
            }
        },
    ]
}
