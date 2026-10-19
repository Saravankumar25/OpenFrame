// The end-to-end driver: drives the real OpenFrame Studio app from inside its own process,
// through the same accessibility tree that VoiceOver and Apple's UI testing framework read.
//
// Why in process: the internal build machine has no full Xcode (so no XCUITest) and the test
// process holds no Accessibility permission. The runner (apps/macos/E2E/run.sh) injects this
// library into the release app with DYLD_INSERT_LIBRARIES. That works only on a build without
// the hardened runtime, which is the unsigned internal disk image; the store build turns the
// hardened runtime on and refuses injection. The driver never reads the app's Swift types: it
// sees only what a person or a screen reader sees, by identifier, role, label and value.

import AppKit
import ApplicationServices

/// One thing on screen, as the accessibility tree shows it.
struct Element: CustomStringConvertible {
    let obj: NSObject

    /// Table and outline rows answer only the older attribute API (accessibilityAttributeValue:),
    /// and so do their cell elements; read these through it.
    private func legacy(_ attribute: String) -> Any? {
        let sel = NSSelectorFromString("accessibilityAttributeValue:")
        guard obj.responds(to: sel) else { return nil }
        return obj.perform(sel, with: attribute)?.takeUnretainedValue()
    }

    private static let legacyNames = [
        "accessibilityIdentifier": "AXIdentifier", "accessibilityRole": "AXRole", "accessibilityLabel": "AXDescription",
        "accessibilityTitle": "AXTitle", "accessibilityValue": "AXValue",
    ]

    private func string(_ selector: String) -> String {
        let sel = NSSelectorFromString(selector)
        guard obj.responds(to: sel), let raw = obj.perform(sel)?.takeUnretainedValue() else {
            guard let name = Self.legacyNames[selector], let raw = legacy(name) else { return "" }
            if let s = raw as? String { return s }
            if let s = raw as? NSAttributedString { return s.string }
            if let n = raw as? NSNumber { return n.stringValue }
            return ""
        }
        if let s = raw as? String { return s }
        if let s = raw as? NSAttributedString { return s.string }
        if let n = raw as? NSNumber { return n.stringValue }
        return ""
    }

    var identifier: String { string("accessibilityIdentifier") }
    var role: String { string("accessibilityRole") }
    var label: String { string("accessibilityLabel") }
    var title: String { string("accessibilityTitle") }
    var value: String {
        if let tv = obj as? NSTextView { return tv.string }
        if let tf = obj as? NSTextField { return tf.stringValue }
        return string("accessibilityValue")
    }
    /// Everything a person could read on this element.
    var text: String { [label, title, value].filter { !$0.isEmpty }.joined(separator: " ") }

    var children: [Element] {
        let sel = NSSelectorFromString("accessibilityChildren")
        if obj.responds(to: sel), let kids = obj.perform(sel)?.takeUnretainedValue() as? [Any] {
            return kids.compactMap { ($0 as? NSObject).map(Element.init) }
        }
        guard let kids = legacy("AXChildren") as? [Any] else { return [] }
        return kids.compactMap { ($0 as? NSObject).map(Element.init) }
    }

    /// Calls an accessibility method that returns a Bool. SwiftUI's elements answer these
    /// methods without declaring the protocol, so a Swift cast would miss them.
    func callBool(_ selector: String) -> Bool? {
        let sel = NSSelectorFromString(selector)
        guard obj.responds(to: sel) else { return nil }
        typealias Fn = @convention(c) (AnyObject, Selector) -> Bool
        return unsafeBitCast(obj.method(for: sel), to: Fn.self)(obj, sel)
    }

    func setBool(_ selector: String, _ value: Bool) {
        let sel = NSSelectorFromString(selector)
        guard obj.responds(to: sel) else { return }
        typealias Fn = @convention(c) (AnyObject, Selector, Bool) -> Void
        unsafeBitCast(obj.method(for: sel), to: Fn.self)(obj, sel, value)
    }

    var frame: NSRect {
        let sel = NSSelectorFromString("accessibilityFrame")
        guard obj.responds(to: sel) else { return .zero }
        typealias Fn = @convention(c) (AnyObject, Selector) -> NSRect
        return unsafeBitCast(obj.method(for: sel), to: Fn.self)(obj, sel)
    }

    var isEnabled: Bool { callBool("isAccessibilityEnabled") ?? true }
    func performPress() -> Bool { callBool("accessibilityPerformPress") ?? false }

    var description: String { "\(role) id=\(identifier) label=\(label) title=\(title) value=\(value.prefix(60))" }
}

struct DriverError: Error, CustomStringConvertible {
    let description: String
    init(_ d: String) { description = d }
}

/// What a scenario uses to act on the app. Every wait has a deadline; nothing waits forever.
@MainActor
final class UI {
    let log: (String) -> Void
    let artifacts: URL
    private var shot = 0

    init(artifacts: URL, log: @escaping (String) -> Void) {
        self.artifacts = artifacts
        self.log = log
    }

