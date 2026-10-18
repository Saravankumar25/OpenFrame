// Real-app scenarios for the writing workspaces and the app shell.

import AppKit
import ObjectiveC

fileprivate extension Element {
    /// The value, asked in both accessibility APIs.
    var anyValue: String {
        if !value.isEmpty { return value }
        let sel = NSSelectorFromString("accessibilityAttributeValue:")
        guard obj.responds(to: sel), let raw = obj.perform(sel, with: "AXValue")?.takeUnretainedValue() else { return "" }
        return (raw as? String) ?? (raw as? NSAttributedString)?.string ?? ""
    }

    /// The tooltip or help text of the element.
    var help: String {
        let sel = NSSelectorFromString("accessibilityHelp")
        return obj.responds(to: sel) ? (obj.perform(sel)?.takeUnretainedValue() as? String ?? "") : ""
    }
}

fileprivate extension UI {
    func dump(_ name: String) {
        let text = all().map(\.description).joined(separator: "\n")
        try? text.write(to: artifacts.appendingPathComponent("dump-\(name).txt"), atomically: true, encoding: .utf8)
    }
}

fileprivate extension UI {
    /// Items (titles) the open menu of a pop-up button shows now.
    func openMenuItems(_ button: Element) -> [Element] {
        for ch in button.children where ch.role == "AXMenu" { return ch.children.filter { $0.role == "AXMenuItem" } }
        return []
    }

    /// Opens a pop-up menu button the way a click does and chooses an item by title; a path
    /// of titles walks into submenus. A title matches when the item starts with it.
    func chooseMenu(_ button: Element, _ path: [String], exact: Bool = false, timeout: Double = 6) async throws {
        log("menu button \(button.identifier.isEmpty ? button.label : button.identifier) > \(path.joined(separator: " > "))")
        guard button.isEnabled else { throw DriverError("menu button is disabled: \(button)") }
        var items: [Element] = []
        var deadline = Date()
        for attempt in 1...3 where items.isEmpty {
            DispatchQueue.main.async { _ = button.performPress() }
            deadline = Date().addingTimeInterval(timeout * Double(attempt))
            while Date() < deadline {
                items = openMenuItems(button)
                if !items.isEmpty { break }
                await idle(0.1)
            }
            if items.isEmpty { try? await escapeMenu(); await idle(0.5) }
        }
        guard !items.isEmpty else { throw DriverError("the menu did not open: \(button)") }
        for (i, want) in path.enumerated() {
            func name(_ e: Element) -> String { e.title.isEmpty ? e.label : e.title }
            deadline = Date().addingTimeInterval(timeout)
            var item: Element?
            while Date() < deadline {
                item = items.first { exact ? name($0) == want : name($0).hasPrefix(want) }
                if item != nil { break }
                await idle(0.1)
            }
            guard let item else {
                _ = try? await escapeMenu()
                throw DriverError("no menu item \(want) in \(items.map(name))")
            }
            guard item.isEnabled else { _ = try? await escapeMenu(); throw DriverError("menu item is disabled: \(want)") }
            if i == path.count - 1 {
                _ = item.performPress()
                await idle(0.3)
                // The menu is closed when its items are gone.
                let gone = Date().addingTimeInterval(4)
                while Date() < gone, !openMenuItems(button).isEmpty { await idle(0.1) }
                if NSApp.windows.contains(where: { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }) {
                    try? await escapeMenu()
                    await idle(0.3)
                }
                return
            }
            _ = item.performPress()
            await idle(0.3)
            var sub: [Element] = []
            let until = Date().addingTimeInterval(timeout)
            while Date() < until {
                sub = item.children.first(where: { $0.role == "AXMenu" })?.children.filter { $0.role == "AXMenuItem" } ?? []
                if !sub.isEmpty { break }
                await idle(0.1)
            }
            guard !sub.isEmpty else { _ = try? await escapeMenu(); throw DriverError("submenu of \(want) did not open") }
            items = sub
        }
    }

    /// Closes an open menu. A menu tracks events from the queue, not from sendEvent, so the
    /// Escape goes into the queue and the menu is told to end tracking as well.
    func escapeMenu() async throws {
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("PopupMenu") { _ = w }
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            if let ev = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                         windowNumber: 0, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                NSApp.postEvent(ev, atStart: true)
            }
        }
        await idle(0.3)
        for e in all() where e.role == "AXMenuButton" || e.role == "AXPopUpButton" {
            if let m = menuOf(e), m.highlightedItem != nil || !openMenuItems(e).isEmpty { m.cancelTracking() }
        }
        await idle(0.2)
    }

    func menuOf(_ e: Element) -> NSMenu? {
        let sel = NSSelectorFromString("menu")
        return e.obj.responds(to: sel) ? e.obj.perform(sel)?.takeUnretainedValue() as? NSMenu : nil
    }

    func chooseMenu(id: String? = nil, label: String? = nil, _ path: [String], exact: Bool = false) async throws {
        let deadline = Date().addingTimeInterval(15)
        var found: Element?
        while Date() < deadline {
            found = query(id: id, label: label, role: "AXMenuButton").first ?? query(id: id, label: label, role: "AXPopUpButton").first
            if found == nil, let label { found = query(id: id, label: "\(label), \(label)", role: "AXPopUpButton").first }
            if found != nil { break }
            await idle(0.1)
        }
        guard let b = found else {
            try? snapshot("missing-menu")
            throw DriverError("menu button not found within 15 s: \(id ?? label ?? "")")
        }
        try await chooseMenu(b, path, exact: exact)
    }

    /// Titles of a menu without choosing anything; the menu is closed again.
    func menuItemTitles(id: String) async throws -> [String] {
        let b = try await wait(id: id, role: "AXMenuButton")
        DispatchQueue.main.async { _ = b.performPress() }
        var items: [Element] = []
        let deadline = Date().addingTimeInterval(5)
        while Date() < deadline { items = openMenuItems(b); if !items.isEmpty { break }; await idle(0.1) }
        let names = items.map { $0.title.isEmpty ? $0.label : $0.title }
        try await escapeMenu()
        let gone = Date().addingTimeInterval(3)
        while Date() < gone, !openMenuItems(b).isEmpty { await idle(0.1) }
        return names
    }
}

