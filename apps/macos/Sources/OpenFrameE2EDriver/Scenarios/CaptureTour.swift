// The capture tour: visits every screen and state of the real Mac app and, for each one, writes a
// picture of the window and a list of every accessibility element into OPENFRAME_E2E_OUT/capture/.
// scripts/mac-html/build-mac-html.mjs turns those files into the MacHTML pages.
//
// Owned by the agent `capture`. Scenarios are named "zz-capture/<area>"; each one starts from an empty
// data folder, so each seeds what it needs. A step that cannot be reached is logged and skipped:
// one missing screen never stops the tour.

import AppKit
import Foundation

// MARK: capture primitive

@MainActor
enum CaptureState {
    /// Order of the first page of the running scenario; pages count up from here.
    static var base = 0
    static var count = 0
    static var names: Set<String> = []
    static var skipped: [String] = []
}

fileprivate extension UI {
    var captureDir: URL {
        let d = artifacts.appendingPathComponent("capture", isDirectory: true)
        try? FileManager.default.createDirectory(at: d, withIntermediateDirectories: true)
        return d
    }

    func captureBegin(order: Int) {
        CaptureState.base = order
        CaptureState.count = 0
    }

    /// The window a picture is about: the key window (or the parent of a key sheet), else the main window.
    func capturePrimary() -> NSWindow? {
        var w = NSApp.keyWindow
        while let parent = w?.sheetParent { w = parent }
        if let w, w.isVisible, w.frame.width > 200 { return w }
        let cands = NSApp.orderedWindows.filter { $0.isVisible && $0.sheetParent == nil && $0.frame.width > 200 && !$0.title.isEmpty && !String(describing: Swift.type(of: $0)).contains("Popup") }
        return cands.first ?? NSApp.windows.first { $0.isVisible && $0.canBecomeMain }
    }

    func captureRender(_ w: NSWindow) -> NSBitmapImageRep? {
        // The window server's own picture of the window is complete (materials, toolbar, sidebar);
        // drawing the views ourselves leaves the translucent parts blank.
        if w.windowNumber > 0,
           let cg = CGWindowListCreateImage(.null, .optionIncludingWindow, CGWindowID(w.windowNumber), [.boundsIgnoreFraming, .bestResolution]),
           cg.width > 10 {
            return NSBitmapImageRep(cgImage: cg)
        }
        guard let view = w.contentView?.superview ?? w.contentView else { return nil }
        guard let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else { return nil }
        view.cacheDisplay(in: view.bounds, to: rep)
        return rep
    }

    /// Windows drawn over the primary one: sheets and popovers.
    func captureOverlays(_ primary: NSWindow) -> [NSWindow] {
        var out: [NSWindow] = []
        func collect(_ w: NSWindow) {
            for s in w.sheets where s.isVisible { out.append(s); collect(s) }
            for c in w.childWindows ?? [] where c.isVisible && !out.contains(c) { out.append(c); collect(c) }
        }
        collect(primary)
        return out
    }

    func captureElements(_ root: NSWindow, origin: NSWindow, kind: String) -> [[String: Any]] {
        var list: [[String: Any]] = []
        let bounds = NSRect(origin: .zero, size: origin.frame.size)
        func walk(_ e: Element, _ depth: Int) {
            guard depth < 60 else { return }
            let f = e.frame
            let rel = NSRect(x: f.minX - origin.frame.minX, y: origin.frame.maxY - f.maxY, width: f.width, height: f.height)
            let role = e.role
            if f.width > 0, f.height > 0, rel.intersects(bounds), role != "AXWindow" || kind == "sheet" {
                var value = e.value
                if value.count > 4000 { value = String(value.prefix(4000)) }
                list.append([
                    "role": role, "id": e.identifier, "label": e.label, "title": e.title, "value": value,
                    "enabled": e.isEnabled, "window": kind, "depth": depth,
                    "x": Double(rel.minX), "y": Double(rel.minY), "w": Double(rel.width), "h": Double(rel.height),
                ])
            }
            for c in e.children { walk(c, depth + 1) }
        }
        let top = Element(obj: root)
        if top.children.isEmpty, let cv = root.contentView { walk(Element(obj: cv), 0) } else { walk(top, 0) }
        return list
    }

    /// Writes <name>.png and <name>.json for the screen as it is now. Never throws: a failed capture is logged.
    /// `menu` draws that menu open at `anchor` (window points, from the top left), since menus are not windows we can draw.
    func capture(_ name: String, title: String, area: String, from: String? = nil, via: String? = nil,
                 menu: NSMenu? = nil, anchor: CGPoint = CGPoint(x: 40, y: 60), settle: Double = 0.8) async {
        await idle(settle)
        guard CaptureState.names.insert(name).inserted else { log("capture \(name): already taken"); return }
        guard let primary = capturePrimary() else { log("capture \(name): no window"); return }
        let overlays = captureOverlays(primary)
        guard let base = captureRender(primary) else { log("capture \(name): could not draw"); return }
        let scale = CGFloat(base.pixelsWide) / primary.frame.width
        let size = primary.frame.size
        guard let out = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: base.pixelsWide, pixelsHigh: base.pixelsHigh, bitsPerSample: 8,
                                         samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                                         bytesPerRow: 0, bitsPerPixel: 0),
              let ctx = NSGraphicsContext(bitmapImageRep: out) else { log("capture \(name): no canvas"); return }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = ctx
        ctx.imageInterpolation = .high
        let canvas = NSRect(x: 0, y: 0, width: CGFloat(out.pixelsWide), height: CGFloat(out.pixelsHigh))
        base.draw(in: canvas)
        var elements = captureElements(primary, origin: primary, kind: "main")
        for o in overlays {
            guard let rep = captureRender(o) else { continue }
            let r = NSRect(x: (o.frame.minX - primary.frame.minX) * scale, y: (o.frame.minY - primary.frame.minY) * scale,
                           width: o.frame.width * scale, height: o.frame.height * scale)
            if o.sheetParent != nil {
                NSColor.black.withAlphaComponent(0.10).setFill()
                canvas.fill(using: .sourceOver)
            }
            let shadow = NSShadow()
            shadow.shadowBlurRadius = 18 * scale
            shadow.shadowOffset = NSSize(width: 0, height: -6 * scale)
            shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
            NSGraphicsContext.saveGraphicsState()
            shadow.set()
            rep.draw(in: r)
            NSGraphicsContext.restoreGraphicsState()
            elements.append(contentsOf: captureElements(o, origin: primary, kind: "sheet"))
        }
        if let menu { elements.append(contentsOf: drawMenu(menu, at: anchor, scale: scale, canvasHeight: canvas.height)) }
        NSGraphicsContext.restoreGraphicsState()
        guard let png = out.representation(using: .png, properties: [:]) else { log("capture \(name): no png"); return }
        CaptureState.count += 1
        let order = CaptureState.base + CaptureState.count
        // The colour the window is mostly drawn in, so the HTML layer alone can use it as a backdrop.
        var counts: [String: Int] = [:]
        for gx in 0..<14 { for gy in 0..<8 {
            let px = Int(Double(base.pixelsWide) * (0.52 + 0.46 * Double(gx) / 14)), py = Int(Double(base.pixelsHigh) * (0.2 + 0.7 * Double(gy) / 8))
            if let c = base.colorAt(x: min(px, base.pixelsWide - 1), y: min(py, base.pixelsHigh - 1))?.usingColorSpace(.sRGB) {
                counts[String(format: "#%02x%02x%02x", Int(c.redComponent * 255), Int(c.greenComponent * 255), Int(c.blueComponent * 255)), default: 0] += 1
            }
        } }
        let bg = counts.max(by: { $0.value < $1.value })?.key ?? "#1c1c1e"
        let dark = primary.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        let meta: [String: Any] = [
            "bg": bg, "dark": dark, "name": name, "title": title, "area": area, "order": order, "from": from ?? "", "via": via ?? "",
            "windowTitle": primary.title, "width": Double(size.width), "height": Double(size.height), "scale": Double(scale),
            "sheets": overlays.count, "elements": elements,
        ]
        do {
            try png.write(to: captureDir.appendingPathComponent("\(name).png"))
            let json = try JSONSerialization.data(withJSONObject: meta, options: [.sortedKeys])
            try json.write(to: captureDir.appendingPathComponent("\(name).json"))
            log("captured \(name) (\(elements.count) elements, \(png.count / 1024) KB)")
        } catch {
            log("capture \(name): write failed \(error)")
        }
    }

    /// Draws an open menu the way macOS shows it, and returns its items as elements.
    /// Must be called with the bitmap context current. Coordinates: window points from the top left.
    func drawMenu(_ menu: NSMenu, at anchor: CGPoint, scale: CGFloat, canvasHeight: CGFloat) -> [[String: Any]] {
        let font = NSFont.menuFont(ofSize: 13)
        let rowH: CGFloat = 22, sepH: CGFloat = 9, padX: CGFloat = 14, padY: CGFloat = 6
        let items = menu.items.filter { !$0.isHidden }
        func shortcut(_ i: NSMenuItem) -> String {
            guard !i.keyEquivalent.isEmpty else { return "" }
            var s = ""
            let m = i.keyEquivalentModifierMask
            if m.contains(.control) { s += "\u{2303}" }
            if m.contains(.option) { s += "\u{2325}" }
            if m.contains(.shift) { s += "\u{21E7}" }
            if m.contains(.command) { s += "\u{2318}" }
            return s + i.keyEquivalent.uppercased()
        }
        let attrs: [NSAttributedString.Key: Any] = [.font: font]
        var left: CGFloat = 0, right: CGFloat = 0
        for i in items where !i.isSeparatorItem {
            left = max(left, (i.title as NSString).size(withAttributes: attrs).width + (i.state != .off ? 16 : 0))
            right = max(right, (shortcut(i) as NSString).size(withAttributes: attrs).width + (i.submenu != nil ? 14 : 0))
        }
        let width = ceil(left + right + padX * 2 + 24)
        let height = items.reduce(padY * 2) { $0 + ($1.isSeparatorItem ? sepH : rowH) }
        let winH = canvasHeight / scale
        let panel = NSRect(x: anchor.x, y: winH - anchor.y - height, width: width, height: height)
        let dark = (capturePrimary() ?? NSApp.windows.first)?.effectiveAppearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
        let bg = dark ? NSColor(white: 0.17, alpha: 0.97) : NSColor(white: 0.97, alpha: 0.97)
        let fg = dark ? NSColor.white : NSColor.black
        NSGraphicsContext.saveGraphicsState()
        let xf = NSAffineTransform()
        xf.scaleX(by: scale, yBy: scale)
        xf.concat()
        let shadow = NSShadow()
        shadow.shadowBlurRadius = 14
        shadow.shadowOffset = NSSize(width: 0, height: -5)
        shadow.shadowColor = NSColor.black.withAlphaComponent(0.4)
        NSGraphicsContext.saveGraphicsState()
        shadow.set()
        bg.setFill()
        NSBezierPath(roundedRect: panel, xRadius: 8, yRadius: 8).fill()
        NSGraphicsContext.restoreGraphicsState()
        (dark ? NSColor(white: 1, alpha: 0.18) : NSColor(white: 0, alpha: 0.18)).setStroke()
        NSBezierPath(roundedRect: panel.insetBy(dx: 0.5, dy: 0.5), xRadius: 8, yRadius: 8).stroke()
        var y = panel.maxY - padY
        var elements: [[String: Any]] = []
        for (n, i) in items.enumerated() {
            if i.isSeparatorItem {
                (dark ? NSColor(white: 1, alpha: 0.2) : NSColor(white: 0, alpha: 0.15)).setFill()
                NSRect(x: panel.minX + 10, y: y - sepH / 2, width: panel.width - 20, height: 1).fill()
                y -= sepH
                continue
            }
            let color = i.isEnabled ? fg : fg.withAlphaComponent(0.35)
            let a: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
            let row = NSRect(x: panel.minX, y: y - rowH, width: panel.width, height: rowH)
            if i.state != .off { ("\u{2713}" as NSString).draw(at: NSPoint(x: row.minX + 6, y: row.minY + 3), withAttributes: a) }
            (i.title as NSString).draw(at: NSPoint(x: row.minX + padX + (i.state != .off ? 6 : 0), y: row.minY + 3), withAttributes: a)
            let sc = shortcut(i)
            if !sc.isEmpty {
                let w = (sc as NSString).size(withAttributes: a).width
                (sc as NSString).draw(at: NSPoint(x: row.maxX - padX - w, y: row.minY + 3), withAttributes: a)
            }
            if i.submenu != nil { ("\u{203A}" as NSString).draw(at: NSPoint(x: row.maxX - padX, y: row.minY + 3), withAttributes: a) }
            elements.append([
                "role": "AXMenuItem", "id": "menu-item-\(n)", "label": i.title, "title": i.title, "value": sc,
                "enabled": i.isEnabled, "window": "menu", "depth": 0,
                "x": Double(row.minX), "y": Double(winH - row.maxY), "w": Double(row.width), "h": Double(rowH),
            ])
            y -= rowH
        }
        NSGraphicsContext.restoreGraphicsState()
        return elements
    }
}

