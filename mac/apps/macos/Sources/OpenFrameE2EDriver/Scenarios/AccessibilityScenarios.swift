// The accessibility pass of the Mac track (phase M9): every main workspace, Settings and the AI
// Assistant panel are read through the accessibility tree that VoiceOver reads, in Light and in
// Dark, and the keyboard alone is walked through each workspace.
//
// What the name pass records, per place and mode:
//   - a control (button, menu button, text field, check box, pop-up, slider, tab, link, list) with no
//     accessible name: no label, title, placeholder or text inside it that VoiceOver would speak;
//   - two visible controls that share one identifier;
//   - an image that is not hidden from VoiceOver and has no description.
// The scenario fails on any unnamed control or shared identifier. The full list goes to accessibility-names.json and
// accessibility-names.txt in the output folder; the keyboard walk writes accessibility-keyboard.txt.

import AppKit

extension Scenarios {
    static let accessibilityGroup: [Scenario] = [
        W("96-keyboard-and-screen-reader/a11y-pass-names-light-and-dark", timeout: 1200) { ui, _, ctx in
            try await a11yProject(ui, ctx)
            let report = A11yReport(ui)
            for mode in ["Light", "Dark"] {
                try await ui.a11ySetMode(mode)
                for place in a11yPlaces {
                    do {
                        try await place.show(ui)
                        await ui.idle(0.8)
                        report.add(place.name, mode, ui.a11yAudit(scope: place.scope))
                    } catch {
                        report.add(place.name, mode, [A11yFinding(kind: "not reached", detail: "\(error)")])
                    }
                    try report.write()
                }
            }
            try report.write()
            let unnamed = report.findings.filter { $0.finding.kind == "unnamed control" }
            try ui.expect(unnamed.isEmpty, "\(unnamed.count) controls without a name, first: \(unnamed.prefix(6).map { "\($0.place) \($0.mode): \($0.finding.detail)" })")
            let dup = report.findings.filter { $0.finding.kind == "duplicate identifier" }
            try ui.expect(dup.isEmpty, "\(dup.count) identifiers shared by two visible controls, first: \(dup.prefix(4).map { "\($0.place) \($0.mode): \($0.finding.detail)" })")
            let missed = report.findings.filter { $0.finding.kind == "not reached" }
            try ui.expect(missed.isEmpty, "places the pass could not open: \(missed.map { "\($0.place) \($0.mode): \($0.finding.detail)" })")
        },
        W("96-keyboard-and-screen-reader/a11y-pass-keyboard-reach", timeout: 900) { ui, _, ctx in
            try await a11yProject(ui, ctx)
            var lines: [String] = ["Keyboard navigation (System Settings) on: \(NSApp.isFullKeyboardAccessEnabled)"]
            var problems: [String] = []
            for place in a11yKeyboardPlaces {
                do {
                    try await place.show(ui)
                    await ui.idle(0.8)
                    let walk = try await ui.a11yKeyboardWalk(place.name)
                    lines += walk.lines
                    problems += walk.problems
                } catch {
                    problems.append("\(place.name): not reached: \(error)")
                }
                try (lines + ["", "Problems:"] + problems).joined(separator: "\n")
                    .write(to: ui.artifacts.appendingPathComponent("accessibility-keyboard.txt"), atomically: true, encoding: .utf8)
            }
            try ui.expect(problems.isEmpty, "keyboard findings: \(problems.prefix(8))")
        },
    ]

    /// A project with the Night Bus screenplay, so the writing and production places have content.
    @MainActor static func a11yProject(_ ui: UI, _ ctx: Context) async throws {
        try await ui.startApp(ctx)
        try await ui.createProject("Quiet Keys")
        try await ui.a11yImportNightBus()
    }

    /// One place the pass reads: how to show it and which part of the screen belongs to it.
    struct A11yPlace {
        let name: String
        let scope: A11yScope
        let show: @MainActor (UI) async throws -> Void
    }

    enum A11yScope { case mainWindow, settingsWindow, assistantPanel }

