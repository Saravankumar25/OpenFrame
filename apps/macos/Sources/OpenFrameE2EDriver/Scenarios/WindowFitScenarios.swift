// Window fit: the whole app stays inside the window and the window stays inside the screen, at the
// smallest window, the first-launch size, a maximised (zoomed) window and a laptop-sized window.
// Every workspace is visited with the sidebar shown and hidden and the AI Assistant open and
// closed. At each step every control a person can use must be inside the window, or inside a
// scroll area that can bring it into view. Each problem is listed with the workspace, the size and
// the control, and the scenario fails with the whole list.

import AppKit

extension Scenarios {
    static let windowFitWorkspaces = [
        "Project Home", "Screenplay", "Story", "Idea Vault", "Breakdown", "Schedule", "Call Sheets", "Budget",
        "Visual Planning", "Files", "Notes and Tasks", "Activity", "Recently Deleted",
    ]

    static let windowFitGroup: [Scenario] = [
        windowFit(.minimum), windowFit(.standard), windowFit(.zoomed), windowFit(.laptop),

        Scenario("95-reachability/window-fits-dialogs", timeout: 420) { ui, _, ctx in
            try await ui.startApp(ctx)
            var report = FitReport()
            await ui.fitLaunchCheck(&report)
            try await ui.fitSize(.minimum, &report)
            ui.fitCheck("Home", &report)
            // New Project sheet on Home.
            try await ui.menu(["File", "New Project…"])
            try await ui.wait(id: "new-project-title", timeout: 15)
            await ui.idle(0.8) // the sheet finishes sliding in
            ui.fitCheck("New Project sheet", &report)
            try? ui.snapshot("new-project-sheet")
            try await ui.fitCloseSheet()
            // Settings window, every page.
            try await ui.fitSettings(&report)
            // About window.
            try await ui.menu([NSApp.mainMenu?.items.first?.title ?? "OpenFrame Studio", "About OpenFrame Studio"])
            try await ui.fitWaitWindow("about", timeout: 10)
            ui.fitCheckWindow("About", id: "about", &report)
            ui.fitCloseWindow("about")
            // A project with the example screenplay, then its sheets and the search palette.
            try await ui.fitSeed()
            try await ui.fitSize(.minimum, &report)
            try await ui.press(id: "project-settings-button")
            try await ui.wait(id: "project-settings-tab-general", timeout: 15)
            await ui.idle(0.8) // the sheet finishes sliding in
            ui.fitCheck("Project Settings sheet", &report)
            try? ui.snapshot("project-settings-sheet")
            try await ui.fitCloseSheet()
            try await ui.menu(["File", "Import Screenplay…"])
            try await ui.wait(id: "import-paste", timeout: 15)
            await ui.idle(0.8) // the sheet finishes sliding in
            ui.fitCheck("Import Screenplay sheet", &report)
            try? ui.snapshot("import-screenplay-sheet")
            try await ui.fitCloseSheet()
            try await ui.menu(["File", "Export Screenplay…"])
            try await ui.wait(id: "export-confirm", timeout: 20)
            await ui.idle(0.8) // the sheet finishes sliding in
            ui.fitCheck("Export Screenplay sheet", &report)
            try? ui.snapshot("export-screenplay-sheet")
            try await ui.fitCloseSheet()
            try await ui.menu(["Help", "Keyboard Shortcuts…"])
            await ui.idle(0.8)
            await ui.idle(0.8) // the sheet finishes sliding in
            ui.fitCheck("Keyboard Shortcuts sheet", &report)
            try? ui.snapshot("keyboard-shortcuts-sheet")
            try await ui.fitCloseSheet()
            try await ui.menu(["Edit", "Search and Commands…"])
            await ui.idle(0.8)
            // The palette floats over the page on purpose; its list may cover the page's lists.
            ui.fitCheck("Search palette", overlaps: false, &report)
            try await ui.escape()
            try report.finish(ui, "dialogs")
        },

        // Resizing must never send the window into a layout loop (AppKit ends the app when one
        // constraint pass keeps asking for another). Every workspace, with the AI Assistant closed
        // and open, goes 20 times between the smallest window and the whole screen, with sizes in
        // between, then must still fit.
        Scenario("95-reachability/window-fits-resize-stress", timeout: 900) { ui, _, ctx in
            try await ui.startApp(ctx)
            var report = FitReport()
            report.size = "resize"
            try await ui.fitSeed()
            guard let w = ui.fitMain else { throw DriverError("no main window") }
            for assistant in [false, true] {
                if ui.exists(id: "ai-panel") != assistant {
                    try await ui.shortcut("j", code: 38)
                    await ui.idle(0.4)
                }
                for ws in windowFitWorkspaces {
                    try await ui.go(ws)
                    await ui.idle(0.4)
                    let vf = ui.fitScreen(w)
                    let small = NSSize(width: max(w.minSize.width, 900), height: max(w.minSize.height, min(652, vf.height)))
                    for i in 0..<20 {
                        // Smallest, whole screen, and two sizes between, in turn.
                        let t: CGFloat = [0, 1, 0.35, 0.7][i % 4]
                        let size = NSSize(width: small.width + (vf.width - small.width) * t, height: small.height + (vf.height - small.height) * t)
                        w.setFrame(NSRect(x: vf.minX, y: vf.maxY - size.height, width: size.width, height: size.height), display: true)
                        await ui.idle(0.12)
                    }
                    await ui.idle(0.6)
                    ui.fitCheck("\(ws) after 20 resizes, assistant \(assistant ? "open" : "closed")", &report)
                }
            }
            try report.finish(ui, "resize-stress")
        },

        Scenario("95-reachability/window-fits-sidebar-relaunch", phases: 2, timeout: 300) { ui, phase, ctx in
            try await ui.startApp(ctx)
            var report = FitReport()
            await ui.fitLaunchCheck(&report)
            if phase == 1 {
                try await ui.fitSeed()
                try await ui.fitSize(.standard, &report)
                // The toolbar button, View > Hide Sidebar and Show Sidebar, and Command+Control+S
                // each hide or show the sidebar; it is hidden when the app quits.
                try await ui.fitSidebar(false, via: .toolbar)
                ui.fitCheck("Screenplay", sidebar: false, &report)
                try await ui.fitSidebar(true, via: .shortcut)
                ui.fitCheck("Screenplay", sidebar: true, &report)
                try await ui.fitSidebar(false, via: .menu)
                try await ui.fitSidebar(true, via: .toolbar)
                try await ui.fitSidebar(false, via: .toolbar)
            } else {
                try await ui.openProject("Night Bus")
                // After a relaunch the window fits the screen and the sidebar is back.
                try await ui.waitUntilFit("the sidebar shows after a relaunch", timeout: 10) { ui.fitSidebarShown() }
                ui.fitCheck("Screenplay after relaunch", sidebar: true, &report)
            }
            try report.finish(ui, "sidebar-relaunch-\(phase)")
        },
    ]