// MARK: helpers (only for this file)

fileprivate extension UI {
    /// Sets Settings, Appearance, Mode (default theme) to Light or Dark, as the pass asks (OPENFRAME_CAPTURE_MODE).
    func capSetMode() async {
        let want = (ProcessInfo.processInfo.environment["OPENFRAME_CAPTURE_MODE"] ?? "dark").lowercased() == "light" ? "Light" : "Dark"
        await step("set mode \(want)") {
            try await menu([appMenu, "Settings\u{2026}"])
            await idle(1.0)
            try await act(label: "Appearance")
            await idle(0.8)
            try await act(label: want, role: "AXRadioButton")
            await idle(1.0)
            await capRecover()
            await idle(0.6)
            // The Settings window is gone: the project window takes the keyboard again, or typing finds no focus.
            if let main = NSApp.windows.filter({ $0.isVisible && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) {
                NSApp.activate(ignoringOtherApps: true)
                main.makeKeyAndOrderFront(nil)
            }
            await idle(0.6)
        }
    }

    func shows(_ text: String) -> Bool { all().contains { $0.text.contains(text) } }

    func queryPrefix(_ prefix: String) -> [Element] { all().filter { $0.identifier.hasPrefix(prefix) } }

    func waitUntil(_ what: String, timeout: Double = 15, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return }
            await idle(0.1)
        }
        throw DriverError("not true within \(Int(timeout)) s: \(what)")
    }

    /// Makes the main window a fixed, roomy size so every page is the same shape.
    func capBigWindow(width: CGFloat = 1440, height: CGFloat = 900) {
        guard let w = NSApp.windows.filter({ $0.isVisible && $0.canBecomeMain && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) else { return }
        let screen = w.screen ?? NSScreen.main
        let vf = screen?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1728, height: 1080)
        // Larger than the screen is allowed: the window keeps its size, the top left stays on screen.
        let target = NSRect(x: vf.minX, y: vf.maxY - height, width: width, height: height)
        if w.frame != target { w.setFrame(target, display: true) }
    }

    func postClick(atScreen center: NSPoint) {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) && !String(describing: Swift.type(of: $0)).contains("PopupMenu") }) else { return }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        for t in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
            if let ev = NSEvent.mouseEvent(with: t, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                           windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: t == .leftMouseDown ? 1 : 0) {
                NSApp.postEvent(ev, atStart: false)
            }
        }
    }

    /// The menu a pop-up button owns (a SwiftUI Menu or Picker), read without opening it.
    func popUpMenu(id: String? = nil, label: String? = nil) -> NSMenu? {
        let e = query(id: id, label: label).first { $0.obj is NSPopUpButtonCell }
        return (e?.obj as? NSPopUpButtonCell)?.menu
    }

    func menuButton(id: String? = nil, label: String? = nil, item: String, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, timeout: timeout, "the menu button \(id ?? label ?? "?")")
        guard let cell = e.obj as? NSPopUpButtonCell else { throw DriverError("not a menu button: \(e)") }
        func find(_ m: NSMenu) -> (NSMenu, Int)? {
            for (i, it) in m.items.enumerated() where !it.isSeparatorItem && it.title.hasPrefix(item) { return (m, i) }
            for it in m.items { if let sub = it.submenu, let r = find(sub) { return r } }
            return nil
        }
        var target: (NSMenu, Int)?
        let menusBefore = NSApp.windows.filter { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }.count
        for _ in 0..<3 where target == nil {
            let frame = e.frame
            postClick(atScreen: NSPoint(x: frame.midX, y: frame.midY))
            let deadline = Date().addingTimeInterval(2)
            while Date() < deadline {
                if let m = cell.menu, let t = find(m) { target = t; break }
                await idle(0.1)
            }
            if target == nil { cell.menu?.cancelTracking(); await idle(0.3) }
        }
        guard let (menu, index) = target else {
            let titles = cell.menu?.items.map(\.title) ?? []
            cell.menu?.cancelTracking()
            throw DriverError("no menu item starting with \(item) in \(titles)")
        }
        guard menu.items[index].isEnabled else { cell.menu?.cancelTracking(); throw DriverError("menu item is disabled: \(menu.items[index].title)") }
        menu.performActionForItem(at: index)
        cell.menu?.cancelTracking()
        await idle(0.3)
        func openMenus() -> Int { NSApp.windows.filter { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }.count }
        let inToolbar = NSApp.windows.contains { $0.isVisible && e.frame.midY > $0.frame.maxY - 60 && $0.frame.contains(NSPoint(x: e.frame.midX, y: e.frame.midY)) }
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("PopupMenu") && openMenus() > menusBefore { w.orderOut(nil) }
        for _ in 0..<10 where inToolbar && openMenus() > menusBefore {
            for t in [NSEvent.EventType.keyDown, .keyUp] {
                if let ev = NSEvent.keyEvent(with: t, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                             windowNumber: 0, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                    NSApp.postEvent(ev, atStart: true)
                }
            }
            cell.menu?.cancelTracking()
            await idle(0.3)
        }
    }

    /// Opens a workspace with the Go menu, else by selecting the sidebar row.
    func capNav(_ workspace: String) async throws {
        capBigWindow()
        try await go(workspace)
        await idle(0.2)
    }

    func capNavByRow(_ workspace: String) async throws {
        let order = ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"]
        guard let outline = try? await wait(label: "Sidebar", timeout: 10, "the sidebar"), let view = outline.obj as? NSOutlineView else {
            throw DriverError("no sidebar to click \(workspace)")
        }
        guard let idx = order.firstIndex(of: workspace) else { throw DriverError("unknown workspace \(workspace)") }
        view.window?.makeFirstResponder(view)
        view.scrollRowToVisible(idx)
        view.selectRowIndexes(IndexSet(integer: idx), byExtendingSelection: false)
        await idle(0.7)
    }

    func typeIn(_ text: String, into id: String? = nil, label: String? = nil, timeout: Double = 15) async throws {
        var e = try await wait(id: id, label: label, timeout: timeout)
        func window(of e: Element) -> NSWindow? {
            let c = NSPoint(x: e.frame.midX, y: e.frame.midY)
            return NSApp.orderedWindows.first { $0.isVisible && $0.frame.contains(c) }
        }
        func hasFocus(_ e: Element) -> NSTextView? {
            guard let w = window(of: e), let tv = w.firstResponder as? NSTextView else { return nil }
            let owner: NSObject = (tv.isFieldEditor ? (tv.delegate as? NSObject) : tv) ?? tv
            let o = Element(obj: owner)
            if owner === e.obj || (!e.label.isEmpty && o.label == e.label) || (!e.identifier.isEmpty && o.identifier == e.identifier) { return tv }
            return nil
        }
        var target: NSTextView?
        for attempt in 0..<5 {
            window(of: e)?.makeKeyAndOrderFront(nil)
            if attempt >= 2 { try await focus(e) } else if e.frame.width > 0 { postClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)) }
            await idle(0.3)
            if let tv = hasFocus(e) { target = tv; break }
            if e.frame.width > 0 { postClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)); await idle(0.4) }
            if let tv = hasFocus(e) { target = tv; break }
            if attempt >= 3, let w = window(of: e), let tv = w.firstResponder as? NSTextView { target = tv; break }
            e = try await wait(id: id, label: label, timeout: timeout)
        }
        guard let tv = target else { throw DriverError("no text input took focus: \(e)") }
        tv.selectAll(nil)
        tv.insertText(text, replacementRange: tv.selectedRange())
        await idle(0.2)
    }

    func windowElements(_ w: NSWindow) -> [Element] {
        var out: [Element] = []
        func walk(_ e: Element, _ d: Int) { out.append(e); if d < 40 { for c in e.children { walk(c, d + 1) } } }
        walk(Element(obj: w), 0)
        return out
    }

    func escapeTo(_ w: NSWindow) async {
        w.makeKeyAndOrderFront(nil)
        for t in [NSEvent.EventType.keyDown, .keyUp] {
            if let ev = NSEvent.keyEvent(with: t, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                         windowNumber: w.windowNumber, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                NSApp.sendEvent(ev)
            }
        }
        await idle(0.4)
    }

    /// Closes anything left open: sheets, drawers, inspectors, the Settings and About windows.
    func capRecover() async {
        for w in NSApp.windows { if let panel = w as? NSSavePanel, panel.isVisible { panel.cancel(nil); await idle(0.4) } }
        for _ in 0..<5 {
            guard let sh = NSApp.windows.last(where: { $0.isVisible && $0.sheetParent != nil }) else { break }
            let cancel = windowElements(sh).first { $0.role == "AXButton" && ["Cancel", "Close", "Done", "Not Now", "OK", "Cancel Import"].contains($0.label.isEmpty ? $0.title : $0.label) && $0.isEnabled }
            if let cancel { _ = cancel.performPress() ; await idle(0.5) } else { await escapeTo(sh) }
        }
        for id in ["drawer-close", "note-close", "task-close", "file-close", "story-inspector-close", "inspector-close", "moodboard-inspector-close", "panel-inspector-close", "shot-inspector-close", "export-close", "callsheet-refresh-cancel"] {
            if exists(id: id), let e = query(id: id).first(where: { $0.role == "AXButton" }) { _ = e.performPress(); await idle(0.4) }
        }
        for w in NSApp.windows where w.isVisible && w.sheetParent == nil && (w.title == "Settings" || ["General", "Appearance", "Offline AI", "Diagnostics", "About OpenFrame Studio"].contains(w.title)) { w.close() }
        await idle(0.3)
    }

    /// Runs one tour step. A failure is logged and recorded; the tour goes on.
    func step(_ what: String, _ body: () async throws -> Void) async {
        do { try await body() } catch {
            log("SKIPPED \(what): \(error)")
            let safe = what.map { $0.isLetter || $0.isNumber ? String($0) : "-" }.joined().prefix(60)
            try? all().map(\.description).joined(separator: "\n").write(to: captureDir.appendingPathComponent("tree-\(safe).txt"), atomically: true, encoding: .utf8)
            let line = "\(what)\t\(error)".replacingOccurrences(of: "\n", with: " ") + "\n"
            let url = captureDir.appendingPathComponent("skipped.tsv")
            if let h = try? FileHandle(forWritingTo: url) { h.seekToEndOfFile(); h.write(Data(line.utf8)); try? h.close() }
            else { try? line.write(to: url, atomically: true, encoding: .utf8) }
            await capRecover()
        }
    }

    /// Presses a control if it is on screen. Returns false when it is not there.
    @discardableResult
    func tryPress(id: String? = nil, label: String? = nil, wait seconds: Double = 3) async -> Bool {
        guard let e = try? await wait(id: id, label: label, timeout: seconds) else { return false }
        do { try await press(e); return true } catch { return false }
    }

    /// Starts a tour scenario: the app, a project called Night Bus, and the bundled example screenplay.
    func capSeed(_ ctx: Context, order: Int, importSample: Bool = true) async throws {
        captureBegin(order: order)
        try await startApp(ctx)
        capBigWindow()
        await capSetMode()
        capBigWindow()
        try await createProject("Night Bus")
        if importSample {
            try await menu(["File", "Import Screenplay\u{2026}"])
            try await press(label: "Example Short", role: "AXButton")
            try await press(id: "import-preview")
            try await waitText("Import Screenplay \u{00B7} Preview")
            try await press(id: "import-confirm")
            try await waitText("Import complete")
            try await press(label: "Open Screenplay", role: "AXButton")
            _ = try await wait(id: "screenplay-editor", timeout: 30)
            await idle(0.8)
        }
    }
}


