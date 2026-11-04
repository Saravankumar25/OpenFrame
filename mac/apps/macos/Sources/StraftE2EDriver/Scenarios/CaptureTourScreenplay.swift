// Capture tour section: Screenplay. Primitives and helpers are in CaptureTour.swift.
//
// Eight scenarios, each with its own app start: the editor and its page bar, an empty project,
// Find and Replace, the side panels (Scene Hub, private notes, comments), drafts and Compare,
// lock and revisions, page setup and title page, and export with its print preview.

import AppKit
import Foundation

fileprivate extension UI {
    func screenplayShot(_ name: String, _ title: String, from: String? = nil, via: String? = nil) async {
        await capture(name, title: title, area: "Screenplay", from: from, via: via)
    }

    /// Puts the caret after the first match of a text in the script, or selects the match.
    @discardableResult
    func screenplayCaret(_ text: String, select: Bool = false) async throws -> NSTextView {
        let tv = try await editor()
        let ns = tv.string as NSString
        let r = ns.range(of: text)
        guard r.location != NSNotFound else { throw DriverError("the script has no \(text)") }
        tv.window?.makeFirstResponder(tv)
        tv.setSelectedRange(select ? r : NSRange(location: r.location + r.length, length: 0))
        tv.scrollRangeToVisible(r)
        await idle(0.4)
        return tv
    }

    /// Chooses an item of the Screenplay Tools menu (submenus included).
    func screenplayTools(_ item: String) async throws {
        try await capMenuButton(id: "screenplay-tools-menu", item: item)
        await idle(0.9)
    }

    func screenplayButton(_ label: String, timeout: Double = 8) async throws {
        try await capAct(label: label, role: "AXButton", timeout: timeout)
        await idle(0.6)
    }

    /// A button inside the element whose label contains `container` (a comment card, for one).
    func screenplayButton(_ label: String, inside container: String) async throws {
        guard let card = all().first(where: { $0.label.contains(container) && !$0.children.isEmpty }) else {
            throw DriverError("nothing labelled \(container)")
        }
        func find(_ e: Element, _ d: Int) -> Element? {
            if e.role == "AXButton" && (e.label == label || e.title == label) { return e }
            guard d < 14 else { return nil }
            for c in e.children { if let f = find(c, d + 1) { return f } }
            return nil
        }
        guard let b = find(card, 0) else { throw DriverError("no \(label) button in \(container)") }
        if !b.performPress() { try await tap(b) }
        await idle(0.8)
    }

    /// A segment of a segmented picker, by the start of its title ("Resolved", "Text-only").
    /// The segmented control is chosen directly: its segments refuse the accessibility press.
    func screenplaySegment(_ prefix: String) async throws {
        func controls(_ v: NSView) -> [NSSegmentedControl] {
            var out: [NSSegmentedControl] = []
            if let s = v as? NSSegmentedControl { out.append(s) }
            for c in v.subviews { out += controls(c) }
            return out
        }
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            for w in NSApp.windows where w.isVisible {
                guard let root = w.contentView?.superview ?? w.contentView else { continue }
                for sc in controls(root) where sc.window != nil && !sc.isHiddenOrHasHiddenAncestor {
                    for i in 0..<sc.segmentCount where (sc.label(forSegment: i) ?? "").hasPrefix(prefix) {
                        sc.selectedSegment = i
                        if let action = sc.action { NSApp.sendAction(action, to: sc.target, from: sc) }
                        await idle(0.8)
                        return
                    }
                }
            }
            await idle(0.2)
        }
        throw DriverError("no segment \(prefix)")
    }

    /// Types into a text area found by its label (a SwiftUI TextEditor), never into a button of the same name.
    func screenplayTypeArea(_ label: String, _ text: String) async throws {
        let deadline = Date().addingTimeInterval(8)
        var target: Element?
        while Date() < deadline, target == nil {
            target = query(label: label).first { ["AXTextArea", "AXScrollArea", "AXTextField"].contains($0.role) && $0.frame.width > 0 }
            if target == nil { await idle(0.2) }
        }
        guard let e = target else { throw DriverError("no text area \(label)") }
        if let tv = e.obj as? NSTextView, let w = tv.window { w.makeKeyAndOrderFront(nil); w.makeFirstResponder(tv) }
        else if let sv = e.obj as? NSScrollView, let tv = sv.documentView as? NSTextView, let w = tv.window { w.makeKeyAndOrderFront(nil); w.makeFirstResponder(tv) }
        else { try await tap(e) }
        await idle(0.3)
        guard let tv = (NSApp.keyWindow ?? windows.last)?.firstResponder as? NSTextView else { throw DriverError("no keyboard focus in \(label)") }
        tv.selectAll(nil)
        tv.insertText(text, replacementRange: tv.selectedRange())
        await idle(0.4)
    }

    /// A pop-up button (a SwiftUI Picker) by its accessibility label, never the text beside it.
    func screenplayPopUp(_ label: String) async throws -> (Element, NSPopUpButtonCell) {
        let deadline = Date().addingTimeInterval(8)
        while Date() < deadline {
            // A Picker with a hidden label reads its label twice ("Paper, Paper").
            if let e = (query(label: label) + query(label: "\(label), \(label)")).first(where: { $0.obj is NSPopUpButtonCell }), let c = e.obj as? NSPopUpButtonCell { return (e, c) }
            await idle(0.2)
        }
        throw DriverError("no pop-up button \(label)")
    }

    func screenplayPick(_ label: String, _ item: String) async throws {
        let (_, cell) = try await screenplayPopUp(label)
        guard let menu = cell.menu, let i = menu.items.firstIndex(where: { $0.title.hasPrefix(item) }) else { throw DriverError("no \(item) in \(label)") }
        menu.performActionForItem(at: i)
        await idle(0.6)
    }

    func screenplayPopUpShot(_ label: String, name: String, title: String, from: String?) async {
        await step("pop-up \(name)") {
            let (e, _) = try await screenplayPopUp(label)
            guard let r = relFrame(e) else { throw DriverError("no frame") }
            let menu = try await peekMenu(e)
            let w = capturePrimary()?.frame.width ?? 1200
            await capture(name, title: title, area: "Screenplay", from: from, via: label, menu: menu, anchor: CGPoint(x: min(r.minX, w - 260), y: r.maxY + 4))
        }
    }

    /// The name the draft menu shows ("Draft: <name>. Switch draft").
    func screenplayDraftName() -> String {
        guard let l = query(id: "draft-menu").first?.label, l.hasPrefix("Draft: ") else { return "" }
        return String(l.dropFirst("Draft: ".count).dropLast(". Switch draft".count))
    }

    /// The right-click menu of the script page, drawn where the click was.
    func screenplayEditorContext(name: String, title: String, from: String?) async {
        await step("editor context \(name)") {
            let tv = try await editor()
            guard let w = tv.window else { throw DriverError("the editor has no window") }
            let range = tv.selectedRange()
            var local = NSPoint(x: tv.visibleRect.midX, y: tv.visibleRect.midY)
            if let lm = tv.layoutManager, let tc = tv.textContainer {
                let g = lm.glyphRange(forCharacterRange: NSRange(location: range.location, length: max(range.length, 1)), actualCharacterRange: nil)
                let rect = lm.boundingRect(forGlyphRange: g, in: tc)
                local = NSPoint(x: rect.midX + tv.textContainerOrigin.x, y: rect.midY + tv.textContainerOrigin.y)
            }
            let screen = w.convertPoint(toScreen: tv.convert(local, to: nil))
            guard let menu = capContextMenuItems(at: screen) else { throw DriverError("no context menu on the page") }
            let anchor = CGPoint(x: screen.x - w.frame.minX, y: w.frame.maxY - screen.y)
            await capture(name, title: title, area: "Screenplay", from: from, via: "screenplay-editor", menu: menu.copy() as? NSMenu ?? menu, anchor: anchor)
        }
    }

    /// Types into the find bar of the page (the system find bar the editor uses).
    func screenplayFindType(_ text: String) async throws {
        let tv = try await editor()
        func fields(_ v: NSView) -> [NSTextField] {
            var out: [NSTextField] = []
            if let f = v as? NSTextField, f.isEditable { out.append(f) }
            for c in v.subviews { out += fields(c) }
            return out
        }
        guard let bar = tv.enclosingScrollView?.findBarView, let field = fields(bar).first, let w = field.window else {
            throw DriverError("the find bar has no field")
        }
        w.makeKeyAndOrderFront(nil)
        w.makeFirstResponder(field)
        await idle(0.3)
        guard let editor = field.currentEditor() as? NSTextView else { throw DriverError("the find field took no focus") }
        editor.selectAll(nil)
        editor.insertText(text, replacementRange: editor.selectedRange())
        await idle(1.2)
    }

    /// The page bar's zoom: presses Zoom In or Zoom Out until the label reads `percent`.
    func screenplayZoom(to percent: Int) async throws {
        for _ in 0..<20 {
            let label = query(id: "screenplay-zoom-reset").first?.label ?? ""
            let now = Int(label.replacingOccurrences(of: "Zoom ", with: "").split(separator: "%").first ?? "") ?? 100
            if now == percent { await idle(0.6); return }
            try await capAct(id: now < percent ? "screenplay-zoom-in" : "screenplay-zoom-out")
            await idle(0.25)
        }
        throw DriverError("the zoom never reached \(percent)%")
    }
}

