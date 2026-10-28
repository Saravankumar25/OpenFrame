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

extension UI {
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
                var item: [String: Any] = [
                    "role": role, "subrole": e.subrole, "id": e.identifier, "label": e.label, "title": e.title, "value": value,
                    "enabled": e.isEnabled, "selected": e.isSelected, "focused": e.isFocused, "window": kind, "depth": depth,
                    "x": Double(rel.minX), "y": Double(rel.minY), "w": Double(rel.width), "h": Double(rel.height),
                ]
                // A pop-up button's choices, so the page can show it as a select with the chosen value.
                if let pop = e.obj as? NSPopUpButton {
                    item["options"] = pop.itemTitles
                    item["value"] = pop.titleOfSelectedItem ?? value
                }
                list.append(item)
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
        // A window that drifted a few points from the tour size (the app sizes itself to its content now and
        // then) is put back; a size chosen on purpose for a narrow-window state is left alone.
        if let w = capturePrimary(), w.sheets.isEmpty,
           abs(w.frame.width - UI.captureSize.width) < 16, abs(w.frame.height - UI.captureSize.height) < 16,
           w.frame.size != UI.captureSize {
            capBigWindow()
            await idle(0.4)
        }
        guard CaptureState.names.insert(name).inserted else { log("capture \(name): already taken"); return }
        // An app in the background draws its windows dimmed. Bring it forward without changing the key window,
        // which would close an open popover.
        if !NSApp.isActive { NSApp.activate(ignoringOtherApps: true); await idle(0.4) }
        guard let primary = capturePrimary() else { log("capture \(name): no window"); return }
        let overlays = captureOverlays(primary)
        // Runners share one screen: a window covered by another app's window stops drawing its content and the
        // window server then hands back a blank picture. Bring the window forward and take it again.
        var picture = captureRender(primary)
        for attempt in 0..<4 {
            guard let rep = picture, captureLooksBlank(rep) else { break }
            log("capture \(name): blank picture, taking it again (\(attempt + 1))")
            NSApp.activate(ignoringOtherApps: true)
            primary.orderFrontRegardless()
            for o in overlays { o.orderFrontRegardless() }
            primary.contentView?.needsDisplay = true
            primary.displayIfNeeded()
            await idle(0.8)
            picture = captureRender(primary)
        }
        guard let base = picture else { log("capture \(name): could not draw"); return }
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