// MARK: more helpers

fileprivate extension UI {
    func act(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 8) async throws {
        let e = try await wait(id: id, label: label, role: role, timeout: timeout)
        guard e.isEnabled else { throw DriverError("control is disabled: \(e)") }
        log("press \(e.identifier.isEmpty ? e.label : e.identifier)")
        if e.performPress() { await idle(0.3); return }
        try await tap(e)
    }

    func tap(_ e: Element, count: Int = 1, right: Bool = false) async throws {
        let f = e.frame
        guard f.width > 0, f.height > 0 else { throw DriverError("element has no frame: \(e)") }
        let center = NSPoint(x: f.midX, y: f.midY)
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }) else { throw DriverError("no window under \(e)") }
        w.makeKeyAndOrderFront(nil)
        let local = w.convertPoint(fromScreen: center)
        let down: NSEvent.EventType = right ? .rightMouseDown : .leftMouseDown
        let up: NSEvent.EventType = right ? .rightMouseUp : .leftMouseUp
        for i in 1...count {
            for type in [down, up] {
                if let ev = NSEvent.mouseEvent(with: type, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                               windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: i, pressure: type == down ? 1 : 0) {
                    NSApp.postEvent(ev, atStart: false)
                }
            }
            await idle(0.08)
        }
        await idle(0.2)
    }

    func fill(_ label: String? = nil, _ text: String, id: String? = nil, submit: Bool = false) async throws {
        let e = try await wait(id: id, label: label, timeout: 8)
        if let tv = e.obj as? NSTextView, let w = tv.window { w.makeKeyAndOrderFront(nil); w.makeFirstResponder(tv) }
        else if let tf = (e.obj as? NSTextField) ?? ((e.obj as? NSCell)?.controlView as? NSTextField), let w = tf.window { w.makeKeyAndOrderFront(nil); w.makeFirstResponder(tf) }
        else { try await tap(e); if !(((NSApp.keyWindow ?? windows.last)?.firstResponder) is NSTextView) { try await focus(e) } }
        await idle(0.15)
        guard let w = NSApp.keyWindow ?? windows.last, let target = w.firstResponder as? NSTextView else { throw DriverError("no keyboard focus for \(id ?? label ?? "?")") }
        target.selectAll(nil)
        target.insertText(text, replacementRange: target.selectedRange())
        await idle(0.2)
        if submit { try await enter() }
    }

    func tab(_ name: String) async throws {
        try await act(label: name, role: "AXRadioButton")
        await idle(0.5)
    }

    /// Labels of the radio buttons in a group (the tabs of a workspace).
    func radioLabels(group id: String) -> [String] {
        guard let g = query(id: id).first(where: { $0.role == "AXRadioGroup" }) else { return [] }
        return g.children.filter { $0.role == "AXRadioButton" }.map { $0.label }
    }

    /// A frame in the main window's own coordinates, from the top left.
    func relFrame(_ e: Element) -> CGRect? {
        guard let w = capturePrimary() else { return nil }
        let f = e.frame
        return CGRect(x: f.minX - w.frame.minX, y: w.frame.maxY - f.maxY, width: f.width, height: f.height)
    }

    /// Opens a pop-up menu button the way a click does, copies its items, and closes it again.
    func peekMenu(_ e: Element) async throws -> NSMenu {
        guard let cell = e.obj as? NSPopUpButtonCell else { throw DriverError("not a menu button: \(e)") }
        var copy: NSMenu?
        func openMenus() -> Int { NSApp.windows.filter { $0.isVisible && String(describing: Swift.type(of: $0)).contains("PopupMenu") }.count }
        let before = openMenus()
        for _ in 0..<3 where copy == nil {
            postClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY))
            let deadline = Date().addingTimeInterval(2)
            while Date() < deadline {
                if let m = cell.menu, m.items.contains(where: { !$0.title.isEmpty || $0.isSeparatorItem }) { copy = m.copy() as? NSMenu; break }
                await idle(0.1)
            }
            if copy == nil { cell.menu?.cancelTracking(); await idle(0.3) }
        }
        // The open menu is a window: take the picture of the real thing too when we can.
        cell.menu?.cancelTracking()
        await idle(0.3)
        for w in NSApp.windows where w.isVisible && String(describing: Swift.type(of: w)).contains("PopupMenu") && openMenus() > before { w.orderOut(nil) }
        for _ in 0..<10 where openMenus() > before {
            for t in [NSEvent.EventType.keyDown, .keyUp] {
                if let ev = NSEvent.keyEvent(with: t, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                             windowNumber: 0, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                    NSApp.postEvent(ev, atStart: true)
                }
            }
            cell.menu?.cancelTracking()
            await idle(0.3)
        }
        guard let copy else { throw DriverError("the menu did not open: \(e)") }
        return copy
    }

    /// Shows a pop-up menu button's menu open, drawn from its real items.
    func capMenu(id: String? = nil, label: String? = nil, name: String, title: String, area: String, from: String? = nil) async {
        await step("menu \(name)") {
            let e = try await wait(id: id, label: label, timeout: 6)
            guard let r = relFrame(e) else { throw DriverError("no frame: \(e)") }
            let menu = try await peekMenu(e)
            let w = capturePrimary()?.frame.width ?? 1200
            await capture(name, title: title, area: area, from: from, via: id ?? label, menu: menu,
                          anchor: CGPoint(x: min(r.minX, w - 260), y: r.maxY + 4))
        }
    }

    /// Presses a control that opens something, captures it, then closes it.
    func visit(press: String? = nil, label: String? = nil, role: String? = nil, name: String, title: String, area: String,
               from: String? = nil, close: String? = nil, closeLabel: String? = nil, settle: Double = 0.6) async {
        await step("visit \(name)") {
            try await act(id: press, label: label, role: role)
            await capture(name, title: title, area: area, from: from, via: press ?? label, settle: settle)
            if let close, await tryPress(id: close, wait: 1.5) {} else if let closeLabel, await tryPress(label: closeLabel, wait: 1.5) {} else { await capRecover() }
            await idle(0.3)
        }
    }

    /// Captures every tab of a radio group, returning to the first.
    func eachTab(group: String, area: String, prefix: String, from: String? = nil, skip: Set<String> = []) async {
        let labels = radioLabels(group: group)
        var last = from
        for l in labels where !skip.contains(l) {
            await step("tab \(l)") {
                try await tab(l)
                let n = "\(prefix)-\(slug(l))"
                await capture(n, title: "\(area): \(l)", area: area, from: last, via: l)
                last = n
            }
        }
        if let f = labels.first { try? await tab(f) }
    }

    func card(_ text: String) -> Element? {
        all().first { $0.identifier.hasPrefix("story-card-") && ($0.label.hasPrefix("Scene Card: \(text)") || $0.label.hasPrefix("Beat: \(text)")) }
    }
}