    static func windowFit(_ size: FitSize) -> Scenario {
        Scenario("95-reachability/window-fits-\(size.rawValue)", timeout: 900) { ui, _, ctx in
            try await ui.startApp(ctx)
            var report = FitReport()
            await ui.fitLaunchCheck(&report)
            try await ui.fitSize(size, &report)
            ui.fitCheck("Home", &report)
            try await ui.fitSeed()
            try await ui.fitSize(size, &report)
            for sidebar in [true, false] {
                if ui.fitSidebarShown() != sidebar {
                    try await ui.fitSidebar(sidebar, via: sidebar ? .menu : .toolbar)
                }
                for assistant in [false, true] {
                    if ui.exists(id: "ai-panel") != assistant {
                        try await ui.shortcut("j", code: 38)
                        if assistant { try await ui.wait(id: "ai-panel", timeout: 10) } else { try await ui.waitGone(id: "ai-panel", timeout: 10) }
                        await ui.idle(0.4)
                    }
                    for ws in windowFitWorkspaces {
                        try await ui.go(ws)
                        await ui.idle(0.5)
                        let place = "\(ws), sidebar \(sidebar ? "shown" : "hidden"), assistant \(assistant ? "open" : "closed")"
                        ui.fitCheck(place, sidebar: sidebar, assistant: assistant, &report)
                        if ws == "Project Home" || ws == "Screenplay" || ws == "Schedule" {
                            try? ui.snapshot("\(size.rawValue)-\(ws)-sidebar-\(sidebar)-assistant-\(assistant)")
                        }
                    }
                }
            }
            try report.finish(ui, size.rawValue)
        }
    }
}