    // MARK: accessibility tree

    /// SwiftUI builds its accessibility tree only after an accessibility client has asked for
    /// it once. Asking our own process needs no permission; it must not happen on the main
    /// thread, which answers the request.
    static func wakeAccessibility() {
        Thread.detachNewThread {
            let app = AXUIElementCreateApplication(getpid())
            var value: AnyObject?
            _ = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value)
            _ = AXUIElementCopyAttributeValue(app, kAXFocusedWindowAttribute as CFString, &value)
        }
    }

    var windows: [NSWindow] { NSApp.windows.filter { $0.isVisible } }

    func all() -> [Element] {
        var out: [Element] = []
        func walk(_ e: Element, _ depth: Int) {
            out.append(e)
            guard depth < 60 else { return }
            for c in e.children { walk(c, depth + 1) }
        }
        // Sheets are child windows: list the frontmost first so their controls win ties.
        for w in windows.reversed() {
            // The window's own tree holds the toolbar too; before SwiftUI has built it, fall
            // back to the content view.
            let root = Element(obj: w)
            if root.children.isEmpty, let cv = w.contentView { walk(Element(obj: cv), 0) } else { walk(root, 0) }
        }
        return out
    }

    func matches(_ e: Element, id: String?, label: String?, role: String?) -> Bool {
        if let id, e.identifier != id { return false }
        if let label, !(e.label == label || e.title == label || e.value == label) { return false }
        if let role, e.role != role { return false }
        return true
    }

    func query(id: String? = nil, label: String? = nil, role: String? = nil) -> [Element] {
        all().filter { matches($0, id: id, label: label, role: role) }
    }

    /// Lets the app run (layout, core replies, animations) for a moment.
    func idle(_ seconds: Double = 0.15) async {
        try? await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
    }

    @discardableResult
    func wait(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 15,
              _ what: String? = nil) async throws -> Element {
        let deadline = Date().addingTimeInterval(timeout)
        var attempts = 0
        while Date() < deadline {
            // Prefer an element whose role says it can be acted on; identifiers set on a
            // container are copied to its children by SwiftUI.
            let found = query(id: id, label: label, role: role)
            if let best = found.first(where: { ["AXButton", "AXTextField", "AXTextArea", "AXCheckBox", "AXPopUpButton", "AXLink", "AXMenuButton", "AXRadioButton", "AXCell", "AXRow"].contains($0.role) }) ?? found.first {
                return best
            }
            attempts += 1
            if attempts % 20 == 0 { UI.wakeAccessibility() }
            await idle(0.1)
        }
        try? snapshot("missing-\(id ?? label ?? "element")")
        throw DriverError("not found within \(Int(timeout)) s: \(what ?? "id=\(id ?? "-") label=\(label ?? "-") role=\(role ?? "-")")")
    }

    func waitGone(id: String? = nil, label: String? = nil, timeout: Double = 15) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if query(id: id, label: label).isEmpty { return }
            await idle(0.1)
        }
        throw DriverError("still on screen after \(Int(timeout)) s: id=\(id ?? "-") label=\(label ?? "-")")
    }

    func exists(id: String? = nil, label: String? = nil, role: String? = nil) -> Bool {
        !query(id: id, label: label, role: role).isEmpty
    }

    /// Waits until some element on screen shows this text (label, title or value).
    func waitText(_ text: String, timeout: Double = 15) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if all().contains(where: { $0.text.contains(text) }) { return }
            await idle(0.1)
        }
        try? snapshot("missing-text")
        throw DriverError("text not shown within \(Int(timeout)) s: \(text)")
    }

    func screenText() -> String { all().map(\.text).filter { !$0.isEmpty }.joined(separator: "\n") }

    // MARK: actions

    func press(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, role: role, timeout: timeout)
        try await press(e)
    }

    func press(_ e: Element) async throws {
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        guard e.isEnabled else { throw DriverError("control is disabled: \(e)") }
        if e.performPress() {
            await idle()
            return
        }
        // Some SwiftUI controls refuse the accessibility press (a button in a sheet, for one).
        // Click it as the mouse does.
        try await click(e)
    }

    /// A real left click in the middle of the element: mouse down and up sent to its window.
    func click(_ e: Element, count: Int = 1) async throws {
        let frame = e.frame
        guard frame.width > 0, frame.height > 0 else { throw DriverError("element has no frame: \(e)") }
        let center = NSPoint(x: frame.midX, y: frame.midY)
        // The frontmost window under the point takes the click.
        let ordered = NSApp.orderedWindows.filter { $0.isVisible && $0.frame.contains(center) }
        guard let w = ordered.first else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for i in 1...count {
            for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                guard let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: [],
                                                  timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w.windowNumber,
                                                  context: nil, eventNumber: 0, clickCount: i, pressure: type == .leftMouseDown ? 1 : 0) else { continue }
                NSApp.sendEvent(ev)
            }
        }
        await idle()
    }

    /// Focuses a text field or text view and replaces its text the way typing does, so bindings,
    /// validation and autosave all run.
    func type(_ text: String, into id: String? = nil, label: String? = nil, replace: Bool = true,
              timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, timeout: timeout)
        log("type into \(id ?? label ?? "?"): \(text.count) characters")
        try await focus(e)
        guard let target = firstResponderText() else { throw DriverError("no text input took focus: \(e)") }
        if replace { target.selectAll(nil) }
        target.insertText(text, replacementRange: target.selectedRange())
        await idle()
    }

    /// Types into whatever has keyboard focus now (the screenplay editor, for example).
    func typeHere(_ text: String) async throws {
        guard let target = firstResponderText() else { throw DriverError("nothing has keyboard focus") }
        target.insertText(text, replacementRange: target.selectedRange())
        await idle()
    }

    func focus(_ e: Element) async throws {
        if let v = e.obj as? NSView, let w = v.window {
            w.makeKeyAndOrderFront(nil)
            if v.acceptsFirstResponder { w.makeFirstResponder(v) }
        }
        e.setBool("setAccessibilityFocused:", true)
        await idle()
    }

    private func firstResponderText() -> NSTextView? {
        let w = NSApp.keyWindow ?? windows.last
        return w?.firstResponder as? NSTextView
    }

    /// Sends a key press to the key window, as the keyboard would.
    func key(_ chars: String, code: UInt16, modifiers: NSEvent.ModifierFlags = []) async throws {
        guard let w = NSApp.keyWindow ?? windows.last else { throw DriverError("no window for a key press") }
        for type in [NSEvent.EventType.keyDown, .keyUp] {
            guard let ev = NSEvent.keyEvent(with: type, location: .zero, modifierFlags: modifiers,
                                            timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: w.windowNumber,
                                            context: nil, characters: chars, charactersIgnoringModifiers: chars,
                                            isARepeat: false, keyCode: code) else { continue }
            NSApp.sendEvent(ev)
        }
        await idle()
    }

    func enter() async throws { try await key("\r", code: 36) }
    func tab() async throws { try await key("\t", code: 48) }
    func escape() async throws { try await key("\u{1b}", code: 53) }
    func shortcut(_ c: String, code: UInt16, _ mods: NSEvent.ModifierFlags = [.command]) async throws {
        try await key(c, code: code, modifiers: mods)
    }

    /// Chooses a menu bar item by its titles, e.g. ["File", "New Project…"].
    func menu(_ path: [String]) async throws {
        log("menu \(path.joined(separator: " > "))")
        guard var menu = NSApp.mainMenu else { throw DriverError("no main menu") }
        for (i, title) in path.enumerated() {
            refresh(menu)
            guard let item = menu.items.first(where: { $0.title == title }) else {
                throw DriverError("no menu item \(title) in \(menu.items.map(\.title))")
            }
            if i == path.count - 1 {
                guard item.isEnabled else { throw DriverError("menu item is disabled: \(title)") }
                let index = menu.index(of: item)
                menu.performActionForItem(at: index)
            } else {
                guard let sub = item.submenu else { throw DriverError("\(title) has no submenu") }
                menu = sub
            }
        }
        await idle(0.3)
    }

    /// SwiftUI fills and enables menu items when a menu is about to open; do the same.
    private func refresh(_ menu: NSMenu) {
        menu.delegate?.menuNeedsUpdate?(menu)
        NotificationCenter.default.post(name: NSMenu.didBeginTrackingNotification, object: menu)
        menu.update()
        NotificationCenter.default.post(name: NSMenu.didEndTrackingNotification, object: menu)
    }

    func menuItemEnabled(_ path: [String]) -> Bool {
        guard var menu = NSApp.mainMenu else { return false }
        for (i, title) in path.enumerated() {
            refresh(menu)
            guard let item = menu.items.first(where: { $0.title == title }) else { return false }
            if i == path.count - 1 { return item.isEnabled }
            guard let sub = item.submenu else { return false }
            menu = sub
        }
        return false
    }

    // MARK: evidence

    /// Saves a picture of every visible window, drawn by the app itself (no screen recording
    /// permission needed).
    func snapshot(_ name: String) throws {
        shot += 1
        for (i, w) in windows.enumerated() {
            guard let view = w.contentView?.superview ?? w.contentView else { continue }
            guard let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { continue }
            view.cacheDisplay(in: view.bounds, to: rep)
            guard let png = rep.representation(using: .png, properties: [:]) else { continue }
            let safe = name.replacingOccurrences(of: "/", with: "-")
            try png.write(to: artifacts.appendingPathComponent(String(format: "%02d-%@-w%d.png", shot, safe, i)))
        }
    }

    func expect(_ condition: Bool, _ message: @autoclosure () -> String) throws {
        if !condition { throw DriverError("expectation failed: \(message())") }
    }
}