fileprivate extension UI {
    /// The context menu a right click on a point opens, as the hosting view builds it.
    func contextMenuItems(at center: NSPoint) -> NSMenu? {
        guard let w = NSApp.orderedWindows.first(where: { $0.isVisible && $0.frame.contains(center) }), let content = w.contentView else { return nil }
        let local = w.convertPoint(fromScreen: center)
        guard let ev = NSEvent.mouseEvent(with: .rightMouseDown, location: local, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                          windowNumber: w.windowNumber, context: nil, eventNumber: 0, clickCount: 1, pressure: 1) else { return nil }
        var v = content.hitTest(content.superview?.convert(local, from: nil) ?? local)
        while let vv = v {
            if let m = vv.menu(for: ev), !m.items.isEmpty { m.update(); return m }
            v = vv.superview
        }
        return nil
    }

    /// Captures the right-click menu of an element, drawn from its real items.
    func capContext(on e: Element, name: String, title: String, area: String, from: String? = nil) async {
        await step("context menu \(name)") {
            let f = e.frame
            guard let menu = contextMenuItems(at: NSPoint(x: f.midX, y: f.midY)), let r = relFrame(e) else { throw DriverError("no context menu on \(e)") }
            await capture(name, title: title, area: area, from: from, via: e.identifier, menu: menu.copy() as? NSMenu ?? menu,
                          anchor: CGPoint(x: r.midX, y: r.midY))
        }
    }

    /// Captures one menu of the menu bar, drawn open under its title.
    func capMainMenu(_ title: String, name: String, area: String = "Menus", from: String? = nil) async {
        await step("main menu \(title)") {
            guard let top = NSApp.mainMenu, let item = (title == "OpenFrame Studio" ? top.items.first : top.items.first(where: { $0.title == title })), let menu = item.submenu else { throw DriverError("no menu \(title)") }
            menu.delegate?.menuNeedsUpdate?(menu)
            NotificationCenter.default.post(name: NSMenu.didBeginTrackingNotification, object: menu)
            menu.update()
            NotificationCenter.default.post(name: NSMenu.didEndTrackingNotification, object: menu)
            let index = top.items.filter { !$0.isHidden }.firstIndex(of: item) ?? 1
            await capture(name, title: "\(title) menu", area: area, from: from, via: "menu-bar", menu: menu.copy() as? NSMenu ?? menu,
                          anchor: CGPoint(x: 20 + CGFloat(max(0, index - 1)) * 62, y: 30))
        }
    }

    var appMenu: String { NSApp.mainMenu?.items.first?.title ?? "OpenFrame Studio" }

    func customAction(_ e: Element, _ name: String) async throws {
        let sel = NSSelectorFromString("accessibilityCustomActions")
        guard e.obj.responds(to: sel), let list = e.obj.perform(sel)?.takeUnretainedValue() as? [NSAccessibilityCustomAction] else {
            throw DriverError("no accessibility actions on \(e)")
        }
        guard let a = list.first(where: { $0.name == name }) else { throw DriverError("no action \(name) in \(list.map(\.name))") }
        _ = a.handler?()
        await idle(0.4)
    }

    /// Chooses an item of a pop-up button by its title (from the start).
    func choose(_ id: String, _ item: String) async throws { try await menuButton(id: id, item: item) }

    func projectMenuPeek(name: String, area: String = "Project", from: String? = nil) async {
        await capMenu(id: "project-menu", name: name, title: "Project menu", area: area, from: from)
    }
}

fileprivate extension UI {
    static let unsafeWords = ["delete", "remove", "pdf", "csv", "save", "reveal", "finder", "choose", "browse", "plain text", "forget", "clear", "reset", "purge", "empty", "stop", "record", "print\u{2026}", "focus", "image", "file", "video", "open externally", "relink", "replace", "export csv"]

    func slug(_ t: String) -> String {
        let allowed = t.lowercased().map { $0.isLetter || $0.isNumber ? String($0) : "-" }.joined()
        var out = allowed
        while out.contains("--") { out = out.replacingOccurrences(of: "--", with: "-") }
        return out.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    }

    func leafTitles(_ m: NSMenu) -> [String] {
        var out: [String] = []
        for i in m.items where !i.isSeparatorItem && i.isEnabled && !i.isHidden {
            if let sub = i.submenu { out.append(contentsOf: leafTitles(sub)) } else { out.append(i.title) }
        }
        return out
    }

    func vaultRow(_ i: Int) async throws {
        let titles = ["All items", "Pinned", "Recently added", "Recently modified"]
        guard i < titles.count, let e = query(id: "vault-nav-\(titles[i])").first(where: { $0.frame.width > 0 }) else { throw DriverError("no vault row \(i)") }
        try await tap(e)
        await idle(0.9)
    }

    var hasOverlay: Bool { NSApp.windows.contains { $0.isVisible && $0.sheetParent != nil } }

    /// Captures a menu open, then every safe item of it in turn (what it opens), closing each again.
    func exploreMenu(_ id: String, area: String, prefix: String, from: String?, extraSkip: [String] = [], limit: Int = 14) async {
        var titles: [String] = []
        await step("explore \(id)") {
            let e = try await wait(id: id, timeout: 6)
            guard let r = relFrame(e) else { throw DriverError("no frame") }
            let menu = try await peekMenu(e)
            titles = leafTitles(menu)
            let w = capturePrimary()?.frame.width ?? 1200
            await capture("\(prefix)-menu", title: "\(area): \(id) menu", area: area, from: from, via: id, menu: menu,
                          anchor: CGPoint(x: min(r.minX, w - 260), y: r.maxY + 4))
        }
        let skip = UI.unsafeWords + extraSkip.map { $0.lowercased() }
        var n = 0
        for t in titles where n < limit {
            if skip.contains(where: { t.lowercased().contains($0) }) { continue }
            n += 1
            await step("\(id) > \(t)") {
                let before = screenText()
                try await menuButton(id: id, item: t)
                await idle(0.7)
                if hasOverlay || screenText() != before {
                    await capture("\(prefix)-\(slug(t))", title: "\(area): \(t)", area: area, from: "\(prefix)-menu", via: id)
                }
                await capRecover()
            }
        }
    }

    /// Presses each of these controls (when on screen) and captures what opens.
    func exploreButtons(_ ids: [(String, String)], area: String, prefix: String, from: String?) async {
        for (id, title) in ids {
            guard exists(id: id) else { continue }
            await step("button \(id)") {
                let before = screenText()
                try await act(id: id)
                await idle(0.7)
                if hasOverlay || screenText() != before {
                    await capture("\(prefix)-\(slug(title))", title: "\(area): \(title)", area: area, from: from, via: id)
                }
                await capRecover()
            }
        }
    }
}

extension Scenarios {
    /// The production base: Breakdown source chosen and suggestions accepted for scenes 1 and 2.
    @MainActor static func capBreakdownBase(_ ui: UI) async throws {
        try await ui.capNav("Breakdown")
        try await ui.waitText("Choose a screenplay source to begin the breakdown.")
        try await ui.act(id: "select-source")
        let deadline = Date().addingTimeInterval(15)
        while Date() < deadline, ui.queryPrefix("draft-").isEmpty { await ui.idle(0.2) }
        guard let d = ui.queryPrefix("draft-").first else { throw DriverError("no draft choice") }
        try await ui.press(d)
        try await ui.act(id: "set-production-source")
        try await ui.waitText("Production Source:")
        for n in 1...2 {
            try await capSelectScene(ui, n)
            try await ui.wait(label: "Scene \(n) breakdown", timeout: 15)
            try await ui.act(id: "suggest-elements")
            try await ui.waitText("Nothing is added to production until you accept.")
            var guardCount = 0
            while guardCount < 30, let b = capAccepts(ui).first {
                try await ui.press(b)
                guardCount += 1
                await ui.idle(0.4)
            }
        }
    }

    @MainActor static func capAccepts(_ ui: UI) -> [Element] {
        ui.all().filter { $0.identifier.hasPrefix("accept-") && $0.role == "AXButton" && $0.identifier != "accept-all-safe" && $0.identifier != "accept-selected" && $0.identifier != "accept-edited" }
    }

    @MainActor static func capSelectScene(_ ui: UI, _ n: Int) async throws {
        guard let outline = ui.query(id: "scene-list").first?.obj as? NSOutlineView else { throw DriverError("no scene list") }
        outline.window?.makeFirstResponder(outline)
        outline.selectRowIndexes(IndexSet(integer: n - 1), byExtendingSelection: false)
        await ui.idle(0.8)
        outline.window?.makeFirstResponder(nil)
    }