    @MainActor static func workspace(_ title: String) -> A11yPlace {
        A11yPlace(name: title, scope: .mainWindow) { ui in
            await ui.a11yCloseExtras()
            try await ui.go(title)
        }
    }

    @MainActor static func storyTab(_ tab: String) -> A11yPlace {
        A11yPlace(name: "Story: \(tab)", scope: .mainWindow) { ui in
            await ui.a11yCloseExtras()
            try await ui.go("Story")
            try await ui.wait(id: "story-tabs", timeout: 15)
            try await Scenarios.segment(ui, group: "story-tabs", tab)
        }
    }

    @MainActor static func productionTab(_ title: String) -> A11yPlace {
        A11yPlace(name: "Production: \(title)", scope: .mainWindow) { ui in
            await ui.a11yCloseExtras()
            try await ui.goProduction(title, id: UI.productionTabs[title] ?? title)
        }
    }

    @MainActor static func settingsTab(_ tab: String) -> A11yPlace {
        A11yPlace(name: "Settings: \(tab)", scope: .settingsWindow) { ui in
            if ui.a11ySettingsWindow() == nil {
                try await ui.menu([ui.appMenu, "Settings\u{2026}"])
                await ui.idle(0.8)
            }
            try await ui.a11ySettingsTab(tab)
        }
    }

    @MainActor static let assistantPlace = A11yPlace(name: "AI Assistant panel", scope: .assistantPanel) { ui in
        await ui.a11yCloseExtras()
        try await ui.go("Screenplay")
        try await ui.a11yOpenAssistant()
        await ui.idle(0.6)
    }

    @MainActor static let productionTitles = ["Overview", "Catalog", "Locations", "Cast & Crew", "Moodboards", "Storyboards", "Shot Lists",
                                              "Schedule", "Day Out of Days", "Daily View", "Sides", "Reports", "Budget"]

    @MainActor static var a11yPlaces: [A11yPlace] {
        [workspace("Home"), workspace("Idea Vault")]
            + ["Story", "Beat Board", "Outline", "Characters"].map(storyTab)
            + [workspace("Screenplay"), workspace("Breakdown")]
            + productionTitles.map(productionTab)
            + [workspace("Call Sheets"), workspace("Files"), assistantPlace]
            + ["General", "Appearance", "Keyboard Shortcuts", "Storage", "Offline AI", "Diagnostics"].map(settingsTab)
    }

    /// The keyboard walk covers the eight workspaces (Story on its Beat Board, Production on Overview).
    @MainActor static var a11yKeyboardPlaces: [A11yPlace] {
        [workspace("Home"), workspace("Idea Vault"), storyTab("Beat Board"), workspace("Screenplay"), workspace("Breakdown"),
         productionTab("Overview"), workspace("Call Sheets"), workspace("Files")]
    }
}

// MARK: findings

struct A11yFinding {
    let kind: String
    let detail: String
}

@MainActor final class A11yReport {
    struct Row { let place: String; let mode: String; let finding: A11yFinding }
    let ui: UI
    var findings: [Row] = []
    var visited: [(place: String, mode: String)] = []

    init(_ ui: UI) { self.ui = ui }

    func add(_ place: String, _ mode: String, _ list: [A11yFinding]) {
        visited.append((place, mode))
        findings += list.map { Row(place: place, mode: mode, finding: $0) }
        ui.log("a11y \(place) \(mode): \(list.filter { $0.kind != "controls read" }.count) findings")
    }

    func write() throws {
        let json = findings.map { ["place": $0.place, "mode": $0.mode, "kind": $0.finding.kind, "detail": $0.finding.detail] }
        let data = try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys])
        try data.write(to: ui.artifacts.appendingPathComponent("accessibility-names.json"))
        var text = ["Counts per place and mode (controls read: unnamed controls / duplicate identifiers / images without description / not reached):"]
        for v in visited {
            let mine = findings.filter { $0.place == v.place && $0.mode == v.mode }
            func n(_ k: String) -> Int { mine.filter { $0.finding.kind == k }.count }
            let read = mine.first { $0.finding.kind == "controls read" }?.finding.detail ?? "0"
            text.append("  \(v.place) [\(v.mode)] \(read) read: \(n("unnamed control")) / \(n("duplicate identifier")) / \(n("image without description")) / \(n("not reached"))")
        }
        text.append("")
        text += findings.filter { $0.finding.kind != "controls read" }.map { "\($0.place) [\($0.mode)] \($0.finding.kind): \($0.finding.detail)" }
        try text.joined(separator: "\n").write(to: ui.artifacts.appendingPathComponent("accessibility-names.txt"), atomically: true, encoding: .utf8)
    }
}