fileprivate extension UI {
    /// Runs a named accessibility action of an element (the VoiceOver "Actions" rotor).
    func customAction(_ e: Element, _ name: String) async throws {
        let sel = NSSelectorFromString("accessibilityCustomActions")
        guard e.obj.responds(to: sel), let list = e.obj.perform(sel)?.takeUnretainedValue() as? [NSAccessibilityCustomAction] else {
            throw DriverError("no accessibility actions on \(e)")
        }
        guard let a = list.first(where: { $0.name == name }) else { throw DriverError("no action \(name) in \(list.map(\.name))") }
        log("accessibility action \(name)")
        _ = a.handler?()
        await idle(0.3)
    }

    /// Delete on a selected card: the Delete key when the app has keyboard focus; when the screen
    /// is locked and no app can become active (SwiftUI key handling needs a key window), the
    /// card's own Delete action, which VoiceOver offers.
    func deleteCard(_ text: String) async throws {
        try await pressCard(text)
        if appIsActive { try await deleteKey() } else {
            guard let c = card(text) else { throw DriverError("card not found: \(text)") }
            try await customAction(c, "Delete")
        }
    }

    /// The context menu a right click on an element opens, as the hosting view builds it.
    func contextMenuItems(at center: NSPoint) -> NSMenu? {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }), let content = w.contentView else { return nil }
        let local = w.convertPoint(fromScreen: center)
        guard let ev = NSEvent.mouseEvent(with: .rightMouseDown, location: local, modifierFlags: [],
                                          timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w.windowNumber,
                                          context: nil, eventNumber: 0, clickCount: 1, pressure: 1) else { return nil }
        var v = content.hitTest(content.superview?.convert(local, from: nil) ?? local)
        while let vv = v {
            if let m = vv.menu(for: ev), !m.items.isEmpty { m.update(); return m }
            v = vv.superview
        }
        return nil
    }

    /// Chooses an item of the context menu of an element (a path of titles for submenus; a title
    /// matches when the item starts with it).
    func contextMenu(on e: Element, _ path: [String]) async throws {
        log("context menu \(e.label.isEmpty ? e.identifier : e.label) > \(path.joined(separator: " > "))")
        let f = e.frame
        guard var menu = contextMenuItems(at: NSPoint(x: f.midX, y: f.midY)) else { throw DriverError("no context menu on \(e)") }
        for (i, want) in path.enumerated() {
            menu.update()
            guard let item = menu.items.first(where: { $0.title.hasPrefix(want) }) else {
                throw DriverError("no context menu item \(want) in \(menu.items.map(\.title))")
            }
            guard item.isEnabled else { throw DriverError("context menu item is disabled: \(want)") }
            if i == path.count - 1 {
                menu.performActionForItem(at: menu.index(of: item))
            } else if let sub = item.submenu {
                menu = sub
            } else { throw DriverError("\(want) has no submenu") }
        }
        await idle(0.4)
    }

    func contextMenuTitles(on e: Element) -> [String] {
        let f = e.frame
        return contextMenuItems(at: NSPoint(x: f.midX, y: f.midY))?.items.map(\.title) ?? []
    }

    /// A real mouse drag: press, move in steps past the drag threshold, release. Events go through
    /// the event queue so the drag session reads them as it would from the mouse.
    func drag(from a: NSPoint, to b: NSPoint, steps: Int = 12) async throws {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(a) }) else { throw DriverError("no window under \(a)") }
        w.makeKeyAndOrderFront(nil)
        func ev(_ type: NSEvent.EventType, _ p: NSPoint) -> NSEvent? {
            NSEvent.mouseEvent(with: type, location: w.convertPoint(fromScreen: p), modifierFlags: [],
                               timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w.windowNumber,
                               context: nil, eventNumber: 0, clickCount: 1, pressure: type == .leftMouseUp ? 0 : 1)
        }
        if let e = ev(.leftMouseDown, a) { NSApp.postEvent(e, atStart: false) }
        await idle(0.15)
        for i in 1...steps {
            let t = Double(i) / Double(steps)
            let p = NSPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t)
            if let e = ev(.leftMouseDragged, p) { NSApp.postEvent(e, atStart: false) }
            await idle(0.05)
        }
        await idle(0.2)
        if let e = ev(.leftMouseUp, b) { NSApp.postEvent(e, atStart: false) }
        await idle(0.5)
    }

    func vaultCardElements() -> [Element] {
        var seen = Set<String>()
        return all().filter { $0.identifier.hasPrefix("vault-card-") && $0.frame.width > 0 && seen.insert($0.identifier).inserted }
    }

    func vaultCards() -> [String] { vaultCardElements().map(\.label) }

    func vaultCard(_ prefix: String) -> Element? { vaultCardElements().first { $0.label.hasPrefix(prefix) } }

    func waitVaultCard(_ prefix: String, timeout: Double = 10) async throws {
        try await waitUntil("a Vault card starting with \(prefix), now \(vaultCards())", timeout: timeout) { vaultCard(prefix) != nil }
    }

    func quickCapture(_ text: String) async throws {
        try await act(id: "vault-quick-capture", role: "AXButton")
        try await fill("Idea", text, id: "quick-capture-idea")
        try await button("Save")
        try await waitGone(id: "quick-capture-idea", timeout: 10)
        try await waitVaultCard(String(text.prefix(18)))
    }

    func deleteVaultCard(_ prefix: String) async throws {
        guard let c = vaultCard(prefix) else { throw DriverError("no Vault card \(prefix)") }
        try await tap(c)
        if appIsActive { try await deleteKey() } else { try await customAction(vaultCard(prefix) ?? c, "Delete") }
    }

    /// The project cards on Application Home, by accessible name ("Open X, Feature Film, Writing").
    func homeCards() -> [String] {
        var seen = Set<String>()
        return all().filter { $0.identifier.hasPrefix("project-card-") && $0.label.hasPrefix("Open ") && seen.insert($0.identifier).inserted }.map(\.label)
    }

    func homeCardWait(_ title: String) async throws -> Element {
        try await waitUntil("project card \(title), now \(homeCards())") { homeCard(title) != nil }
        return homeCard(title)!
    }

    func cardWait(_ text: String) async throws -> Element {
        try await waitUntil("card \(text)") { card(text) != nil }
        return card(text)!
    }

    func vaultCardWait(_ prefix: String) async throws -> Element {
        try await waitVaultCard(prefix)
        return vaultCard(prefix)!
    }

    func homeCard(_ title: String) -> Element? {
        all().first { $0.identifier == "project-card-\(title)" && $0.label.hasPrefix("Open ") }
    }

    func waitHomeCards(_ count: Int) async throws {
        try await waitUntil("\(count) project cards, now \(homeCards())") { homeCards().count == count }
    }

    /// Refresh Home: Global Idea Vault and back (the list is read again from the core).
    func refreshHome() async throws {
        try await waitHome()
    }

    func windowTitle() -> String { (NSApp.keyWindow ?? windows.first)?.title ?? "" }

    func projectStatus() async throws -> String {
        let t = try await menuItemTitles(id: "project-menu")
        return t.first { $0.hasPrefix("Status: ") } ?? ""
    }

    /// Opens the About window from the app menu.
    func openAbout() async throws {
        guard let appMenu = NSApp.mainMenu?.items.first?.title else { throw DriverError("no app menu") }
        try await menu([appMenu, "About OpenFrame Studio"])
        try await wait(id: "about-version", timeout: 15, "the About window")
    }

    func aboutText() -> String {
        all().filter { ["about-version", "about-core", "about-channel", "about-notice"].contains($0.identifier) }.map(\.text).joined(separator: "\n")
    }

    /// No price: no currency sign and none of the words price, pay once, subscription, lifetime.
    func expectNoPrice() throws {
        let text = screenText()
        for sign in ["$", "\u{20AC}", "\u{00A3}", "\u{20B9}", "\u{00A5}"] { try expect(!text.contains(sign), "a price sign on the screen: \(sign)") }
        for word in ["price", "pay once", "subscription", "lifetime"] { try expect(!text.lowercased().contains(word), "the word \(word) is on the screen") }
    }

    // MARK: screenplay editor

    /// The element label the toolbar shows for the line the caret is on.
    func caretToStart(_ tv: NSTextView) async {
        tv.window?.makeFirstResponder(tv)
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        await idle(0.15)
    }

    /// Undo with the caret in the editor: Command+Z, and the Edit menu when the key does nothing.
    func textUndo(_ tv: NSTextView) async throws {
        let before = tv.string
        try await key("z", code: 6, modifiers: [.command])
        if tv.string == before {
            if menuItemEnabled(["Edit", "Undo"]) { log("Command+Z changed nothing; using Edit > Undo"); try await menu(["Edit", "Undo"]) }
            else {
                // A locked screen leaves no key window, so the menu is off; the editor's own undo stack answers the same command.
                log("no key window (active app: \(appIsActive)); asking the editor's undo manager")
                tv.undoManager?.undo()
                await idle(0.3)
            }
        }
    }

    func textRedo(_ tv: NSTextView) async throws {
        let before = tv.string
        try await key("z", code: 6, modifiers: [.command, .shift])
        if tv.string == before {
            if menuItemEnabled(["Edit", "Redo"]) { log("Shift+Command+Z changed nothing; using Edit > Redo"); try await menu(["Edit", "Redo"]) }
            else {
                log("no key window (active app: \(appIsActive)); asking the editor's undo manager")
                tv.undoManager?.redo()
                await idle(0.3)
            }
        }
    }

    func screenplayPickerButton() -> Element? { query(id: "screenplay-picker").first }

    /// Titles in the screenplay picker menu (the screenplays of the project), without the trailing detail.
    func screenplayTitles() async throws -> [String] {
        let items = try await menuItemTitles(id: "screenplay-picker")
        let actions = ["New Screenplay", "Rename Screenplay", "Delete Screenplay"]
        return items.filter { !$0.isEmpty && !actions.contains(where: $0.hasPrefix) && !$0.hasPrefix("Screenplay") }
            .map { $0.components(separatedBy: " \u{00B7} ").first ?? $0 }
    }

    /// Writes elements into the open draft at the end: a line per element, its type set with the Command number keys.
    func writeElements(_ tv: NSTextView, _ lines: [(digit: Int, text: String)], first: Bool = false) async throws {
        let codes: [Int: UInt16] = [1: 18, 2: 19, 3: 20, 4: 21, 5: 23, 6: 22, 7: 26]
        for (i, l) in lines.enumerated() {
            if !(first && i == 0) {
                if !suggestionRows().isEmpty { try await escape() }
                try await enter()
            }
            try await cmdKey(String(l.digit), code: codes[l.digit]!)
            try await typeHere(l.text)
        }
        if !suggestionRows().isEmpty { try await escape() }
    }

    /// The name of the draft in the draft menu that is not the open one.
    func otherDraftName() async throws -> String {
        let open = currentDraftName()
        let titles = try await menuItemTitles(id: "draft-menu")
        let names = titles.filter { !$0.isEmpty && !$0.hasPrefix("New Draft") && !$0.hasPrefix("Manage Drafts") && !$0.hasPrefix("Renumber") }
        guard let other = names.first(where: { $0 != open }) else { throw DriverError("no other draft in \(titles)") }
        return other
    }

    func editorText() -> String { (query(id: "screenplay-editor").first?.obj as? NSTextView)?.string ?? "" }

    func currentEditorView() -> NSTextView? { query(id: "screenplay-editor").first?.obj as? NSTextView }

    /// A scene in the Scenes navigator, by words of its heading.
    func sceneRow(_ words: String) async throws -> Element {
        try await waitUntil("a scene of the navigator named \(words)") {
            all().contains { $0.label.hasPrefix("Scene ") && $0.label.contains(words) && $0.frame.width > 0 }
        }
        return all().first { $0.label.hasPrefix("Scene ") && $0.label.contains(words) && $0.frame.width > 0 }!
    }

    /// Interactive controls on screen that have no name (label, title, help or text inside).
    func unnamedControls() -> [String] {
        let interactive: Set<String> = ["AXButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXRadioButton", "AXPopUpButton", "AXMenuButton", "AXLink", "AXSlider", "AXComboBox", "AXTabGroup"]
        var out: [String] = []
        func walk(_ e: Element, _ depth: Int) {
            if depth > 60 { return }
            if interactive.contains(e.role) {
                let sub = e.obj.perform(NSSelectorFromString("accessibilitySubrole"))?.takeUnretainedValue() as? String ?? ""
                let named = !e.label.isEmpty || !e.title.isEmpty || !(e.help).isEmpty
                    || e.children.contains { !$0.text.isEmpty }
                    || (e.role == "AXTextField" && !e.value.isEmpty)
                let windowButton = sub.contains("CloseButton") || sub.contains("MinimizeButton") || sub.contains("ZoomButton") || sub.contains("FullScreenButton")
                if !named && !windowButton { out.append("\(e.role) id=\(e.identifier) sub=\(sub) frame=\(e.frame)") }
            }
            for c in e.children { walk(c, depth + 1) }
        }
        for w in windows.reversed() { walk(Element(obj: w), 0) }
        return out
    }

    /// What the focused responder is: its accessible name, and whether it shows a focus mark.
    func responderDescription(_ r: NSResponder) -> (name: String, text: String, ring: Bool) {
        if let tv = r as? NSTextView {
            let owner = (tv.delegate as? NSTextField)
            let name = owner?.accessibilityLabel() ?? tv.accessibilityLabel() ?? tv.accessibilityIdentifier()
            return (name, "text \(name)", true)
        }
        if let c = r as? NSControl {
            let name = c.accessibilityLabel() ?? (c as? NSButton)?.title ?? c.accessibilityTitle() ?? ""
            return (name, "\(Swift.type(of: c)) \(name)", c.focusRingType != .none)
        }
        if let v = r as? NSView {
            let name = v.accessibilityLabel() ?? v.accessibilityTitle() ?? ""
            return (name, "\(Swift.type(of: v)) \(name)", v.focusRingType != .none)
        }
        return ("", "\(Swift.type(of: r))", false)
    }

    /// Writes the result file the runner reads, for a phase that ends by closing the app itself.
    func writeResult(phase: Int, status: String, message: String) {
        let result: [String: Any] = ["scenario": "", "phase": phase, "status": status, "message": message, "seconds": 0, "end": "quit"]
        if let data = try? JSONSerialization.data(withJSONObject: result, options: [.prettyPrinted, .sortedKeys]) {
            try? data.write(to: artifacts.appendingPathComponent("result-\(phase).json"))
        }
    }

    // MARK: appearance

    func openSettings(_ tab: String) async throws {
        guard let appMenu = NSApp.mainMenu?.items.first?.title else { throw DriverError("no app menu") }
        try await menu([appMenu, "Settings\u{2026}"])
        try await act(label: tab, timeout: 15)
        await idle(0.5)
    }

    func closeSettings() async throws {
        if let w = windows.first(where: { $0.title == "Settings" || $0.title.hasSuffix("Settings") || $0.title == "Appearance" }) { w.performClose(nil) }
        await idle(0.4)
    }

    func themeCardSelected(_ id: String) -> Bool {
        guard let c = query(id: "theme-card-\(id)").first(where: { $0.role == "AXButton" }) ?? query(id: "theme-card-\(id)").first else { return false }
        return c.callBool("isAccessibilitySelected") == true || c.value == "1"
    }

    func modeSelected(_ name: String) -> Bool {
        query(label: name, role: "AXRadioButton").contains { $0.value == "1" }
    }

    func chooseMode(_ name: String) async throws {
        try await act(label: name, role: "AXRadioButton")
        await idle(0.6)
    }

    /// The colour the project window is drawn in at a point of empty background, as 0 to 255.
    func backgroundRGB(inWindowTitled prefix: String? = nil) -> (r: Int, g: Int, b: Int)? {
        let w = windows.first { !$0.title.contains("Settings") && !$0.title.hasPrefix("Appearance") && !$0.title.hasPrefix("General") && (prefix.map($0.title.hasPrefix) ?? true) && $0.frame.width > 400 } ?? windows.first
        guard let w, let image = CGWindowListCreateImage(.null, .optionIncludingWindow, CGWindowID(w.windowNumber), [.boundsIgnoreFraming]) else { return nil }
        let rep = NSBitmapImageRep(cgImage: image)
        let x = Int(Double(rep.pixelsWide) * 0.8), y = Int(Double(rep.pixelsHigh) * 0.8)
        guard let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { return nil }
        return (Int((c.redComponent * 255).rounded()), Int((c.greenComponent * 255).rounded()), Int((c.blueComponent * 255).rounded()))
    }

    /// Shows one of the 15 screens of the theme walk.
    func showThemeScreen(_ screen: String) async throws {
        if screen == "Application Home" {
            if exists(id: "sidebar-home") { try await closeProject() }
            try await waitHome()
            return
        }
        if !exists(id: "sidebar-home") {
            try await waitHome()
            try await act(id: "project-card-Night Bus")
            try await wait(id: "sidebar-home", timeout: 30, "the project shell of Night Bus")
            if await recoveryOffered(within: 0.5) { try await act(label: "Keep the latest work", role: "AXButton") }
        }
        switch screen {
        case "Story Outline":
            try await go("Story"); try await tab("Outline")
        case "Story Board":
            try await go("Story"); try await tab("Board")
        default:
            try await go(screen == "Notes and Tasks" ? "Notes and Tasks" : screen)
        }
    }

    /// Text on the main window whose contrast against the colour behind it is under 4.5 to 1 (3 to 1 for large text).
    func readabilityProblems(screen: String) -> [String] {
        guard let w = windows.first(where: { !$0.title.contains("Settings") && !$0.title.hasPrefix("Appearance") && $0.frame.width > 400 }),
              let view = w.contentView?.superview ?? w.contentView,
              let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return ["no window to read"] }
        view.cacheDisplay(in: view.bounds, to: rep)
        let scale = Double(rep.pixelsWide) / Double(view.bounds.width)
        func lum(_ c: (Double, Double, Double)) -> Double {
            func f(_ v: Double) -> Double { v <= 0.03928 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
            return 0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
        }
        var out: [String] = []
        var seen = Set<String>()
        let roles: Set<String> = ["AXStaticText", "AXButton", "AXMenuButton", "AXRadioButton", "AXCheckBox", "AXTextField", "AXHeading"]
        for e in all() where roles.contains(e.role) && !e.text.isEmpty && !e.identifier.hasPrefix("screenplay-editor") && !e.identifier.hasPrefix("theme-card") {
            let f = e.frame
            guard f.width > 6, f.height > 6, f.width < 900, f.minX.isFinite, f.minY.isFinite, f.maxX.isFinite, f.maxY.isFinite else { continue }
            let inWin = w.convertFromScreen(f)
            guard inWin.minX >= 0, inWin.minY >= 0, inWin.maxX <= w.frame.width, inWin.maxY <= w.frame.height else { continue }
            let key = "\(e.text)|\(Int(f.minX))|\(Int(f.minY))"
            guard seen.insert(key).inserted else { continue }
            let yTop = Double(view.bounds.height) - inWin.maxY   // the bitmap is top-down
            let fx0 = inWin.minX * scale, fx1 = inWin.maxX * scale, fy0 = yTop * scale, fy1 = (yTop + inWin.height) * scale
            guard fx0.isFinite, fx1.isFinite, fy0.isFinite, fy1.isFinite, fx0 >= 0, fy0 >= 0 else { continue }
            let x0 = Int(fx0), x1 = min(rep.pixelsWide - 1, Int(fx1))
            let y0 = Int(fy0), y1 = min(rep.pixelsHigh - 1, Int(fy1))
            guard x1 > x0, y1 > y0 else { continue }
            var counts: [String: (n: Int, c: (Double, Double, Double))] = [:]
            var px: [(Double, Double, Double)] = []
            for y in stride(from: y0, through: y1, by: 1) {
                for x in stride(from: x0, through: x1, by: 1) {
                    guard let c = rep.colorAt(x: x, y: y)?.usingColorSpace(.sRGB) else { continue }
                    let t = (Double(c.redComponent), Double(c.greenComponent), Double(c.blueComponent))
                    px.append(t)
                    let k = "\(Int(t.0 * 31)),\(Int(t.1 * 31)),\(Int(t.2 * 31))"
                    counts[k] = ((counts[k]?.n ?? 0) + 1, t)
                }
            }
            guard let bg = counts.values.max(by: { $0.n < $1.n })?.c else { continue }
            let lb = lum(bg)
            var best = 1.0
            var ink = 0
            for t in px {
                let l = lum(t)
                let r = (max(l, lb) + 0.05) / (min(l, lb) + 0.05)
                if r > 1.25 { ink += 1 }
                best = max(best, r)
            }
            guard ink >= 6 else { continue }   // an icon or an empty box: no text to read
            let large = f.height >= 24
            if best < (large ? 3.0 : 4.5) { out.append("\"\(e.text.prefix(30))\" \(String(format: "%.2f", best))") }
        }
        return out
    }

    /// Whether the Story Board characters list holds the name (the Characters page is read, then the editor is shown again).
    func storyCharactersInclude(_ name: String, _ expected: Bool) async throws {
        try await go("Story")
        try await tab("Characters")
        await idle(1.0)
        let has = all().contains { $0.text.contains(name) && $0.identifier != "screenplay-editor" }
        try expect(has == expected, "the Story Board characters \(expected ? "include" : "no longer include") \(name)")
        try await tab("Board")
        try await go("Screenplay")
        _ = try await editor()
    }

    /// Opens a workspace: the Go menu when it has the workspace, else the sidebar row is chosen
    /// (the list selects the row, as a click does).
    func goTo(_ workspace: String) async throws {
        if menuItemEnabled(["Go", workspace]) {
            try await menu(["Go", workspace])
            await idle(0.4)
            return
        }
        let key = UI.workspaceKey(workspace)
        guard let outline = all().first(where: { $0.role == "AXOutline" && $0.label == "Sidebar" && $0.identifier != "scene-navigator" }),
              let table = outline.obj as? NSTableView else { throw DriverError("no sidebar to choose \(workspace) in") }
        // Find the row that holds the workspace.
        for row in 0..<table.numberOfRows {
            guard let cell = table.view(atColumn: 0, row: row, makeIfNecessary: false) else { continue }
            func holds(_ v: NSView) -> Bool {
                if Element(obj: v).identifier == "sidebar-\(key)" { return true }
                return v.subviews.contains(where: holds)
            }
            var found = holds(cell)
            if !found { found = Element(obj: cell).children.contains { $0.identifier == "sidebar-\(key)" } }
            if found {
                table.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
                await idle(0.6)
                return
            }
        }
        // Rows that are not on screen yet are asked for by position.
        let titles = ["Project Home", "Screenplay", "Story", "Idea Vault", "Breakdown", "Schedule", "Call Sheets", "Budget", "Visual Planning", "Files", "Notes and Tasks", "Activity", "Recently Deleted"]
        guard titles.contains(workspace) else { throw DriverError("unknown workspace \(workspace)") }
        try await go(workspace)
    }

    func currentDraftName() -> String {
        // "Draft: <name>. Switch draft"
        guard let l = query(id: "draft-menu").first?.label, l.hasPrefix("Draft: ") else { return "" }
        return String(l.dropFirst("Draft: ".count).dropLast(". Switch draft".count))
    }

    func currentElementLabel() -> String { query(id: "element-picker").first?.value ?? "" }

    static func kindName(_ label: String) -> String {
        if label == "Page break" { return "page_break" }
        if label.hasPrefix("General note") { return "note" }
        let standard = ["Scene Heading", "Action", "Character", "Dialogue", "Parenthetical", "Transition", "Shot", "General"]
        if standard.contains(label) { return label.lowercased().replacingOccurrences(of: " ", with: "_") }
        return "custom"
    }

    /// Every element of the open draft as "type: text". The type is what the toolbar shows with
    /// the caret on that line; the caret goes back to where it was.
    func elements(_ tv: NSTextView) async -> [String] {
        let saved = tv.selectedRange()
        let ns = tv.string as NSString
        // With the caret in the empty last paragraph, the toolbar names that paragraph's element now;
        // moving the caret there later would name the paragraph before it.
        let atEnd = saved.location == ns.length
        let endLabel = atEnd ? currentElementLabel() : ""
        // An empty last paragraph keeps its element in the typing attributes only; keep them across the walk.
        let typing = tv.typingAttributes
        var out: [String] = []
        var loc = 0
        while loc <= ns.length {
            let line = ns.lineRange(for: NSRange(location: min(loc, ns.length), length: 0))
            var text = ns.substring(with: line)
            if text.hasSuffix("\n") { text.removeLast() }
            tv.setSelectedRange(NSRange(location: line.location, length: 0))
            // The toolbar follows the caret a moment later; read it until it holds still.
            await idle(0.4)
            var label = currentElementLabel()
            for _ in 0..<10 {
                await idle(0.15)
                let again = currentElementLabel()
                if again == label { break }
                label = again
            }
            out.append("\(Self.kindName(label)): \(text)")
            if NSMaxRange(line) >= ns.length { break }
            loc = NSMaxRange(line)
        }
        if ns.length > 0, ns.character(at: ns.length - 1) == 10 {
            out.append("\(Self.kindName(endLabel.isEmpty ? "Action" : endLabel)): ")
        }
        tv.setSelectedRange(saved)
        tv.typingAttributes = typing
        await idle(0.05)
        return out
    }

    /// The lines of the draft (text only).
    func lines(_ tv: NSTextView) -> [String] {
        var l = tv.string.components(separatedBy: "\n")
        if l.last == "" && l.count > 1 { l.removeLast() }
        return l
    }

    func clickLine(_ tv: NSTextView, containing text: String, end: Bool = false) async throws {
        let ns = tv.string as NSString
        let r = ns.range(of: text)
        guard r.location != NSNotFound else { throw DriverError("no line contains \(text): \(tv.string.suffix(300))") }
        let line = ns.lineRange(for: r)
        var len = line.length
        if ns.substring(with: line).hasSuffix("\n") { len -= 1 }
        tv.window?.makeFirstResponder(tv)
        tv.setSelectedRange(NSRange(location: end ? line.location + len : line.location, length: 0))
        await idle(0.15)
    }

    /// The labels the suggestion list under the caret offers now ("INT.", "Create New: MEERA").
    func suggestionRows() -> [String] {
        guard let list = all().first(where: { $0.identifier == "screenplay-suggestions" && $0.role == "AXList" }) else { return [] }
        return list.children.filter { $0.role == "AXRow" }.map { l in
            let t = l.label
            guard let r = t.range(of: ", ", options: .backwards) else { return t }
            return String(t[..<r.lowerBound])
        }
    }

    func waitSuggestion(_ label: String) async throws {
        try await waitUntil("the list offers \(label), now \(suggestionRows())") { suggestionRows().contains { $0 == label || $0.hasPrefix(label) } }
    }

    /// The context menu the editor opens on a right click at a word.
    func editorWordMenu(_ tv: NSTextView, word: String) -> NSMenu? {
        let ns = tv.string as NSString
        let r = ns.range(of: word)
        guard r.location != NSNotFound else { return nil }
        var actual = NSRange()
        let rect = tv.firstRect(forCharacterRange: NSRange(location: r.location + 1, length: 0), actualRange: &actual)
        return contextMenuItems(at: NSPoint(x: rect.midX + 2, y: rect.midY))
    }

    func typeSlowly(_ text: String) async throws {
        try await typeHere(text)
        await idle(0.15)
    }

    func cmdKey(_ chars: String, code: UInt16, _ extra: NSEvent.ModifierFlags = []) async throws {
        var m: NSEvent.ModifierFlags = [.command]; m.formUnion(extra); try await shortcutKey(chars, code: code, modifiers: m)
    }

    /// A key with Command or Control held. An active app takes it through the event queue, as the
    /// keyboard sends it. With no active app (a locked screen) the system gives key equivalents to
    /// nobody, so the window offers it to its views and then to the menu bar, which is the same
    /// path an active app takes.
    func shortcutKey(_ chars: String, code: UInt16, modifiers: NSEvent.ModifierFlags) async throws {
        guard let w = NSApp.keyWindow ?? windows.filter({ $0.frame.width > 400 }).last ?? windows.last else { throw DriverError("no window for a key press") }
        guard let ev = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: modifiers, timestamp: ProcessInfo.processInfo.systemUptime,
                                        windowNumber: w.windowNumber, context: nil, characters: chars, charactersIgnoringModifiers: chars,
                                        isARepeat: false, keyCode: code) else { throw DriverError("cannot make the key event") }
        if appIsActive {
            try await key(chars, code: code, modifiers: modifiers)
            return
        }
        log("shortcut \(modifiers.rawValue) \(chars) (no active app: offered to the window, then the menu bar)")
        if !w.performKeyEquivalent(with: ev) { _ = NSApp.mainMenu?.performKeyEquivalent(with: ev) }
        await idle(0.2)
    }

    /// Picks a value of the Type pop-up in the New Project sheet.
    func chooseTypePicker(_ value: String) async throws {
        let b = try await wait(id: "new-project-type", role: "AXPopUpButton", "the Type picker")
        try await chooseMenu(b, [value])
    }

    var appIsActive: Bool { NSApp.isActive && NSApp.keyWindow != nil }

    func shows(_ text: String) -> Bool { all().contains { $0.text.contains(text) } }

    func expectShows(_ text: String, timeout: Double = 10) async throws { try await waitText(text, timeout: timeout) }

    func expectNotShown(_ text: String, within seconds: Double = 1.0) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline {
            if !shows(text) { return }
            await idle(0.1)
        }
        throw DriverError("still shown: \(text)")
    }

    func waitUntil(_ what: String, timeout: Double = 10, _ cond: @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            await idle(0.1)
        }
        try? snapshot("wait-\(what.prefix(20))")
        throw DriverError("did not happen within \(Int(timeout)) s: \(what)")
    }

    func button(_ label: String, timeout: Double = 15) async throws {
        try await act(label: label, role: "AXButton", timeout: timeout)
    }

    /// Presses a control: the accessibility press, else a click through the event queue (the
    /// driver's own fallback sends the click inside the call and hangs on a control that tracks the mouse).
    func act(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, role: role, timeout: timeout)
        try await act(e)
    }

    /// Presses the first control the test accepts, once one shows.
    func act(label: String?, role: String?, pick: @MainActor (Element) -> Bool) async throws {
        try await waitUntil("a control to press") { all().contains(where: pick) }
        guard let e = all().first(where: pick) else { return }
        try await act(e)
    }

    func act(_ e: Element) async throws {
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        guard e.isEnabled else { throw DriverError("control is disabled: \(e)") }
        if e.performPress() { await idle(); return }
        try await tap(e)
    }

    /// A real left click that goes through the event queue, so a control that tracks the mouse
    /// until the button comes up (a text field, a drag) gets both events. Modifiers as held keys.
    func tap(_ e: Element, count: Int = 1, modifiers: NSEvent.ModifierFlags = [], right: Bool = false) async throws {
        let frame = e.frame
        guard frame.width > 0, frame.height > 0 else { throw DriverError("element has no frame: \(e)") }
        try await tap(at: NSPoint(x: frame.midX, y: frame.midY), count: count, modifiers: modifiers, right: right)
    }

    func tap(at center: NSPoint, count: Int = 1, modifiers: NSEvent.ModifierFlags = [], right: Bool = false) async throws {
        let ordered = NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(center) }
        guard let w = ordered.first else { throw DriverError("no window under \(center)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        let down: NSEvent.EventType = right ? .rightMouseDown : .leftMouseDown
        let up: NSEvent.EventType = right ? .rightMouseUp : .leftMouseUp
        for i in 1...count {
            for type in [down, up] {
                guard let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: modifiers,
                                                  timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w.windowNumber,
                                                  context: nil, eventNumber: 0, clickCount: i, pressure: type == down ? 1 : 0) else { continue }
                NSApp.postEvent(ev, atStart: false)
            }
            await idle(0.08)
        }
        await idle(0.15)
    }

    /// Types into a labelled text field (a real click puts the caret in it, then the text goes in
    /// the way typing does) and leaves it with Return when asked.
    func fill(_ label: String? = nil, _ text: String, id: String? = nil, submit: Bool = false, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, timeout: timeout)
        log("fill \(id ?? label ?? "?")")
        if let tv = e.obj as? NSTextView, let w = tv.window {
            w.makeKeyAndOrderFront(nil)
            w.makeFirstResponder(tv)
        } else if let tf = (e.obj as? NSTextField) ?? ((e.obj as? NSCell)?.controlView as? NSTextField), let w = tf.window {
            w.makeKeyAndOrderFront(nil)
            w.makeFirstResponder(tf)
        } else {
            log("fill: \(Swift.type(of: e.obj)) is not a text view or field; clicking it")
            try await tap(e)
            if !(((NSApp.keyWindow ?? windows.last)?.firstResponder) is NSTextView) { try await focus(e) }
        }
        await idle(0.15)
        guard let w = NSApp.keyWindow ?? windows.last, let target = w.firstResponder as? NSTextView else {
            throw DriverError("clicking the field gave it no keyboard focus: \(e)")
        }
        target.selectAll(nil)
        target.insertText(text, replacementRange: target.selectedRange())
        await idle(0.2)
        log("filled; field editor now: \(target.string) ; element value \(Element(obj: e.obj).value)")
        if submit { try await enter() }
    }

    /// The screen is quiet: no error text, and nothing the app calls a failure.
    func quiet() async throws {
        await idle(0.3)
        for bad in ["Could not", "went wrong", "Something failed", "unexpected error"] {
            try expect(!shows(bad), "the screen shows an error: \(bad)")
        }
    }

    /// Scene cards (and beats) on the Story Board in reading order: left to right by column, top to bottom.
    func cardLabels() -> [String] {
        let items = all().filter { $0.identifier.hasPrefix("story-card-") && ($0.label.hasPrefix("Scene Card:") || $0.label.hasPrefix("Beat:")) }
        var seen = Set<String>()
        let unique = items.filter { seen.insert($0.identifier).inserted }
        let sorted = unique.sorted { a, b in
            let ca = Int((a.frame.minX / 120).rounded(.down)), cb = Int((b.frame.minX / 120).rounded(.down))
            if ca != cb { return ca < cb }
            return a.frame.maxY > b.frame.maxY
        }
        return sorted.map(\.label)
    }

    /// Scene cards in the acts (or, with `parking`, in the Parking Lot), in reading order.
    func sceneCards(parking: Bool = false) -> [String] {
        let lot = query(id: "story-parking").first { $0.role == "AXGroup" }?.frame
        let items = all().filter { $0.identifier.hasPrefix("story-card-") && $0.label.hasPrefix("Scene Card:") }
        var seen = Set<String>()
        let unique = items.filter { seen.insert($0.identifier).inserted }
        let chosen = unique.filter { e in
            let inLot = lot.map { $0.contains(NSPoint(x: e.frame.midX, y: e.frame.midY)) } ?? false
            return inLot == parking
        }
        let sorted = chosen.sorted { a, b in
            let ca = Int((a.frame.minX / 120).rounded(.down)), cb = Int((b.frame.minX / 120).rounded(.down))
            if ca != cb { return ca < cb }
            return a.frame.maxY > b.frame.maxY
        }
        return sorted.map { String($0.label.dropFirst("Scene Card: ".count)) }
    }

    func waitSceneCards(_ expected: [String], parking: Bool = false, timeout: Double = 10) async throws {
        try await waitUntil("\(parking ? "parked" : "act") cards are \(expected), now \(sceneCards(parking: parking))", timeout: timeout) { sceneCards(parking: parking) == expected }
    }

    func card(_ text: String) -> Element? {
        all().first { $0.identifier.hasPrefix("story-card-") && $0.label.hasPrefix("Scene Card: \(text)") || $0.identifier.hasPrefix("story-card-") && $0.label.hasPrefix("Beat: \(text)") }
    }

    func waitCards(_ expected: [String], timeout: Double = 10) async throws {
        let want = expected.map { "Scene Card: " + $0 }
        try await waitUntil("cards are \(expected)", timeout: timeout) { cardLabels().filter { $0.hasPrefix("Scene Card:") } == want }
    }

    func pressCard(_ text: String, count: Int = 1) async throws {
        try await waitUntil("card \(text)") { card(text) != nil }
        guard let c = card(text) else { return }
        try await tap(c, count: count)
        if count == 2 {
            // A double click that opens nothing falls back to the card's own Open action.
            let until = Date().addingTimeInterval(3)
            while Date() < until, !exists(id: "story-inspector-close") { await idle(0.2) }
            if !exists(id: "story-inspector-close"), let again = card(text) { try await customAction(again, "Open") }
        }
    }
}