    /// The schedule: created from the production source, two days, scenes in day 1, a meal break.
    @MainActor static func capScheduleBase(_ ui: UI) async throws {
        try await capBreakdownBase(ui)
        try await ui.capNav("Schedule")
        try await ui.act(id: "create-shooting-schedule")
        try await ui.act(id: "create-schedule-confirm")
        try await ui.waitText("unscheduled")
        for iso in ["2027-06-14", "2027-06-15"] {
            var done = false
            for _ in 0..<3 where !done {
                do {
                    try await ui.act(id: "create-shooting-day")
                    _ = try await ui.wait(id: "day-date", timeout: 8)
                    await ui.idle(1.2)
                    try await ui.typeIn(iso, into: "day-date")
                    try await ui.act(id: "day-sheet-confirm")
                    try await ui.waitGone(id: "day-date")
                    done = true
                } catch { await ui.capRecover(); await ui.idle(0.8) }
            }
            if !done { throw DriverError("could not add the shooting day \(iso)") }
        }
        try await ui.menuButton(id: "strip-actions-1", item: "Shoot Day 1")
        await ui.idle(0.5)
        try await ui.menuButton(id: "strip-actions-2", item: "Shoot Day 1")
        await ui.idle(0.5)
    }
}

extension Scenarios {
    /// One tour scenario: its own app start, then the project Night Bus with the example screenplay.
    static func tour(_ name: String, order: Int, project: Bool = true, importSample: Bool = true, timeout: Double = 600,
                     _ body: @escaping @MainActor (UI, Context) async throws -> Void) -> Scenario {
        Scenario("zz-capture/\(name)", timeout: timeout) { ui, _, ctx in
            if project { try await ui.capSeed(ctx, order: order, importSample: importSample) } else {
                ui.captureBegin(order: order)
                try await ui.startApp(ctx)
                ui.capBigWindow()
            }
            try await body(ui, ctx)
            await ui.capRecover()
            await ui.idle(0.3)
        }
    }
}

// MARK: the tour

extension Scenarios {
    static let captureGroup: [Scenario] = captureStart + captureScreenplay + captureStory + captureVault + captureBreakdown + captureSchedule + captureCallSheets + captureBudget + captureVisual + captureProjectPages + captureAI + capturePackages + captureLifecycle

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
                try await ui.typeIn("zzzz nothing", into: "palette-field")
                await ui.capture("search-palette-empty", title: "Search and Commands, nothing found", area: "Search", from: "search-palette-home", via: "palette-field")
                await ui.capRecover()
                try? await ui.escape()
            }
            await ui.step("settings") {
                var last: String? = nil
                try await ui.menu([ui.appMenu, "Settings\u{2026}"])
                await ui.idle(1.0)
                for (key, label) in [("general", "General"), ("appearance", "Appearance"), ("offline-ai", "Offline AI"), ("diagnostics", "Diagnostics")] {
                    if label != "General" { try await ui.act(label: label) ; await ui.idle(0.8) }
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
                try await ui.act(id: "ai-toggle")
            }
            await ui.step("search palette project") {
                try await ui.shortcut("k", code: 40)
                _ = try await ui.wait(id: "palette-field", timeout: 6)
                await ui.capture("search-palette-project", title: "Search and Commands inside a project", area: "Search", from: "project-home", via: "shortcut-k")
                try await ui.typeIn("bus", into: "palette-field")
                await ui.idle(0.8)
                await ui.capture("search-palette-results", title: "Search results", area: "Search", from: "search-palette-project", via: "palette-field")
                await ui.capRecover()
                try? await ui.escape()
            }
            await ui.step("all projects") {
                try await ui.act(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-filled", title: "Application Home with a project", area: "Home", from: "project-home", via: "close-project")
                if let card = ui.query(id: "project-card-Night Bus").first { await ui.capContext(on: card, name: "home-card-menu", title: "Project card menu", area: "Home", from: "home-filled") }
                if ui.exists(id: "home-global-vault") { await ui.visit(press: "home-global-vault", name: "home-global-vault", title: "Global Idea Vault", area: "Home", from: "home-filled", close: nil) }
            }
            await ui.step("second project") {
                try await ui.createProject("Harbour Lights")
                try await ui.act(id: "close-project")
                try await ui.waitHome()
                await ui.capture("home-two-projects", title: "Application Home with two projects", area: "Home", from: "home-filled")
            }
        },
    ]

    static let captureStory: [Scenario] = [
        Scenario("zz-capture/03-story", timeout: 500) { ui, _, ctx in
            try await ui.capSeed(ctx, order: 3000)
            try await ui.capNav("Story")
            try await ui.wait(id: "story-tabs", timeout: 20)
            await ui.capture("story-board-empty", title: "Beat Board, nothing on it yet", area: "Story")
            await ui.capMenu(id: "story-add-menu", name: "story-add-menu", title: "Beat Board: Add menu", area: "Story", from: "story-board-empty")
            await ui.step("add act") {
                try await ui.act(label: "Add Act", role: "AXButton")
                await ui.capture("story-add-act-inline", title: "Adding an act on the board", area: "Story", from: "story-board-empty", via: "story-empty")
                try await ui.fill("Act title", "Act One", submit: true)
                await ui.idle(0.6)
            }
            await ui.step("add sequence") {
                try await ui.act(label: "Add Sequence", role: "AXButton")
                await ui.capture("story-add-sequence-sheet", title: "Add a sequence", area: "Story", from: "story-board-empty", via: "story-add-menu")
                try await ui.fill("Sequence name", "The last bus", submit: true)
                await ui.idle(0.5)
            }
            let cards = ["Meera misses the last bus", "The driver will not wait", "A lost bag on the back seat", "The ticket inspector boards", "Dawn at the depot"]
            for (n, c) in cards.enumerated() {
                await ui.step("add card \(n)") {
                    try await ui.menuButton(id: "story-add-menu", item: "Scene Card")
                    if n == 0 { await ui.capture("story-add-card-sheet", title: "Add a scene card", area: "Story", from: "story-board-empty", via: "story-add-menu") }
                    try await ui.fill("Short description", c, submit: true)
                    await ui.idle(0.5)
                }
            }
            await ui.capture("story-board-filled", title: "Beat Board with an act, a sequence and scene cards", area: "Story", from: "story-add-card-sheet")
            await ui.step("card drawer") {
                guard let c = ui.card(cards[0]) else { throw DriverError("no card") }
                try await ui.tap(c, count: 2)
                if (try? await ui.wait(id: "story-inspector-close", timeout: 3)) == nil, let again = ui.card(cards[0]) { try await ui.customAction(again, "Open") }
                try await ui.wait(id: "story-inspector-close", timeout: 6)
                await ui.capture("story-card-drawer", title: "Scene card drawer", area: "Story", from: "story-board-filled", via: "story-card")
                _ = await ui.tryPress(id: "story-inspector-close")
            }
            await ui.capMenu(id: "story-more", name: "story-more-menu", title: "Beat Board: More menu", area: "Story", from: "story-board-filled")
            await ui.step("card display") {
                try await ui.menuButton(id: "story-more", item: "Card Display Settings")
                await ui.capture("story-card-display-sheet", title: "Card Display Settings", area: "Story", from: "story-board-filled", via: "story-more")
                await ui.capRecover()
            }
            await ui.step("apply order") {
                try await ui.menuButton(id: "story-more", item: "Apply Board Order")
                await ui.capture("story-apply-order-sheet", title: "Apply Board Order to Screenplay", area: "Story", from: "story-board-filled", via: "story-more")
                await ui.capRecover()
            }
            await ui.visit(press: "story-build", name: "story-build-sheet", title: "Build Screenplay from the board", area: "Story", from: "story-board-filled", close: nil)
            await ui.visit(press: "story-parking-toggle", name: "story-parking-lot", title: "Beat Board with the Parking Lot open", area: "Story", from: "story-board-filled")
            _ = await ui.tryPress(id: "story-parking-toggle")
            await ui.step("outline") {
                try await ui.tab("Outline")
                await ui.capture("story-outline", title: "Story outline", area: "Story", from: "story-board-filled", via: "story-tabs")
            }
            await ui.step("timeline") {
                try await ui.tab("Timeline")
                await ui.capture("story-timeline", title: "Story timeline", area: "Story", from: "story-outline", via: "story-tabs")
            }
            await ui.step("characters") {
                try await ui.tab("Characters")
                await ui.capture("story-characters-empty", title: "Characters, none yet", area: "Story", from: "story-timeline", via: "story-tabs")
                for (name, role) in [("Meera", "A night shift nurse"), ("Dev", "The bus driver")] {
                    try await ui.act(label: "Add Character")
                    if name == "Meera" { await ui.capture("story-add-character-sheet", title: "Add a character", area: "Story", from: "story-characters-empty", via: "story-add-character") }
                    try await ui.fill(nil, name, id: "story-new-character-name")
                    try await ui.fill(nil, role, id: "story-new-character-role")
                    try await ui.act(label: "Create Character")
                    await ui.idle(0.6)
                }
                await ui.capture("story-characters", title: "Characters", area: "Story", from: "story-add-character-sheet")
                await ui.visit(label: "Edit", name: "story-character-edit", title: "Edit a character", area: "Story", from: "story-characters", closeLabel: "Save")
                await ui.capMenu(id: "story-character-more", name: "story-character-menu", title: "Character: More menu", area: "Story", from: "story-characters")
            }
            await ui.step("episodes") {
                try await ui.tab("Board")
                if ui.exists(id: "story-add-episode") { await ui.visit(press: "story-add-episode", name: "story-add-episode", title: "Add an episode", area: "Story", from: "story-board-filled") }
            }
        },
    ]
}

