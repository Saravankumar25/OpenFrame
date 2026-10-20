// Capture tour section: Application Home, first run, app settings and About. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

extension Scenarios {
    static let captureStart: [Scenario] = [
        tour("00-start", order: 0, project: false) { ui, ctx in
            await ui.capture("home-empty", title: "Application Home, no projects yet", area: "Home")
            await ui.capMainMenu("OpenFrame Studio", name: "menu-app", from: "home-empty")
            await ui.capMainMenu("File", name: "menu-file-home", from: "home-empty")
            await ui.capMainMenu("View", name: "menu-view-home", from: "home-empty")
            await ui.step("new project sheet") {
                try await ui.menu(["File", "New Project\u{2026}"])
                _ = try await ui.wait(id: "new-project-title", timeout: 8)
                await ui.capture("new-project-sheet", title: "New Project", area: "Home", from: "home-empty", via: "home-new-project")
                await ui.capture("new-project-empty-title", title: "New Project, Create stays off until there is a title", area: "Home", from: "new-project-sheet", via: "new-project-title")
                try await ui.fill(nil, "a/b:c", id: "new-project-title")
                await ui.capture("new-project-error", title: "New Project, a title the app cannot use", area: "Home", from: "new-project-sheet", via: "new-project-title")
                try await ui.fill(nil, "Night Bus", id: "new-project-title")
                await ui.capture("new-project-filled", title: "New Project, filled in", area: "Home", from: "new-project-empty-title", via: "new-project-title")
                await ui.capMenu(id: "new-project-type", name: "new-project-type-menu", title: "New Project, project type", area: "Home", from: "new-project-filled")
                await ui.capRecover()
            }
            await ui.step("search palette home") {
                try await ui.shortcut("k", code: 40)
                _ = try await ui.wait(id: "palette-field", timeout: 6)
                await ui.capture("search-palette-home", title: "Search and Commands", area: "Search", from: "home-empty", via: "shortcut-k")
                try await ui.capTypeIn("zzzz nothing", into: "palette-field")
                await ui.capture("search-palette-empty", title: "Search and Commands, nothing found", area: "Search", from: "search-palette-home", via: "palette-field")
                await ui.capRecover()
                try? await ui.escape()
            }
            await ui.step("settings") {
                var last: String? = nil
                try await ui.menu([ui.appMenu, "Settings\u{2026}"])
                await ui.idle(1.0)
                for (key, label) in [("general", "General"), ("appearance", "Appearance"), ("offline-ai", "Offline AI"), ("diagnostics", "Diagnostics")] {
                    if label != "General" { try await ui.capAct(label: label) ; await ui.idle(0.8) }
                    for w in NSApp.windows { ui.log("win \(type(of: w)) vis=\(w.isVisible) title=\(w.title) frame=\(w.frame) key=\(w.isKeyWindow) main=\(w.canBecomeMain)") }
                    let n = "settings-\(key)"
                    await ui.capture(n, title: "Settings: \(label)", area: "Settings", from: last ?? "menu-app", via: label)
                    last = n
                }
                await ui.capRecover()
            }
            await ui.step("about") {
                try await ui.menu([ui.appMenu, "About OpenFrame Studio"])
                _ = try await ui.wait(id: "about-version", timeout: 10)
                await ui.capture("about", title: "About OpenFrame Studio", area: "Settings", from: "menu-app", via: "menu-about")
                await ui.capRecover()
            }
        },
    ]
}