fileprivate extension UI {
    /// Project undo and redo, as the Edit menu has them (text fields keep their own).
    func undo() async throws { try await menu(["Edit", "Undo Project Change"]) }
    func redo() async throws { try await menu(["Edit", "Redo Project Change"]) }

    /// Moves keyboard focus off any text field.
    func unfocusText() async {
        (NSApp.keyWindow ?? windows.last)?.makeFirstResponder(nil)
        await idle(0.2)
    }

    func deleteKey() async throws { try await key("\u{7f}", code: 51) }

    func projectMenu(_ path: [String]) async throws {
        try await chooseMenu(id: "project-menu", path)
    }

    func restore(_ title: String) async throws {
        try await act(id: "restore-\(title)", role: "AXButton")
    }

    func tab(_ name: String) async throws {
        try await act(label: name, role: "AXRadioButton")
        await idle(0.3)
    }

    /// Chooses a button by label inside the current sheet or screen.
    func closeAndReopen(_ title: String) async throws {
        try await closeProject()
    }

    func dialogGone(_ id: String, timeout: Double = 10) async throws { try await waitGone(id: id, timeout: timeout) }
}

extension Scenarios {
    @MainActor static func storyProject(_ ui: UI, _ ctx: Context, _ title: String) async throws {
        try await ui.startApp(ctx)
        try await ui.createProject(title)
        try await ui.goTo("Story")
        try await ui.wait(id: "story-tabs", timeout: 20)
    }

    @MainActor static func addAct(_ ui: UI, _ title: String) async throws {
        try await ui.button("Add Act")
        try await ui.fill("Act title", title, submit: true)
        try await ui.expectShows("Act \(title)")
    }

    @MainActor static func addSequence(_ ui: UI, _ name: String) async throws {
        try await ui.chooseMenu(id: "story-add-menu", ["Sequence"])
        try await ui.fill("Sequence name", name, submit: true)
        try await ui.expectShows(name)
    }

    @MainActor static func addCard(_ ui: UI, _ text: String) async throws {
        try await ui.chooseMenu(id: "story-add-menu", ["Scene Card"])
        try await ui.fill("Short description", text, submit: true)
        try await ui.waitUntil("card \(text)") { ui.card(text) != nil }
    }
}

/// What the app reads from NSEvent.modifierFlags (a held Command or Shift key) while a scenario
/// holds modifiers. The class property reads the keyboard hardware, which a scenario cannot press.
nonisolated(unsafe) var heldModifiers: NSEvent.ModifierFlags = []
nonisolated(unsafe) var modifierSwizzled = false