extension Scenarios {
    static let captureScreenplay: [Scenario] = [
        tour("02-screenplay", order: 2000) { ui, _ in
            await ui.capture("screenplay-editor", title: "Screenplay editor with the example script", area: "Screenplay")
            await ui.step("type") {
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("\n\nEXT. DEPOT - DAWN\n")
                await ui.idle(0.6)
                await ui.capture("screenplay-editor-typing", title: "Screenplay editor while typing", area: "Screenplay", from: "screenplay-editor", via: "screenplay-editor")
            }
            await ui.exploreMenu("screenplay-tools-menu", area: "Screenplay", prefix: "screenplay-tools", from: "screenplay-editor")
            await ui.exploreMenu("screenplay-interchange-menu", area: "Screenplay", prefix: "screenplay-interchange", from: "screenplay-editor")
            await ui.exploreMenu("layout-menu", area: "Screenplay", prefix: "screenplay-layout", from: "screenplay-editor")
            await ui.exploreMenu("draft-menu", area: "Screenplay", prefix: "screenplay-draft", from: "screenplay-editor")
            await ui.capMenu(id: "element-picker", name: "screenplay-element-picker", title: "Screenplay: element type menu", area: "Screenplay", from: "screenplay-editor")
            await ui.exploreButtons([("comments-button", "Comments"), ("compare-button", "Compare drafts"), ("replace-toggle", "Find and replace"),
                                     ("screenplay-print", "Print preview"), ("screenplay-export", "Export screenplay"), ("screenplay-import", "Import screenplay"),
                                     ("screenplay-title", "Title page"), ("scene-navigator", "Scene navigator")], area: "Screenplay", prefix: "screenplay", from: "screenplay-editor")
        },
    ]

    static let captureVault: [Scenario] = [
        tour("04-vault", order: 4000, importSample: false) { ui, _ in
            try await ui.capNav("Idea Vault")
            try await ui.wait(id: "vault-items", timeout: 15)
            await ui.capture("vault-empty", title: "Idea Vault, empty", area: "Idea Vault")
            await ui.exploreMenu("vault-add", area: "Idea Vault", prefix: "vault-add", from: "vault-empty")
            for (n, text) in ["A bus that never reaches its stop", "Rain on the depot roof, a single lamp", "What if the driver is the ticket inspector?"].enumerated() {
                await ui.step("quick capture \(n)") {
                    try await ui.act(id: "vault-quick-capture")
                    _ = try await ui.wait(id: "quick-capture-idea", timeout: 6)
                    if n == 0 { await ui.capture("vault-quick-capture-sheet", title: "Quick capture an idea", area: "Idea Vault", from: "vault-empty", via: "vault-quick-capture") }
                    try await ui.fill(nil, text, id: "quick-capture-idea")
                    try await ui.act(label: "Save", role: "AXButton")
                    await ui.idle(0.8)
                }
            }
            await ui.capture("vault-filled", title: "Idea Vault with ideas", area: "Idea Vault", from: "vault-quick-capture-sheet")
            await ui.eachTab(group: "vault-view-mode", area: "Idea Vault", prefix: "vault-view", from: "vault-filled")
            await ui.step("open card") {
                guard let c = ui.queryPrefix("vault-card-").first(where: { $0.frame.width > 0 }) else { throw DriverError("no vault card") }
                try await ui.tap(c, count: 2)
                _ = try await ui.wait(id: "vault-drawer", timeout: 6)
                await ui.capture("vault-item-drawer", title: "Idea Vault item drawer", area: "Idea Vault", from: "vault-filled", via: "vault-card")
                await ui.exploreButtons([("drawer-send-to-story", "Send to Story"), ("drawer-add-tag", "Add tag"), ("drawer-research-flag", "Research flag")], area: "Idea Vault", prefix: "vault-drawer", from: "vault-item-drawer")
                _ = await ui.tryPress(id: "drawer-close")
            }
            await ui.step("folder") {
                guard let f = ui.queryPrefix("vault-folder-").first(where: { $0.frame.width > 0 }) else { throw DriverError("no folder") }
                try await ui.tap(f, count: 2)
                await ui.idle(0.6)
                await ui.capture("vault-folder", title: "Inside a folder", area: "Idea Vault", from: "vault-filled", via: "vault-folder")
                _ = await ui.tryPress(id: "vault-back")
            }
            await ui.eachTab(group: "vault-scope", area: "Idea Vault", prefix: "vault-scope", from: "vault-filled")
            try? await ui.tab("This Project")
            await ui.step("new folder") {
                try await ui.act(id: "vault-new-folder")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                await ui.capture("vault-new-folder-sheet", title: "New folder", area: "Idea Vault", from: "vault-empty", via: "vault-new-folder")
                try await ui.fill(nil, "Night scenes", id: "name-sheet-field", submit: true)
                await ui.idle(0.6)
            }
            await ui.step("new collection") {
                try await ui.act(id: "vault-new-collection")
                _ = try await ui.wait(id: "name-sheet-field", timeout: 6)
                await ui.capture("vault-new-collection-sheet", title: "New collection", area: "Idea Vault", from: "vault-empty", via: "vault-new-collection")
                try await ui.fill(nil, "Look and feel", id: "name-sheet-field", submit: true)
                await ui.idle(0.6)
            }
            await ui.capture("vault-with-folder-and-collection", title: "Idea Vault with a folder and a collection", area: "Idea Vault", from: "vault-new-collection-sheet", via: "name-sheet-field")
            await ui.step("recently deleted") {
                try await ui.act(id: "vault-recently-deleted")
                await ui.idle(0.8)
                await ui.capture("vault-recently-deleted", title: "Idea Vault: Recently Deleted", area: "Idea Vault", from: "vault-filled", via: "vault-recently-deleted")
            }
        },
    ]

    static let captureBreakdown: [Scenario] = [
        tour("05-breakdown", order: 5000) { ui, _ in
            try await ui.capNav("Breakdown")
            await ui.capture("breakdown-no-source", title: "Breakdown, choose a screenplay source", area: "Breakdown")
            await ui.step("source") {
                try await ui.act(id: "select-source")
                let deadline = Date().addingTimeInterval(15)
                while Date() < deadline, ui.queryPrefix("draft-").isEmpty { await ui.idle(0.2) }
                await ui.capture("breakdown-source-sheet", title: "Choose the production source", area: "Breakdown", from: "breakdown-no-source", via: "select-source")
                guard let d = ui.queryPrefix("draft-").first else { throw DriverError("no draft") }
                try await ui.press(d)
                try await ui.act(id: "set-production-source")
                try await ui.waitText("Production Source:")
                await ui.idle(0.6)
            }
            await ui.capture("breakdown-scenes", title: "Breakdown: scene list and scene panel", area: "Breakdown", from: "breakdown-source-sheet", via: "set-production-source")
            await ui.step("suggest") {
                try await Scenarios.capSelectScene(ui, 1)
                try await ui.wait(label: "Scene 1 breakdown", timeout: 15)
                await ui.capture("breakdown-scene-1", title: "Breakdown: scene 1 panel", area: "Breakdown", from: "breakdown-scenes", via: "scene-list")
                try await ui.act(id: "suggest-elements")
                try await ui.waitText("Nothing is added to production until you accept.")
                await ui.capture("breakdown-suggestions", title: "Breakdown: suggested elements waiting for review", area: "Breakdown", from: "breakdown-scene-1", via: "suggest-elements")
                if let b = Scenarios.capAccepts(ui).first { try await ui.press(b); await ui.idle(0.6) }
                await ui.capture("breakdown-accepted-one", title: "Breakdown: one suggestion accepted", area: "Breakdown", from: "breakdown-suggestions", via: "accept")
                var n = 0
                while n < 30, let b = Scenarios.capAccepts(ui).first { try await ui.press(b); n += 1; await ui.idle(0.4) }
                await ui.capture("breakdown-all-accepted", title: "Breakdown: all suggestions accepted", area: "Breakdown", from: "breakdown-accepted-one")
            }
            await ui.step("add element") {
                try await ui.act(id: "add-element")
                _ = try await ui.wait(id: "add-name", timeout: 6)
                await ui.capture("breakdown-add-element-sheet", title: "Add an element to the scene", area: "Breakdown", from: "breakdown-all-accepted", via: "add-element")
                await ui.capMenu(id: "add-category", name: "breakdown-add-category-menu", title: "Add element: category", area: "Breakdown", from: "breakdown-add-element-sheet")
                try await ui.typeIn("Flickering monitor", into: "add-name")
                await ui.idle(1.2)
                await ui.capture("breakdown-add-element-filled", title: "Add an element, name typed", area: "Breakdown", from: "breakdown-add-element-sheet", via: "add-name")
                await ui.capRecover()
            }
            await ui.step("second scene") {
                try await Scenarios.capSelectScene(ui, 2)
                await ui.idle(0.8)
                await ui.capture("breakdown-scene-2", title: "Breakdown: scene 2 panel", area: "Breakdown", from: "breakdown-scenes", via: "scene-list")
            }
            await ui.exploreButtons([("mark-complete", "Mark scene complete"), ("scene-filter", "Scene filter"), ("script-tag-menu", "Tag selection"), ("quick-reuse", "Quick reuse"),
                                     ("review-suggestions", "Review suggestions"), ("change-source", "Change source"), ("open-needs-review", "Needs review"), ("removed-scenes", "Removed scenes")],
                                    area: "Breakdown", prefix: "breakdown", from: "breakdown-scenes")
            await ui.eachTab(group: "breakdown-tabs", area: "Breakdown", prefix: "breakdown-tab", from: "breakdown-scenes")
            await ui.step("catalog") {
                if ui.exists(id: "catalog-tab") { try await ui.act(id: "catalog-tab"); await ui.idle(0.6); await ui.capture("breakdown-catalog", title: "Breakdown: catalog of elements", area: "Breakdown", from: "breakdown-scenes", via: "catalog-tab") }
                if ui.exists(id: "catalog-show-archived") { try await ui.act(id: "catalog-show-archived"); await ui.idle(0.4); await ui.capture("breakdown-catalog-archived", title: "Catalog with archived items", area: "Breakdown", from: "breakdown-catalog", via: "catalog-show-archived"); try? await ui.act(id: "catalog-show-archived") }
            }
            await ui.step("cast crew") {
                if ui.exists(id: "cast-crew-tab") { try await ui.act(id: "cast-crew-tab"); await ui.idle(0.6); await ui.capture("breakdown-cast-crew", title: "Breakdown: cast and crew", area: "Breakdown", from: "breakdown-scenes", via: "cast-crew-tab")
                    await ui.exploreButtons([("add-person", "Add a person"), ("people-view", "People view")], area: "Breakdown", prefix: "breakdown-people", from: "breakdown-cast-crew") }
            }
            await ui.step("locations") {
                if ui.exists(id: "locations-tab") { try await ui.act(id: "locations-tab"); await ui.idle(0.6); await ui.capture("breakdown-locations", title: "Breakdown: locations", area: "Breakdown", from: "breakdown-scenes", via: "locations-tab")
                    await ui.exploreButtons([("add-location", "Add a location")], area: "Breakdown", prefix: "breakdown-location", from: "breakdown-locations") }
            }
            await ui.step("categories") {
                if ui.exists(id: "categories-tab") { try await ui.act(id: "categories-tab"); await ui.idle(0.6); await ui.capture("breakdown-categories", title: "Breakdown: category styles", area: "Breakdown", from: "breakdown-scenes", via: "categories-tab") }
            }
        },
    ]

