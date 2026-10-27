// Real-app scenarios for the Windows port of the writing screens: the Story page (opens first, A4
// sheet, formatting kept across a restart, Dark page), the Beat Board in Horizontal rows, the
// screenplay's Dark page and the first-page number switch, and the character relationship maps.

import AppKit
import Foundation
import PDFKit

// MARK: helpers (only for this file)

fileprivate extension UI {
    /// The Story page's text view.
    func storyText(timeout: Double = 15) async throws -> NSTextView {
        let e = try await wait(id: "story-page-text", timeout: timeout, "the Story page")
        guard let tv = e.obj as? NSTextView else { throw DriverError("the Story page is not a text view: \(e)") }
        return tv
    }

    /// Whether the text at `index` of the Story page is bold.
    func storyBold(at index: Int) async throws -> Bool {
        let tv = try await storyText()
        guard let s = tv.textStorage, index < s.length else { return false }
        return (s.attribute(NSAttributedString.Key("of.story.b"), at: index, effectiveRange: nil) as? Bool) == true
    }

    /// A Command shortcut on the Story page. With the app in front it goes through the event
    /// queue as the keyboard sends it; with another app in front (no key window) it is offered
    /// to the page's window and then the menu bar, the order AppKit uses for a key equivalent.
    func storyShortcut(_ c: String, code: UInt16, in tv: NSTextView) async throws {
        if NSApp.isActive && NSApp.keyWindow != nil { return try await shortcut(c, code: code) }
        guard let w = tv.window, let ev = NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: .command, timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: w.windowNumber, context: nil, characters: c, charactersIgnoringModifiers: c,
            isARepeat: false, keyCode: code) else { throw DriverError("no window for Command+\(c)") }
        log("Command+\(c) with no active app: offered to the window, then the menu bar")
        if !w.performKeyEquivalent(with: ev) { _ = NSApp.mainMenu?.performKeyEquivalent(with: ev) }
        await idle(0.2)
    }
}

extension Scenarios {
    static let portLine = "The night bus leaves at ten."

    /// Register in Scenarios.swift: `+ writingPortScenarios` in `writingGroup` or `all`.
    static let writingPortScenarios: [Scenario] = [
        Scenario("03-story-page/opens-on-the-page-formats-and-keeps-it", phases: 2, timeout: 400) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Story Sheet")
                // The Story workspace opens on the Story page: no tab is chosen first.
                try await ui.go("Story")
                let tv = try await ui.storyText(timeout: 20)
                tv.window?.makeKeyAndOrderFront(nil)
                tv.window?.makeFirstResponder(tv)
                try await ui.typeHere(portLine)
                // Command+B on "The night".
                tv.setSelectedRange(NSRange(location: 0, length: 9))
                try await ui.storyShortcut("b", code: 11, in: tv)
                let bold = try await ui.storyBold(at: 0)
                try ui.expect(bold, "Command+B made the selection bold")
                // Command+] one size up while the page has the keyboard.
                try await ui.storyShortcut("]", code: 30, in: tv)
                try await ui.waitText("Saved", timeout: 15)
                try await ui.press(id: "story-page-dark")
                await ui.idle(0.5)
                try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua, "the sheet is dark")
                try ui.snapshot("story-page-dark")
                try await ui.press(id: "story-page-dark")
                try await ui.press(id: "story-page-zoom-in")
                try await ui.waitText("110%", timeout: 5)
                try await ui.press(id: "story-page-zoom-reset")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Story Sheet")
                try await ui.go("Story")
                _ = try await ui.storyText(timeout: 20)
                try await ui.waitText(portLine, timeout: 15)
                let first = try await ui.storyBold(at: 0)
                let rest = try await ui.storyBold(at: 12)
                try ui.expect(first, "the bold text is still bold after a restart")
                try ui.expect(!rest, "the rest is not bold")
            }
        },

        Scenario("31-story-board-flow/horizontal-rows-are-remembered", phases: 2, timeout: 400) { ui, phase, ctx in
            try await ui.startApp(ctx)
            if phase == 1 {
                try await ui.createProject("Rows Night")
                try await ui.go("Story")
                try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
                try await Scenarios.addAct(ui, "Act One")
                try await Scenarios.addSequence(ui, "Depot")
                try await Scenarios.addCard(ui, "The driver waits")
                try await Scenarios.addCard(ui, "The bus pulls out")
                try await Scenarios.segment(ui, group: "story-board-layout", "Horizontal")
                await ui.idle(0.6)
                let board = try await ui.wait(id: "story-board", timeout: 10, "the Beat Board")
                try ui.expect(board.value.contains("Horizontal"), "the board says it is in rows")
                try ui.snapshot("story-board-rows")
                try await ui.closeProject()
            } else {
                try await ui.openProject("Rows Night")
                try await ui.go("Story")
                try await Scenarios.segment(ui, group: "story-tabs", "Beat Board")
                let board = try await ui.wait(id: "story-board", timeout: 15, "the Beat Board")
                try ui.expect(board.value.contains("Horizontal"), "the layout was remembered")
                try await Scenarios.segment(ui, group: "story-board-layout", "Vertical")
            }
        },

        Scenario("44-scene-hub-comments-compare/dark-page-is-screen-only", timeout: 400) { ui, _, ctx in
            try await ui.startApp(ctx)
            try await ui.createProject("Dark Draft")
            try await ui.go("Screenplay")
            let tv = try await ui.editor()
            await ui.caretToEnd(tv)
            try await ui.typeHere(portLine)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua, "the page is white paper by default")
            // Dark page is an item of the Screenplay Tools menu (a pop-up menu, chosen by title).
            try await Scenarios.chooseMenuItem(ui, button: "screenplay-tools-menu", item: "Dark page")
            await ui.idle(0.6)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua, "the page is dark paper")
            try ui.expect(ui.brightness(tv.backgroundColor) < 0.2, "the page background is dark: \(tv.backgroundColor)")
            try ui.snapshot("screenplay-dark-page")
            // Print and PDF are not changed: the Print preview (the PDF the core writes) is white paper.
            try await Scenarios.chooseMenuItem(ui, button: "screenplay-tools-menu", item: "Print")
            _ = try await ui.wait(id: "print-preview-sheet", timeout: 30, "the Print preview")
            // The last page holds the script (the first is the title page).
            var page: PDFPage?
            let until = Date().addingTimeInterval(30)
            while Date() < until, page == nil {
                if let doc = ui.previewPDFView()?.document, doc.pageCount > 0 { page = doc.page(at: doc.pageCount - 1) }
                if page == nil { await ui.idle(0.2) }
            }
            guard let page else { throw DriverError("the Print preview showed no page") }
            try ui.expect(page.string?.contains(portLine) == true, "the printed page holds the script: \(page.string ?? "")")
            let (mean, darkest) = try ui.pageBrightness(page)
            try ui.expect(mean > 0.9, "the printed page is white paper (mean brightness \(mean))")
            try ui.expect(darkest < 0.4, "the printed text is dark ink (darkest \(darkest))")
            try ui.snapshot("screenplay-dark-page-print")
            try await ui.press(id: "print-close")
            await ui.idle(0.6)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua, "the page is still dark after printing")
            try await Scenarios.chooseMenuItem(ui, button: "screenplay-tools-menu", item: "Light page")
            await ui.idle(0.4)
            try ui.expect(tv.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .aqua, "the page is white again")
            try ui.expect(ui.brightness(tv.backgroundColor) > 0.9, "the page background is white again: \(tv.backgroundColor)")
        },
    ]
}