/// The four window sizes of the scenario.
enum FitSize: String {
    /// The smallest window the app allows.
    case minimum = "minimum"
    /// The first-launch size (1360 by 860), which the app must fit to a smaller screen.
    case standard = "default"
    /// Zoomed (the green button, Window > Zoom), which fills the screen's visible frame.
    case zoomed = "maximised"
    /// A 1280 by 800 window, as on a 13-inch laptop screen.
    case laptop = "1280x800"
}

/// Every problem found, with where it was found.
struct FitReport {
    var size = ""
    var problems: [String] = []
    var checks = 0

    mutating func add(_ place: String, _ what: String) { problems.append("[\(size)] \(place): \(what)") }

    @MainActor func finish(_ ui: UI, _ name: String) throws {
        let text = problems.isEmpty ? "no problems in \(checks) checks" : problems.joined(separator: "\n")
        try? text.write(to: ui.artifacts.appendingPathComponent("fit-\(name).txt"), atomically: true, encoding: .utf8)
        ui.log("window fit: \(checks) checks, \(problems.count) problems")
        if !problems.isEmpty {
            // Repeated problems (the same control on many steps) are shown once in the message.
            var seen = Set<String>(), unique: [String] = []
            for p in problems where seen.insert(p).inserted { unique.append(p) }
            throw DriverError("\(problems.count) window fit problems (full list in fit-\(name).txt):\n" + unique.prefix(40).joined(separator: "\n"))
        }
    }
}

enum SidebarRoute { case toolbar, menu, shortcut }

private let fitTolerance: CGFloat = 1.5
private let interactiveRoles: Set<String> = [
    "AXButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXPopUpButton", "AXMenuButton", "AXRadioButton", "AXLink",
    "AXRow", "AXComboBox", "AXSlider", "AXSearchField", "AXDisclosureTriangle", "AXIncrementor", "AXTab", "AXColorWell",
    "AXSegmentedControl",
]

private extension NSRect {
    func holds(_ r: NSRect) -> Bool {
        r.minX >= minX - fitTolerance && r.maxX <= maxX + fitTolerance && r.minY >= minY - fitTolerance && r.maxY <= maxY + fitTolerance
    }
    var short: String {
        // An element that is not laid out yet can report a frame of NaN or infinity.
        func n(_ v: CGFloat) -> String { v.isFinite ? String(Int(v.rounded())) : "?" }
        return "(\(n(minX)),\(n(minY)) \(n(width))x\(n(height)))"
    }
    var isFiniteRect: Bool { origin.x.isFinite && origin.y.isFinite && size.width.isFinite && size.height.isFinite }
}

extension UI {
    /// The project and Home window (not Settings, About, sheets or menus).
    var fitMain: NSWindow? {
        let visible = NSApp.windows.filter { $0.isVisible && $0.sheetParent == nil && $0.canBecomeMain }
        return visible.first { $0.identifier?.rawValue.hasPrefix("main") == true }
            ?? visible.filter { $0.identifier?.rawValue != "about" && !($0.identifier?.rawValue.contains("Settings") ?? false) }
                .max { $0.frame.width * $0.frame.height < $1.frame.width * $1.frame.height }
    }

    func fitScreen(_ w: NSWindow) -> NSRect { (w.screen ?? NSScreen.main)?.visibleFrame ?? .zero }