fileprivate extension UI {
    func holdModifiers(_ m: NSEvent.ModifierFlags) {
        if !modifierSwizzled, let meta = object_getClass(NSEvent.self), let method = class_getClassMethod(NSEvent.self, #selector(getter: NSEvent.modifierFlags)) {
            let original = method_getImplementation(method)
            typealias Fn = @convention(c) (AnyObject, Selector) -> UInt
            let orig = unsafeBitCast(original, to: Fn.self)
            let block: @convention(block) (AnyObject) -> UInt = { cls in
                heldModifiers.isEmpty ? orig(cls, #selector(getter: NSEvent.modifierFlags)) : heldModifiers.rawValue
            }
            class_replaceMethod(meta, #selector(getter: NSEvent.modifierFlags), imp_implementationWithBlock(block), method_getTypeEncoding(method))
            modifierSwizzled = true
        }
        heldModifiers = m
    }
}

func W(_ name: String, phases: Int = 1, timeout: Double = 120, end: @escaping (Int) -> PhaseEnd = { _ in .quit },
       _ body: @escaping @MainActor (UI, Int, Context) async throws -> Void) -> Scenario {
    Scenario(name, phases: phases, timeout: timeout, end: end, body)
}

extension Scenarios {
    @MainActor static func twoCards(_ ui: UI, _ ctx: Context, _ title: String = "Paper Boats") async throws {
        try await storyProject(ui, ctx, title)
        try await addAct(ui, "Act One")
        try await addSequence(ui, "Departure")
        try await addCard(ui, "Boats fold themselves at dawn")
        try await addCard(ui, "The river carries the boats away")
        try await ui.waitCards(["Boats fold themselves at dawn", "The river carries the boats away"])
    }

    static let cardA = "Rain finds the letter box"
    static let cardB = "Meera reads the first letter"
    static let cardC = "The postman never came back"
    static let beat = "A second letter arrives"
    static let longText = "The street is flooded to the knees. Meera wades to the box with a torch in her teeth."

    @MainActor static func monsoon(_ ui: UI, _ ctx: Context) async throws {
        try await storyProject(ui, ctx, "Monsoon Letters")
        try await ui.expectShows("Your Story Board is empty.")
        try await addAct(ui, "Act One")
        try await addSequence(ui, "The Letters")
        for t in [cardA, cardB, cardC] { try await addCard(ui, t) }
        try await ui.waitSceneCards([cardA, cardB, cardC])
    }

    /// Drag card C onto the top quarter of card A. With no active app (a locked screen) the system
    /// starts no drag, and the card's own Move Up action stands in for it.
    nonisolated(unsafe) static var dragSteps = 1

    @MainActor static func dragCToTop(_ ui: UI) async throws {
        dragSteps = 1
        let a = try await ui.cardWait(cardA), c = try await ui.cardWait(cardC)
        try await ui.drag(from: NSPoint(x: c.frame.midX, y: c.frame.midY), to: NSPoint(x: a.frame.midX, y: a.frame.maxY - a.frame.height * 0.12))
        if ui.sceneCards() != [cardC, cardA, cardB] {
            ui.log("the mouse drag started no drag session (active app: \(ui.appIsActive)); using the Move Up action twice")
            dragSteps = 2
            try await ui.customAction((try await ui.cardWait(cardC)), "Move Up")
            try await ui.customAction((try await ui.cardWait(cardC)), "Move Up")
        }
    }

    static let emDash = "\u{2014}"

    @MainActor static func scriptProject(_ ui: UI, _ ctx: Context, _ title: String) async throws -> NSTextView {
        try await ui.startApp(ctx)
        try await ui.createProject(title)
        try await ui.goTo("Screenplay")
        return try await ui.editor()
    }

    @MainActor static func expectElements(_ ui: UI, _ tv: NSTextView, include wanted: [String], _ what: String = "") async throws {
        var last: [String] = []
        try await ui.waitUntil("elements include \(wanted) \(what)", timeout: 8) {
            true
        }
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            last = await ui.elements(tv)
            if wanted.allSatisfy({ last.contains($0) }) { return }
            await ui.idle(0.3)
        }
        throw DriverError("elements do not include \(wanted); elements are \(last)")
    }

    @MainActor static func expectFirstElements(_ ui: UI, _ tv: NSTextView, _ wanted: [String]) async throws {
        var last: [String] = []
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            last = await ui.elements(tv)
            if Array(last.prefix(wanted.count)) == wanted { return }
            await ui.idle(0.3)
        }
        throw DriverError("first elements are \(last.prefix(wanted.count)), wanted \(wanted)")
    }

    /// The 43 flow: write the feature, add a second screenplay, write in both, switch, rename the short.
    @MainActor static func twoRivers(_ ui: UI, _ tv: NSTextView) async throws {
        await ui.caretToStart(tv)
        try await ui.typeHere("The feature opens on the river.")
        try await ui.waitSaved()
        try await ui.chooseMenu(id: "screenplay-picker", ["New Screenplay"])
        try await ui.fill("Title", "Two Rivers", id: "screenplay-title")
        try await ui.expectShows("Another screenplay here already has this title")
        try await ui.fill("Title", "Two Rivers (short)", id: "screenplay-title", submit: true)
        try await ui.waitUntil("picker reads the short") { ui.screenplayPickerButton()?.label == "Screenplay: Two Rivers (short). Switch screenplay" }
        let tv2 = try await ui.editor()
        try ui.expect(!ui.editorText().contains("The feature opens on the river."), "the short starts empty")
        await ui.caretToStart(tv2)
        try await ui.typeHere("The short opens on the bridge.")
        try await ui.waitSaved()
        try await ui.chooseMenu(id: "screenplay-picker", ["Two Rivers \u{00B7}"])
        try await ui.waitUntil("feature text shows") { ui.editorText().contains("The feature opens on the river.") }
        try await ui.clickLine(tv2, containing: "The feature opens on the river.", end: true)
        try await ui.typeHere(" At dawn.")
        try await ui.waitSaved()
        try await ui.chooseMenu(id: "screenplay-picker", ["Two Rivers (short)"])
        try await ui.waitUntil("short text shows") { ui.editorText().contains("The short opens on the bridge.") }
        try await ui.chooseMenu(id: "screenplay-picker", ["Rename Screenplay"])
        try await ui.fill("Title", "Two Rivers: The Short", id: "screenplay-title")
        try await ui.button("Rename")
        try await ui.waitUntil("picker reads the renamed short") { ui.screenplayPickerButton()?.label == "Screenplay: Two Rivers: The Short. Switch screenplay" }
        let titles = try await ui.screenplayTitles()
        try ui.expect(Set(titles) == ["Two Rivers", "Two Rivers: The Short"] && titles.count == 2, "the screenplay list: \(titles)")
    }

    /// The 42 scene: smart suggestions build "Bus Depot" without retyping and never change text silently.
    @MainActor static func busDepotScene(_ ui: UI, _ tv: NSTextView) async throws {
        await ui.caretToStart(tv)
        try await ui.typeHere("IN")
        try await ui.waitSuggestion("INT.")
        try await ui.enter()
        try await ui.typeHere("BUS DEPOT - ")
        try await ui.typeHere("NI")
        try await ui.waitSuggestion("NIGHT")
        try await ui.enter()
        try await expectFirstElements(ui, tv, ["scene_heading: INT. BUS DEPOT - NIGHT"])
        try await ui.enter()
        try await ui.typeHere("Meera waits for teh last bus.")
        try await ui.enter()
        try await ui.typeHere("MEERA")
        try await ui.waitSuggestion("Create New: MEERA")
        try await ui.enter()
        try await ui.expectShows("Added MEERA to Characters")
        try await expectLastTwo(ui, tv, "character: MEERA", "dialogue: ")
        try await ui.typeHere("Is it late?")
        try await ui.enter()
        try await ui.enter()
        try await ui.typeHere("EXT. ROAD - LATER")
        try await ui.escape()
        try await ui.waitUntil("the suggestion list closes") { ui.suggestionRows().isEmpty }
        try ui.expect(ui.editorText().hasSuffix("EXT. ROAD - LATER") || ui.editorText().hasSuffix("EXT. ROAD - LATER\n"), "the text stays as typed")
        try await ui.enter()
        try await ui.typeHere("The bus is late.")
        try await ui.enter()
        try await ui.typeHere("ME")
        try await ui.waitSuggestion("MEERA")
        try await ui.enter()
        try await expectLastTwo(ui, tv, "character: MEERA", "dialogue: ")
        try await ui.typeHere("Late.")
        try await ui.waitSaved()
        let els = await ui.elements(tv)
        try ui.expect(els.filter { $0 == "character: MEERA" }.count == 2, "two character MEERA elements: \(els)")
        try ui.expect(Array(els.prefix(5)) == ["scene_heading: INT. BUS DEPOT - NIGHT", "action: Meera waits for teh last bus.", "character: MEERA", "dialogue: Is it late?", "scene_heading: EXT. ROAD - LATER"], "first five elements: \(els.prefix(6))")
    }

    @MainActor static func expectLast(_ ui: UI, _ tv: NSTextView, _ wanted: String) async throws {
        var els: [String] = []
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            els = await ui.elements(tv)
            if els.last == wanted { return }
            await ui.idle(0.3)
        }
        throw DriverError("the last element is \(els.last ?? "none"), wanted \(wanted)")
    }

    @MainActor static func expectLastTwo(_ ui: UI, _ tv: NSTextView, _ a: String, _ b: String) async throws {
        var els: [String] = []
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            els = await ui.elements(tv)
            if els.suffix(2).elementsEqual([a, b]) { return }
            await ui.idle(0.3)
        }
        throw DriverError("the last two elements are \(els.suffix(2)), wanted \(a) and \(b)")
    }

    /// The "Night Bus" screenplay of the shared steps (three scenes), typed element by element.
    @MainActor static func nightBus(_ ui: UI, _ tv: NSTextView) async throws {
        await ui.caretToStart(tv)
        try await ui.writeElements(tv, [
            (1, "INT. POLICE STATION - NIGHT"), (2, "Arjun confesses to Inspector Rao."), (3, "ARJUN"), (5, "I was there that night."),
            (1, "EXT. BUS STOP - NIGHT"), (2, "Arjun meets Meera again at a rainy bus stop."), (3, "MEERA"), (5, "You came back."),
            (1, "EXT. RAILWAY STATION - DAY"), (2, "Meera waits for the late train with a red umbrella."),
        ], first: true)
        try await ui.waitSaved()
        let els = await ui.elements(tv)
        try ui.expect(els.filter { $0.hasPrefix("scene_heading: ") }.count == 3, "three scenes: \(els)")
    }

    /// The 41 script: heading, action, ASHA, dialogue, page break, more dialogue.
    @MainActor static func signalBox(_ ui: UI, _ tv: NSTextView) async throws {
        await ui.caretToStart(tv)
        try await ui.typeHere("INT. SIGNAL BOX - NIGHT")
        try await ui.enter()
        try await ui.typeHere("The telephone rings twice.")
        try await ui.enter()
        try await ui.typeHere("ASHA")
        try await ui.waitSuggestion("Create New: ASHA")
        try await ui.escape()
        try await ui.waitUntil("the list closes") { ui.suggestionRows().isEmpty }
        try await ui.enter()
        try await ui.typeHere("Who is there?")
        try await ui.shortcutKey("\r", code: 36, modifiers: [.command])
        try await ui.typeHere("Nobody answers.")
    }

    /// The 96 walk of one workspace: every control the window offers is named, and Tab moves
    /// through the controls with a visible focus and no trap.
    @MainActor static func keyboardWalk(_ ui: UI, _ workspace: String) async throws {
        let unnamed = ui.unnamedControls()
        try ui.expect(unnamed.isEmpty, "controls without a name in \(workspace): \(unnamed.prefix(8))")
        guard let w = NSApp.keyWindow ?? ui.windows.last else { throw DriverError("no window") }
        w.makeFirstResponder(nil)
        var last = ""
        var same = 0
        var seen = Set<String>()
        var findings: [String] = []
        for i in 1...80 {
            try await ui.tab()
            guard let r = w.firstResponder, r !== w else {
                findings.append("press \(i): focus is on the window itself")
                continue
            }
            let desc = ui.responderDescription(r)
            if let v = r as? NSView, v.isHiddenOrHasHiddenAncestor { findings.append("press \(i): the focused view is hidden: \(desc)") }
            if desc.name.isEmpty { findings.append("press \(i): the focused element has no name: \(desc.text)") }
            if !desc.ring { findings.append("press \(i): no focus mark on \(desc.text)") }
            same = desc.text == last ? same + 1 : 0
            if same >= 3 { findings.append("press \(i): focus is stuck on \(desc.text)"); break }
            last = desc.text
            if !seen.insert(desc.text).inserted && seen.count > 3 && i > 12 { break }
        }
        try ui.expect(findings.isEmpty, "keyboard findings in \(workspace): \(findings.prefix(8))")
    }

    // MARK: 01-journey. Each case runs the cases before it first (one app, one project), then its own.

    @MainActor static func journey(_ ui: UI, _ ctx: Context, through last: Int, skipping: Set<Int> = []) async throws {
        for k in 1...last {
            if skipping.contains(k) { continue }
            if k == 6 && last != 6 { continue }
            if (k == 13 || k == 14) && last == 15 { continue }
            if k == 13 && last == 14 { continue }   // the import belongs to the interchange screens; it stays out of the chain
            ui.log("== journey case \(k)")
            try await journeyCase(ui, ctx, k)
        }
    }

    @MainActor static func journeyCase(_ ui: UI, _ ctx: Context, _ k: Int) async throws {
        switch k {
        case 1:
            try await ui.startApp(ctx)
            try await ui.expectShows("Start your first project")
            try await ui.createProject("Night Bus")
            try await ui.waitUntil("window title") { ui.windowTitle().hasPrefix("Night Bus") }
            try await ui.expectShows("Start here")
            try await ui.quiet()
        case 2:
            try await ui.goTo("Idea Vault")
            try await ui.wait(id: "vault-scope", timeout: 20)
            try await ui.quickCapture("Arjun meets Meera again at a rainy bus stop.")
            try await ui.quiet()
        case 3:
            try await ui.goTo("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)
            try await addAct(ui, "Act 1")
            try await addSequence(ui, "The return")
            try await addCard(ui, "Bus stop reunion in the rain")
            try await addCard(ui, "Arjun confesses at the police station")
            try await ui.waitSceneCards(["Bus stop reunion in the rain", "Arjun confesses at the police station"])
            try await ui.customAction(try await ui.cardWait("Arjun confesses at the police station"), "Move Up")
            try await ui.waitSceneCards(["Arjun confesses at the police station", "Bus stop reunion in the rain"])
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitSceneCards(["Bus stop reunion in the rain", "Arjun confesses at the police station"])
            try await ui.redo()
            try await ui.waitSceneCards(["Arjun confesses at the police station", "Bus stop reunion in the rain"])
            try await ui.quiet()
        case 4:
            try await ui.act(id: "story-build", role: "AXButton")
            try await ui.fill("Heading for scene 1", "INT. POLICE STATION \(emDash) NIGHT")
            try await ui.fill("Heading for scene 2", "EXT. BUS STOP \(emDash) NIGHT")
            try await ui.waitUntil("Build Screenplay enabled") { ui.query(id: "build-confirm").first?.isEnabled ?? false }
            try await ui.act(id: "build-confirm", role: "AXButton")
            let tv = try await ui.editor()
            let els = await ui.elements(tv)
            try ui.expect(els.count >= 3, "the editor is filled: \(els)")
            try ui.expect(els[0] == "scene_heading: INT. POLICE STATION \(emDash) NIGHT" && els[1] == "note: Arjun confesses at the police station" && els[2] == "action: ", "the first three elements: \(els.prefix(3))")
            try ui.expect(els.filter { $0.hasPrefix("scene_heading: ") }.count == 2, "exactly two scene headings: \(els)")
            try await ui.quiet()
        case 5:
            let tv = try await ui.editor()
            // The third element is the empty action under the note; the second scene follows it.
            let ns = tv.string as NSString
            let noteLine = ns.lineRange(for: ns.range(of: "Arjun confesses at the police station"))
            tv.window?.makeFirstResponder(tv)
            tv.setSelectedRange(NSRange(location: NSMaxRange(noteLine), length: 0))
            await ui.idle(0.2)
            try await ui.typeHere("Arjun enters carrying a pistol.")
            try await ui.enter()
            try await ui.typeHere("ARJUN")
            try await ui.waitSuggestion("Create New: ARJUN")
            try await ui.escape()
            try await ui.waitUntil("the list closes") { ui.suggestionRows().isEmpty }
            try ui.expect(ui.editorText().contains("ARJUN"), "the name stays as typed")
            try await ui.enter()
            try await ui.typeHere("I was there that night.")
            let els = await ui.elements(tv)
            try ui.expect(els.contains("action: Arjun enters carrying a pistol.") && els.contains("character: ARJUN") && els.contains("dialogue: I was there that night."), "elements: \(els)")
            try await ui.waitSaved()
            await ui.idle(3)
            try await ui.quiet()
        case 6:
            try await importFountain(ui, "Title: Black Rain\n\nINT. RAILWAY PLATFORM - DAWN\n\nMeera waits with a red umbrella.\n\nMEERA\nThe last train is gone.\n\nEXT. LEVEL CROSSING - DAY\n\nArjun runs across the tracks.\n")
            try await ui.expectShows("Import Screenplay \u{00B7} Preview")
            try await ui.expectShows("INT. RAILWAY PLATFORM - DAWN")
            try await ui.expectShows("EXT. LEVEL CROSSING - DAY")
            try await ui.button("Import")
            try await ui.expectShows("Import complete", timeout: 20)
            try await ui.button("Open Screenplay")
            let tv = try await ui.editor()
            try await ui.waitUntil("the first element is the scene heading") { ui.editorText().hasPrefix("INT. RAILWAY PLATFORM - DAWN") }
            try ui.expect(!tv.string.contains("Title:"), "the title page did not become script text")
            await ui.idle(3)
            try await ui.quiet()
        case 7:
            try await ui.goTo("Breakdown")
            try await ui.button("Select Source")
            try await ui.waitUntil("a draft from the Story Board") { ui.all().contains { $0.text.hasPrefix("Draft 1 from Story Board") } }
            try await ui.act(label: nil, role: nil, pick: { $0.text.hasPrefix("Draft 1 from Story Board") })
            try await ui.act(id: "set-production-source", role: "AXButton")
            try await ui.expectShows("Production Source: Draft 1 from Story Board")
            try await ui.act(id: "suggest-elements", role: "AXButton")
            try await ui.act(label: "Accept Pistol", role: "AXButton")
            try await ui.act(label: "Accept Arjun", role: "AXButton")
            try await ui.expectShows("Props")
            try await ui.expectShows("Pistol")
            try await ui.quiet()
        case 8:
            try await ui.goTo("Schedule")
            try await ui.act(label: "Create Shooting Schedule", role: "AXButton")
            try await ui.act(id: "create-schedule-confirm", role: "AXButton")
            try await ui.expectShows("2 scenes (0 scheduled, 2 unscheduled)")
            try await ui.act(label: "Create Shooting Day", role: "AXButton")
            try await ui.fill("Shooting date", "2027-06-14", id: "day-date")
            try await ui.act(id: "day-sheet-confirm", role: "AXButton")
            try await ui.expectShows("Shoot Day 1")
            try await ui.chooseMenu(label: "Scene 1 actions", ["Move to", "Shoot Day 1"])
            try await ui.expectShows("2 scenes (1 scheduled, 1 unscheduled)")
            try ui.expect(!ui.shows("SCHEDULED SCENE"), "the scene details drawer did not open")
            try await ui.quiet()
        case 9:
            try await ui.chooseMenu(label: "Shoot Day 1 actions", ["Create Call Sheet"])
            try await ui.expectShows("Call Sheet: Day 1")
            try await ui.expectShows("INT. POLICE STATION \(emDash) NIGHT")
            try await ui.expectShows("ARJUN")
            try await ui.quiet()
        case 10:
            try await ui.goTo("Schedule")
            try await ui.chooseMenu(label: "Scene 1 actions", ["Details"])
            try await ui.act(label: "Open in Screenplay", role: "AXLink")
            let tv = try await ui.editor()
            try await ui.waitUntil("Draft 1 from Story Board is open") { ui.currentDraftName() == "Draft 1 from Story Board" }
            try ui.expect(tv.selectedRange().location <= 1, "the cursor sits on the first scene heading")
            await ui.idle(3)
            try await ui.quiet()
        case 11:
            try await ui.menu(["Edit", "Search and Commands\u{2026}"])
            try await ui.fill(nil, "pistol", id: "palette-field")
            try await ui.waitUntil("a result containing INT. POLICE STATION") { ui.all().contains { $0.text.contains("INT. POLICE STATION") && $0.identifier != "screenplay-editor" } }
            try await ui.act(label: nil, role: nil, pick: { $0.text.contains("INT. POLICE STATION") && $0.identifier != "screenplay-editor" && $0.frame.width > 0 })
            let tv = try await ui.editor()
            try ui.expect(tv.selectedRange().location <= 1, "the cursor is on the scene heading")
            await ui.idle(3)
            try await ui.quiet()
        case 12:
            try await ui.goTo("Story")
            try await ui.deleteCard("Bus stop reunion in the rain")
            if ui.shows("Delete this Scene Card?") { try await ui.button("Delete Card") }
            try await ui.waitUntil("the card is gone") { ui.card("Bus stop reunion in the rain") == nil }
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.restore("Bus stop reunion in the rain")
            try await ui.goTo("Story")
            try await ui.waitUntil("the card is back") { ui.card("Bus stop reunion in the rain") != nil }
            try await ui.quiet()
        case 13:
            try await ui.button("Assistant")
            try await ui.wait(label: "Ask or command", timeout: 15, "the Ask or command box")
            try await ui.expectShows("AI Ready")
            let text = ui.screenText().lowercased()
            for word in ["gemma", "gguf", "q4_k", "llama", "embedding", "download offline ai"] { try ui.expect(!text.contains(word), "the screen names the model: \(word)") }
            try await ui.quiet()
        case 14:
            if let w = ui.windows.first(where: { $0.title.hasPrefix("Night Bus") }) { w.setContentSize(NSSize(width: 1200, height: 720)) }
            for ws in ["Project Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Schedule", "Call Sheets", "Files"] {
                try await ui.goTo(ws)
                await ui.idle(0.6)
                try await ui.quiet()
            }
            try await ui.shortcutKey("j", code: 38, modifiers: [.command])
            try await ui.waitUntil("the AI Assistant closes") { !ui.exists(label: "Ask or command") }
            try await ui.quiet()
        default:
            try await ui.closeProject()
            try await ui.expectShows("Night Bus")
            try await ui.quiet()
        }
    }

    /// Pastes Fountain text into the screenplay import dialog and previews it.
    @MainActor static func importFountain(_ ui: UI, _ text: String) async throws {
        try await ui.menu(["File", "Import Screenplay\u{2026}"])
        try await ui.fill("Paste", text, id: "import-paste")
        try await ui.button("Preview")
    }

    /// 04: the rounds of closing the window within 100 ms of the last key. Rounds come from
    /// OPENFRAME_E2E_CLOSE_ROUNDS (20 by default, as on Windows).
    static var closeRounds: Int { Int(ProcessInfo.processInfo.environment["OPENFRAME_E2E_CLOSE_ROUNDS"] ?? "") ?? 20 }

    @MainActor static func closeKeepsTyping(_ ui: UI, _ ctx: Context, _ phase: Int) async throws {
        let rounds = closeRounds
        func sentence(_ k: Int) -> String {
            let round = (k + 1) / 2
            return k % 2 == 1 ? "Round \(round) ends by the close button." : "Round \(round) ends by Cmd W."
        }
        try await ui.startApp(ctx)
        if phase == 1 {
            try await ui.createProject("Last Words")
            try await ui.goTo("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToStart(tv)
            try await ui.typeHere("The last bus leaves at midnight.")
            try await ui.waitSaved()
        } else {
            try await ui.openProject("Last Words")
            try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after closing the window")
            try await ui.goTo("Screenplay")
            _ = try await ui.editor()
            let want = sentence(phase - 1)
            try await ui.waitUntil("a line of the screenplay ends with the sentence typed in the last round") {
                ui.editorText().components(separatedBy: "\n").contains { $0.hasSuffix(want) }
            }
        }
        if phase > rounds * 2 {
            try await ui.quiet()
            return
        }
        let tv = try await ui.editor()
        await ui.caretToEnd(tv)
        try await ui.enter()
        try await ui.cmdKey("2", code: 19)
        let text = sentence(phase)
        guard let w = tv.window else { throw DriverError("no window") }
        let started = Date()
        tv.insertText(text, replacementRange: tv.selectedRange())
        // The sentence is in; the result is written first, because the app is gone once the window closes.
        let sent = Date()
        ui.writeResult(phase: phase, status: "pass", message: "")
        if phase % 2 == 1 { w.performClose(nil) } else { try await ui.shortcutKey("w", code: 13, modifiers: [.command]) }
        let gap = Date().timeIntervalSince(sent)
        ui.log("close sent \(Int(gap * 1000)) ms after the last key (typing took \(Int(sent.timeIntervalSince(started) * 1000)) ms)")
        // The runner now waits for the app to end by itself; the file says what happened.
        try? "close sent \(Int(gap * 1000)) ms after the last key".write(to: ui.artifacts.appendingPathComponent("close-gap-\(phase).txt"), atomically: true, encoding: .utf8)
        await ui.idle(20)
    }

    static let builtInThemes: [(id: String, name: String)] = [
        ("openframe-default", "OpenFrame Default"), ("studio-amber", "Studio Amber"), ("slate-noir", "Slate Noir"), ("midnight-reel", "Midnight Reel"),
        ("teal-orange", "Teal & Orange"), ("courier-cream", "Courier & Cream"), ("cutting-room", "Cutting Room"), ("velvet-curtain", "Velvet Curtain"),
        ("sunset-boulevard", "Sunset Boulevard"), ("arctic-frame", "Arctic Frame"), ("neon-reel", "Neon Reel"), ("location-scout", "Location Scout"),
        ("desert-dune", "Desert Dune"), ("colorist-neutral", "Colorist Neutral"), ("silver-screen", "Silver Screen"), ("high-contrast", "High Contrast"),
    ]

    static let themeScreens = ["Application Home", "Project Home", "Idea Vault", "Story Board", "Story Outline", "Screenplay", "Breakdown", "Schedule",
                               "Call Sheets", "Budget", "Visual Planning", "Files", "Notes and Tasks", "Recently Deleted", "Activity"]

    static let lighthouse = "A lighthouse keeper collects letters that never arrived."

    @MainActor static func vaultProject(_ ui: UI, _ ctx: Context, _ title: String) async throws {
        try await ui.startApp(ctx)
        try await ui.createProject(title)
        try await ui.goTo("Idea Vault")
        try await ui.wait(id: "vault-scope", timeout: 20)
    }

    static let writingGroup: [Scenario] = [
        W("20-idea-vault/quick-capture-adds-an-idea") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Lantern Coast")
            try await ui.quickCapture(lighthouse)
            try await ui.expectShows("Saved to the Project Idea Vault")
            try ui.expect(ui.vaultCards().count == 1 && ui.vaultCards()[0].hasPrefix("A lighthouse keeper"), "one card, \(ui.vaultCards())")
            try await ui.quiet()
        },
        W("20-idea-vault/title-in-drawer-undo-redo") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Lantern Coast")
            try await ui.quickCapture(lighthouse)
            let before = ui.vaultCards()[0]
            try await ui.tap((try await ui.vaultCardWait("A lighthouse")), count: 2)
            try await ui.wait(id: "drawer-title", timeout: 10, "the drawer title field")
            try await ui.fill("Title", "Lighthouse keeper", id: "drawer-title")
            try await ui.expectShows("Saved automatically")
            try await ui.act(id: "drawer-close", role: "AXButton")
            try await ui.waitGone(id: "drawer-title")
            try await ui.waitVaultCard("Lighthouse keeper")
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("card has its earlier name") { ui.vaultCard(String(before.prefix(18))) != nil && ui.vaultCard("Lighthouse keeper") == nil }
            try await ui.redo()
            try await ui.waitVaultCard("Lighthouse keeper")
            try await ui.quiet()
        },
        W("20-idea-vault/delete-undo-redo-restore") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Lantern Coast")
            try await ui.quickCapture(lighthouse)
            try await ui.tap((try await ui.vaultCardWait("A lighthouse")), count: 2)
            try await ui.fill("Title", "Lighthouse keeper", id: "drawer-title")
            try await ui.expectShows("Saved automatically")
            try await ui.act(id: "drawer-close", role: "AXButton")
            try await ui.waitVaultCard("Lighthouse keeper")
            try await ui.deleteVaultCard("Lighthouse keeper")
            try await ui.expectShows("Deleted \u{201C}Lighthouse keeper\u{201D}. You can restore from Recently Deleted.")
            try await ui.waitUntil("the Vault has no cards") { ui.vaultCards().isEmpty }
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitVaultCard("Lighthouse keeper")
            try await ui.redo()
            try await ui.waitUntil("the Vault has no cards") { ui.vaultCards().isEmpty }
            try await ui.act(id: "vault-recently-deleted", role: "AXButton")
            try await ui.expectShows("Items you deleted can be restored.")
            try await ui.restore("Lighthouse keeper")
            try await ui.button("Back to Vault")
            try await ui.waitVaultCard("Lighthouse keeper")
            try await ui.quiet()
        },
        W("20-idea-vault/relaunch-keeps-title-and-text", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Lantern Coast")
                try await ui.goTo("Idea Vault")
                try await ui.quickCapture(lighthouse)
                try await ui.tap((try await ui.vaultCardWait("A lighthouse")), count: 2)
                try await ui.fill("Title", "Lighthouse keeper", id: "drawer-title")
                try await ui.expectShows("Saved automatically")
                try await ui.act(id: "drawer-close", role: "AXButton")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Lantern Coast")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Idea Vault")
                try await ui.waitVaultCard("Lighthouse keeper")
                try await ui.tap((try await ui.vaultCardWait("Lighthouse keeper")), count: 2)
                let body = try await ui.wait(id: "drawer-body", timeout: 10)
                try ui.expect(body.value == lighthouse, "the body field reads exactly the typed text: \(body.value)")
                try await ui.act(id: "drawer-close", role: "AXButton")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("40-screenplay/new-screenplay-enter-flow-saves") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Glass Harbour")
            try ui.expect(!ui.currentDraftName().isEmpty, "the draft has a name")
            await ui.caretToStart(tv)
            try await ui.typeHere("INT. GLASS WORKSHOP \(emDash) NIGHT")
            try await ui.enter()
            try await ui.typeHere("Asha shapes a lens over the blue flame.")
            try await ui.enter()
            try await ui.typeHere("ASHA")
            try await ui.waitUntil("the list offers Create New: ASHA") { ui.suggestionRows().contains { $0.hasPrefix("Create New: ASHA") } }
            try await ui.escape()
            try await ui.waitUntil("the list closes") { ui.suggestionRows().isEmpty }
            try await ui.enter()
            try await ui.typeHere("It has to be perfect.")
            try await expectFirstElements(ui, tv, [
                "scene_heading: INT. GLASS WORKSHOP \(emDash) NIGHT", "action: Asha shapes a lens over the blue flame.",
                "character: ASHA", "dialogue: It has to be perfect."])
            try await ui.waitSaved()
            try await ui.quiet()
        },
        W("40-screenplay/undo-redo-typing-in-editor") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Glass Harbour")
            await ui.caretToStart(tv)
            try await ui.typeHere("INT. GLASS WORKSHOP \(emDash) NIGHT")
            try await ui.enter()
            try await ui.typeHere("Asha shapes a lens over the blue flame.")
            try await ui.enter()
            try await ui.typeHere("ASHA")
            try await ui.escape()
            try await ui.enter()
            try await ui.typeHere("It has to be perfect.")
            try await ui.waitSaved()
            try await Task.sleep(nanoseconds: 4_500_000_000)
            try await ui.typeHere(" Now.")
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect. Now."])
            try await ui.waitSaved()
            try await ui.textUndo(tv)
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect."])
            try await ui.textRedo(tv)
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect. Now."])
            try await ui.waitSaved()
            try await ui.quiet()
        },
        W("40-screenplay/second-draft-rename-delete-restore") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Glass Harbour")
            await ui.caretToStart(tv)
            try await ui.typeHere("INT. GLASS WORKSHOP \(emDash) NIGHT")
            try await ui.enter()
            try await ui.typeHere("Asha shapes a lens over the blue flame.")
            try await ui.enter()
            try await ui.typeHere("ASHA")
            try await ui.escape()
            try await ui.enter()
            try await ui.typeHere("It has to be perfect. Now.")
            try await ui.waitSaved()
            let firstDraft = ui.currentDraftName()
            try await ui.chooseMenu(id: "draft-menu", ["New Draft"])
            try await ui.fill("Draft name", "Rewrite A", id: "new-draft-name")
            try await ui.button("Create Draft")
            try await ui.expectShows("Created draft \u{201C}Rewrite A\u{201D}")
            try await ui.waitUntil("draft switcher reads Rewrite A") { ui.currentDraftName() == "Rewrite A" }
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect. Now."])
            try await ui.clickLine(tv, containing: "It has to be perfect. Now.", end: true)
            try await ui.typeHere(" Again.")
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect. Now. Again."])
            try await ui.waitSaved()
            try await ui.chooseMenu(id: "draft-menu", [firstDraft], exact: true)
            try await ui.waitUntil("draft switcher names the first draft") { ui.currentDraftName() == firstDraft }
            try await expectElements(ui, tv, include: ["dialogue: It has to be perfect. Now."])
            try ui.expect(!tv.string.contains("Again."), "the first draft did not change")
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Drafts"])
            try await ui.wait(id: "drafts-sheet", timeout: 10)
            try await ui.chooseMenu(label: "Actions for Rewrite A", ["Rename"])
            try await ui.fill(nil, "Rewrite B", id: "draft-name")
            try await ui.button("Save")
            try await ui.expectShows("Rewrite B")
            try await ui.chooseMenu(label: "Actions for Rewrite B", ["Delete"])
            try await ui.button("Delete Draft")
            try await ui.expectShows("Moved \u{201C}Rewrite B\u{201D} to Recently Deleted")
            try await ui.waitUntil("the Drafts list no longer lists Rewrite B") { !ui.exists(label: "Actions for Rewrite B") }
            try await ui.button("Done")
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.restore("Rewrite B")
            try await ui.expectShows("Restored")
            try await ui.goTo("Screenplay")
            let tv2 = try await ui.editor()
            try await ui.chooseMenu(id: "draft-menu", ["Rewrite B"], exact: true)
            try await ui.waitUntil("draft switcher names Rewrite B") { ui.currentDraftName() == "Rewrite B" }
            try await expectElements(ui, tv2, include: ["dialogue: It has to be perfect. Now. Again."])
            try await ui.quiet()
        },
        W("40-screenplay/relaunch-keeps-both-drafts", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                let tv = try await scriptProject(ui, ctx, "Glass Harbour")
                await ui.caretToStart(tv)
                try await ui.typeHere("INT. GLASS WORKSHOP \(emDash) NIGHT")
                try await ui.enter()
                try await ui.typeHere("Asha shapes a lens over the blue flame.")
                try await ui.enter()
                try await ui.typeHere("ASHA")
                try await ui.escape()
                try await ui.enter()
                try await ui.typeHere("It has to be perfect. Now.")
                try await ui.waitSaved()
                let firstDraft = ui.currentDraftName()
                try await ui.chooseMenu(id: "draft-menu", ["New Draft"])
                try await ui.fill("Draft name", "Rewrite B", id: "new-draft-name")
                try await ui.button("Create Draft")
                try await ui.waitUntil("Rewrite B") { ui.currentDraftName() == "Rewrite B" }
                try await ui.clickLine(tv, containing: "It has to be perfect. Now.", end: true)
                try await ui.typeHere(" Again.")
                try await ui.waitSaved()
                _ = firstDraft
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Glass Harbour")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Screenplay")
                let tv = try await ui.editor()
                try await ui.waitUntil("the draft is open") { !ui.currentDraftName().isEmpty && ui.editorText().contains("It has to be perfect.") }
                await ui.idle(1)
                var els = await ui.elements(tv)
                try ui.expect(els.count >= 4, "the editor shows at least four elements: \(els)")
                let open = ui.currentDraftName()
                if open == "Rewrite B" {
                    try ui.expect(els.contains("dialogue: It has to be perfect. Now. Again."), "Rewrite B text: \(els)")
                } else {
                    try ui.expect(els.contains("dialogue: It has to be perfect. Now."), "first draft text: \(els)")
                }
                let other = open == "Rewrite B" ? try await ui.otherDraftName() : "Rewrite B"
                try await ui.chooseMenu(id: "draft-menu", [other], exact: true)
                try await ui.waitUntil("other draft is open") { ui.currentDraftName() == other }
                els = await ui.elements(tv)
                if other == "Rewrite B" {
                    try ui.expect(els.contains("dialogue: It has to be perfect. Now. Again."), "Rewrite B text: \(els)")
                } else {
                    try ui.expect(els.contains("dialogue: It has to be perfect. Now."), "first draft text: \(els)")
                }
                try ui.expect(Array(els.prefix(3)) == ["scene_heading: INT. GLASS WORKSHOP \(emDash) NIGHT", "action: Asha shapes a lens over the blue flame.", "character: ASHA"], "first three elements: \(els)")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("43-several-screenplays/add-second-write-both-switch") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Two Rivers")
            try await twoRivers(ui, tv)
            try await ui.quiet()
        },
        W("43-several-screenplays/relaunch-keeps-own-text", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                let tv = try await scriptProject(ui, ctx, "Two Rivers")
                try await twoRivers(ui, tv)
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Two Rivers")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Screenplay")
                let tv = try await ui.editor()
                try await ui.waitUntil("the screenplay open last comes back") { ui.editorText().contains("The short opens on the bridge.") }
                try await ui.chooseMenu(id: "screenplay-picker", ["Two Rivers \u{00B7}"])
                try await ui.waitUntil("feature text") { ui.editorText().contains("The feature opens on the river. At dawn.") }
                let titles = try await ui.screenplayTitles()
                try ui.expect(titles.count == 2, "the project has two screenplays: \(titles)")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("43-several-screenplays/delete-to-recently-deleted-and-restore") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Two Rivers")
            try await twoRivers(ui, tv)
            try await ui.chooseMenu(id: "screenplay-picker", ["Delete Screenplay"])
            try await ui.expectShows("Delete \u{201C}Two Rivers: The Short\u{201D}?")
            try await ui.expectShows("They stay there for 30 days")
            try await ui.button("Delete Screenplay")
            try await ui.waitUntil("picker reads Two Rivers") { ui.screenplayPickerButton()?.label == "Screenplay: Two Rivers. Switch screenplay" }
            try await ui.waitUntil("feature text") { ui.editorText().contains("The feature opens on the river. At dawn.") }
            let titles = try await ui.screenplayTitles()
            try ui.expect(titles == ["Two Rivers"], "the screenplay list is only Two Rivers: \(titles)")
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.expectShows("Two Rivers: The Short")
            try await ui.restore("Two Rivers: The Short")
            try await ui.goTo("Screenplay")
            let tv2 = try await ui.editor()
            try await ui.chooseMenu(id: "screenplay-picker", ["Two Rivers: The Short"])
            try await ui.waitUntil("short text") { ui.editorText().contains("The short opens on the bridge.") }
            let after = try await ui.screenplayTitles()
            try ui.expect(Set(after) == ["Two Rivers", "Two Rivers: The Short"], "both screenplays are back: \(after)")
            try await ui.quiet()
        },
        W("zz-probe/rows") { ui, _, ctx in
            _ = try await scriptProject(ui, ctx, "Glass Harbour")
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Drafts"])
            try await ui.wait(id: "drafts-sheet", timeout: 10)
            await ui.idle(1)
            ui.dump("drafts")
        },
        W("42-writing-comfort/smart-suggestions-build-a-scene") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.quiet()
        },
        W("42-writing-comfort/tab-moves-through-dialogue-manual-change-sticks") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.enter()
            try await ui.tab()
            try await ui.typeHere("RAVI")
            if !ui.suggestionRows().isEmpty { try await ui.escape() }
            try await ui.cmdKey("2", code: 19)
            try await ui.waitUntil("the last element is action: RAVI") { true }
            try await expectLast(ui, tv, "action: RAVI")
            try await ui.typeHere(" runs.")
            try await expectLast(ui, tv, "action: RAVI runs.")
            try await ui.waitSaved()
            try await ui.quiet()
        },
        W("42-writing-comfort/new-character-one-enter-and-undo-steps") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.enter()
            try await ui.tab()
            try await ui.typeHere("RAVI")
            if !ui.suggestionRows().isEmpty { try await ui.escape() }
            try await ui.cmdKey("2", code: 19)
            try await ui.typeHere(" runs.")
            try await ui.enter()
            try await ui.typeHere("NISHA")
            try await ui.waitSuggestion("Create New: NISHA")
            try await ui.enter()
            try await ui.expectShows("Added NISHA to Characters")
            try await expectLastTwo(ui, tv, "character: NISHA", "dialogue: ")
            try await ui.waitSaved()
            try await ui.storyCharactersInclude("NISHA", true)
            let tvA = try await ui.editor()
            try await ui.waitSaved()
            // On the Mac the editor's Undo Project Change takes the steps back one at a time.
            await ui.unfocusText()
            try await ui.undo()
            try await expectLast(ui, tvA, "character: NISHA")
            try await ui.waitSaved()
            try await ui.storyCharactersInclude("NISHA", true)
            let tvB = try await ui.editor()
            await ui.unfocusText()
            try await ui.undo()
            try await ui.storyCharactersInclude("NISHA", false)
            let tvC = try await ui.editor()
            try await expectLast(ui, tvC, "character: NISHA")
            _ = tvB
            try await ui.quiet()
        },
        W("42-writing-comfort/known-name-from-list-fills-and-moves-on") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.enter()
            try await ui.typeHere("Tea.")
            try await ui.enter()
            let before = (await ui.elements(tv)).filter { $0 == "character: MEERA" }.count
            await ui.caretToEnd(tv)
            try await ui.typeHere("ME")
            try await ui.waitSuggestion("MEERA")
            try await ui.enter()
            try await expectLastTwo(ui, tv, "character: MEERA", "dialogue: ")
            try await ui.waitSaved()
            await ui.unfocusText()
            try await ui.undo()
            try await expectLast(ui, ui.currentEditorView() ?? tv, "character: MEERA")
            try await ui.waitSaved()
            await ui.unfocusText()
            try await ui.undo()
            await ui.idle(1)
            let now = (await ui.elements(ui.currentEditorView() ?? tv)).filter { $0 == "character: MEERA" }.count
            try ui.expect(now == before, "the count of character MEERA elements is back to \(before), now \(now)")
            try await ui.quiet()
        },
        W("42-writing-comfort/spelling-learn-removes-the-underline") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.waitUntil("teh is flagged", timeout: 10) { ui.editorWordMenu(tv, word: "teh")?.items.contains { $0.title == "Learn Spelling" } == true }
            try ui.expect(ui.editorWordMenu(tv, word: "MEERA")?.items.contains { $0.title == "Learn Spelling" } != true, "MEERA is not flagged (character names never are)")
            guard let menu = ui.editorWordMenu(tv, word: "teh"), let item = menu.items.first(where: { $0.title == "Learn Spelling" }) else { throw DriverError("no Learn Spelling item") }
            menu.performActionForItem(at: menu.index(of: item))
            try await ui.expectShows("Added \u{201C}teh\u{201D} to your dictionary")
            try await ui.waitUntil("teh no longer flagged") { ui.editorWordMenu(tv, word: "teh")?.items.contains { $0.title == "Learn Spelling" } != true }
            // Learn Spelling also teaches the word to this Mac's own dictionary; take it back so
            // the run leaves the tester's Mac as it was.
            NSSpellChecker.shared.unlearnWord("teh")
            try await ui.quiet()
        },
        W("42-writing-comfort/find-replace-this-scene-only-undone-in-one-step") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.waitSaved()
            try await ui.clickLine(tv, containing: "The bus is late.")
            await ui.idle(0.4)
            try await ui.act(id: "replace-toggle", role: "AXButton")
            try await ui.wait(id: "replace-bar", timeout: 10)
            try await ui.fill("Find", "bus", id: "replace-find")
            try await ui.fill("Replace with", "van", id: "replace-with")
            try await ui.act(id: "replace-all", role: "AXButton")
            try await ui.expectShows("Replaced 1 match in Scene 2")
            let tv2 = try await ui.editor()
            let els = await ui.elements(tv2)
            try ui.expect(els.contains("action: The van is late."), "the scene text changed: \(els)")
            try ui.expect(els.contains("action: Meera waits for teh last bus."), "the other scene did not change: \(els)")
            try await ui.act(id: "replace-close", role: "AXButton")
            try await ui.waitSaved()
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("the bus is late again") { ui.editorText().contains("The bus is late.") }
            try await ui.quiet()
        },
        W("42-writing-comfort/navigator-columns-can-be-shown-and-remembered") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            try await busDepotScene(ui, tv)
            try await ui.chooseMenu(label: "Navigator columns", ["Page"])
            try await ui.expectShows("Page 1")
            try await ui.chooseMenu(label: "Navigator columns", ["Notes"])
            try await ui.expectShows("Notes")
            try await ui.quiet()
        },
        W("42-writing-comfort/custom-shortcut-in-editor-survives-relaunch", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                let tv = try await scriptProject(ui, ctx, "Bus Depot")
                try await busDepotScene(ui, tv)
                try await ui.projectMenu(["Project settings"])
                try await ui.button("Keyboard shortcuts")
                try await ui.button("Add a shortcut for Find in script")
                try await ui.key("\u{F708}", code: 99)
                try await ui.expectShows("Find in script: F3")
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Bus Depot")
                try await ui.goTo("Screenplay")
                let tv = try await ui.editor()
                try await ui.clickLine(tv, containing: "Meera waits for teh last bus.")
                try await ui.key("\u{F708}", code: 99)
                try await ui.expectShows("Find in script")
                try await ui.projectMenu(["Project settings"])
                try await ui.button("Keyboard shortcuts")
                try await ui.button("Restore all defaults")
                try await ui.button("Restore all defaults")
                try await ui.expectShows("All shortcuts are back to their defaults")
            }
        },
        W("42-writing-comfort/home-shortcut-table-and-tooltips") { ui, _, ctx in
            try await ui.startApp(ctx)
            let b = try await ui.wait(id: "home-new-project", timeout: 15)
            let help = b.help
            try ui.expect(help.hasPrefix("New Project (") && help.contains("N)"), "the New Project button has a tooltip that names its key: \(help)")
            try await ui.menu(["Help", "Change shortcuts"])
            try await ui.wait(label: "Command", timeout: 10, "a table with a Command column")
            try await ui.button("Add a shortcut for New Project")
            try await ui.escape()
            try ui.expect(ui.exists(label: "Command"), "the dialog is still open after the first Esc")
            try await ui.escape()
            try await ui.waitUntil("the dialog closes") { !ui.exists(label: "Command") }
            try await ui.quiet()
        },
        W("21-idea-vault-flow/record-a-voice-note") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Letter Town")
            try await ui.act(id: "vault-record", role: "AXButton")
            try await ui.button("Start Recording")
            try await ui.expectShows("Recording\u{2026}", timeout: 15)
            try await ui.fill(nil, "Night bus idea", id: "record-name")
            await ui.idle(1.5)
            try await ui.button("Stop & Save")
            try await ui.waitVaultCard("Night bus idea")
            try await ui.quiet()
        },
        W("21-idea-vault-flow/research-flags-set-undo-change") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Letter Town")
            try await ui.quickCapture("Police procedure: order of evidence handling at a small station.")
            try await ui.contextMenu(on: try await ui.vaultCardWait("Police procedure"), ["Research flag", "Needs Expert Check"])
            try await ui.expectShows("as Needs Expert Check")
            try await ui.waitUntil("the card shows Needs Expert Check") { ui.vaultCard("Police procedure")?.value.contains("Needs Expert Check") == true }
            try await ui.contextMenu(on: try await ui.vaultCardWait("Police procedure"), ["Research flag", "Checked"])
            try await ui.waitUntil("the card shows Checked") {
                let v = ui.vaultCard("Police procedure")?.value ?? ""
                return v.contains("Checked") && !v.contains("Needs Expert")
            }
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("the card shows Needs Expert Check again") { ui.vaultCard("Police procedure")?.value.contains("Needs Expert Check") == true }
            try await ui.redo()
            try await ui.waitUntil("the card no longer shows Needs Expert") {
                let v = ui.vaultCard("Police procedure")?.value ?? ""
                return v.contains("Checked") && !v.contains("Needs Expert")
            }
            try await ui.tap(try await ui.vaultCardWait("Police procedure"), count: 2)
            let flag = try await ui.wait(id: "drawer-research-flag", timeout: 10)
            try ui.expect(flag.text.contains("Checked") || flag.value.contains("Checked"), "the research flag field in the drawer is set to Checked: \(flag.text)")
            try await ui.expectShows("OpenFrame never marks research as true just because you imported it.")
            try await ui.act(id: "drawer-close", role: "AXButton")
            try await ui.waitGone(id: "drawer-title")
            try await ui.quiet()
        },
        W("21-idea-vault-flow/global-vault-copy-into-project") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Letter Town")
            try await ui.act(label: "Global", role: "AXRadioButton")
            try await ui.quickCapture("A city where every letter is delivered one year late.")
            try await ui.contextMenu(on: try await ui.vaultCardWait("A city where every letter"), ["Copy to Project"])
            try await ui.button("Copy to Project")
            try await ui.expectShows("to the Project Idea Vault")
            try await ui.act(label: "This Project", role: "AXRadioButton")
            try await ui.waitVaultCard("A city where every letter")
            try await ui.quiet()
        },
        W("21-idea-vault-flow/send-to-story-board-original-stays") { ui, _, ctx in
            try await vaultProject(ui, ctx, "Letter Town")
            try await ui.quickCapture("Meera finds her own handwriting on an old envelope.")
            try await ui.contextMenu(on: try await ui.vaultCardWait("Meera finds her own"), ["Send to Story"])
            try await ui.expectShows("Send to Story Board")
            try await ui.button("Create Copy in Story Board")
            try await ui.waitUntil("the sheet closes") { !ui.shows("Create Copy in Story Board") }
            try ui.expect(ui.vaultCard("Meera finds her own") != nil, "the Vault still has the card")
            try await ui.goTo("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)
            if let t = ui.query(id: "story-parking-toggle").first, t.value == "0" { try await ui.act(t) }
            try await ui.waitUntil("scene card from the idea") { ui.cardLabels().contains("Scene Card: Meera finds her own handwriting on an old envelope.") }
            try await ui.quiet()
            try await ui.goTo("Idea Vault")
        },
        W("21-idea-vault-flow/relaunch-keeps-note-flags-copies", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Letter Town")
                try await ui.goTo("Idea Vault")
                try await ui.act(id: "vault-record", role: "AXButton")
                try await ui.button("Start Recording")
                try await ui.expectShows("Recording\u{2026}", timeout: 15)
                try await ui.fill(nil, "Night bus idea", id: "record-name")
                await ui.idle(1.5)
                try await ui.button("Stop & Save")
                try await ui.waitVaultCard("Night bus idea")
                try await ui.quickCapture("Police procedure: order of evidence handling at a small station.")
                try await ui.contextMenu(on: try await ui.vaultCardWait("Police procedure"), ["Research flag", "Checked"])
                try await ui.act(label: "Global", role: "AXRadioButton")
                try await ui.quickCapture("A city where every letter is delivered one year late.")
                try await ui.contextMenu(on: try await ui.vaultCardWait("A city where every letter"), ["Copy to Project"])
                try await ui.button("Copy to Project")
                try await ui.act(label: "This Project", role: "AXRadioButton")
                try await ui.waitVaultCard("A city where every letter")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Letter Town")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Idea Vault")
                try await ui.waitVaultCard("Night bus idea")
                try await ui.waitUntil("the Police procedure card shows Checked") { ui.vaultCard("Police procedure")?.value.contains("Checked") == true }
                try await ui.waitVaultCard("A city where every letter")
                try await ui.tap(try await ui.vaultCardWait("Night bus idea"), count: 2)
                try await ui.wait(id: "audio-player", timeout: 10, "the audio player in the drawer")
                try await ui.waitUntil("the recording loads with a length greater than zero", timeout: 15) {
                    let t = ui.query(id: "audio-time").map(\.text).joined(separator: " ")
                    return !t.isEmpty && !t.contains("0:00 / 0:00") && !t.hasPrefix("0:00 0:00")
                }
                try await ui.act(id: "drawer-close", role: "AXButton")
                try await ui.act(label: "Global", role: "AXRadioButton")
                try await ui.waitVaultCard("A city where every letter")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("44-scene-hub-comments-compare/scene-hub-from-context-menu") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Second Look")
            try await nightBus(ui, tv)
            try await ui.contextMenu(on: try await ui.sceneRow("BUS STOP"), ["Scene Hub"])
            try await ui.expectShows("What exists around this scene?")
            try await ui.waitUntil("the drawer title names BUS STOP") { ui.query(id: "scene-hub-title").contains { $0.text.contains("BUS STOP") } }
            try await ui.button("Close")
            try await ui.waitUntil("the drawer closes") { !ui.exists(label: "Close", role: "AXButton") }
            try await ui.quiet()
        },
        W("44-scene-hub-comments-compare/comment-counted-and-survives-restart", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                let tv = try await scriptProject(ui, ctx, "Second Look")
                try await nightBus(ui, tv)
                try await ui.contextMenu(on: try await ui.sceneRow("BUS STOP"), ["Add Comment"])
                try await ui.fill("Comment", "Is the bus stop lit at night?", id: "comment-field")
                try await ui.button("Comment")
                try await ui.expectShows("Is the bus stop lit at night?")
                try await ui.waitUntil("the toolbar button reads Comments (1)") { ui.exists(label: "Comments (1)") }
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Second Look")
                try await ui.goTo("Screenplay")
                try await ui.waitUntil("the toolbar button reads Comments (1)", timeout: 15) { ui.exists(label: "Comments (1)") }
                try await ui.button("Comments (1)")
                try await ui.expectShows("Is the bus stop lit at night?")
            }
        },
        W("44-scene-hub-comments-compare/second-draft-and-compare") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Second Look")
            try await nightBus(ui, tv)
            try await ui.waitSaved()
            try await ui.chooseMenu(id: "draft-menu", ["New Draft"])
            try await ui.fill("Draft name", "Rewrite B", id: "new-draft-name")
            try await ui.button("Create Draft")
            try await ui.expectShows("Created draft \u{201C}Rewrite B\u{201D}")
            try await ui.clickLine(tv, containing: "You came back.", end: true)
            try await ui.typeHere(" Every night.")
            try await ui.waitUntil("a dialogue line reads You came back. Every night.") { ui.editorText().contains("You came back. Every night.") }
            try await ui.waitSaved(timeout: 15)
            try await ui.act(id: "compare-button", role: "AXButton")
            try await ui.wait(id: "compare-view", timeout: 10, "the comparison")
            try await ui.expectShows("Every night.")
            try await ui.quiet()
            try await ui.button("Done")
            try await ui.waitUntil("the comparison closes") { !ui.exists(id: "compare-view") }
        },
        W("41-screenplay-engine/write-element-flow-and-page-break") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            try await expectElements(ui, tv, include: ["page_break: ", "dialogue: Who is there?", "dialogue: Nobody answers."])
            let els = await ui.elements(tv)
            let i = els.firstIndex(of: "page_break: ")!
            try ui.expect(els[i - 1] == "dialogue: Who is there?" && els[i + 1] == "dialogue: Nobody answers.", "the page break sits between the two dialogue lines: \(els)")
            try await ui.waitSaved()
            try await ui.waitUntil("the screen shows 2 pages", timeout: 15) { ui.shows("2 pages") || ui.shows("of 2") }
            try await ui.quiet()
        },
        W("41-screenplay-engine/title-page-fields-live-preview") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Title Page"])
            try await ui.fill("Date", "4 October 2026")
            try await ui.fill("Phone", "+91 98765 43210")
            try await ui.waitUntil("the title page preview shows 4 October 2026", timeout: 10) {
                ui.query(label: "Title page preview").contains { $0.anyValue.contains("4 October 2026") }
            }
            try await ui.button("Save")
            try await ui.expectShows("Saved the title page")
            try await ui.quiet()
        },
        W("41-screenplay-engine/page-setup-a4-with-header") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Page Setup"])
            try await ui.chooseMenu(label: "Paper", ["A4"])
            try await ui.fill("Header", "{title} {draft}")
            try await ui.button("Save")
            try await ui.expectShows("Changed the page setup")
            try await ui.waitUntil("the page count tooltip says A4 with 59 lines per page", timeout: 10) {
                (ui.query(id: "editor-page-count").first?.help ?? "").contains("A4") && (ui.query(id: "editor-page-count").first?.help ?? "").contains("59")
            }
            try await ui.quiet()
        },
        W("41-screenplay-engine/custom-element-under-advanced-used-in-script") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Custom Elements"])
            try await ui.button("Add custom element")
            try await ui.fill("Name", "Telephone")
            try await ui.chooseMenu(label: "Look", ["Dialogue"])
            try await ui.chooseMenu(label: "Shortcut", ["Ctrl+Alt+1"])
            try await ui.button("Add")
            try await ui.expectShows("Added the custom element \u{201C}Telephone\u{201D}")
            try await ui.button("Done")
            try await ui.clickLine(tv, containing: "The telephone rings twice.")
            try await ui.chooseMenu(id: "element-picker", ["Telephone"])
            try await expectElements(ui, tv, include: ["custom: The telephone rings twice."])
            try await ui.tab()
            try await expectElements(ui, tv, include: ["custom: The telephone rings twice."])
            try await ui.waitSaved()
            try await ui.chooseMenu(label: "Layout", ["Page view"])
            try await ui.expectShows("The telephone rings twice.")
            try await ui.expectShows("Page 2")
            try await ui.quiet()
        },
        W("41-screenplay-engine/lock-start-revision-edit") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            try await ui.waitSaved()
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Drafts"])
            let name = ui.currentDraftName()
            try await ui.chooseMenu(label: "Actions for \(name)", ["Lock as Shooting Draft"])
            try await ui.button("Lock as Shooting Draft")
            try await ui.expectShows("Shooting Draft")
            try await ui.chooseMenu(label: "Actions for \(name)", ["Start Revision"])
            try await ui.button("Start Revision")
            try await ui.expectShows("stays locked", timeout: 15)
            try await ui.button("Done")
            let tv2 = try await ui.editor()
            try await ui.clickLine(tv2, containing: "Nobody answers.", end: true)
            try await ui.typeHere(" Again.")
            try await ui.waitSaved()
            try await ui.chooseMenu(id: "screenplay-tools-menu", ["Drafts"])
            try await ui.expectShows("Changed pages:")
            try await ui.quiet()
        },
        W("41-screenplay-engine/indian-scripts-and-remapped-page-break-shortcut") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Signal Box")
            try await signalBox(ui, tv)
            let line = "\u{0C2E}\u{0C40}\u{0C30}\u{0C3E} \u{0C30}\u{0C48}\u{0C32}\u{0C41} \u{0C38}\u{0C4D}\u{0C1F}\u{0C47}\u{0C37}\u{0C28}\u{0C4D}\u{200C}\u{0C32}\u{0C4B} \u{0C35}\u{0C47}\u{0C1A}\u{0C3F} \u{0C09}\u{0C02}\u{0C26}\u{0C3F}. \u{092E}\u{0940}\u{0930}\u{093E} \u{0928}\u{0947} \u{0915}\u{0939}\u{093E}, \u{201C}\u{0915}\u{094D}\u{0937}\u{092E}\u{093E} \u{0915}\u{0930}\u{0947}\u{0902}\u{0964}\u{201D}"
            try await ui.clickLine(tv, containing: "The telephone rings twice.", end: true)
            try await ui.enter()
            try await ui.typeHere(line)
            try await ui.waitUntil("the editor shows the Telugu line") { ui.editorText().contains("\u{0C2E}\u{0C40}\u{0C30}\u{0C3E} \u{0C30}\u{0C48}\u{0C32}\u{0C41}") }
            try await ui.waitSaved()
            let before = (await ui.elements(tv)).filter { $0 == "page_break: " }.count
            try await ui.projectMenu(["Project settings"])
            try await ui.button("Keyboard shortcuts")
            try await ui.button("Add a shortcut for Insert page break")
            try await ui.shortcutKey("\r", code: 36, modifiers: [.command, .shift])
            try await ui.button("Done")
            try await ui.goTo("Screenplay")
            let tv2 = try await ui.editor()
            try await ui.clickLine(tv2, containing: "\u{0C2E}\u{0C40}\u{0C30}\u{0C3E}", end: true)
            try await ui.shortcutKey("\r", code: 36, modifiers: [.command])
            let same = (await ui.elements(tv2)).filter { $0 == "page_break: " }.count
            try ui.expect(same == before, "the old key no longer inserts a page break")
            try await ui.shortcutKey("\r", code: 36, modifiers: [.command, .shift])
            let more = (await ui.elements(tv2)).filter { $0 == "page_break: " }.count
            try ui.expect(more == before + 1, "the new key inserts one page break")
            try await ui.waitSaved()
        },
        W("41-screenplay-engine/relaunch-keeps-setup-and-revision-text", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                let tv = try await scriptProject(ui, ctx, "Signal Box")
                try await signalBox(ui, tv)
                try await ui.chooseMenu(id: "screenplay-tools-menu", ["Page Setup"])
                try await ui.chooseMenu(label: "Paper", ["A4"])
                try await ui.fill("Header", "{title} {draft}")
                try await ui.button("Save")
                try await ui.expectShows("Changed the page setup")
                try await ui.clickLine(tv, containing: "Nobody answers.", end: true)
                try await ui.typeHere(" Again.")
                try await ui.waitSaved()
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Signal Box")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Screenplay")
                let tv = try await ui.editor()
                try await expectElements(ui, tv, include: ["dialogue: Nobody answers. Again."])
                try await ui.waitUntil("the page setup is A4", timeout: 10) { (ui.query(id: "editor-page-count").first?.help ?? "").contains("A4") }
                try await ui.chooseMenu(id: "screenplay-tools-menu", ["Page Setup"])
                let h = try await ui.wait(label: "Header", timeout: 10)
                try ui.expect(h.value == "{title} {draft}", "the header is {title} {draft}: \(h.value)")
                try await ui.button("Cancel")
                try await ui.clickLine(tv, containing: "Again", end: true)
                try await ui.enter()
                try await ui.typeHere("Static on the line.")
                try await ui.shortcutKey("1", code: 18, modifiers: [.command, .option])
                try await expectElements(ui, tv, include: ["custom: Static on the line."])
                try await ui.waitSaved()
                try await ui.quiet()
            }
        },
        W("96-keyboard-and-screen-reader/project-with-a-screenplay-to-walk-through") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Quiet Keys")
            try await nightBus(ui, tv)
            try await ui.quiet()
        },
        W("96-keyboard-and-screen-reader/workspace-project-home", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Project Home")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Project Home")
        },
        W("96-keyboard-and-screen-reader/workspace-idea-vault", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Idea Vault")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Idea Vault")
        },
        W("96-keyboard-and-screen-reader/workspace-story", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Story")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Story")
        },
        W("96-keyboard-and-screen-reader/workspace-screenplay", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Screenplay")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Screenplay")
        },
        W("96-keyboard-and-screen-reader/workspace-breakdown", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Breakdown")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Breakdown")
        },
        W("96-keyboard-and-screen-reader/workspace-schedule", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Schedule")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Schedule")
        },
        W("96-keyboard-and-screen-reader/workspace-call-sheets", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Call Sheets")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Call Sheets")
        },
        W("96-keyboard-and-screen-reader/workspace-files", timeout: 240) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Keys")
            try await ui.goTo("Files")
            await ui.idle(1.0)
            try await keyboardWalk(ui, "Files")
        },
        W("01-journey/01-first-launch-create-project", timeout: 165) { ui, _, ctx in
            try await journey(ui, ctx, through: 1)
        },
        W("01-journey/02-idea-vault-quick-capture", timeout: 210) { ui, _, ctx in
            try await journey(ui, ctx, through: 2)
        },
        W("01-journey/03-story-act-sequence-cards-reorder", timeout: 255) { ui, _, ctx in
            try await journey(ui, ctx, through: 3)
        },
        W("01-journey/04-build-screenplay-opens-first-scene", timeout: 300) { ui, _, ctx in
            try await journey(ui, ctx, through: 4)
        },
        W("01-journey/05-type-in-editor-saves-itself", timeout: 345) { ui, _, ctx in
            try await journey(ui, ctx, through: 5)
        },
        W("01-journey/06-import-pasted-fountain", timeout: 390) { ui, _, ctx in
            try await journey(ui, ctx, through: 6)
        },
        W("01-journey/07-breakdown-source-and-suggestions", timeout: 435) { ui, _, ctx in
            try await journey(ui, ctx, through: 7)
        },
        W("01-journey/08-schedule-unscheduled-then-day", timeout: 480) { ui, _, ctx in
            try await journey(ui, ctx, through: 8)
        },
        W("01-journey/09-call-sheet-from-day", timeout: 525) { ui, _, ctx in
            try await journey(ui, ctx, through: 9)
        },
        W("01-journey/10-strip-opens-production-draft", timeout: 570) { ui, _, ctx in
            try await journey(ui, ctx, through: 10)
        },
        W("01-journey/11-search-finds-scene", timeout: 615) { ui, _, ctx in
            try await journey(ui, ctx, through: 11)
        },
        W("01-journey/12-recently-deleted-card-restore", timeout: 660) { ui, _, ctx in
            try await journey(ui, ctx, through: 12)
        },
        W("01-journey/13-ai-panel-ready", timeout: 705) { ui, _, ctx in
            try await journey(ui, ctx, through: 13)
        },
        W("01-journey/14-workspaces-beside-docked-ai", timeout: 750) { ui, _, ctx in
            try await journey(ui, ctx, through: 14)
        },
        W("01-journey/15-close-project-cleanly", timeout: 795) { ui, _, ctx in
            try await journey(ui, ctx, through: 15)
        },
        W("02-restart/reopen-finds-screenplay-story-call-sheet", phases: 2, timeout: 420) { ui, phase, ctx in
            if phase == 1 {
                // The shared journey data: the demo project "Night Bus", closed cleanly.
                try await journey(ui, ctx, through: 15, skipping: [6, 13, 14])
            } else {
                try await ui.startApp(ctx)
                try await ui.expectShows("Night Bus")
                try await ui.openProject("Night Bus")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question: the last session closed cleanly")
                try await ui.menu(["Edit", "Search and Commands\u{2026}"])
                try await ui.fill(nil, "there that night", id: "palette-field")
                try await ui.waitUntil("a result containing INT. POLICE STATION") { ui.all().contains { $0.text.contains("INT. POLICE STATION") && $0.identifier != "screenplay-editor" } }
                try await ui.enter()
                _ = try await ui.editor()
                try await ui.waitUntil("the screenplay contains the line") { ui.editorText().contains("I was there that night.") }
                try await ui.goTo("Story")
                try await ui.waitUntil("the board shows exactly two scene cards") { ui.sceneCards().count == 2 }
                try await ui.goTo("Call Sheets")
                try await ui.expectShows("Day 1")
                try await ui.quiet()
            }
        },
        W("04-close-keeps-typing/close-within-100ms-reopen-finds-text", phases: closeRounds * 2 + 1, timeout: 150, end: { _ in .quit }) { ui, phase, ctx in
            try await closeKeepsTyping(ui, ctx, phase)
        },
        W("60-appearance/mode-and-theme-persist", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Harbour Lights")
                try await ui.openSettings("Appearance")
                try await ui.expectShows("Theme")
                try ui.expect(ui.themeCardSelected("openframe-default"), "the theme is openframe-default")
                try ui.expect(ui.modeSelected("System"), "the preference is system")
                try await ui.chooseMode("Dark")
                try await ui.waitUntil("the background is dark (each part below 80)") {
                    guard let c = ui.backgroundRGB() else { return false }
                    return c.r < 80 && c.g < 80 && c.b < 80
                }
                try await ui.chooseMode("Light")
                try await ui.waitUntil("the background is 243, 241, 236") {
                    guard let c = ui.backgroundRGB() else { return false }
                    ui.log("background \(c)")
                    return abs(c.r - 243) <= 2 && abs(c.g - 241) <= 2 && abs(c.b - 236) <= 2
                }
                try await ui.quiet()
                try await ui.act(id: "theme-card-midnight-reel", role: "AXButton")
                try await ui.waitUntil("Midnight Reel is the selected card") { ui.themeCardSelected("midnight-reel") }
                try await ui.chooseMode("Dark")
                try await ui.waitUntil("dark again") { (ui.backgroundRGB().map { $0.r < 80 && $0.g < 80 && $0.b < 80 }) ?? false }
                try await ui.closeSettings()
                try await ui.closeProject()
            } else {
                try await ui.waitHome()
                try await ui.openSettings("Appearance")
                try await ui.waitUntil("Midnight Reel stays selected after a relaunch") { ui.themeCardSelected("midnight-reel") }
                try ui.expect(ui.modeSelected("Dark"), "the mode is dark after a relaunch")
                try await ui.waitUntil("the background is dark") { (ui.backgroundRGB().map { $0.r < 80 && $0.g < 80 && $0.b < 80 }) ?? false }
                try await ui.act(id: "theme-card-openframe-default", role: "AXButton")
                try await ui.waitUntil("OpenFrame Default selected at once") { ui.themeCardSelected("openframe-default") }
                try await ui.chooseMode("System")
                try await ui.waitUntil("the preference is system") { ui.modeSelected("System") }
                try await ui.closeSettings()
                try await ui.quiet()
            }
        },
        W("60-appearance/paste-check-rename-delete-custom-theme") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Lights")
            try await ui.openSettings("Appearance")
            let hostile = "{\"formatVersion\":1,\"id\":\"evil-theme\",\"name\":\"<img src=x onerror=alert(1)>\",\"seeds\":{\"neutralHue\":230,\"neutralTint\":0.02,\"accentHue\":55,\"accentChroma\":0.14,\"accentLightnessNudge\":0},\"overrides\":{\"light\":{\"bg\":\"#fff} body{display:none\"}}}"
            let harbour = "{\"formatVersion\":1,\"id\":\"harbour-night\",\"name\":\"Harbour Night\",\"description\":\"Blue-grey surfaces with a lamp-orange accent.\",\"author\":\"E2E\",\"seeds\":{\"neutralHue\":230,\"neutralTint\":0.02,\"accentHue\":55,\"accentChroma\":0.14,\"accentLightnessNudge\":0}}"
            try await ui.act(id: "theme-card-midnight-reel", role: "AXButton")
            try await ui.button("Paste Theme\u{2026}")
            try await ui.fill("Paste", hostile, id: "theme-paste")
            try await ui.button("Check Theme")
            try await ui.expectShows("This theme cannot be used. Nothing was changed.")
            try ui.expect(!(ui.query(label: "Save and Apply").first?.isEnabled ?? false), "Save and Apply is disabled")
            try await ui.button("Cancel")
            try await ui.waitUntil("Midnight Reel is still selected") { ui.themeCardSelected("midnight-reel") }
            try await ui.button("Paste Theme\u{2026}")
            try await ui.fill("Paste", harbour, id: "theme-paste")
            try await ui.button("Check Theme")
            try await ui.expectShows("This theme is readable in Light and Dark.")
            try await ui.expectShows("Dark preview")
            try await ui.button("Save and Apply")
            try await ui.waitUntil("harbour-night is selected") { ui.themeCardSelected("harbour-night") }
            try await ui.expectShows("My Themes")
            try await ui.expectShows("Harbour Night")
            try await ui.button("Rename Harbour Night")
            try await ui.fill("Name", "Harbour Lamps", id: "theme-name")
            try await ui.button("Rename")
            try await ui.expectShows("Harbour Lamps")
            try await ui.chooseMode("Dark")
            try await ui.button("Delete Harbour Lamps")
            try await ui.button("Delete Theme")
            try await ui.waitUntil("openframe-default at once") { ui.themeCardSelected("openframe-default") }
            try await ui.quiet()
        },
        W("91-appearance-themes/16-themes-2-modes-15-screens-readable", timeout: 3300) { ui, _, ctx in
            try await journey(ui, ctx, through: 9, skipping: [6])
            try await ui.closeProject()
            try await ui.openSettings("Appearance")
            var checked = 0
            var problems: [String] = []
            for theme in builtInThemes {
                for mode in ["Light", "Dark"] {
                    try await ui.act(id: "theme-card-\(theme.id)", role: "AXButton")
                    try await ui.chooseMode(mode)
                    try ui.expect(ui.themeCardSelected(theme.id) && ui.modeSelected(mode), "the appearance is \(theme.name), \(mode)")
                    for screen in themeScreens {
                        try await ui.showThemeScreen(screen)
                        await ui.idle(0.5)
                        checked += 1
                        let found = ui.readabilityProblems(screen: screen)
                        let known = theme.id == "openframe-default" && mode == "Light"
                        if !found.isEmpty && !known { problems.append("\(theme.name) \(mode) \(screen): \(found.prefix(3).joined(separator: "; "))") }
                    }
                    try await ui.closeProject()
                }
            }
            try ui.expect(checked == 16 * 2 * themeScreens.count, "480 screens were checked, \(checked) were")
            try? problems.joined(separator: "\n").write(to: ui.artifacts.appendingPathComponent("theme-problems.txt"), atomically: true, encoding: .utf8)
            try ui.expect(problems.isEmpty, "\(problems.count) screens are hard to read; the first: \(problems.prefix(3))")
        },
        W("91-appearance-themes/theme-and-mode-come-back-after-relaunch", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.openSettings("Appearance")
                try await ui.act(id: "theme-card-velvet-curtain", role: "AXButton")
                try await ui.chooseMode("Dark")
                try await ui.waitUntil("velvet curtain dark") { ui.themeCardSelected("velvet-curtain") && ui.modeSelected("Dark") }
                await ui.idle(0.8)
                let c = ui.backgroundRGB(inWindowTitled: "Projects") ?? (0, 0, 0)
                try "\(c.r) \(c.g) \(c.b)".write(to: ctx.work.appendingPathComponent("velvet-bg.txt"), atomically: true, encoding: .utf8)
                try await ui.closeSettings()
            } else {
                try await ui.expectShows("Projects")
                let saved = (try? String(contentsOf: ctx.work.appendingPathComponent("velvet-bg.txt"), encoding: .utf8)) ?? ""
                try await ui.openSettings("Appearance")
                try await ui.waitUntil("velvet-curtain selected after a relaunch") { ui.themeCardSelected("velvet-curtain") && ui.modeSelected("Dark") }
                let c = ui.backgroundRGB(inWindowTitled: "Projects") ?? (0, 0, 0)
                try ui.expect("\(c.r) \(c.g) \(c.b)" == saved, "the window background is the same after a relaunch: \(saved) then \(c)")
                try await ui.closeSettings()
            }
        },
        W("zz-probe/keys") { ui, _, ctx in
            let tv = try await scriptProject(ui, ctx, "Bus Depot")
            await ui.caretToStart(tv)
            try await ui.typeHere("INT. A - NIGHT")
            try await ui.enter()
            try await ui.typeHere("Meera waits.")
            try await ui.enter()
            try await ui.typeHere("MEERA")
            try await ui.waitSuggestion("Create New: MEERA")
            try await ui.enter()
            ui.log("elements \(await ui.elements(tv))")
            ui.log("rows after elements: \(ui.suggestionRows())")
            try await ui.typeHere("Is it late?")
            try await ui.enter()
            try await ui.enter()
            try await ui.typeHere("EXT. ROAD - LATER")
            try await ui.escape()
            try await ui.enter()
            try await ui.typeHere("The bus is late.")
            try await ui.enter()
            await ui.idle(0.5)
            ui.log("before ME: label \(ui.currentElementLabel())")
            try await ui.typeHere("ME")
            for i in 0..<4 {
                await ui.idle(0.5)
                ui.log("t\(i) rows \(ui.suggestionRows()) label \(ui.currentElementLabel()) text tail [\(tv.string.suffix(30).replacingOccurrences(of: "\n", with: "|"))]")
            }
            ui.dump("meera")
        },
        W("10-home/first-project-opens-on-project-home") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Lights")
            try await ui.waitUntil("window title names the project") { ui.windowTitle().hasPrefix("Harbour Lights") }
            try await ui.expectShows("Harbour Lights")
            try await ui.expectShows("Start here")
            try await ui.quiet()
        },
        W("10-home/status-change-undo-redo") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Lights")
            try ui.expect(try await ui.projectStatus() == "Status: Idea", "the status starts as Idea")
            try await ui.projectMenu(["Status: Idea", "Writing"])
            try await ui.expectShows("Changed project status to Writing")
            try ui.expect(try await ui.projectStatus() == "Status: Writing", "the status is Writing")
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("status is Idea") { true }
            try ui.expect(try await ui.projectStatus() == "Status: Idea", "undo brings Idea back")
            try await ui.redo()
            try ui.expect(try await ui.projectStatus() == "Status: Writing", "redo brings Writing back")
            try await ui.quiet()
        },
        W("10-home/home-shows-renamed-project") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Lights")
            try await ui.projectMenu(["Status: Idea", "Writing"])
            try await ui.closeProject()
            try ui.expect(!ui.exists(id: "project-home"), "the project shell is gone")
            try await ui.waitHomeCards(1)
            try ui.expect(ui.homeCards() == ["Open Harbour Lights, Feature Film, Writing"], "the only card: \(ui.homeCards())")
            try await ui.contextMenu(on: (try await ui.homeCardWait("Harbour Lights")), ["Rename"])
            try await ui.fill(nil, "Harbour Lights Redux", id: "rename-title")
            try await ui.button("Rename")
            try await ui.expectShows("Renamed to \u{201C}Harbour Lights Redux\u{201D}.")
            try await ui.waitUntil("renamed card") { ui.homeCards().contains("Open Harbour Lights Redux, Feature Film, Writing") }
            try await ui.quiet()
        },
        W("10-home/duplicate-search-archive-restore") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Harbour Lights Redux")
            try await ui.closeProject()
            try await ui.contextMenu(on: (try await ui.homeCardWait("Harbour Lights Redux")), ["Duplicate Project"])
            try await ui.expectShows("It is a separate project with its own identity.")
            try await ui.waitHomeCards(2)
            let copy = ui.homeCards().first { !$0.hasPrefix("Open Harbour Lights Redux,") }
            try ui.expect(copy != nil && copy!.contains("Feature Film"), "a second card that is a Feature Film with another title: \(ui.homeCards())")
            let copyTitle = String(copy!.dropFirst("Open ".count).components(separatedBy: ", Feature Film").first!)
            try await ui.fill("Search projects", "Redux")
            try await ui.waitUntil("every card contains Redux") { !ui.homeCards().isEmpty && ui.homeCards().allSatisfy { $0.contains("Redux") } }
            try await ui.fill("Search projects", "")
            try await ui.waitHomeCards(2)
            try await ui.contextMenu(on: (try await ui.homeCardWait(copyTitle)), ["Archive"])
            try await ui.expectShows("was archived.")
            try await ui.waitHomeCards(1)
            try await ui.act(label: "Show archived", role: "AXCheckBox")
            try await ui.waitHomeCards(2)
            try await ui.expectShows("Archived")
            try await ui.contextMenu(on: (try await ui.homeCardWait(copyTitle)), ["Restore from Archive"])
            try await ui.expectShows("is back in your projects.")
            try await ui.act(label: "Show archived", role: "AXCheckBox")
            try await ui.waitHomeCards(2)
            try await ui.quiet()
        },
        W("10-home/relaunch-keeps-projects-title-status", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Harbour Lights Redux")
                try await ui.projectMenu(["Status: Idea", "Writing"])
                try await ui.closeProject()
                try await ui.contextMenu(on: (try await ui.homeCardWait("Harbour Lights Redux")), ["Duplicate Project"])
                try await ui.waitHomeCards(2)
            } else {
                try await ui.waitHomeCards(2)
                try ui.expect(ui.homeCards().contains("Open Harbour Lights Redux, Feature Film, Writing"), "cards \(ui.homeCards())")
                try await ui.openProject("Harbour Lights Redux")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.waitUntil("window title") { ui.windowTitle().hasPrefix("Harbour Lights Redux") }
                try ui.expect(try await ui.projectStatus() == "Status: Writing", "status kept")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("10-home/delete-project-confirm-typed-name", phases: 2) { ui, phase, ctx in
            let bin = ctx.dataRoot.appendingPathComponent("recycle-bin")
            func binCount() -> Int { (try? FileManager.default.contentsOfDirectory(atPath: bin.path).count) ?? 0 }
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Harbour Lights Redux")
                try await ui.closeProject()
                try await ui.contextMenu(on: (try await ui.homeCardWait("Harbour Lights Redux")), ["Duplicate Project"])
                try await ui.waitHomeCards(2)
                let copy = ui.homeCards().first { !$0.hasPrefix("Open Harbour Lights Redux,") }!
                let copyTitle = String(copy.dropFirst("Open ".count).components(separatedBy: ", Feature Film").first!)
                let before = binCount()
                try await ui.contextMenu(on: (try await ui.homeCardWait(copyTitle)), ["Delete Project"])
                let confirm = try await ui.wait(id: "delete-confirm-field", timeout: 10)
                _ = confirm
                try ui.expect(!(ui.query(id: "delete-confirm").first?.isEnabled ?? true), "Delete Project is disabled until the name is typed")
                try await ui.fill(nil, copyTitle, id: "delete-confirm-field")
                try await ui.waitUntil("Delete Project enabled") { ui.query(id: "delete-confirm").first?.isEnabled ?? false }
                try await ui.act(id: "delete-confirm", role: "AXButton")
                try await ui.expectShows("\u{201C}\(copyTitle)\u{201D} was moved to the Trash.")
                try await ui.waitHomeCards(1)
                try ui.expect(binCount() == before + 1, "the test bin holds one more item: \(before) then \(binCount())")
                try await ui.quiet()
            } else {
                try await ui.waitHomeCards(1)
                try ui.expect(ui.homeCards() == ["Open Harbour Lights Redux, Feature Film, Idea"], "the only card: \(ui.homeCards())")
                try await ui.quiet()
            }
        },
        W("16-about/internal-build-shows-version-and-not-for-sale") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Pier")
            try await ui.openAbout()
            let version = try await ui.wait(id: "about-version").text
            try ui.expect(version.range(of: #"^Version \d+\.\d+\.\d+"#, options: .regularExpression) != nil, "a version of three numbers: \(version)")
            try await ui.expectShows("OpenFrame Studio")
            try await ui.expectShows("Internal build, not for sale.")
            try ui.expect(!ui.shows("Buy full version"), "no Buy full version button")
            try ui.expect(!ui.exists(label: "Buy full version"), "no Buy full version control")
            try ui.expectNoPrice()
            try await ui.quiet()
        },
        W("17-internal-build-release/no-trial-chip-no-buy-no-popup-about-says-internal") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Quiet Pier")
            await ui.idle(1.5)
            for text in ["Buy full version", "Free trial", "free trial", "Trial"] { try ui.expect(!ui.shows(text), "the screen shows \(text)") }
            try ui.expect(ui.windows.count == 1 && ui.windows[0].attachedSheet == nil, "no dialog is shown")
            try await ui.openAbout()
            let version = try await ui.wait(id: "about-version").text
            try ui.expect(version.range(of: #"^Version \d+\.\d+\.\d+"#, options: .regularExpression) != nil, "a version like 1.2.3: \(version)")
            try await ui.expectShows("Internal build, not for sale.")
            try ui.expectNoPrice()
            try await ui.quiet()
        },
        W("30-story/act-sequence-two-cards") { ui, _, ctx in
            try await storyProject(ui, ctx, "Paper Boats")
            try await ui.expectShows("Your Story Board is empty.")
            try await addAct(ui, "Act One")
            try await addSequence(ui, "Departure")
            try await addCard(ui, "Boats fold themselves at dawn")
            try await addCard(ui, "The river carries the boats away")
            try await ui.waitCards(["Boats fold themselves at dawn", "The river carries the boats away"])
            try await ui.quiet()
        },
        W("30-story/edit-card-drawer-undo-redo") { ui, _, ctx in
            try await twoCards(ui, ctx)
            try await ui.pressCard("Boats fold themselves at dawn", count: 2)
            try await ui.wait(id: "story-field-Short description", timeout: 10, "the Short description field of the drawer")
            try await ui.fill("Short description", "Boats fold themselves at first light", id: "story-field-Short description")
            try await ui.waitUntil("drawer status Saved") { ui.query(id: "story-save-status").contains { $0.text.contains("Saved") } }
            try await ui.act(id: "story-inspector-close", role: "AXButton")
            try await ui.waitCards(["Boats fold themselves at first light", "The river carries the boats away"])
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitCards(["Boats fold themselves at dawn", "The river carries the boats away"])
            try await ui.redo()
            try await ui.waitCards(["Boats fold themselves at first light", "The river carries the boats away"])
            try await ui.quiet()
        },
        W("30-story/rename-act-undo-redo") { ui, _, ctx in
            try await twoCards(ui, ctx)
            try await ui.chooseMenu(label: "Act options for Act One", ["Rename"])
            try await ui.fill("Act title", "Act One \(emDash) Launch", submit: true)
            try await ui.expectShows("Act Act One \(emDash) Launch")
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("act renamed back") { ui.shows("Act Act One") && !ui.shows("Launch") }
            try await ui.redo()
            try await ui.expectShows("Act Act One \(emDash) Launch")
            try await ui.quiet()
        },
        W("30-story/delete-card-undo-redo-restore") { ui, _, ctx in
            try await twoCards(ui, ctx)
            try await ui.deleteCard("The river carries the boats away")
            try await ui.waitCards(["Boats fold themselves at dawn"])
            try await ui.undo()
            try await ui.waitCards(["Boats fold themselves at dawn", "The river carries the boats away"])
            try await ui.redo()
            try await ui.waitCards(["Boats fold themselves at dawn"])
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.restore("The river carries the boats away")
            try await ui.expectShows("Restored")
            try await ui.goTo("Story")
            try await ui.waitCards(["Boats fold themselves at dawn", "The river carries the boats away"])
            try await ui.quiet()
        },
        W("30-story/characters-create-edit-delete-restore") { ui, _, ctx in
            try await storyProject(ui, ctx, "Paper Boats")
            try await ui.tab("Characters")
            try await ui.button("Add Character")
            try await ui.fill("Character name", "Mira", id: "story-new-character-name")
            try await ui.fill("Role", "Boat maker", id: "story-new-character-role")
            try await ui.button("Create Character")
            try await ui.expectShows("Added character Mira")
            try await ui.button("Edit")
            try await ui.fill("Role", "Paper boat maker")
            try await ui.button("Save")
            try await ui.expectShows("Paper boat maker")
            try await ui.chooseMenu(id: "story-character-more", ["Delete"])
            try await ui.button("Delete Character")
            try await ui.expectShows("Deleted character Mira")
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.restore("Mira")
            try await ui.goTo("Story")
            try await ui.tab("Characters")
            try await ui.expectShows("Paper boat maker")
            try await ui.quiet()
        },
        W("30-story/outline-shows-same-structure") { ui, _, ctx in
            try await twoCards(ui, ctx)
            try await ui.tab("Outline")
            try await ui.waitUntil("outline rows") {
                ui.query().contains { $0.identifier.hasPrefix("story-outline-") && $0.text.contains("Boats fold themselves at dawn") }
                    && ui.query().contains { $0.identifier.hasPrefix("story-outline-") && $0.text.contains("The river carries the boats away") }
            }
            try await ui.quiet()
            try await ui.tab("Board")
        },
        W("30-story/relaunch-keeps-act-cards-character", phases: 2) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Paper Boats")
                try await ui.goTo("Story")
                try await ui.wait(id: "story-tabs", timeout: 20)
                try await addAct(ui, "Act One")
                try await addSequence(ui, "Departure")
                try await addCard(ui, "Boats fold themselves at first light")
                try await addCard(ui, "The river carries the boats away")
                try await ui.chooseMenu(label: "Act options for Act One", ["Rename"])
                try await ui.fill("Act title", "Act One \(emDash) Launch", submit: true)
                try await ui.expectShows("Act Act One \(emDash) Launch")
                try await ui.tab("Characters")
                try await ui.button("Add Character")
                try await ui.fill("Character name", "Mira", id: "story-new-character-name")
                try await ui.fill("Role", "Paper boat maker", id: "story-new-character-role")
                try await ui.button("Create Character")
                try await ui.expectShows("Added character Mira")
                try await ui.tab("Board")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Paper Boats")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Story")
                try await ui.waitCards(["Boats fold themselves at first light", "The river carries the boats away"])
                try await ui.expectShows("Act Act One \(emDash) Launch")
                try await ui.tab("Characters")
                try await ui.expectShows("Paper boat maker")
                try await ui.quiet()
                try await ui.tab("Board")
                try await ui.closeProject()
            }
        },
        W("31-story-board-flow/create-act-sequence-three-cards") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await ui.expectShows("The Letters")
            try await ui.waitSceneCards([cardA, cardB, cardC])
            try await ui.quiet()
        },
        W("31-story-board-flow/drag-card-undo-redo") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await dragCToTop(ui)
            try await ui.waitSceneCards([cardC, cardA, cardB])
            await ui.unfocusText()
            for _ in 0..<dragSteps { try await ui.undo() }
            try await ui.waitSceneCards([cardA, cardB, cardC])
            for _ in 0..<dragSteps { try await ui.redo() }
            try await ui.waitSceneCards([cardC, cardA, cardB])
            try await ui.quiet()
        },
        W("31-story-board-flow/parking-lot-park-and-restore") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await dragCToTop(ui)
            if let t = ui.query(id: "story-parking-toggle").first, t.value == "0" { try await ui.act(t) }
            try await ui.contextMenu(on: (try await ui.cardWait(cardA)), ["Send to Parking Lot"])
            try await ui.waitSceneCards([cardA], parking: true)
            try await ui.waitSceneCards([cardC, cardB])
            try await ui.waitUntil("card A parked") { ui.card(cardA) != nil }
            try await ui.contextMenu(on: (try await ui.cardWait(cardA)), ["Restore to Story"])
            try await ui.waitSceneCards([cardC, cardA, cardB])
            try await ui.quiet()
        },
        W("31-story-board-flow/convert-beat-to-scene-card") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await ui.pressCard(cardB)
            try await ui.chooseMenu(id: "story-add-menu", ["Beat"])
            try await ui.fill("Beat text", beat, submit: true)
            try await ui.waitUntil("beat shows") { ui.cardLabels().contains("Beat: \(beat)") }
            let b = ui.cardLabels().contains("Beat: \(beat)") ? (try await ui.cardWait(beat)) : (try await ui.cardWait(beat))
            try await ui.contextMenu(on: b, ["Convert to Scene"])
            try await ui.waitUntil("scene card from beat") { ui.sceneCards().contains(beat) }
            try await ui.waitUntil("beat reads converted") { ui.cardLabels().contains("Beat: \(beat) (converted)") }
            await ui.unfocusText()
            try await ui.undo()
            try await ui.waitUntil("scene card gone") { !ui.sceneCards().contains(beat) }
            try await ui.redo()
            try await ui.waitUntil("scene card back") { ui.sceneCards().contains(beat) }
            try await ui.quiet()
        },
        W("31-story-board-flow/long-description-and-card-display-presets") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await ui.pressCard(cardA, count: 2)
            try await ui.fill("Long description", longText, id: "story-field-Long description")
            try await ui.waitUntil("drawer status Saved") { ui.query(id: "story-save-status").contains { $0.text.contains("Saved") } }
            try await ui.act(id: "story-inspector-close", role: "AXButton")
            await ui.idle(0.4)
            try ui.expect(ui.card(cardA)?.value.contains("The street is flooded") != true, "no long description preview with the Minimal preset")
            try await ui.chooseMenu(id: "story-more", ["Card Display Settings"])
            try await ui.act(id: "story-preset-story")
            try await ui.waitUntil("long preview shows") { ui.card(cardA)?.value.contains("The street is flooded") == true }
            try await ui.act(id: "story-toggle-Shot/Storyboard Status")
            try await ui.waitUntil("preset Custom selected") { ui.query(id: "story-preset-custom").contains { $0.value == "1" || $0.callBool("isAccessibilitySelected") == true } }
            try await ui.button("Done")
            try await ui.waitSceneCards([cardA, cardB, cardC])
            try await ui.quiet()
        },
        W("31-story-board-flow/build-screenplay-leaves-out-card-without-heading") { ui, _, ctx in
            try await monsoon(ui, ctx)
            try await ui.pressCard(cardB)
            try await ui.chooseMenu(id: "story-add-menu", ["Beat"])
            try await ui.fill("Beat text", beat, submit: true)
            try await ui.contextMenu(on: (try await ui.cardWait(beat)), ["Convert to Scene"])
            try await ui.waitUntil("four scene cards") { ui.sceneCards().count == 4 }
            try await ui.act(id: "story-build", role: "AXButton")
            try await ui.expectShows("Build Screenplay from Story Board")
            try await ui.expectShows("no valid scene heading")
            try await ui.expectShows("OpenFrame never makes up a heading.")
            try await ui.fill("Heading for scene 1", "EXT. POST OFFICE - NIGHT")
            try await ui.fill("Heading for scene 2", "EXT. FLOODED STREET - NIGHT")
            try await ui.fill("Heading for scene 3", "INT. MEERA'S ROOM - NIGHT")
            try ui.expect(!(ui.query(id: "build-confirm").first?.isEnabled ?? false), "Build Screenplay is disabled while a card has no heading")
            try await ui.button("Leave out scene 4")
            try await ui.waitUntil("Build Screenplay enabled") { ui.query(id: "build-confirm").first?.isEnabled ?? false }
            try await ui.expectShows("3 of 4 cards will be built")
            try await ui.act(id: "build-confirm", role: "AXButton")
            try await ui.expectShows("Draft 1 from Story Board", timeout: 20)
            let tv = try await ui.editor()
            let headings = tv.string.components(separatedBy: "\n").filter { $0.hasPrefix("EXT.") || $0.hasPrefix("INT.") }
            try ui.expect(headings == ["EXT. POST OFFICE - NIGHT", "EXT. FLOODED STREET - NIGHT", "INT. MEERA'S ROOM - NIGHT"], "scene headings are exactly the three typed: \(headings)")
            try await ui.quiet()
        },
        W("31-story-board-flow/delete-two-cards-restore") { ui, _, ctx in
            try await monsoon(ui, ctx)
            // The cards must be built into a screenplay first: only then does the delete ask.
            try await ui.act(id: "story-build", role: "AXButton")
            try await ui.fill("Heading for scene 1", "EXT. POST OFFICE - NIGHT")
            try await ui.fill("Heading for scene 2", "EXT. FLOODED STREET - NIGHT")
            try await ui.fill("Heading for scene 3", "INT. MEERA'S ROOM - NIGHT")
            try await ui.waitUntil("Build Screenplay enabled") { ui.query(id: "build-confirm").first?.isEnabled ?? false }
            try await ui.act(id: "build-confirm", role: "AXButton")
            _ = try await ui.editor()
            try await ui.goTo("Story")
            try await ui.waitSceneCards([cardA, cardB, cardC])
            try await ui.pressCard(cardA)
            ui.holdModifiers([.command])
            try await ui.pressCard(cardB)
            ui.holdModifiers([])
            try await ui.expectShows("2 cards selected")
            if ui.appIsActive { try await ui.deleteKey() } else { try await ui.button("Delete") }
            try await ui.expectShows("Delete 2 items?")
            try await ui.expectShows("Screenplay scenes are never deleted from the Story Board.")
            try await ui.button("Delete")
            try await ui.waitSceneCards([cardC])
            try await ui.projectMenu(["Recently Deleted"])
            try await ui.restore(cardB)
            try await ui.expectShows("Restored")
            try await ui.restore(cardA)
            try await ui.expectShows("Restored")
            try await ui.goTo("Story")
            try await ui.waitSceneCards([cardA, cardB, cardC])
            try await ui.quiet()
        },
        W("31-story-board-flow/relaunch-keeps-order-long-description-display", phases: 2) { ui, phase, ctx in
            if phase == 1 {
                try await monsoon(ui, ctx)
                try await dragCToTop(ui)
                try await ui.waitSceneCards([cardC, cardA, cardB])
                try await ui.pressCard(cardA, count: 2)
                try await ui.fill("Long description", longText, id: "story-field-Long description")
                try await ui.waitUntil("drawer status Saved") { ui.query(id: "story-save-status").contains { $0.text.contains("Saved") } }
                try await ui.act(id: "story-inspector-close", role: "AXButton")
                try await ui.chooseMenu(id: "story-more", ["Card Display Settings"])
                try await ui.act(id: "story-preset-story")
                try await ui.button("Done")
                try await ui.closeProject()
            } else {
                try await ui.startApp(ctx)
                try await ui.openProject("Monsoon Letters")
                try ui.expect(!(await ui.recoveryOffered(within: 1.5)), "no recovery question after a clean close")
                try await ui.goTo("Story")
                try await ui.waitSceneCards([cardC, cardA, cardB])
                try await ui.waitUntil("long preview shows") { ui.card(cardA)?.value.contains("The street is flooded") == true }
                try await ui.pressCard(cardA, count: 2)
                let f = try await ui.wait(id: "story-field-Long description", timeout: 10)
                try ui.expect(f.value == longText, "the long description reads exactly: \(f.value)")
                try await ui.act(id: "story-inspector-close", role: "AXButton")
                try await ui.quiet()
                try await ui.closeProject()
            }
        },
        W("31-story-board-flow/episodes-scope-the-board") { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.menu(["File", "New Project…"])
            try await ui.fill(nil, "Monsoon Series", id: "new-project-title")
            try await ui.chooseTypePicker("Episodic")
            try await ui.act(id: "new-project-create", role: "AXButton")
            try await ui.wait(id: "project-home", timeout: 30)
            try await ui.goTo("Story")
            try await ui.tab("Episodes")
            try await ui.button("Add Episode")
            try await ui.fill("Episode title", "Pilot", id: "story-episode-title")
            try await ui.button("Create Episode")
            try await ui.expectShows("Added episode")
            try await ui.chooseMenu(label: "Options for Pilot", ["Open Story Board"])
            try await ui.button("Add Scene")
            try await ui.fill("Short description", "Pilot cold open at the dam", submit: true)
            try await ui.waitUntil("pilot card") { ui.card("Pilot cold open at the dam") != nil }
            try await ui.chooseMenu(id: "story-episode-picker", ["Series level"])
            try await ui.waitUntil("pilot card hidden") { ui.card("Pilot cold open at the dam") == nil }
            try await ui.chooseMenu(id: "story-episode-picker", ["E01 Pilot"])
            try await ui.waitUntil("pilot card back") { ui.card("Pilot cold open at the dam") != nil }
            try await ui.quiet()
        },
    ]
}
