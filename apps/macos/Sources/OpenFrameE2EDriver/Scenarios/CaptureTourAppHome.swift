// Capture tour section: first run, Application Home with no projects, the New Project sheet, the menu bar,
// Search and Commands, About and every page of the Settings window. Primitives and helpers are in CaptureTour.swift.

import AppKit
import Foundation

/// Titles the Settings window takes (it is named after the page it shows).
private let homeSettingsTitles: Set<String> = ["Settings", "General", "Appearance", "Keyboard Shortcuts", "Offline AI", "Diagnostics"]

fileprivate extension UI {
    /// The Settings window when it is open.
    func homeSettingsWindow() -> NSWindow? {
        NSApp.windows.first { $0.isVisible && $0.sheetParent == nil && homeSettingsTitles.contains($0.title) }
    }

    /// Closes the Settings window (capRecover does not know the Keyboard Shortcuts title) and gives the main window the keyboard.
    func homeCloseSettings() async {
        await capRecover()
        for w in NSApp.windows where w.isVisible && w.sheetParent == nil && homeSettingsTitles.contains(w.title) { w.close() }
        await idle(0.4)
        if let main = NSApp.windows.filter({ $0.isVisible && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) {
            NSApp.activate(ignoringOtherApps: true)
            main.makeKeyAndOrderFront(nil)
        }
        await idle(0.3)
    }

    /// Scrolls the tallest scroll view of a window to a fraction of its height (0 top, 1 bottom).
    func homeScroll(_ w: NSWindow, to fraction: CGFloat) async throws {
        var views: [NSScrollView] = []
        func walk(_ v: NSView) { if let s = v as? NSScrollView { views.append(s) }; for c in v.subviews { walk(c) } }
        if let content = w.contentView { walk(content) }
        guard let sv = views.filter({ ($0.documentView?.frame.height ?? 0) > $0.contentView.bounds.height + 4 })
            .max(by: { ($0.documentView?.frame.height ?? 0) < ($1.documentView?.frame.height ?? 0) }),
              let doc = sv.documentView else { throw DriverError("nothing to scroll in \(w.title)") }
        let room = doc.frame.height - sv.contentView.bounds.height
        let y = doc.isFlipped ? room * fraction : room * (1 - fraction)
        sv.contentView.scroll(to: NSPoint(x: 0, y: y))
        sv.reflectScrolledClipView(sv.contentView)
        await idle(0.6)
    }

    /// Opens the Settings window on a page, by the toolbar of the window.
    func homeSettingsPage(_ label: String) async throws {
        if homeSettingsWindow() == nil {
            try await menu([appMenu, "Settings\u{2026}"])
            try await capWaitUntil("the Settings window", timeout: 8) { homeSettingsWindow() != nil }
            await idle(0.8)
        }
        homeSettingsWindow()?.makeKeyAndOrderFront(nil)
        try await capAct(label: label)
        await idle(0.9)
    }

    /// Opens Search and Commands from the Edit menu, else with Command+K.
    func homeOpenPalette() async throws {
        if menuItemEnabled(["Edit", "Search and Commands\u{2026}"]) {
            try await menu(["Edit", "Search and Commands\u{2026}"])
        } else {
            try await shortcut("k", code: 40)
        }
        _ = try await wait(id: "palette-field", timeout: 6)
        await idle(0.4)
    }
}

extension Scenarios {
    static let captureStart: [Scenario] = [
        Scenario("zz-capture/00-start", timeout: 600) { ui, _, ctx in
            ui.captureBegin(order: 0)
            // First run: the welcome page asks for a name before Application Home.
            let deadline = Date().addingTimeInterval(30)
            while Date() < deadline, !ui.exists(id: "welcome-name"), !ui.exists(id: "home-empty") { await ui.idle(0.2) }
            ui.capBigWindow()
            if ui.exists(id: "welcome-name") {
                await ui.capSetMode()
                ui.capBigWindow()
                // The field starts with the Mac account name: empty it, so the page shows its required-name message.
                try? await ui.capTypeIn("", into: "welcome-name")
                await ui.capture("home-welcome", title: "Welcome, the name is still empty", area: "Home", via: "first-launch")
                await ui.step("welcome filled") {
                    try await ui.capTypeIn("E2E Tester", into: "welcome-name")
                    await ui.capture("home-welcome-filled", title: "Welcome, a name typed", area: "Home", from: "home-welcome", via: "welcome-name")
                }
            }
            try await ui.startApp(ctx)
            ui.capBigWindow()
            await ui.capSetMode()
            ui.capBigWindow()

            await ui.capture("home-empty", title: "Application Home, no projects yet", area: "Home", from: "home-welcome-filled", via: "welcome-continue")
            for (menu, name) in [("OpenFrame Studio", "menu-app"), ("File", "menu-file-home"), ("Edit", "menu-edit-home"), ("View", "menu-view-home"),
                                 ("Go", "menu-go-home"), ("Window", "menu-window-home"), ("Help", "menu-help-home")] {
                await ui.capMainMenu(menu, name: name, from: "home-empty")
            }

            await ui.step("new project sheet") {
                try await ui.capAct(id: "home-new-project")
                _ = try await ui.wait(id: "new-project-title", timeout: 8)
                await ui.capture("new-project-sheet", title: "New Project, Create stays off until there is a title", area: "Home", from: "home-empty", via: "home-new-project")
                try await ui.fill(nil, "Night Bus", id: "new-project-title")
                await ui.capture("new-project-filled", title: "New Project, a title typed", area: "Home", from: "new-project-sheet", via: "new-project-title")
                await ui.capMenu(id: "new-project-type", name: "new-project-type-menu", title: "New Project, project type", area: "Home", from: "new-project-filled")
                try await ui.choose("new-project-type", "Short Film")
                await ui.capture("new-project-short-film", title: "New Project, Short Film chosen", area: "Home", from: "new-project-type-menu", via: "new-project-type")
                await ui.capRecover()
            }

            await ui.step("search palette home") {
                try await ui.homeOpenPalette()
                await ui.capture("search-palette-home", title: "Search and Commands", area: "Search", from: "home-empty", via: "menu-edit-search")
                try await ui.capTypeIn("settings", into: "palette-field")
                await ui.idle(0.6)
                await ui.capture("search-palette-home-commands", title: "Search and Commands, matching commands", area: "Search", from: "search-palette-home", via: "palette-field")
                try await ui.capTypeIn("zzzz nothing", into: "palette-field")
                await ui.idle(0.6)
                await ui.capture("search-palette-empty", title: "Search and Commands, nothing found", area: "Search", from: "search-palette-home", via: "palette-field")
                try? await ui.escape()
                await ui.capRecover()
            }
            if ui.exists(id: "palette-field") { try? await ui.escape(); await ui.idle(0.4) }

            await ui.step("shortcuts sheet home") {
                try await ui.menu(["Help", "Keyboard Shortcuts\u{2026}"])
                _ = try await ui.wait(label: "Filter commands", timeout: 8)
                await ui.capture("home-shortcuts-sheet", title: "Keyboard Shortcuts from the Help menu", area: "Settings", from: "menu-help-home", via: "menu-help")
                await ui.capRecover()
            }

            await ui.step("about") {
                try await ui.menu([ui.appMenu, "About OpenFrame Studio"])
                _ = try await ui.wait(id: "about-version", timeout: 10)
                await ui.capture("about", title: "About OpenFrame Studio", area: "Settings", from: "menu-app", via: "menu-about")
                await ui.capRecover()
            }

            // The Settings window: General, Appearance, Keyboard Shortcuts, Offline AI, Diagnostics.
            await ui.step("settings general") {
                try await ui.capAct(id: "home-settings")
                try await ui.capWaitUntil("the Settings window", timeout: 8) { ui.homeSettingsWindow() != nil }
                try await ui.homeSettingsPage("General")
                await ui.capture("settings-general", title: "Settings: General", area: "Settings", from: "home-empty", via: "home-settings")
                try await ui.capTypeIn("Asha Rao", into: "settings-display-name")
                await ui.capture("settings-general-name-edited", title: "Settings: General, a new name ready to save", area: "Settings", from: "settings-general", via: "settings-display-name")
                try await ui.capTypeIn("E2E Tester", into: "settings-display-name")
            }
            await ui.step("settings appearance") {
                try await ui.homeSettingsPage("Appearance")
                await ui.capture("settings-appearance", title: "Settings: Appearance", area: "Settings", from: "settings-general", via: "Appearance")
                if let w = ui.homeSettingsWindow() {
                    try await ui.homeScroll(w, to: 1)
                    await ui.capture("settings-appearance-scrolled", title: "Settings: Appearance, My Themes at the bottom", area: "Settings", from: "settings-appearance", via: "scroll")
                    try await ui.homeScroll(w, to: 0)
                }
            }
            await ui.step("settings theme chosen") {
                let cards = ui.capQueryPrefix("theme-card-").filter { $0.frame.width > 0 }
                guard cards.count > 1 else { throw DriverError("fewer than two theme cards") }
                let selected = cards.first { $0.isSelected } ?? cards[0]
                let other = cards.first { $0.identifier != selected.identifier }!
                try await ui.capAct(id: other.identifier)
                await ui.idle(1.0)
                await ui.capture("settings-appearance-theme-chosen", title: "Settings: Appearance, another theme chosen", area: "Settings", from: "settings-appearance", via: other.identifier)
                try await ui.capAct(id: selected.identifier)
                await ui.idle(1.0)
            }
            await ui.step("settings paste theme") {
                if let w = ui.homeSettingsWindow() { try? await ui.homeScroll(w, to: 1) }
                try await ui.capAct(id: "settings-paste-theme")
                _ = try await ui.wait(id: "theme-import-text", timeout: 8)
                await ui.capture("settings-theme-paste-sheet", title: "Paste Theme, empty", area: "Settings", from: "settings-appearance-scrolled", via: "settings-paste-theme")
                try await ui.capTypeIn("{ \"name\": \"Not a theme\" }", into: "theme-import-text")
                try await ui.capAct(label: "Check Theme")
                await ui.idle(1.2)
                await ui.capture("settings-theme-paste-error", title: "Paste Theme, text that is not a theme", area: "Settings", from: "settings-theme-paste-sheet", via: "theme-import-check")
                await ui.capRecover()
                if let w = ui.homeSettingsWindow() { try? await ui.homeScroll(w, to: 0) }
            }
            await ui.step("settings shortcuts") {
                try await ui.homeSettingsPage("Keyboard Shortcuts")
                _ = try await ui.wait(id: "shortcuts-filter", timeout: 8)
                await ui.capture("settings-keyboard-shortcuts", title: "Settings: Keyboard Shortcuts", area: "Settings", from: "settings-appearance", via: "Keyboard Shortcuts")
                if let w = ui.homeSettingsWindow() {
                    try await ui.homeScroll(w, to: 1)
                    await ui.capture("settings-keyboard-shortcuts-scrolled", title: "Settings: Keyboard Shortcuts, the end of the list", area: "Settings", from: "settings-keyboard-shortcuts", via: "scroll")
                    try await ui.homeScroll(w, to: 0)
                }
                try await ui.capTypeIn("project", into: "shortcuts-filter")
                await ui.idle(0.6)
                await ui.capture("settings-shortcuts-filtered", title: "Settings: Keyboard Shortcuts, filtered", area: "Settings", from: "settings-keyboard-shortcuts", via: "shortcuts-filter")
                try await ui.capTypeIn("zzzz", into: "shortcuts-filter")
                await ui.idle(0.6)
                await ui.capture("settings-shortcuts-none", title: "Settings: Keyboard Shortcuts, no command matches", area: "Settings", from: "settings-keyboard-shortcuts", via: "shortcuts-filter")
                try await ui.capTypeIn("project", into: "shortcuts-filter")
                await ui.idle(0.6)
            }
            await ui.step("settings shortcut recording") {
                guard let add = ui.capQueryPrefix("shortcut-add-").first(where: { $0.frame.width > 0 && $0.isEnabled }) else { throw DriverError("no Add button") }
                try await ui.capAct(id: add.identifier)
                await ui.idle(0.6)
                await ui.capture("settings-shortcut-recording", title: "Settings: Keyboard Shortcuts, waiting for the new keys", area: "Settings", from: "settings-shortcuts-filtered", via: add.identifier)
                if let w = ui.homeSettingsWindow() { await ui.escapeTo(w) }
            }
            await ui.step("settings offline ai") {
                try await ui.homeSettingsPage("Offline AI")
                await ui.capture("settings-offline-ai", title: "Settings: Offline AI", area: "Settings", from: "settings-keyboard-shortcuts", via: "Offline AI")
            }
            await ui.step("settings diagnostics") {
                try await ui.homeSettingsPage("Diagnostics")
                await ui.capture("settings-diagnostics", title: "Settings: Diagnostics", area: "Settings", from: "settings-offline-ai", via: "Diagnostics")
            }
            await ui.homeCloseSettings()

            await ui.capRecover()
        },
    ]
}