extension Scenarios {
    static let captureScreenplay: [Scenario] = [
        // The editor with the imported example, the page bar and its menus, the scene list, layouts and zoom.
        tour("02-screenplay-editor", order: 2000) { ui, _ in
            await ui.capture("screenplay-editor", title: "Screenplay editor with the example script", area: "Screenplay")
            await ui.step("type") {
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("\n\nEXT. DEPOT - DAWN\n")
                await ui.idle(0.6)
                await ui.capture("screenplay-editor-typing", title: "Screenplay editor while typing", area: "Screenplay", from: "screenplay-editor", via: "screenplay-editor")
            }
            await ui.exploreMenu("screenplay-tools-menu", area: "Screenplay", prefix: "screenplay-tools", from: "screenplay-editor")
            await ui.step("standard mode after the tools menu") { try await ui.screenplayTools("Standard Mode") }
            await ui.exploreMenu("draft-menu", area: "Screenplay", prefix: "screenplay-draft", from: "screenplay-editor")
            await ui.capMenu(id: "element-picker", name: "screenplay-element-picker", title: "Screenplay: element type menu", area: "Screenplay", from: "screenplay-editor")
            await ui.step("element picker on dialogue") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                await ui.capMenu(id: "element-picker", name: "screenplay-element-picker-dialogue", title: "Element type menu on a line of dialogue", area: "Screenplay", from: "screenplay-editor")
            }
            await ui.exploreButtons([("comments-button", "Comments"), ("compare-button", "Compare drafts"), ("replace-toggle", "Find and replace")],
                                    area: "Screenplay", prefix: "screenplay", from: "screenplay-editor")
            // The replace bar stays open after its button; close it so later pages do not carry it.
            await ui.tryPress(id: "replace-close", wait: 2)

            // The screenplay picker and its sheets.
            await ui.capMenu(id: "screenplay-picker", name: "screenplay-picker-menu", title: "Screenplay picker menu", area: "Screenplay", from: "screenplay-editor")
            await ui.step("new screenplay sheet") {
                try await ui.capMenuButton(id: "screenplay-picker", item: "New Screenplay")
                _ = try await ui.wait(id: "screenplay-title", timeout: 8)
                await ui.screenplayShot("screenplay-new-screenplay-sheet", "New Screenplay sheet", from: "screenplay-picker-menu", via: "screenplay-picker")
                try await ui.fill("Title", "Lost Property", id: "screenplay-title")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-new-screenplay-duplicate", "New Screenplay: the title is already taken", from: "screenplay-new-screenplay-sheet", via: "screenplay-title")
                await ui.capRecover()
            }
            await ui.step("rename screenplay sheet") {
                try await ui.capMenuButton(id: "screenplay-picker", item: "Rename Screenplay")
                _ = try await ui.wait(id: "screenplay-title", timeout: 8)
                await ui.screenplayShot("screenplay-rename-screenplay-sheet", "Rename Screenplay sheet", from: "screenplay-picker-menu", via: "screenplay-picker")
                await ui.capRecover()
            }
            await ui.step("delete screenplay sheet") {
                try await ui.capMenuButton(id: "screenplay-picker", item: "Delete Screenplay")
                try await ui.waitText("Recently Deleted", timeout: 8)
                await ui.screenplayShot("screenplay-delete-screenplay-sheet", "Delete Screenplay confirmation", from: "screenplay-picker-menu", via: "screenplay-picker")
                await ui.capRecover()
            }

            // The scene list: hidden, columns, context menu.
            await ui.step("scenes hidden") {
                try await ui.capAct(id: "scenes-toggle")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-scenes-hidden", "Screenplay with the scene list hidden", from: "screenplay-editor", via: "scenes-toggle")
                try await ui.capAct(id: "scenes-toggle")
                await ui.idle(0.8)
            }
            await ui.capMenu(id: "navigator-columns", name: "screenplay-navigator-columns-menu", title: "Scene list: Columns menu", area: "Screenplay", from: "screenplay-editor")
            await ui.step("navigator columns shown") {
                try await ui.capMenuButton(id: "navigator-columns", item: "Page")
                await ui.idle(0.5)
                try await ui.capMenuButton(id: "navigator-columns", item: "Notes")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-navigator-columns-shown", "Scene list with the Page and Notes columns", from: "screenplay-navigator-columns-menu", via: "navigator-columns")
                try await ui.capMenuButton(id: "navigator-columns", item: "Page")
                await ui.idle(0.4)
                try await ui.capMenuButton(id: "navigator-columns", item: "Notes")
                await ui.idle(0.4)
            }
            await ui.step("scene row context menu") {
                guard let row = ui.capQueryPrefix("scene-row-").dropFirst().first(where: { $0.role == "AXButton" }) ?? ui.capQueryPrefix("scene-row-").first else { throw DriverError("no scene rows") }
                await ui.capContext(on: row, name: "screenplay-scene-row-context", title: "Scene list: right-click menu of a scene", area: "Screenplay", from: "screenplay-editor")
            }
            await ui.step("editor context menus") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                await ui.screenplayEditorContext(name: "screenplay-editor-context", title: "Script page: right-click menu", from: "screenplay-editor")
                try await ui.screenplayCaret("REGISTER in slow", select: true)
                await ui.screenplayShot("screenplay-editor-selection", "Script page with a word selected", from: "screenplay-editor", via: "screenplay-editor")
                await ui.screenplayEditorContext(name: "screenplay-editor-context-selection", title: "Script page: right-click menu on a selection", from: "screenplay-editor-selection")
            }
            await ui.step("misspelled word context") {
                let tv = try await ui.editor()
                tv.scrollRangeToVisible(NSRange(location: 0, length: 0))
                try await ui.screenplayCaret("A knock on the window bars.")
                try await ui.typeHere(" Kiran waits by teh door.")
                await ui.idle(1.2)
                tv.scrollRangeToVisible(NSRange(location: 0, length: 0))
                try await ui.screenplayCaret("teh", select: true)
                await ui.screenplayEditorContext(name: "screenplay-editor-context-spelling", title: "Script page: right-click menu on a misspelled word", from: "screenplay-editor")
            }