    /// True when the window's content below the toolbar is a single flat colour: the picture of a window whose
    /// content was not drawn. Every real screen has some text there, so it has several colours.
    func captureLooksBlank(_ rep: NSBitmapImageRep) -> Bool {
        let w = rep.pixelsWide, h = rep.pixelsHigh
        guard w > 100, h > 200 else { return false }
        var seen = Set<UInt32>()
        let top = Int(Double(h) * 0.14)
        for y in stride(from: top, to: h - 8, by: 6) {
            for x in stride(from: 4, to: w - 4, by: 6) {
                guard let c = rep.colorAt(x: x, y: y) else { continue }
                let key = (UInt32(c.redComponent * 31) << 10) | (UInt32(c.greenComponent * 31) << 5) | UInt32(c.blueComponent * 31)
                seen.insert(key)
                if seen.count > 3 { return false }
            }
        }
        return true
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

// MARK: helpers (shared by the section files)

extension UI {
    /// Sets Settings, Appearance, Mode (default theme) to Light or Dark, as the pass asks (OPENFRAME_CAPTURE_MODE).
    func capSetMode() async {
        let want = (ProcessInfo.processInfo.environment["OPENFRAME_CAPTURE_MODE"] ?? "dark").lowercased() == "light" ? "Light" : "Dark"
        await step("set mode \(want)") {
            try await menu([appMenu, "Settings\u{2026}"])
            await idle(1.0)
            try await capAct(label: "Appearance")
            await idle(0.8)
            try await capAct(label: want, role: "AXRadioButton")
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

    func capShows(_ text: String) -> Bool { all().contains { $0.text.contains(text) } }

    func capQueryPrefix(_ prefix: String) -> [Element] { all().filter { $0.identifier.hasPrefix(prefix) } }

    func capWaitUntil(_ what: String, timeout: Double = 15, _ condition: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return }
            await idle(0.1)
        }
        throw DriverError("not true within \(Int(timeout)) s: \(what)")
    }

    /// The one window size of the tour (points): the whole visible part of the screen (below the menu bar, beside
    /// the Dock), as when the owner zooms the window to fill the screen. Every page has this size.
    static var captureSize: CGSize { NSScreen.main?.visibleFrame.size ?? CGSize(width: 1280, height: 628) }

    func capBigWindow(width: CGFloat? = nil, height: CGFloat? = nil) {
        let width = width ?? UI.captureSize.width, height = height ?? UI.captureSize.height
        guard let w = NSApp.windows.filter({ $0.isVisible && $0.canBecomeMain && $0.sheetParent == nil && $0.frame.width > 900 }).max(by: { $0.frame.width < $1.frame.width }) else { return }
        let screen = w.screen ?? NSScreen.main
        let vf = screen?.visibleFrame ?? NSRect(x: 0, y: 0, width: 1728, height: 1080)
        // Larger than the screen is allowed: the window keeps its size, the top left stays on screen.
        let target = NSRect(x: vf.minX, y: vf.maxY - height, width: width, height: height)
        if w.frame != target { w.setFrame(target, display: true) }
    }

    func capPostClick(atScreen center: NSPoint) {
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

    func capMenuButton(id: String? = nil, label: String? = nil, item: String, timeout: Double = 15) async throws {
        let e = try await wait(id: id, label: label, timeout: timeout, "the menu button \(id ?? label ?? "?")")
        guard e.obj is NSPopUpButtonCell else { throw DriverError("not a menu button: \(e)") }
        func find(_ m: NSMenu) -> (NSMenu, Int)? {
            for (i, it) in m.items.enumerated() where !it.isSeparatorItem && it.title.hasPrefix(item) { return (m, i) }
            for it in m.items { if let sub = it.submenu, let r = find(sub) { return r } }
            return nil
        }
        let before = MenuTracking.shared.open.count
        guard let open = await openPopUp(e) else { throw DriverError("the menu did not open: \(e)") }
        guard let (menu, index) = find(open) else {
            let titles = open.items.map(\.title)
            await closeMenus(downTo: before)
            throw DriverError("no menu item starting with \(item) in \(titles)")
        }
        guard menu.items[index].isEnabled else {
            await closeMenus(downTo: before)
            throw DriverError("menu item is disabled: \(menu.items[index].title)")
        }
        menu.performActionForItem(at: index)
        await closeMenus(downTo: before)
    }

    /// Opens a workspace with the Go menu, else by selecting the sidebar row.
    func capNav(_ workspace: String) async throws {
        capBigWindow()
        try await go(workspace)
        await idle(0.2)
    }

    func capTypeIn(_ text: String, into id: String? = nil, label: String? = nil, timeout: Double = 15) async throws {
        try await capTypeIn(text, strict: false) { try await self.wait(id: id, label: label, timeout: timeout) }
    }

    /// Types into the text input `find` returns. Use it for fields that share an identifier and
    /// have no label of their own (their container's identifier covers them); focus then must be on
    /// that very input.
    func capTypeIn(_ text: String, strict: Bool, find: () async throws -> Element) async throws {
        var e = try await find()
        func window(of e: Element) -> NSWindow? {
            let c = NSPoint(x: e.frame.midX, y: e.frame.midY)
            return NSApp.orderedWindows.first { $0.isVisible && $0.frame.contains(c) }
        }
        func hasFocus(_ e: Element) -> NSTextView? {
            guard let w = window(of: e), let tv = w.firstResponder as? NSTextView else { return nil }
            let owner: NSObject = (tv.isFieldEditor ? (tv.delegate as? NSObject) : tv) ?? tv
            let o = Element(obj: owner)
            if owner === e.obj || (strict && e.frame.insetBy(dx: -4, dy: -4).contains(NSPoint(x: o.frame.midX, y: o.frame.midY))) { return tv }
            if !strict && ((!e.label.isEmpty && o.label == e.label) || (!e.identifier.isEmpty && o.identifier == e.identifier)) { return tv }
            return nil
        }
        var target: NSTextView?
        for attempt in 0..<5 {
            await capReveal(e)
            window(of: e)?.makeKeyAndOrderFront(nil)
            if attempt >= 2 { try await focus(e) } else if e.frame.width > 0 { capPostClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)) }
            await idle(0.3)
            if let tv = hasFocus(e) { target = tv; break }
            if e.frame.width > 0 { capPostClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY)); await idle(0.4) }
            if let tv = hasFocus(e) { target = tv; break }
            if !strict, attempt >= 3, let w = window(of: e), let tv = w.firstResponder as? NSTextView { target = tv; break }
            e = try await find()
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

    /// Hides the project sidebar and checks that it went: the toolbar button then reads Show Sidebar.
    /// A press on the button did not always take, so View > Hide Sidebar is the second way.
    func capHideSidebar() async {
        for attempt in 0..<3 {
            if exists(label: "Show Sidebar") { return }
            if attempt == 0 { _ = await tryPress(id: "sidebar-toggle", wait: 3) } else { try? await menu(["View", "Hide Sidebar"]) }
            await idle(0.8)
        }
        if !exists(label: "Show Sidebar") { log("the sidebar is still shown") }
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

extension UI {
    func capAct(id: String? = nil, label: String? = nil, role: String? = nil, timeout: Double = 8) async throws {
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

    func capTab(_ name: String) async throws {
        try await capAct(label: name, role: "AXRadioButton")
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
        let before = MenuTracking.shared.open.count
        let menu = await openPopUp(e)
        let copy = menu?.copy() as? NSMenu
        await closeMenus(downTo: before)
        guard let copy else { throw DriverError("the menu did not open: \(e)") }
        return copy
    }

    /// Clicks a pop-up button and waits until AppKit says its menu is tracking. A pop-up button's
    /// menu holds its items before it opens, so the items alone do not tell that the click landed:
    /// a menu closed before its click arrives would open afterwards and never close.
    func openPopUp(_ e: Element) async -> NSMenu? {
        guard let cell = e.obj as? NSPopUpButtonCell else { return nil }
        let tracking = MenuTracking.shared
        let before = tracking.open.count
        for _ in 0..<3 {
            capPostClick(atScreen: NSPoint(x: e.frame.midX, y: e.frame.midY))
            let deadline = Date().addingTimeInterval(2)
            while Date() < deadline {
                if tracking.open.count > before { return tracking.open.last ?? cell.menu }
                await idle(0.1)
            }
        }
        // A click that never reaches the button (one below the visible part of an inspector) opens
        // nothing. Close a menu that opens late, then read the items the button holds unopened.
        await idle(0.5)
        await closeMenus(downTo: before)
        guard let menu = cell.menu, menu.items.contains(where: { !$0.title.isEmpty }) else { return nil }
        log("the menu did not open on a click; reading its items unopened")
        return menu
    }

    /// Scrolls the scroll view that holds an element until the whole element is in sight, the way a
    /// person scrolls to it before clicking. A click posted at a frame outside the visible part of a
    /// scroll view lands on whatever covers that point instead.
    func capReveal(_ e: Element) async {
        let f = e.frame
        guard f.width > 0, f.height > 0 else { return }
        let center = NSPoint(x: f.midX, y: f.midY)
        func scrollViews(_ v: NSView, _ out: inout [NSScrollView]) {
            if let sv = v as? NSScrollView { out.append(sv) }
            for c in v.subviews { scrollViews(c, &out) }
        }
        var found: [NSScrollView] = []
        for w in NSApp.windows where w.isVisible { if let root = w.contentView?.superview ?? w.contentView { scrollViews(root, &found) } }
        // The innermost scroll view whose content holds the element.
        let holders = found.filter { sv in
            guard let doc = sv.documentView, let w = sv.window else { return false }
            let frame = w.convertToScreen(sv.convert(sv.bounds, to: nil))
            return center.x >= frame.minX && center.x <= frame.maxX && w.convertToScreen(doc.convert(doc.bounds, to: nil)).contains(center)
        }
        guard let sv = holders.min(by: { ($0.documentView?.frame.height ?? 0) < ($1.documentView?.frame.height ?? 0) }),
              let doc = sv.documentView, let w = sv.window else { return }
        let visible = w.convertToScreen(sv.contentView.convert(sv.contentView.bounds, to: nil))
        if visible.contains(f) { return }
        let local = doc.convert(w.convertFromScreen(f), from: nil)
        doc.scrollToVisible(local.insetBy(dx: 0, dy: -12))
        sv.reflectScrolledClipView(sv.contentView)
        await idle(0.4)
    }

    /// Scrolls the scroll view that holds an element (an inspector, a settings page) until the element
    /// is in sight, the way a person scrolls down to a control below the visible part of a panel. A
    /// click posted at a point outside the visible part lands on whatever covers that point instead.
    func capScrollTo(id: String) async throws {
        let e = try await wait(id: id, timeout: 8)
        func scrollViews(_ v: NSView, _ out: inout [NSScrollView]) {
            if let sv = v as? NSScrollView { out.append(sv) }
            for c in v.subviews { scrollViews(c, &out) }
        }
        let x = e.frame.midX
        var picked: NSScrollView?
        // Front window first (a sheet before its parent), then the tallest scrollable view in its column.
        for w in NSApp.orderedWindows where w.isVisible && w.frame.minX <= x && x <= w.frame.maxX {
            var found: [NSScrollView] = []
            if let root = w.contentView?.superview ?? w.contentView { scrollViews(root, &found) }
            let holders = found.filter { sv in
                guard let doc = sv.documentView, sv.window != nil else { return false }
                let vis = w.convertToScreen(sv.convert(sv.bounds, to: nil))
                return x >= vis.minX && x <= vis.maxX && doc.frame.height > sv.contentView.bounds.height + 4
            }
            if let sv = holders.max(by: { $0.frame.height < $1.frame.height }) { picked = sv; break }
        }
        guard let sv = picked, let doc = sv.documentView, let w = sv.window else { throw DriverError("no scroll view holds \(id)") }
        func visible() -> CGRect { w.convertToScreen(sv.contentView.convert(sv.contentView.bounds, to: nil)) }
        for _ in 0..<15 {
            let vis = visible()
            if vis.contains(e.frame) { break }
            let clip = sv.contentView
            let delta: CGFloat = e.frame.midY < vis.midY ? 160 : -160
            let y = max(0, min(clip.bounds.origin.y + (doc.isFlipped ? delta : -delta), doc.frame.height - clip.bounds.height))
            clip.scroll(to: NSPoint(x: clip.bounds.origin.x, y: y))
            sv.reflectScrolledClipView(clip)
            await idle(0.25)
        }
        guard visible().contains(e.frame) else { throw DriverError("\(id) is still out of sight: \(e.frame) in \(visible())") }
        await idle(0.3)
    }

    /// Closes the menus that opened after `count` were tracking, and waits until AppKit says they closed.
    func closeMenus(downTo count: Int) async {
        let tracking = MenuTracking.shared
        for attempt in 0..<20 where tracking.open.count > count {
            tracking.open.last?.cancelTrackingWithoutAnimation()
            await idle(0.15)
            if tracking.open.count > count && attempt >= 2 {
                for t in [NSEvent.EventType.keyDown, .keyUp] {
                    if let ev = NSEvent.keyEvent(with: t, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                                 windowNumber: 0, context: nil, characters: "\u{1b}", charactersIgnoringModifiers: "\u{1b}", isARepeat: false, keyCode: 53) {
                        NSApp.postEvent(ev, atStart: true)
                    }
                }
                await idle(0.15)
            }
        }
        if tracking.open.count > count { log("a menu is still open after closing it: \(tracking.open.map(\.title))") }
        await idle(0.2)
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
            try await capAct(id: press, label: label, role: role)
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
                try await capTab(l)
                let n = "\(prefix)-\(slug(l))"
                await capture(n, title: "\(area): \(l)", area: area, from: last, via: l)
                last = n
            }
        }
        if let f = labels.first { try? await capTab(f) }
    }

    func capCard(_ text: String) -> Element? {
        all().first { $0.identifier.hasPrefix("story-card-") && ($0.label.hasPrefix("Scene Card: \(text)") || $0.label.hasPrefix("Beat: \(text)")) }
    }
}

extension UI {
    /// The context menu a right click on a point opens, as the hosting view builds it.
    func capContextMenuItems(at center: NSPoint) -> NSMenu? {
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
            guard let menu = capContextMenuItems(at: NSPoint(x: f.midX, y: f.midY)), let r = relFrame(e) else { throw DriverError("no context menu on \(e)") }
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

    func capCustomAction(_ e: Element, _ name: String) async throws {
        let sel = NSSelectorFromString("accessibilityCustomActions")
        guard e.obj.responds(to: sel), let list = e.obj.perform(sel)?.takeUnretainedValue() as? [NSAccessibilityCustomAction] else {
            throw DriverError("no accessibility actions on \(e)")
        }
        guard let a = list.first(where: { $0.name == name }) else { throw DriverError("no action \(name) in \(list.map(\.name))") }
        _ = a.handler?()
        await idle(0.4)
    }

    /// Chooses an item of a pop-up button by its title (from the start).
    func choose(_ id: String, _ item: String) async throws { try await capMenuButton(id: id, item: item) }

    func projectMenuPeek(name: String, area: String = "Project", from: String? = nil) async {
        await capMenu(id: "project-menu", name: name, title: "Project menu", area: area, from: from)
    }
}

extension UI {
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
                try await capMenuButton(id: id, item: t)
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
                try await capAct(id: id)
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
        try await ui.capAct(id: "select-source")
        let deadline = Date().addingTimeInterval(15)
        while Date() < deadline, ui.capQueryPrefix("draft-").isEmpty { await ui.idle(0.2) }
        guard let d = ui.capQueryPrefix("draft-").first else { throw DriverError("no draft choice") }
        try await ui.press(d)
        try await ui.capAct(id: "set-production-source")
        try await ui.waitText("Production Source:")
        for n in 1...2 {
            try await capSelectScene(ui, n)
            try await ui.wait(label: "Scene \(n) breakdown", timeout: 15)
            try await ui.capAct(id: "suggest-elements")
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
    static let captureGroup: [Scenario] = captureStart + captureHome + captureScreenplay + captureStory + captureVault + captureBreakdown + captureSchedule + captureCallSheets + captureBudget + captureVisual + captureProjectPages + captureAI + capturePackages + captureLifecycle + captureAdded
}

/// The menus AppKit is tracking now, from its begin and end notifications.
@MainActor
final class MenuTracking {
    static let shared = MenuTracking()
    private(set) var open: [NSMenu] = []

    private init() {
        let nc = NotificationCenter.default
        nc.addObserver(forName: NSMenu.didBeginTrackingNotification, object: nil, queue: nil) { n in
            guard let m = n.object as? NSMenu else { return }
            MainActor.assumeIsolated { MenuTracking.shared.open.append(m) }
        }
        nc.addObserver(forName: NSMenu.didEndTrackingNotification, object: nil, queue: nil) { n in
            guard let m = n.object as? NSMenu else { return }
            MainActor.assumeIsolated { MenuTracking.shared.open.removeAll { $0 === m } }
        }
    }
}