    func waitUntilFit(_ what: String, timeout: Double, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return }
            await idle(0.1)
        }
        throw DriverError("not true within \(Int(timeout)) s: \(what)")
    }

    /// Right after launch, before the scenario changes anything, the window lies on the screen.
    func fitLaunchCheck(_ report: inout FitReport) async {
        guard let w = fitMain else { report.add("launch", "no main window"); return }
        let vf = fitScreen(w)
        // The window settles within a second of showing (macOS restores its last frame).
        let deadline = Date().addingTimeInterval(2)
        while Date() < deadline, !vf.holds(w.frame) { await idle(0.1) }
        log("screen visible frame \(vf.short), window at launch \(w.frame.short), content minimum \(w.contentMinSize), frame minimum \(w.minSize)")
        report.checks += 1
        if !vf.holds(w.frame) { report.add("launch", "two seconds after launch the window \(w.frame.short) is not inside the screen's visible frame \(vf.short)") }
    }

    /// Puts the main window at one of the four sizes, the way a person or macOS would.
    func fitSize(_ size: FitSize, _ report: inout FitReport) async throws {
        guard let w = fitMain else { throw DriverError("no main window") }
        report.size = size.rawValue
        let vf = fitScreen(w)
        if size != .zoomed, w.isZoomed { w.zoom(nil); await idle(0.6) }
        switch size {
        case .minimum:
            // What the window accepts as its smallest frame. It must be the 900 by 600 minimum,
            // or smaller only when the screen itself is smaller.
            let fromContent = w.frameRect(forContentRect: NSRect(origin: .zero, size: w.contentMinSize)).size
            let minFrame = NSSize(width: max(w.minSize.width, fromContent.width), height: max(w.minSize.height, fromContent.height))
            let chrome = w.frame.height - w.contentLayoutRect.height
            let roomy = vf.width >= 900 && vf.height >= 600 + chrome
            report.checks += 1
            if roomy && (w.contentMinSize.width < 899 || w.contentMinSize.height < 599) {
                report.add("minimum", "the content minimum \(w.contentMinSize) is below 900 by 600 on a screen that has room for it")
            }
            if !roomy && (minFrame.width > vf.width + fitTolerance || minFrame.height > vf.height + fitTolerance) {
                report.add("minimum", "the smallest window \(minFrame) does not fit the screen \(vf.short)")
            }
            w.setFrame(NSRect(x: vf.minX, y: vf.maxY - minFrame.height, width: minFrame.width, height: minFrame.height), display: true)
        case .standard:
            w.setFrame(NSRect(x: vf.minX, y: vf.maxY - 860, width: 1360, height: 860), display: true)
        case .laptop:
            w.setFrame(NSRect(x: vf.minX, y: vf.maxY - 800, width: 1280, height: 800), display: true)
        case .zoomed:
            if !w.isZoomed { w.zoom(nil) }
        }
        await idle(1.0)
        log("window \(size.rawValue): \(w.frame.short) on screen \(vf.short), zoomed \(w.isZoomed)")
        report.checks += 1
        if size == .zoomed && (abs(w.frame.width - vf.width) > 2 || abs(w.frame.height - vf.height) > 2) {
            report.add("zoom", "the zoomed window \(w.frame.short) does not fill the screen's visible frame \(vf.short)")
        }
    }

    /// A project called Night Bus with the bundled example screenplay.
    func fitSeed() async throws {
        try await createProject("Night Bus")
        try await menu(["File", "Import Screenplay\u{2026}"])
        try await press(label: "Example Short", role: "AXButton")
        try await press(id: "import-preview")
        try await waitText("Import Screenplay \u{00B7} Preview", timeout: 30)
        try await press(id: "import-confirm")
        try await waitText("Import complete", timeout: 30)
        try await press(label: "Open Screenplay", role: "AXButton")
        _ = try await wait(id: "screenplay-editor", timeout: 30)
        await idle(0.6)
    }

    func fitCloseSheet() async throws {
        guard let w = fitMain else { return }
        for _ in 0..<3 where w.attachedSheet != nil {
            if let sheet = w.attachedSheet {
                sheet.makeKeyAndOrderFront(nil)
                if let cancel = Element(obj: sheet).fitWalk().first(where: { ["Cancel", "Close", "Done"].contains($0.label) || ["Cancel", "Close", "Done"].contains($0.title) }), cancel.role == "AXButton" {
                    try await press(cancel)
                } else {
                    try await escape()
                }
            }
            await idle(0.6)
        }
        try expect(w.attachedSheet == nil, "the sheet closed")
    }

    func fitWaitWindow(_ id: String, timeout: Double) async throws {
        try await waitUntilFit("window \(id)", timeout: timeout) { NSApp.windows.contains { $0.isVisible && $0.identifier?.rawValue.hasPrefix(id) == true } }
        await idle(0.5)
    }

    func fitCloseWindow(_ id: String) {
        NSApp.windows.filter { $0.isVisible && $0.identifier?.rawValue.hasPrefix(id) == true }.forEach { $0.performClose(nil) }
    }

    static let settingsPages = ["General", "Appearance", "Keyboard Shortcuts", "Offline AI", "Diagnostics"]

    func fitSettings(_ report: inout FitReport) async throws {
        guard let appMenu = NSApp.mainMenu?.items.first?.title else { throw DriverError("no app menu") }
        try await menu([appMenu, "Settings\u{2026}"])
        try await waitUntilFit("the Settings window", timeout: 10) { self.fitSettingsWindow != nil }
        await idle(0.6)
        for page in UI.settingsPages {
            guard let tab = query(label: page).first(where: { ["AXRadioButton", "AXButton", "AXTab"].contains($0.role) }) else {
                report.add("Settings", "no \(page) page button"); continue
            }
            try await fitPress(tab)
            await idle(0.8)
            guard let sw = fitSettingsWindow else { report.add("Settings: \(page)", "the Settings window closed"); continue }
            fitCheckWindow("Settings: \(page)", window: sw, &report)
        }
        fitSettingsWindow?.performClose(nil)
        await idle(0.4)
    }

    /// Presses a control; one that refuses the accessibility press gets a mouse click put in the
    /// event queue (a toolbar item tracks the mouse until the button comes up, so a click sent
    /// straight to the window would wait for ever).
    func fitPress(_ e: Element) async throws {
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        if e.performPress() { await idle(); return }
        let f = e.frame
        let center = NSPoint(x: f.midX, y: f.midY)
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }) else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for t in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            if let ev = NSEvent.mouseEvent(with: t, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                           windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: t == .leftMouseDown ? 1 : 0) {
                NSApp.postEvent(ev, atStart: false)
            }
        }
        await idle(0.3)
    }

    var fitSettingsWindow: NSWindow? {
        NSApp.windows.first { $0.isVisible && $0.sheetParent == nil && UI.settingsPages.contains($0.title) }
    }

    func fitCheckWindow(_ place: String, id: String, _ report: inout FitReport) {
        guard let w = NSApp.windows.first(where: { $0.isVisible && $0.identifier?.rawValue.hasPrefix(id) == true }) else {
            report.add(place, "the window did not open"); return
        }
        fitCheckWindow(place, window: w, &report)
    }

    func fitCheckWindow(_ place: String, window w: NSWindow, _ report: inout FitReport) {
        report.checks += 1
        let vf = fitScreen(w)
        if !vf.holds(w.frame) { report.add(place, "the window \(w.frame.short) is not inside the screen's visible frame \(vf.short)") }
        for p in fitProblems(in: w) { report.add(place, p) }
    }

    // MARK: sidebar

    /// The sidebar is shown when its workspace rows are inside the window.
    func fitSidebarShown() -> Bool {
        guard let w = fitMain else { return false }
        let rows = query(id: "sidebar-screenplay")
        return rows.contains { $0.frame.width > 1 && w.frame.intersects($0.frame) && $0.frame.minX >= w.frame.minX - 1 }
    }

    func fitSidebar(_ show: Bool, via route: SidebarRoute) async throws {
        switch route {
        case .toolbar:
            let button = try await wait(id: "sidebar-toggle", role: "AXButton", timeout: 5, "the sidebar button in the toolbar")
            let expected = show ? "Show Sidebar" : "Hide Sidebar"
            try expect(button.label == expected, "the sidebar button is named \(expected): \(button)")
            try await press(button)
        case .menu:
            try await menu(["View", show ? "Show Sidebar" : "Hide Sidebar"])
        case .shortcut:
            // Key presses this driver sends do not reach SwiftUI menu key equivalents (Command+
            // Control+1 for Go > Project Home does not either), so check that the View menu item
            // carries Command+Control+S and choose it.
            let title = show ? "Show Sidebar" : "Hide Sidebar"
            _ = menuItemEnabled(["View", title])
            guard let item = NSApp.mainMenu?.items.first(where: { $0.title == "View" })?.submenu?.items.first(where: { $0.title == title }) else {
                throw DriverError("no View > \(title)")
            }
            try expect(item.keyEquivalent == "s" && item.keyEquivalentModifierMask == [.command, .control],
                       "View > \(title) has the shortcut Command+Control+S: \(item.keyEquivalentModifierMask.rawValue) \(item.keyEquivalent)")
            try await menu(["View", title])
        }
        try await waitUntilFit("the sidebar is \(show ? "shown" : "hidden")", timeout: 5) { self.fitSidebarShown() == show }
        // Let the sidebar finish sliding in or out before measuring it.
        var last = NSRect.zero
        for _ in 0..<20 {
            await idle(0.2)
            let now = query(label: "Sidebar", role: "AXOutline").first { $0.identifier.isEmpty }?.frame ?? .zero
            if now == last { break }
            last = now
        }
    }

    // MARK: the check

    func fitCheck(_ place: String, sidebar: Bool? = nil, assistant: Bool? = nil, overlaps: Bool = true, _ report: inout FitReport) {
        guard let w = fitMain else { report.add(place, "no main window"); return }
        report.checks += 1
        let vf = fitScreen(w)
        if !vf.holds(w.frame) { report.add(place, "the window \(w.frame.short) is not inside the screen's visible frame \(vf.short)") }
        if let sheet = w.attachedSheet {
            log("\(place): sheet frame \(sheet.frame.short), content \(sheet.convertToScreen(sheet.contentLayoutRect).short), content view \(sheet.contentView.map { sheet.convertToScreen($0.convert($0.bounds, to: nil)).short } ?? "-"), window \(w.frame.short)")
            if !vf.holds(sheet.frame) { report.add(place, "the sheet \(sheet.frame.short) is not inside the screen's visible frame \(vf.short)") }
            if sheet.frame.height > w.frame.height + fitTolerance || sheet.frame.width > w.frame.width + fitTolerance {
                report.add(place, "the sheet \(sheet.frame.short) is larger than its window \(w.frame.short)")
            }
            for p in fitProblems(in: sheet) { report.add(place, "in the sheet: \(p)") }
            return
        }
        for p in fitProblems(in: w, skipSidebar: sidebar == false, overlaps: overlaps) { report.add(place, p) }
        if sidebar == true {
            let content = w.convertToScreen(w.contentLayoutRect)
            let sidebarList = query(label: "Sidebar", role: "AXOutline").first { $0.identifier.isEmpty && $0.frame.width > 1 }?.frame
            if let list = sidebarList, list.width < 180 { report.add(place, "the sidebar \(list.short) is only \(Int(list.width)) points wide") }
            for ws in Scenarios.windowFitWorkspaces {
                let key = UI.workspaceKey(ws)
                guard let row = query(id: "sidebar-\(key)").first(where: { $0.frame.width > 1 }) else {
                    report.add(place, "the sidebar row \(ws) is missing"); continue
                }
                if !w.frame.holds(row.frame) { report.add(place, "the sidebar row \(ws) \(row.frame.short) is outside the window \(w.frame.short)") }
                // The row's name must not be cut by the sidebar's edge.
                if let list = sidebarList, row.frame.maxX > list.maxX + fitTolerance || row.frame.minX < list.minX - fitTolerance {
                    report.add(place, "the sidebar row \(ws) \(row.frame.short) is cut by the sidebar edge \(list.short)")
                }
            }
            if let saved = query(id: "save-status").first(where: { $0.frame.width > 1 }) {
                if !content.holds(saved.frame) || !vf.holds(saved.frame) {
                    report.add(place, "the Saved status \(saved.frame.short) is not inside the window content \(content.short)")
                }
            } else {
                report.add(place, "the Saved status at the foot of the sidebar is missing")
            }
        } else if sidebar == false, fitSidebarShown() {
            report.add(place, "the sidebar still shows after it was hidden: \(query(id: "sidebar-screenplay").map(\.frame.short))")
        }
        if assistant == true {
            if let panel = query(id: "ai-panel").first(where: { $0.frame.width > 1 }) {
                if !w.frame.holds(panel.frame) { report.add(place, "the AI Assistant panel \(panel.frame.short) is outside the window \(w.frame.short)") }
                if panel.frame.width < 240 { report.add(place, "the AI Assistant panel is only \(Int(panel.frame.width)) points wide") }
            } else {
                report.add(place, "the AI Assistant panel is missing")
            }
        }
    }

    /// Every control of the window that is outside the window, or outside its scroll area with no
    /// way to scroll it into view.
    func fitProblems(in w: NSWindow, skipSidebar: Bool = false, overlaps: Bool = true) -> [String] {
        var out: [String] = []
        let frame = w.frame
        let content = w.convertToScreen(w.contentLayoutRect)
        var scrollAreasSeen = Set<ObjectIdentifier>()
        var areas: [(String, NSRect)] = []
        var scrollTries = 0

        func describe(_ e: Element) -> String {
            let name = e.identifier.isEmpty ? (e.label.isEmpty ? (e.title.isEmpty ? String(e.value.prefix(40)) : e.title) : e.label) : e.identifier
            return "\(e.role) \"\(name)\""
        }

        func walk(_ e: Element, depth: Int, scroll: (Element, NSRect)?, toolbar: Bool) {
            guard depth < 60 else { return }
            let role = e.role
            var scroll = scroll
            var toolbar = toolbar
            if role == "AXToolbar" { toolbar = true }
            if skipSidebar, e.identifier.hasPrefix("sidebar-") { return }
            let f = e.frame
            guard f.isFiniteRect else {
                for c in e.children { walk(c, depth: depth + 1, scroll: scroll, toolbar: toolbar) }
                return
            }
            if role == "AXScrollArea", f.width > 1, f.height > 1 {
                if scrollAreasSeen.insert(ObjectIdentifier(e.obj)).inserted, !frame.holds(f) {
                    // A scroll area larger than the window: its edge is cut off and cannot be scrolled to.
                    let visible = f.intersection(frame)
                    out.append("scroll area \(describe(e)) \(f.short) reaches outside the window \(frame.short) (\(Int(f.height - visible.height)) points cut off)")
                }
                let visible = f.intersection(frame)
                scroll = (e, visible)
                areas.append((describe(e), f))
            }
            if interactiveRoles.contains(role), f.width > 0.5, f.height > 0.5 {
                if let (area, visible) = scroll {
                    let shown = visible.intersection(toolbar ? frame : content)
                    if !shown.holds(f) && !fitReachable(f, in: area, window: w) {
                        // Ask the scroll area to bring the control into view, as VoiceOver does.
                        scrollTries += 1
                        if scrollTries > 40 {
                            if scrollTries == 41 { log("more than 40 controls outside their scroll areas in \(w.title); the rest are listed without trying to scroll") }
                            out.append("\(describe(e)) \(f.short) is outside its scroll area \(shown.short) (not tried)")
                        } else if fitScrollIntoView(e), shown.holds(e.frame) || frame.holds(e.frame) && shown.intersects(e.frame) {
                            // Reachable by scrolling.
                        } else {
                            let doc = fitScrollView(for: area, window: w).flatMap { sv in sv.documentView.map { w.convertToScreen($0.convert($0.bounds, to: nil)).short } } ?? "no scroll view found"
                            out.append("\(describe(e)) \(f.short) is outside its scroll area \(shown.short) and cannot be scrolled into view (document \(doc))")
                        }
                    }
                } else {
                    // The toolbar and the title bar (the window buttons) lie above the content.
                    let bounds = toolbar || f.minY >= content.maxY - 1 ? frame : content
                    if !bounds.holds(f) {
                        out.append("\(describe(e)) \(f.short) is outside the window \(toolbar ? "" : "content ")\(bounds.short)")
                    }
                }
            }
            for c in e.children { walk(c, depth: depth + 1, scroll: scroll, toolbar: toolbar) }
        }
        walk(Element(obj: w), depth: 0, scroll: nil, toolbar: false)
        // Two scroll areas side by side that overlap: one pane grew past its edge and the other
        // pane covers part of it (the cut part cannot be scrolled to).
        for (i, a) in areas.enumerated() where overlaps {
            for b in areas[(i + 1)...] {
                let overlap = a.1.intersection(b.1)
                guard overlap.width > 8, overlap.height > 8, !a.1.holds(b.1), !b.1.holds(a.1) else { continue }
                out.append("scroll area \(a.0) \(a.1.short) overlaps scroll area \(b.0) \(b.1.short)")
            }
        }
        return out
    }

    /// Performs the accessibility "scroll to visible" action, then lets the app lay out.
    func fitScrollIntoView(_ e: Element) -> Bool {
        let modern = NSSelectorFromString("accessibilityPerformScrollToVisible")
        var asked = false
        if e.obj.responds(to: modern) { _ = e.callBool("accessibilityPerformScrollToVisible"); asked = true }
        let legacy = NSSelectorFromString("accessibilityPerformAction:")
        if !asked, e.obj.responds(to: legacy) { _ = e.obj.perform(legacy, with: "AXScrollToVisible"); asked = true }
        if asked { RunLoop.current.run(until: Date().addingTimeInterval(0.1)) }
        return asked
    }

    /// Whether a scroll area can bring this frame into view: the frame lies inside the scroll
    /// view's document.
    func fitReachable(_ f: NSRect, in area: Element, window w: NSWindow) -> Bool {
        guard let sv = fitScrollView(for: area, window: w), let doc = sv.documentView else { return false }
        // A disclosure triangle reports a frame a few points outside its row; allow for that.
        let docRect = w.convertToScreen(doc.convert(doc.bounds, to: nil)).insetBy(dx: -4, dy: -4)
        return docRect.holds(f)
    }

    func fitScrollView(for area: Element, window w: NSWindow) -> NSScrollView? {
        if let sv = area.obj as? NSScrollView { return sv }
        let f = area.frame
        guard let root = w.contentView?.superview ?? w.contentView else { return nil }
        let p = w.convertPoint(fromScreen: NSPoint(x: f.midX, y: f.midY))
        var v = root.hitTest(root.convert(p, from: nil))
        while let view = v {
            if let sv = view as? NSScrollView { return sv }
            v = view.superview
        }
        return nil
    }
}

extension Element {
    func fitWalk() -> [Element] {
        var out: [Element] = []
        func walk(_ e: Element, _ d: Int) { out.append(e); if d < 40 { for c in e.children { walk(c, d + 1) } } }
        walk(self, 0)
        return out
    }

    @MainActor func fitInToolbar(_ w: NSWindow) -> Bool {
        let f = frame
        return f.minY >= w.convertToScreen(w.contentLayoutRect).maxY - 1
    }
}