    static let captureSchedule: [Scenario] = [
        tour("06-schedule", order: 6000, timeout: 900) { ui, _ in
            try await Scenarios.capBreakdownBase(ui)
            try await ui.capNav("Schedule")
            await ui.capture("schedule-empty", title: "Schedule, not created yet", area: "Schedule")
            await ui.step("create schedule") {
                try await ui.act(id: "create-shooting-schedule")
                await ui.idle(0.6)
                await ui.capture("schedule-create-sheet", title: "Create the shooting schedule", area: "Schedule", from: "schedule-empty", via: "create-shooting-schedule")
                try await ui.act(id: "create-schedule-confirm")
                try await ui.waitText("unscheduled")
                await ui.idle(0.8)
                await ui.capture("schedule-board-unscheduled", title: "Schedule board, scenes waiting", area: "Schedule", from: "schedule-create-sheet", via: "create-schedule-confirm")
            }
            await ui.step("days") {
                try await ui.act(id: "create-shooting-day")
                _ = try await ui.wait(id: "day-date", timeout: 6)
                await ui.capture("schedule-add-day-sheet", title: "Add a shooting day", area: "Schedule", from: "schedule-board-unscheduled", via: "create-shooting-day")
                for iso in ["2027-06-14", "2027-06-15"] {
                    if !ui.exists(id: "day-date") { try await ui.act(id: "create-shooting-day") }
                    try await ui.typeIn(iso, into: "day-date")
                    try await ui.act(id: "day-sheet-confirm")
                    try await ui.waitGone(id: "day-date")
                }
                try await ui.act(id: "add-off-day")
                await ui.capture("schedule-add-off-day-sheet", title: "Add a day off", area: "Schedule", from: "schedule-board-unscheduled", via: "add-off-day")
                try await ui.act(id: "day-sheet-confirm")
                await ui.idle(0.6)
            }
            await ui.step("scenes into day") {
                await ui.capMenu(id: "strip-actions-1", name: "schedule-strip-menu", title: "Schedule: scene strip menu", area: "Schedule", from: "schedule-board-unscheduled")
                try await ui.menuButton(id: "strip-actions-1", item: "Shoot Day 1")
                await ui.idle(0.5)
                try await ui.menuButton(id: "strip-actions-2", item: "Shoot Day 1")
                await ui.idle(0.8)
                await ui.capture("schedule-board", title: "Schedule board with scenes in Day 1", area: "Schedule", from: "schedule-board-unscheduled")
            }
            await ui.step("meal break") {
                await ui.capMenu(id: "day-actions-1", name: "schedule-day-menu", title: "Schedule: day column menu", area: "Schedule", from: "schedule-board")
                try await ui.menuButton(id: "day-actions-1", item: "Meal Break")
                _ = try await ui.wait(id: "marker-duration", timeout: 6)
                await ui.capture("schedule-meal-break-sheet", title: "Add a meal break", area: "Schedule", from: "schedule-day-menu", via: "day-actions-1")
                try await ui.typeIn("45m", into: "marker-duration")
                try await ui.act(id: "marker-save")
                await ui.idle(0.8)
            }
            await ui.step("day detail") {
                try await ui.act(id: "day-open-1")
                _ = try await ui.wait(id: "day-detail", timeout: 10)
                await ui.capture("schedule-day-detail", title: "Shooting Day 1 detail and timeline", area: "Schedule", from: "schedule-board", via: "day-open-1")
                try await ui.typeIn("07:00", into: "day-crew-call")
                try await ui.typeIn("08:00", into: "day-shooting-start")
                try await ui.typeIn("19:00", into: "day-target-wrap")
                await ui.idle(0.6)
                await ui.capture("schedule-day-detail-times", title: "Shooting Day 1 with times set", area: "Schedule", from: "schedule-day-detail")
                await ui.exploreButtons([("day-add-meal", "Add meal"), ("day-add-travel", "Add travel"), ("day-add-company-move", "Add company move"), ("day-add-custom", "Add custom"), ("day-export", "Export day")], area: "Schedule", prefix: "schedule-day", from: "schedule-day-detail")
                try await ui.act(id: "day-back")
                await ui.idle(0.6)
            }
            for (m, label) in [("schedule-mode-list", "list"), ("schedule-mode-calendar", "calendar"), ("schedule-mode-board", "board")] {
                await ui.step("mode \(label)") {
                    try await ui.act(id: m)
                    await ui.idle(0.8)
                    await ui.capture("schedule-\(label)", title: "Schedule as a \(label)", area: "Schedule", from: "schedule-board", via: m)
                }
            }
            await ui.exploreButtons([("schedule-filters", "Filters"), ("suggest-grouping", "Suggest grouping"), ("warnings-button", "Warnings"), ("schedule-settings", "Schedule settings"),
                                     ("schedule-export", "Export schedule"), ("schedule-pool", "Unscheduled pool")], area: "Schedule", prefix: "schedule", from: "schedule-board")
            await ui.step("dood") {
                try await ui.go("Day Out of Days")
                _ = try await ui.wait(id: "dood-table", timeout: 10)
                await ui.capture("schedule-dood", title: "Day Out of Days", area: "Schedule", from: "schedule-board", via: "schedule-section-dood")
                await ui.exploreButtons([("dood-export", "Export Day Out of Days"), ("dood-note", "Day Out of Days note")], area: "Schedule", prefix: "schedule-dood", from: "schedule-dood")
                try await ui.go("Schedule")
            }
        },
    ]

    static let captureCallSheets: [Scenario] = [
        tour("07-callsheets", order: 7000, timeout: 1100) { ui, _ in
            try await Scenarios.capScheduleBase(ui)
            try await ui.capNav("Call Sheets")
            await ui.capture("callsheets-empty", title: "Call Sheets, none yet", area: "Call Sheets")
            await ui.step("create") {
                try await ui.act(id: "callsheet-create")
                await ui.idle(0.8)
                await ui.capture("callsheet-create-sheet", title: "Create a call sheet", area: "Call Sheets", from: "callsheets-empty", via: "callsheet-create")
                await ui.capMenu(id: "callsheet-create-day", name: "callsheet-create-day-menu", title: "Create a call sheet: pick a day", area: "Call Sheets", from: "callsheet-create-sheet")
                await ui.capMenu(id: "callsheet-create-template", name: "callsheet-create-template-menu", title: "Create a call sheet: template", area: "Call Sheets", from: "callsheet-create-sheet")
                try await ui.act(id: "callsheet-create-confirm")
                _ = try await ui.wait(id: "callsheet-document", timeout: 25)
                await ui.idle(1.0)
                await ui.capture("callsheet-editor", title: "Call sheet editor", area: "Call Sheets", from: "callsheet-create-sheet", via: "callsheet-create-confirm")
            }
            await ui.exploreMenu("callsheet-add-section", area: "Call Sheets", prefix: "callsheet-section", from: "callsheet-editor", limit: 4)
            await ui.exploreButtons([("callsheet-add-crew-row", "Add crew row"), ("callsheet-add-extras-row", "Add extras row"), ("callsheet-add-custom-field", "Add custom field"), ("callsheet-finalize", "Finalize"), ("callsheet-export", "Export"), ("callsheet-refresh", "Refresh from schedule")],
                                    area: "Call Sheets", prefix: "callsheet", from: "callsheet-editor")
            await ui.step("back to list") {
                try await ui.act(id: "callsheet-back")
                await ui.idle(0.8)
                await ui.capture("callsheets-list", title: "Call Sheets list", area: "Call Sheets", from: "callsheet-editor", via: "callsheet-back")
            }
            await ui.eachTab(group: "callsheets-tabs", area: "Call Sheets", prefix: "callsheets-tab", from: "callsheets-list")
        },
    ]

    static let captureBudget: [Scenario] = [
        tour("08-budget", order: 8000, importSample: false) { ui, _ in
            try await ui.capNav("Budget")
            await ui.capture("budget-start", title: "Budget, not started", area: "Budget")
            await ui.capMenu(id: "budget-start-currency", name: "budget-currency-menu", title: "Budget: choose a currency", area: "Budget", from: "budget-start")
            await ui.step("start") {
                try await ui.act(label: "Start Budget", role: "AXButton")
                await ui.idle(1.2)
                await ui.capture("budget-empty-table", title: "Budget, no lines yet", area: "Budget", from: "budget-start", via: "Start Budget")
            }
            for (n, (cat, desc, amt)) in [("Cast", "Lead actors", "40000"), ("Crew", "Camera team", "25000"), ("Locations", "Depot hire", "12000")].enumerated() {
                await ui.step("line \(n)") {
                    try await ui.act(id: "budget-add-line")
                    _ = try await ui.wait(id: "of-bl-desc", timeout: 6)
                    if n == 0 { await ui.capture("budget-add-line-sheet", title: "Add a budget line", area: "Budget", from: "budget-empty-table", via: "budget-add-line") }
                    try await ui.typeIn(desc, into: "of-bl-desc")
                    try await ui.typeIn(amt, into: "of-bl-amt")
                    if n == 0 { await ui.capture("budget-add-line-filled", title: "Add a budget line, filled in", area: "Budget", from: "budget-add-line-sheet", via: "of-bl-desc") }
                    try await ui.act(id: "budget-line-sheet-primary")
                    await ui.idle(0.8)
                    _ = cat
                }
            }
            await ui.capture("budget-table", title: "Budget with lines", area: "Budget", from: "budget-add-line-sheet")
            await ui.exploreButtons([("budget-edit-plan", "Edit plan"), ("budget-save-snapshot", "Save a snapshot"), ("budget-snapshots", "Snapshots"), ("budget-notes", "Notes"), ("budget-mark-reviewed", "Mark reviewed"), ("budget-export", "Export budget")],
                                    area: "Budget", prefix: "budget", from: "budget-table")
        },
    ]