// MARK: reading the tree

extension UI {
    /// The project window: the widest visible window that is not a sheet.
    func a11yMainWindow() -> NSWindow? {
        NSApp.windows.filter { $0.isVisible && $0.sheetParent == nil && $0.frame.width > 700 && !a11yIsSettings($0) }
            .max { $0.frame.width < $1.frame.width }
    }

    func a11yIsSettings(_ w: NSWindow) -> Bool {
        w.identifier?.rawValue.contains("Settings") == true || w.identifier?.rawValue.contains("settings") == true
            || ["Settings", "General", "Appearance", "Keyboard Shortcuts", "Storage", "Offline AI", "Diagnostics"].contains(w.title)
            || w.title.hasSuffix("Settings")
    }

    func a11ySettingsWindow() -> NSWindow? {
        NSApp.windows.first { $0.isVisible && $0.sheetParent == nil && a11yIsSettings($0) }
    }

    /// Closes the Settings window and the AI Assistant panel so the next place starts clean.
    func a11yCloseExtras() async {
        if let s = a11ySettingsWindow() {
            s.performClose(nil)
            await idle(0.4)
        }
        if exists(id: "ai-panel") {
            try? await a11yPress(label: "Close AI Assistant", role: "AXButton", timeout: 5)
            try? await waitGone(id: "ai-panel", timeout: 8)
        }
        if let main = a11yMainWindow() {
            NSApp.activate(ignoringOtherApps: true)
            main.makeKeyAndOrderFront(nil)
        }
    }

    /// File > Import Screenplay with the Night Bus text pasted, then the imported screenplay opens.
    func a11yImportNightBus() async throws {
        try await menu(["File", "Import Screenplay\u{2026}"])
        try await type(Scenarios.nightBus, into: "import-paste")
        try await a11yPress(id: "import-preview")
        try await waitText("Import Screenplay \u{00B7} Preview", timeout: 30)
        try await a11yPress(id: "import-confirm")
        try await waitText("Import complete", timeout: 30)
        try await a11yPress(id: "import-open-screenplay")
        try await wait(id: "screenplay-editor", timeout: 30, "the screenplay editor after the import")
    }

    /// Command+J opens the AI Assistant panel. The key goes to the project window, which is made
    /// key first; it is tried again when the window was not ready. The toolbar button is not used:
    /// its accessibility press reports a failure although it toggles the panel, so a click after it
    /// would close the panel again.
    func a11yOpenAssistant() async throws {
        for attempt in 1...3 where !exists(id: "ai-panel") {
            NSApp.activate(ignoringOtherApps: true)
            a11yMainWindow()?.makeKeyAndOrderFront(nil)
            await idle(0.5)
            try await shortcut("j", code: 38)
            let deadline = Date().addingTimeInterval(4)
            while Date() < deadline, !exists(id: "ai-panel") { await idle(0.2) }
            if !exists(id: "ai-panel") { log("Command+J try \(attempt): no AI Assistant panel yet (key window \(NSApp.keyWindow?.title ?? "none"))") }
        }
        try await wait(id: "ai-panel", timeout: 5, "the AI Assistant panel after Command+J")
    }

    /// Presses a control: the accessibility press, else a click posted to the event queue. The
    /// driver's own click sends the event inside the call and never returns on a control that
    /// tracks the mouse (the Mode segments of Settings).
    func a11yPress(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 15) async throws {
        try await a11yPress(try await wait(id: id, label: label, role: role, timeout: timeout))
    }

