// Capture tour section: AI Assistant, packages and licence screens. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureAI: [Scenario] = [
        tour("13-ai", order: 13000) { ui, _ in
            try await ui.capNav("Project Home")
            await ui.step("ai panel") {
                try await ui.shortcut("j", code: 38)
                await ui.idle(1.5)
                if !ui.exists(id: "ai-panel") && !ui.exists(id: "ai-input") { try await ui.capAct(id: "ai-toggle"); await ui.idle(1.5) }
                await ui.capture("ai-panel", title: "AI Assistant panel", area: "AI Assistant", from: "project-home", via: "ai-toggle")
                await ui.capMenu(id: "ai-settings-menu", name: "ai-settings-menu", title: "AI Assistant: settings menu", area: "AI Assistant", from: "ai-panel")
                await ui.exploreButtons([("ai-new-conversation", "New conversation"), ("ai-mode-chip", "Mode"), ("ai-open-settings", "Open settings")], area: "AI Assistant", prefix: "ai", from: "ai-panel")
                try await ui.capTypeIn("What is on the Beat Board?", into: "ai-input")
                await ui.capture("ai-panel-question-typed", title: "AI Assistant, a question typed", area: "AI Assistant", from: "ai-panel", via: "ai-input")
            }
            for w in ["Story", "Breakdown", "Visual Planning"] {
                await ui.step("ai scope \(w)") {
                    try await ui.capNav(w)
                    await ui.idle(0.8)
                    await ui.capture("ai-panel-\(ui.slug(w))", title: "AI Assistant open in \(w)", area: "AI Assistant", from: "ai-panel", via: "sidebar")
                }
            }
        },
    ]

    static let capturePackages: [Scenario] = [
        tour("11-packages", order: 11000) { ui, _ in
            await ui.capMainMenu("File", name: "menu-file-with-project", from: "screenplay-editor")
            var items: [(String, [String])] = [("Back Up Project\u{2026}", ["File", "Back Up Project\u{2026}"]), ("Export Full Project\u{2026}", ["File", "Export Full Project\u{2026}"]),
                                              ("Import Exchange Package\u{2026}", ["File", "Import Exchange Package\u{2026}"]), ("Review Queue\u{2026}", ["File", "Review Queue\u{2026}"]),
                                              ("Import & Export History\u{2026}", ["File", "Import & Export History\u{2026}"]), ("Print\u{2026}", ["File", "Print\u{2026}"])]
            if let file = NSApp.mainMenu?.items.first(where: { $0.title == "File" })?.submenu, let share = file.items.first(where: { $0.title == "Share for Review" })?.submenu {
                for i in share.items.filter({ !$0.isSeparatorItem }).prefix(2) { items.append(("Share: \(i.title)", ["File", "Share for Review", i.title])) }
            }
            for (title, path) in items {
                await ui.step("file menu \(title)") {
                    try await ui.menu(path)
                    await ui.idle(1.0)
                    await ui.capture("package-\(ui.slug(title))", title: "File: \(title)", area: "Packages", from: "menu-file-with-project", via: "menu-file")
                    if title.hasPrefix("Import & Export History") {
                        await ui.eachTab(group: "history-tabs", area: "Packages", prefix: "package-history", from: "package-import-export-history")
                    }
                    await ui.capRecover()
                }
            }
        },
    ]
}