            // Zoom.
            await ui.step("zoom") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                try await ui.screenplayZoom(to: 50)
                await ui.screenplayShot("screenplay-zoom-50", "Screenplay zoomed out to 50 percent", from: "screenplay-editor", via: "screenplay-zoom-out")
                try await ui.screenplayZoom(to: 150)
                await ui.screenplayShot("screenplay-zoom-150", "Screenplay zoomed in to 150 percent", from: "screenplay-editor", via: "screenplay-zoom-in")
                try await ui.screenplayZoom(to: 200)
                await ui.screenplayShot("screenplay-zoom-200", "Screenplay at the largest zoom, 200 percent", from: "screenplay-zoom-150", via: "screenplay-zoom-in")
                try await ui.capAct(id: "screenplay-zoom-reset")
                await ui.idle(0.5)
            }
            await ui.step("zoom reset safety") { try await ui.screenplayZoom(to: 100) }

            // Layouts.
            await ui.step("focus mode") {
                try await ui.screenplayTools("Focus Mode")
                _ = try await ui.wait(id: "focus-exit", timeout: 8)
                await ui.screenplayShot("screenplay-layout-focus", "Focus Mode: only the page", from: "screenplay-tools-menu", via: "layout-menu")
                try await ui.capAct(id: "focus-exit")
                await ui.idle(0.8)
            }
            await ui.step("standard mode again") { try await ui.screenplayTools("Standard Mode") }
        },

        // A new project with nothing written: the empty states of the editor and its panels.
        tour("02-screenplay-empty", order: 2100, project: false) { ui, _ in
            await ui.capSetMode()
            ui.capBigWindow()
            try await ui.createProject("Empty Short")
            ui.capBigWindow()
            try await ui.capNav("Screenplay")
            _ = try await ui.editor()
            await ui.idle(1.0)
            await ui.screenplayShot("screenplay-empty-editor", "Screenplay editor of a new project, nothing written yet")
            await ui.capMenu(id: "element-picker", name: "screenplay-empty-element-picker", title: "Element type menu on an empty page", area: "Screenplay", from: "screenplay-empty-editor")
            await ui.capMenu(id: "draft-menu", name: "screenplay-empty-draft-menu", title: "Draft menu with the first draft only", area: "Screenplay", from: "screenplay-empty-editor")
            // A new page opens with the heading suggestions; close them, and make room for the inspector.
            try? await ui.escape()
            await ui.tryPress(id: "sidebar-toggle", wait: 3)
            await ui.idle(0.8)
            await ui.step("empty comments") {
                try await ui.capAct(id: "comments-button")
                try await ui.waitText("No comments yet.", timeout: 8)
                await ui.screenplayShot("screenplay-empty-comments", "Comments panel with no comments", from: "screenplay-empty-editor", via: "comments-button")
            }
            await ui.step("empty scene hub") {
                try await ui.screenplaySegment("Scene")
                await ui.screenplayShot("screenplay-empty-scene-hub", "Scene Hub with no scene yet", from: "screenplay-empty-comments", via: "Scene")
                try await ui.screenplaySegment("Draft")
                await ui.screenplayShot("screenplay-empty-draft-info", "Draft information of the first draft", from: "screenplay-empty-comments", via: "Draft")
                try await ui.capAct(id: "inspector-close")
                await ui.idle(0.6)
            }
            await ui.tryPress(id: "sidebar-toggle", wait: 3)
            await ui.step("empty drafts") {
                try await ui.screenplayTools("Drafts")
                _ = try await ui.wait(id: "drafts-sheet", timeout: 8)
                await ui.screenplayShot("screenplay-empty-drafts", "Drafts sheet with one empty draft", from: "screenplay-tools-menu", via: "screenplay-tools-menu")
                await ui.capRecover()
            }
            await ui.step("empty page view") {
                try await ui.screenplayTools("Page View")
                _ = try await ui.wait(id: "page-view-sheet", timeout: 8)
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-empty-page-view", "Page View of an empty screenplay", from: "screenplay-tools-menu", via: "page-view-menu-item")
                await ui.capRecover()
            }
            await ui.step("empty export") {
                try await ui.screenplayTools("Export Screenplay")
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-empty-export", "Export Screenplay from an empty screenplay", from: "screenplay-tools-menu", via: "screenplay-export")
                await ui.capRecover()
            }
            await ui.step("first scene suggestions") {
                let tv = try await ui.editor()
                await ui.caretToEnd(tv)
                try await ui.typeHere("IN")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-empty-suggestions", "Typing a first scene heading: the suggestion list", from: "screenplay-empty-editor", via: "screenplay-editor")
                try await ui.escape()
                try await ui.typeHere("T. BUS DEPOT - NIGHT")
                try await ui.enter()
                try await ui.typeHere("Meera waits under the only working lamp.")
                await ui.idle(1.2)
                await ui.screenplayShot("screenplay-empty-first-scene", "The first scene written in a new project", from: "screenplay-empty-suggestions", via: "screenplay-editor")
            }
        },

        // Find in the script and Find and Replace, with matches and without.
        tour("02-screenplay-find", order: 2200) { ui, _ in
            await ui.step("find bar") {
                try await ui.screenplayCaret("FADE IN:")
                try await ui.capAct(id: "find-button")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-find-bar", "Find bar over the script", from: "screenplay-editor", via: "find-button")
                try await ui.screenplayFindType("kite")
                await ui.screenplayShot("screenplay-find-results", "Find: kite, the matches in the script", from: "screenplay-find-bar", via: "find-button")
                try await ui.screenplayFindType("zeppelin")
                await ui.screenplayShot("screenplay-find-no-results", "Find: no match for the word", from: "screenplay-find-bar", via: "find-button")
                if !(await ui.tryPress(label: "Done", wait: 2)) { try? await ui.escape() }
                await ui.idle(0.5)
            }
            await ui.step("replace whole script") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                try await ui.capAct(id: "replace-toggle")
                _ = try await ui.wait(id: "replace-bar", timeout: 8)
                try await ui.capTypeIn("goat", into: "replace-find")
                try await ui.capTypeIn("buffalo", into: "replace-with")
                await ui.screenplayShot("screenplay-replace-filled", "Find and Replace filled in, this scene only", from: "screenplay-find-and-replace", via: "replace-toggle")
                try await ui.screenplaySegment("Whole Script")
                try await ui.capAct(id: "replace-match-case")
                await ui.idle(0.4)
                await ui.screenplayShot("screenplay-replace-whole-script", "Find and Replace: whole script, match case", from: "screenplay-replace-filled", via: "replace-scope")
                try await ui.capAct(id: "replace-all")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-replace-done", "Find and Replace: the matches replaced", from: "screenplay-replace-whole-script", via: "replace-all")
            }
            await ui.step("replace no match") {
                try await ui.capTypeIn("zeppelin", into: "replace-find")
                try await ui.capAct(id: "replace-all")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-replace-no-match", "Find and Replace: nothing matches", from: "screenplay-replace-whole-script", via: "replace-all")
                try await ui.capAct(id: "replace-close")
                await ui.idle(0.5)
            }
        },

        // The inspector: Scene Hub with a private note, comments with a reply, a resolved one, the Draft tab.
        tour("02-screenplay-panels", order: 2300) { ui, _ in
            // The inspector needs the room of the project sidebar on a 1280-point screen.
            await ui.capHideSidebar()
            await ui.idle(0.8)
            await ui.step("scene hub details") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                try await ui.screenplayTools("Scene Hub")
                _ = try await ui.wait(id: "scene-hub", timeout: 8)
                try await ui.fill("Story day", "Day 1")
                await ui.idle(0.5)
                await ui.screenplayShot("screenplay-scene-hub-details-edited", "Scene Hub: scene details changed, not saved yet", from: "screenplay-tools-scene-hub", via: "Story day")
                try await ui.capAct(id: "scene-details-save")
                await ui.idle(0.8)
            }
            await ui.step("private note") {
                try await ui.capAct(id: "private-note-add")
                await ui.idle(0.6)
                await ui.screenplayShot("screenplay-private-note-new", "Scene Hub: writing a private note", from: "screenplay-tools-scene-hub", via: "private-note-add")
                try await ui.screenplayTypeArea("Private note", "Ask the station master if the goat story is true.")
                try await ui.capAct(id: "private-note-save")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-private-note-saved", "Scene Hub with a private note", from: "screenplay-private-note-new", via: "private-note-save")
            }
            // The note sits below the visible part of the inspector: scroll down to it first.
            await ui.step("private note in sight") { try await ui.capScrollTo(id: "private-note-actions") }
            await ui.capMenu(id: "private-note-actions", name: "screenplay-private-note-actions", title: "Private note actions menu", area: "Screenplay", from: "screenplay-private-note-saved")
            await ui.step("private note editing") {
                try await ui.capMenuButton(id: "private-note-actions", item: "Edit")
                await ui.idle(0.6)
                await ui.screenplayShot("screenplay-private-note-editing", "Editing a private note", from: "screenplay-private-note-actions", via: "private-note-actions")
                try await ui.screenplayButton("Cancel")
            }
            await ui.step("scene hub of another scene") {
                guard let row = ui.capQueryPrefix("scene-row-").filter({ $0.role == "AXButton" }).dropFirst(2).first else { throw DriverError("no third scene row") }
                guard let menu = ui.capContextMenuItems(at: NSPoint(x: row.frame.midX, y: row.frame.midY)), let i = menu.items.firstIndex(where: { $0.title == "Scene Hub" }) else { throw DriverError("no Scene Hub in the row menu") }
                menu.performActionForItem(at: i)
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-scene-hub-from-list", "Scene Hub of scene 3, opened from the scene list", from: "screenplay-scene-row-context", via: "Scene Hub")
            }
            await ui.step("comment composer with a quote") {
                try await ui.screenplayCaret("hand in a kite", select: true)
                try await ui.screenplayTools("Add Comment")
                _ = try await ui.wait(id: "comment-field", timeout: 8)
                await ui.screenplayShot("screenplay-comment-composer-quote", "Comment on selected text: the composer with the quote", from: "screenplay-editor-selection", via: "screenplay-tools-menu")
                try await ui.fill("Comment", "Should Kiran say which kite? The office has many.", id: "comment-field")
                try await ui.capAct(id: "comment-post")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-comment-posted", "Comments panel with one comment", from: "screenplay-comment-composer-quote", via: "comment-post")
            }
            await ui.step("reply") {
                try await ui.screenplayButton("Reply")
                await ui.screenplayShot("screenplay-comment-reply-composer", "Writing a reply to a comment", from: "screenplay-comment-posted", via: "Reply")
                try await ui.screenplayTypeArea("Reply", "Yes, the red one with the newspaper tail.")
                try await ui.screenplayButton("Reply")
                await ui.idle(0.6)
                await ui.screenplayShot("screenplay-comment-with-reply", "A comment with a reply", from: "screenplay-comment-reply-composer", via: "Reply")
            }
            await ui.step("draft comment resolved") {
                try await ui.screenplayButton("Comment on Draft")
                _ = try await ui.wait(id: "comment-field", timeout: 8)
                await ui.screenplayShot("screenplay-comment-composer-draft", "Composer for a comment on the whole draft", from: "screenplay-comment-with-reply", via: "Comment on Draft")
                try await ui.fill("Comment", "Title page date checked against the schedule.", id: "comment-field")
                try await ui.capAct(id: "comment-post")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-comments-two-open", "Comments panel with two open comments", from: "screenplay-comment-composer-draft", via: "comment-post")
                try await ui.screenplayButton("Resolve", inside: "Title page date checked")
                await ui.screenplayShot("screenplay-comments-after-resolve", "Comments panel after resolving a comment", from: "screenplay-comments-two-open", via: "Resolve")
                try await ui.screenplaySegment("Resolved")
                await ui.screenplayShot("screenplay-comments-resolved-tab", "Resolved comments", from: "screenplay-comments-after-resolve", via: "Resolved")
                try await ui.screenplaySegment("Open")
            }
            await ui.capMenu(label: "Comment actions", name: "screenplay-comment-actions", title: "Comment actions menu", area: "Screenplay", from: "screenplay-comment-with-reply")
            await ui.step("comment in discussion") {
                try await ui.capMenuButton(label: "Comment actions", item: "Mark In Discussion")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-comment-in-discussion", "A comment marked In Discussion", from: "screenplay-comment-actions", via: "Comment actions")
            }
            await ui.step("draft tab") {
                try await ui.screenplaySegment("Draft")
                await ui.screenplayShot("screenplay-inspector-draft", "Inspector: Draft information with open comments", from: "screenplay-comments-after-resolve", via: "Draft")
            }
            await ui.step("scene comment from the list") {
                guard let row = ui.capQueryPrefix("scene-row-").filter({ $0.role == "AXButton" }).dropFirst().first else { throw DriverError("no second scene row") }
                guard let menu = ui.capContextMenuItems(at: NSPoint(x: row.frame.midX, y: row.frame.midY)), let i = menu.items.firstIndex(where: { $0.title == "Add Comment" }) else { throw DriverError("no Add Comment in the row menu") }
                menu.performActionForItem(at: i)
                _ = try await ui.wait(id: "comment-field", timeout: 8)
                await ui.screenplayShot("screenplay-comment-composer-scene", "Composer for a comment on a whole scene", from: "screenplay-scene-row-context", via: "Add Comment")
                try? await ui.capAct(label: "Cancel", role: "AXButton")
            }
            await ui.tryPress(id: "sidebar-toggle", wait: 3)
        },

        // A second draft with two changed lines, the drafts sheet and Compare.
        tour("02-screenplay-drafts", order: 2400) { ui, _ in
            let first = ui.screenplayDraftName()
            await ui.step("new draft") {
                try await ui.capMenuButton(id: "draft-menu", item: "New Draft")
                _ = try await ui.wait(id: "new-draft-name", timeout: 8)
                try await ui.fill("Draft name", "Rewrite A", id: "new-draft-name")
                await ui.screenplayShot("screenplay-new-draft-named", "New Draft sheet with a name", from: "screenplay-draft-new-draft", via: "new-draft-name")
                try await ui.screenplayButton("Create Draft")
                try await ui.waitText("Created draft", timeout: 10)
                await ui.screenplayShot("screenplay-second-draft-created", "The second draft created and open", from: "screenplay-new-draft-named", via: "Create Draft")
            }
            await ui.step("two changed lines") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                try await ui.typeHere(" A red one.")
                try await ui.screenplayCaret("Let go!")
                try await ui.typeHere(" Now!")
                try await ui.waitSaved()
                await ui.screenplayShot("screenplay-second-draft-edited", "Rewrite A with two changed lines", from: "screenplay-second-draft-created", via: "screenplay-editor")
            }
            await ui.capMenu(id: "draft-menu", name: "screenplay-draft-menu-two", title: "Draft menu with two drafts", area: "Screenplay", from: "screenplay-second-draft-edited")
            await ui.step("drafts sheet") {
                try await ui.screenplayTools("Drafts")
                _ = try await ui.wait(id: "drafts-sheet", timeout: 8)
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-drafts-two", "Drafts sheet with two drafts", from: "screenplay-tools-menu", via: "screenplay-tools-menu")
                await ui.capMenu(label: "Actions for Rewrite A", name: "screenplay-draft-actions-current", title: "Actions menu of the current draft", area: "Screenplay", from: "screenplay-drafts-two")
                await ui.capMenu(label: "Actions for \(first)", name: "screenplay-draft-actions-other", title: "Actions menu of an earlier draft", area: "Screenplay", from: "screenplay-drafts-two")
            }
            await ui.step("rename draft") {
                try await ui.capMenuButton(label: "Actions for Rewrite A", item: "Rename")
                _ = try await ui.wait(id: "draft-name", timeout: 8)
                await ui.screenplayShot("screenplay-draft-rename", "Rename a draft", from: "screenplay-draft-actions-current", via: "Rename")
                try await ui.screenplayButton("Cancel")
            }
            await ui.step("delete draft") {
                try await ui.capMenuButton(label: "Actions for \(first)", item: "Delete")
                try await ui.waitText("Recently Deleted", timeout: 8)
                await ui.screenplayShot("screenplay-draft-delete", "Delete a draft: the confirmation", from: "screenplay-draft-actions-other", via: "Delete")
                try await ui.screenplayButton("Cancel")
            }
            await ui.step("new draft from earlier") {
                try await ui.capMenuButton(label: "Actions for \(first)", item: "New Draft From This")
                _ = try await ui.wait(id: "new-draft-name", timeout: 8)
                await ui.screenplayShot("screenplay-draft-new-from", "New draft from an earlier draft", from: "screenplay-draft-actions-other", via: "New Draft From This")
                try await ui.screenplayButton("Cancel")
            }
            await ui.capRecover()
            await ui.step("compare text only") {
                try await ui.capAct(id: "compare-button")
                _ = try await ui.wait(id: "compare-view", timeout: 10)
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-compare-text-only", "Compare: Text-only, side by side", from: "screenplay-second-draft-edited", via: "compare-button")
            }
            await ui.step("compare next change") {
                try await ui.capAct(label: "Next change", role: "AXButton")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-compare-next-change", "Compare: the next change", from: "screenplay-compare-text-only", via: "Next change")
            }
            await ui.step("compare scene only") {
                try await ui.screenplaySegment("Scene-only")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-compare-scene-only", "Compare: Scene-only", from: "screenplay-compare-text-only", via: "Scene-only")
            }
            await ui.step("compare inline") {
                try await ui.screenplaySegment("Text inline")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-compare-inline", "Compare: Text inline", from: "screenplay-compare-text-only", via: "Text inline")
            }
            await ui.capMenu(label: "Before", name: "screenplay-compare-before-menu", title: "Compare: Before draft menu", area: "Screenplay", from: "screenplay-compare-text-only")
            await ui.step("compare same draft") {
                try await ui.capMenuButton(label: "After", item: first)
                await ui.idle(0.8)
                try await ui.capMenuButton(label: "Before", item: first)
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-compare-same-draft", "Compare: the same draft on both sides", from: "screenplay-compare-before-menu", via: "Before")
            }
            await ui.capRecover()
        },

        // Lock as Shooting Draft, a revision with its colour, revised pages, page view with marks.
        tour("02-screenplay-lock", order: 2500) { ui, _ in
            await ui.step("lock") {
                try await ui.waitSaved()
                try await ui.screenplayTools("Lock as Shooting Draft")
                try await ui.waitText("Number of scenes", timeout: 15)
                try await ui.capAct(label: "Lock as Shooting Draft", role: "AXButton")
                try await ui.waitText("Shooting Draft", timeout: 10)
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-locked", "The Shooting Draft, locked", from: "screenplay-tools-lock-as-shooting-draft", via: "Lock as Shooting Draft")
            }
            await ui.capMenu(id: "screenplay-tools-menu", name: "screenplay-tools-menu-locked", title: "Screenplay Tools menu of a locked draft", area: "Screenplay", from: "screenplay-locked")
            await ui.step("revisions locked") {
                try await ui.screenplayTools("Revisions")
                _ = try await ui.wait(id: "revisions-sheet", timeout: 10)
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-revisions-locked", "Revisions of a locked draft, none yet", from: "screenplay-tools-menu-locked", via: "revisions-button")
            }
            await ui.step("start revision") {
                try await ui.capAct(id: "revisions-start")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-start-revision", "Start Revision: label, colour and date", from: "screenplay-revisions-locked", via: "revisions-start")
                try await ui.capAct(label: "Pink revision colour", role: "AXButton")
                await ui.idle(0.5)
                guard let go = ui.query(label: "Start Revision", role: "AXButton").first(where: { $0.identifier.isEmpty }) else { throw DriverError("no Start Revision button in the question") }
                if !go.performPress() { try await ui.tap(go) }
                try await ui.waitText("stays locked", timeout: 15)
                await ui.idle(0.6)
                await ui.screenplayShot("screenplay-revision-started", "A revision started from the locked draft", from: "screenplay-start-revision", via: "Start Revision")
                await ui.capRecover()
            }
            await ui.step("revised lines") {
                try await ui.screenplayCaret("Let go!")
                try await ui.typeHere(" Now!")
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                try await ui.typeHere(" A red one.")
                try await ui.waitSaved()
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-revision-edited", "Revision draft with revised lines", from: "screenplay-revision-started", via: "screenplay-editor")
            }
            await ui.step("revisions changed pages") {
                try await ui.screenplayTools("Revisions")
                _ = try await ui.wait(id: "revisions-sheet", timeout: 10)
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-revisions-changed", "Revisions: changed scenes and pages", from: "screenplay-revision-edited", via: "revisions-button")
                await ui.capRecover()
            }
            await ui.step("page view with marks") {
                try await ui.screenplayTools("Page View")
                _ = try await ui.wait(id: "page-view-sheet", timeout: 10)
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-page-view-revision", "Page View of the revision", from: "screenplay-revision-edited", via: "page-view-menu-item")
                try await ui.capAct(label: "Show revision marks")
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-page-view-marks-toggled", "Page View with revision marks switched", from: "screenplay-page-view-revision", via: "page-view-marks")
                await ui.capRecover()
            }
            await ui.step("revision label and colour") {
                let current = ui.screenplayDraftName()
                try await ui.screenplayTools("Drafts")
                _ = try await ui.wait(id: "drafts-sheet", timeout: 8)
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-drafts-locked-and-revision", "Drafts: the locked draft and its revision", from: "screenplay-revision-edited", via: "screenplay-tools-menu")
                try await ui.capMenuButton(label: "Actions for \(current)", item: "Revision Label")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-revision-label-colour", "Revision label and colour", from: "screenplay-drafts-locked-and-revision", via: "Actions for \(current)")
                await ui.capRecover()
            }
            await ui.step("inspector draft revision") {
                try await ui.screenplayTools("Show Inspector")
                try await ui.screenplaySegment("Draft")
                await ui.screenplayShot("screenplay-inspector-draft-revision", "Inspector: Draft information of a revision", from: "screenplay-revision-edited", via: "Draft")
                try await ui.capAct(id: "inspector-close")
            }
        },

        // Page setup, title page, custom elements, page view.
        tour("02-screenplay-setup", order: 2600) { ui, _ in
            await ui.step("page setup paper") {
                try await ui.screenplayTools("Page Setup")
                _ = try await ui.wait(id: "page-setup-sheet", timeout: 8)
                await ui.idle(0.8)
                await ui.screenplayPopUpShot("Paper", name: "screenplay-page-setup-paper", title: "Page Setup: Paper menu", from: "screenplay-tools-page-setup")
                try await ui.screenplayPick("Paper", "A4")
                await ui.idle(0.4)
                try await ui.fill("Header", "{title} {draft}")
                await ui.idle(0.6)
                await ui.screenplayShot("screenplay-page-setup-a4-header", "Page Setup: A4 with a header", from: "screenplay-page-setup-paper", via: "Paper")
                await ui.screenplayPopUpShot("Line spacing", name: "screenplay-page-setup-line-spacing", title: "Page Setup: Line spacing menu", from: "screenplay-page-setup-a4-header")
                try await ui.screenplayButton("Save")
                try await ui.waitText("Changed the page setup", timeout: 10)
                await ui.screenplayShot("screenplay-page-setup-saved", "Page setup saved: A4 pages", from: "screenplay-page-setup-a4-header", via: "page-setup-save")
            }
            await ui.step("title page filled") {
                try await ui.screenplayTools("Title Page")
                _ = try await ui.wait(id: "title-page-preview", timeout: 8)
                try await ui.fill("Date", "20 October 2026")
                try await ui.fill("Phone", "+91 98765 43210")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-title-page-filled", "Title Page with a date and phone number, live preview", from: "screenplay-tools-title-page", via: "Date")
                try await ui.screenplayButton("Save")
                try await ui.waitText("Saved the title page", timeout: 10)
                await ui.screenplayShot("screenplay-title-page-saved", "Title page saved", from: "screenplay-title-page-filled", via: "page-setup-save")
            }
            await ui.step("custom element form") {
                try await ui.screenplayTools("Custom Elements")
                _ = try await ui.wait(id: "custom-elements-sheet", timeout: 8)
                // The sheet's identifier covers the ids of its controls: find them by their labels.
                try await ui.screenplayButton("Add custom element")
                _ = try await ui.wait(label: "Name in Final Draft files", timeout: 8)
                await ui.screenplayShot("screenplay-custom-element-form", "Custom Elements: a new element", from: "screenplay-tools-custom-elements", via: "custom-element-add")
                try await ui.fill("Name", "Telephone")
                await ui.screenplayPopUpShot("Looks", name: "screenplay-custom-element-looks", title: "Custom element: Looks menu", from: "screenplay-custom-element-form")
                try await ui.screenplayPick("Looks", "Like Dialogue")
                await ui.screenplayPopUpShot("Shortcut", name: "screenplay-custom-element-shortcut", title: "Custom element: Shortcut menu", from: "screenplay-custom-element-form")
                try await ui.screenplayPick("Shortcut", "Command+Option+1")
                try await ui.screenplayButton("Add")
                try await ui.waitText("Added the custom element", timeout: 10)
                await ui.screenplayShot("screenplay-custom-elements-one", "Custom Elements with one element", from: "screenplay-custom-element-form", via: "custom-element-save")
                try await ui.screenplayButton("Done")
                await ui.idle(0.6)
            }
            await ui.step("element picker with custom") {
                try await ui.screenplayCaret("Did anyone hand in a kite?")
                await ui.capMenu(id: "element-picker", name: "screenplay-element-picker-custom", title: "Element type menu with a custom element", area: "Screenplay", from: "screenplay-custom-elements-one")
                try await ui.capMenuButton(id: "element-picker", item: "Telephone")
                await ui.idle(0.8)
                await ui.screenplayShot("screenplay-custom-element-in-script", "A line set to the custom element Telephone", from: "screenplay-element-picker-custom", via: "element-picker")
            }
            await ui.step("page view a4") {
                try await ui.screenplayTools("Page View")
                _ = try await ui.wait(id: "page-view-sheet", timeout: 8)
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-page-view-a4", "Page View on A4 with the header", from: "screenplay-page-setup-saved", via: "page-view-menu-item")
                await ui.capRecover()
            }
        },

        // Export Screenplay: every format, scope and option, and the print preview.
        tour("02-screenplay-export", order: 2700) { ui, _ in
            await ui.step("export formats") {
                try await ui.screenplayTools("Export Screenplay")
                _ = try await ui.wait(id: "export-confirm", timeout: 15)
                await ui.idle(0.8)
                for (f, label) in [("fdx", "Final Draft"), ("fountain", "Fountain"), ("docx", "DOCX")] {
                    try await ui.capAct(id: "export-format-\(f)")
                    await ui.idle(0.4)
                    await ui.screenplayShot("screenplay-export-\(f)", "Export Screenplay: \(label) chosen", from: "screenplay-tools-export-screenplay", via: "export-format-\(f)")
                }
                try await ui.capAct(id: "export-more-formats")
                await ui.idle(0.4)
                try await ui.capAct(id: "export-format-txt")
                await ui.idle(0.4)
                await ui.screenplayShot("screenplay-export-txt", "Export Screenplay: additional formats, plain text chosen", from: "screenplay-tools-export-screenplay", via: "export-more-formats")
                try await ui.capAct(id: "export-format-pdf")
                try await ui.capAct(id: "export-scope-selected")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-export-selected-scenes", "Export Screenplay: selected scenes", from: "screenplay-tools-export-screenplay", via: "export-scope-selected")
                try await ui.capAct(id: "export-scope-all")
                try await ui.capAct(id: "export-title-page")
                try await ui.capAct(id: "export-revision-marked")
                await ui.idle(0.5)
                await ui.screenplayShot("screenplay-export-options", "Export Screenplay: no title page, revision-marked", from: "screenplay-tools-export-screenplay", via: "export-revision-marked")
                try await ui.capAct(id: "export-title-page")
                try await ui.capAct(id: "export-clean")
            }
            await ui.step("print preview from export") {
                try await ui.capAct(id: "export-preview")
                _ = try await ui.wait(id: "print-print", timeout: 15)
                await ui.idle(2.0)
                await ui.screenplayShot("screenplay-print-preview", "Print Preview of the screenplay", from: "screenplay-tools-export-screenplay", via: "export-preview")
                try await ui.capAct(id: "print-next")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-print-preview-page-2", "Print Preview, the next page", from: "screenplay-print-preview", via: "print-next")
                try await ui.capAct(id: "print-zoom-in")
                try await ui.capAct(id: "print-zoom-in")
                await ui.idle(1.0)
                await ui.screenplayShot("screenplay-print-preview-zoomed", "Print Preview zoomed in", from: "screenplay-print-preview", via: "print-zoom-in")
                try await ui.capAct(id: "print-title-page")
                await ui.idle(1.5)
                await ui.screenplayShot("screenplay-print-preview-no-title", "Print Preview without the title page", from: "screenplay-print-preview", via: "print-title-page")
                await ui.capRecover()
            }
            await ui.step("print preview from menu") {
                try await ui.screenplayTools("Print")
                _ = try await ui.wait(id: "print-print", timeout: 15)
                await ui.idle(2.0)
                await ui.screenplayShot("screenplay-print-from-menu", "Print Preview opened from Screenplay Tools", from: "screenplay-tools-menu", via: "screenplay-print")
                await ui.capRecover()
            }
        },
    ]
}