    func a11yPress(_ e: Element) async throws {
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        guard e.isEnabled else { throw DriverError("control is disabled: \(e)") }
        if e.performPress() { await idle(); return }
        let f = e.frame
        let center = NSPoint(x: f.midX, y: f.midY)
        guard f.width > 0, let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }) else {
            throw DriverError("no window under \(e)")
        }
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

    /// A tab of the Settings window toolbar, found in that window only (a label elsewhere can match too).
    func a11ySettingsTab(_ tab: String) async throws {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let w = a11ySettingsWindow() {
                w.makeKeyAndOrderFront(nil)
                var found: Element?
                func walk(_ e: Element, _ d: Int) {
                    guard found == nil, d < 30 else { return }
                    if ["AXButton", "AXRadioButton", "AXToolbarButton"].contains(e.role), e.label == tab || e.title == tab, e.frame.width > 0 { found = e; return }
                    for c in e.children { walk(c, d + 1) }
                }
                walk(Element(obj: w), 0)
                // The window resizes for each tab; a press during that animation can miss.
                if let found, (try? await a11yPress(found)) != nil {
                    await idle(0.6)
                    return
                }
            }
            await idle(0.3)
        }
        throw DriverError("no Settings tab \(tab)")
    }

    /// Settings, Appearance, Mode: Light or Dark, then back to the project window.
    func a11ySetMode(_ mode: String) async throws {
        try await menu([appMenu, "Settings\u{2026}"])
        await idle(0.8)
        try await a11ySettingsTab("Appearance")
        try await a11yPress(label: mode, role: "AXRadioButton")
        await idle(1.0)
        await a11yCloseExtras()
        await idle(0.6)
    }

    private func a11yString(_ e: Element, _ selector: String) -> String {
        let sel = NSSelectorFromString(selector)
        guard e.obj.responds(to: sel), let raw = e.obj.perform(sel)?.takeUnretainedValue() else { return "" }
        if let s = raw as? String { return s }
        if let s = raw as? NSAttributedString { return s.string }
        return ""
    }

    /// The text VoiceOver could speak from inside a control (a button that holds a Text).
    private func a11yInnerText(_ e: Element, depth: Int = 0) -> String {
        guard depth < 4 else { return "" }
        for c in e.children {
            if c.role == "AXStaticText" || c.role == "AXImage", !(c.label + c.title + c.value).isEmpty { return c.label + c.title + c.value }
            let t = a11yInnerText(c, depth: depth + 1)
            if !t.isEmpty { return t }
        }
        return ""
    }

    static let a11yControlRoles: Set<String> = ["AXButton", "AXMenuButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXPopUpButton",
                                                "AXSlider", "AXRadioButton", "AXTabGroup", "AXComboBox", "AXLink", "AXIncrementor",
                                                "AXDisclosureTriangle", "AXSearchField", "AXSegmentedControl",
                                                "AXOutline", "AXTable", "AXList"]
    /// Parts of the system scroll bar and window frame: AppKit names them for VoiceOver itself.
    static let a11ySystemSubroles: Set<String> = ["AXCloseButton", "AXMinimizeButton", "AXZoomButton", "AXFullScreenButton",
                                                  "AXIncrementArrow", "AXDecrementArrow", "AXIncrementPage", "AXDecrementPage"]

    /// Reads one place and returns what a VoiceOver user would miss.
    func a11yAudit(scope: Scenarios.A11yScope) -> [A11yFinding] {
        var roots: [(Element, NSRect)] = []
        switch scope {
        case .mainWindow:
            if let w = a11yMainWindow() { roots.append((Element(obj: w), w.frame)) }
        case .settingsWindow:
            if let w = a11ySettingsWindow() { roots.append((Element(obj: w), w.frame)) }
        case .assistantPanel:
            // SwiftUI copies the panel's identifier to its children, so the panel is the area
            // those elements cover: the window is read, limited to that area.
            if let w = a11yMainWindow() {
                let parts = query(id: "ai-panel").map(\.frame).filter { $0.width > 1 && $0.height > 1 }
                if let first = parts.first { roots.append((Element(obj: w), parts.dropFirst().reduce(first) { $0.union($1) })) }
            }
        }
        guard !roots.isEmpty else { return [A11yFinding(kind: "not reached", detail: "no window for \(scope)")] }
        var out: [A11yFinding] = []
        var ids: [String: [String]] = [:]
        var read = 0
        func visible(_ e: Element, in win: NSRect) -> Bool {
            let f = e.frame
            guard f.width > 1, f.height > 1, f.minX.isFinite, f.minY.isFinite else { return false }
            return scope == .assistantPanel ? win.contains(NSPoint(x: f.midX, y: f.midY)) : win.intersects(f)
        }
        func walk(_ e: Element, _ win: NSRect, _ depth: Int) {
            guard depth < 60 else { return }
            let role = e.role
            if role == "AXScrollBar" { return }
            let sub = e.subrole
            if Self.a11yControlRoles.contains(role), !Self.a11ySystemSubroles.contains(sub), visible(e, in: win) {
                read += 1
                let placeholder = a11yString(e, "accessibilityPlaceholderValue")
                let named = !e.label.isEmpty || !e.title.isEmpty || !placeholder.isEmpty
                    || (["AXButton", "AXCheckBox", "AXRadioButton", "AXMenuButton", "AXLink", "AXDisclosureTriangle"].contains(role) && !a11yInnerText(e).isEmpty)
                let where_ = "at \(Int(e.frame.minX)),\(Int(e.frame.minY)) \(Int(e.frame.width))x\(Int(e.frame.height))"
                if !named {
                    out.append(A11yFinding(kind: "unnamed control", detail: "\(role)\(sub.isEmpty ? "" : "/\(sub)") id=\(e.identifier) value=\(e.value.prefix(40)) help=\(a11yString(e, "accessibilityHelp").prefix(40)) \(where_)"))
                }
                if !e.identifier.isEmpty {
                    let key = "\(role) \(e.label.isEmpty ? e.title : e.label) \(where_)"
                    if !(ids[e.identifier]?.contains(key) ?? false) { ids[e.identifier, default: []].append(key) }
                }
            }
            if role == "AXImage", visible(e, in: win), e.label.isEmpty, e.title.isEmpty {
                out.append(A11yFinding(kind: "image without description", detail: "AXImage id=\(e.identifier) at \(Int(e.frame.minX)),\(Int(e.frame.minY)) \(Int(e.frame.width))x\(Int(e.frame.height))"))
            }
            for c in e.children { walk(c, win, depth + 1) }
        }
        for (root, win) in roots { walk(root, win, 0) }
        out.append(A11yFinding(kind: "controls read", detail: "\(read)"))
        for (id, uses) in ids.sorted(by: { $0.key < $1.key }) where uses.count > 1 {
            out.append(A11yFinding(kind: "duplicate identifier", detail: "\(id) on \(uses.count) controls: \(uses.prefix(4).joined(separator: "; "))"))
        }
        return out
    }

    // MARK: keyboard

    /// The element that has keyboard focus, as VoiceOver sees it.
    func a11yFocused() -> Element? {
        let sel = NSSelectorFromString("accessibilityFocusedUIElement")
        guard NSApp.responds(to: sel), let raw = NSApp.perform(sel)?.takeUnretainedValue() as? NSObject else { return nil }
        let e = Element(obj: raw)
        if e.obj === NSApp || e.role == "AXApplication" || e.role == "AXWindow" { return nil }
        return e
    }

    func a11yFocusName(_ e: Element) -> String {
        let name = [e.label, e.title, a11yString(e, "accessibilityPlaceholderValue"), a11yInnerText(e)].first { !$0.isEmpty } ?? ""
        return "\(e.role) \(e.identifier.isEmpty ? "" : "#\(e.identifier) ")\"\(name.prefix(40))\""
    }

    /// The controls Tab must reach in the project window. macOS moves Tab through every control
    /// only when "Keyboard navigation" is on in System Settings; with it off (this machine), Tab
    /// stops at text fields, text views and lists, so those are the controls that must be reached.
    /// The app cannot turn the setting on for itself, and the run never changes the person's
    /// system settings.
    func a11yKeyCandidates(_ w: NSWindow) -> [String] {
        let roles: Set<String> = NSApp.isFullKeyboardAccessEnabled
            ? Self.a11yControlRoles.union(["AXOutline", "AXTable"])
            : ["AXTextField", "AXTextArea", "AXOutline", "AXTable"]
        var out: [String] = []
        func walk(_ e: Element, _ d: Int) {
            guard d < 60 else { return }
            if e.role == "AXScrollBar" { return }
            let f = e.frame
            if roles.contains(e.role), !Self.a11ySystemSubroles.contains(e.subrole), e.isEnabled,
               f.width > 1, f.height > 1, w.frame.intersects(f) {
                let n = a11yFocusName(e)
                if !out.contains(n) { out.append(n) }
            }
            for c in e.children { walk(c, d + 1) }
        }
        walk(Element(obj: w), 0)
        return out
    }

    /// Tab from nothing through the window, then Shift+Tab back: every stop is named, focus never
    /// sticks on one control while others wait, every control Tab should reach is reached, and
    /// Shift+Tab walks the same stops in reverse. A text view that takes Tab as text (the
    /// screenplay) is left with Control+Tab, the macOS way out of a text view.
    func a11yKeyboardWalk(_ place: String) async throws -> (lines: [String], problems: [String]) {
        guard let w = a11yMainWindow() else { throw DriverError("no project window") }
        NSApp.activate(ignoringOtherApps: true)
        w.makeKeyAndOrderFront(nil)
        w.makeFirstResponder(nil)
        await idle(0.3)
        let candidates = a11yKeyCandidates(w)
        var stops: [String] = []
        var problems: [String] = []
        var same = 0
        var wrapped = false
        func press(_ mods: NSEvent.ModifierFlags) async throws -> String? {
            let inText = (w.firstResponder as? NSTextView).map { !$0.isFieldEditor } ?? false
            // Shift+Tab types the back-tab character, as the keyboard does.
            let chars = mods.contains(.shift) ? "\u{19}" : "\t"
            try await key(chars, code: 48, modifiers: inText ? mods.union(.control) : mods)
            await idle(0.25)
            return a11yFocused().map(a11yFocusName)
        }
        for i in 1...60 {
            guard let name = try await press([]) else {
                problems.append("\(place): press \(i): no element has focus")
                continue
            }
            if name.hasSuffix("\"\"") { problems.append("\(place): press \(i): the focused element has no name: \(name)") }
            if stops.last == name { same += 1 } else { same = 0 }
            if same >= 3 {
                // One stop in the whole window is the loop itself, not a trap.
                if candidates.count > 1 { problems.append("\(place): press \(i): focus stuck on \(name)") }
                break
            }
            if let first = stops.first, name == first, stops.count > 1 { wrapped = true; break }
            if stops.last != name { stops.append(name) }
        }
        // Shift+Tab from where the walk ended goes back through the stops in reverse.
        let ring = stops.count > 1 ? stops : []
        var expected: [String] = []
        if !ring.isEmpty {
            var at = wrapped ? 0 : ring.count - 1
            for _ in 0..<min(4, ring.count - 1) { at = (at - 1 + ring.count) % ring.count; expected.append(ring[at]) }
        }
        var back: [String] = []
        for _ in expected { back.append(try await press([.shift]) ?? "nothing") }
        let backOK = back == expected
        if !backOK { problems.append("\(place): Shift+Tab does not retrace the Tab stops: back \(back), expected \(expected)") }
        let missed = candidates.filter { !stops.contains($0) }
        if !missed.isEmpty { problems.append("\(place): Tab never reaches \(missed)") }
        var lines = ["", "\(place): \(stops.count) Tab stops, \(candidates.count) controls Tab should reach, \(missed.count) missed; Shift+Tab retraces: \(backOK)"]
        lines += stops.map { "  \($0)" }
        return (lines, problems)
    }
}