    static let captureVisual: [Scenario] = [
        tour("09-visual", order: 9000) { ui, _ in
            try await ui.capNav("Visual Planning")
            await ui.capture("visual-moodboards", title: "Visual Planning: Moodboards", area: "Visual Planning")
            await ui.step("new moodboard") {
                try await ui.act(id: "moodboard-new")
                _ = try await ui.wait(id: "mb-name", timeout: 6)
                await ui.capture("moodboard-new-sheet", title: "New moodboard", area: "Visual Planning", from: "visual-moodboards", via: "moodboard-new")
                try await ui.typeIn("Night Bus look", into: "mb-name")
                try await ui.act(id: "moodboard-new-sheet-primary")
                await ui.idle(1.0)
                await ui.capture("moodboard-canvas", title: "Moodboard canvas", area: "Visual Planning", from: "moodboard-new-sheet")
            }
            await ui.exploreMenu("moodboard-add-image", area: "Visual Planning", prefix: "moodboard-add-image", from: "moodboard-canvas", limit: 3)
            await ui.step("moodboard note") {
                try await ui.act(id: "moodboard-add-note")
                await ui.idle(0.8)
                await ui.capture("moodboard-add-note", title: "Moodboard: add a note", area: "Visual Planning", from: "moodboard-canvas", via: "moodboard-add-note")
                try? await ui.fill(nil, "Sodium lamps, wet tarmac", id: "new-note")
                await ui.capRecover()
            }
            await ui.exploreButtons([("moodboard-add-link", "Add a link"), ("moodboard-export", "Export moodboard"), ("moodboard-back", "Back to moodboards")], area: "Visual Planning", prefix: "moodboard", from: "moodboard-canvas")
            await ui.eachTab(group: "visual-tabs", area: "Visual Planning", prefix: "visual-tab", from: "visual-moodboards")
            await ui.step("storyboard") {
                try await ui.tab("Storyboards")
                await ui.exploreButtons([("storyboard-new", "New storyboard"), ("storyboard-add-panel", "Add a panel"), ("storyboard-add-empty", "Add an empty panel"), ("storyboard-empty-new", "New storyboard")], area: "Visual Planning", prefix: "storyboard", from: "visual-tab-storyboards")
            }
            await ui.step("shots") {
                try await ui.tab("Shot Lists")
                await ui.exploreButtons([("shots-add", "Add a shot"), ("shots-more", "More"), ("shots-export", "Export shot list")], area: "Visual Planning", prefix: "shots", from: "visual-tab-shot-lists")
            }
        },
    ]

    static let captureProjectPages: [Scenario] = [
        tour("10-files-notes", order: 10000, importSample: false) { ui, _ in
            try await ui.capNav("Files")
            await ui.capture("files-empty", title: "Files, empty", area: "Files")
            await ui.exploreMenu("add-file", area: "Files", prefix: "files-add", from: "files-empty", extraSkip: ["file", "copy"])
            await ui.step("folder") {
                try await ui.act(id: "new-folder")
                _ = try await ui.wait(id: "folder-name", timeout: 6)
                await ui.capture("files-new-folder-sheet", title: "New folder", area: "Files", from: "files-empty", via: "new-folder")
                try await ui.typeIn("References", into: "folder-name")
                try await ui.act(id: "folder-confirm")
                await ui.idle(0.8)
                await ui.capture("files-with-folder", title: "Files with a folder", area: "Files", from: "files-new-folder-sheet")
            }
            await ui.exploreMenu("files-view", area: "Files", prefix: "files-view", from: "files-empty", limit: 3)
            try await ui.capNav("Notes and Tasks")
            await ui.capture("notes-empty", title: "Notes and Tasks, empty", area: "Notes and Tasks")
            await ui.step("note") {
                try await ui.act(id: "new-note")
                _ = try await ui.wait(id: "note-title", timeout: 8)
                await ui.capture("notes-new-note", title: "New note", area: "Notes and Tasks", from: "notes-empty", via: "new-note")
                try await ui.fill(nil, "Depot permit call", id: "note-title")
                try await ui.fill(nil, "Ask the depot manager about the lamp rig before Friday.", id: "note-body")
                await ui.idle(1.2)
                await ui.capture("notes-note-written", title: "A note written", area: "Notes and Tasks", from: "notes-new-note", via: "note-body")
                await ui.exploreButtons([("note-pin", "Pin the note")], area: "Notes and Tasks", prefix: "notes", from: "notes-note-written")
                _ = await ui.tryPress(id: "note-close")
            }
            await ui.step("task") {
                try await ui.act(id: "new-task")
                _ = try await ui.wait(id: "task-title", timeout: 8)
                await ui.capture("tasks-new-task", title: "New task", area: "Notes and Tasks", from: "notes-empty", via: "new-task")
                try await ui.fill(nil, "Book the lamp rig", id: "task-title")
                await ui.exploreButtons([("task-has-due", "Due date"), ("task-related-open", "Link to something")], area: "Notes and Tasks", prefix: "tasks", from: "tasks-new-task")
                try? await ui.act(id: "task-save")
                await ui.idle(0.8)
                await ui.capture("notes-and-tasks", title: "Notes and Tasks with a note and a task", area: "Notes and Tasks", from: "tasks-new-task", via: "task-save")
            }
            await ui.exploreMenu("tasks-filter", area: "Notes and Tasks", prefix: "tasks-filter", from: "notes-and-tasks", limit: 4)
            try await ui.capNav("Activity")
            await ui.capture("activity", title: "Activity", area: "Activity")
            await ui.exploreMenu("activity-filter", area: "Activity", prefix: "activity-filter", from: "activity", limit: 5)
            try await ui.capNav("Recently Deleted")
            await ui.capture("recently-deleted-empty", title: "Recently Deleted, empty", area: "Recently Deleted")
        },
    ]

    static let captureAI: [Scenario] = [
        tour("13-ai", order: 13000) { ui, _ in
            try await ui.capNav("Project Home")
            await ui.step("ai panel") {
                try await ui.shortcut("j", code: 38)
                await ui.idle(1.5)
                if !ui.exists(id: "ai-panel") && !ui.exists(id: "ai-input") { try await ui.act(id: "ai-toggle"); await ui.idle(1.5) }
                await ui.capture("ai-panel", title: "AI Assistant panel", area: "AI Assistant", from: "project-home", via: "ai-toggle")
                await ui.capMenu(id: "ai-settings-menu", name: "ai-settings-menu", title: "AI Assistant: settings menu", area: "AI Assistant", from: "ai-panel")
                await ui.exploreButtons([("ai-new-conversation", "New conversation"), ("ai-mode-chip", "Mode"), ("ai-open-settings", "Open settings")], area: "AI Assistant", prefix: "ai", from: "ai-panel")
                try await ui.typeIn("What is on the Beat Board?", into: "ai-input")
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
}

extension Scenarios {
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

    static let captureLifecycle: [Scenario] = [
        Scenario("zz-capture/12-recovery", phases: 2, timeout: 300, end: { $0 == 1 ? .hold : .quit }) { ui, phase, ctx in
            ui.captureBegin(order: 12000 + (phase - 1) * 100)
            try await ui.startApp(ctx)
            ui.capBigWindow()
            if phase == 1 { await ui.capSetMode() }
            if phase == 1 {
                try await ui.createProject("Night Bus")
                try await ui.capNav("Screenplay")
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("INT. DEPOT - NIGHT")
                try await ui.waitSaved()
            } else {
                try await ui.waitHome()
                await ui.capture("home-after-crash", title: "Application Home after a forced quit", area: "Recovery")
                try await ui.press(id: "project-card-Night Bus")
                _ = try await ui.wait(id: "project-home", timeout: 30)
                let offered = await ui.recoveryOffered(within: 10)
                await ui.capture("recovery-alert", title: "Recovery question after the app closed unexpectedly", area: "Recovery", from: "home-after-crash", via: "project-card-Night Bus")
                if offered { try? await ui.press(label: "Keep the latest work", role: "AXButton") }
                await ui.capRecover()
            }
        },
        Scenario("zz-capture/12-unavailable", phases: 2, timeout: 300, end: { _ in .quit }) { ui, phase, ctx in
            ui.captureBegin(order: 12500 + (phase - 1) * 100)
            try await ui.startApp(ctx)
            ui.capBigWindow()
            if phase == 1 { await ui.capSetMode() }
            if phase == 1 {
                try await ui.createProject("Night Bus")
                try await ui.closeProject()
                // The project folder is moved away while the app is closed, so the card reports it missing.
                let root = ctx.dataRoot.appendingPathComponent("projects")
                let away = ctx.dataRoot.appendingPathComponent("moved-away")
                try? FileManager.default.removeItem(at: away)
                try FileManager.default.moveItem(at: root, to: away)
            } else {
                try await ui.waitHome()
                await ui.capture("home-project-unavailable", title: "Application Home, a project that cannot be found", area: "Recovery")
                if let card = ui.query(id: "project-card-Night Bus").first { try? await ui.press(card) }
                await ui.idle(1.0)
                await ui.capture("project-unavailable", title: "Project unavailable", area: "Recovery", from: "home-project-unavailable", via: "project-card-Night Bus")
                await ui.capRecover()
            }
        },
    ]
}