// MARK: helpers for the screenplay Dark page scenario

fileprivate extension UI {
    /// Brightness (0 black, 1 white) of a colour in sRGB.
    func brightness(_ color: NSColor?) -> Double {
        guard let c = color?.usingColorSpace(.sRGB) else { return -1 }
        return Double(0.2126 * c.redComponent + 0.7152 * c.greenComponent + 0.0722 * c.blueComponent)
    }

    /// The PDF view of the open Print preview.
    func previewPDFView() -> PDFView? {
        func find(_ v: NSView) -> PDFView? {
            if let p = v as? PDFView { return p }
            for s in v.subviews { if let p = find(s) { return p } }
            return nil
        }
        for w in NSApp.windows where w.isVisible { if let c = w.contentView, let p = find(c) { return p } }
        return nil
    }

    /// Mean and darkest brightness of a PDF page drawn as the PDF holds it.
    func pageBrightness(_ page: PDFPage) throws -> (Double, Double) {
        let bounds = page.bounds(for: .mediaBox)
        let w = 300, h = max(1, Int(300 * bounds.height / max(bounds.width, 1)))
        guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: w, pixelsHigh: h, bitsPerSample: 8,
                                         samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                                         bytesPerRow: 0, bitsPerPixel: 0),
              let ctx = NSGraphicsContext(bitmapImageRep: rep) else { throw DriverError("the PDF page could not be drawn") }
        // White paper under the page, as a printer has: a dark page would have to paint its own fill.
        ctx.cgContext.setFillColor(CGColor(gray: 1, alpha: 1))
        ctx.cgContext.fill(CGRect(x: 0, y: 0, width: w, height: h))
        ctx.cgContext.scaleBy(x: CGFloat(w) / bounds.width, y: CGFloat(h) / bounds.height)
        page.draw(with: .mediaBox, to: ctx.cgContext)
        ctx.flushGraphics()
        var sum = 0.0, darkest = 1.0
        for y in 0..<h { for x in 0..<w {
            let b = brightness(rep.colorAt(x: x, y: y))
            sum += b
            darkest = min(darkest, b)
        } }
        return (sum / Double(w * h), darkest)
    }
}
